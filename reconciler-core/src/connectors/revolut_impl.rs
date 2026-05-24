//! Revolut Business connector for Kvittovalvet / Reconciler.
//!
//! Auth flow:
//!   1. Build a signed RS256 JWT assertion (client_assertion).
//!   2. POST /auth/token with grant_type=authorization_code + refresh_token as `code`.
//!   3. Cache the resulting access_token; re-issue when < 60 s remains.
//!
//! Implements `BankingProvider` from `crate::connectors`.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::connectors::{Balance, BankingProvider, HealthStatus, ProviderHealth};
use crate::models::*;

// ── Constants ────────────────────────────────────────────────────────────────

const BASE_URL: &str = "https://b2b.revolut.com/api/1.0";
const SECRETS_PATH: &str = "/home/userwinston/.openclaw/secrets/revolut-business-api.json";
const PRIVATE_KEY_PATH_DEFAULT: &str =
    "/home/userwinston/.openclaw/secrets/revolut-private.pem";

// ── Secrets file schema ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SecretsFile {
    revolut_business: RevolutSecrets,
}

#[derive(Debug, Deserialize)]
struct RevolutSecrets {
    client_id: String,
    refresh_token: String,
    #[serde(default)]
    private_key_path: Option<String>,
}

// ── Revolut REST response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct RevolutAccount {
    id: String,
    balance: f64,
    currency: String,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RevolutTransaction {
    id: String,
    #[serde(rename = "type")]
    tx_type: String,
    state: String,
    created_at: String,
    amount: f64,
    currency: String,
    description: Option<String>,
    #[serde(default)]
    legs: Vec<RevolutLeg>,
    merchant: Option<RevolutMerchant>,
}

#[derive(Debug, Deserialize)]
struct RevolutLeg {
    counterparty: Option<RevolutCounterparty>,
}

#[derive(Debug, Deserialize)]
struct RevolutCounterparty {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RevolutMerchant {
    name: Option<String>,
    category_code: Option<String>,
    country: Option<String>,
}

// ── JWT claims ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: i64,
    exp: i64,
}

// ── Token cache ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: DateTime<Utc>,
}

// ── Connector ─────────────────────────────────────────────────────────────────

pub struct RevolutConnector {
    client: reqwest::Client,
    client_id: String,
    refresh_token: String,
    private_key_pem: String,
    cache: Arc<RwLock<Option<CachedToken>>>,
}

