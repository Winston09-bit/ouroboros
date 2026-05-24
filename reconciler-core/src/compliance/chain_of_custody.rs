use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Actors and Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustodyActor {
    System,
    Agent(String),
    User(Uuid),
    ExternalApi(String),
}

impl std::fmt::Display for CustodyActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustodyActor::System => write!(f, "System"),
            CustodyActor::Agent(name) => write!(f, "Agent({})", name),
            CustodyActor::User(id) => write!(f, "User({})", id),
            CustodyActor::ExternalApi(name) => write!(f, "ExternalApi({})", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustodyAction {
    // Retrieval attempts
    ApiRetrievalAttempted,
    ApiRetrievalSucceeded,
    ApiRetrievalFailed { reason: String },
    PeppolRequestSent,
    EmailSent { to: String, subject: String },
    EmailOpened { at: DateTime<Utc> },
    EmailReplied { from: String },
    SmsSent { to: String },
    SmsDelivered,
    SmsReplied,
    VoiceCallInitiated,
    VoiceCallCompleted { outcome: String },
    PostalLetterSent { tracking: Option<String> },
    // Evidence
    EvidenceFound { source: String },
    EvidenceVerified { confidence: f64 },
    EvidenceRejected { reason: String },
    EvidenceUploaded { by: String },
    EvidenceAttached,
    // Escalation
    EscalationStarted { step: u8 },
    EscalationCompleted { step: u8 },
    EscalationFailed { step: u8, reason: String },
    // Outcome
    VerificationSucceeded,
    VerificationFailed { reason: String },
    LegalExportGenerated,
    FailureCertificateIssued,
}

impl CustodyAction {
    pub fn description(&self) -> String {
        match self {
            CustodyAction::ApiRetrievalAttempted => "API retrieval attempted".into(),
            CustodyAction::ApiRetrievalSucceeded => "API retrieval succeeded".into(),
            CustodyAction::ApiRetrievalFailed { reason } => {
                format!("API retrieval failed: {}", reason)
            }
            CustodyAction::PeppolRequestSent => "Peppol request sent".into(),
            CustodyAction::EmailSent { to, subject } => {
                format!("Email sent to {} – '{}'", to, subject)
            }
            CustodyAction::EmailOpened { at } => format!("Email opened at {}", at),
            CustodyAction::EmailReplied { from } => format!("Email reply received from {}", from),
            CustodyAction::SmsSent { to } => format!("SMS sent to {}", to),
            CustodyAction::SmsDelivered => "SMS delivered".into(),
            CustodyAction::SmsReplied => "SMS reply received".into(),
            CustodyAction::VoiceCallInitiated => "Voice call initiated".into(),
            CustodyAction::VoiceCallCompleted { outcome } => {
                format!("Voice call completed: {}", outcome)
            }
            CustodyAction::PostalLetterSent { tracking } => {
                format!(
                    "Postal letter sent (tracking: {})",
                    tracking.as_deref().unwrap_or("n/a")
                )
            }
            CustodyAction::EvidenceFound { source } => format!("Evidence found via {}", source),
            CustodyAction::EvidenceVerified { confidence } => {
                format!("Evidence verified (confidence {:.0}%)", confidence * 100.0)
            }
            CustodyAction::EvidenceRejected { reason } => {
                format!("Evidence rejected: {}", reason)
            }
            CustodyAction::EvidenceUploaded { by } => format!("Evidence uploaded by {}", by),
            CustodyAction::EvidenceAttached => "Evidence attached to record".into(),
            CustodyAction::EscalationStarted { step } => {
                format!("Escalation step {} started", step)
            }
            CustodyAction::EscalationCompleted { step } => {
                format!("Escalation step {} completed", step)
            }
            CustodyAction::EscalationFailed { step, reason } => {
                format!("Escalation step {} failed: {}", step, reason)
            }
            CustodyAction::VerificationSucceeded => "Verification succeeded".into(),
            CustodyAction::VerificationFailed { reason } => {
                format!("Verification failed: {}", reason)
            }
            CustodyAction::LegalExportGenerated => "Legal export package generated".into(),
            CustodyAction::FailureCertificateIssued => "Failure certificate issued".into(),
        }
    }

    pub fn status_tag(&self) -> &'static str {
        match self {
            CustodyAction::ApiRetrievalSucceeded
            | CustodyAction::EvidenceFound { .. }
            | CustodyAction::EvidenceVerified { .. }
            | CustodyAction::EvidenceAttached
            | CustodyAction::EscalationCompleted { .. }
            | CustodyAction::VerificationSucceeded
            | CustodyAction::LegalExportGenerated
            | CustodyAction::SmsDelivered
            | CustodyAction::SmsReplied
            | CustodyAction::EmailReplied { .. }
            | CustodyAction::EmailOpened { .. }
            | CustodyAction::VoiceCallCompleted { .. } => "success",

            CustodyAction::ApiRetrievalFailed { .. }
            | CustodyAction::EvidenceRejected { .. }
            | CustodyAction::EscalationFailed { .. }
            | CustodyAction::VerificationFailed { .. } => "failure",

            CustodyAction::EscalationStarted { .. }
            | CustodyAction::ApiRetrievalAttempted
            | CustodyAction::PeppolRequestSent
            | CustodyAction::EmailSent { .. }
            | CustodyAction::SmsSent { .. }
            | CustodyAction::VoiceCallInitiated
            | CustodyAction::PostalLetterSent { .. } => "pending",

            _ => "info",
        }
    }
}

