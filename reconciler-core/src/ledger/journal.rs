use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Core types ──────────────────────────────────────────────────────────────

/// Determines whether an entry increases or decreases an account
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryDirection {
    Debit,
    Credit,
}

/// Lifecycle state of a journal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalStatus {
    /// Created but not yet validated and locked
    Draft,
    /// Validated, locked, and applied to ledger balances
    Posted,
    /// Fully reversed by a counter-journal; original entries are immutable
    Reversed,
}

/// Immutable identity of who/what created an audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub user_id: String,
    pub session_id: Uuid,
    pub ip_address: Option<String>,
}

/// One event in a journal's audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub actor: AuditActor,
    pub metadata: Option<serde_json::Value>,
}

impl AuditEvent {
    pub fn new(event: impl Into<String>, actor: AuditActor, metadata: Option<serde_json::Value>) -> Self {
        Self {
            timestamp: Utc::now(),
            event: event.into(),
            actor,
            metadata,
        }
    }
}

// ── LedgerEntry ─────────────────────────────────────────────────────────────

/// A single debit or credit line within a journal.
/// Immutable once the parent journal is Posted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Stable, content-addressable identity for this line
    pub id: Uuid,
    /// Parent journal identifier
    pub journal_id: Uuid,
    /// BAS 2024 account code, e.g. "1930"
    pub account_code: String,
    /// Debit or credit
    pub direction: EntryDirection,
    /// Always positive; direction determines sign in the ledger
    pub amount: Decimal,
    /// ISO 4217 currency code
    pub currency: String,
    /// Exchange rate to base currency (SEK), 1.0 if already base
    pub fx_rate: Decimal,
    /// Amount converted to base currency (SEK)
    pub amount_base: Decimal,
    /// Human-readable line description
    pub description: String,
    /// Accounting period the entry belongs to, format "YYYY-MM"
    pub period: String,
    /// Timestamp when this entry was recorded
    pub recorded_at: DateTime<Utc>,
    /// Optional reference to a source document (invoice, receipt, etc.)
    pub source_document_id: Option<Uuid>,
    /// Arbitrary key-value metadata for this line
    pub metadata: Option<serde_json::Value>,
}

impl LedgerEntry {
    /// Create a new debit entry
    pub fn debit(
        journal_id: Uuid,
        account_code: impl Into<String>,
        amount: Decimal,
        currency: impl Into<String>,
        fx_rate: Decimal,
        description: impl Into<String>,
        period: impl Into<String>,
    ) -> Self {
        let amount_base = amount * fx_rate;
        Self {
            id: Uuid::new_v4(),
            journal_id,
            account_code: account_code.into(),
            direction: EntryDirection::Debit,
            amount,
            currency: currency.into(),
            fx_rate,
            amount_base,
            description: description.into(),
            period: period.into(),
            recorded_at: Utc::now(),
            source_document_id: None,
            metadata: None,
        }
    }

    /// Create a new credit entry
    pub fn credit(
        journal_id: Uuid,
        account_code: impl Into<String>,
        amount: Decimal,
        currency: impl Into<String>,
        fx_rate: Decimal,
        description: impl Into<String>,
        period: impl Into<String>,
    ) -> Self {
        let amount_base = amount * fx_rate;
        Self {
            id: Uuid::new_v4(),
            journal_id,
            account_code: account_code.into(),
            direction: EntryDirection::Credit,
            amount,
            currency: currency.into(),
            fx_rate,
            amount_base,
            description: description.into(),
            period: period.into(),
            recorded_at: Utc::now(),
            source_document_id: None,
            metadata: None,
        }
    }

    /// Returns the signed effect on the account (positive = debit, negative = credit)
    /// in base currency.
    pub fn signed_amount_base(&self) -> Decimal {
        match self.direction {
            EntryDirection::Debit => self.amount_base,
            EntryDirection::Credit => -self.amount_base,
        }
    }

    /// Build a reversal entry (flips direction, keeps amounts positive)
    pub fn reversal_entry(&self, reversal_journal_id: Uuid) -> Self {
        let reversed_direction = match self.direction {
            EntryDirection::Debit => EntryDirection::Credit,
            EntryDirection::Credit => EntryDirection::Debit,
        };
        Self {
            id: Uuid::new_v4(),
            journal_id: reversal_journal_id,
            account_code: self.account_code.clone(),
            direction: reversed_direction,
            amount: self.amount,
            currency: self.currency.clone(),
            fx_rate: self.fx_rate,
            amount_base: self.amount_base,
            description: format!("REVERSAL: {}", self.description),
            period: self.period.clone(),
            recorded_at: Utc::now(),
            source_document_id: self.source_document_id,
            metadata: self.metadata.clone(),
        }
    }
}

