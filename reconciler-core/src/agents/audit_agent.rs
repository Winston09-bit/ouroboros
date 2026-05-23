// audit_agent.rs — Audit Preparation Agent
//
// Bygger kompletta revisionspaket, verifierar täckning, genererar
// trial balance, exporterar revision-ready material och förbereder
// svar på de vanligaste revisorfrågorna.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared domain types (reused from other modules via a common crate in prod)
// ---------------------------------------------------------------------------

/// A posted general-ledger entry.
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub description: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub reference: String,
    pub transaction_id: Option<Uuid>,
    pub posted_at: DateTime<Utc>,
    pub receipt_attached: bool,
    pub approved_by: Option<String>,
}

impl LedgerEntry {
    pub fn net(&self) -> Decimal {
        self.debit - self.credit
    }
}

/// A financial transaction (source document).
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub company_id: Uuid,
    pub date: NaiveDate,
    pub amount: Decimal,
    pub currency: String,
    pub description: String,
    pub vendor_name: Option<String>,
    pub receipt_attached: bool,
    pub approved_by: Option<String>,
    pub account_code: String,
    pub vat_code: String,
    pub is_intercompany: bool,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Full revision package for a company and period.
#[derive(Debug, Clone)]
pub struct AuditPack {
    pub id: Uuid,
    pub company_id: Uuid,
    pub period: String,
    pub ledger_entries: Vec<LedgerEntry>,
    pub transactions: Vec<Transaction>,
    pub trial_balance: TrialBalance,
    pub completeness_report: CompletenessReport,
    pub auditor_questions: Vec<AuditorQuestion>,
    pub generated_at: DateTime<Utc>,
    pub integrity_hash: String,
}

/// Trial balance: all accounts with opening balance, movements, closing balance.
#[derive(Debug, Clone)]
pub struct TrialBalance {
    pub period: String,
    pub accounts: Vec<TrialBalanceRow>,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    pub is_balanced: bool,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TrialBalanceRow {
    pub account_code: String,
    pub account_name: String,
    pub opening_balance: Decimal,
    pub period_debits: Decimal,
    pub period_credits: Decimal,
    pub closing_balance: Decimal,
}

impl TrialBalanceRow {
    pub fn net_movement(&self) -> Decimal {
        self.period_debits - self.period_credits
    }
}

/// Completeness report: which transactions have all required supporting docs.
#[derive(Debug, Clone)]
pub struct CompletenessReport {
    pub total_transactions: usize,
    pub transactions_with_receipt: usize,
    pub transactions_with_approval: usize,
    pub missing_receipt_ids: Vec<Uuid>,
    pub missing_approval_ids: Vec<Uuid>,
    pub completeness_pct: f64,
}

impl CompletenessReport {
    pub fn full_coverage(&self) -> bool {
        self.completeness_pct >= 100.0
    }
}

/// Revision-ready export manifest.
#[derive(Debug, Clone)]
pub struct AuditExport {
    pub pack_id: Uuid,
    pub manifest: Vec<ExportItem>,
    pub integrity_hash: String,
    pub total_transactions: usize,
    pub total_amount: Decimal,
    pub completeness_pct: f64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExportItem {
    pub item_type: ExportItemType,
    pub filename: String,
    pub description: String,
    pub record_count: usize,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportItemType {
    GeneralLedger,
    TrialBalance,
    Transactions,
    VatReport,
    SupportingDocumentManifest,
    AuditorQuestionsJson,
    IntegrityManifest,
}

impl fmt::Display for ExportItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ExportItemType::GeneralLedger => "GENERAL_LEDGER",
            ExportItemType::TrialBalance => "TRIAL_BALANCE",
            ExportItemType::Transactions => "TRANSACTIONS",
            ExportItemType::VatReport => "VAT_REPORT",
            ExportItemType::SupportingDocumentManifest => "SUPPORTING_DOCS",
            ExportItemType::AuditorQuestionsJson => "AUDITOR_QUESTIONS",
            ExportItemType::IntegrityManifest => "INTEGRITY_MANIFEST",
        };
        write!(f, "{}", s)
    }
}

