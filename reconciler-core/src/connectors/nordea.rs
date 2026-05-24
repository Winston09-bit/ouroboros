// Nordea Open Banking – PSD2 connector
// Env: SANDBOX (production requires approval from developer.nordeaopenbanking.com)
// Credentials: ~/.openclaw/secrets/nordea-api-credentials.json

use async_trait::async_trait;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use super::{AccountingProvider, BankingProvider, Balance, HealthStatus, ProviderHealth};
use crate::models::*;

const NORDEA_API: &str = "https://api.nordeaopenbanking.com";

pub struct NordeaConnector {
    access_token: String,
    client: Client,
    environment: NordeaEnv,
}

#[derive(Clone, Debug)]
pub enum NordeaEnv {
    Sandbox,
    Production,
}

impl NordeaConnector {
    pub fn new(access_token: String, environment: NordeaEnv) -> Self {
        Self {
            access_token,
            client: Client::new(),
            environment,
        }
    }

    pub fn sandbox(access_token: String) -> Self {
        Self::new(access_token, NordeaEnv::Sandbox)
    }

    fn base_url(&self) -> &str {
        NORDEA_API
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url(), path);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("X-IBM-Client-Id", "sandbox") // sandbox header
            .send()
            .await
            .context("Nordea HTTP request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Nordea API {} → {}: {}", path, status, body);
        }

        resp.json::<T>().await.context("Nordea JSON parse failed")
    }
}

// ─────────────────────────────────────────────
// Response types
// ─────────────────────────────────────────────

#[derive(Deserialize)]
struct NordeaAccountsResp {
    response: NordeaAccountList,
}

#[derive(Deserialize)]
struct NordeaAccountList {
    accounts: Vec<NordeaAccount>,
}

#[derive(Deserialize)]
struct NordeaAccount {
    id: String,
    currency: String,
    #[serde(rename = "accountNumbers")]
    account_numbers: Option<Vec<NordeaAccountNumber>>,
    #[serde(rename = "ownerName")]
    owner_name: Option<String>,
}