// ── Journal ─────────────────────────────────────────────────────────────────

/// A balanced group of ledger entries (a voucher).
/// Immutability contract: once `status == Posted`, only `Reversed` state is allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
    pub id: Uuid,
    /// Human-readable description of the economic event
    pub description: String,
    /// The economic date (may differ from posted_at for accruals)
    pub date: DateTime<Utc>,
    /// Balanced set of debit/credit entries
    pub entries: Vec<LedgerEntry>,
    /// User or system that posted this journal
    pub posted_by: String,
    /// ML / rule confidence that this posting is correct [0.0, 1.0]
    pub confidence: f64,
    /// Current lifecycle state
    pub status: JournalStatus,
    /// When the journal was first created
    pub created_at: DateTime<Utc>,
    /// When the journal was locked (Posted)
    pub posted_at: Option<DateTime<Utc>>,
    /// Link to the journal that reversed this one, if any
    pub reversed_by: Option<Uuid>,
    /// Link to the original journal this one reverses, if any
    pub reversal_of: Option<Uuid>,
    /// Ordered log of all state changes
    pub audit_trail: Vec<AuditEvent>,
    /// Optional external document reference (e.g. invoice ID, receipt S3 key)
    pub source_reference: Option<String>,
}

impl Journal {
    /// Create a new Draft journal (no entries yet)
    pub fn new(
        description: impl Into<String>,
        date: DateTime<Utc>,
        posted_by: impl Into<String>,
        confidence: f64,
        source_reference: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            date,
            entries: Vec::new(),
            posted_by: posted_by.into(),
            confidence,
            status: JournalStatus::Draft,
            created_at: Utc::now(),
            posted_at: None,
            reversed_by: None,
            reversal_of: None,
            audit_trail: Vec::new(),
            source_reference,
        }
    }

    /// Create a new Draft journal with a full entry set already attached
    pub fn with_entries(
        description: impl Into<String>,
        date: DateTime<Utc>,
        entries: Vec<LedgerEntry>,
        posted_by: impl Into<String>,
        confidence: f64,
        source_reference: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        // Re-stamp all entries with this journal's id so they're consistent
        let stamped: Vec<LedgerEntry> = entries
            .into_iter()
            .map(|mut e| {
                e.journal_id = id;
                e
            })
            .collect();
        Self {
            id,
            description: description.into(),
            date,
            entries: stamped,
            posted_by: posted_by.into(),
            confidence,
            status: JournalStatus::Draft,
            created_at: Utc::now(),
            posted_at: None,
            reversed_by: None,
            reversal_of: None,
            audit_trail: Vec::new(),
            source_reference,
        }
    }

    /// Add a ledger entry (only allowed in Draft state)
    pub fn add_entry(&mut self, mut entry: LedgerEntry) -> Result<(), JournalError> {
        if self.status != JournalStatus::Draft {
            return Err(JournalError::ImmutableJournal {
                journal_id: self.id,
                status: self.status,
            });
        }
        entry.journal_id = self.id;
        self.entries.push(entry);
        Ok(())
    }

    /// Lock the journal as Posted (irreversible except by reversal)
    pub fn post(&mut self, actor: AuditActor) -> Result<(), JournalError> {
        if self.status != JournalStatus::Draft {
            return Err(JournalError::ImmutableJournal {
                journal_id: self.id,
                status: self.status,
            });
        }
        self.status = JournalStatus::Posted;
        self.posted_at = Some(Utc::now());
        self.audit_trail.push(AuditEvent::new(
            "POSTED",
            actor,
            Some(serde_json::json!({ "entry_count": self.entries.len() })),
        ));
        Ok(())
    }

    /// Mark the journal as Reversed (called after the counter-journal is posted)
    pub fn mark_reversed(&mut self, reversal_journal_id: Uuid, actor: AuditActor) -> Result<(), JournalError> {
        if self.status != JournalStatus::Posted {
            return Err(JournalError::CannotReverseNonPosted {
                journal_id: self.id,
                status: self.status,
            });
        }
        self.status = JournalStatus::Reversed;
        self.reversed_by = Some(reversal_journal_id);
        self.audit_trail.push(AuditEvent::new(
            "REVERSED",
            actor,
            Some(serde_json::json!({ "reversal_journal_id": reversal_journal_id })),
        ));
        Ok(())
    }

    /// Build a new reversal journal that counter-posts every entry in `self`.
    /// The returned journal is in Draft state and must be validated + posted separately.
    pub fn build_reversal(
        &self,
        description: impl Into<String>,
        reversal_date: DateTime<Utc>,
        posted_by: impl Into<String>,
    ) -> Result<Journal, JournalError> {
        if self.status != JournalStatus::Posted {
            return Err(JournalError::CannotReverseNonPosted {
                journal_id: self.id,
                status: self.status,
            });
        }

        let reversal_id = Uuid::new_v4();
        let reversal_entries: Vec<LedgerEntry> = self
            .entries
            .iter()
            .map(|e| e.reversal_entry(reversal_id))
            .collect();

        Ok(Journal {
            id: reversal_id,
            description: description.into(),
            date: reversal_date,
            entries: reversal_entries,
            posted_by: posted_by.into(),
            confidence: 1.0,
            status: JournalStatus::Draft,
            created_at: Utc::now(),
            posted_at: None,
            reversed_by: None,
            reversal_of: Some(self.id),
            audit_trail: Vec::new(),
            source_reference: self.source_reference.clone(),
        })
    }

    /// Sum of all debit amounts (base currency)
    pub fn debit_total(&self) -> Decimal {
        self.entries
            .iter()
            .filter(|e| e.direction == EntryDirection::Debit)
            .map(|e| e.amount_base)
            .sum()
    }

    /// Sum of all credit amounts (base currency)
    pub fn credit_total(&self) -> Decimal {
        self.entries
            .iter()
            .filter(|e| e.direction == EntryDirection::Credit)
            .map(|e| e.amount_base)
            .sum()
    }

    /// True when debit_total == credit_total (balanced journal)
    pub fn is_balanced(&self) -> bool {
        self.debit_total() == self.credit_total()
    }

    /// Accounting period derived from the journal's economic date ("YYYY-MM")
    pub fn period(&self) -> String {
        self.date.format("%Y-%m").to_string()
    }
}