/// A question the auditor is likely to ask, with a prepared response.
#[derive(Debug, Clone)]
pub struct AuditorQuestion {
    pub id: Uuid,
    pub priority: QuestionPriority,
    pub category: QuestionCategory,
    pub question: String,
    pub related_transaction_ids: Vec<Uuid>,
    pub suggested_response: String,
    pub supporting_documents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuestionPriority {
    Low,
    Medium,
    High,
}

impl fmt::Display for QuestionPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionCategory {
    MissingDocumentation,
    UnusualTransaction,
    RelatedParty,
    VatCompliance,
    InternalControls,
    GoingConcern,
    Completeness,
    Intercompany,
}

impl fmt::Display for QuestionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ---------------------------------------------------------------------------
// Threshold configuration
// ---------------------------------------------------------------------------

struct AuditThresholds {
    /// Single transaction above this = unusual / requires note
    unusual_transaction_amount: Decimal,
    /// Top-N largest transactions are always highlighted
    top_n_highlight: usize,
    /// Percentage completeness below which we raise a Completeness flag
    completeness_warning_pct: f64,
    /// Trial balance tolerance (rounding)
    balance_tolerance: Decimal,
}

impl Default for AuditThresholds {
    fn default() -> Self {
        AuditThresholds {
            unusual_transaction_amount: dec!(100_000),
            top_n_highlight: 10,
            completeness_warning_pct: 95.0,
            balance_tolerance: dec!(0.01),
        }
    }
}

// ---------------------------------------------------------------------------
// Main agent struct
// ---------------------------------------------------------------------------

pub struct AuditAgent {
    thresholds: AuditThresholds,
}

impl AuditAgent {
    pub fn new() -> Self {
        AuditAgent {
            thresholds: AuditThresholds::default(),
        }
    }

