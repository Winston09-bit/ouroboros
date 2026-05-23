// revolut.rs — Revolut Business Banking Connector
// Implements BankingProvider trait with JWT auth, multi-currency, and webhook handling.

use crate::canonical::{
    CanonicalError, Money, Transaction, TransactionDirection,
};
use crate::traits::BankingProvider;
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode as ReqwestStatus};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ─── Configuration ────────────────────────────────────────────────────────────

const REVOLUT_BASE_URL: &str = "https://b2b.revolut.com/api/1.0";
const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 500;
const MAX_TRANSACTIONS_PER_PAGE: u32 = 1000;

#[derive(Debug, Clone)]
pub struct RevolutConfig {
    /// PEM-encoded private key for JWT signing
    pub private_key_pem: String,
    /// Client ID from Revolut Business Developer Portal
    pub client_id: String,
    /// JWT issuer (redirect URI registered with Revolut)
    pub jwt_issuer: String,
    /// Refresh token from OAuth2 consent
    pub refresh_token: String,
}

// ─── Token Cache ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct TokenCache {
    access_token: String,
    expires_at: DateTime<Utc>,
}

impl TokenCache {
    fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at - chrono::Duration::seconds(60)
    }
}

// ─── Raw API Types — Revolut Business ────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutAccount {
    pub id: String,
    pub name: String,
    pub balance: f64,
    pub currency: String,
    pub state: String,
    #[serde(rename = "public_token")]
    pub public_token: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutTransaction {
    pub id: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub request_id: Option<String>,
    pub state: String,
    pub reason_code: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub completed_at: Option<String>,
    pub description: Option<String>,
    pub merchant: Option<RevolutMerchant>,
    pub legs: Vec<RevolutLeg>,
    pub reference: Option<String>,
    pub counterparty: Option<RevolutCounterparty>,
    pub related_transaction_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutLeg {
    pub id: String,
    pub account_id: String,
    pub amount: f64,
    pub currency: String,
    pub balance: Option<f64>,
    pub description: Option<String>,
    pub bill_amount: Option<f64>,
    pub bill_currency: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutMerchant {
    pub name: Option<String>,
    pub city: Option<String>,
    pub country_code: Option<String>,
    pub category_code: Option<String>,
    pub mcc: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutCounterparty {
    pub id: Option<String>,
    pub name: Option<String>,
    pub account_type: Option<String>,
    pub account_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutPayment {
    pub id: String,
    pub state: String,
    #[serde(rename = "type")]
    pub payment_type: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub completed_at: Option<String>,
    pub request_id: Option<String>,
    pub reference: Option<String>,
    pub counterparty: Option<RevolutPaymentCounterparty>,
    pub legs: Vec<RevolutLeg>,
    pub reason_code: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutPaymentCounterparty {
    pub account_id: Option<String>,
    pub account_no: Option<String>,
    pub iban: Option<String>,
    pub sort_code: Option<String>,
    pub name: Option<String>,
    pub bank_country: Option<String>,
}

// ─── Webhook Types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "event")]
pub enum RevolutWebhookEvent {
    #[serde(rename = "TransactionCreated")]
    TransactionCreated(RevolutWebhookPayload),
    #[serde(rename = "TransactionCompleted")]
    TransactionCompleted(RevolutWebhookPayload),
    #[serde(rename = "TransactionFailed")]
    TransactionFailed(RevolutWebhookPayload),
    #[serde(rename = "PaymentCompleted")]
    PaymentCompleted(RevolutWebhookPayload),
    #[serde(rename = "PaymentFailed")]
    PaymentFailed(RevolutWebhookPayload),
}

#[derive(Debug, Deserialize, Clone)]
pub struct RevolutWebhookPayload {
    pub timestamp: String,
    pub data: serde_json::Value,
}

// ─── Balance Type ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub account_id: String,
    pub name: String,
    pub balance: Money,
    pub state: String,
}

// ─── Token Exchange ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

// ─── Connector ────────────────────────────────────────────────────────────────

pub struct RevolutConnector {
    http: Client,
    config: RevolutConfig,
    token_cache: tokio::sync::Mutex<Option<TokenCache>>,
    /// Mutable refresh token (Revolut rotates refresh tokens)
    current_refresh_token: tokio::sync::Mutex<String>,
}

impl RevolutConnector {
    pub fn new(config: RevolutConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("reconciler/1.0")
            .build()
            .expect("Failed to build HTTP client");

        let refresh_token = config.refresh_token.clone();

        Self {
            http,
            config,
            token_cache: tokio::sync::Mutex::new(None),
            current_refresh_token: tokio::sync::Mutex::new(refresh_token),
        }
    }

    // ── OAuth2 Token Management ───────────────────────────────────────────────

    async fn ensure_token(&self) -> Result<String, CanonicalError> {
        let mut cache = self.token_cache.lock().await;
        if let Some(ref t) = *cache {
            if !t.is_expired() {
                return Ok(t.access_token.clone());
            }
        }

        let refresh_token = self.current_refresh_token.lock().await.clone();
        let token = self.exchange_refresh_token(&refresh_token).await?;

        // If Revolut issued a new refresh token, store it
        if let Some(new_refresh) = &token.refresh_token {
            *self.current_refresh_token.lock().await = new_refresh.clone();
        }

        let access_token = token.access_token.clone();
        *cache = Some(TokenCache {
            access_token: token.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(token.expires_in),
        });

        info!("Revolut token refreshed, expires in {}s", token.expires_in);
        Ok(access_token)
    }

    async fn exchange_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, CanonicalError> {
        // Build client assertion JWT for Revolut's private_key_jwt auth
        let client_assertion = self.build_client_assertion()?;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
            ("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer"),
            ("client_assertion", &client_assertion),
        ];

        let resp = self
            .http
            .post("https://b2b.revolut.com/api/1.0/auth/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(CanonicalError::AuthError(format!(
                "Revolut token refresh failed: HTTP {status} — {body}"
            )));
        }

        resp.json::<TokenResponse>()
            .await
            .map_err(|e| CanonicalError::ParseError(e.to_string()))
    }

    /// Build a client_assertion JWT signed with the configured private key.
    fn build_client_assertion(&self) -> Result<String, CanonicalError> {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        #[derive(Serialize)]
        struct Claims {
            iss: String,
            sub: String,
            aud: String,
            exp: i64,
            iat: i64,
            jti: String,
        }

        let now = Utc::now().timestamp();
        let claims = Claims {
            iss: self.config.jwt_issuer.clone(),
            sub: self.config.client_id.clone(),
            aud: "https://revolut.com".to_string(),
            exp: now + 300, // 5 minutes
            iat: now,
            jti: Uuid::new_v4().to_string(),
        };

        let key = EncodingKey::from_rsa_pem(self.config.private_key_pem.as_bytes())
            .map_err(|e| CanonicalError::AuthError(format!("Invalid Revolut private key: {e}")))?;

        let mut header = Header::new(Algorithm::RS256);
        header.typ = Some("JWT".to_string());

        encode(&header, &claims, &key)
            .map_err(|e| CanonicalError::AuthError(format!("JWT signing failed: {e}")))
    }

    // ── HTTP Helpers with Retry ───────────────────────────────────────────────

    async fn get_with_retry<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, CanonicalError> {
        let url = format!("{REVOLUT_BASE_URL}{path}");
        let mut attempt = 0u32;

        loop {
            let token = self.ensure_token().await?;

            let resp = self
                .http
                .get(&url)
                .bearer_auth(&token)
                .query(query)
                .send()
                .await
                .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

            let status = resp.status();

            match status {
                s if s == ReqwestStatus::TOO_MANY_REQUESTS => {
                    attempt += 1;
                    if attempt > MAX_RETRIES {
                        return Err(CanonicalError::RateLimited(
                            "Revolut: exceeded max retries".into(),
                        ));
                    }
                    let retry_after = resp
                        .headers()
                        .get("Retry-After")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or_else(|| {
                            (INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1)).min(30_000)
                        });
                    warn!(
                        "Revolut 429 rate limit, backing off {}ms (attempt {}/{})",
                        retry_after, attempt, MAX_RETRIES
                    );
                    sleep(Duration::from_millis(retry_after)).await;
                    continue;
                }
                s if s == ReqwestStatus::UNAUTHORIZED => {
                    *self.token_cache.lock().await = None;
                    attempt += 1;
                    if attempt > 2 {
                        return Err(CanonicalError::AuthError(
                            "Revolut: 401 after token refresh".into(),
                        ));
                    }
                    continue;
                }
                s if s.is_server_error() => {
                    attempt += 1;
                    if attempt > MAX_RETRIES {
                        let body = resp.text().await.unwrap_or_default();
                        return Err(CanonicalError::ApiError(format!(
                            "Revolut server error on {path}: {body}"
                        )));
                    }
                    let delay = (INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1)).min(30_000);
                    warn!("Revolut 5xx, retry {}ms (attempt {})", delay, attempt);
                    sleep(Duration::from_millis(delay)).await;
                    continue;
                }
                s if !s.is_success() => {
                    let body = resp.text().await.unwrap_or_default();
                    return Err(CanonicalError::ApiError(format!(
                        "Revolut GET {path} failed: HTTP {status} — {body}"
                    )));
                }
                _ => {}
            }

            return resp
                .json::<T>()
                .await
                .map_err(|e| CanonicalError::ParseError(format!("Revolut parse: {e}")));
        }
    }

    // ── Canonical Mapping ─────────────────────────────────────────────────────

    /// Convert a Revolut transaction to Canonical Transaction.
    /// Multi-currency: use the first leg's amount and currency.
    pub fn to_canonical_transaction(tx: &RevolutTransaction) -> Transaction {
        let leg = tx.legs.first();
        let (amount, currency, direction) = if let Some(leg) = leg {
            let dir = if leg.amount >= 0.0 {
                TransactionDirection::Incoming
            } else {
                TransactionDirection::Outgoing
            };
            let abs_amount = Decimal::from_str(&format!("{:.6}", leg.amount.abs()))
                .unwrap_or_default();
            (abs_amount, leg.currency.clone(), dir)
        } else {
            (Decimal::ZERO, "USD".into(), TransactionDirection::Incoming)
        };

        let counterparty_name = tx
            .counterparty
            .as_ref()
            .and_then(|c| c.name.clone())
            .or_else(|| tx.merchant.as_ref().and_then(|m| m.name.clone()));

        let counterparty_id = tx
            .counterparty
            .as_ref()
            .and_then(|c| c.id.clone());

        let posted_at = tx
            .completed_at
            .as_deref()
            .or(Some(tx.created_at.as_str()))
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Transaction {
            id: Uuid::new_v4().to_string(),
            external_id: tx.id.clone(),
            source: "revolut".into(),
            direction,
            amount: Money { amount, currency },
            counterparty_id,
            counterparty_name,
            description: tx.description.clone(),
            posted_at,
            raw: serde_json::to_value(tx).unwrap_or_default(),
        }
    }

    /// Stream transactions by paginating with cursor-based pagination.
    pub async fn stream_transactions(
        &self,
        account_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<RevolutTransaction>, CanonicalError> {
        let from_ts = from.timestamp_millis();
        let to_ts = to.timestamp_millis();
        let count = MAX_TRANSACTIONS_PER_PAGE.to_string();

        let mut all: Vec<RevolutTransaction> = Vec::new();
        let mut last_id: Option<String> = None;

        loop {
            let mut query: Vec<(&str, String)> = vec![
                ("from".to_string(), from_ts.to_string()),
                ("to".to_string(), to_ts.to_string()),
                ("count".to_string(), count.clone()),
            ];

            if let Some(ref cursor) = last_id {
                query.push(("last_retrieved_id".to_string(), cursor.clone()));
            }

            // Convert to &str slices for the helper
            let query_refs: Vec<(&str, &str)> =
                query.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

            let path = format!("/accounts/{account_id}/transactions");
            let batch: Vec<RevolutTransaction> =
                self.get_with_retry(&path, &query_refs).await?;

            let fetched = batch.len();
            debug!(
                "Revolut stream_transactions: batch={fetched}, account={account_id}"
            );

            if fetched == 0 {
                break;
            }

            last_id = batch.last().map(|t| t.id.clone());
            all.extend(batch);

            if fetched < MAX_TRANSACTIONS_PER_PAGE as usize {
                break;
            }
        }

        info!(
            "Revolut stream_transactions: {} total for account={}",
            all.len(),
            account_id
        );
        Ok(all)
    }
}

