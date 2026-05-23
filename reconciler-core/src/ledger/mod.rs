pub mod accounts;
pub mod journal;

use crate::ledger::accounts::ChartOfAccounts;
use crate::ledger::journal::{AuditActor, EntryDirection, Journal, JournalError, JournalStatus, LedgerEntry};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ── Public re-exports ────────────────────────────────────────────────────────
pub use accounts::{Account, AccountClassifier, AccountSuggestion, AccountType, ChartOfAccounts as AccountRegistry, NormalBalance, VatClass};
pub use journal::{AuditEvent, EntryDirection as Direction, Journal as LedgerJournal, JournalBuilder, JournalStatus as Status, LedgerEntry};

// ── Value types ──────────────────────────────────────────────────────────────

pub type JournalId = Uuid;

/// Balance of a single account for one period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_code: String,
    pub account_name: String,
    pub period: String,
    /// Net balance: debit-normal accounts → positive when debits exceed credits
    /// credit-normal accounts → positive when credits exceed debits
    pub balance: Decimal,
    /// Raw debit total (base currency)
    pub debit_total: Decimal,
    /// Raw credit total (base currency)
    pub credit_total: Decimal,
    pub currency: String,
    pub computed_at: DateTime<Utc>,
}

/// Aggregate trial balance for a period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalance {
    pub period: String,
    pub rows: Vec<TrialBalanceRow>,
    pub debit_total: Decimal,
    pub credit_total: Decimal,
    pub is_balanced: bool,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalanceRow {
    pub account_code: String,
    pub account_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
}

impl TrialBalance {
    fn build(period: impl Into<String>, rows: Vec<TrialBalanceRow>) -> Self {
        let debit_total: Decimal = rows.iter().map(|r| r.debit).sum();
        let credit_total: Decimal = rows.iter().map(|r| r.credit).sum();
        let is_balanced = debit_total == credit_total;
        Self {
            period: period.into(),
            rows,
            debit_total,
            credit_total,
            is_balanced,
            computed_at: Utc::now(),
        }
    }
}

/// Full ledger state at a given point in time (for temporal replays)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerSnapshot {
    /// Cutoff timestamp used to filter entries
    pub as_of: DateTime<Utc>,
    /// All journal entries recorded at or before `as_of`
    pub entries: Vec<LedgerEntry>,
    /// Per-account balances derived from `entries`
    pub balances: HashMap<String, Decimal>,
    /// Number of journals included
    pub journal_count: usize,
    pub created_at: DateTime<Utc>,
}

// ── Validation ───────────────────────────────────────────────────────────────

/// Validates a proposed set of journal entries before posting
pub struct JournalValidator<'a> {
    chart: &'a ChartOfAccounts,
}

impl<'a> JournalValidator<'a> {
    pub fn new(chart: &'a ChartOfAccounts) -> Self {
        Self { chart }
    }

    /// Full validation pass. Returns Ok(()) when all invariants hold.
    pub fn validate(&self, journal: &Journal) -> Result<(), JournalError> {
        if journal.entries.is_empty() {
            return Err(JournalError::EmptyJournal {
                journal_id: journal.id,
            });
        }

        let has_debit = journal
            .entries
            .iter()
            .any(|e| e.direction == EntryDirection::Debit);
        let has_credit = journal
            .entries
            .iter()
            .any(|e| e.direction == EntryDirection::Credit);

        if !has_debit || !has_credit {
            return Err(JournalError::MissingDebitOrCredit);
        }

        for entry in &journal.entries {
            if entry.amount < Decimal::ZERO {
                return Err(JournalError::NegativeAmount {
                    entry_id: entry.id,
                    amount: entry.amount,
                });
            }
            if entry.amount_base < Decimal::ZERO {
                return Err(JournalError::NegativeAmount {
                    entry_id: entry.id,
                    amount: entry.amount_base,
                });
            }
            if !self.chart.is_valid_code(&entry.account_code) {
                return Err(JournalError::InvalidAccountCode {
                    entry_id: entry.id,
                    code: entry.account_code.clone(),
                });
            }
        }

        if !journal.is_balanced() {
            return Err(JournalError::UnbalancedJournal {
                journal_id: journal.id,
                debits: journal.debit_total(),
                credits: journal.credit_total(),
            });
        }

        Ok(())
    }
}