    pub fn with_unusual_threshold(mut self, amount: Decimal) -> Self {
        self.thresholds.unusual_transaction_amount = amount;
        self
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Build a complete audit pack for a company and period.
    pub async fn build_audit_pack(
        &self,
        company_id: Uuid,
        period: &str,
        ledger_entries: Vec<LedgerEntry>,
        transactions: Vec<Transaction>,
    ) -> AuditPack {
        let completeness = self.verify_completeness(&transactions);
        let trial_bal = self.trial_balance(&ledger_entries, period);

        // Build preliminary pack (without hash yet)
        let mut pack = AuditPack {
            id: Uuid::new_v4(),
            company_id,
            period: period.to_owned(),
            ledger_entries: ledger_entries.clone(),
            transactions: transactions.clone(),
            trial_balance: trial_bal,
            completeness_report: completeness,
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: String::new(),
        };

        pack.auditor_questions = self.identify_auditor_questions(&pack);

        // Now compute the integrity hash over the assembled pack
        pack.integrity_hash = self.hash_pack(&pack);

        pack
    }

    /// Check all transactions for completeness (receipt + approval).
    pub fn verify_completeness(&self, txns: &[Transaction]) -> CompletenessReport {
        let total = txns.len();
        let mut with_receipt = 0usize;
        let mut with_approval = 0usize;
        let mut missing_receipt = Vec::new();
        let mut missing_approval = Vec::new();

        for txn in txns {
            if txn.receipt_attached {
                with_receipt += 1;
            } else {
                missing_receipt.push(txn.id);
            }
            if txn.approved_by.is_some() {
                with_approval += 1;
            } else {
                missing_approval.push(txn.id);
            }
        }

        let completeness_pct = if total == 0 {
            100.0
        } else {
            // Completeness = proportion that have both receipt AND approval
            let fully_complete = txns
                .iter()
                .filter(|t| t.receipt_attached && t.approved_by.is_some())
                .count();
            fully_complete as f64 / total as f64 * 100.0
        };

        CompletenessReport {
            total_transactions: total,
            transactions_with_receipt: with_receipt,
            transactions_with_approval: with_approval,
            missing_receipt_ids: missing_receipt,
            missing_approval_ids: missing_approval,
            completeness_pct,
        }
    }

    /// Generate a trial balance from ledger entries for the given period.
    pub fn trial_balance(&self, entries: &[LedgerEntry], period: &str) -> TrialBalance {
        // Aggregate by account_code
        let mut rows: BTreeMap<String, (String, Decimal, Decimal)> = BTreeMap::new();

        for entry in entries {
            let row = rows
                .entry(entry.account_code.clone())
                .or_insert((entry.account_name.clone(), Decimal::ZERO, Decimal::ZERO));
            row.1 += entry.debit;
            row.2 += entry.credit;
        }

        let mut account_rows: Vec<TrialBalanceRow> = rows
            .into_iter()
            .map(|(code, (name, debits, credits))| {
                let closing = debits - credits;
                TrialBalanceRow {
                    account_code: code,
                    account_name: name,
                    opening_balance: Decimal::ZERO, // would come from prior period in prod
                    period_debits: debits,
                    period_credits: credits,
                    closing_balance: closing,
                }
            })
            .collect();

        // Sort by account code (BAS plan order)
        account_rows.sort_by(|a, b| a.account_code.cmp(&b.account_code));

        let total_debits: Decimal = account_rows.iter().map(|r| r.period_debits).sum();
        let total_credits: Decimal = account_rows.iter().map(|r| r.period_credits).sum();

        let diff = (total_debits - total_credits).abs();
        let is_balanced = diff <= self.thresholds.balance_tolerance;

        TrialBalance {
            period: period.to_owned(),
            accounts: account_rows,
            total_debits,
            total_credits,
            is_balanced,
            generated_at: Utc::now(),
        }
    }

    /// Build a revision-ready export manifest from an audit pack.
    pub fn export_for_auditor(&self, pack: &AuditPack) -> AuditExport {
        let total_amount: Decimal = pack.transactions.iter().map(|t| t.amount).sum();

        let mut items: Vec<ExportItem> = Vec::new();

        // General ledger export
        items.push(self.build_export_item(
            ExportItemType::GeneralLedger,
            format!("general_ledger_{}.json", pack.period),
            "Full general ledger for the period".to_owned(),
            pack.ledger_entries.len(),
            &serialize_ledger_entries(&pack.ledger_entries),
        ));

        // Trial balance
        items.push(self.build_export_item(
            ExportItemType::TrialBalance,
            format!("trial_balance_{}.json", pack.period),
            "Trial balance with all accounts".to_owned(),
            pack.trial_balance.accounts.len(),
            &serialize_trial_balance(&pack.trial_balance),
        ));

        // Transaction list
        items.push(self.build_export_item(
            ExportItemType::Transactions,
            format!("transactions_{}.json", pack.period),
            "All transactions for the period".to_owned(),
            pack.transactions.len(),
            &serialize_transactions(&pack.transactions),
        ));

        // Supporting documents manifest
        let missing_receipts = &pack.completeness_report.missing_receipt_ids;
        items.push(self.build_export_item(
            ExportItemType::SupportingDocumentManifest,
            format!("supporting_docs_manifest_{}.json", pack.period),
            format!(
                "Document coverage: {:.1}% ({} missing receipts)",
                pack.completeness_report.completeness_pct,
                missing_receipts.len()
            ),
            pack.completeness_report.total_transactions,
            &format!("{} documents checked", pack.completeness_report.total_transactions),
        ));

        // Auditor Q&A
        items.push(self.build_export_item(
            ExportItemType::AuditorQuestionsJson,
            format!("auditor_questions_{}.json", pack.period),
            format!(
                "{} anticipated auditor questions with prepared responses",
                pack.auditor_questions.len()
            ),
            pack.auditor_questions.len(),
            &serialize_auditor_questions(&pack.auditor_questions),
        ));

        // Integrity manifest
        let integrity_data = format!(
            "pack_id={} hash={} period={}",
            pack.id, pack.integrity_hash, pack.period
        );
        items.push(self.build_export_item(
            ExportItemType::IntegrityManifest,
            format!("integrity_{}.txt", pack.period),
            "Cryptographic integrity manifest".to_owned(),
            1,
            &integrity_data,
        ));

        AuditExport {
            pack_id: pack.id,
            manifest: items,
            integrity_hash: pack.integrity_hash.clone(),
            total_transactions: pack.transactions.len(),
            total_amount,
            completeness_pct: pack.completeness_report.completeness_pct,
            generated_at: Utc::now(),
        }
    }

    /// Compute SHA-256 over canonical pack fields.
    pub fn hash_pack(&self, pack: &AuditPack) -> String {
        let mut hasher = Sha256::new();

        // Pack identity
        hasher.update(pack.id.as_bytes());
        hasher.update(pack.company_id.as_bytes());
        hasher.update(pack.period.as_bytes());

        // Ledger entries (sorted by id for determinism)
        let mut entry_ids: Vec<Uuid> = pack.ledger_entries.iter().map(|e| e.id).collect();
        entry_ids.sort();
        for id in &entry_ids {
            hasher.update(id.as_bytes());
        }

        // Transaction ids (sorted)
        let mut txn_ids: Vec<Uuid> = pack.transactions.iter().map(|t| t.id).collect();
        txn_ids.sort();
        for id in &txn_ids {
            hasher.update(id.as_bytes());
        }

        // Financial totals
        let tb_debit = pack.trial_balance.total_debits.to_string();
        let tb_credit = pack.trial_balance.total_credits.to_string();
        hasher.update(tb_debit.as_bytes());
        hasher.update(tb_credit.as_bytes());

        // Timestamp (truncated to second for stability)
        hasher.update(pack.generated_at.timestamp().to_string().as_bytes());

        format!("{:x}", hasher.finalize())
    }

    /// Identify the questions a competent auditor is likely to raise.
    pub fn identify_auditor_questions(&self, pack: &AuditPack) -> Vec<AuditorQuestion> {
        let mut questions: Vec<AuditorQuestion> = Vec::new();

        // --- Q1: Missing receipts ---
        let missing = &pack.completeness_report.missing_receipt_ids;
        if !missing.is_empty() {
            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: if missing.len() > 10 {
                    QuestionPriority::High
                } else {
                    QuestionPriority::Medium
                },
                category: QuestionCategory::MissingDocumentation,
                question: format!(
                    "{} transactions are missing original receipts or invoices. Can you provide these?",
                    missing.len()
                ),
                related_transaction_ids: missing.clone(),
                suggested_response: format!(
                    "We have {missing_count} transactions without attached receipts. \
                    We will obtain copies from vendors for those still recoverable. \
                    Transactions older than 3 months where the original is lost will be \
                    replaced with a statutory declaration and bank statement confirmation.",
                    missing_count = missing.len()
                ),
                supporting_documents: vec!["bank_statements.pdf".to_owned(), "vendor_emails.pdf".to_owned()],
            });
        }