// ─── BankingProvider Trait Impl ───────────────────────────────────────────────

#[async_trait]
impl BankingProvider for RevolutConnector {
    async fn fetch_balances(&self) -> Result<Vec<AccountBalance>, CanonicalError> {
        let accounts: Vec<RevolutAccount> =
            self.get_with_retry("/accounts", &[]).await?;

        let balances: Vec<AccountBalance> = accounts
            .iter()
            .filter(|a| a.state == "active")
            .map(|a| AccountBalance {
                account_id: a.id.clone(),
                name: a.name.clone(),
                balance: Money {
                    amount: Decimal::from_str(&format!("{:.6}", a.balance))
                        .unwrap_or_default(),
                    currency: a.currency.clone(),
                },
                state: a.state.clone(),
            })
            .collect();

        info!("Revolut fetch_balances: {} active accounts", balances.len());
        Ok(balances)
    }

    async fn fetch_transactions(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>, CanonicalError> {
        // Get all active accounts first
        let accounts: Vec<RevolutAccount> =
            self.get_with_retry("/accounts", &[]).await?;

        let mut all_transactions: Vec<Transaction> = Vec::new();

        for account in accounts.iter().filter(|a| a.state == "active") {
            let raw = self.stream_transactions(&account.id, from, to).await?;
            let canonical: Vec<Transaction> =
                raw.iter().map(Self::to_canonical_transaction).collect();
            all_transactions.extend(canonical);
        }

        info!(
            "Revolut fetch_transactions: {} total canonical transactions",
            all_transactions.len()
        );
        Ok(all_transactions)
    }

    async fn verify_payment(&self, payment_id: &str) -> Result<RevolutPayment, CanonicalError> {
        let path = format!("/payments/{payment_id}");
        let payment: RevolutPayment = self.get_with_retry(&path, &[]).await?;
        debug!("Revolut verify_payment: id={payment_id} state={}", payment.state);
        Ok(payment)
    }
}

// ─── Webhook Handler ──────────────────────────────────────────────────────────

pub struct RevolutWebhookHandler {
    connector: Arc<RevolutConnector>,
    /// Shared event sink for incoming webhook events
    event_tx: tokio::sync::broadcast::Sender<RevolutWebhookEvent>,
}

impl RevolutWebhookHandler {
    pub fn new(
        connector: Arc<RevolutConnector>,
        event_tx: tokio::sync::broadcast::Sender<RevolutWebhookEvent>,
    ) -> Self {
        Self { connector, event_tx }
    }