// ── Storage abstraction ──────────────────────────────────────────────────────

/// Internal append-only storage for posted journals.
/// In production this would be backed by Postgres with an advisory lock per journal_id;
/// here we use an in-process RwLock-guarded Vec for simplicity and testability.
struct LedgerStore {
    journals: Vec<Journal>,
}

impl LedgerStore {
    fn new() -> Self {
        Self {
            journals: Vec::new(),
        }
    }

    fn append(&mut self, journal: Journal) {
        self.journals.push(journal);
    }

    fn get(&self, id: &Uuid) -> Option<&Journal> {
        self.journals.iter().find(|j| &j.id == id)
    }

    fn get_mut(&mut self, id: &Uuid) -> Option<&mut Journal> {
        self.journals.iter_mut().find(|j| &j.id == id)
    }

    fn all_posted_entries_before(&self, cutoff: &DateTime<Utc>) -> Vec<&LedgerEntry> {
        self.journals
            .iter()
            .filter(|j| {
                j.status == JournalStatus::Posted
                    && j.posted_at.map(|t| &t <= cutoff).unwrap_or(false)
            })
            .flat_map(|j| j.entries.iter())
            .collect()
    }

    fn all_posted_entries_in_period<'a>(&'a self, period: &str) -> Vec<&'a LedgerEntry> {
        self.journals
            .iter()
            .filter(|j| j.status == JournalStatus::Posted)
            .flat_map(|j| j.entries.iter())
            .filter(|e| e.period == period)
            .collect()
    }
}

// ── LedgerEngine ─────────────────────────────────────────────────────────────

/// The central double-entry ledger engine.
///
/// # Invariants
/// - Every posted journal is balanced (debit total == credit total in base currency).
/// - Journals are append-only: once Posted they can only transition to Reversed.
/// - A reversal creates a new counter-journal; the original journal is never mutated.
/// - All monetary amounts are stored in base currency (SEK) alongside the original
///   amount and FX rate for full auditability.
pub struct LedgerEngine {
    chart: Arc<ChartOfAccounts>,
    store: Arc<RwLock<LedgerStore>>,
}

impl LedgerEngine {
    /// Create a new engine backed by the standard BAS 2024 chart of accounts
    pub fn new_bas2024() -> Self {
        Self {
            chart: Arc::new(ChartOfAccounts::bas_2024()),
            store: Arc::new(RwLock::new(LedgerStore::new())),
        }
    }

    /// Create a new engine with a custom chart of accounts
    pub fn with_chart(chart: ChartOfAccounts) -> Self {
        Self {
            chart: Arc::new(chart),
            store: Arc::new(RwLock::new(LedgerStore::new())),
        }
    }

    /// Validate, lock, and record a journal.
    ///
    /// The `entries` slice must form a balanced set (Σ debits == Σ credits in base
    /// currency). Returns the UUID of the newly created journal on success.
    pub fn post_journal(
        &self,
        description: impl Into<String>,
        date: DateTime<Utc>,
        entries: Vec<LedgerEntry>,
        posted_by: impl Into<String>,
        confidence: f64,
        source_reference: Option<String>,
        actor: AuditActor,
    ) -> Result<JournalId> {
        let posted_by = posted_by.into();
        let mut journal = Journal::with_entries(
            description,
            date,
            entries,
            &posted_by,
            confidence,
            source_reference,
        );

        let validator = JournalValidator::new(&self.chart);
        validator
            .validate(&journal)
            .context("Journal validation failed")?;

        journal
            .post(actor)
            .context("Failed to transition journal to Posted")?;

        let id = journal.id;
        let mut store = self
            .store
            .write()
            .map_err(|_| anyhow!("Ledger store lock poisoned"))?;
        store.append(journal);
        Ok(id)
    }

    /// Retrieve all entries belonging to a specific journal
    pub fn get_journal_entries(&self, journal_id: Uuid) -> Result<Vec<LedgerEntry>> {
        let store = self
            .store
            .read()
            .map_err(|_| anyhow!("Ledger store lock poisoned"))?;

        store
            .get(&journal_id)
            .map(|j| j.entries.clone())
            .ok_or_else(|| anyhow!("Journal {} not found", journal_id))
    }