        // --- Q2: Missing approvals ---
        let missing_approvals = &pack.completeness_report.missing_approval_ids;
        if !missing_approvals.is_empty() {
            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: QuestionPriority::Medium,
                category: QuestionCategory::InternalControls,
                question: format!(
                    "{} transactions lack documented approval. What is your authorization process?",
                    missing_approvals.len()
                ),
                related_transaction_ids: missing_approvals.clone(),
                suggested_response: "Our authorization policy requires sign-off for all transactions \
                    above SEK 5,000. Smaller transactions may be approved via our expense system. \
                    We are retrospectively adding digital approvals for the flagged items.".to_owned(),
                supporting_documents: vec!["authorization_policy.pdf".to_owned()],
            });
        }

        // --- Q3: Trial balance imbalance ---
        if !pack.trial_balance.is_balanced {
            let diff =
                (pack.trial_balance.total_debits - pack.trial_balance.total_credits).abs();
            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: QuestionPriority::High,
                category: QuestionCategory::Completeness,
                question: format!(
                    "The trial balance is out of balance by {:.2} SEK. What is the cause?",
                    diff
                ),
                related_transaction_ids: vec![],
                suggested_response:
                    "We are investigating the imbalance. Likely causes: a late journal entry, \
                    a currency rounding difference, or an import error from the payment processor. \
                    A correcting journal will be posted once the root cause is identified.".to_owned(),
                supporting_documents: vec!["trial_balance.xlsx".to_owned()],
            });
        }

        // --- Q4: Unusual / large transactions ---
        let unusual_txns: Vec<&Transaction> = pack
            .transactions
            .iter()
            .filter(|t| t.amount >= self.thresholds.unusual_transaction_amount)
            .collect();

        if !unusual_txns.is_empty() {
            // Sort by amount desc, take top N
            let mut sorted = unusual_txns.clone();
            sorted.sort_by(|a, b| b.amount.cmp(&a.amount));
            let top: Vec<&Transaction> = sorted
                .into_iter()
                .take(self.thresholds.top_n_highlight)
                .collect();

            let ids: Vec<Uuid> = top.iter().map(|t| t.id).collect();
            let total: Decimal = top.iter().map(|t| t.amount).sum();

            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: QuestionPriority::High,
                category: QuestionCategory::UnusualTransaction,
                question: format!(
                    "{} transactions exceed {:.0} SEK (total: {:.2} SEK). \
                    Please provide business justification for each.",
                    top.len(),
                    self.thresholds.unusual_transaction_amount,
                    total
                ),
                related_transaction_ids: ids,
                suggested_response:
                    "These transactions represent significant capital expenditures, major \
                    vendor payments, and inter-company settlements as approved by the board. \
                    Board minutes, purchase orders, and contracts are attached.".to_owned(),
                supporting_documents: vec![
                    "board_minutes.pdf".to_owned(),
                    "contracts.pdf".to_owned(),
                    "purchase_orders.pdf".to_owned(),
                ],
            });
        }

        // --- Q5: Intercompany transactions ---
        let interco_txns: Vec<Uuid> = pack
            .transactions
            .iter()
            .filter(|t| t.is_intercompany)
            .map(|t| t.id)
            .collect();

        if !interco_txns.is_empty() {
            let interco_total: Decimal = pack
                .transactions
                .iter()
                .filter(|t| t.is_intercompany)
                .map(|t| t.amount)
                .sum();

            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: QuestionPriority::Medium,
                category: QuestionCategory::Intercompany,
                question: format!(
                    "{} intercompany transactions totalling {:.2} SEK were identified. \
                    Are these at arm's length? Is there a transfer pricing policy?",
                    interco_txns.len(),
                    interco_total
                ),
                related_transaction_ids: interco_txns,
                suggested_response: "All intercompany transactions are conducted at arm's length \
                    in accordance with our transfer pricing policy (attached). The transactions \
                    represent management fees, shared service charges, and intra-group loans \
                    documented by signed agreements.".to_owned(),
                supporting_documents: vec![
                    "transfer_pricing_policy.pdf".to_owned(),
                    "intercompany_agreements.pdf".to_owned(),
                ],
            });
        }

        // --- Q6: Completeness below warning threshold ---
        if pack.completeness_report.completeness_pct
            < self.thresholds.completeness_warning_pct
        {
            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: QuestionPriority::High,
                category: QuestionCategory::Completeness,
                question: format!(
                    "Documentation completeness is only {:.1}%. What steps are being taken to address this?",
                    pack.completeness_report.completeness_pct
                ),
                related_transaction_ids: pack.completeness_report.missing_receipt_ids.clone(),
                suggested_response: "We acknowledge the documentation gaps. A remediation plan \
                    has been initiated to obtain missing documents. Internal controls have been \
                    strengthened to prevent recurrence in future periods.".to_owned(),
                supporting_documents: vec!["remediation_plan.pdf".to_owned()],
            });
        }

        // --- Q7: Account codes with unusual activity ---
        let unusual_accounts = self.detect_unusual_account_activity(&pack.ledger_entries);
        for (account, reason) in unusual_accounts {
            questions.push(AuditorQuestion {
                id: Uuid::new_v4(),
                priority: QuestionPriority::Medium,
                category: QuestionCategory::UnusualTransaction,
                question: format!(
                    "Account {} shows unusual activity: {}. Can you explain?",
                    account, reason
                ),
                related_transaction_ids: vec![],
                suggested_response: format!(
                    "The unusual activity in account {} is attributable to a one-time event \
                    during the period. We can provide detailed sub-ledger breakdown upon request.",
                    account
                ),
                supporting_documents: vec!["sub_ledger_detail.xlsx".to_owned()],
            });
        }

        // Sort by priority (High first)
        questions.sort_by(|a, b| b.priority.cmp(&a.priority));
        questions
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn build_export_item(
        &self,
        item_type: ExportItemType,
        filename: String,
        description: String,
        record_count: usize,
        content: &str,
    ) -> ExportItem {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        ExportItem {
            item_type,
            filename,
            description,
            record_count,
            checksum,
        }
    }

    /// Detect accounts with potentially unusual activity patterns.
    fn detect_unusual_account_activity(
        &self,
        entries: &[LedgerEntry],
    ) -> Vec<(String, String)> {
        let mut issues = Vec::new();

        // Group by account code
        let mut account_map: HashMap<String, Vec<&LedgerEntry>> = HashMap::new();
        for e in entries {
            account_map.entry(e.account_code.clone()).or_default().push(e);
        }

        for (account, entries) in &account_map {
            let total_debit: Decimal = entries.iter().map(|e| e.debit).sum();
            let total_credit: Decimal = entries.iter().map(|e| e.credit).sum();
            let net = (total_debit - total_credit).abs();

            // Large credit balance on expense account (BAS 5xxx, 6xxx, 7xxx)
            if account.starts_with('5') || account.starts_with('6') || account.starts_with('7') {
                if total_credit > total_debit && net > dec!(10_000) {
                    issues.push((
                        account.clone(),
                        format!(
                            "Expense account has net credit balance of {:.2} SEK (unusual)",
                            net
                        ),
                    ));
                }
            }

            // Large debit balance on liability account (BAS 2xxx)
            if account.starts_with('2') {
                if total_debit > total_credit && net > dec!(50_000) {
                    issues.push((
                        account.clone(),
                        format!(
                            "Liability account has net debit balance of {:.2} SEK (unusual)",
                            net
                        ),
                    ));
                }
            }

            // Large debit on revenue account (BAS 3xxx)
            if account.starts_with('3') {
                if total_debit > total_credit && net > dec!(10_000) {
                    issues.push((
                        account.clone(),
                        format!(
                            "Revenue account has net debit balance of {:.2} SEK – possible reversal or error",
                            net
                        ),
                    ));
                }
            }

            // Very high entry count on a single account (potential journal manipulation)
            if entries.len() > 200 {
                issues.push((
                    account.clone(),
                    format!(
                        "Unusual number of journal entries ({}) – verify for manual adjustments",
                        entries.len()
                    ),
                ));
            }
        }

        issues
    }
}

