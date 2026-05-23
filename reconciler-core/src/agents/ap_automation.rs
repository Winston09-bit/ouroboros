// ap_automation.rs — AP (Accounts Payable) Automation Agent
//
// Autonom agent som hanterar leverantörsfakturor end-to-end:
// klassificering, duplikatkontroll, three-way matching, auto-bokning,
// godkännande-flöde och betalningsförslag.

use chrono::{Datelike, Days, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// External trait stubs (defined in other modules)
// ---------------------------------------------------------------------------

/// Represents a raw incoming invoice (populated by OCR / EDI / email parser).
#[derive(Debug, Clone)]
pub struct Invoice {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub vendor_name: String,
    pub vendor_vat_number: Option<String>,
    pub amount_excl_vat: Decimal,
    pub vat_amount: Decimal,
    pub currency: String,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,
    pub invoice_number: String,
    pub description: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub purchase_order_ref: Option<String>,
    pub raw_text: String,
}

impl Invoice {
    pub fn total_amount(&self) -> Decimal {
        self.amount_excl_vat + self.vat_amount
    }

    /// Fingerprint used for duplicate detection.
    pub fn dedup_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.vendor_id.as_bytes());
        hasher.update(self.total_amount().to_string().as_bytes());
        hasher.update(
            format!(
                "{}-{}",
                self.invoice_date.year(),
                self.invoice_date.month()
            )
            .as_bytes(),
        );
        hasher.update(self.invoice_number.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub vat_rate: Decimal,
    pub account_hint: Option<String>,
}

/// Purchase order as stored in ERP.
#[derive(Debug, Clone)]
pub struct PurchaseOrder {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub po_number: String,
    pub total_amount: Decimal,
    pub currency: String,
    pub status: PoStatus,
    pub delivery_confirmed: bool,
    pub line_items: Vec<PoLineItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoStatus {
    Open,
    PartiallyReceived,
    FullyReceived,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct PoLineItem {
    pub description: String,
    pub quantity_ordered: Decimal,
    pub quantity_received: Decimal,
    pub unit_price: Decimal,
}

/// Minimal ledger entry used for booking.
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub account_code: String,
    pub description: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub reference: String,
    pub posted_at: chrono::DateTime<Utc>,
}

/// Trait that any ERP / accounting back-end must implement.
#[async_trait::async_trait]
pub trait AccountingProvider: Send + Sync {
    /// Fetch all invoices already booked for a given vendor in a year-month.
    async fn get_booked_invoices(
        &self,
        vendor_id: Uuid,
        year: i32,
        month: u32,
    ) -> anyhow::Result<Vec<Invoice>>;

    /// Fetch purchase orders matching a reference string.
    async fn get_purchase_orders(
        &self,
        vendor_id: Uuid,
        po_ref: &str,
    ) -> anyhow::Result<Vec<PurchaseOrder>>;

    /// Post a double-entry booking to the ledger.
    async fn post_ledger_entry(&self, entry: LedgerEntry) -> anyhow::Result<Uuid>;

    /// Create an approval workflow request.
    async fn create_approval_request(
        &self,
        request: ApprovalRequest,
    ) -> anyhow::Result<Uuid>;

    /// Fetch all purchase orders for a vendor (used for three-way match scan).
    async fn list_open_purchase_orders(
        &self,
        vendor_id: Uuid,
    ) -> anyhow::Result<Vec<PurchaseOrder>>;
}

// ---------------------------------------------------------------------------
// Confidence engine
// ---------------------------------------------------------------------------

/// Produces confidence scores and combines individual signals.
pub struct ConfidenceEngine {
    /// Minimum confidence required for fully automatic booking.
    pub auto_book_threshold: f64,
}

impl ConfidenceEngine {
    pub fn new(auto_book_threshold: f64) -> Self {
        Self { auto_book_threshold }
    }

    /// Combine multiple per-check scores (weighted average).
    pub fn combine(&self, signals: &[(f64 /* score */, f64 /* weight */)]) -> f64 {
        if signals.is_empty() {
            return 0.0;
        }
        let (weighted_sum, total_weight): (f64, f64) =
            signals.iter().fold((0.0, 0.0), |(ws, tw), (s, w)| {
                (ws + s * w, tw + w)
            });
        if total_weight == 0.0 {
            0.0
        } else {
            (weighted_sum / total_weight).clamp(0.0, 1.0)
        }
    }

    pub fn should_auto_book(&self, confidence: f64) -> bool {
        confidence >= self.auto_book_threshold
    }
}

// ---------------------------------------------------------------------------
// Result / output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ApResult {
    pub invoice_id: Uuid,
    pub status: ApStatus,
    pub account_code: Option<String>,
    pub confidence: f64,
    pub auto_booked: bool,
    pub requires_approval: bool,
    pub payment_suggestion: Option<PaymentSuggestion>,
    pub audit_trail: Vec<AuditEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApStatus {
    AutoBooked,
    PendingApproval,
    Rejected,
    ManualRequired,
    Duplicate,
}

impl fmt::Display for ApStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApStatus::AutoBooked => write!(f, "AUTO_BOOKED"),
            ApStatus::PendingApproval => write!(f, "PENDING_APPROVAL"),
            ApStatus::Rejected => write!(f, "REJECTED"),
            ApStatus::ManualRequired => write!(f, "MANUAL_REQUIRED"),
            ApStatus::Duplicate => write!(f, "DUPLICATE"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaymentSuggestion {
    pub amount: Decimal,
    pub currency: String,
    pub due_date: NaiveDate,
    /// Suggested payment date optimised for cash-flow (pay as late as possible
    /// without incurring late fees or losing early-payment discounts).
    pub suggested_date: NaiveDate,
    pub bank_account: String,
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct AccountSuggestion {
    pub code: String,
    pub name: String,
    pub confidence: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub issues: Vec<VerificationIssue>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct VerificationIssue {
    pub kind: IssueKind,
    pub message: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueKind {
    MissingVatNumber,
    VatCalculationMismatch,
    DuplicateInvoice,
    UnknownVendor,
    AmountZero,
    PastDueDate,
    CurrencyMismatch,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub vendor_name: String,
    pub amount: Decimal,
    pub currency: String,
    pub suggested_account: Option<String>,
    pub reason: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BookingResult {
    pub ledger_entry_id: Uuid,
    pub account_code: String,
    pub amount: Decimal,
    pub posted_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub detail: String,
}

impl AuditEvent {
    fn agent(action: impl Into<String>, detail: impl Into<String>) -> Self {
        AuditEvent {
            timestamp: Utc::now(),
            actor: "ApAutomationAgent".into(),
            action: action.into(),
            detail: detail.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Vendor classification rules
// ---------------------------------------------------------------------------

/// One entry in the vendor classification rule-set.
struct ClassificationRule {
    /// Keywords matched case-insensitively against vendor name or description.
    keywords: Vec<&'static str>,
    account_code: &'static str,
    account_name: &'static str,
    base_confidence: f64,
}

/// Build the static rule table.  Rules are evaluated in order; first match wins.
fn classification_rules() -> Vec<ClassificationRule> {
    vec![
        // Cloud / hosting
        ClassificationRule {
            keywords: vec!["aws", "amazon web services", "gcp", "google cloud", "azure", "microsoft azure", "digitalocean", "hetzner", "cloudflare"],
            account_code: "6540",
            account_name: "IT-tjänster och molntjänster",
            base_confidence: 0.96,
        },
        // Travel – air & rail
        ClassificationRule {
            keywords: vec!["sj", "sj ab", "ryanair", "sas", "norwegian", "wizz", "easyjet", "lufthansa", "klm", "british airways", "flixbus", "vy"],
            account_code: "5800",
            account_name: "Resor och transport",
            base_confidence: 0.95,
        },
        // Hotels & accommodation
        ClassificationRule {
            keywords: vec!["hotel", "hotell", "airbnb", "booking.com", "elite hotel", "scandic", "radisson", "marriott", "hilton"],
            account_code: "5810",
            account_name: "Hotell och logi",
            base_confidence: 0.94,
        },
        // Food & restaurants
        ClassificationRule {
            keywords: vec!["ica", "coop", "lidl", "hemköp", "willys", "restaurang", "restaurant", "café", "mcdonalds", "subway", "pizza", "sushi", "lunch"],
            account_code: "6000",
            account_name: "Representation och mat",
            base_confidence: 0.90,
        },
        // Consultants
        ClassificationRule {
            keywords: vec!["konsult", "consultant", "consulting", "advisory", "accenture", "deloitte", "kpmg", "pwc", "ey ", "ernst & young"],
            account_code: "6230",
            account_name: "Konsulttjänster",
            base_confidence: 0.93,
        },
        // Marketing / advertising
        ClassificationRule {
            keywords: vec!["google ads", "meta ads", "facebook ads", "instagram ads", "linkedin ads", "twitter ads", "reklam", "marknadsföring", "advertising"],
            account_code: "6410",
            account_name: "Annonsering och marknadsföring",
            base_confidence: 0.93,
        },
        // Telecom
        ClassificationRule {
            keywords: vec!["tele2", "telia", "tre ", "telenor", "comviq", "bredband", "mobil", "telefoni"],
            account_code: "6211",
            account_name: "Telefoni och bredband",
            base_confidence: 0.95,
        },
        // Office supplies
        ClassificationRule {
            keywords: vec!["kontorsmaterial", "pappersindustri", "ikea", "staples", "officemax", "biltema", "rusta"],
            account_code: "6110",
            account_name: "Kontorsmaterial",
            base_confidence: 0.88,
        },
        // Software licenses (not cloud infra)
        ClassificationRule {
            keywords: vec!["license", "licens", "subscription", "prenumeration", "adobe", "figma", "slack", "atlassian", "jira", "github", "gitlab"],
            account_code: "6530",
            account_name: "Programvarulicenser",
            base_confidence: 0.92,
        },
        // Rent & facilities
        ClassificationRule {
            keywords: vec!["hyra", "rent", "fastighet", "lokalhyra", "leasing"],
            account_code: "5010",
            account_name: "Hyra lokaler",
            base_confidence: 0.91,
        },
    ]
}

/// Attempt to classify an invoice into a BAS account code.
fn classify_invoice(invoice: &Invoice) -> AccountSuggestion {
    let haystack = format!(
        "{} {}",
        invoice.vendor_name.to_lowercase(),
        invoice.description.to_lowercase()
    );

    let rules = classification_rules();
    for rule in &rules {
        let matched_keyword = rule
            .keywords
            .iter()
            .find(|&&kw| haystack.contains(kw));
        if let Some(kw) = matched_keyword {
            return AccountSuggestion {
                code: rule.account_code.to_owned(),
                name: rule.account_name.to_owned(),
                confidence: rule.base_confidence,
                reasoning: format!(
                    "Vendor/description matched keyword '{}' → BAS {}",
                    kw, rule.account_code
                ),
            };
        }
    }

    // Fallback: generic expense account
    AccountSuggestion {
        code: "6999".to_owned(),
        name: "Övriga externa kostnader (oklassificerade)".to_owned(),
        confidence: 0.40,
        reasoning: "No classification rule matched; manual review required.".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Three-way match logic
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ThreeWayMatchResult {
    pub matched: bool,
    pub purchase_order: Option<PurchaseOrder>,
    pub match_type: MatchType,
    pub discrepancies: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchType {
    /// Invoice amount ≈ PO amount, delivery confirmed.
    FullThreeWay,
    /// PO found, delivery not yet confirmed.
    TwoWay,
    /// Only vendor matches (PO ref missing or no match).
    VendorOnly,
    /// No match at all.
    None,
}

fn three_way_match(invoice: &Invoice, po: &PurchaseOrder) -> ThreeWayMatchResult {
    let mut discrepancies = Vec::new();

    // 1. Vendor match (should always hold if we found the PO)
    if invoice.vendor_id != po.vendor_id {
        discrepancies.push(format!(
            "Vendor ID mismatch: invoice={} po={}",
            invoice.vendor_id, po.vendor_id
        ));
    }

    // 2. Amount tolerance: accept ±2 %
    let tolerance = po.total_amount * Decimal::new(2, 2); // 0.02
    let amount_diff = (invoice.total_amount() - po.total_amount).abs();
    let amount_ok = amount_diff <= tolerance;
    if !amount_ok {
        discrepancies.push(format!(
            "Amount mismatch: invoice={} po={} diff={}",
            invoice.total_amount(),
            po.total_amount,
            amount_diff
        ));
    }

    // 3. Currency
    if invoice.currency != po.currency {
        discrepancies.push(format!(
            "Currency mismatch: invoice={} po={}",
            invoice.currency, po.currency
        ));
    }

    // 4. Delivery confirmation
    let delivery_ok = po.delivery_confirmed || po.status == PoStatus::FullyReceived;
    if !delivery_ok {
        discrepancies.push("Delivery not confirmed".to_owned());
    }

    let match_type = if discrepancies.is_empty() && delivery_ok {
        MatchType::FullThreeWay
    } else if amount_ok && invoice.currency == po.currency && !delivery_ok {
        MatchType::TwoWay
    } else if discrepancies.iter().all(|d| d.contains("Delivery")) {
        MatchType::TwoWay
    } else {
        MatchType::VendorOnly
    };

    let confidence = match match_type {
        MatchType::FullThreeWay => 0.97,
        MatchType::TwoWay => 0.82,
        MatchType::VendorOnly => 0.55,
        MatchType::None => 0.0,
    };

    ThreeWayMatchResult {
        matched: match_type == MatchType::FullThreeWay || match_type == MatchType::TwoWay,
        purchase_order: Some(po.clone()),
        match_type,
        discrepancies,
        confidence,
    }
}

// ---------------------------------------------------------------------------
// Main agent struct
// ---------------------------------------------------------------------------

pub struct ApAutomationAgent {
    pub confidence_engine: Arc<ConfidenceEngine>,
    pub erp_connector: Arc<dyn AccountingProvider>,
    /// Per-currency default bank accounts (IBAN or internal ref).
    pub bank_accounts: HashMap<String, String>,
    /// Approval required above this threshold.
    pub approval_threshold: Decimal,
    /// Company's default payment bank account.
    pub default_bank_account: String,
}

impl ApAutomationAgent {
    pub fn new(
        confidence_engine: Arc<ConfidenceEngine>,
        erp_connector: Arc<dyn AccountingProvider>,
        approval_threshold: Decimal,
        default_bank_account: impl Into<String>,
    ) -> Self {
        let mut bank_accounts = HashMap::new();
        bank_accounts.insert("SEK".to_owned(), "SE00 0000 0000 0000 0000 0001".to_owned());
        bank_accounts.insert("EUR".to_owned(), "SE00 0000 0000 0000 0000 0002".to_owned());
        bank_accounts.insert("USD".to_owned(), "SE00 0000 0000 0000 0000 0003".to_owned());

        ApAutomationAgent {
            confidence_engine,
            erp_connector,
            bank_accounts,
            approval_threshold,
            default_bank_account: default_bank_account.into(),
        }
    }

    // -----------------------------------------------------------------------
    // Public entry point
    // -----------------------------------------------------------------------

    /// Process an incoming invoice end-to-end.
    pub async fn process_invoice(&self, invoice: &Invoice) -> ApResult {
        let mut trail: Vec<AuditEvent> = Vec::new();
        trail.push(AuditEvent::agent(
            "RECEIVED",
            format!("Invoice {} from {}", invoice.invoice_number, invoice.vendor_name),
        ));

        // --- Step 1: Verification ---
        let verification = self.verify_invoice(invoice).await;
        trail.push(AuditEvent::agent(
            "VERIFIED",
            format!(
                "valid={} issues={}",
                verification.valid,
                verification.issues.len()
            ),
        ));

        // Hard reject on blocking issues
        let blocking: Vec<_> = verification
            .issues
            .iter()
            .filter(|i| i.blocking)
            .collect();

        if !blocking.is_empty() {
            let reasons: Vec<String> = blocking.iter().map(|i| i.message.clone()).collect();
            trail.push(AuditEvent::agent(
                "REJECTED",
                format!("Blocking issues: {}", reasons.join("; ")),
            ));

            // Duplicate is a special status
            let status = if blocking
                .iter()
                .any(|i| i.kind == IssueKind::DuplicateInvoice)
            {
                ApStatus::Duplicate
            } else {
                ApStatus::Rejected
            };

            return ApResult {
                invoice_id: invoice.id,
                status,
                account_code: None,
                confidence: 0.0,
                auto_booked: false,
                requires_approval: false,
                payment_suggestion: None,
                audit_trail: trail,
            };
        }

        // --- Step 2: Account classification ---
        let account_suggestion = self.suggest_account_code(invoice);
        trail.push(AuditEvent::agent(
            "CLASSIFIED",
            format!(
                "account={} ({}) confidence={:.2}",
                account_suggestion.code,
                account_suggestion.name,
                account_suggestion.confidence
            ),
        ));

        // --- Step 3: PO matching ---
        let po_match = self.match_purchase_order(invoice).await;
        let po_confidence = if let Some(ref po) = po_match {
            let result = three_way_match(invoice, po);
            trail.push(AuditEvent::agent(
                "PO_MATCHED",
                format!(
                    "po={} type={:?} discrepancies={}",
                    po.po_number,
                    result.match_type,
                    result.discrepancies.len()
                ),
            ));
            result.confidence
        } else {
            trail.push(AuditEvent::agent("PO_MATCH", "No matching PO found"));
            0.5 // neutral — no PO is common for service invoices
        };

        // --- Step 4: Combined confidence ---
        let combined_confidence = self.confidence_engine.combine(&[
            (verification.confidence, 2.0),
            (account_suggestion.confidence, 3.0),
            (po_confidence, 2.0),
        ]);

        // --- Step 5: Payment suggestion ---
        let payment_suggestion = Some(self.suggest_payment(invoice));

        // --- Step 6: Decide action ---
        let requires_approval = invoice.total_amount() > self.approval_threshold;

        if requires_approval {
            let approval_req = self
                .request_approval(invoice, self.approval_threshold)
                .await;
            trail.push(AuditEvent::agent(
                "APPROVAL_REQUESTED",
                format!("approval_id={}", approval_req.id),
            ));
            return ApResult {
                invoice_id: invoice.id,
                status: ApStatus::PendingApproval,
                account_code: Some(account_suggestion.code),
                confidence: combined_confidence,
                auto_booked: false,
                requires_approval: true,
                payment_suggestion,
                audit_trail: trail,
            };
        }

        if self.confidence_engine.should_auto_book(combined_confidence) {
            let booking = self
                .auto_book(invoice, &account_suggestion.code)
                .await;
            match booking {
                Ok(result) => {
                    trail.push(AuditEvent::agent(
                        "AUTO_BOOKED",
                        format!(
                            "ledger_entry={} account={}",
                            result.ledger_entry_id, result.account_code
                        ),
                    ));
                    ApResult {
                        invoice_id: invoice.id,
                        status: ApStatus::AutoBooked,
                        account_code: Some(result.account_code),
                        confidence: combined_confidence,
                        auto_booked: true,
                        requires_approval: false,
                        payment_suggestion,
                        audit_trail: trail,
                    }
                }
                Err(err) => {
                    trail.push(AuditEvent::agent(
                        "AUTO_BOOK_FAILED",
                        format!("error={}", err),
                    ));
                    ApResult {
                        invoice_id: invoice.id,
                        status: ApStatus::ManualRequired,
                        account_code: Some(account_suggestion.code),
                        confidence: combined_confidence,
                        auto_booked: false,
                        requires_approval: false,
                        payment_suggestion,
                        audit_trail: trail,
                    }
                }
            }
        } else {
            trail.push(AuditEvent::agent(
                "MANUAL_REQUIRED",
                format!("confidence={:.2} below threshold", combined_confidence),
            ));
            ApResult {
                invoice_id: invoice.id,
                status: ApStatus::ManualRequired,
                account_code: Some(account_suggestion.code),
                confidence: combined_confidence,
                auto_booked: false,
                requires_approval: false,
                payment_suggestion,
                audit_trail: trail,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal methods
    // -----------------------------------------------------------------------

    /// Verify invoice: vendor, amounts, VAT, duplicates.
    async fn verify_invoice(&self, invoice: &Invoice) -> VerificationResult {
        let mut issues = Vec::new();

        // 1. Amount must be positive
        if invoice.amount_excl_vat <= Decimal::ZERO {
            issues.push(VerificationIssue {
                kind: IssueKind::AmountZero,
                message: "Invoice net amount is zero or negative".to_owned(),
                blocking: true,
            });
        }

        // 2. VAT number present (warn, not block, for domestic invoices >25 % threshold)
        if invoice.vendor_vat_number.is_none() {
            issues.push(VerificationIssue {
                kind: IssueKind::MissingVatNumber,
                message: "Vendor VAT number not present on invoice".to_owned(),
                blocking: false,
            });
        }

        // 3. VAT sanity check: accepted Swedish rates are 0 %, 6 %, 12 %, 25 %
        let vat_amount = invoice.vat_amount;
        let net = invoice.amount_excl_vat;
        let computed_rate = if net > Decimal::ZERO {
            (vat_amount / net * Decimal::new(100, 0))
                .round_dp(0)
                .to_f64()
                .unwrap_or(f64::MAX)
        } else {
            0.0
        };
        let valid_rates = [0.0_f64, 6.0, 12.0, 25.0];
        if !valid_rates.iter().any(|r| (computed_rate - r).abs() < 0.5) {
            issues.push(VerificationIssue {
                kind: IssueKind::VatCalculationMismatch,
                message: format!(
                    "Computed VAT rate {:.1}% does not match any Swedish standard rate (0/6/12/25%)",
                    computed_rate
                ),
                blocking: false,
            });
        }

        // 4. Past due date – warn only
        if invoice.due_date < Utc::now().date_naive() {
            issues.push(VerificationIssue {
                kind: IssueKind::PastDueDate,
                message: format!("Invoice due date {} is in the past", invoice.due_date),
                blocking: false,
            });
        }

        // 5. Duplicate detection
        let existing = self
            .erp_connector
            .get_booked_invoices(
                invoice.vendor_id,
                invoice.invoice_date.year(),
                invoice.invoice_date.month(),
            )
            .await
            .unwrap_or_default();

        let key = invoice.dedup_key();
        let is_duplicate = existing.iter().any(|e| e.dedup_key() == key);
        if is_duplicate {
            issues.push(VerificationIssue {
                kind: IssueKind::DuplicateInvoice,
                message: format!(
                    "Duplicate detected: same vendor + amount + period + invoice number (key={})",
                    &key[..12]
                ),
                blocking: true,
            });
        }

        // Compute verification confidence
        let deductions: f64 = issues
            .iter()
            .map(|i| if i.blocking { 0.5 } else { 0.05 })
            .sum();
        let confidence = (1.0_f64 - deductions).clamp(0.0, 1.0);

        let valid = !issues.iter().any(|i| i.blocking);
        VerificationResult {
            valid,
            issues,
            confidence,
        }
    }

    /// Try to find a matching purchase order by PO reference or vendor scan.
    async fn match_purchase_order(&self, invoice: &Invoice) -> Option<PurchaseOrder> {
        // Try explicit PO reference first
        if let Some(ref po_ref) = invoice.purchase_order_ref {
            let candidates = self
                .erp_connector
                .get_purchase_orders(invoice.vendor_id, po_ref)
                .await
                .unwrap_or_default();
            if let Some(po) = candidates.into_iter().next() {
                return Some(po);
            }
        }

        // Fall back: scan open POs for this vendor and pick the closest amount match
        let open_pos = self
            .erp_connector
            .list_open_purchase_orders(invoice.vendor_id)
            .await
            .unwrap_or_default();

        let tolerance = invoice.total_amount() * Decimal::new(5, 2); // 5 %
        open_pos.into_iter().find(|po| {
            po.currency == invoice.currency
                && (po.total_amount - invoice.total_amount()).abs() <= tolerance
                && po.status != PoStatus::Cancelled
        })
    }

    /// Classify the invoice into a BAS account code.
    pub fn suggest_account_code(&self, invoice: &Invoice) -> AccountSuggestion {
        classify_invoice(invoice)
    }

    /// Create an approval workflow request if amount exceeds threshold.
    async fn request_approval(
        &self,
        invoice: &Invoice,
        threshold: Decimal,
    ) -> ApprovalRequest {
        let suggestion = self.suggest_account_code(invoice);
        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            invoice_id: invoice.id,
            vendor_name: invoice.vendor_name.clone(),
            amount: invoice.total_amount(),
            currency: invoice.currency.clone(),
            suggested_account: Some(suggestion.code),
            reason: format!(
                "Invoice amount {} {} exceeds auto-approval threshold {}",
                invoice.total_amount(),
                invoice.currency,
                threshold
            ),
            created_at: Utc::now(),
        };

        // Best-effort: send to ERP; ignore errors (logged upstream)
        let _ = self
            .erp_connector
            .create_approval_request(request.clone())
            .await;
        request
    }

    /// Post a ledger entry when confidence is high enough.
    async fn auto_book(
        &self,
        invoice: &Invoice,
        account: &str,
    ) -> anyhow::Result<BookingResult> {
        // Debit: expense account; Credit: accounts payable (2440)
        let entry = LedgerEntry {
            id: Uuid::new_v4(),
            account_code: account.to_owned(),
            description: format!(
                "Auto-booked: {} – {} ({})",
                invoice.vendor_name, invoice.description, invoice.invoice_number
            ),
            debit: invoice.amount_excl_vat,
            credit: invoice.amount_excl_vat,
            reference: invoice.invoice_number.clone(),
            posted_at: Utc::now(),
        };

        let id = self.erp_connector.post_ledger_entry(entry).await?;
        Ok(BookingResult {
            ledger_entry_id: id,
            account_code: account.to_owned(),
            amount: invoice.total_amount(),
            posted_at: Utc::now(),
        })
    }

    /// Suggest an optimal payment date (as late as possible, respecting terms).
    pub fn suggest_payment(&self, invoice: &Invoice) -> PaymentSuggestion {
        let today = Utc::now().date_naive();

        // Strategy: pay on due date if > 5 days away; else pay in 2 days.
        let days_until_due = (invoice.due_date - today).num_days();
        let suggested_date = if days_until_due > 5 {
            invoice.due_date
        } else if days_until_due > 0 {
            today.checked_add_days(Days::new(2)).unwrap_or(today)
        } else {
            // Already past due – pay ASAP
            today.checked_add_days(Days::new(1)).unwrap_or(today)
        };

        let bank_account = self
            .bank_accounts
            .get(&invoice.currency)
            .cloned()
            .unwrap_or_else(|| self.default_bank_account.clone());

        PaymentSuggestion {
            amount: invoice.total_amount(),
            currency: invoice.currency.clone(),
            due_date: invoice.due_date,
            suggested_date,
            bank_account,
            reference: format!("{}/{}", invoice.vendor_name, invoice.invoice_number),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_invoice() -> Invoice {
        Invoice {
            id: Uuid::new_v4(),
            vendor_id: Uuid::new_v4(),
            vendor_name: "AWS EMEA SARL".to_owned(),
            vendor_vat_number: Some("LU26375245".to_owned()),
            amount_excl_vat: dec!(1000.00),
            vat_amount: dec!(250.00),
            currency: "SEK".to_owned(),
            invoice_date: Utc::now().date_naive(),
            due_date: Utc::now()
                .date_naive()
                .checked_add_days(Days::new(30))
                .unwrap(),
            invoice_number: "INV-2024-001".to_owned(),
            description: "Amazon Web Services usage".to_owned(),
            line_items: vec![],
            purchase_order_ref: None,
            raw_text: "AWS invoice 2024".to_owned(),
        }
    }

    #[test]
    fn test_classify_aws() {
        let inv = sample_invoice();
        let suggestion = classify_invoice(&inv);
        assert_eq!(suggestion.code, "6540");
        assert!(suggestion.confidence > 0.90);
    }

    #[test]
    fn test_classify_sj() {
        let mut inv = sample_invoice();
        inv.vendor_name = "SJ AB".to_owned();
        inv.description = "Train ticket Stockholm-Gothenburg".to_owned();
        let suggestion = classify_invoice(&inv);
        assert_eq!(suggestion.code, "5800");
    }

    #[test]
    fn test_classify_restaurant() {
        let mut inv = sample_invoice();
        inv.vendor_name = "ICA Maxi".to_owned();
        inv.description = "Groceries".to_owned();
        let suggestion = classify_invoice(&inv);
        assert_eq!(suggestion.code, "6000");
    }

    #[test]
    fn test_classify_consultant() {
        let mut inv = sample_invoice();
        inv.vendor_name = "Accenture AB".to_owned();
        inv.description = "Management consulting Q1".to_owned();
        let suggestion = classify_invoice(&inv);
        assert_eq!(suggestion.code, "6230");
    }

    #[test]
    fn test_classify_fallback() {
        let mut inv = sample_invoice();
        inv.vendor_name = "Okänd leverantör XYZ".to_owned();
        inv.description = "Diverse services".to_owned();
        let suggestion = classify_invoice(&inv);
        assert_eq!(suggestion.code, "6999");
        assert!(suggestion.confidence < 0.50);
    }

    #[test]
    fn test_dedup_key_stability() {
        let inv = sample_invoice();
        let k1 = inv.dedup_key();
        let k2 = inv.dedup_key();
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_payment_suggestion_future_due() {
        let engine = Arc::new(ConfidenceEngine::new(0.95));

        struct MockErp;
        #[async_trait::async_trait]
        impl AccountingProvider for MockErp {
            async fn get_booked_invoices(&self, _: Uuid, _: i32, _: u32) -> anyhow::Result<Vec<Invoice>> { Ok(vec![]) }
            async fn get_purchase_orders(&self, _: Uuid, _: &str) -> anyhow::Result<Vec<PurchaseOrder>> { Ok(vec![]) }
            async fn post_ledger_entry(&self, _: LedgerEntry) -> anyhow::Result<Uuid> { Ok(Uuid::new_v4()) }
            async fn create_approval_request(&self, _: ApprovalRequest) -> anyhow::Result<Uuid> { Ok(Uuid::new_v4()) }
            async fn list_open_purchase_orders(&self, _: Uuid) -> anyhow::Result<Vec<PurchaseOrder>> { Ok(vec![]) }
        }

        let agent = ApAutomationAgent::new(
            engine,
            Arc::new(MockErp),
            dec!(50000),
            "SE00 0000 0000 0000 0000 0001",
        );

        let inv = sample_invoice();
        let sug = agent.suggest_payment(&inv);
        assert_eq!(sug.currency, "SEK");
        assert!(sug.suggested_date >= Utc::now().date_naive());
    }

    #[test]
    fn test_three_way_match_full() {
        let inv = sample_invoice();
        let po = PurchaseOrder {
            id: Uuid::new_v4(),
            vendor_id: inv.vendor_id,
            po_number: "PO-2024-001".to_owned(),
            total_amount: inv.total_amount(),
            currency: inv.currency.clone(),
            status: PoStatus::FullyReceived,
            delivery_confirmed: true,
            line_items: vec![],
        };
        let result = three_way_match(&inv, &po);
        assert_eq!(result.match_type, MatchType::FullThreeWay);
        assert!(result.matched);
        assert!(result.confidence > 0.95);
    }

    #[test]
    fn test_three_way_match_amount_discrepancy() {
        let inv = sample_invoice();
        let po = PurchaseOrder {
            id: Uuid::new_v4(),
            vendor_id: inv.vendor_id,
            po_number: "PO-2024-002".to_owned(),
            total_amount: dec!(9999.00), // large discrepancy
            currency: inv.currency.clone(),
            status: PoStatus::Open,
            delivery_confirmed: false,
            line_items: vec![],
        };
        let result = three_way_match(&inv, &po);
        assert!(!result.discrepancies.is_empty());
        assert_eq!(result.match_type, MatchType::VendorOnly);
    }

    #[test]
    fn test_confidence_engine_combine() {
        let engine = ConfidenceEngine::new(0.95);
        let score = engine.combine(&[(1.0, 1.0), (0.9, 2.0), (0.8, 1.0)]);
        // weighted: (1.0*1 + 0.9*2 + 0.8*1) / 4 = 3.5/4 = 0.875
        assert!((score - 0.875).abs() < 0.001);
    }
}
