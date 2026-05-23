// kivra.rs — Kivra Business Connector
// Fetches invoices from Kivra Business mailbox, downloads PDFs, marks processed.

use crate::canonical::{CanonicalError, Invoice, InvoiceStatus, Money};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ─── Constants ────────────────────────────────────────────────────────────────

const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 500;
const DEFAULT_BASE_URL: &str = "https://api.kivra.com/v1/business";

// ─── Public Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KivraInvoice {
    pub id: String,
    pub sender_name: String,
    pub sender_org_number: String,
    pub amount: Decimal,
    pub vat_amount: Decimal,
    pub currency: String,
    pub due_date: NaiveDate,
    pub invoice_number: String,
    pub pdf_available: bool,
    pub received_at: DateTime<Utc>,
    pub status: Option<String>,
    pub reference: Option<String>,
    pub ocr_number: Option<String>,
    pub bank_account: Option<KivraBankAccount>,
    pub sender_address: Option<KivraAddress>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KivraBankAccount {
    pub bank_giro: Option<String>,
    pub plus_giro: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KivraAddress {
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

// ─── Raw API Response Types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct KivraInvoiceListResponse {
    invoices: Vec<RawKivraInvoice>,
    total: u32,
    page: u32,
    per_page: u32,
}

#[derive(Debug, Deserialize)]
struct RawKivraInvoice {
    id: String,
    sender: RawKivraSender,
    amount: RawKivraAmount,
    vat_amount: Option<RawKivraAmount>,
    due_date: String,
    invoice_number: String,
    pdf_available: Option<bool>,
    received_at: String,
    status: Option<String>,
    reference: Option<String>,
    ocr_number: Option<String>,
    payment_details: Option<RawKivraPaymentDetails>,
}

#[derive(Debug, Deserialize)]
struct RawKivraSender {
    name: String,
    org_number: String,
    address: Option<RawKivraAddressRaw>,
}

#[derive(Debug, Deserialize)]
struct RawKivraAddressRaw {
    street: Option<String>,
    city: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawKivraAmount {
    value: String,      // Kivra sends amounts as strings for precision
    currency: String,
}

#[derive(Debug, Deserialize)]
struct RawKivraPaymentDetails {
    bank_giro: Option<String>,
    plus_giro: Option<String>,
    iban: Option<String>,
    bic: Option<String>,
}

#[derive(Debug, Serialize)]
struct MarkProcessedRequest {
    status: String,
    processed_at: String,
}

// ─── Connector ────────────────────────────────────────────────────────────────

pub struct KivraConnector {
    client: Client,
    api_key: String,
    base_url: String,
}

impl KivraConnector {
    /// Create with explicit base URL (useful for staging environments).
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60)) // PDFs can be large
            .user_agent("reconciler/1.0 (+https://wavult.com)")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    /// Create with default Kivra Business API base URL.
    pub fn new_default(api_key: impl Into<String>) -> Self {
        Self::new(api_key, DEFAULT_BASE_URL)
    }

    // ── Rate Limit & Retry ────────────────────────────────────────────────────

    async fn get_bytes_with_retry(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<bytes::Bytes, CanonicalError> {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt = 0u32;

        loop {
            let resp = self
                .client
                .get(&url)
                .header("X-API-Key", &self.api_key)
                .header("Accept", "*/*")
                .query(query)
                .send()
                .await
                .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

            let status = resp.status();

            if status == StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    return Err(CanonicalError::RateLimited(
                        "Kivra: exceeded max retries after 429".into(),
                    ));
                }
                let delay = resp
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or_else(|| {
                        (INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1)).min(60_000)
                    });
                warn!("Kivra 429 rate limit, backing off {}ms (attempt {})", delay, attempt);
                sleep(Duration::from_millis(delay)).await;
                continue;
            }

            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                return Err(CanonicalError::AuthError(format!(
                    "Kivra: auth failed on {path} — HTTP {status}"
                )));
            }

            if status.is_server_error() {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    let body = resp.text().await.unwrap_or_default();
                    return Err(CanonicalError::ApiError(format!(
                        "Kivra server error on {path}: {body}"
                    )));
                }
                let delay = (INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1)).min(30_000);
                warn!("Kivra 5xx on {path}, retry {}ms (attempt {})", delay, attempt);
                sleep(Duration::from_millis(delay)).await;
                continue;
            }

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(CanonicalError::ApiError(format!(
                    "Kivra GET {path} failed: HTTP {status} — {body}"
                )));
            }

            return resp
                .bytes()
                .await
                .map_err(|e| CanonicalError::NetworkError(e.to_string()));
        }
    }

    async fn get_json_with_retry<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, CanonicalError> {
        let raw = self.get_bytes_with_retry(path, query).await?;
        serde_json::from_slice(&raw)
            .map_err(|e| CanonicalError::ParseError(format!("Kivra JSON parse on {path}: {e}")))
    }

    async fn patch_with_retry<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), CanonicalError> {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt = 0u32;

        loop {
            let resp = self
                .client
                .patch(&url)
                .header("X-API-Key", &self.api_key)
                .json(body)
                .send()
                .await
                .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

            let status = resp.status();

            if status == StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    return Err(CanonicalError::RateLimited(
                        "Kivra PATCH: exceeded max retries".into(),
                    ));
                }
                let delay = (INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1)).min(30_000);
                sleep(Duration::from_millis(delay)).await;
                continue;
            }

            if status.is_server_error() {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    return Err(CanonicalError::ApiError(format!(
                        "Kivra PATCH {path} server error: HTTP {status}"
                    )));
                }
                let delay = (INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1)).min(30_000);
                sleep(Duration::from_millis(delay)).await;
                continue;
            }

            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                return Err(CanonicalError::ApiError(format!(
                    "Kivra PATCH {path} failed: HTTP {status} — {body_text}"
                )));
            }

            return Ok(());
        }
    }

    // ── Raw → Typed Mapping ───────────────────────────────────────────────────

    fn parse_raw_invoice(raw: RawKivraInvoice) -> Result<KivraInvoice, CanonicalError> {
        let amount = Decimal::from_str_radix(&raw.amount.value, 10)
            .map_err(|e| CanonicalError::ParseError(format!("Kivra amount parse: {e}")))?;

        let vat_amount = raw
            .vat_amount
            .as_ref()
            .map(|v| Decimal::from_str_radix(&v.value, 10))
            .transpose()
            .map_err(|e| CanonicalError::ParseError(format!("Kivra vat parse: {e}")))?
            .unwrap_or(Decimal::ZERO);

        let currency = raw.amount.currency.to_uppercase();

        let due_date = NaiveDate::parse_from_str(&raw.due_date, "%Y-%m-%d")
            .map_err(|e| CanonicalError::ParseError(format!("Kivra due_date parse: {e}")))?;

        let received_at = DateTime::parse_from_rfc3339(&raw.received_at)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| CanonicalError::ParseError(format!("Kivra received_at parse: {e}")))?;

        let bank_account = raw.payment_details.map(|pd| KivraBankAccount {
            bank_giro: pd.bank_giro,
            plus_giro: pd.plus_giro,
            iban: pd.iban,
            bic: pd.bic,
        });

        let sender_address = raw.sender.address.map(|a| KivraAddress {
            street: a.street,
            city: a.city,
            postal_code: a.postal_code,
            country: a.country,
        });

        Ok(KivraInvoice {
            id: raw.id,
            sender_name: raw.sender.name,
            sender_org_number: raw.sender.org_number,
            amount,
            vat_amount,
            currency,
            due_date,
            invoice_number: raw.invoice_number,
            pdf_available: raw.pdf_available.unwrap_or(false),
            received_at,
            status: raw.status,
            reference: raw.reference,
            ocr_number: raw.ocr_number,
            bank_account,
            sender_address,
        })
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Fetch all invoices in the Kivra Business mailbox (paginated).
    pub async fn fetch_invoices(&self) -> Result<Vec<KivraInvoice>, CanonicalError> {
        let mut all: Vec<KivraInvoice> = Vec::new();
        let mut page = 1u32;
        let per_page = 50u32;

        loop {
            let page_str = page.to_string();
            let per_page_str = per_page.to_string();

            let resp: KivraInvoiceListResponse = self
                .get_json_with_retry(
                    "/invoices",
                    &[("page", &page_str), ("per_page", &per_page_str)],
                )
                .await?;

            let fetched = resp.invoices.len();
            debug!(
                "Kivra fetch_invoices: page={page}, fetched={fetched}, total={}",
                resp.total
            );

            for raw in resp.invoices {
                match Self::parse_raw_invoice(raw) {
                    Ok(inv) => all.push(inv),
                    Err(e) => warn!("Skipping Kivra invoice due to parse error: {e}"),
                }
            }

            let fetched_so_far = (page * per_page) as usize;
            if fetched < per_page as usize || fetched_so_far >= resp.total as usize {
                break;
            }
            page += 1;
        }

        info!("Kivra fetch_invoices: {} invoices total", all.len());
        Ok(all)
    }

    /// Download the PDF for a specific invoice, returned as raw bytes.
    pub async fn download_pdf(&self, invoice_id: &str) -> Result<Vec<u8>, CanonicalError> {
        let path = format!("/invoices/{invoice_id}/pdf");
        let bytes = self.get_bytes_with_retry(&path, &[]).await?;

        if bytes.len() < 4 || &bytes[..4] != b"%PDF" {
            return Err(CanonicalError::ParseError(format!(
                "Kivra: response for invoice {invoice_id} is not a valid PDF"
            )));
        }

        info!(
            "Kivra download_pdf: invoice_id={invoice_id} size={}b",
            bytes.len()
        );
        Ok(bytes.to_vec())
    }

    /// Mark an invoice as processed in Kivra.
    pub async fn mark_processed(&self, invoice_id: &str) -> Result<(), CanonicalError> {
        let path = format!("/invoices/{invoice_id}");
        let body = MarkProcessedRequest {
            status: "processed".to_string(),
            processed_at: Utc::now().to_rfc3339(),
        };
        self.patch_with_retry(&path, &body).await?;
        info!("Kivra mark_processed: invoice_id={invoice_id}");
        Ok(())
    }

    /// Convert a KivraInvoice to the canonical Invoice model.
    pub fn to_canonical(kivra: &KivraInvoice) -> Invoice {
        let status = match kivra.status.as_deref() {
            Some("processed") | Some("paid") => InvoiceStatus::Paid,
            Some("pending") | Some("unread") => InvoiceStatus::Unpaid,
            Some("overdue") => InvoiceStatus::Overdue,
            Some("cancelled") => InvoiceStatus::Cancelled,
            Some(other) => InvoiceStatus::Unknown(other.to_string()),
            None => InvoiceStatus::Unpaid,
        };

        Invoice {
            id: Uuid::new_v4().to_string(),
            external_id: kivra.id.clone(),
            source: "kivra".into(),
            invoice_number: Some(kivra.invoice_number.clone()),
            counterparty_name: kivra.sender_name.clone(),
            counterparty_id: Some(kivra.sender_org_number.clone()),
            is_outgoing: true, // Kivra Business: these are incoming supplier invoices to pay
            amount: Money {
                amount: kivra.amount,
                currency: kivra.currency.clone(),
            },
            vat_amount: Some(Money {
                amount: kivra.vat_amount,
                currency: kivra.currency.clone(),
            }),
            status,
            invoice_date: kivra.received_at.format("%Y-%m-%d").to_string(),
            due_date: Some(kivra.due_date.format("%Y-%m-%d").to_string()),
            line_items: vec![], // Kivra invoices typically don't expose line items via API
            raw: serde_json::to_value(kivra).unwrap_or_default(),
        }
    }
}