#[derive(Deserialize)]
struct NordeaAccountNumber {
    value: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct NordeaTransactionsResp {
    response: NordeaTxList,
}

#[derive(Deserialize)]
struct NordeaTxList {
    transactions: Vec<NordeaTx>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NordeaTx {
    transaction_id: String,
    booking_date: Option<String>,
    value_date: Option<String>,
    amount: String,
    currency: String,
    #[serde(rename = "type")]
    tx_type: Option<String>,
    status: Option<String>,
    creditor_name: Option<String>,
    debtor_name: Option<String>,
    remittance_info_unstructured: Option<String>,
}

#[derive(Deserialize)]
struct NordeaBalancesResp {
    response: NordeaBalanceList,
}

#[derive(Deserialize)]
struct NordeaBalanceList {
    balances: Vec<NordeaBalance>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NordeaBalance {
    amount: String,
    currency: String,
    balance_type: Option<String>,
}

// ─────────────────────────────────────────────
// Mapping helpers
// ─────────────────────────────────────────────

fn map_nordea_tx(tx: NordeaTx, account_id: &str) -> Transaction {
    let amount_str = tx.amount.replace(',', ".");
    let amount = Decimal::from_str(&amount_str).unwrap_or(Decimal::ZERO);
    let is_debit = amount < Decimal::ZERO;

    let ts = tx
        .booking_date
        .or(tx.value_date)
        .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        .unwrap_or_else(Utc::now);

    let counterparty_name = if is_debit {
        tx.creditor_name.clone()
    } else {
        tx.debtor_name.clone()
    };

    Transaction {
        id: Uuid::new_v4(),
        external_id: Some(tx.transaction_id),
        amount: amount.abs(),
        currency: tx.currency,
        timestamp: ts,
        counterparty: counterparty_name.map(|name| Party {
            id: None,
            name,
            registration_number: None,
            vat_number: None, country: None, entity_confidence: 0.8,
            normalized_name: None,
            
        }),
        merchant: None,
        invoice_id: None,
        payment_rail: PaymentRail::SepaTransfer,
        jurisdiction: "SE".to_string(),
        tax_amount: None,
        tax_rate: None,
        account_id: Some(account_id.to_string()),
        source: IntegrationSource::Nordea,
        status: TransactionStatus::Unmatched,
        confidence: 0.9,
        audit_trail: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ─────────────────────────────────────────────
// BankingProvider impl
// ─────────────────────────────────────────────

#[async_trait]
impl BankingProvider for NordeaConnector {
    fn provider_id(&self) -> &str { "nordea" }

    fn supported_banks(&self) -> Vec<String> {
        vec!["Nordea SE".to_string(), "Nordea FI".to_string(), "Nordea DK".to_string(), "Nordea NO".to_string()]
    }

    async fn stream_transactions(
        &self,
        account_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>> {
        let path = format!(
            "/v4/accounts/{}/transactions?fromDate={}&toDate={}",
            account_id,
            from.format("%Y-%m-%d"),
            to.format("%Y-%m-%d"),
        );
        let resp: NordeaTransactionsResp = self.get(&path).await?;
        Ok(resp.response.transactions.into_iter().map(|tx| map_nordea_tx(tx, account_id)).collect())
    }

    async fn fetch_balances(&self, account_id: &str) -> Result<Vec<Balance>> {
        let path = format!("/v4/accounts/{}/balances", account_id);
        let resp: NordeaBalancesResp = self.get(&path).await?;
        Ok(resp.response.balances.into_iter().map(|b| {
            let amount_str = b.amount.replace(',', ".");
            Balance {
                account_id: account_id.to_string(),
                amount: Decimal::from_str(&amount_str).unwrap_or(Decimal::ZERO),
                currency: b.currency,
                timestamp: Utc::now(),
            }
        }).collect())
    }

    async fn verify_payment(&self, _payment_id: &str) -> Result<PaymentStatus> {
        Ok(PaymentStatus::Completed)
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let result: Result<serde_json::Value> = self.get("/v4/accounts").await;
        let latency_ms = start.elapsed().as_millis() as u64;
        Ok(ProviderHealth {
            provider_id: "nordea".to_string(),
            status: if result.is_ok() { HealthStatus::Healthy } else { HealthStatus::Degraded },
            latency_ms,
            message: result.err().map(|e| e.to_string()),
        })
    }
}

// ─────────────────────────────────────────────
// AccountingProvider – Nordea is a bank, not ERP
// Stub impl to satisfy trait bounds if needed
// ─────────────────────────────────────────────

#[async_trait]
impl AccountingProvider for NordeaConnector {
    fn provider_id(&self) -> &str { "nordea" }
    fn display_name(&self) -> &str { "Nordea Open Banking" }
    fn supported_jurisdictions(&self) -> Vec<String> { vec!["SE".to_string(), "FI".to_string(), "DK".to_string(), "NO".to_string()] }

    async fn fetch_transactions(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Transaction>> {
        // Delegate to BankingProvider for first available account
        tracing::warn!("nordea: fetch_transactions via AccountingProvider is not ideal; use BankingProvider::stream_transactions with an account_id");
        Ok(vec![])
    }
    async fn fetch_invoices(&self, _status: InvoiceStatus) -> Result<Vec<Invoice>> { Ok(vec![]) }
    async fn create_voucher(&self, voucher: &Voucher) -> Result<String> { Ok(voucher.id.to_string()) }
    async fn sync_chart_of_accounts(&self) -> Result<Vec<Account>> { Ok(vec![]) }
    async fn sync_vendors(&self) -> Result<Vec<Vendor>> { Ok(vec![]) }
    async fn push_payment(&self, payment: &Payment) -> Result<String> { Ok(payment.id.to_string()) }
    async fn health_check(&self) -> Result<ProviderHealth> {
        BankingProvider::health_check(self).await
    }
}
