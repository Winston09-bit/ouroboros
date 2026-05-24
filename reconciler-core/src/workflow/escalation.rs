use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationAction {
    ApiRetrieval,
    PeppolRequest,
    EmailFirst,
    EmailReminder { reminder_number: u8 },
    SmsReminder,
    RegisteredLetterFirst,
    RegisteredLetterFinal,
    LegalExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStep {
    pub step: u8,
    pub name: String,
    pub description: String,
    #[serde(with = "duration_serde")]
    pub wait_after_prev: Duration,
    pub action: EscalationAction,
    pub is_automated: bool,
    pub legal_weight: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    pub steps: Vec<EscalationStep>,
}

impl EscalationConfig {
    pub fn standard() -> Self {
        Self {
            steps: vec![
                EscalationStep {
                    step: 1,
                    name: "API Retrieval".into(),
                    description: "Direkt API-hämtning från merchant/ERP".into(),
                    wait_after_prev: Duration::minutes(0),
                    action: EscalationAction::ApiRetrieval,
                    is_automated: true,
                    legal_weight: "none".to_string(),
                },
                EscalationStep {
                    step: 2,
                    name: "Peppol Request".into(),
                    description: "EDI/Peppol-förfrågan".into(),
                    wait_after_prev: Duration::hours(1),
                    action: EscalationAction::PeppolRequest,
                    is_automated: true,
                    legal_weight: "low".to_string(),
                },
                EscalationStep {
                    step: 3,
                    name: "Email First".into(),
                    description: "Professionellt mail till merchant".into(),
                    wait_after_prev: Duration::hours(2),
                    action: EscalationAction::EmailFirst,
                    is_automated: true,
                    legal_weight: "low".to_string(),
                },
                EscalationStep {
                    step: 4,
                    name: "Email Reminder".into(),
                    description: "Påminnelsemail (3 dagar)".into(),
                    wait_after_prev: Duration::days(3),
                    action: EscalationAction::EmailReminder { reminder_number: 1 },
                    is_automated: true,
                    legal_weight: "medium".to_string(),
                },
                EscalationStep {
                    step: 5,
                    name: "SMS Reminder".into(),
                    description: "SMS till kontaktperson".into(),
                    wait_after_prev: Duration::days(4),
                    action: EscalationAction::SmsReminder,
                    is_automated: true,
                    legal_weight: "medium".to_string(),
                },
                EscalationStep {
                    step: 6,
                    name: "Registered Letter".into(),
                    description: "Rekommenderat brev".into(),
                    wait_after_prev: Duration::days(7),
                    action: EscalationAction::RegisteredLetterFirst,
                    is_automated: false,
                    legal_weight: "high".to_string(),
                },
                EscalationStep {
                    step: 7,
                    name: "Legal Export".into(),
                    description: "Juridisk export + Failure Certificate".into(),
                    wait_after_prev: Duration::days(14),
                    action: EscalationAction::LegalExport,
                    is_automated: true,
                    legal_weight: "legal".to_string(),
                },
            ],
        }
    }

    pub fn step(&self, n: u8) -> Option<&EscalationStep> {
        self.steps.iter().find(|s| s.step == n)
    }

    pub fn next_step(&self, current: u8) -> Option<&EscalationStep> {
        self.steps.iter().find(|s| s.step == current + 1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStep {
    pub step: u8,
    pub name: String,
    pub completed_at: DateTime<Utc>,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStatus {
    pub current_step: u8,
    pub total_steps: u8,
    pub next_action_at: Option<DateTime<Utc>>,
    pub next_action: Option<String>,
    pub completed_steps: Vec<CompletedStep>,
    pub is_complete: bool,
}

pub struct EscalationOrchestrator {
    config: EscalationConfig,
}

impl EscalationOrchestrator {
    pub fn new(config: EscalationConfig) -> Self {
        Self { config }
    }

    pub fn with_standard_config() -> Self {
        Self::new(EscalationConfig::standard())
    }

    pub fn status(&self, completed: &[CompletedStep]) -> EscalationStatus {
        let total_steps = self.config.steps.len() as u8;
        let current_step = completed.last().map(|s| s.step).unwrap_or(0);
        let is_complete = current_step >= total_steps;

        let (next_action_at, next_action) = if is_complete {
            (None, None)
        } else {
            let next_step_num = current_step + 1;
            if let Some(next) = self.config.step(next_step_num) {
                let base_time = completed
                    .last()
                    .map(|s| s.completed_at)
                    .unwrap_or_else(Utc::now);
                let due_at = base_time + next.wait_after_prev;
                (Some(due_at), Some(next.name.clone()))
            } else {
                (None, None)
            }
        };

        EscalationStatus {
            current_step,
            total_steps,
            next_action_at,
            next_action,
            completed_steps: completed.to_vec(),
            is_complete,
        }
    }

    pub fn due_steps<'a>(
        &'a self,
        completed: &[CompletedStep],
        from: DateTime<Utc>,
    ) -> Vec<&'a EscalationStep> {
        let last_completed_step = completed.last().map(|s| s.step).unwrap_or(0);
        let last_completed_at = completed.last().map(|s| s.completed_at).unwrap_or(from);

        self.config
            .steps
            .iter()
            .filter(|step| {
                if step.step <= last_completed_step {
                    return false;
                }
                // Only consider the immediate next step due (sequential escalation)
                if step.step != last_completed_step + 1 {
                    return false;
                }
                let due_at = last_completed_at + step.wait_after_prev;
                from >= due_at
            })
            .collect()
    }
}

/// Serde helper for chrono::Duration (stores as total seconds i64)
mod duration_serde {
    use chrono::Duration;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.num_seconds().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = i64::deserialize(d)?;
        Ok(Duration::seconds(secs))
    }
}
