use async_trait::async_trait;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use super::{Balance, BankingProvider, HealthStatus, ProviderHealth};
use crate::models::*;

const BASE_URL: &str = "https://api.enablebanking.com";

// ─────────────────────────────────────────────
// CONNECTOR
// ─────────────────────────────────────────────

pub struct EnableBankingConnector {
    app_id: String,
    session_id: String,
    private_key_path: String,
    client: reqwest::Client,
}

impl EnableBankingConnector {
    pub fn new(app_id: String, session_id: String, private_key_path: String) -> Self {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Failed to build reqwest client");
        Self { app_id, session_id, private_key_path, client }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        use std::env;
        let app_id     = env::var("ENABLE_BANKING_APP_ID")
            .unwrap_or_else(|_| "a1427b33-1ee3-4e2b-8d53-ecbfabe41572".to_string());
        let session_id = env::var("ENABLE_BANKING_SESSION")
            .unwrap_or_else(|_| "e625b217-a84c-4a7e-a58a-33bda27a8a59".to_string());
        let key_path   = env::var("ENABLE_BANKING_KEY_PATH")
            .unwrap_or_else(|_| "/home/userwinston/.openclaw/secrets/enable-banking-key.pem".to_string());
        Ok(Self::new(app_id, session_id, key_path))
    }

    /// Convenience constructor with hardcoded credentials.
    pub fn with_default_config() -> Self {
        Self::new(
            "a1427b33-1ee3-4e2b-8d53-ecbfabe41572".to_string(),
            "e625b217-a84c-4a7e-a58a-33bda27a8a59".to_string(),
            "/home/userwinston/.openclaw/secrets/enable-banking-key.pem".to_string(),
        )
    }

    // ── JWT ─────────────────────────────────

    fn generate_jwt(&self) -> Result<String> {
        let pem = std::fs::read(&self.private_key_path)
            .with_context(|| format!("Failed to read private key: {}", self.private_key_path))?;

        let key = EncodingKey::from_rsa_pem(&pem)
            .context("Failed to parse RSA PEM key")?;

        let now = Utc::now().timestamp();
        let claims = JwtClaims { iss: self.app_id.clone(), iat: now, exp: now + 3600 };

        encode(&Header::new(Algorithm::RS256), &claims, &key)
            .context("JWT signing failed")
    }

    // ── HTTP helper ──────────────────────────

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let token = self.generate_jwt()?;
        let url = format!("{}{}", BASE_URL, path);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .with_context(|| format!("HTTP GET failed: {}", url))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Enable Banking {} – {}: {}", url, status, body);
        }

        resp.json::<T>()
            .await
            .with_context(|| format!("JSON decode failed for {}", url))
    }
}

// ─────────────────────────────────────────────
// JWT CLAIMS
// ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    iss: String,
    iat: i64,
    exp: i64,
}

// ─────────────────────────────────────────────
// ENABLE BANKING API TYPES
// ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AccountsResponse {
    #[serde(default)]
    accounts: Vec<EbAccount>,
}

#[derive(Debug, Deserialize)]
struct EbAccount {
    uid: Option<String>,
    #[serde(rename = "resourceId")]
    resource_id: Option<String>,
}

impl EbAccount {
    fn id(&self) -> Option<&str> {
        self.uid.as_deref().or(self.resource_id.as_deref())
    }
}

#[derive(Debug, Deserialize)]
struct TransactionsResponse {
    transactions: Option<Vec<EbTransaction>>,
}

#[derive(Debug, Deserialize)]
struct EbTransaction {
    #[serde(rename = "transactionId")]
    transaction_id: Option<String>,
    #[serde(rename = "transactionAmount")]
    transaction_amount: Option<EbAmount>,
    #[serde(rename = "bookingDate")]
    booking_date: Option<String>,
    #[serde(rename = "valueDate")]
    value_date: Option<String>,
    #[serde(rename = "transactionDate")]
    transaction_date: Option<String>,
    #[serde(rename = "creditDebitIndicator")]
    credit_debit_indicator: Option<String>,
    #[serde(rename = "remittanceInformationUnstructured")]
    remittance_information_unstructured: Option<String>,
    #[serde(rename = "remittanceInformationUnstructuredArray")]
    remittance_information_array: Option<Vec<String>>,
    #[serde(rename = "creditorName")]
    creditor_name: Option<String>,
    #[serde(rename = "debtorName")]
    debtor_name: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EbAmount {
    amount: String,
    currency: String,
}

#[derive(Debug, Deserialize)]
struct BalancesResponse {
    balances: Vec<EbBalance>,
}

#[derive(Debug, Deserialize)]
struct EbBalance {
    #[serde(rename = "balanceAmount")]
    balance_amount: EbAmount,
    #[serde(rename = "balanceType")]
    balance_type: Option<String>,
    #[serde(rename = "lastChangeDateTime")]
    last_change_date_time: Option<String>,
}

// ─────────────────────────────────────────────
// MAPPING HELPERS
// ─────────────────────────────────────────────

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc())
}

fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn map_transaction(tx: &EbTransaction, account_id: &str) -> Transaction {
    let raw_amount = tx
        .transaction_amount
        .as_ref()
        .and_then(|a| Decimal::from_str(&a.amount).ok())
        .unwrap_or(Decimal::ZERO);

    let currency = tx
        .transaction_amount
        .as_ref()
        .map(|a| a.currency.clone())
        .unwrap_or_else(|| "EUR".to_string());

    // Debit = negative, Credit = positive
    let signed_amount = match tx.credit_debit_indicator.as_deref() {
        Some("DBIT") => -raw_amount,
        _ => raw_amount,
    };

    let timestamp = tx
        .booking_date
        .as_deref()
        .or(tx.value_date.as_deref())
        .or(tx.transaction_date.as_deref())
        .and_then(parse_date)
        .unwrap_or_else(Utc::now);

    let remittance = tx
        .remittance_information_unstructured
        .clone()
        .or_else(|| tx.remittance_information_array.as_ref()?.first().cloned());

    let counterparty_name = tx.creditor_name.clone().or_else(|| tx.debtor_name.clone());

    let description = remittance
        .clone()
        .or_else(|| counterparty_name.clone())
        .unwrap_or_default();

    let tx_status = match tx.status.as_deref() {
        Some("BOOK") => TransactionStatus::Unmatched,
        Some("PDNG") => TransactionStatus::Unmatched,
        _ => TransactionStatus::Unmatched,
    };

    Transaction {
        id: Uuid::new_v4(),
        external_id: tx.transaction_id.clone(),
        amount: signed_amount,
        currency,
        timestamp,
        counterparty: counterparty_name.map(|name| Party {
            id: None,
            name,
            normalized_name: None,
            registration_number: None,
            vat_number: None, country: None, entity_confidence: 0.8,
        }),
        merchant: Some(MerchantInfo {
            raw_name: description,
            normalized_name: None,
            entity_id: None,
            mcc: None,
            country: None,
            confidence: 0.5,
        }),
        invoice_id: None,
        payment_rail: PaymentRail::SepaTransfer,
        jurisdiction: "SE".to_string(),
        tax_amount: None,
        tax_rate: None,
        account_id: Some(account_id.to_string()),
        source: IntegrationSource::ApiDirect,
        status: tx_status,
        confidence: 0.8,
        audit_trail: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ─────────────────────────────────────────────
// BankingProvider IMPL
// ─────────────────────────────────────────────

#[async_trait]
impl BankingProvider for EnableBankingConnector {
    fn provider_id(&self) -> &str {
        "enable_banking"
    }

    fn supported_banks(&self) -> Vec<String> {
        vec![
            "nordea".to_string(),
            "seb".to_string(),
            "swedbank".to_string(),
            "handelsbanken".to_string(),
        ]
    }

    /// Fetch transactions for a given account and date range.
    /// Pass one of the known account UUIDs:
    ///   SEK: 65f16d5c-0803-4b49-934e-24c23aff52fd
    ///   EUR: 67333f2a-164a-4b23-803b-3c949d0d218b
    async fn stream_transactions(
        &self,
        account_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>> {
        let date_from = from.format("%Y-%m-%d");
        let date_to = to.format("%Y-%m-%d");

        let path = format!(
            "/sessions/{}/accounts/{}/transactions?date_from={}&date_to={}",
            self.session_id, account_id, date_from, date_to
        );

        let resp: TransactionsResponse = self.get(&path).await?;

        let txs = resp
            .transactions
            .unwrap_or_default()
            .iter()
            .map(|tx| map_transaction(tx, account_id))
            .collect();

        Ok(txs)
    }

    async fn fetch_balances(&self, account_id: &str) -> Result<Vec<Balance>> {
        let path = format!(
            "/sessions/{}/accounts/{}/balances",
            self.session_id, account_id
        );

        let resp: BalancesResponse = self.get(&path).await?;

        let balances = resp
            .balances
            .iter()
            .map(|b| {
                let amount =
                    Decimal::from_str(&b.balance_amount.amount).unwrap_or(Decimal::ZERO);

                let timestamp = b
                    .last_change_date_time
                    .as_deref()
                    .and_then(parse_rfc3339)
                    .unwrap_or_else(Utc::now);

                Balance {
                    account_id: account_id.to_string(),
                    amount,
                    currency: b.balance_amount.currency.clone(),
                    timestamp,
                }
            })
            .collect();

        Ok(balances)
    }

    /// Enable Banking is read-only; payment verification not supported.
    async fn verify_payment(&self, _payment_id: &str) -> Result<PaymentStatus> {
        Ok(PaymentStatus::Pending)
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let path = format!("/sessions/{}/accounts", self.session_id);

        match self.get::<serde_json::Value>(&path).await {
            Ok(_) => Ok(ProviderHealth {
                provider_id: "enable_banking".to_string(),
                status: HealthStatus::Healthy,
                latency_ms: start.elapsed().as_millis() as u64,
                message: None,
            }),
            Err(e) => Ok(ProviderHealth {
                provider_id: "enable_banking".to_string(),
                status: HealthStatus::Down,
                latency_ms: start.elapsed().as_millis() as u64,
                message: Some(e.to_string()),
            }),
        }
    }
}