impl RevolutConnector {
    /// Construct a connector by reading credentials from disk.
    pub async fn new() -> Result<Self> {
        // --- Read secrets JSON -----------------------------------------------
        let raw = tokio::fs::read_to_string(SECRETS_PATH)
            .await
            .with_context(|| format!("Cannot read {}", SECRETS_PATH))?;

        let secrets: SecretsFile =
            serde_json::from_str(&raw).context("Cannot parse revolut-business-api.json")?;

        let RevolutSecrets {
            client_id,
            refresh_token,
            private_key_path,
        } = secrets.revolut_business;

        // --- Read private key ------------------------------------------------
        let key_path = private_key_path
            .as_deref()
            .unwrap_or(PRIVATE_KEY_PATH_DEFAULT);

        let private_key_pem = tokio::fs::read_to_string(key_path)
            .await
            .with_context(|| format!("Cannot read private key from {}", key_path))?;

        // --- HTTP client -----------------------------------------------------
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            client_id,
            refresh_token,
            private_key_pem,
            cache: Arc::new(RwLock::new(None)),
        })
    }

    // ── Auth helpers ──────────────────────────────────────────────────────────

    fn build_jwt(&self) -> Result<String> {
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            iss: self.client_id.clone(),
            sub: self.client_id.clone(),
            aud: "https://revolut.com".to_string(),
            iat: now,
            exp: now + 3600,
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(self.private_key_pem.as_bytes())
            .context("Failed to parse RSA private key")?;

        encode(&header, &claims, &key).context("Failed to sign JWT assertion")
    }

    async fn access_token(&self) -> Result<String> {
        // Return cached token when still valid (with 60 s buffer).
        {
            let guard = self.cache.read().await;
            if let Some(ref cached) = *guard {
                if cached.expires_at > Utc::now() + Duration::seconds(60) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Issue new token via JWT assertion flow.
        let jwt = self.build_jwt()?;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", self.refresh_token.as_str()),
            (
                "client_assertion_type",
                "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
            ),
            ("client_assertion", jwt.as_str()),
        ];

        let resp = self
            .client
            .post(&format!("{}/auth/token", BASE_URL))
            .form(&params)
            .send()
            .await
            .context("POST /auth/token network error")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Revolut auth failed {}: {}", status, body);
        }

        let tr: TokenResponse = resp.json().await.context("Cannot parse token response")?;
        let expires_at = Utc::now() + Duration::seconds(tr.expires_in as i64);
        let token = tr.access_token.clone();

        {
            let mut guard = self.cache.write().await;
            *guard = Some(CachedToken {
                token: tr.access_token,
                expires_at,
            });
        }

        Ok(token)
    }

    // ── Generic authenticated GET ─────────────────────────────────────────────

    async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let token = self.access_token().await?;
        let url = format!("{}{}", BASE_URL, path);

        self.client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .with_context(|| format!("GET {} network error", url))
    }

    // ── Mapping helpers ───────────────────────────────────────────────────────

    fn map_tx(&self, rt: RevolutTransaction, account_id: &str) -> Transaction {
        let timestamp = DateTime::parse_from_rfc3339(&rt.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let amount = Decimal::from_str(&format!("{:.10}", rt.amount)).unwrap_or_default();

        // Prefer description from first leg, fall back to top-level.
        let counterparty = rt
            .legs
            .iter()
            .find_map(|l| l.counterparty.as_ref()?.name.as_ref())
            .map(|name| Party {
                id: None,
                name: name.clone(),
                normalized_name: None,
                registration_number: None,
                vat_number: None, country: None, entity_confidence: 0.8,
            });

        let merchant = rt.merchant.as_ref().map(|m| MerchantInfo {
            raw_name: m.name.clone().unwrap_or_default(),
            normalized_name: None,
            entity_id: None,
            mcc: m.category_code.clone(),
            country: m.country.clone(),
            confidence: 0.85,
        });

        let status = match rt.state.as_str() {
            "completed" => TransactionStatus::Matched,
            "pending" | "processing" => TransactionStatus::Unmatched,
            "failed" | "declined" | "reverted" => TransactionStatus::Disputed,
            _ => TransactionStatus::Unmatched,
        };

        let payment_rail = match rt.tx_type.as_str() {
            "card_payment" | "card_refund" | "card_chargeback" => PaymentRail::Card,
            "transfer" | "topup" | "exchange" => PaymentRail::SepaTransfer,
            _ => PaymentRail::Unknown,
        };

        Transaction {
            id: Uuid::new_v4(),
            external_id: Some(rt.id),
            amount,
            currency: rt.currency,
            timestamp,
            counterparty,
            merchant,
            invoice_id: None,
            payment_rail,
            jurisdiction: "EU".to_string(),
            tax_amount: None,
            tax_rate: None,
            account_id: Some(account_id.to_string()),
            source: IntegrationSource::Revolut,
            status,
            confidence: 0.95,
            audit_trail: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

// ── BankingProvider impl ──────────────────────────────────────────────────────

#[async_trait]
impl BankingProvider for RevolutConnector {
    fn provider_id(&self) -> &str {
        "revolut-business"
    }

    fn supported_banks(&self) -> Vec<String> {
        vec!["Revolut Business".to_string()]
    }

    /// Fetch all transactions for `account_id` in the given date window.
    ///
    /// Pass `"all"` as `account_id` to retrieve transactions across all accounts
    /// (the Revolut API returns all if no account filter is applied).
    async fn stream_transactions(
        &self,
        account_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>> {
        let from_str = from.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let to_str = to.format("%Y-%m-%dT%H:%M:%S%.3fZ");

        // Build query; Revolut ignores unknown params so including account is safe.
        let path = if account_id == "all" {
            format!(
                "/transactions?from={}&to={}&count=1000",
                from_str, to_str
            )
        } else {
            format!(
                "/transactions?from={}&to={}&count=1000&account={}",
                from_str, to_str, account_id
            )
        };

        let resp = self.get(&path).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GET /transactions failed {}: {}", status, body);
        }

        let raw: Vec<RevolutTransaction> = resp
            .json()
            .await
            .context("Cannot parse /transactions response")?;

        Ok(raw.into_iter().map(|rt| self.map_tx(rt, account_id)).collect())
    }

    /// Fetch current balances.
    ///
    /// Pass `"all"` as `account_id` to return balances for every active account.
    async fn fetch_balances(&self, account_id: &str) -> Result<Vec<Balance>> {
        let resp = self.get("/accounts").await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GET /accounts failed {}: {}", status, body);
        }

        let accounts: Vec<RevolutAccount> = resp
            .json()
            .await
            .context("Cannot parse /accounts response")?;

        let balances = accounts
            .into_iter()
            .filter(|a| {
                // Only active accounts; filter by id unless caller wants all.
                let active = a.state.as_deref().map_or(true, |s| s == "active");
                let matches = account_id == "all" || a.id == account_id;
                active && matches
            })
            .map(|a| Balance {
                account_id: a.id,
                amount: Decimal::from_str(&format!("{:.10}", a.balance)).unwrap_or_default(),
                currency: a.currency,
                timestamp: Utc::now(),
            })
            .collect();

        Ok(balances)
    }

    /// Verify a single transaction/payment by its Revolut transaction ID.
    async fn verify_payment(&self, payment_id: &str) -> Result<PaymentStatus> {
        let path = format!("/transaction/{}", payment_id);
        let resp = self.get(&path).await?;

        if resp.status().as_u16() == 404 {
            return Ok(PaymentStatus::Failed);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GET /transaction/{} failed {}: {}", payment_id, status, body);
        }

        let tx: RevolutTransaction = resp
            .json()
            .await
            .with_context(|| format!("Cannot parse transaction {}", payment_id))?;

        let status = match tx.state.as_str() {
            "completed" => PaymentStatus::Completed,
            "pending" => PaymentStatus::Pending,
            "processing" => PaymentStatus::Processing,
            "failed" | "declined" | "reverted" => PaymentStatus::Failed,
            _ => PaymentStatus::Pending,
        };

        Ok(status)
    }

    /// Lightweight liveness check: authenticate + fetch accounts.
    async fn health_check(&self) -> Result<ProviderHealth> {
        let t0 = Instant::now();

        // First ensure we can get a token (covers auth failures).
        let token_result = self.access_token().await;
        if let Err(e) = token_result {
            return Ok(ProviderHealth {
                provider_id: self.provider_id().to_string(),
                status: HealthStatus::Down,
                latency_ms: t0.elapsed().as_millis() as u64,
                message: Some(format!("Auth failed: {}", e)),
            });
        }

        let resp = self.get("/accounts").await;
        let latency_ms = t0.elapsed().as_millis() as u64;

        match resp {
            Ok(r) if r.status().is_success() => Ok(ProviderHealth {
                provider_id: self.provider_id().to_string(),
                status: HealthStatus::Healthy,
                latency_ms,
                message: None,
            }),
            Ok(r) => Ok(ProviderHealth {
                provider_id: self.provider_id().to_string(),
                status: HealthStatus::Degraded,
                latency_ms,
                message: Some(format!("Unexpected HTTP {}", r.status())),
            }),
            Err(e) => Ok(ProviderHealth {
                provider_id: self.provider_id().to_string(),
                status: HealthStatus::Down,
                latency_ms,
                message: Some(e.to_string()),
            }),
        }
    }
}

impl RevolutConnector {
    /// Sync constructor for use in non-async contexts (reads credentials synchronously)
    pub fn from_env() -> anyhow::Result<Self> {
        let secrets_path = std::env::var("REVOLUT_SECRETS_PATH")
            .unwrap_or_else(|_| "/home/userwinston/.openclaw/secrets/revolut-business-api.json".to_string());
        let key_path = std::env::var("REVOLUT_KEY_PATH")
            .unwrap_or_else(|_| "/home/userwinston/.openclaw/secrets/revolut-private.pem".to_string());

        let raw = std::fs::read_to_string(&secrets_path)
            .with_context(|| format!("Cannot read {}", secrets_path))?;
        let secrets: SecretsFile = serde_json::from_str(&raw)
            .context("Cannot parse revolut-business-api.json")?;
        let creds = secrets.revolut_business;

        let private_key_pem = std::fs::read_to_string(&key_path)
            .with_context(|| format!("Cannot read {}", key_path))?;

        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            client_id: creds.client_id,
            refresh_token: creds.refresh_token,
            private_key_pem,
            cache: Arc::new(RwLock::new(None)),
        })
    }
}
