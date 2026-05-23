/// QuickBooks Online connector — AccountingProvider impl
///
/// OAuth2 refresh-token flow, IDS v3 REST API, multi-currency,
/// canonical data model mapping.
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{header, Client};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::RwLock;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Canonical Data Model (shared types — in practice live in crate::types)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub name: String,
    pub org_number: Option<String>,
    pub vat_number: Option<String>,
    pub country_code: String,
    pub address: Option<Address>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub id: Option<String>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub tax_amount: Option<Decimal>,
    pub account_code: Option<String>,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub external_id: String,
    pub txn_type: String,
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    pub exchange_rate: Option<Decimal>,
    pub source_account: String,
    pub destination_account: Option<String>,
    pub vendor: Option<Party>,
    pub customer: Option<Party>,
    pub line_items: Vec<LineItem>,
    pub doc_number: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub external_id: String,
    pub invoice_number: Option<String>,
    pub date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub customer: Party,
    pub line_items: Vec<LineItem>,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub total: Decimal,
    pub balance: Decimal,
    pub currency: String,
    pub exchange_rate: Option<Decimal>,
    pub status: InvoiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Sent,
    PartiallyPaid,
    Paid,
    Void,
    Overdue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub date: NaiveDate,
    pub description: String,
    pub lines: Vec<JournalLine>,
    pub currency: String,
    pub exchange_rate: Option<Decimal>,
    pub doc_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalLine {
    pub account_code: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub posting_type: PostingType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostingType {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub external_id: String,
    pub name: String,
    pub account_type: String,
    pub account_subtype: Option<String>,
    pub account_code: Option<String>,
    pub currency: Option<String>,
    pub active: bool,
    pub balance: Option<Decimal>,
}

// ---------------------------------------------------------------------------
// AccountingProvider trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AccountingProvider: Send + Sync {
    async fn fetch_transactions(
        &self,
        since: DateTime<Utc>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Transaction>>;

    async fn fetch_invoices(&self) -> Result<Vec<Invoice>>;

    async fn create_voucher(&self, entry: &JournalEntry) -> Result<String>;

    async fn sync_chart_of_accounts(&self) -> Result<Vec<Account>>;

    async fn sync_vendors(&self) -> Result<Vec<Party>>;
}

// ---------------------------------------------------------------------------
// QuickBooks-specific config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QboConfig {
    /// Intuit realm / company ID
    pub realm_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub environment: QboEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QboEnvironment {
    Production,
    Sandbox,
}

impl QboEnvironment {
    fn base_url(&self) -> &'static str {
        match self {
            QboEnvironment::Production => "https://quickbooks.api.intuit.com/v3/company",
            QboEnvironment::Sandbox => "https://sandbox-quickbooks.api.intuit.com/v3/company",
        }
    }

    fn token_url(&self) -> &'static str {
        "https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer"
    }
}

// ---------------------------------------------------------------------------
// OAuth2 token state (thread-safe, auto-refresh)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QboToken {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

impl QboToken {
    fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at - chrono::Duration::seconds(60)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IntuitTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    #[serde(rename = "x_refresh_token_expires_in")]
    refresh_token_expires_in: Option<i64>,
    token_type: String,
}

// ---------------------------------------------------------------------------
// QBO raw API response wrappers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct QboQueryResponse<T> {
    #[serde(rename = "QueryResponse")]
    query_response: QboQueryBody<T>,
    time: String,
}

#[derive(Debug, Deserialize)]
struct QboQueryBody<T> {
    #[serde(flatten)]
    items: HashMap<String, Vec<T>>,
    startPosition: Option<i64>,
    maxResults: Option<i64>,
    totalCount: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct QboCreateResponse {
    #[serde(rename = "JournalEntry")]
    journal_entry: Option<QboJournalEntryResponse>,
    time: String,
}

#[derive(Debug, Deserialize)]
struct QboJournalEntryResponse {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "SyncToken")]
    sync_token: String,
}

