//! Tink PSD2 banking connector for Kvittovalvet / Reconciler.
//!
//! Auth flow:
//!   1. POST /api/v1/oauth/token (client_credentials) → cached client token.
//!   2. POST /api/v1/oauth/authorization-grant/delegate (with client token) → code.
//!   3. POST /api/v1/oauth/token (authorization_code + code) → user-scoped token.
//!
//! Implements `BankingProvider` from `crate::connectors`.
//! account_id format expected: "user:<tink_user_id>:acc:<tink_account_id>"

use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::connectors::{Balance, BankingProvider, HealthStatus, ProviderHealth};
use crate::models::*;

// ── Constants ─────────────────────────────────────────────────────────────────

const BASE_URL: &str = "https://api.tink.com";
const SECRETS_PATH: &str = "/home/userwinston/.openclaw/secrets/tink.json";
const TOKEN_EXPIRY_BUFFER_SECS: i64 = 60;
const HTTP_TIMEOUT_SECS: u64 = 30;

// ── Secrets ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TinkSecrets {
    client_id: String,
    client_secret: String,
}

// ── REST response types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct DelegateResponse {
    code: String,
}

#[derive(Debug, Deserialize)]
struct TinkTransactionsResponse {
    transactions: Vec<TinkTransaction>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TinkTransaction {
    id: String,
    amount: TinkAmount,
    dates: TinkDates,
    descriptions: Option<TinkDescriptions>,
    #[serde(rename = "merchantInformation")]
    merchant_information: Option<TinkMerchant>,
    status: Option<String>,
    counterparties: Option<Vec<TinkCounterparty>>,
}

#[derive(Debug, Deserialize)]
struct TinkAmount {
    value: TinkValue,
    #[serde(rename = "currencyCode")]
    currency_code: String,
}

#[derive(Debug, Deserialize)]
struct TinkValue {
    #[serde(rename = "unscaledValue")]
    unscaled_value: String,
    scale: String,
}

#[derive(Debug, Deserialize)]
struct TinkDates {
    booked: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TinkDescriptions {
    original: Option<String>,
    display: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TinkMerchant {
    name: Option<String>,
    #[serde(rename = "categoryCode")]
    category_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TinkCounterparty {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TinkBalancesResponse {
    balances: Vec<TinkBalanceEntry>,
}

#[derive(Debug, Deserialize)]
struct TinkBalanceEntry {
    #[serde(rename = "accountId")]
    account_id: Option<String>,
    amount: TinkAmount,
}

// ── Token cache ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CachedToken {
    token: String,
    expires_at: DateTime<Utc>,
}

// ── Connector ─────────────────────────────────────────────────────────────────

pub struct TinkConnector {
    client: reqwest::Client,
    client_id: String,
    client_secret: String,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
}

impl TinkConnector {
    /// Construct by reading credentials from disk synchronously.
    pub fn from_env() -> Result<Self> {
        let raw = std::fs::read_to_string(SECRETS_PATH)
            .with_context(|| format!("Cannot read {}", SECRETS_PATH))?;

        let secrets: TinkSecrets =
            serde_json::from_str(&raw).context("Cannot parse tink.json")?;

        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            client_id: secrets.client_id,
            client_secret: secrets.client_secret,
            cached_token: Arc::new(RwLock::new(None)),
        })
    }

    // ── Auth ──────────────────────────────────────────────────────────────────

    /// Fetch (or return cached) client_credentials access token.
    pub async fn client_token(&self) -> Result<String> {
        {
            let guard = self.cached_token.read().await;
            if let Some(ref cached) = *guard {
                if cached.expires_at > Utc::now() + Duration::seconds(TOKEN_EXPIRY_BUFFER_SECS) {
                    return Ok(cached.token.clone());
                }
            }
        }

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "client_credentials"),
            (
                "scope",
                "transactions:read,accounts:read,balances:read,credentials:read",
            ),
        ];

        let resp = self
            .client
            .post(&format!("{}/api/v1/oauth/token", BASE_URL))
            .form(&params)
            .send()
            .await
            .context("POST /api/v1/oauth/token network error")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Tink client_credentials failed {}: {}", status, body);
        }

        let tr: TokenResponse = resp.json().await.context("Cannot parse token response")?;
        let expires_in = if tr.expires_in > 0 { tr.expires_in as i64 } else { 3600 };
        let expires_at = Utc::now() + Duration::seconds(expires_in);
        let token = tr.access_token.clone();

        {
            let mut guard = self.cached_token.write().await;
            *guard = Some(CachedToken {
                token: tr.access_token,
                expires_at,
            });
        }

