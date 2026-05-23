// visma.rs — Visma eEkonomi Connector
// Implements AccountingProvider trait with full OAuth2, rate limiting, and canonical mapping.

use crate::canonical::{
    Account, CanonicalError, ChartOfAccounts, Invoice, InvoiceStatus, LineItem, Money, Transaction,
    TransactionDirection, Vendor,
};
use crate::traits::AccountingProvider;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ─── Configuration ────────────────────────────────────────────────────────────

const VISMA_BASE_URL: &str = "https://eaccountingapi.visma.net/v2";
const VISMA_CONNECT_URL: &str = "https://connect.visma.com/connect/token";
const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 500;

#[derive(Debug, Clone)]
pub struct VismaConfig {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
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

// ─── Raw API Types — Visma eEkonomi ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct VismaPagedResponse<T> {
    #[serde(rename = "Data")]
    data: Vec<T>,
    #[serde(rename = "Meta")]
    meta: Option<VismaMeta>,
}

#[derive(Debug, Deserialize)]
struct VismaMeta {
    #[serde(rename = "TotalNumberOfResults")]
    total: u64,
    #[serde(rename = "CurrentPage")]
    current_page: u32,
}

#[derive(Debug, Deserialize)]
struct VismaSupplierInvoice {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "InvoiceNumber")]
    invoice_number: Option<String>,
    #[serde(rename = "SupplierId")]
    supplier_id: String,
    #[serde(rename = "SupplierName")]
    supplier_name: Option<String>,
    #[serde(rename = "InvoiceDate")]
    invoice_date: String,
    #[serde(rename = "DueDate")]
    due_date: Option<String>,
    #[serde(rename = "TotalAmount")]
    total_amount: f64,
    #[serde(rename = "VatAmount")]
    vat_amount: Option<f64>,
    #[serde(rename = "CurrencyCode")]
    currency_code: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "Rows")]
    rows: Option<Vec<VismaInvoiceRow>>,
}

#[derive(Debug, Deserialize)]
struct VismaCustomerInvoice {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "InvoiceNumber")]
    invoice_number: Option<String>,
    #[serde(rename = "CustomerId")]
    customer_id: String,
    #[serde(rename = "CustomerName")]
    customer_name: Option<String>,
    #[serde(rename = "InvoiceDate")]
    invoice_date: String,
    #[serde(rename = "DueDate")]
    due_date: Option<String>,
    #[serde(rename = "TotalAmount")]
    total_amount: f64,
    #[serde(rename = "VatAmount")]
    vat_amount: Option<f64>,
    #[serde(rename = "CurrencyCode")]
    currency_code: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "Rows")]
    rows: Option<Vec<VismaInvoiceRow>>,
}

#[derive(Debug, Deserialize)]
struct VismaInvoiceRow {
    #[serde(rename = "ArticleNumber")]
    article_number: Option<String>,
    #[serde(rename = "ArticleDescription")]
    article_description: Option<String>,
    #[serde(rename = "Quantity")]
    quantity: Option<f64>,
    #[serde(rename = "Price")]
    price: Option<f64>,
    #[serde(rename = "TotalAmount")]
    total_amount: f64,
    #[serde(rename = "AccountNumber")]
    account_number: Option<String>,
    #[serde(rename = "VatPercent")]
    vat_percent: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct VismaAccount {
    #[serde(rename = "Number")]
    number: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AccountType")]
    account_type: Option<String>,
    #[serde(rename = "IsActive")]
    is_active: bool,
    #[serde(rename = "VatCodeId")]
    vat_code_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VismaSupplier {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "CorporateIdentityNumber")]
    org_number: Option<String>,
    #[serde(rename = "Email")]
    email: Option<String>,
    #[serde(rename = "Phone")]
    phone: Option<String>,
    #[serde(rename = "IsActive")]
    is_active: bool,
    #[serde(rename = "BankAccount")]
    bank_account: Option<VismaBankAccount>,
    #[serde(rename = "CurrencyCode")]
    currency_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VismaBankAccount {
    #[serde(rename = "BankAccountNumber")]
    account_number: Option<String>,
    #[serde(rename = "Iban")]
    iban: Option<String>,
    #[serde(rename = "Bic")]
    bic: Option<String>,
}

#[derive(Debug, Serialize)]
struct VismaVoucherRequest {
    #[serde(rename = "VoucherDate")]
    voucher_date: String,
    #[serde(rename = "VoucherText")]
    voucher_text: String,
    #[serde(rename = "Rows")]
    rows: Vec<VismaVoucherRow>,
}

#[derive(Debug, Serialize)]
struct VismaVoucherRow {
    #[serde(rename = "AccountNumber")]
    account_number: String,
    #[serde(rename = "DebitAmount")]
    debit_amount: Option<f64>,
    #[serde(rename = "CreditAmount")]
    credit_amount: Option<f64>,
    #[serde(rename = "TransactionText")]
    transaction_text: Option<String>,
    #[serde(rename = "CostCenterItemId1")]
    cost_center_1: Option<String>,
}

// ─── Connector ────────────────────────────────────────────────────────────────

pub struct VismaConnector {
    http: Client,
    config: VismaConfig,
    token_cache: tokio::sync::Mutex<Option<TokenCache>>,
}

impl VismaConnector {
    pub fn new(config: VismaConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("reconciler/1.0 (+https://wavult.com)")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            config,
            token_cache: tokio::sync::Mutex::new(None),
        }
    }