// ---------------------------------------------------------------------------
// QBO Transaction (IDS v3)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QboTransaction {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "TxnDate")]
    txn_date: String,
    #[serde(rename = "DocNumber")]
    doc_number: Option<String>,
    #[serde(rename = "PrivateNote")]
    private_note: Option<String>,
    #[serde(rename = "CurrencyRef")]
    currency_ref: Option<QboCurrencyRef>,
    #[serde(rename = "ExchangeRate")]
    exchange_rate: Option<f64>,
    #[serde(rename = "TotalAmt")]
    total_amt: f64,
    #[serde(rename = "Line")]
    line: Option<Vec<QboLine>>,
    #[serde(rename = "MetaData")]
    meta_data: Option<QboMetaData>,
    #[serde(rename = "VendorRef")]
    vendor_ref: Option<QboRef>,
    #[serde(rename = "CustomerRef")]
    customer_ref: Option<QboRef>,
    #[serde(rename = "APAccountRef")]
    ap_account_ref: Option<QboRef>,
    #[serde(rename = "ARAccountRef")]
    ar_account_ref: Option<QboRef>,
}

#[derive(Debug, Deserialize)]
struct QboCurrencyRef {
    value: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QboLine {
    #[serde(rename = "Id")]
    id: Option<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "Amount")]
    amount: f64,
    #[serde(rename = "DetailType")]
    detail_type: String,
    #[serde(rename = "JournalEntryLineDetail")]
    journal_entry_line_detail: Option<QboJournalEntryLineDetail>,
    #[serde(rename = "SalesItemLineDetail")]
    sales_item_line_detail: Option<QboSalesItemLineDetail>,
    #[serde(rename = "AccountBasedExpenseLineDetail")]
    account_expense_detail: Option<QboAccountExpenseDetail>,
}

#[derive(Debug, Deserialize)]
struct QboJournalEntryLineDetail {
    #[serde(rename = "PostingType")]
    posting_type: String,
    #[serde(rename = "AccountRef")]
    account_ref: QboRef,
    #[serde(rename = "Entity")]
    entity: Option<QboEntity>,
}

#[derive(Debug, Deserialize)]
struct QboSalesItemLineDetail {
    #[serde(rename = "ItemRef")]
    item_ref: Option<QboRef>,
    #[serde(rename = "Qty")]
    qty: Option<f64>,
    #[serde(rename = "UnitPrice")]
    unit_price: Option<f64>,
    #[serde(rename = "TaxCodeRef")]
    tax_code_ref: Option<QboRef>,
}

#[derive(Debug, Deserialize)]
struct QboAccountExpenseDetail {
    #[serde(rename = "AccountRef")]
    account_ref: QboRef,
    #[serde(rename = "TaxCodeRef")]
    tax_code_ref: Option<QboRef>,
}

#[derive(Debug, Deserialize)]
struct QboEntity {
    #[serde(rename = "EntityRef")]
    entity_ref: Option<QboRef>,
    #[serde(rename = "Type")]
    entity_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QboRef {
    value: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QboMetaData {
    #[serde(rename = "CreateTime")]
    create_time: String,
    #[serde(rename = "LastUpdatedTime")]
    last_updated_time: String,
}

// ---------------------------------------------------------------------------
// QBO Invoice (IDS v3)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QboInvoice {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "DocNumber")]
    doc_number: Option<String>,
    #[serde(rename = "TxnDate")]
    txn_date: String,
    #[serde(rename = "DueDate")]
    due_date: Option<String>,
    #[serde(rename = "CustomerRef")]
    customer_ref: QboRef,
    #[serde(rename = "BillAddr")]
    bill_addr: Option<QboAddress>,
    #[serde(rename = "Line")]
    line: Vec<QboLine>,
    #[serde(rename = "TxnTaxDetail")]
    txn_tax_detail: Option<QboTxnTaxDetail>,
    #[serde(rename = "Subtotal")]
    subtotal: Option<f64>,
    #[serde(rename = "TotalAmt")]
    total_amt: f64,
    #[serde(rename = "Balance")]
    balance: f64,
    #[serde(rename = "CurrencyRef")]
    currency_ref: Option<QboCurrencyRef>,
    #[serde(rename = "ExchangeRate")]
    exchange_rate: Option<f64>,
    #[serde(rename = "EmailStatus")]
    email_status: Option<String>,
    #[serde(rename = "MetaData")]
    meta_data: Option<QboMetaData>,
}

#[derive(Debug, Deserialize)]
struct QboAddress {
    #[serde(rename = "Line1")]
    line1: Option<String>,
    #[serde(rename = "City")]
    city: Option<String>,
    #[serde(rename = "PostalCode")]
    postal_code: Option<String>,
    #[serde(rename = "Country")]
    country: Option<String>,
    #[serde(rename = "CountryCode")]
    country_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QboTxnTaxDetail {
    #[serde(rename = "TotalTax")]
    total_tax: Option<f64>,
    #[serde(rename = "TaxLine")]
    tax_line: Option<Vec<serde_json::Value>>,
}

// ---------------------------------------------------------------------------
// QBO Account
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QboAccount {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AcctNum")]
    acct_num: Option<String>,
    #[serde(rename = "AccountType")]
    account_type: String,
    #[serde(rename = "AccountSubType")]
    account_sub_type: Option<String>,
    #[serde(rename = "CurrencyRef")]
    currency_ref: Option<QboCurrencyRef>,
    #[serde(rename = "Active")]
    active: bool,
    #[serde(rename = "CurrentBalance")]
    current_balance: Option<f64>,
}

// ---------------------------------------------------------------------------
// QBO Vendor
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QboVendor {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "DisplayName")]
    display_name: String,
    #[serde(rename = "CompanyName")]
    company_name: Option<String>,
    #[serde(rename = "PrimaryEmailAddr")]
    primary_email_addr: Option<QboEmail>,
    #[serde(rename = "BillAddr")]
    bill_addr: Option<QboAddress>,
    #[serde(rename = "GivenName")]
    given_name: Option<String>,
    #[serde(rename = "FamilyName")]
    family_name: Option<String>,
    #[serde(rename = "TaxIdentifier")]
    tax_identifier: Option<String>,
    #[serde(rename = "Active")]
    active: bool,
    #[serde(rename = "CurrencyRef")]
    currency_ref: Option<QboCurrencyRef>,
}