        Ok(token)
    }

    /// Obtain a user-scoped token via the delegate grant flow.
    pub async fn user_token(&self, user_id: &str) -> Result<String> {
        let client_tok = self.client_token().await?;

        // Step 1: delegate grant → authorization code
        let delegate_params = [
            ("user_id", user_id),
            ("id_hint", user_id),
            ("scope", "transactions:read,accounts:read,balances:read"),
        ];

        let resp = self
            .client
            .post(&format!(
                "{}/api/v1/oauth/authorization-grant/delegate",
                BASE_URL
            ))
            .bearer_auth(&client_tok)
            .form(&delegate_params)
            .send()
            .await
            .context("POST /api/v1/oauth/authorization-grant/delegate network error")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Tink delegate grant failed {}: {}", status, body);
        }

        let delegate: DelegateResponse = resp
            .json()
            .await
            .context("Cannot parse delegate grant response")?;

        // Step 2: exchange code → user-scoped access token
        let exchange_params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", delegate.code.as_str()),
        ];

        let resp = self
            .client
            .post(&format!("{}/api/v1/oauth/token", BASE_URL))
            .form(&exchange_params)
            .send()
            .await
            .context("POST /api/v1/oauth/token (authorization_code) network error")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Tink authorization_code exchange failed {}: {}", status, body);
        }

        let tr: TokenResponse = resp
            .json()
            .await
            .context("Cannot parse user token response")?;

        Ok(tr.access_token)
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Parse "user:<uid>:acc:<aid>" → (uid, aid).
    fn parse_account_id(account_id: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = account_id.splitn(4, ':').collect();
        if parts.len() == 4 && parts[0] == "user" && parts[2] == "acc" {
            Ok((parts[1].to_string(), parts[3].to_string()))
        } else {
            anyhow::bail!(
                "Invalid account_id format. Expected 'user:<uid>:acc:<aid>', got: {}",
                account_id
            )
        }
    }

    /// Convert Tink `{ unscaledValue, scale }` to `Decimal`.
    /// E.g. unscaledValue="1050", scale="2" → 10.50
    fn parse_tink_decimal(value: &TinkValue) -> Decimal {
        let unscaled: i64 = value.unscaled_value.parse().unwrap_or(0);
        let scale: u32 = value.scale.parse().unwrap_or(0);
        Decimal::new(unscaled, scale)
    }

    fn map_transaction(&self, t: TinkTransaction, raw_account_id: &str) -> Transaction {
        let amount = Self::parse_tink_decimal(&t.amount.value);

        // Prefer booked date, fall back to value date, then now.
        let timestamp = t
            .dates
            .booked
            .as_deref()
            .or(t.dates.value.as_deref())
            .and_then(|s| {
                // Tink may return "YYYY-MM-DD" or RFC3339
                if s.len() == 10 {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                } else {
                    DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }
            })
            .unwrap_or_else(Utc::now);

        let counterparty = t
            .counterparties
            .as_ref()
            .and_then(|cp| cp.first())
            .and_then(|cp| cp.name.as_ref())
            .map(|name| Party {
                id: None,
                name: name.clone(),
                normalized_name: None,
                registration_number: None,
                vat_number: None,
                country: None,
                entity_confidence: 0.7,
            });

        let merchant = t.merchant_information.as_ref().map(|m| MerchantInfo {
            raw_name: m.name.clone().unwrap_or_default(),
            normalized_name: None,
            entity_id: None,
            mcc: m.category_code.clone(),
            country: None,
            confidence: 0.8,
        });

        // Status: Tink sandbox may return BOOKED / PENDING / UNDEFINED
        let status = match t.status.as_deref() {
            Some("BOOKED") => TransactionStatus::Unmatched,
            Some("PENDING") => TransactionStatus::Unmatched,
            _ => TransactionStatus::Unmatched,
        };

        Transaction {
            id: Uuid::new_v4(),
            external_id: Some(t.id),
            amount,
            currency: t.amount.currency_code,
            timestamp,
            counterparty,
            merchant,
            invoice_id: None,
            payment_rail: PaymentRail::SepaTransfer,
            jurisdiction: "SE".to_string(),
            tax_amount: None,
            tax_rate: None,
            account_id: Some(raw_account_id.to_string()),
            source: IntegrationSource::Tink,
            status,
            confidence: 0.9,
            audit_trail: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// GET with bearer token; returns raw Response.
    async fn authed_get(&self, token: &str, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .with_context(|| format!("GET {} network error", url))
    }
}

// ── BankingProvider ───────────────────────────────────────────────────────────

#[async_trait]
impl BankingProvider for TinkConnector {
    fn provider_id(&self) -> &str {
        "tink"
    }

    fn supported_banks(&self) -> Vec<String> {
        vec![
            "Swedbank".to_string(),
            "Handelsbanken".to_string(),
            "SEB".to_string(),
            "Nordea".to_string(),
        ]
    }

    /// Fetch transactions for an account.
    ///
    /// `account_id` must be in the form `"user:<tink_user_id>:acc:<tink_account_id>"`.
    async fn stream_transactions(
        &self,
        account_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>> {
        let (user_id, tink_account_id) = Self::parse_account_id(account_id)?;
        let token = self.user_token(&user_id).await?;

        let from_str = from.format("%Y-%m-%d");
        let to_str = to.format("%Y-%m-%d");

        let mut all: Vec<Transaction> = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut path = format!(
                "/data/v2/transactions?accountIdIn={}&bookedDateGte={}&bookedDateLte={}",
                tink_account_id, from_str, to_str
            );

            if let Some(ref pt) = page_token {
                path.push_str(&format!("&pageToken={}", pt));
            }

            let resp = self.authed_get(&token, &path).await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("GET /data/v2/transactions failed {}: {}", status, body);
            }

            let page: TinkTransactionsResponse = resp
                .json()
                .await
                .context("Cannot parse /data/v2/transactions response")?;

            for tx in page.transactions {
                all.push(self.map_transaction(tx, account_id));
            }

            match page.next_page_token {
                Some(ref pt) if !pt.is_empty() => {
                    page_token = Some(pt.clone());
                }
                _ => break,
            }
        }

        Ok(all)
    }

    /// Fetch balances for an account.
    ///
    /// `account_id` must be in the form `"user:<tink_user_id>:acc:<tink_account_id>"`.
    async fn fetch_balances(&self, account_id: &str) -> Result<Vec<Balance>> {
        let (user_id, tink_account_id) = Self::parse_account_id(account_id)?;
        let token = self.user_token(&user_id).await?;

        let path = format!(
            "/data/v2/balances?accountIdIn={}",
            tink_account_id
        );

        let resp = self.authed_get(&token, &path).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GET /data/v2/balances failed {}: {}", status, body);
        }

        let body: TinkBalancesResponse = resp
            .json()
            .await
            .context("Cannot parse /data/v2/balances response")?;

        let balances = body
            .balances
            .into_iter()
            .map(|b| Balance {
                account_id: b
                    .account_id
                    .unwrap_or_else(|| tink_account_id.clone()),
                amount: Self::parse_tink_decimal(&b.amount.value),
                currency: b.amount.currency_code,
                timestamp: Utc::now(),
            })
            .collect();

        Ok(balances)
    }

    /// Verify a payment by looking up the transaction by its Tink transaction ID.
    ///
    /// `payment_id` format: `"user:<uid>:tx:<tink_tx_id>"`
    /// Falls back to treating the raw id as a transaction id with a client-level token.
    async fn verify_payment(&self, payment_id: &str) -> Result<PaymentStatus> {
        // Try to parse "user:<uid>:tx:<tx_id>" first; else use client token with the raw id.
        let (token, tx_id) = if payment_id.starts_with("user:") {
            let parts: Vec<&str> = payment_id.splitn(4, ':').collect();
            if parts.len() == 4 && parts[2] == "tx" {
                let user_tok = self.user_token(parts[1]).await?;
                (user_tok, parts[3].to_string())
            } else {
                anyhow::bail!(
                    "Invalid payment_id format. Expected 'user:<uid>:tx:<tx_id>', got: {}",
                    payment_id
                );
            }
        } else {
            // Bare transaction ID: use client token (limited, may return 403 in production)
            let tok = self.client_token().await?;
            (tok, payment_id.to_string())
        };

        let path = format!("/data/v2/transactions/{}", tx_id);
        let resp = self.authed_get(&token, &path).await?;

        if resp.status().as_u16() == 404 {
            return Ok(PaymentStatus::Failed);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GET /data/v2/transactions/{} failed {}: {}", tx_id, status, body);
        }

        // Tink returns a single transaction object; status field drives our result.
        let tx: TinkTransaction = resp
            .json()
            .await
            .with_context(|| format!("Cannot parse transaction {}", tx_id))?;

        let payment_status = match tx.status.as_deref() {
            Some("BOOKED") => PaymentStatus::Completed,
            Some("PENDING") => PaymentStatus::Pending,
            _ => PaymentStatus::Pending,
        };

        Ok(payment_status)
    }

    /// Liveness check: attempt to obtain a client_credentials token.
    async fn health_check(&self) -> Result<ProviderHealth> {
        let t0 = Instant::now();

        match self.client_token().await {
            Ok(_) => Ok(ProviderHealth {
                provider_id: self.provider_id().to_string(),
                status: HealthStatus::Healthy,
                latency_ms: t0.elapsed().as_millis() as u64,
                message: None,
            }),
            Err(e) => Ok(ProviderHealth {
                provider_id: self.provider_id().to_string(),
                status: HealthStatus::Down,
                latency_ms: t0.elapsed().as_millis() as u64,
                message: Some(format!("Auth failed: {}", e)),
            }),
        }
    }
}
