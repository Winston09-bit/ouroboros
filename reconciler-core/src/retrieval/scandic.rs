//! Scandic Friends booking receipt retrieval provider.
//!
//! Scandic Hotels exposes booking history and folios (VAT receipts) via their
//! customer portal and a semi-public API used by the Scandic app.
//!
//! Observed API pattern:
//!   POST https://api.scandichotels.com/auth/v1/token  → { access_token }
//!   GET  https://api.scandichotels.com/stays/v1/bookings
//!   GET  https://api.scandichotels.com/stays/v1/bookings/{id}/folio  → PDF URL
//!
//! Auth: Scandic Friends email + password → OAuth2 Bearer JWT.
//!
//! This provider is structurally complete. Implement HTTP calls once you
//! have a test Scandic Friends account and can validate the endpoint shapes.

use anyhow::{bail, Result};
use async_trait::async_trait;
use uuid::Uuid;

use super::{
    ReceiptQuery, ReceiptRetrievalProvider, RetrievalCapabilities, RetrievedReceipt,
};

const MERCHANT_ID: &str      = "scandic";
const AUTH_URL: &str         = "https://api.scandichotels.com/auth/v1/token";
const BOOKINGS_URL: &str     = "https://api.scandichotels.com/stays/v1/bookings";

#[allow(dead_code)]
pub struct ScandicProvider {
    email:    Option<String>,
    password: Option<String>,
    http:     reqwest::Client,
}

impl ScandicProvider {
    pub fn new(email: Option<String>, password: Option<String>) -> Self {
        Self {
            email,
            password,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ReceiptRetrievalProvider for ScandicProvider {
    fn merchant_id(&self) -> &str {
        MERCHANT_ID
    }

    fn capabilities(&self) -> RetrievalCapabilities {
        RetrievalCapabilities {
            supports_email_search:   true,   // Scandic sends folio PDFs by email
            supports_app_api:        true,   // Scandic app API
            supports_web_scrape:     false,
            supports_postal_request: false,
            typical_latency_seconds: 5,
        }
    }

    async fn search_receipts(&self, query: &ReceiptQuery) -> Result<Vec<RetrievedReceipt>> {
        if self.email.is_none() {
            tracing::info!(
                merchant_id   = MERCHANT_ID,
                auth_url      = AUTH_URL,
                bookings_url  = BOOKINGS_URL,
                amount        = %query.amount,
                date          = %query.date,
                "UNIMPLEMENTED: would authenticate Scandic Friends and search bookings/folios; \
                 no credentials configured"
            );
            return Ok(vec![]);
        }

        // TODO:
        // 1. POST AUTH_URL with { email, password, grant_type: "password" } → { access_token }
        // 2. GET BOOKINGS_URL?checkIn[gte]=…&checkIn[lte]=…  with Bearer token
        // 3. For matching bookings, GET BOOKINGS_URL/{id}/folio  → PDF download URL
        // 4. Download PDF, store as raw_pdf in RetrievedReceipt
        tracing::info!(
            merchant_id = MERCHANT_ID,
            "Scandic credentials present but fetch logic not yet implemented"
        );
        Ok(vec![])
    }

    async fn fetch_receipt(&self, receipt_ref: &str) -> Result<RetrievedReceipt> {
        bail!("Scandic fetch_receipt({receipt_ref}): not yet implemented");
    }

    async fn health_check(&self) -> Result<bool> {
        match self.http.get("https://www.scandichotels.se/").send().await {
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
struct ScandicTokenResponse {
    access_token: String,
    expires_in:   u64,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct ScandicBooking {
    id:             String,
    total_amount:   f64,
    currency:       String,
    check_in_date:  String,
    check_out_date: String,
    hotel_name:     Option<String>,
    folio_url:      Option<String>,
}

impl From<ScandicBooking> for RetrievedReceipt {
    fn from(b: ScandicBooking) -> Self {
        RetrievedReceipt {
            id:          Uuid::new_v4(),
            merchant_id: MERCHANT_ID.to_string(),
            amount:      rust_decimal::Decimal::try_from(b.total_amount).unwrap_or_default(),
            currency:    b.currency,
            date:        chrono::Utc::now(), // parse b.check_in_date when implementing
            items:       vec![],
            vat_amount:  None,
            vat_rate:    None,
            raw_pdf:     None, // populated after downloading folio_url
            raw_html:    None,
            source:      MERCHANT_ID.to_string(),
            confidence:  0.90,
        }
    }
}