    // ── OAuth2 Token Management ───────────────────────────────────────────────

    async fn ensure_token(&self) -> Result<String, CanonicalError> {
        let mut cache = self.token_cache.lock().await;

        if let Some(ref cached) = *cache {
            if !cached.is_expired() {
                return Ok(cached.access_token.clone());
            }
            debug!("Visma token expired, refreshing...");
        }

        let token = self.fetch_token().await?;
        let access_token = token.access_token.clone();
        *cache = Some(TokenCache {
            access_token: token.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(token.expires_in),
        });

        info!("Visma OAuth2 token refreshed, expires in {}s", token.expires_in);
        Ok(access_token)
    }

    async fn fetch_token(&self) -> Result<TokenResponse, CanonicalError> {
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("scope", "ea:api ea:accounting"),
        ];

        let resp = self
            .http
            .post(VISMA_CONNECT_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(CanonicalError::AuthError(format!(
                "Visma token fetch failed: HTTP {status} — {body}"
            )));
        }

        resp.json::<TokenResponse>()
            .await
            .map_err(|e| CanonicalError::ParseError(e.to_string()))
    }

    // ── HTTP Helpers with Rate Limit + Backoff ────────────────────────────────

    async fn get_paged<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<T>, CanonicalError> {
        let mut all_results: Vec<T> = Vec::new();
        let mut page = 1u32;

        loop {
            let mut q = query.to_vec();
            let page_str = page.to_string();
            q.push(("$page", &page_str));
            q.push(("$pagesize", "100"));

            let resp: VismaPagedResponse<T> = self
                .get_with_retry(endpoint, &q)
                .await?;

            let total = resp.meta.as_ref().map(|m| m.total).unwrap_or(0);
            let fetched = resp.data.len();
            all_results.extend(resp.data);

            debug!(
                "Visma paged fetch {endpoint}: page={page}, got={fetched}, total={total}"
            );

            // Determine if there's a next page
            let per_page = 100u64;
            let fetched_so_far = (page as u64) * per_page;
            if fetched_so_far >= total || fetched == 0 {
                break;
            }
            page += 1;
        }

        Ok(all_results)
    }

    async fn get_with_retry<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
    ) -> Result<T, CanonicalError> {
        let url = format!("{VISMA_BASE_URL}{endpoint}");
        let mut attempt = 0u32;

        loop {
            let token = self.ensure_token().await?;

            let resp = self
                .http
                .get(&url)
                .bearer_auth(&token)
                .header("VismaNetApi-TenantId", &self.config.tenant_id)
                .query(query)
                .send()
                .await
                .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

            let status = resp.status();

            if status == StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    return Err(CanonicalError::RateLimited(
                        "Visma: exceeded max retries after 429".into(),
                    ));
                }
                let retry_after = resp
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or_else(|| {
                        let backoff =
                            INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                        backoff.min(30_000)
                    });

                warn!(
                    "Visma 429 rate limit hit, backing off {}ms (attempt {}/{})",
                    retry_after, attempt, MAX_RETRIES
                );
                sleep(Duration::from_millis(retry_after)).await;
                continue;
            }

            if status == StatusCode::UNAUTHORIZED {
                // Force token refresh on next attempt
                *self.token_cache.lock().await = None;
                attempt += 1;
                if attempt > 2 {
                    return Err(CanonicalError::AuthError(
                        "Visma: 401 Unauthorized after token refresh".into(),
                    ));
                }
                continue;
            }

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(CanonicalError::ApiError(format!(
                    "Visma GET {endpoint} failed: HTTP {status} — {body}"
                )));
            }

            return resp
                .json::<T>()
                .await
                .map_err(|e| CanonicalError::ParseError(e.to_string()));
        }
    }

    async fn post_with_retry<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> Result<T, CanonicalError> {
        let url = format!("{VISMA_BASE_URL}{endpoint}");
        let mut attempt = 0u32;

        loop {
            let token = self.ensure_token().await?;

            let resp = self
                .http
                .post(&url)
                .bearer_auth(&token)
                .header("VismaNetApi-TenantId", &self.config.tenant_id)
                .json(body)
                .send()
                .await
                .map_err(|e| CanonicalError::NetworkError(e.to_string()))?;

            let status = resp.status();

            if status == StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt > MAX_RETRIES {
                    return Err(CanonicalError::RateLimited(
                        "Visma: exceeded max retries on POST".into(),
                    ));
                }
                let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                let delay = backoff.min(30_000);
                warn!("Visma POST 429, backing off {}ms", delay);
                sleep(Duration::from_millis(delay)).await;
                continue;
            }

            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                return Err(CanonicalError::ApiError(format!(
                    "Visma POST {endpoint} failed: HTTP {status} — {body_text}"
                )));
            }

            return resp
                .json::<T>()
                .await
                .map_err(|e| CanonicalError::ParseError(e.to_string()));
        }
    }

    // ── Mapping Helpers ───────────────────────────────────────────────────────

    fn supplier_invoice_to_transaction(inv: &VismaSupplierInvoice) -> Transaction {
        Transaction {
            id: Uuid::new_v4().to_string(),
            external_id: inv.id.clone(),
            source: "visma".into(),
            direction: TransactionDirection::Outgoing,
            amount: Money {
                amount: Decimal::from_str(&format!("{:.6}", inv.total_amount))
                    .unwrap_or_default(),
                currency: inv.currency_code.clone(),
            },
            counterparty_id: Some(inv.supplier_id.clone()),
            counterparty_name: inv.supplier_name.clone(),
            description: inv
                .invoice_number
                .as_ref()
                .map(|n| format!("Supplier Invoice #{n}")),
            posted_at: DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", inv.invoice_date))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            raw: serde_json::to_value(inv).unwrap_or_default(),
        }
    }

    fn customer_invoice_to_transaction(inv: &VismaCustomerInvoice) -> Transaction {
        Transaction {
            id: Uuid::new_v4().to_string(),
            external_id: inv.id.clone(),
            source: "visma".into(),
            direction: TransactionDirection::Incoming,
            amount: Money {
                amount: Decimal::from_str(&format!("{:.6}", inv.total_amount))
                    .unwrap_or_default(),
                currency: inv.currency_code.clone(),
            },
            counterparty_id: Some(inv.customer_id.clone()),
            counterparty_name: inv.customer_name.clone(),
            description: inv
                .invoice_number
                .as_ref()
                .map(|n| format!("Customer Invoice #{n}")),
            posted_at: DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", inv.invoice_date))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            raw: serde_json::to_value(inv).unwrap_or_default(),
        }
    }

    fn parse_invoice_status(status: &str) -> InvoiceStatus {
        match status {
            "Unpaid" => InvoiceStatus::Unpaid,
            "Paid" => InvoiceStatus::Paid,
            "PartiallyPaid" => InvoiceStatus::PartiallyPaid,
            "Overdue" => InvoiceStatus::Overdue,
            "Cancelled" | "Voided" => InvoiceStatus::Cancelled,
            _ => InvoiceStatus::Unknown(status.to_string()),
        }
    }

    fn supplier_invoice_to_canonical(inv: &VismaSupplierInvoice) -> Invoice {
        Invoice {
            id: Uuid::new_v4().to_string(),
            external_id: inv.id.clone(),
            source: "visma".into(),
            invoice_number: inv.invoice_number.clone(),
            counterparty_name: inv.supplier_name.clone().unwrap_or_default(),
            counterparty_id: Some(inv.supplier_id.clone()),
            is_outgoing: true,
            amount: Money {
                amount: Decimal::from_str(&format!("{:.6}", inv.total_amount))
                    .unwrap_or_default(),
                currency: inv.currency_code.clone(),
            },
            vat_amount: inv.vat_amount.map(|v| Money {
                amount: Decimal::from_str(&format!("{:.6}", v)).unwrap_or_default(),
                currency: inv.currency_code.clone(),
            }),
            status: Self::parse_invoice_status(&inv.status),
            invoice_date: inv.invoice_date.clone(),
            due_date: inv.due_date.clone(),
            line_items: inv
                .rows
                .as_ref()
                .map(|rows| {
                    rows.iter()
                        .map(|r| LineItem {
                            description: r.article_description.clone(),
                            quantity: r.quantity.map(|q| {
                                Decimal::from_str(&format!("{:.4}", q)).unwrap_or_default()
                            }),
                            unit_price: r.price.map(|p| Money {
                                amount: Decimal::from_str(&format!("{:.6}", p))
                                    .unwrap_or_default(),
                                currency: inv.currency_code.clone(),
                            }),
                            total: Money {
                                amount: Decimal::from_str(&format!(
                                    "{:.6}",
                                    r.total_amount
                                ))
                                .unwrap_or_default(),
                                currency: inv.currency_code.clone(),
                            },
                            account_number: r.account_number.clone(),
                            vat_rate: r.vat_percent,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            raw: serde_json::to_value(inv).unwrap_or_default(),
        }
    }
}

// ─── AccountingProvider Trait Impl ───────────────────────────────────────────

#[async_trait]
impl AccountingProvider for VismaConnector {
    async fn fetch_transactions(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Transaction>, CanonicalError> {
        let from_str = from.format("%Y-%m-%d").to_string();
        let to_str = to.format("%Y-%m-%d").to_string();

        let date_filter = format!("InvoiceDate ge '{from_str}' and InvoiceDate le '{to_str}'");

        // Fetch both supplier and customer invoices in parallel
        let (supplier_res, customer_res) = tokio::try_join!(
            self.get_paged::<VismaSupplierInvoice>(
                "/supplierinvoices",
                &[("$filter", &date_filter)]
            ),
            self.get_paged::<VismaCustomerInvoice>(
                "/customerinvoices",
                &[("$filter", &date_filter)]
            ),
        )?;

        let mut transactions: Vec<Transaction> = supplier_res
            .iter()
            .map(Self::supplier_invoice_to_transaction)
            .collect();

        transactions.extend(
            customer_res
                .iter()
                .map(Self::customer_invoice_to_transaction),
        );

        info!(
            "Visma fetch_transactions: {} supplier + {} customer invoices",
            supplier_res.len(),
            customer_res.len()
        );

        Ok(transactions)
    }

    async fn fetch_invoices(&self) -> Result<Vec<Invoice>, CanonicalError> {
        let unpaid_filter = "Status eq 'Unpaid'";

        let invoices = self
            .get_paged::<VismaSupplierInvoice>(
                "/supplierinvoices",
                &[("$filter", unpaid_filter)],
            )
            .await?;

        let canonical: Vec<Invoice> = invoices
            .iter()
            .map(Self::supplier_invoice_to_canonical)
            .collect();

        info!("Visma fetch_invoices: {} unpaid supplier invoices", canonical.len());
        Ok(canonical)
    }

    async fn create_voucher(
        &self,
        date: &str,
        description: &str,
        debit_account: &str,
        credit_account: &str,
        amount: &Money,
    ) -> Result<String, CanonicalError> {
        let amount_f64: f64 = amount
            .amount
            .to_string()
            .parse()
            .map_err(|_| CanonicalError::ParseError("Invalid voucher amount".into()))?;

        let voucher = VismaVoucherRequest {
            voucher_date: date.to_string(),
            voucher_text: description.to_string(),
            rows: vec![
                VismaVoucherRow {
                    account_number: debit_account.to_string(),
                    debit_amount: Some(amount_f64),
                    credit_amount: None,
                    transaction_text: Some(description.to_string()),
                    cost_center_1: None,
                },
                VismaVoucherRow {
                    account_number: credit_account.to_string(),
                    debit_amount: None,
                    credit_amount: Some(amount_f64),
                    transaction_text: Some(description.to_string()),
                    cost_center_1: None,
                },
            ],
        };

        #[derive(Deserialize)]
        struct VoucherCreatedResponse {
            #[serde(rename = "Id")]
            id: String,
        }

        let resp: VoucherCreatedResponse = self
            .post_with_retry("/vouchers", &voucher)
            .await?;

        info!("Visma create_voucher: created voucher id={}", resp.id);
        Ok(resp.id)
    }

    async fn sync_chart_of_accounts(&self) -> Result<ChartOfAccounts, CanonicalError> {
        let accounts = self
            .get_paged::<VismaAccount>("/accounts", &[])
            .await?;

        let canonical: Vec<Account> = accounts
            .iter()
            .filter(|a| a.is_active)
            .map(|a| Account {
                number: a.number.clone(),
                name: a.name.clone(),
                account_type: a.account_type.clone(),
                vat_code: a.vat_code_id.clone(),
            })
            .collect();

        info!("Visma sync_chart_of_accounts: {} active accounts", canonical.len());
        Ok(ChartOfAccounts { accounts: canonical })
    }

    async fn sync_vendors(&self) -> Result<Vec<Vendor>, CanonicalError> {
        let suppliers = self
            .get_paged::<VismaSupplier>("/suppliers", &[])
            .await?;

        let vendors: Vec<Vendor> = suppliers
            .iter()
            .filter(|s| s.is_active)
            .map(|s| Vendor {
                id: s.id.clone(),
                name: s.name.clone(),
                org_number: s.org_number.clone(),
                email: s.email.clone(),
                phone: s.phone.clone(),
                iban: s
                    .bank_account
                    .as_ref()
                    .and_then(|b| b.iban.clone()),
                bic: s
                    .bank_account
                    .as_ref()
                    .and_then(|b| b.bic.clone()),
                currency: s.currency_code.clone(),
            })
            .collect();

        info!("Visma sync_vendors: {} active suppliers", vendors.len());
        Ok(vendors)
    }
}
