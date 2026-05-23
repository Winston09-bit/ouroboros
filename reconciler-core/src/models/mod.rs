use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─────────────────────────────────────────────
// AUDIT EVENT — immutable, append-only
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor: String,          // "system", "user:uuid", "agent:receipt-recovery"
    pub action: String,         // "auto_booked", "matched", "escalated"
    pub reason: String,
    pub confidence: f64,
    pub source: String,         // "fortnox", "tink", "ai-engine"
    pub payload: serde_json::Value,
}

impl AuditEvent {
    pub fn new(actor: &str, action: &str, reason: &str, confidence: f64, source: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            action: action.to_string(),
            reason: reason.to_string(),
            confidence,
            source: source.to_string(),
            payload: serde_json::Value::Null,
        }
    }
}

// ─────────────────────────────────────────────
// CANONICAL TRANSACTION
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub counterparty: Option<Party>,
    pub merchant: Option<MerchantInfo>,
    pub invoice_id: Option<Uuid>,
    pub payment_rail: PaymentRail,
    pub jurisdiction: String,       // "SE", "US", "GB"
    pub tax_amount: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub account_id: Option<String>,
    pub source: IntegrationSource,
    pub status: TransactionStatus,
    pub confidence: f64,
    pub audit_trail: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Unmatched,
    Matched,
    Booked,
    Disputed,
    Escalated,
    ManualReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantInfo {
    pub raw_name: String,
    pub normalized_name: Option<String>,
    pub entity_id: Option<Uuid>,
    pub mcc: Option<String>,
    pub country: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentRail {
    Card,
    SepaTransfer,
    SwiftWire,
    Ach,
    Fps,
    Rtgs,
    Unknown,
}

// ─────────────────────────────────────────────
// CANONICAL INVOICE
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub invoice_number: String,
    pub vendor: Option<Party>,
    pub customer: Option<Party>,
    pub amount: Decimal,
    pub tax_amount: Decimal,
    pub currency: String,
    pub issued_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub status: InvoiceStatus,
    pub source: IntegrationSource,
    pub jurisdiction: String,
    pub line_items: Vec<LineItem>,
    pub documents: Vec<Document>,
    pub confidence: f64,
    pub audit_trail: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceStatus {
    Received,
    Pending,
    Approved,
    Paid,
    Overdue,
    Disputed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total: Decimal,
    pub tax_rate: Option<Decimal>,
    pub account_code: Option<String>,
}

// ─────────────────────────────────────────────
// LEDGER ENTRY (double-entry)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub journal_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub currency: String,
    pub description: String,
    pub transaction_id: Option<Uuid>,
    pub invoice_id: Option<Uuid>,
    pub jurisdiction: String,
    pub period: String,           // "2026-05"
    pub confidence: f64,
    pub audit_trail: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────
// PARTY (vendor / customer / merchant)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: Option<Uuid>,
    pub name: String,
    pub normalized_name: Option<String>,
    pub registration_number: Option<String>,
    pub vat_number: Option<String>,
    pub country: Option<String>,
    pub entity_confidence: f64,
}

// ─────────────────────────────────────────────
// VENDOR
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    pub id: Uuid,
    pub party: Party,
    pub default_account_code: Option<String>,
    pub payment_terms_days: Option<i32>,
    pub preferred_currency: Option<String>,
    pub audit_trail: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────
// ACCOUNT (chart of accounts)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub currency: String,
    pub parent_code: Option<String>,
    pub jurisdiction: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountType {
    Asset, Liability, Equity, Revenue, Expense,
}

// ─────────────────────────────────────────────
// TAX EVENT
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxEvent {
    pub id: Uuid,
    pub event_type: TaxEventType,
    pub amount: Decimal,
    pub tax_amount: Decimal,
    pub tax_rate: Decimal,
    pub jurisdiction: String,
    pub period: String,
    pub transaction_id: Option<Uuid>,
    pub invoice_id: Option<Uuid>,
    pub confidence: f64,
    pub audit_trail: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaxEventType {
    VatInput, VatOutput, SalesTax, WithholdingTax, PayrollTax,
}

// ─────────────────────────────────────────────
// PAYMENT
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub from_account: String,
    pub to_account: String,
    pub reference: Option<String>,
    pub invoice_ids: Vec<Uuid>,
    pub status: PaymentStatus,
    pub rail: PaymentRail,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub audit_trail: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Draft, Pending, Processing, Completed, Failed, Cancelled,
}

// ─────────────────────────────────────────────
// DOCUMENT
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub doc_type: DocumentType,
    pub filename: String,
    pub storage_url: String,
    pub ocr_text: Option<String>,
    pub extracted_data: Option<serde_json::Value>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    Receipt, Invoice, BankStatement, Contract, AuditReport, Other,
}

// ─────────────────────────────────────────────
// INTEGRATION SOURCE
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationSource {
    Fortnox, Visma, Xero, QuickBooks, NetSuite, Sap, Dynamics,
    Tink, Nordea, Revolut, Seb, Handelsbanken, Plaid, Swift,
    Peppol, Kivra, Email, ManualUpload, ApiDirect,
}

// ─────────────────────────────────────────────
// VOUCHER (for ERP posting)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voucher {
    pub id: Uuid,
    pub description: String,
    pub date: DateTime<Utc>,
    pub entries: Vec<LedgerEntry>,
    pub transaction_id: Option<Uuid>,
    pub invoice_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