    /// Axum handler: POST /webhooks/revolut
    pub async fn handle(
        State(handler): State<Arc<RevolutWebhookHandler>>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        // Revolut doesn't sign webhook payloads by default in Business API v1.0,
        // but we validate the content type and log the event.
        let content_type = headers
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") {
            return (StatusCode::BAD_REQUEST, "Expected application/json").into_response();
        }

        match handler.parse_event(&body) {
            Ok(event) => {
                info!(
                    "Revolut webhook received: {:?}",
                    std::mem::discriminant(&event)
                );
                if let Err(e) = handler.event_tx.send(event) {
                    error!("Failed to broadcast Revolut webhook event: {e}");
                }
                StatusCode::OK.into_response()
            }
            Err(e) => {
                error!("Failed to parse Revolut webhook: {e}");
                (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response()
            }
        }
    }

    /// Parse raw webhook body into a typed event.
    pub fn parse_event(&self, body: &[u8]) -> Result<RevolutWebhookEvent, CanonicalError> {
        serde_json::from_slice(body)
            .map_err(|e| CanonicalError::ParseError(format!("Revolut webhook parse: {e}")))
    }

    /// Convert a webhook event to a canonical transaction (if applicable).
    pub fn event_to_transaction(event: &RevolutWebhookEvent) -> Option<Transaction> {
        let payload = match event {
            RevolutWebhookEvent::TransactionCreated(p)
            | RevolutWebhookEvent::TransactionCompleted(p)
            | RevolutWebhookEvent::PaymentCompleted(p) => p,
            _ => return None,
        };

        // Try to deserialize the inner data as a RevolutTransaction
        serde_json::from_value::<RevolutTransaction>(payload.data.clone())
            .ok()
            .map(|tx| RevolutConnector::to_canonical_transaction(&tx))
    }
}