impl Default for AuditAgent {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers (lightweight; prod would use serde_json)
// ---------------------------------------------------------------------------

fn serialize_ledger_entries(entries: &[LedgerEntry]) -> String {
    let rows: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                r#"{{"id":"{}","account":"{}","description":"{}","debit":{},"credit":{},"reference":"{}","posted_at":"{}"}}"#,
                e.id,
                e.account_code,
                e.description.replace('"', r#"\""#),
                e.debit,
                e.credit,
                e.reference.replace('"', r#"\""#),
                e.posted_at.to_rfc3339()
            )
        })
        .collect();
    format!("[{}]", rows.join(",\n"))
}

fn serialize_transactions(txns: &[Transaction]) -> String {
    let rows: Vec<String> = txns
        .iter()
        .map(|t| {
            format!(
                r#"{{"id":"{}","date":"{}","amount":{},"currency":"{}","description":"{}","account":"{}","vat_code":"{}","receipt_attached":{},"approved_by":{}}}"#,
                t.id,
                t.date,
                t.amount,
                t.currency,
                t.description.replace('"', r#"\""#),
                t.account_code,
                t.vat_code,
                t.receipt_attached,
                t.approved_by
                    .as_deref()
                    .map(|s| format!(r#""{}""#, s))
                    .unwrap_or_else(|| "null".to_owned())
            )
        })
        .collect();
    format!("[{}]", rows.join(",\n"))
}

fn serialize_trial_balance(tb: &TrialBalance) -> String {
    let rows: Vec<String> = tb
        .accounts
        .iter()
        .map(|r| {
            format!(
                r#"{{"account":"{}","name":"{}","debits":{},"credits":{},"closing":{}}}"#,
                r.account_code,
                r.account_name.replace('"', r#"\""#),
                r.period_debits,
                r.period_credits,
                r.closing_balance
            )
        })
        .collect();
    format!(
        r#"{{"period":"{}","total_debits":{},"total_credits":{},"is_balanced":{},"accounts":[{}]}}"#,
        tb.period, tb.total_debits, tb.total_credits, tb.is_balanced,
        rows.join(",\n")
    )
}

fn serialize_auditor_questions(questions: &[AuditorQuestion]) -> String {
    let rows: Vec<String> = questions
        .iter()
        .map(|q| {
            format!(
                r#"{{"id":"{}","priority":"{}","category":"{}","question":"{}","related_count":{},"suggested_response":"{}"}}"#,
                q.id,
                q.priority,
                q.category,
                q.question.replace('"', r#"\""#),
                q.related_transaction_ids.len(),
                q.suggested_response.replace('"', r#"\""#)
            )
        })
        .collect();
    format!("[{}]", rows.join(",\n"))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_entry(account: &str, name: &str, debit: Decimal, credit: Decimal) -> LedgerEntry {
        LedgerEntry {
            id: Uuid::new_v4(),
            account_code: account.to_owned(),
            account_name: name.to_owned(),
            description: "Test entry".to_owned(),
            debit,
            credit,
            reference: "REF-001".to_owned(),
            transaction_id: None,
            posted_at: Utc::now(),
            receipt_attached: true,
            approved_by: Some("manager@company.se".to_owned()),
        }
    }

    fn make_txn(amount: Decimal, receipt: bool, approved: bool) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            amount,
            currency: "SEK".to_owned(),
            description: "Test transaction".to_owned(),
            vendor_name: Some("Test Vendor AB".to_owned()),
            receipt_attached: receipt,
            approved_by: if approved {
                Some("cfo@company.se".to_owned())
            } else {
                None
            },
            account_code: "6540".to_owned(),
            vat_code: "MP1I".to_owned(),
            is_intercompany: false,
            notes: None,
        }
    }

    #[test]
    fn test_trial_balance_balanced() {
        let agent = AuditAgent::new();
        let entries = vec![
            make_entry("1510", "Kundfordringar", dec!(10000), dec!(0)),
            make_entry("3000", "Försäljning", dec!(0), dec!(10000)),
        ];
        let tb = agent.trial_balance(&entries, "2024-01");
        assert!(tb.is_balanced, "Expected balanced trial balance");
        assert_eq!(tb.total_debits, dec!(10000));
        assert_eq!(tb.total_credits, dec!(10000));
    }

    #[test]
    fn test_trial_balance_unbalanced() {
        let agent = AuditAgent::new();
        let entries = vec![
            make_entry("1510", "Kundfordringar", dec!(10000), dec!(0)),
            make_entry("3000", "Försäljning", dec!(0), dec!(9999)), // 1 SEK off
        ];
        let tb = agent.trial_balance(&entries, "2024-01");
        assert!(!tb.is_balanced);
    }

    #[test]
    fn test_trial_balance_multi_accounts_aggregated() {
        let agent = AuditAgent::new();
        let entries = vec![
            make_entry("6540", "IT", dec!(500), dec!(0)),
            make_entry("6540", "IT", dec!(500), dec!(0)),
            make_entry("2440", "Leverantörsskulder", dec!(0), dec!(1000)),
        ];
        let tb = agent.trial_balance(&entries, "2024-01");
        assert!(tb.is_balanced);
        let it_row = tb.accounts.iter().find(|r| r.account_code == "6540").unwrap();
        assert_eq!(it_row.period_debits, dec!(1000));
    }

    #[test]
    fn test_verify_completeness_full() {
        let agent = AuditAgent::new();
        let txns = vec![
            make_txn(dec!(1000), true, true),
            make_txn(dec!(500), true, true),
        ];
        let report = agent.verify_completeness(&txns);
        assert_eq!(report.completeness_pct, 100.0);
        assert!(report.missing_receipt_ids.is_empty());
    }

    #[test]
    fn test_verify_completeness_partial() {
        let agent = AuditAgent::new();
        let txns = vec![
            make_txn(dec!(1000), true, true),
            make_txn(dec!(500), false, false),
            make_txn(dec!(200), false, true),
        ];
        let report = agent.verify_completeness(&txns);
        assert!(report.completeness_pct < 100.0);
        assert_eq!(report.missing_receipt_ids.len(), 2);
        assert_eq!(report.missing_approval_ids.len(), 1);
    }

    #[test]
    fn test_hash_pack_deterministic() {
        let agent = AuditAgent::new();
        let txn1 = make_txn(dec!(1000), true, true);
        let entry1 = make_entry("6540", "IT", dec!(1000), dec!(0));
        let entry2 = make_entry("2440", "AP", dec!(0), dec!(1000));
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            ledger_entries: vec![entry1.clone(), entry2.clone()],
            transactions: vec![txn1.clone()],
            trial_balance: agent.trial_balance(&[entry1, entry2], "2024-01"),
            completeness_report: agent.verify_completeness(&[txn1]),
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: String::new(),
        };
        let h1 = agent.hash_pack(&pack);
        let h2 = agent.hash_pack(&pack);
        assert_eq!(h1, h2, "Hash must be deterministic");
        assert_eq!(h1.len(), 64, "SHA-256 hex = 64 chars");
    }

    #[test]
    fn test_questions_missing_receipts() {
        let agent = AuditAgent::new();
        let txns = vec![
            make_txn(dec!(1000), false, true),
            make_txn(dec!(2000), false, true),
        ];
        let completeness = agent.verify_completeness(&txns);
        let entry1 = make_entry("6540", "IT", dec!(3000), dec!(0));
        let entry2 = make_entry("2440", "AP", dec!(0), dec!(3000));
        let tb = agent.trial_balance(&[entry1.clone(), entry2.clone()], "2024-01");
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            ledger_entries: vec![entry1, entry2],
            transactions: txns.clone(),
            trial_balance: tb,
            completeness_report: completeness,
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: String::new(),
        };
        let questions = agent.identify_auditor_questions(&pack);
        assert!(
            questions.iter().any(|q| q.category == QuestionCategory::MissingDocumentation),
            "Expected MissingDocumentation question"
        );
    }

    #[test]
    fn test_questions_unusual_amount() {
        let agent = AuditAgent::new();
        let txns = vec![make_txn(dec!(500_000), true, true)];
        let completeness = agent.verify_completeness(&txns);
        let entry1 = make_entry("6540", "IT", dec!(500_000), dec!(0));
        let entry2 = make_entry("2440", "AP", dec!(0), dec!(500_000));
        let tb = agent.trial_balance(&[entry1.clone(), entry2.clone()], "2024-01");
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            ledger_entries: vec![entry1, entry2],
            transactions: txns,
            trial_balance: tb,
            completeness_report: completeness,
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: String::new(),
        };
        let questions = agent.identify_auditor_questions(&pack);
        assert!(
            questions.iter().any(|q| q.category == QuestionCategory::UnusualTransaction),
            "Expected UnusualTransaction question for large amount"
        );
    }

    #[test]
    fn test_export_manifest_has_all_types() {
        let agent = AuditAgent::new();
        let entries = vec![
            make_entry("6540", "IT", dec!(1000), dec!(0)),
            make_entry("2440", "AP", dec!(0), dec!(1000)),
        ];
        let txns = vec![make_txn(dec!(1000), true, true)];
        let completeness = agent.verify_completeness(&txns);
        let tb = agent.trial_balance(&entries, "2024-01");
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            ledger_entries: entries,
            transactions: txns,
            trial_balance: tb,
            completeness_report: completeness,
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: "abc123".to_owned(),
        };
        let export = agent.export_for_auditor(&pack);
        let types: Vec<&ExportItemType> = export.manifest.iter().map(|i| &i.item_type).collect();
        assert!(types.contains(&&ExportItemType::GeneralLedger));
        assert!(types.contains(&&ExportItemType::TrialBalance));
        assert!(types.contains(&&ExportItemType::Transactions));
        assert!(types.contains(&&ExportItemType::SupportingDocumentManifest));
        assert!(types.contains(&&ExportItemType::AuditorQuestionsJson));
        assert!(types.contains(&&ExportItemType::IntegrityManifest));
    }

    #[test]
    fn test_export_checksums_are_hex() {
        let agent = AuditAgent::new();
        let entries = vec![make_entry("6540", "IT", dec!(500), dec!(500))];
        let txns = vec![make_txn(dec!(500), true, true)];
        let completeness = agent.verify_completeness(&txns);
        let tb = agent.trial_balance(&entries, "2024-01");
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            ledger_entries: entries,
            transactions: txns,
            trial_balance: tb,
            completeness_report: completeness,
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: "dummyhash".to_owned(),
        };
        let export = agent.export_for_auditor(&pack);
        for item in &export.manifest {
            assert_eq!(item.checksum.len(), 64, "Checksum for {} should be 64-char hex", item.filename);
            assert!(
                item.checksum.chars().all(|c| c.is_ascii_hexdigit()),
                "Checksum for {} is not valid hex", item.filename
            );
        }
    }

    #[test]
    fn test_intercompany_question_raised() {
        let agent = AuditAgent::new();
        let mut txn = make_txn(dec!(50_000), true, true);
        txn.is_intercompany = true;
        let txns = vec![txn];
        let completeness = agent.verify_completeness(&txns);
        let entries = vec![make_entry("6230", "Konsult", dec!(50_000), dec!(50_000))];
        let tb = agent.trial_balance(&entries, "2024-01");
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            ledger_entries: entries,
            transactions: txns,
            trial_balance: tb,
            completeness_report: completeness,
            auditor_questions: vec![],
            generated_at: Utc::now(),
            integrity_hash: String::new(),
        };
        let questions = agent.identify_auditor_questions(&pack);
        assert!(
            questions.iter().any(|q| q.category == QuestionCategory::Intercompany),
            "Expected Intercompany question"
        );
    }
}