    /// Calculate the balance for a single account in a given period ("YYYY-MM").
    ///
    /// For debit-normal accounts (assets, expenses) a positive balance means
    /// debit > credit. For credit-normal accounts the convention is inverted.
    pub fn get_balance(&self, account_code: &str, period: &str) -> Result<AccountBalance> {
        let account = self
            .chart
            .get(account_code)
            .ok_or_else(|| anyhow!("Account code '{}' not in chart of accounts", account_code))?;

        let store = self
            .store
            .read()
            .map_err(|_| anyhow!("Ledger store lock poisoned"))?;

        let entries = store.all_posted_entries_in_period(period);
        let account_entries: Vec<&&LedgerEntry> = entries
            .iter()
            .filter(|e| e.account_code == account_code)
            .collect();

        let debit_total: Decimal = account_entries
            .iter()
            .filter(|e| e.direction == EntryDirection::Debit)
            .map(|e| e.amount_base)
            .sum();

        let credit_total: Decimal = account_entries
            .iter()
            .filter(|e| e.direction == EntryDirection::Credit)
            .map(|e| e.amount_base)
            .sum();

        let balance = match account.normal_balance {
            NormalBalance::Debit => debit_total - credit_total,
            NormalBalance::Credit => credit_total - debit_total,
        };

        Ok(AccountBalance {
            account_code: account_code.to_string(),
            account_name: account.name.clone(),
            period: period.to_string(),
            balance,
            debit_total,
            credit_total,
            currency: "SEK".to_string(),
            computed_at: Utc::now(),
        })
    }

    /// Build a trial balance for a period showing raw debit/credit totals per account.
    ///
    /// The trial balance includes every account that has at least one entry in the period.
    /// `is_balanced` will be true when Σ debits == Σ credits (a fundamental accounting invariant;
    /// if false it indicates a bug in the engine or data corruption).
    pub fn get_trial_balance(&self, period: &str) -> Result<TrialBalance> {
        let store = self
            .store
            .read()
            .map_err(|_| anyhow!("Ledger store lock poisoned"))?;

        let entries = store.all_posted_entries_in_period(period);

        // Aggregate per account code
        let mut account_totals: HashMap<String, (Decimal, Decimal)> = HashMap::new();
        for entry in &entries {
            let totals = account_totals
                .entry(entry.account_code.clone())
                .or_insert((Decimal::ZERO, Decimal::ZERO));
            match entry.direction {
                EntryDirection::Debit => totals.0 += entry.amount_base,
                EntryDirection::Credit => totals.1 += entry.amount_base,
            }
        }

        let mut rows: Vec<TrialBalanceRow> = account_totals
            .into_iter()
            .map(|(code, (debit, credit))| {
                let name = self
                    .chart
                    .get(&code)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| code.clone());
                TrialBalanceRow {
                    account_code: code,
                    account_name: name,
                    debit,
                    credit,
                }
            })
            .collect();

        rows.sort_by(|a, b| a.account_code.cmp(&b.account_code));

