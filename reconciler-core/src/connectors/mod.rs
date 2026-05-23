pub mod fortnox;
pub mod visma;
pub mod xero;
pub mod tink;
pub mod nordea;

use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::models::*;

// ─────────────────────────────────────────────
// CORE CONNECTOR TRAIT
// Every integration implements this.
// New connectors = days, not months.
// ─────────────────────────────────────────────
#[async_trait]
pub trait AccountingProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn supported_jurisdictions(&self) -> Vec<String>;

    async fn fetch_transactions(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>>;

    async fn fetch_invoices(&self, status: InvoiceStatus) -> Result<Vec<Invoice>>;
    async fn create_voucher(&self, voucher: &Voucher) -> Result<String>;
    async fn sync_chart_of_accounts(&self) -> Result<Vec<Account>>;
    async fn sync_vendors(&self) -> Result<Vec<Vendor>>;
    async fn push_payment(&self, payment: &Payment) -> Result<String>;
    async fn health_check(&self) -> Result<ProviderHealth>;
}

#[async_trait]
pub trait BankingProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn supported_banks(&self) -> Vec<String>;

    async fn stream_transactions(
        &self,
        account_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>>;

    async fn fetch_balances(&self, account_id: &str) -> Result<Vec<Balance>>;
    async fn verify_payment(&self, payment_id: &str) -> Result<PaymentStatus>;
    async fn health_check(&self) -> Result<ProviderHealth>;
}

#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub status: HealthStatus,
    pub latency_ms: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus { Healthy, Degraded, Down }

#[derive(Debug, Clone)]
pub struct Balance {
    pub account_id: String,
    pub amount: rust_decimal::Decimal,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
}

// Provider registry
pub struct ProviderRegistry {
    accounting: Vec<Box<dyn AccountingProvider>>,
    banking: Vec<Box<dyn BankingProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { accounting: vec![], banking: vec![] }
    }

    pub fn register_accounting(&mut self, p: Box<dyn AccountingProvider>) {
        self.accounting.push(p);
    }

    pub fn register_banking(&mut self, p: Box<dyn BankingProvider>) {
        self.banking.push(p);
    }

    pub fn accounting_providers(&self) -> &[Box<dyn AccountingProvider>] {
        &self.accounting
    }

    pub fn banking_providers(&self) -> &[Box<dyn BankingProvider>] {
        &self.banking
    }
}
pub mod revolut;
pub mod stripe_connector;
pub mod kivra;
pub mod quickbooks;
