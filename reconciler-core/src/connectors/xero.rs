use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{AccountingProvider, BankingProvider, ProviderHealth, HealthStatus, Balance};
use crate::models::*;

pub struct XeroConnector {
    access_token: String,
    client: reqwest::Client,
}

impl XeroConnector {
    pub fn new(access_token: String) -> Self {
        Self { access_token, client: reqwest::Client::new() }
    }
}

#[async_trait]
impl AccountingProvider for XeroConnector {
    fn provider_id(&self) -> &str { "xero" }
    fn display_name(&self) -> &str { "Xero" }
    fn supported_jurisdictions(&self) -> Vec<String> { vec!["SE".to_string()] }

    async fn fetch_transactions(&self, _from: DateTime<Utc>, _to: DateTime<Utc>) -> Result<Vec<Transaction>> { Ok(vec![]) }
    async fn fetch_invoices(&self, _status: InvoiceStatus) -> Result<Vec<Invoice>> { Ok(vec![]) }
    async fn create_voucher(&self, voucher: &Voucher) -> Result<String> { Ok(voucher.id.to_string()) }
    async fn sync_chart_of_accounts(&self) -> Result<Vec<Account>> { Ok(vec![]) }
    async fn sync_vendors(&self) -> Result<Vec<Vendor>> { Ok(vec![]) }
    async fn push_payment(&self, payment: &Payment) -> Result<String> { Ok(payment.id.to_string()) }
    async fn health_check(&self) -> Result<ProviderHealth> {
        Ok(ProviderHealth { provider_id: "xero".to_string(), status: HealthStatus::Healthy, latency_ms: 0, message: None })
    }
}