#[derive(Debug, Deserialize)]
struct QboEmail {
    #[serde(rename = "Address")]
    address: String,
}

// ---------------------------------------------------------------------------
// QuickBooksConnector
// ---------------------------------------------------------------------------

pub struct QuickBooksConnector {
    config: QboConfig,
    client: Client,
    token: RwLock<QboToken>,
}

impl QuickBooksConnector {
    /// Construct with an initial refresh token (obtained from OAuth2 callback).
    pub async fn new(config: QboConfig, refresh_token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        // Bootstrap: exchange refresh token immediately so we have a valid access token.
        let token = Self::do_refresh(&client, &config, &refresh_token).await?;

        Ok(Self {
            config,
            client,
            token: RwLock::new(token),
        })
    }

    // -----------------------------------------------------------------------
    // OAuth2 helpers
    // -----------------------------------------------------------------------

    async fn do_refresh(client: &Client, config: &QboConfig, refresh_token: &str) -> Result<QboToken> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        let resp = client
            .post(config.environment.token_url())
            .basic_auth(&config.client_id, Some(&config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Token refresh HTTP request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Token refresh failed {}: {}", status, body));
        }

        let token_resp: IntuitTokenResponse = resp.json().await.context("Deserializing token response")?;

        Ok(QboToken {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(token_resp.expires_in),
        })
    }

    /// Ensure access token is valid, refresh if expired.
    async fn ensure_valid_token(&self) -> Result<String> {
        {
            let token = self.token.read().await;
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }
        }

        // Upgrade to write lock and refresh.
        let mut token = self.token.write().await;
        // Double-check: another task may have already refreshed.
        if !token.is_expired() {
            return Ok(token.access_token.clone());
        }

