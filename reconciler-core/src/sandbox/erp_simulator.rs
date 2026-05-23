// src/sandbox/erp_simulator.rs
//
// Simulates the Fortnox ERP REST API (v3) via an embedded Axum HTTP server.
// Supports: invoices, vouchers, suppliers; rate limiting (429 every 10th
// request), malformed-JSON injection, configurable latency, and a recording
// buffer for posted vouchers so tests can assert what the reconciler emitted.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use rand::Rng;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use tokio::time::sleep;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Domain types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceStatus {
    Unpaid,
    Paid,
    Overdue,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimulatedInvoice {
    pub document_number:    String,
    pub customer_name:      String,
    pub total_with_vat:     Decimal,
    pub total_without_vat:  Decimal,
    pub vat:                Decimal,
    pub currency:           String,
    pub invoice_date:       String,   // YYYY-MM-DD
    pub due_date:           String,
    pub status:             InvoiceStatus,
    pub our_reference:      String,
    pub your_reference:     String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VoucherRow {
    pub account:     i32,
    pub debit:       Decimal,
    pub credit:      Decimal,
    pub description: String,
    pub transaction_information: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimulatedVoucher {
    pub voucher_series:  String,
    pub voucher_number:  i64,
    pub description:     String,
    pub voucher_date:    String,
    pub year:            i32,
    pub rows:            Vec<VoucherRow>,
    pub reference_type:  String,
    pub reference_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimulatedVendor {
    pub supplier_number:      String,
    pub name:                 String,
    pub organisation_number:  String,
    pub email:                String,
    pub currency:             String,
    pub country:              String,
    pub vat_number:           String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedVoucherRecord {
    pub id:          Uuid,
    pub received_at: DateTime<Utc>,
    pub payload:     Value,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal state (Arc-shared with Axum handlers)
// ─────────────────────────────────────────────────────────────────────────────

pub struct FortnoxState {
    pub invoices:                    Vec<SimulatedInvoice>,
    pub vouchers:                    Vec<SimulatedVoucher>,
    pub vendors:                     Vec<SimulatedVendor>,
    pub posted_vouchers:             Vec<SimulatedVoucherRecord>,
    request_count:                   u64,
    pub simulate_rate_limit:         bool,
    pub malformed_probability:       f64,   // 0.0–1.0
    pub response_delay_ms:           u64,
    voucher_counter:                 i64,
}

type SharedState = Arc<Mutex<FortnoxState>>;

impl FortnoxState {
    fn new() -> Self {
        let mut s = Self {
            invoices:              Vec::new(),
            vouchers:              Vec::new(),
            vendors:               Vec::new(),
            posted_vouchers:       Vec::new(),
            request_count:         0,
            simulate_rate_limit:   false,
            malformed_probability: 0.0,
            response_delay_ms:     0,
            voucher_counter:       1,
        };
        s.seed_vendors();
        s
    }

    fn seed_vendors(&mut self) {
        let vendors = vec![
            SimulatedVendor {
                supplier_number:     "LEV001".to_string(),
                name:                "Amazon Web Services EMEA SARL".to_string(),
                organisation_number: "B229516589".to_string(),
                email:               "aws-billing@amazon.com".to_string(),
                currency:            "EUR".to_string(),
                country:             "LU".to_string(),
                vat_number:          "LU26375245".to_string(),
            },
            SimulatedVendor {
                supplier_number:     "LEV002".to_string(),
                name:                "Telia Sverige AB".to_string(),
                organisation_number: "556103-4249".to_string(),
                email:               "fakturor@telia.se".to_string(),
                currency:            "SEK".to_string(),
                country:             "SE".to_string(),
                vat_number:          "SE556103424901".to_string(),
            },
            SimulatedVendor {
                supplier_number:     "LEV003".to_string(),
                name:                "Slack Technologies LLC".to_string(),
                organisation_number: "US-463869076".to_string(),
                email:               "invoices@slack.com".to_string(),
                currency:            "USD".to_string(),
                country:             "US".to_string(),
                vat_number:          "EU372006816".to_string(),
            },
            SimulatedVendor {
                supplier_number:     "LEV004".to_string(),
                name:                "Fortnox AB".to_string(),
                organisation_number: "556469-6291".to_string(),
                email:               "faktura@fortnox.se".to_string(),
                currency:            "SEK".to_string(),
                country:             "SE".to_string(),
                vat_number:          "SE556469629101".to_string(),
            },
            SimulatedVendor {
                supplier_number:     "LEV005".to_string(),
                name:                "Google Cloud EMEA Limited".to_string(),
                organisation_number: "IE503503".to_string(),
                email:               "billing@google.com".to_string(),
                currency:            "EUR".to_string(),
                country:             "IE".to_string(),
                vat_number:          "IE6388047V".to_string(),
            },
        ];
        self.vendors = vendors;
    }

    fn next_voucher_number(&mut self) -> i64 {
        let n = self.voucher_counter;
        self.voucher_counter += 1;
        n
    }

    /// Check if this request should be rate-limited (every 10th).
    fn should_rate_limit(&mut self) -> bool {
        self.request_count += 1;
        self.simulate_rate_limit && (self.request_count % 10 == 0)
    }

    /// Decide if the response should be intentionally malformed.
    fn should_corrupt(&self) -> bool {
        self.malformed_probability > 0.0
            && rand::thread_rng().gen_bool(self.malformed_probability)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FortnoxSimulator — public façade
// ─────────────────────────────────────────────────────────────────────────────

pub struct FortnoxSimulator {
    state: SharedState,
}

impl Default for FortnoxSimulator {
    fn default() -> Self { Self::new() }
}

impl FortnoxSimulator {
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(FortnoxState::new())) }
    }

    // ── Data seeding ────────────────────────────────────────────────────────

    /// Generate `count` realistic supplier invoices and add them to the store.
    pub fn generate_invoice_set(&self, count: usize) {
        let mut rng   = rand::thread_rng();
        let templates = vec![
            ("Amazon Web Services", "EUR", 50.0_f64, 8000.0_f64),
            ("Telia Sverige AB",    "SEK", 199.0,    799.0),
            ("Slack Technologies",  "USD", 70.0,     800.0),
            ("Fortnox AB",          "SEK", 449.0,    1299.0),
            ("Google Cloud",        "EUR", 20.0,     5000.0),
            ("Microsoft Azure",     "EUR", 100.0,    6000.0),
            ("Adobe Systems",       "SEK", 349.0,    599.0),
            ("GitHub Inc",          "USD", 40.0,     200.0),
        ];

        let mut state = self.state.lock().unwrap();
        let base_doc  = state.invoices.len() as i64;

        for i in 0..count {
            let (name, currency, min, max) = &templates[rng.gen_range(0..templates.len())];
            let total_with_vat    = Decimal::from_f64(rng.gen_range(*min..=*max)).unwrap_or_default().round_dp(2);
            let vat_divisor       = Decimal::from_str("1.25").unwrap();
            let total_without_vat = (total_with_vat / vat_divisor).round_dp(2);
            let vat               = (total_with_vat - total_without_vat).round_dp(2);

            let days_ago: i64 = rng.gen_range(0..60);
            let invoice_dt    = Utc::now() - Duration::days(days_ago);
            let due_dt        = invoice_dt + Duration::days(30);

            let status = if days_ago > 35 {
                InvoiceStatus::Overdue
            } else if rng.gen_bool(0.4) {
                InvoiceStatus::Paid
            } else {
                InvoiceStatus::Unpaid
            };

            state.invoices.push(SimulatedInvoice {
                document_number:   format!("{}", base_doc + i as i64 + 1001),
                customer_name:     name.to_string(),
                total_with_vat,
                total_without_vat,
                vat,
                currency:          currency.to_string(),
                invoice_date:      invoice_dt.format("%Y-%m-%d").to_string(),
                due_date:          due_dt.format("%Y-%m-%d").to_string(),
                status,
                our_reference:     format!("PO-{:05}", base_doc + i as i64),
                your_reference:    Uuid::new_v4().to_string()[..8].to_uppercase(),
            });
        }
    }

    // ── Fault injection controls ─────────────────────────────────────────────

    /// Enable rate-limit simulation (HTTP 429 on every 10th request).
    pub fn simulate_rate_limit(&self) {
        self.state.lock().unwrap().simulate_rate_limit = true;
    }

    pub fn disable_rate_limit(&self) {
        self.state.lock().unwrap().simulate_rate_limit = false;
    }

    /// Set probability of returning a malformed (corrupted) JSON response.
    pub fn simulate_malformed_response(&self, probability: f64) {
        self.state.lock().unwrap().malformed_probability = probability.clamp(0.0, 1.0);
    }

    /// Add artificial latency to every response.
    pub fn simulate_slow_response(&self, delay_ms: u64) {
        self.state.lock().unwrap().response_delay_ms = delay_ms;
    }

    pub fn disable_slow_response(&self) {
        self.state.lock().unwrap().response_delay_ms = 0;
    }

    // ── Voucher recording ────────────────────────────────────────────────────

    /// Manually record a voucher (use from tests to bypass HTTP layer).
    pub fn record_posted_voucher(&self, voucher: Value) {
        self.state.lock().unwrap().posted_vouchers.push(SimulatedVoucherRecord {
            id:          Uuid::new_v4(),
            received_at: Utc::now(),
            payload:     voucher,
        });
    }

    pub fn posted_voucher_count(&self) -> usize {
        self.state.lock().unwrap().posted_vouchers.len()
    }

    pub fn get_posted_vouchers(&self) -> Vec<SimulatedVoucherRecord> {
        self.state.lock().unwrap().posted_vouchers.clone()
    }

    pub fn invoice_count(&self) -> usize {
        self.state.lock().unwrap().invoices.len()
    }

    // ── Server ───────────────────────────────────────────────────────────────

    /// Start the Axum HTTP server on `port`.  Runs until the process exits.
    pub async fn start_server(&self, port: u16) -> Result<(), String> {
        let app = build_router(self.state.clone());
        let addr = format!("127.0.0.1:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Cannot bind {addr}: {e}"))?;

        println!("[FortnoxSimulator] listening on http://{addr}");
        axum::serve(listener, app)
            .await
            .map_err(|e| format!("Server error: {e}"))
    }

    /// Same as `start_server` but spawned in the background; returns the port.
    pub async fn start_background(&self, port: u16) -> Result<u16, String> {
        let state = self.state.clone();
        tokio::spawn(async move {
            let app      = build_router(state);
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
                .await
                .expect("bind failed");
            axum::serve(listener, app).await.unwrap();
        });
        // Give the server a moment to be ready
        sleep(std::time::Duration::from_millis(50)).await;
        Ok(port)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Router construction
// ─────────────────────────────────────────────────────────────────────────────

fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/3/invoices",         get(list_invoices).post(create_invoice))
        .route("/3/invoices/:id",     get(get_invoice))
        .route("/3/vouchers",         get(list_vouchers).post(create_voucher))
        .route("/3/vouchers/:series/:number", get(get_voucher))
        .route("/3/suppliers",        get(list_suppliers))
        .route("/3/suppliers/:id",    get(get_supplier))
        .with_state(state)
}

// ─────────────────────────────────────────────────────────────────────────────
// Middleware helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Apply delay + rate-limit + malformed checks at the top of every handler.
/// Returns `None` if the request should proceed normally, or `Some(response)`.
async fn preflight(state: &SharedState) -> Option<(StatusCode, String)> {
    let (delay, rate_limit, corrupt) = {
        let mut s = state.lock().unwrap();
        let rl    = s.should_rate_limit();
        let delay = s.response_delay_ms;
        let corr  = s.should_corrupt();
        (delay, rl, corr)
    };

    if delay > 0 {
        sleep(std::time::Duration::from_millis(delay)).await;
    }

    if rate_limit {
        return Some((
            StatusCode::TOO_MANY_REQUESTS,
            json!({
                "ErrorInformation": {
                    "error": 429,
                    "message": "Rate limit exceeded. Please retry after 5 seconds.",
                    "code": 2001560
                }
            })
            .to_string(),
        ));
    }

    if corrupt {
        return Some((
            StatusCode::OK,
            r#"{"Invoices":[{"DocumentNumber":broken json"#.to_string(),
        ));
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Invoice handlers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ListQuery {
    offset: Option<usize>,
    limit:  Option<usize>,
}

async fn list_invoices(
    State(state): State<SharedState>,
    Query(q):     Query<ListQuery>,
) -> impl IntoResponse {
    if let Some((code, body)) = preflight(&state).await {
        return (code, body);
    }
    let s      = state.lock().unwrap();
    let offset = q.offset.unwrap_or(0);
    let limit  = q.limit.unwrap_or(100).min(500);
    let page: Vec<&SimulatedInvoice> = s.invoices.iter().skip(offset).take(limit).collect();
    let body = json!({ "Invoices": page, "MetaInformation": { "@TotalResources": s.invoices.len(), "@TotalPages": 1, "@CurrentPage": 1 } });
    (StatusCode::OK, body.to_string())
}

async fn get_invoice(
    State(state): State<SharedState>,
    Path(id):     Path<String>,
) -> impl IntoResponse {
    if let Some((code, body)) = preflight(&state).await {
        return (code, body);
    }
    let s = state.lock().unwrap();
    match s.invoices.iter().find(|inv| inv.document_number == id) {
        Some(inv) => (StatusCode::OK, json!({ "Invoice": inv }).to_string()),
        None      => (StatusCode::NOT_FOUND, json!({ "ErrorInformation": { "error": 404, "message": "Invoice not found" } }).to_string()),
    }
}

async fn create_invoice(
    State(state): State<SharedState>,
    Json(body):   Json<Value>,
) -> impl IntoResponse {
    if let Some((code, resp)) = preflight(&state).await {
        return (code, resp);
    }
    let mut s    = state.lock().unwrap();
    let doc_num  = format!("{}", 1000 + s.invoices.len() + 1);
    let invoice  = SimulatedInvoice {
        document_number:   doc_num.clone(),
        customer_name:     body["CustomerName"].as_str().unwrap_or("Unknown").to_string(),
        total_with_vat:    Decimal::from_f64(body["Total"].as_f64().unwrap_or(0.0)).unwrap_or_default().round_dp(2),
        total_without_vat: Decimal::from_f64(body["TotalExclTax"].as_f64().unwrap_or(0.0)).unwrap_or_default().round_dp(2),
        vat:               Decimal::from_f64(body["TotalVAT"].as_f64().unwrap_or(0.0)).unwrap_or_default().round_dp(2),
        currency:          body["Currency"].as_str().unwrap_or("SEK").to_string(),
        invoice_date:      Utc::now().format("%Y-%m-%d").to_string(),
        due_date:          (Utc::now() + Duration::days(30)).format("%Y-%m-%d").to_string(),
        status:            InvoiceStatus::Unpaid,
        our_reference:     body["OurReference"].as_str().unwrap_or("").to_string(),
        your_reference:    body["YourReference"].as_str().unwrap_or("").to_string(),
    };
    s.invoices.push(invoice.clone());
    (StatusCode::CREATED, json!({ "Invoice": invoice }).to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Voucher handlers
// ─────────────────────────────────────────────────────────────────────────────

async fn list_vouchers(
    State(state): State<SharedState>,
    Query(q):     Query<ListQuery>,
) -> impl IntoResponse {
    if let Some((code, body)) = preflight(&state).await {
        return (code, body);
    }
    let s      = state.lock().unwrap();
    let offset = q.offset.unwrap_or(0);
    let limit  = q.limit.unwrap_or(100).min(500);
    let page: Vec<&SimulatedVoucher> = s.vouchers.iter().skip(offset).take(limit).collect();
    (StatusCode::OK, json!({ "Vouchers": page }).to_string())
}

async fn get_voucher(
    State(state): State<SharedState>,
    Path((series, number)): Path<(String, i64)>,
) -> impl IntoResponse {
    if let Some((code, body)) = preflight(&state).await {
        return (code, body);
    }
    let s = state.lock().unwrap();
    match s.vouchers.iter().find(|v| v.voucher_series == series && v.voucher_number == number) {
        Some(v) => (StatusCode::OK, json!({ "Voucher": v }).to_string()),
        None    => (StatusCode::NOT_FOUND, json!({ "ErrorInformation": { "error": 404, "message": "Voucher not found" } }).to_string()),
    }
}

async fn create_voucher(
    State(state): State<SharedState>,
    Json(body):   Json<Value>,
) -> impl IntoResponse {
    if let Some((code, resp)) = preflight(&state).await {
        return (code, resp);
    }

    let mut s      = state.lock().unwrap();
    let vnum       = s.next_voucher_number();
    let rows_raw   = body["Voucher"]["VoucherRows"].as_array().cloned().unwrap_or_default();

    let rows: Vec<VoucherRow> = rows_raw
        .iter()
        .map(|r| VoucherRow {
            account:                 r["Account"].as_i64().unwrap_or(0) as i32,
            debit:                   Decimal::from_f64(r["Debit"].as_f64().unwrap_or(0.0)).unwrap_or_default().round_dp(2),
            credit:                  Decimal::from_f64(r["Credit"].as_f64().unwrap_or(0.0)).unwrap_or_default().round_dp(2),
            description:             r["Description"].as_str().unwrap_or("").to_string(),
            transaction_information: r["TransactionInformation"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    let voucher = SimulatedVoucher {
        voucher_series:   body["Voucher"]["VoucherSeries"].as_str().unwrap_or("A").to_string(),
        voucher_number:   vnum,
        description:      body["Voucher"]["Description"].as_str().unwrap_or("").to_string(),
        voucher_date:     body["Voucher"]["VoucherDate"].as_str().unwrap_or(&Utc::now().format("%Y-%m-%d").to_string()).to_string(),
        year:             Utc::now().year(),
        rows,
        reference_type:   body["Voucher"]["ReferenceType"].as_str().unwrap_or("TRANSACTION").to_string(),
        reference_number: body["Voucher"]["ReferenceNumber"].as_str().unwrap_or("").to_string(),
    };

    // Also record in the posted_vouchers log for test assertions
    s.posted_vouchers.push(SimulatedVoucherRecord {
        id:          Uuid::new_v4(),
        received_at: Utc::now(),
        payload:     body,
    });

    s.vouchers.push(voucher.clone());
    (StatusCode::CREATED, json!({ "Voucher": voucher }).to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Supplier handlers
// ─────────────────────────────────────────────────────────────────────────────

async fn list_suppliers(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    if let Some((code, body)) = preflight(&state).await {
        return (code, body);
    }
    let s = state.lock().unwrap();
    (StatusCode::OK, json!({ "Suppliers": s.vendors }).to_string())
}

async fn get_supplier(
    State(state): State<SharedState>,
    Path(id):     Path<String>,
) -> impl IntoResponse {
    if let Some((code, body)) = preflight(&state).await {
        return (code, body);
    }
    let s = state.lock().unwrap();
    match s.vendors.iter().find(|v| v.supplier_number == id) {
        Some(v) => (StatusCode::OK, json!({ "Supplier": v }).to_string()),
        None    => (StatusCode::NOT_FOUND, json!({ "ErrorInformation": { "error": 404, "message": "Supplier not found" } }).to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_invoice_set() {
        let sim = FortnoxSimulator::new();
        sim.generate_invoice_set(20);
        assert_eq!(sim.invoice_count(), 20);
    }

    #[test]
    fn test_record_posted_voucher() {
        let sim = FortnoxSimulator::new();
        sim.record_posted_voucher(json!({ "Voucher": { "Description": "test" } }));
        assert_eq!(sim.posted_voucher_count(), 1);
    }

    #[test]
    fn test_state_vendors_seeded() {
        let state = FortnoxState::new();
        assert!(!state.vendors.is_empty(), "vendors should be pre-seeded");
    }

    #[test]
    fn test_rate_limit_triggers_every_tenth() {
        let mut state = FortnoxState::new();
        state.simulate_rate_limit = true;
        let results: Vec<bool> = (1..=20).map(|_| state.should_rate_limit()).collect();
        let limited: Vec<usize> = results
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(i, _)| i + 1)
            .collect();
        // Positions 10 and 20 should be rate-limited (1-indexed)
        assert!(limited.contains(&10));
        assert!(limited.contains(&20));
    }

    #[tokio::test]
    async fn test_server_responds_to_invoices() {
        let sim  = FortnoxSimulator::new();
        sim.generate_invoice_set(5);
        let port = sim.start_background(18080).await.unwrap();
        let url  = format!("http://127.0.0.1:{port}/3/invoices");
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body: Value = resp.json().await.unwrap();
        assert!(body["Invoices"].is_array());
        assert_eq!(body["Invoices"].as_array().unwrap().len(), 5);
    }
}
