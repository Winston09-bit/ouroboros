use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

pub mod ica;
pub mod okq8;
pub mod circle_k;
pub mod clas_ohlson;
pub mod scandic;

// ─── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait ReceiptRetrievalProvider: Send + Sync {
    /// Stable merchant identifier, e.g. "ica-handla", "okq8", …
    fn merchant_id(&self) -> &str;
    fn capabilities(&self) -> RetrievalCapabilities;
    async fn search_receipts(&self, query: &ReceiptQuery) -> Result<Vec<RetrievedReceipt>>;
    async fn fetch_receipt(&self, receipt_ref: &str) -> Result<RetrievedReceipt>;
    async fn health_check(&self) -> Result<bool>;
}

// ─── Capabilities ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RetrievalCapabilities {
    pub supports_email_search:   bool,
    pub supports_app_api:        bool,
    pub supports_web_scrape:     bool,
    pub supports_postal_request: bool,
    pub typical_latency_seconds: u32,
}

// ─── Query ────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ReceiptQuery {
    pub amount:              Decimal,
    pub date:                DateTime<Utc>,
    pub date_tolerance_days: i32,
    pub card_last4:          Option<String>,
    pub user_email:          Option<String>,
    pub user_phone:          Option<String>,
}

// ─── Result types ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RetrievedReceipt {
    pub id:          Uuid,
    pub merchant_id: String,
    pub amount:      Decimal,
    pub currency:    String,
    pub date:        DateTime<Utc>,
    pub items:       Vec<ReceiptLineItem>,
    pub vat_amount:  Option<Decimal>,
    pub vat_rate:    Option<Decimal>,
    pub raw_pdf:     Option<Vec<u8>>,
    pub raw_html:    Option<String>,
    /// Source identifier, e.g. "ica-handla", "okq8-kort-app"
    pub source:      String,
    /// 0.0 – 1.0 match confidence
    pub confidence:  f64,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ReceiptLineItem {
    pub description: String,
    pub quantity:    Decimal,
    pub unit_price:  Decimal,
    pub vat_rate:    Option<Decimal>,
}

// ─── Registry ─────────────────────────────────────────────────────────────────

pub struct RetrievalRegistry {
    providers: Vec<Box<dyn ReceiptRetrievalProvider>>,
}

impl RetrievalRegistry {
    pub fn new() -> Self {
        Self { providers: Vec::new() }
    }

    pub fn register(&mut self, p: Box<dyn ReceiptRetrievalProvider>) {
        tracing::info!(
            merchant_id = p.merchant_id(),
            "RetrievalRegistry: registered provider"
        );
        self.providers.push(p);
    }

    /// Fan-out to all registered providers; returns all hits, sorted by confidence desc.
    pub async fn search_all(&self, query: &ReceiptQuery) -> Vec<RetrievedReceipt> {
        let mut results = Vec::new();
        for provider in &self.providers {
            match provider.search_receipts(query).await {
                Ok(mut hits) => results.append(&mut hits),
                Err(e) => tracing::warn!(
                    merchant_id = provider.merchant_id(),
                    error = %e,
                    "RetrievalRegistry: provider search failed"
                ),
            }
        }
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Fan-out to a specific merchant provider only.
    pub async fn search_merchant(
        &self,
        merchant_id: &str,
        query: &ReceiptQuery,
    ) -> Vec<RetrievedReceipt> {
        let mut results = Vec::new();
        for provider in self.providers.iter().filter(|p| p.merchant_id() == merchant_id) {
            match provider.search_receipts(query).await {
                Ok(mut hits) => results.append(&mut hits),
                Err(e) => tracing::warn!(
                    merchant_id,
                    error = %e,
                    "RetrievalRegistry: merchant search failed"
                ),
            }
        }
        results
    }
}

impl Default for RetrievalRegistry {
    fn default() -> Self {
        Self::new()
    }
}
