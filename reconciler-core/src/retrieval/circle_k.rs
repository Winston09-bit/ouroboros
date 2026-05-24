//! Circle K Extraclub receipt retrieval provider.
//!
//! Circle K's Extraclub app communicates with `https://api.circlek.com/` and
//! regional variants (`https://api.circlek.se/`).  The loyalty programme
//! records fuel + shop purchases tied to the membership card.
//!
//! Auth: email + password → JWT via POST /v1/auth/login.
//!
//! This provider is structurally complete. Implement the HTTP calls once
//! you have an Extraclub test account and can capture the app traffic.

use anyhow::{bail, Result};
use async_trait::async_trait;
use uuid::Uuid;

use super::{
    ReceiptQuery, ReceiptRetrievalProvider, RetrievalCapabilities, RetrievedReceipt,
};

const MERCHANT_ID: &str    = "circle-k";
const BASE_URL: &str       = "https://api.circlek.se";
const AUTH_PATH: &str      = "/v1/auth/login";
const PURCHASES_PATH: &str = "/v1/transactions";

#[allow(dead_code)]
pub struct CircleKProvider {
    email:    Option<String>,
    password: Option<String>,
    http:     reqwest::Client,
}

impl CircleKProvider {
    pub fn new(email: Option<String>, password: Option<String>) -> Self {
        Self {
            email,
            password,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ReceiptRetrievalProvider for CircleKProvider {
    fn merchant_id(&self) -> &str {
        MERCHANT_ID
    }

    fn capabilities(&self) -> RetrievalCapabilities {
        RetrievalCapabilities {
            supports_email_search:   true,   // Extraclub sends email receipts
            supports_app_api:        true,   // Extraclub app API
            supports_web_scrape:     false,
            supports_postal_request: false,
            typical_latency_seconds: 3,
        }
    }

    async fn search_receipts(&self, query: &ReceiptQuery) -> Result<Vec<RetrievedReceipt>> {
        if self.email.is_none() {
            tracing::info!(
                merchant_id  = MERCHANT_ID,
                auth_url     = %format!("{BASE_URL}{AUTH_PATH}"),
                purchase_url = %format!("{BASE_URL}{PURCHASES_PATH}"),
                amount       = %query.amount,
                date         = %query.date,
                "UNIMPLEMENTED: would authenticate Circle K Extraclub and search transactions; \
                 no credentials configured"
            );
            return Ok(vec![]);
        }

        // TODO:
        // 1. POST {BASE_URL}{AUTH_PATH} with { email, password } → { token }
        // 2. GET {BASE_URL}{PURCHASES_PATH}?dateFrom=…&dateTo=…  with Bearer token
        // 3. Filter by amount ± tolerance, map → RetrievedReceipt
        tracing::info!(
            merchant_id = MERCHANT_ID,
            "Circle K credentials present but fetch logic not yet implemented"
        );
        Ok(vec![])
    }

    async fn fetch_receipt(&self, receipt_ref: &str) -> Result<RetrievedReceipt> {
        bail!("CircleK fetch_receipt({receipt_ref}): not yet implemented");
    }

    async fn health_check(&self) -> Result<bool> {
        match self.http.get(format!("{BASE_URL}/v1/health")).send().await {
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

// ─── Placeholder response shapes ─────────────────────────────────────────────

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct CircleKTransaction {
    id:       String,
    amount:   f64,
    currency: String,
    date:     String,
    site:     Option<String>,
}

impl From<CircleKTransaction> for RetrievedReceipt {
    fn from(t: CircleKTransaction) -> Self {
        RetrievedReceipt {
            id:          Uuid::new_v4(),
            merchant_id: MERCHANT_ID.to_string(),
            amount:      rust_decimal::Decimal::try_from(t.amount).unwrap_or_default(),
            currency:    t.currency,
            date:        chrono::Utc::now(),
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