        Ok(TrialBalance::build(period, rows))
    }

    /// Reverse a previously posted journal.
    ///
    /// Creates a new counter-journal with all entries flipped (debits ↔ credits),
    /// posts it, and marks the original journal as Reversed. Both journals are
    /// immutable after this call completes.
    pub fn reverse_journal(
        &self,
        journal_id: Uuid,
        reversal_description: impl Into<String>,
        reversal_date: DateTime<Utc>,
        posted_by: impl Into<String>,
        actor: AuditActor,
    ) -> Result<JournalId> {
        let reversal_actor_clone = actor.clone();
        let posted_by = posted_by.into();

        // Read the original to build the reversal journal
        let reversal_journal = {
            let store = self
                .store
                .read()
                .map_err(|_| anyhow!("Ledger store lock poisoned"))?;

            let original = store
                .get(&journal_id)
                .ok_or_else(|| anyhow!("Journal {} not found", journal_id))?;

            original
                .build_reversal(reversal_description, reversal_date, &posted_by)
                .context("Failed to build reversal journal")?
        };

        // Validate and post the reversal
        let validator = JournalValidator::new(&self.chart);
        validator
            .validate(&reversal_journal)
            .context("Reversal journal validation failed")?;

        let mut reversal_journal = reversal_journal;
        reversal_journal
            .post(actor)
            .context("Failed to post reversal journal")?;

        let reversal_id = reversal_journal.id;

        // Atomically append and mark original as reversed
        let mut store = self
            .store
            .write()
            .map_err(|_| anyhow!("Ledger store lock poisoned"))?;

        store.append(reversal_journal);

        let original = store
            .get_mut(&journal_id)
            .ok_or_else(|| anyhow!("Journal {} disappeared during reversal", journal_id))?;

        original
            .mark_reversed(reversal_id, reversal_actor_clone)
            .context("Failed to mark original journal as reversed")?;

        Ok(reversal_id)
    }

    /// Reconstruct the complete ledger state as it existed at `timestamp`.
    ///
    /// Only entries from journals posted at or before `timestamp` are included,
    /// giving full temporal accounting support (point-in-time reconstruction).
    pub fn replay_from(&self, timestamp: DateTime<Utc>) -> Result<LedgerSnapshot> {
        let store = self
            .store
            .read()
            .map_err(|_| anyhow!("Ledger store lock poisoned"))?;

        let entries: Vec<LedgerEntry> = store
            .all_posted_entries_before(&timestamp)
            .into_iter()
            .cloned()
            .collect();

        let journal_count = store
            .journals
            .iter()
            .filter(|j| {
                j.status == JournalStatus::Posted
                    && j.posted_at.map(|t| t <= timestamp).unwrap_or(false)
            })
            .count();

        // Build running balances (signed: debit positive, credit negative)
        let mut balances: HashMap<String, Decimal> = HashMap::new();
        for entry in &entries {
            let bal = balances
                .entry(entry.account_code.clone())
                .or_insert(Decimal::ZERO);
            *bal += entry.signed_amount_base();
        }

        Ok(LedgerSnapshot {
            as_of: timestamp,
            entries,
            balances,
            journal_count,
            created_at: Utc::now(),
        })
    }

    /// Expose the chart of accounts
    pub fn chart(&self) -> &ChartOfAccounts {
        &self.chart
    }
}

// ── Convenience trait for LedgerEntry signed amount ─────────────────────────

trait SignedAmount {
    fn signed_amount_base(&self) -> Decimal;
}

impl SignedAmount for LedgerEntry {
    fn signed_amount_base(&self) -> Decimal {
        match self.direction {
            EntryDirection::Debit => self.amount_base,
            EntryDirection::Credit => -self.amount_base,
        }
    }
}

// ── Clone for AuditActor (needed for reversal) ───────────────────────────────

