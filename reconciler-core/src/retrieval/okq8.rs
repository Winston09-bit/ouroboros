//! OKQ8 kort/app receipt retrieval provider.
//!
//! OKQ8 exposes purchase history in their "OKQ8 Kort" app.
//! The app communicates with `https://api.okq8.se/` (unofficial, TLS-pinned).
//!
//! Auth: username + password → Bearer JWT via POST /auth/token.
//!
//! This provider is structurally complete; flesh out the HTTP calls once you
//! have a test account or an official API agreement.

use anyhow::{bail, Result};
use async_trait::async_trait;
use uuid::Uuid;

use super::{
    ReceiptQuery, ReceiptRetrievalProvider, RetrievalCapabilities, RetrievedReceipt,
};

const MERCHANT_ID: &str = "okq8";
const AUTH_ENDPOINT: &str     = "https://api.okq8.se/auth/token";
const RECEIPTS_ENDPOINT: &str = "https://api.okq8.se/purchases";

#[allow(dead_code)]
pub struct Okq8Provider {
    username:     Option<String>,
    password:     Option<String>,
    /// Cached JWT after successful auth; refresh on 401.
    cached_token: tokio::sync::Mutex<Option<String>>,
    http:         reqwest::Client,
}

impl Okq8Provider {
    pub fn new(username: Option<String>, password: Option<String>) -> Self {
        Self {
            username,
            password,
            cached_token: tokio::sync::Mutex::new(None),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ReceiptRetrievalProvider for Okq8Provider {
    fn merchant_id(&self) -> &str {
        MERCHANT_ID
    }

    fn capabilities(&self) -> RetrievalCapabilities {
        RetrievalCapabilities {
            supports_email_search:   false,
            supports_app_api:        true,
            supports_web_scrape:     false,
            supports_postal_request: false,
            typical_latency_seconds: 3,
        }
    }

    async fn search_receipts(&self, query: &ReceiptQuery) -> Result<Vec<RetrievedReceipt>> {
        if self.username.is_none() {
            tracing::info!(
                merchant_id = MERCHANT_ID,
                auth_endpoint     = AUTH_ENDPOINT,
                receipts_endpoint = RECEIPTS_ENDPOINT,
                amount = %query.amount,
                date   = %query.date,
                "UNIMPLEMENTED: would POST credentials to OKQ8 auth, then GET purchases; \
                 no credentials configured"
            );
            return Ok(vec![]);
        }

        // TODO:
        // 1. POST AUTH_ENDPOINT with { username, password } → { access_token }
        // 2. GET RECEIPTS_ENDPOINT?from=…&to=…&amount=…  with Bearer token
        // 3. Map response items → RetrievedReceipt
        tracing::info!(
            merchant_id = MERCHANT_ID,
            "OKQ8 credentials present but fetch logic not yet implemented"
        );
        Ok(vec![])
    }

    async fn fetch_receipt(&self, receipt_ref: &str) -> Result<RetrievedReceipt> {
        bail!("OKQ8 fetch_receipt({receipt_ref}): not yet implemented");
    }

    async fn health_check(&self) -> Result<bool> {
        match self.http.get("https://www.okq8.se/").send().await {
            Ok(r) => {
                let up = r.status().as_u16() < 500;
                tracing::info!(merchant_id = MERCHANT_ID, status = %r.status(), "health check");
                Ok(up)
            }
            Err(e) => {
                tracing::warn!(merchant_id = MERCHANT_ID, error = %e, "health check failed");
                Ok(false)
            }
        }
    }
}

// ─── Placeholder response shapes (fill in from real traffic) ─────────────────

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Okq8AuthResponse {
    access_token: String,
    expires_in:   u64,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Okq8Purchase {
    id:       String,
    amount:   f64,
    currency: String,
    date:     String,
    station:  Option<String>,
}

impl From<Okq8Purchase> for RetrievedReceipt {
    fn from(p: Okq8Purchase) -> Self {
        RetrievedReceipt {
            id:          Uuid::new_v4(),
            merchant_id: MERCHANT_ID.to_string(),
            amount:      rust_decimal::Decimal::try_from(p.amount).unwrap_or_default(),
            currency:    p.currency,
            date:        chrono::Utc::now(), // parse p.date properly when implementing
            items:       vec![],
            vat_amount:  None,
            vat_rate:    None,
            raw_pdf:     None,
            raw_html:    None,
            source:      MERCHANT_ID.to_string(),
            confidence:  0.85,
        }
    }
}