        let new_token =
            Self::do_refresh(&self.client, &self.config, &token.refresh_token).await?;
        *token = new_token;
        Ok(token.access_token.clone())
    }

    // -----------------------------------------------------------------------
    // IDS v3 query helper
    // -----------------------------------------------------------------------

    async fn query_raw(&self, query: &str) -> Result<serde_json::Value> {
        let access_token = self.ensure_valid_token().await?;
        let url = format!(
            "{}/{}/query",
            self.config.environment.base_url(),
            self.config.realm_id
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .header(header::ACCEPT, "application/json")
            .query(&[("query", query), ("minorversion", "65")])
            .send()
            .await
            .context("IDS query HTTP request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("IDS query failed {}: {}", status, body));
        }

        resp.json().await.context("Deserializing IDS query response")
    }

    /// Paginated query — pages through until maxResults < page_size.
    async fn query_all_raw(&self, base_query: &str) -> Result<Vec<serde_json::Value>> {
        let page_size = 500usize;
        let mut start = 1usize;
        let mut all: Vec<serde_json::Value> = Vec::new();

        loop {
            let paged = format!(
                "{} STARTPOSITION {} MAXRESULTS {}",
                base_query, start, page_size
            );
            let body = self.query_raw(&paged).await?;

            let items = body
                .get("QueryResponse")
                .and_then(|qr| {
                    // The QueryResponse may have exactly one array field whose name
                    // matches the entity type (e.g. "Transaction", "Invoice").
                    qr.as_object().and_then(|m| {
                        m.values()
                            .find(|v| v.is_array())
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.to_vec())
                    })
                })
                .unwrap_or_default();

            let count = items.len();
            all.extend(items);

            if count < page_size {
                break;
            }
            start += page_size;
        }

        Ok(all)
    }

    // -----------------------------------------------------------------------
    // Canonical mapping helpers
    // -----------------------------------------------------------------------

    fn parse_decimal(v: f64) -> Decimal {
        // Store as minor units then convert to avoid float imprecision.
        Decimal::from_str(&format!("{:.10}", v)).unwrap_or_default()
    }

    fn parse_date(s: &str) -> NaiveDate {
        // QBO returns dates as "YYYY-MM-DD"
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive())
    }

    fn parse_datetime(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }

    fn map_qbo_address(addr: &QboAddress) -> Address {
        Address {
            street: addr.line1.clone(),
            city: addr.city.clone(),
            postal_code: addr.postal_code.clone(),
            country_code: addr
                .country_code
                .clone()
                .or_else(|| addr.country.clone())
                .unwrap_or_else(|| "US".to_string()),
        }
    }

    fn map_qbo_vendor_to_party(v: &QboVendor) -> Party {
        Party {
            id: Uuid::new_v4(),
            external_id: Some(format!("qbo:{}", v.id)),
            name: v.display_name.clone(),
            org_number: None,
            vat_number: v.tax_identifier.clone(),
            country_code: v
                .bill_addr
                .as_ref()
                .and_then(|a| a.country_code.clone().or_else(|| a.country.clone()))
                .unwrap_or_else(|| "US".to_string()),
            address: v.bill_addr.as_ref().map(Self::map_qbo_address),
            email: v.primary_email_addr.as_ref().map(|e| e.address.clone()),
        }
    }

    fn map_qbo_invoice_line(line: &QboLine) -> Option<LineItem> {
        if line.detail_type == "SubTotalLineDetail" || line.detail_type == "DiscountLineDetail" {
            return None;
        }
        let (qty, unit_price) = match &line.sales_item_line_detail {
            Some(d) => (
                d.qty.map(Self::parse_decimal).unwrap_or(Decimal::ONE),
                d.unit_price.map(Self::parse_decimal).unwrap_or_else(|| Self::parse_decimal(line.amount)),
            ),
            None => (Decimal::ONE, Self::parse_decimal(line.amount)),
        };
        Some(LineItem {
            id: line.id.clone(),
            description: line.description.clone().unwrap_or_default(),
            quantity: qty,
            unit_price,
            amount: Self::parse_decimal(line.amount),
            tax_amount: None,
            account_code: line
                .account_expense_detail
                .as_ref()
                .map(|d| d.account_ref.value.clone()),
            currency: "USD".to_string(), // overridden by caller
        })
    }

    fn map_qbo_transaction(raw: &serde_json::Value) -> Result<Transaction> {
        let t: QboTransaction =
            serde_json::from_value(raw.clone()).context("Deserializing QboTransaction")?;

        let currency = t
            .currency_ref
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_else(|| "USD".to_string());

        let lines: Vec<LineItem> = t
            .line
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter_map(|l| {
                let mut item = Self::map_qbo_invoice_line(l)?;
                item.currency = currency.clone();
                Some(item)
            })
            .collect();

        let (created_at, updated_at) = match &t.meta_data {
            Some(m) => (
                Self::parse_datetime(&m.create_time),
                Self::parse_datetime(&m.last_updated_time),
            ),
            None => (Utc::now(), Utc::now()),
        };

        Ok(Transaction {
            id: Uuid::new_v4(),
            external_id: format!("qbo:{}", t.id),
            txn_type: "Transaction".to_string(),
            date: Self::parse_date(&t.txn_date),
            description: t.private_note.unwrap_or_default(),
            amount: Self::parse_decimal(t.total_amt),
            currency,
            exchange_rate: t.exchange_rate.map(Self::parse_decimal),
            source_account: t
                .ap_account_ref
                .or(t.ar_account_ref)
                .map(|r| r.value)
                .unwrap_or_default(),
            destination_account: None,
            vendor: None,
            customer: None,
            line_items: lines,
            doc_number: t.doc_number,
            metadata: HashMap::new(),
            created_at,
            updated_at,
        })
    }

    fn map_qbo_invoice(raw: &serde_json::Value) -> Result<Invoice> {
        let inv: QboInvoice =
            serde_json::from_value(raw.clone()).context("Deserializing QboInvoice")?;

        let currency = inv
            .currency_ref
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_else(|| "USD".to_string());

        let lines: Vec<LineItem> = inv
            .line
            .iter()
            .filter_map(|l| {
                let mut item = Self::map_qbo_invoice_line(l)?;
                item.currency = currency.clone();
                Some(item)
            })
            .collect();

        let tax = inv
            .txn_tax_detail
            .as_ref()
            .and_then(|d| d.total_tax)
            .map(Self::parse_decimal)
            .unwrap_or_default();

        let subtotal = inv
            .subtotal
            .map(Self::parse_decimal)
            .unwrap_or_else(|| Self::parse_decimal(inv.total_amt) - tax);

        let balance = Self::parse_decimal(inv.balance);
        let status = if balance == Decimal::ZERO {
            InvoiceStatus::Paid
        } else if balance < Self::parse_decimal(inv.total_amt) {
            InvoiceStatus::PartiallyPaid
        } else {
            InvoiceStatus::Sent
        };

        Ok(Invoice {
            id: Uuid::new_v4(),
            external_id: format!("qbo:{}", inv.id),
            invoice_number: inv.doc_number,
            date: Self::parse_date(&inv.txn_date),
            due_date: inv.due_date.as_deref().map(Self::parse_date),
            customer: Party {
                id: Uuid::new_v4(),
                external_id: Some(format!("qbo:{}", inv.customer_ref.value)),
                name: inv.customer_ref.name.unwrap_or_default(),
                org_number: None,
                vat_number: None,
                country_code: inv
                    .bill_addr
                    .as_ref()
                    .and_then(|a| a.country_code.clone().or_else(|| a.country.clone()))
                    .unwrap_or_else(|| "US".to_string()),
                address: inv.bill_addr.as_ref().map(Self::map_qbo_address),
                email: None,
            },
            line_items: lines,
            subtotal,
            tax_total: tax,
            total: Self::parse_decimal(inv.total_amt),
            balance,
            currency,
            exchange_rate: inv.exchange_rate.map(Self::parse_decimal),
            status,
        })
    }
}

