use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceState {
    // Initial states
    Ingested,

    // Matching
    Matching,
    Matched,
    PartialMatch,

    // Evidence retrieval
    SearchingEvidence,
    EvidenceFound,
    EvidencePending,
    MissingEvidence,

    // Retrieval pipeline
    ApiRetrieving { attempt: u8 },
    EmailSent { step: u8, sent_at: DateTime<Utc> },
    AwaitingReply { since: DateTime<Utc> },
    ReminderSent { reminder_number: u8 },
    Escalated { level: u8 },
    PostalSent,

    // Terminal states
    Verified,
    PartiallyVerified,
    Unrecoverable,
    LegallyDocumented,

    // Special
    PendingReview,
    VatConflict,
    Disputed,
    Rejected,
}

impl EvidenceState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            EvidenceState::Verified
                | EvidenceState::PartiallyVerified
                | EvidenceState::Unrecoverable
                | EvidenceState::LegallyDocumented
                | EvidenceState::Rejected
        )
    }

    pub fn label(&self) -> &str {
        match self {
            EvidenceState::Ingested => "Ingested",
            EvidenceState::Matching => "Matching",
            EvidenceState::Matched => "Matched",
            EvidenceState::PartialMatch => "PartialMatch",
            EvidenceState::SearchingEvidence => "SearchingEvidence",
            EvidenceState::EvidenceFound => "EvidenceFound",
            EvidenceState::EvidencePending => "EvidencePending",
            EvidenceState::MissingEvidence => "MissingEvidence",
            EvidenceState::ApiRetrieving { .. } => "ApiRetrieving",
            EvidenceState::EmailSent { .. } => "EmailSent",
            EvidenceState::AwaitingReply { .. } => "AwaitingReply",
            EvidenceState::ReminderSent { .. } => "ReminderSent",
            EvidenceState::Escalated { .. } => "Escalated",
            EvidenceState::PostalSent => "PostalSent",
            EvidenceState::Verified => "Verified",
            EvidenceState::PartiallyVerified => "PartiallyVerified",
            EvidenceState::Unrecoverable => "Unrecoverable",
            EvidenceState::LegallyDocumented => "LegallyDocumented",
            EvidenceState::PendingReview => "PendingReview",
            EvidenceState::VatConflict => "VatConflict",
            EvidenceState::Disputed => "Disputed",
            EvidenceState::Rejected => "Rejected",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            EvidenceState::Ingested => "Transaktion inläst, ej behandlad",
            EvidenceState::Matching => "Matchar mot ERP",
            EvidenceState::Matched => "Matchad mot verifikation",
            EvidenceState::PartialMatch => "Delvis matchad",
            EvidenceState::SearchingEvidence => "Söker underlag",
            EvidenceState::EvidenceFound => "Underlag hittat",
            EvidenceState::EvidencePending => "Underlag begärt, väntar",
            EvidenceState::MissingEvidence => "Underlag saknas",
            EvidenceState::ApiRetrieving { .. } => "Hämtar via API",
            EvidenceState::EmailSent { .. } => "Email skickat till merchant",
            EvidenceState::AwaitingReply { .. } => "Inväntar svar",
            EvidenceState::ReminderSent { .. } => "Påminnelse skickad",
            EvidenceState::Escalated { .. } => "Eskalerad",
            EvidenceState::PostalSent => "Rekommenderat brev skickat",
            EvidenceState::Verified => "Fullständigt verifierat",
            EvidenceState::PartiallyVerified => "Delvis verifierat",
            EvidenceState::Unrecoverable => "Ej möjligt att få underlag",
            EvidenceState::LegallyDocumented => "Failure certificate utfärdat",
            EvidenceState::PendingReview => "Manuell granskning krävs",
            EvidenceState::VatConflict => "VAT-mismatch",
            EvidenceState::Disputed => "Under tvist",
            EvidenceState::Rejected => "Underkänd",
        }
    }

    pub fn next_action(&self) -> Option<String> {
        match self {
            EvidenceState::Ingested => Some("Start matching against ERP".into()),
            EvidenceState::Matching => Some("Await match result".into()),
            EvidenceState::Matched => Some("Retrieve evidence".into()),
            EvidenceState::PartialMatch => Some("Review unmatched portions".into()),
            EvidenceState::SearchingEvidence => Some("Await evidence search result".into()),
            EvidenceState::EvidenceFound => Some("Verify evidence".into()),
            EvidenceState::EvidencePending => Some("Await evidence arrival".into()),
            EvidenceState::MissingEvidence => Some("Initiate escalation".into()),
            EvidenceState::ApiRetrieving { attempt } => {
                Some(format!("Await API response (attempt {})", attempt))
            }
            EvidenceState::EmailSent { step, .. } => {
                Some(format!("Await reply to email step {}", step))
            }
            EvidenceState::AwaitingReply { .. } => Some("Wait for merchant reply".into()),
            EvidenceState::ReminderSent { reminder_number } => {
                Some(format!("Await reply to reminder #{}", reminder_number))
            }
            EvidenceState::Escalated { level } => {
                Some(format!("Execute escalation level {} action", level))
            }
            EvidenceState::PostalSent => Some("Await postal delivery confirmation".into()),
            EvidenceState::PendingReview => Some("Assign to reviewer".into()),
            EvidenceState::VatConflict => Some("Resolve VAT discrepancy".into()),
            EvidenceState::Disputed => Some("Initiate dispute resolution".into()),
            // Terminal states
            EvidenceState::Verified
            | EvidenceState::PartiallyVerified
            | EvidenceState::Unrecoverable
            | EvidenceState::LegallyDocumented
            | EvidenceState::Rejected => None,
        }
    }

    pub fn allows_transition_to(&self, next: &EvidenceState) -> bool {
        if self.is_terminal() {
            return false;
        }
        use EvidenceState::*;
        match (self, next) {
            // From Ingested
            (Ingested, Matching) => true,
            (Ingested, PendingReview) => true,

            // From Matching
            (Matching, Matched) => true,
            (Matching, PartialMatch) => true,
            (Matching, MissingEvidence) => true,
            (Matching, PendingReview) => true,

            // From Matched
            (Matched, SearchingEvidence) => true,
            (Matched, EvidenceFound) => true,
            (Matched, Verified) => true,

            // From PartialMatch
            (PartialMatch, SearchingEvidence) => true,
            (PartialMatch, PartiallyVerified) => true,
            (PartialMatch, PendingReview) => true,

            // From SearchingEvidence
            (SearchingEvidence, EvidenceFound) => true,
            (SearchingEvidence, EvidencePending) => true,
            (SearchingEvidence, MissingEvidence) => true,
            (SearchingEvidence, ApiRetrieving { .. }) => true,

            // From EvidenceFound
            (EvidenceFound, Verified) => true,
            (EvidenceFound, PartiallyVerified) => true,
            (EvidenceFound, VatConflict) => true,
            (EvidenceFound, PendingReview) => true,

            // From EvidencePending
            (EvidencePending, EvidenceFound) => true,
            (EvidencePending, MissingEvidence) => true,
            (EvidencePending, AwaitingReply { .. }) => true,

            // From MissingEvidence
            (MissingEvidence, ApiRetrieving { .. }) => true,
            (MissingEvidence, EmailSent { .. }) => true,
            (MissingEvidence, Escalated { .. }) => true,
            (MissingEvidence, Unrecoverable) => true,

            // From ApiRetrieving
            (ApiRetrieving { .. }, EvidenceFound) => true,
            (ApiRetrieving { .. }, ApiRetrieving { .. }) => true,
            (ApiRetrieving { .. }, EmailSent { .. }) => true,
            (ApiRetrieving { .. }, MissingEvidence) => true,

            // From EmailSent
            (EmailSent { .. }, AwaitingReply { .. }) => true,
            (EmailSent { .. }, EvidenceFound) => true,
            (EmailSent { .. }, ReminderSent { .. }) => true,
            (EmailSent { .. }, Escalated { .. }) => true,

            // From AwaitingReply
            (AwaitingReply { .. }, EvidenceFound) => true,
            (AwaitingReply { .. }, ReminderSent { .. }) => true,
            (AwaitingReply { .. }, Escalated { .. }) => true,
            (AwaitingReply { .. }, MissingEvidence) => true,

            // From ReminderSent
            (ReminderSent { .. }, AwaitingReply { .. }) => true,
            (ReminderSent { .. }, EvidenceFound) => true,
            (ReminderSent { .. }, Escalated { .. }) => true,
            (ReminderSent { .. }, PostalSent) => true,

            // From Escalated
            (Escalated { .. }, Escalated { .. }) => true,
            (Escalated { .. }, PostalSent) => true,
            (Escalated { .. }, EvidenceFound) => true,
            (Escalated { .. }, LegallyDocumented) => true,
            (Escalated { .. }, Unrecoverable) => true,

            // From PostalSent
            (PostalSent, EvidenceFound) => true,
            (PostalSent, LegallyDocumented) => true,
            (PostalSent, Unrecoverable) => true,
            (PostalSent, Escalated { .. }) => true,

            // From PendingReview
            (PendingReview, Matching) => true,
            (PendingReview, SearchingEvidence) => true,
            (PendingReview, Escalated { .. }) => true,
            (PendingReview, Verified) => true,
            (PendingReview, Rejected) => true,
            (PendingReview, Disputed) => true,

            // From VatConflict
            (VatConflict, PendingReview) => true,
            (VatConflict, Verified) => true,
            (VatConflict, PartiallyVerified) => true,
            (VatConflict, Disputed) => true,

            // From Disputed
            (Disputed, PendingReview) => true,
            (Disputed, Verified) => true,
            (Disputed, Rejected) => true,
            (Disputed, LegallyDocumented) => true,

            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: EvidenceState,
    pub to: EvidenceState,
    pub timestamp: DateTime<Utc>,
    pub triggered_by: String,
    pub reason: Option<String>,
}

pub struct StateMachine {
    pub current_state: EvidenceState,
    pub history: Vec<StateTransition>,
    pub transaction_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StateMachine {
    pub fn new(transaction_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            current_state: EvidenceState::Ingested,
            history: Vec::new(),
            transaction_id,
            started_at: now,
            updated_at: now,
        }
    }

    pub fn transition(
        &mut self,
        to: EvidenceState,
        triggered_by: &str,
        reason: Option<String>,
    ) -> anyhow::Result<&StateTransition> {
        if !self.can_transition_to(&to) {
            return Err(anyhow::anyhow!(
                "Invalid transition from {:?} to {:?}",
                self.current_state,
                to
            ));
        }
        let now = Utc::now();
        let transition = StateTransition {
            from: self.current_state.clone(),
            to: to.clone(),
            timestamp: now,
            triggered_by: triggered_by.to_string(),
            reason,
        };
        self.history.push(transition);
        self.current_state = to;
        self.updated_at = now;
        Ok(self.history.last().unwrap())
    }

    pub fn can_transition_to(&self, state: &EvidenceState) -> bool {
        self.current_state.allows_transition_to(state)
    }

    pub fn time_in_current_state(&self) -> chrono::Duration {
        let since = self
            .history
            .last()
            .map(|t| t.timestamp)
            .unwrap_or(self.started_at);
        Utc::now() - since
    }

    pub fn escalation_level(&self) -> u8 {
        match &self.current_state {
            EvidenceState::Escalated { level } => *level,
            EvidenceState::ReminderSent { reminder_number } => *reminder_number,
            EvidenceState::PostalSent => 6,
            EvidenceState::LegallyDocumented => 7,
            _ => 0,
        }
    }
}
