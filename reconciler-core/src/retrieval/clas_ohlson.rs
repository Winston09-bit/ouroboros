//! Clas Ohlson Club receipt retrieval provider.
//!
//! Clas Ohlson's loyalty programme (Club Clas) stores digital receipts
//! accessible at `https://www.clasohlson.com/se/my-pages/receipts` and via
//! a semi-public REST API used by their website.
//!
//! Observed API pattern (browser traffic):
//!   POST https://www.clasohlson.com/api/loyalty/login   → set-cookie: session
//!   GET  https://www.clasohlson.com/api/loyalty/receipts?page=1
//!   GET  https://www.clasohlson.com/api/loyalty/receipts/{id}
//!
//! Auth: email + password → session cookie.
//!
//! This provider is structurally complete. Wire up the HTTP calls once you
//! have a test Club Clas account.

use anyhow::{bail, Result};
use async_trait::async_trait;
use uuid::Uuid;

use super::{
    ReceiptLineItem, ReceiptQuery, ReceiptRetrievalProvider, RetrievalCapabilities,
    RetrievedReceipt,
};

const MERCHANT_ID: &str      = "clas-ohlson";
const BASE_URL: &str         = "https://www.clasohlson.com";
const LOGIN_PATH: &str       = "/api/loyalty/login";
const RECEIPTS_PATH: &str    = "/api/loyalty/receipts";

#[allow(dead_code)]
pub struct ClasOhlsonProvider {
    email:    Option<String>,
    password: Option<String>,
    /// Cookie jar persisted across requests.
    http:     reqwest::Client,
}

impl ClasOhlsonProvider {
    pub fn new(email: Option<String>, password: Option<String>) -> Self {
        // NOTE: full cookie-jar support requires the `cookies` feature on reqwest.
        // For now we use the default client; add `.cookie_store(true)` once
        // that feature is enabled in Cargo.toml.
        let http = reqwest::Client::new();
        Self { email, password, http }
    }
}

#[async_trait]
impl ReceiptRetrievalProvider for ClasOhlsonProvider {
    fn merchant_id(&self) -> &str {
        MERCHANT_ID
    }

    fn capabilities(&self) -> RetrievalCapabilities {
        RetrievalCapabilities {
            supports_email_search:   true,   // Club Clas sends email receipts
            supports_app_api:        false,  // App uses same web API, no separate SDK
            supports_web_scrape:     true,   // Cookie-session approach
            supports_postal_request: false,
            typical_latency_seconds: 4,
        }
    }

    async fn search_receipts(&self, query: &ReceiptQuery) -> Result<Vec<RetrievedReceipt>> {
        if self.email.is_none() {
            tracing::info!(
                merchant_id  = MERCHANT_ID,
                login_url    = %format!("{BASE_URL}{LOGIN_PATH}"),
                receipts_url = %format!("{BASE_URL}{RECEIPTS_PATH}"),
                amount       = %query.amount,
                date         = %query.date,
                "UNIMPLEMENTED: would log in to Club Clas and search digital receipts; \
                 no credentials configured"
            );
            return Ok(vec![]);
        }

        // TODO:
        // 1. POST {BASE_URL}{LOGIN_PATH} with JSON { email, password }
        //    → response sets a session cookie; client jar retains it
        // 2. GET {BASE_URL}{RECEIPTS_PATH}?page=1  (paginate as needed)
        // 3. Filter by date ± tolerance and amount, map → RetrievedReceipt
        // 4. Optionally GET individual receipt for line-items
        tracing::info!(
            merchant_id = MERCHANT_ID,
            "Clas Ohlson credentials present but fetch logic not yet implemented"
        );
        Ok(vec![])
    }

    async fn fetch_receipt(&self, receipt_ref: &str) -> Result<RetrievedReceipt> {
        bail!("ClasOhlson fetch_receipt({receipt_ref}): not yet implemented");
    }

    async fn health_check(&self) -> Result<bool> {
        match self.http.get(BASE_URL).send().await {
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
struct ClasOhlsonReceipt {
    id:       String,
    total:    f64,
    currency: String,
    date:     String,
    items:    Vec<ClasOhlsonLineItem>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct ClasOhlsonLineItem {
    name:       String,
    qty:        f64,
    unit_price: f64,
    vat_rate:   Option<f64>,
}

impl From<ClasOhlsonReceipt> for RetrievedReceipt {
    fn from(r: ClasOhlsonReceipt) -> Self {
        let items = r.items.into_iter().map(|i| ReceiptLineItem {
            description: i.name,
            quantity:    rust_decimal::Decimal::try_from(i.qty).unwrap_or_default(),
            unit_price:  rust_decimal::Decimal::try_from(i.unit_price).unwrap_or_default(),
            vat_rate:    i.vat_rate.and_then(|v| rust_decimal::Decimal::try_from(v).ok()),
        }).collect();

        RetrievedReceipt {
            id:          Uuid::new_v4(),
            merchant_id: MERCHANT_ID.to_string(),
            amount:      rust_decimal::Decimal::try_from(r.total).unwrap_or_default(),
            currency:    r.currency,
            date:        chrono::Utc::now(),
            items,
            vat_amount:  None,
            vat_rate:    None,
            raw_pdf:     None,
            raw_html:    None,
            source:      MERCHANT_ID.to_string(),
            confidence:  0.90,
        }
    }
}