// ---------------------------------------------------------------------------
// AccountingProvider implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl AccountingProvider for QuickBooksConnector {
    /// Fetch all transactions modified since `since`.
    /// Maps every IDS Transaction entity (Bill, Invoice, Payment, JE, etc.) to
    /// the canonical Transaction struct.
    async fn fetch_transactions(
        &self,
        since: DateTime<Utc>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Transaction>> {
        let since_str = since.format("%Y-%m-%dT%H:%M:%S").to_string();
        let until_str = until
            .unwrap_or_else(Utc::now)
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        let query = format!(
            "SELECT * FROM Transaction WHERE MetaData.LastUpdatedTime >= '{}' \
             AND MetaData.LastUpdatedTime <= '{}'",
            since_str, until_str
        );

        let raws = self.query_all_raw(&query).await?;

        let transactions: Vec<Transaction> = raws
            .iter()
            .filter_map(|raw| match Self::map_qbo_transaction(raw) {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::warn!("Skipping malformed QBO transaction: {}", e);
                    None
                }
            })
            .collect();

        Ok(transactions)
    }

    /// Fetch all open invoices (Balance > 0).
    async fn fetch_invoices(&self) -> Result<Vec<Invoice>> {
        let query = "SELECT * FROM Invoice WHERE Balance > '0.0'";
        let raws = self.query_all_raw(query).await?;

        let invoices: Vec<Invoice> = raws
            .iter()
            .filter_map(|raw| match Self::map_qbo_invoice(raw) {
                Ok(inv) => Some(inv),
                Err(e) => {
                    tracing::warn!("Skipping malformed QBO invoice: {}", e);
                    None
                }
            })
            .collect();

        Ok(invoices)
    }

    /// Create a journal entry (voucher) in QBO.
    /// Returns the new JournalEntry ID.
    async fn create_voucher(&self, entry: &JournalEntry) -> Result<String> {
        let access_token = self.ensure_valid_token().await?;

        // Build IDS JournalEntry JSON payload.
        let lines: Vec<serde_json::Value> = entry
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let posting_type = match line.posting_type {
                    PostingType::Debit => "Debit",
                    PostingType::Credit => "Credit",
                };
                serde_json::json!({
                    "Id": (i + 1).to_string(),
                    "Description": line.description,
                    "Amount": line.amount.to_string().parse::<f64>().unwrap_or_default(),
                    "DetailType": "JournalEntryLineDetail",
                    "JournalEntryLineDetail": {
                        "PostingType": posting_type,
                        "AccountRef": { "value": line.account_code }
                    }
                })
            })
            .collect();

        let mut payload = serde_json::json!({
            "TxnDate": entry.date.format("%Y-%m-%d").to_string(),
            "PrivateNote": entry.description,
            "Line": lines,
            "CurrencyRef": { "value": entry.currency }
        });

        if let Some(doc_num) = &entry.doc_number {
            payload["DocNumber"] = serde_json::json!(doc_num);
        }
        if let Some(rate) = entry.exchange_rate {
            payload["ExchangeRate"] =
                serde_json::json!(rate.to_string().parse::<f64>().unwrap_or(1.0));
        }

        let url = format!(
            "{}/{}/journalentry",
            self.config.environment.base_url(),
            self.config.realm_id
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .query(&[("minorversion", "65")])
            .json(&payload)
            .send()
            .await
            .context("create_voucher HTTP request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("create_voucher failed {}: {}", status, body));
        }

        let create_resp: QboCreateResponse =
            resp.json().await.context("Deserializing create_voucher response")?;

        create_resp
            .journal_entry
            .map(|je| je.id)
            .ok_or_else(|| anyhow!("No JournalEntry ID in response"))
    }

    /// Sync the full chart of accounts.
    async fn sync_chart_of_accounts(&self) -> Result<Vec<Account>> {
        let query = "SELECT * FROM Account";
        let raws = self.query_all_raw(query).await?;

        let accounts: Vec<Account> = raws
            .iter()
            .filter_map(|raw| {
                let a: QboAccount = serde_json::from_value(raw.clone()).ok()?;
                Some(Account {
                    id: Uuid::new_v4().to_string(),
                    external_id: format!("qbo:{}", a.id),
                    name: a.name,
                    account_type: a.account_type,
                    account_subtype: a.account_sub_type,
                    account_code: a.acct_num,
                    currency: a.currency_ref.map(|c| c.value),
                    active: a.active,
                    balance: a.current_balance.map(Self::parse_decimal),
                })
            })
            .collect();

        Ok(accounts)
    }

    /// Sync all vendors.
    async fn sync_vendors(&self) -> Result<Vec<Party>> {
        let query = "SELECT * FROM Vendor WHERE Active = true";
        let raws = self.query_all_raw(query).await?;

        let vendors: Vec<Party> = raws
            .iter()
            .filter_map(|raw| {
                let v: QboVendor = serde_json::from_value(raw.clone()).ok()?;
                Some(Self::map_qbo_vendor_to_party(&v))
            })
            .collect();

        Ok(vendors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_decimal_precision() {
        let d = QuickBooksConnector::parse_decimal(1234.56);
        assert_eq!(d.to_string(), "1234.5600000000");
    }

    #[test]
    fn parse_date_valid() {
        let d = QuickBooksConnector::parse_date("2024-03-15");
        assert_eq!(d.to_string(), "2024-03-15");
    }

    #[test]
    fn invoice_status_paid_when_balance_zero() {
        // Balance 0 → Paid
        let raw = serde_json::json!({
            "Id": "42",
            "TxnDate": "2024-01-01",
            "CustomerRef": { "value": "1", "name": "ACME" },
            "Line": [],
            "TotalAmt": 1000.0,
            "Balance": 0.0
        });
        let inv = QuickBooksConnector::map_qbo_invoice(&raw).unwrap();
        assert!(matches!(inv.status, InvoiceStatus::Paid));
    }

    #[test]
    fn invoice_status_partial() {
        let raw = serde_json::json!({
            "Id": "43",
            "TxnDate": "2024-01-02",
            "CustomerRef": { "value": "2", "name": "Beta Corp" },
            "Line": [],
            "TotalAmt": 500.0,
            "Balance": 250.0
        });
        let inv = QuickBooksConnector::map_qbo_invoice(&raw).unwrap();
        assert!(matches!(inv.status, InvoiceStatus::PartiallyPaid));
    }
}
