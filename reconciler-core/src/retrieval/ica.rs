//! ICA Handla receipt retrieval provider.
//!
//! ICA's customer API is **not publicly documented**. This provider is
//! structurally complete and ready to be wired up once credentials/tokens
//! are available through an official partnership or reverse-engineered flow.
//!
//! Known endpoint candidates (unofficial, subject to change):
//!   - `https://apigw.ica.se/api/handla/v2/receipts`        (app traffic)
//!   - `https://apigw.ica.se/api/handla/v2/receipts/{id}`
//!
//! Auth: Bearer token obtained via ICA Handla OAuth2 (customer app login).

use anyhow::{bail, Result};
use async_trait::async_trait;
use uuid::Uuid;

use super::{
    ReceiptLineItem, ReceiptQuery, ReceiptRetrievalProvider, RetrievalCapabilities,
    RetrievedReceipt,
};

const MERCHANT_ID: &str = "ica-handla";
const RECEIPTS_ENDPOINT: &str = "https://apigw.ica.se/api/handla/v2/receipts";

#[allow(dead_code)]
pub struct IcaProvider {
    /// OAuth2 Bearer token for the customer's ICA account.
    /// Obtain via ICA Handla mobile-app login flow (PKCE).
    bearer_token: Option<String>,
    http: reqwest::Client,
}

impl IcaProvider {
    pub fn new(bearer_token: Option<String>) -> Self {
        Self {
            bearer_token,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ReceiptRetrievalProvider for IcaProvider {
    fn merchant_id(&self) -> &str {
        MERCHANT_ID
    }

    fn capabilities(&self) -> RetrievalCapabilities {
        RetrievalCapabilities {
            supports_email_search:   false,
            supports_app_api:        true,   // via ICA Handla app API
            supports_web_scrape:     false,
            supports_postal_request: false,
            typical_latency_seconds: 2,
        }
    }

    async fn search_receipts(&self, query: &ReceiptQuery) -> Result<Vec<RetrievedReceipt>> {
        if self.bearer_token.is_none() {
            tracing::info!(
                merchant_id = MERCHANT_ID,
                endpoint    = RECEIPTS_ENDPOINT,
                amount      = %query.amount,
                date        = %query.date,
                "UNIMPLEMENTED: would POST to ICA Handla receipt search with amount+date filter; \
                 no bearer token configured"
            );
            return Ok(vec![]);
        }

        // TODO: implement when bearer_token is available
        // let resp = self.http
        //     .get(RECEIPTS_ENDPOINT)
        //     .bearer_auth(token)
        //     .query(&[("from", from_date), ("to", to_date), ("amount", amount_str)])
        //     .send().await?
        //     .error_for_status()?
        //     .json::<IcaReceiptsResponse>().await?;
        tracing::info!(
            merchant_id = MERCHANT_ID,
            "ICA bearer token present but fetch logic not yet implemented"
        );
        Ok(vec![])
    }

    async fn fetch_receipt(&self, receipt_ref: &str) -> Result<RetrievedReceipt> {
        let Some(_token) = &self.bearer_token else {
            bail!("ICA provider: no bearer token configured");
        };
        // TODO: GET {RECEIPTS_ENDPOINT}/{receipt_ref}
        tracing::info!(
            merchant_id = MERCHANT_ID,
            receipt_ref,
            "UNIMPLEMENTED: would GET individual ICA receipt"
        );
        // Return a placeholder so callers can pattern-match on confidence=0.0
        Ok(RetrievedReceipt {
            id:          Uuid::new_v4(),
            merchant_id: MERCHANT_ID.to_string(),
            amount:      rust_decimal::Decimal::ZERO,
            currency:    "SEK".to_string(),
            date:        chrono::Utc::now(),
            items:       vec![ReceiptLineItem {
                description: "UNIMPLEMENTED".to_string(),
                quantity:    rust_decimal::Decimal::ONE,
                unit_price:  rust_decimal::Decimal::ZERO,
                vat_rate:    None,
            }],
            vat_amount:  None,
            vat_rate:    None,
            raw_pdf:     None,
            raw_html:    None,
            source:      MERCHANT_ID.to_string(),
            confidence:  0.0,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // Ping the gateway root; a 200/401 both mean it's reachable.
        match self.http.get("https://apigw.ica.se/").send().await {
            Ok(r) => {
                let up = r.status().is_success() || r.status() == 401;
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