impl Clone for AuditActor {
    fn clone(&self) -> Self {
        Self {
            user_id: self.user_id.clone(),
            session_id: self.session_id,
            ip_address: self.ip_address.clone(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn test_actor() -> AuditActor {
        AuditActor {
            user_id: "test-user".to_string(),
            session_id: Uuid::new_v4(),
            ip_address: Some("127.0.0.1".to_string()),
        }
    }

    fn make_engine() -> LedgerEngine {
        LedgerEngine::new_bas2024()
    }

    fn make_balanced_entries(journal_id: Uuid, amount: Decimal) -> Vec<LedgerEntry> {
        vec![
            LedgerEntry::debit(
                journal_id,
                "1930",
                amount,
                "SEK",
                dec!(1.0),
                "Bank inbetalning",
                "2024-01",
            ),
            LedgerEntry::credit(
                journal_id,
                "3110",
                amount,
                "SEK",
                dec!(1.0),
                "Tjänsteintäkt",
                "2024-01",
            ),
        ]
    }

    #[test]
    fn post_balanced_journal_succeeds() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let entries = make_balanced_entries(tmp_id, dec!(10000.00));

        let result = engine.post_journal(
            "Tjänsteintäkt januari",
            Utc::now(),
            entries,
            "user1",
            1.0,
            None,
            test_actor(),
        );
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    }

    #[test]
    fn post_unbalanced_journal_fails() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let mut entries = make_balanced_entries(tmp_id, dec!(10000.00));
        // Intentionally create imbalance
        entries[0].amount_base = dec!(9999.00);

        let result = engine.post_journal(
            "Obalanserad journal",
            Utc::now(),
            entries,
            "user1",
            1.0,
            None,
            test_actor(),
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("unbalanced") || err_str.contains("Unbalanced"),
            "Expected unbalanced error, got: {}",
            err_str
        );
    }

    #[test]
    fn post_empty_journal_fails() {
        let engine = make_engine();
        let result = engine.post_journal(
            "Tom journal",
            Utc::now(),
            vec![],
            "user1",
            1.0,
            None,
            test_actor(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn invalid_account_code_fails() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let entries = vec![
            LedgerEntry::debit(tmp_id, "9999", dec!(100), "SEK", dec!(1), "test", "2024-01"),
            LedgerEntry::credit(tmp_id, "9998", dec!(100), "SEK", dec!(1), "test", "2024-01"),
        ];
        let result = engine.post_journal(
            "Ogiltiga konton",
            Utc::now(),
            entries,
            "user1",
            1.0,
            None,
            test_actor(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn get_balance_after_posting() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let entries = make_balanced_entries(tmp_id, dec!(5000.00));

        engine
            .post_journal("Test", Utc::now(), entries, "user1", 1.0, None, test_actor())
            .unwrap();

        let balance = engine.get_balance("1930", "2024-01").unwrap();
        assert_eq!(balance.balance, dec!(5000.00));
        assert_eq!(balance.debit_total, dec!(5000.00));
        assert_eq!(balance.credit_total, dec!(0));
    }

    #[test]
    fn trial_balance_is_balanced_after_valid_posting() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let entries = make_balanced_entries(tmp_id, dec!(7500.00));

        engine
            .post_journal("Test", Utc::now(), entries, "user1", 1.0, None, test_actor())
            .unwrap();

        let tb = engine.get_trial_balance("2024-01").unwrap();
        assert!(tb.is_balanced, "Trial balance should be balanced");
        assert_eq!(tb.debit_total, tb.credit_total);
    }

    #[test]
    fn reversal_zeroes_balance() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let entries = make_balanced_entries(tmp_id, dec!(3000.00));

        let journal_id = engine
            .post_journal(
                "Tjänsteintäkt",
                Utc::now(),
                entries,
                "user1",
                1.0,
                None,
                test_actor(),
            )
            .unwrap();

        engine
            .reverse_journal(
                journal_id,
                "Reversal av tjänsteintäkt",
                Utc::now(),
                "user1",
                test_actor(),
            )
            .unwrap();

        let balance = engine.get_balance("1930", "2024-01").unwrap();
        assert_eq!(balance.balance, dec!(0), "Balance should be zero after reversal");
    }

    #[test]
    fn replay_from_excludes_future_entries() {
        let engine = make_engine();
        let past = Utc::now() - chrono::Duration::hours(2);
        let now = Utc::now();

        let tmp_id = Uuid::new_v4();
        let entries = make_balanced_entries(tmp_id, dec!(1000.00));

        engine
            .post_journal("Past journal", now, entries, "user1", 1.0, None, test_actor())
            .unwrap();

        // Replay from before posting — should see no entries
        let snapshot = engine.replay_from(past).unwrap();
        assert_eq!(
            snapshot.journal_count, 0,
            "Snapshot from before posting should have 0 journals"
        );

        // Replay from after posting — should see the entry
        let snapshot_now = engine.replay_from(Utc::now() + chrono::Duration::seconds(1)).unwrap();
        assert_eq!(
            snapshot_now.journal_count, 1,
            "Snapshot from after posting should have 1 journal"
        );
    }

    #[test]
    fn get_journal_entries_returns_correct_entries() {
        let engine = make_engine();
        let tmp_id = Uuid::new_v4();
        let entries = make_balanced_entries(tmp_id, dec!(2500.00));

        let journal_id = engine
            .post_journal(
                "Hämta entries",
                Utc::now(),
                entries,
                "user1",
                1.0,
                None,
                test_actor(),
            )
            .unwrap();

        let fetched = engine.get_journal_entries(journal_id).unwrap();
        assert_eq!(fetched.len(), 2);
        assert!(fetched.iter().any(|e| e.account_code == "1930"));
        assert!(fetched.iter().any(|e| e.account_code == "3110"));
    }
}