// ---------------------------------------------------------------------------
// CustodyEvent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEvent {
    pub id: Uuid,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub actor: CustodyActor,
    pub action: CustodyAction,
    pub object_id: Uuid,
    pub object_type: String,
    pub channel: Option<String>,
    pub content_hash: Option<String>,
    pub previous_hash: String,
    pub event_hash: String,
    pub metadata: serde_json::Value,
}

impl CustodyEvent {
    fn compute_hash(
        sequence: u64,
        timestamp: &DateTime<Utc>,
        actor: &CustodyActor,
        action: &CustodyAction,
        object_id: &Uuid,
        previous_hash: &str,
        metadata: &serde_json::Value,
    ) -> String {
        let actor_str = serde_json::to_string(actor).unwrap_or_default();
        let action_str = serde_json::to_string(action).unwrap_or_default();
        let meta_str = serde_json::to_string(metadata).unwrap_or_default();

        let payload = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            sequence,
            timestamp.timestamp_nanos_opt().unwrap_or(0),
            actor_str,
            action_str,
            object_id,
            previous_hash,
            meta_str,
        );

        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn build(
        sequence: u64,
        actor: CustodyActor,
        action: CustodyAction,
        object_id: Uuid,
        object_type: String,
        channel: Option<String>,
        content_hash: Option<String>,
        previous_hash: String,
        metadata: serde_json::Value,
    ) -> Self {
        let timestamp = Utc::now();
        let event_hash = Self::compute_hash(
            sequence,
            &timestamp,
            &actor,
            &action,
            &object_id,
            &previous_hash,
            &metadata,
        );

        CustodyEvent {
            id: Uuid::new_v4(),
            sequence,
            timestamp,
            actor,
            action,
            object_id,
            object_type,
            channel,
            content_hash,
            previous_hash,
            event_hash,
            metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// TimelineEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub actor: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// LegalExport / ExportSummary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSummary {
    pub total_events: usize,
    pub retrieval_attempts: usize,
    pub emails_sent: usize,
    pub sms_sent: usize,
    pub escalation_level: u8,
    pub final_status: String,
    pub has_evidence: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalExport {
    pub generated_at: DateTime<Utc>,
    pub object_id: Uuid,
    pub chain_hash: String,
    pub integrity_verified: bool,
    pub events: Vec<CustodyEvent>,
    pub summary: ExportSummary,
}

// ---------------------------------------------------------------------------
// CustodyChain
// ---------------------------------------------------------------------------

pub struct CustodyChain {
    events: Vec<CustodyEvent>,
    object_id: Uuid,
    object_type: String,
}

const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

impl CustodyChain {
    pub fn new(object_id: Uuid) -> Self {
        Self::new_typed(object_id, "transaction".into())
    }

    pub fn new_typed(object_id: Uuid, object_type: String) -> Self {
        CustodyChain {
            events: Vec::new(),
            object_id,
            object_type,
        }
    }

    pub fn append(
        &mut self,
        actor: CustodyActor,
        action: CustodyAction,
        channel: Option<String>,
        metadata: serde_json::Value,
    ) -> &CustodyEvent {
        let sequence = self.events.len() as u64;
        let previous_hash = self
            .events
            .last()
            .map(|e| e.event_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        let event = CustodyEvent::build(
            sequence,
            actor,
            action,
            self.object_id,
            self.object_type.clone(),
            channel,
            None,
            previous_hash,
            metadata,
        );

        self.events.push(event);
        self.events.last().unwrap()
    }

    /// Verify the full hash chain integrity.
    pub fn verify_integrity(&self) -> bool {
        if self.events.is_empty() {
            return true;
        }

        for (i, event) in self.events.iter().enumerate() {
            // Verify sequence number
            if event.sequence != i as u64 {
                return false;
            }

            // Verify previous_hash linkage
            let expected_prev = if i == 0 {
                GENESIS_HASH.to_string()
            } else {
                self.events[i - 1].event_hash.clone()
            };
            if event.previous_hash != expected_prev {
                return false;
            }

            // Re-compute event_hash and compare
            let recomputed = CustodyEvent::compute_hash(
                event.sequence,
                &event.timestamp,
                &event.actor,
                &event.action,
                &event.object_id,
                &event.previous_hash,
                &event.metadata,
            );
            if event.event_hash != recomputed {
                return false;
            }
        }
        true
    }

    pub fn events(&self) -> &[CustodyEvent] {
        &self.events
    }

    pub fn as_timeline(&self) -> Vec<TimelineEntry> {
        self.events
            .iter()
            .map(|e| TimelineEntry {
                timestamp: e.timestamp,
                description: e.action.description(),
                actor: e.actor.to_string(),
                status: e.action.status_tag().to_string(),
            })
            .collect()
    }

    /// Compute a single hash covering the entire chain.
    fn chain_hash(&self) -> String {
        let mut hasher = Sha256::new();
        for event in &self.events {
            hasher.update(event.event_hash.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    pub fn to_legal_export(&self) -> LegalExport {
        let integrity_verified = self.verify_integrity();
        let chain_hash = self.chain_hash();

        // Build summary
        let mut retrieval_attempts = 0usize;
        let mut emails_sent = 0usize;
        let mut sms_sent = 0usize;
        let mut escalation_level: u8 = 0;
        let mut final_status = "pending".to_string();
        let mut has_evidence = false;

        for event in &self.events {
            match &event.action {
                CustodyAction::ApiRetrievalAttempted
                | CustodyAction::ApiRetrievalSucceeded
                | CustodyAction::ApiRetrievalFailed { .. }
                | CustodyAction::PeppolRequestSent => retrieval_attempts += 1,

                CustodyAction::EmailSent { .. } => emails_sent += 1,
                CustodyAction::SmsSent { .. } => sms_sent += 1,

                CustodyAction::EscalationStarted { step } => {
                    if *step > escalation_level {
                        escalation_level = *step;
                    }
                }
                CustodyAction::EscalationCompleted { step } => {
                    if *step > escalation_level {
                        escalation_level = *step;
                    }
                }

                CustodyAction::EvidenceFound { .. }
                | CustodyAction::EvidenceVerified { .. }
                | CustodyAction::EvidenceAttached => has_evidence = true,

                CustodyAction::VerificationSucceeded => {
                    final_status = "verified".to_string();
                }
                CustodyAction::VerificationFailed { reason } => {
                    final_status = format!("failed: {}", reason);
                }
                CustodyAction::FailureCertificateIssued => {
                    final_status = "unrecoverable".to_string();
                }
                _ => {}
            }
        }

        let summary = ExportSummary {
            total_events: self.events.len(),
            retrieval_attempts,
            emails_sent,
            sms_sent,
            escalation_level,
            final_status,
            has_evidence,
        };

        LegalExport {
            generated_at: Utc::now(),
            object_id: self.object_id,
            chain_hash,
            integrity_verified,
            events: self.events.clone(),
            summary,
        }
    }
}