// ─── Builder Pattern ──────────────────────────────────────────────────────────

pub struct KivraConnectorBuilder {
    api_key: String,
    base_url: String,
}

impl KivraConnectorBuilder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn build(self) -> KivraConnector {
        KivraConnector::new(self.api_key, self.base_url)
    }
}

// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_kivra_invoice() -> KivraInvoice {
        KivraInvoice {
            id: "kiv-001".into(),
            sender_name: "Telia Sverige AB".into(),
            sender_org_number: "556430-0142".into(),
            amount: dec!(1250.00),
            vat_amount: dec!(250.00),
            currency: "SEK".into(),
            due_date: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            invoice_number: "INV-20260601-001".into(),
            pdf_available: true,
            received_at: Utc::now(),
            status: Some("pending".into()),
            reference: Some("REF12345".into()),
            ocr_number: Some("12345678".into()),
            bank_account: Some(KivraBankAccount {
                bank_giro: Some("123-4567".into()),
                plus_giro: None,
                iban: None,
                bic: None,
            }),
            sender_address: None,
        }
    }

    #[test]
    fn test_to_canonical_status_mapping() {
        let mut inv = sample_kivra_invoice();
        inv.status = Some("pending".into());
        let canon = KivraConnector::to_canonical(&inv);
        assert!(matches!(canon.status, InvoiceStatus::Unpaid));
    }

    #[test]
    fn test_to_canonical_paid_mapping() {
        let mut inv = sample_kivra_invoice();
        inv.status = Some("paid".into());
        let canon = KivraConnector::to_canonical(&inv);
        assert!(matches!(canon.status, InvoiceStatus::Paid));
    }

    #[test]
    fn test_to_canonical_amount() {
        let inv = sample_kivra_invoice();
        let canon = KivraConnector::to_canonical(&inv);
        assert_eq!(canon.amount.amount, dec!(1250.00));
        assert_eq!(canon.amount.currency, "SEK");
        assert_eq!(
            canon.vat_amount.as_ref().unwrap().amount,
            dec!(250.00)
        );
    }

    #[test]
    fn test_to_canonical_source() {
        let inv = sample_kivra_invoice();
        let canon = KivraConnector::to_canonical(&inv);
        assert_eq!(canon.source, "kivra");
        assert_eq!(canon.external_id, "kiv-001");
    }
}