// ── JournalError ─────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("Journal {journal_id} is {status:?} and cannot be modified")]
    ImmutableJournal {
        journal_id: Uuid,
        status: JournalStatus,
    },

    #[error("Cannot reverse journal {journal_id}: status is {status:?}, must be Posted")]
    CannotReverseNonPosted {
        journal_id: Uuid,
        status: JournalStatus,
    },

    #[error("Journal {journal_id} is unbalanced: debits={debits}, credits={credits}")]
    UnbalancedJournal {
        journal_id: Uuid,
        debits: Decimal,
        credits: Decimal,
    },

    #[error("Journal {journal_id} has no entries")]
    EmptyJournal { journal_id: Uuid },

    #[error("Negative amount {amount} on entry {entry_id}")]
    NegativeAmount { entry_id: Uuid, amount: Decimal },

    #[error("Invalid account code '{code}' on entry {entry_id}")]
    InvalidAccountCode { entry_id: Uuid, code: String },

    #[error("Journal must have at least one debit and one credit entry")]
    MissingDebitOrCredit,
}

// ── JournalBuilder ───────────────────────────────────────────────────────────

/// Fluent builder for constructing journals before validation
pub struct JournalBuilder {
    description: String,
    date: DateTime<Utc>,
    posted_by: String,
    confidence: f64,
    entries: Vec<LedgerEntry>,
    source_reference: Option<String>,
}

impl JournalBuilder {
    pub fn new(
        description: impl Into<String>,
        date: DateTime<Utc>,
        posted_by: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            date,
            posted_by: posted_by.into(),
            confidence: 1.0,
            entries: Vec::new(),
            source_reference: None,
        }
    }

    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn source_reference(mut self, reference: impl Into<String>) -> Self {
        self.source_reference = Some(reference.into());
        self
    }

    pub fn debit(
        mut self,
        account_code: impl Into<String>,
        amount: Decimal,
        currency: impl Into<String>,
        fx_rate: Decimal,
        description: impl Into<String>,
    ) -> Self {
        let period = self.date.format("%Y-%m").to_string();
        let entry = LedgerEntry::debit(
            Uuid::nil(), // will be stamped by Journal::with_entries
            account_code,
            amount,
            currency,
            fx_rate,
            description,
            period,
        );
        self.entries.push(entry);
        self
    }

    pub fn credit(
        mut self,
        account_code: impl Into<String>,
        amount: Decimal,
        currency: impl Into<String>,
        fx_rate: Decimal,
        description: impl Into<String>,
    ) -> Self {
        let period = self.date.format("%Y-%m").to_string();
        let entry = LedgerEntry::credit(
            Uuid::nil(),
            account_code,
            amount,
            currency,
            fx_rate,
            description,
            period,
        );
        self.entries.push(entry);
        self
    }

    /// Finalise into a Draft `Journal` (not yet validated)
    pub fn build(self) -> Journal {
        Journal::with_entries(
            self.description,
            self.date,
            self.entries,
            self.posted_by,
            self.confidence,
            self.source_reference,
        )
    }
}
