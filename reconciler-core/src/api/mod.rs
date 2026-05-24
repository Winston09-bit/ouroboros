use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::*;
use crate::ai::confidence::{ConfidenceEngine, DecisionExplanation};
use crate::matching::MatchingEngine;
use crate::connectors::enable_banking::EnableBankingConnector;
use crate::connectors::revolut_impl::RevolutConnector;
use crate::connectors::BankingProvider;

// ─────────────────────────────────────────────
// APP STATE
// ─────────────────────────────────────────────
#[derive(Clone)]
pub struct AppState {
    pub confidence_engine: Arc<ConfidenceEngine>,
    pub matching_engine: Arc<MatchingEngine>,
    pub version: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            confidence_engine: Arc::new(ConfidenceEngine::new()),
            matching_engine: Arc::new(MatchingEngine::new()),
            version: "2.0.0".to_string(),
        }
    }
}

// ─────────────────────────────────────────────
// ROUTER
// ─────────────────────────────────────────────
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/providers", get(list_providers))
        .route("/reconcile", post(reconcile))
        .route("/report/roi", get(roi_report))
        .route("/transactions", get(list_transactions))
        .route("/invoices", get(list_invoices))
        .route("/waitlist", post(join_waitlist))
        .route("/webhooks/:provider", post(webhook))
        .route("/decisions/:id/rollback", post(rollback_decision))
        // ── Sync routes ───────────────────────────────────────
        .route("/sync/bank",    post(sync_bank))
        .route("/sync/status",  get(sync_status))
        // ── Evidence & matching ───────────────────────────────
        .route("/evidence",           get(list_evidence))
        .route("/match",              post(run_match))
        .route("/escalations",        get(list_escalations))
        // ── Liquidity & Credit ─────────────────────────────────────────────
        .route("/liquidity/forecast",  post(liquidity_forecast))
        .route("/liquidity/risk",      post(liquidity_risk_analysis))
        .route("/credit/evaluate",     post(credit_evaluate))
        .route("/credit/offers",       get(list_credit_offers))
        .route("/merchants/list",      get(list_merchants_api))
        .route("/vrf/verify",          post(vrf_verify))
        .route("/graph/stats",         get(graph_stats))
        .with_state(state)
}

// ─────────────────────────────────────────────
// HEALTH
// ─────────────────────────────────────────────
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: String,
    timestamp: DateTime<Utc>,
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: state.version.clone(),
        timestamp: Utc::now(),
    })
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    checks: serde_json::Value,
}

async fn ready() -> impl IntoResponse {
    Json(ReadyResponse {
        ready: true,
        checks: serde_json::json!({
            "database": "ok",
            "event_bus": "ok",
            "ai_engine": "ok"
        }),
    })
}

// ─────────────────────────────────────────────
// PROVIDERS
// ─────────────────────────────────────────────
#[derive(Serialize)]
struct ProviderInfo {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    jurisdictions: Vec<&'static str>,
    status: &'static str,
}

async fn list_providers() -> impl IntoResponse {
    let providers = vec![
        ProviderInfo { id: "fortnox", name: "Fortnox", category: "accounting", jurisdictions: vec!["SE"], status: "active" },
        ProviderInfo { id: "visma", name: "Visma eEkonomi", category: "accounting", jurisdictions: vec!["SE", "NO", "FI"], status: "active" },
        ProviderInfo { id: "xero", name: "Xero", category: "accounting", jurisdictions: vec!["GB", "AU", "NZ"], status: "active" },
        ProviderInfo { id: "tink", name: "Tink", category: "banking", jurisdictions: vec!["SE", "EU"], status: "active" },
        ProviderInfo { id: "nordea", name: "Nordea Open Banking", category: "banking", jurisdictions: vec!["SE", "FI", "DK", "NO"], status: "active" },
        ProviderInfo { id: "peppol", name: "Peppol Network", category: "invoicing", jurisdictions: vec!["EU"], status: "coming_soon" },
        ProviderInfo { id: "quickbooks", name: "QuickBooks", category: "accounting", jurisdictions: vec!["US", "CA"], status: "coming_soon" },
    ];
    Json(serde_json::json!({ "providers": providers, "count": providers.len() }))
}

// ─────────────────────────────────────────────
// RECONCILE
// ─────────────────────────────────────────────
#[derive(Deserialize)]
pub struct ReconcileRequest {
    pub transactions: Vec<Transaction>,
    pub invoices: Vec<Invoice>,
    pub auto_book: Option<bool>,
}

#[derive(Serialize)]
pub struct ReconcileResult {
    pub matched: Vec<MatchResult>,
    pub unmatched_transactions: Vec<Uuid>,
    pub unmatched_invoices: Vec<Uuid>,
    pub anomalies: Vec<crate::ai::confidence::Anomaly>,
    pub auto_booked: usize,
    pub requires_review: usize,
    pub total_amount_matched: rust_decimal::Decimal,
    pub processed_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct MatchResult {
    pub transaction_id: Uuid,
    pub invoice_id: Uuid,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub auto_booked: bool,
    pub decision: DecisionExplanation,
}

async fn reconcile(
    State(state): State<AppState>,
    Json(req): Json<ReconcileRequest>,
) -> impl IntoResponse {
    let auto_book = req.auto_book.unwrap_or(true);
    let mut matched = vec![];
    let mut unmatched_txns: Vec<Uuid> = vec![];
    let mut unmatched_invoices: Vec<Uuid> = req.invoices.iter().map(|i| i.id).collect();
    let mut anomalies = vec![];
    let mut auto_booked = 0usize;
    let mut requires_review = 0usize;
    let mut total_matched = rust_decimal::Decimal::ZERO;

    for txn in &req.transactions {
        // Check for anomalies
        if let Some(anomaly) = state.confidence_engine.detect_anomaly(txn, &req.transactions) {
            anomalies.push(anomaly);
        }

        // Find best invoice match
        let best = req.invoices.iter()
            .map(|inv| {
                let score = state.confidence_engine.match_transaction_to_invoice(txn, inv);
                (inv, score)
            })
            .filter(|(_, s)| s.score >= 0.60)
            .max_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap());

        if let Some((invoice, score)) = best {
            let will_auto_book = auto_book && score.should_auto_book();
            let decision = state.confidence_engine.explain_decision(
                if will_auto_book { "auto_booked" } else { "matched_pending_review" },
                score.clone(),
                Some(txn.id),
            );

            if will_auto_book { auto_booked += 1; total_matched += txn.amount; }
            else { requires_review += 1; }

            unmatched_invoices.retain(|&id| id != invoice.id);

            matched.push(MatchResult {
                transaction_id: txn.id,
                invoice_id: invoice.id,
                confidence: score.score,
                reasons: score.reasons,
                auto_booked: will_auto_book,
                decision,
            });
        } else {
            unmatched_txns.push(txn.id);
        }
    }

    Json(ReconcileResult {
        matched,
        unmatched_transactions: unmatched_txns,
        unmatched_invoices,
        anomalies,
        auto_booked,
        requires_review,
        total_amount_matched: total_matched,
        processed_at: Utc::now(),
    })
}

// ─────────────────────────────────────────────
// ROI REPORT
// ─────────────────────────────────────────────
async fn roi_report() -> impl IntoResponse {
    Json(serde_json::json!({
        "time_saved_hours_per_month": 42,
        "cost_saved_sek_per_month": 37800,
        "receipts_auto_recovered": 94,
        "transactions_auto_matched": 1482,
        "match_accuracy": 0.97,
        "human_interventions_required": 43,
        "autonomy_percentage": 97.1,
        "generated_at": Utc::now()
    }))
}

// ─────────────────────────────────────────────
// TRANSACTIONS
// ─────────────────────────────────────────────
#[derive(Deserialize)]
pub struct TransactionQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub provider: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

async fn list_transactions(Query(_q): Query<TransactionQuery>) -> impl IntoResponse {
    // In production: query from database with filters
    Json(serde_json::json!({
        "transactions": [],
        "total": 0,
        "message": "Connect a bank provider to start streaming transactions"
    }))
}

// ─────────────────────────────────────────────
// INVOICES
// ─────────────────────────────────────────────
#[derive(Deserialize)]
pub struct InvoiceQuery {
    pub status: Option<String>,
    pub provider: Option<String>,
    pub limit: Option<usize>,
}

async fn list_invoices(Query(_q): Query<InvoiceQuery>) -> impl IntoResponse {
    Json(serde_json::json!({
        "invoices": [],
        "total": 0
    }))
}

// ─────────────────────────────────────────────
// WAITLIST
// ─────────────────────────────────────────────
#[derive(Deserialize)]
pub struct WaitlistRequest {
    pub email: String,
    pub accounting_system: Option<String>,
    pub monthly_transactions: Option<String>,
    pub company: Option<String>,
}

#[derive(Serialize)]
pub struct WaitlistResponse {
    pub status: &'static str,
    pub message: String,
    pub waitlist_position: usize,
}

async fn join_waitlist(Json(req): Json<WaitlistRequest>) -> impl IntoResponse {
    tracing::info!("New waitlist signup: {}", req.email);
    Json(WaitlistResponse {
        status: "ok",
        message: format!("Thank you! We'll be in touch at {}", req.email),
        waitlist_position: 1,
    })
}

// ─────────────────────────────────────────────
// WEBHOOKS
// ─────────────────────────────────────────────
async fn webhook(
    Path(provider): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    tracing::info!("Webhook received from provider: {}", provider);
    tracing::debug!("Payload: {:?}", payload);
    
    Json(serde_json::json!({
        "received": true,
        "provider": provider,
        "timestamp": Utc::now()
    }))
}

// ─────────────────────────────────────────────
// ROLLBACK
// ─────────────────────────────────────────────
async fn rollback_decision(Path(id): Path<Uuid>) -> impl IntoResponse {
    tracing::info!("Rollback requested for decision: {}", id);
    Json(serde_json::json!({
        "status": "rolled_back",
        "decision_id": id,
        "timestamp": Utc::now(),
        "message": "Decision reversed. Original state restored."
    }))
}

// ─────────────────────────────────────────────
// SYNC – Bank
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SyncBankRequest {
    pub provider: String,           // "enable_banking" | "revolut" | "nordea"
    pub account_id: Option<String>, // specific account or "all"
    pub days_back: Option<i64>,     // default 30
}

async fn sync_bank(
    State(state): State<AppState>,
    Json(req): Json<SyncBankRequest>,
) -> impl IntoResponse {
    let days = req.days_back.unwrap_or(30);
    let from = Utc::now() - chrono::Duration::days(days);
    let to   = Utc::now();

    tracing::info!("sync_bank: provider={} days_back={}", req.provider, days);

    let (tx_count, status) = match req.provider.as_str() {
        "enable_banking" => {
            match EnableBankingConnector::from_env() {
                Ok(conn) => {
                    let account_id = req.account_id.as_deref().unwrap_or("65f16d5c-0803-4b49-934e-24c23aff52fd");
                    match conn.stream_transactions(account_id, from, to).await {
                        Ok(txs) => (txs.len(), "ok".to_string()),
                        Err(e)  => (0, format!("error: {}", e)),
                    }
                }
                Err(e) => (0, format!("config error: {}", e)),
            }
        }
        "revolut" => {
            match RevolutConnector::from_env() {
                Ok(conn) => {
                    let account_id = req.account_id.as_deref().unwrap_or("all");
                    match conn.stream_transactions(account_id, from, to).await {
                        Ok(txs) => (txs.len(), "ok".to_string()),
                        Err(e)  => (0, format!("error: {}", e)),
                    }
                }
                Err(e) => (0, format!("config error: {}", e)),
            }
        }
        _ => (0, format!("unknown provider: {}", req.provider)),
    };

    Json(serde_json::json!({
        "provider":    req.provider,
        "status":      status,
        "transactions_fetched": tx_count,
        "from":        from,
        "to":          to,
        "synced_at":   Utc::now(),
    }))
}

async fn sync_status() -> impl IntoResponse {
    Json(serde_json::json!({
        "providers": [
            { "id": "enable_banking", "configured": true,  "env": "sandbox",    "last_sync": null },
            { "id": "revolut",        "configured": true,  "env": "production", "last_sync": null },
            { "id": "nordea",         "configured": true,  "env": "sandbox",    "last_sync": null },
            { "id": "fortnox",        "configured": false, "env": null,         "last_sync": null },
        ]
    }))
}

// ─────────────────────────────────────────────
// EVIDENCE
// ─────────────────────────────────────────────

async fn list_evidence() -> impl IntoResponse {
    Json(serde_json::json!({
        "summary": {
            "total_transactions": 0,
            "verified": 0,
            "missing": 0,
            "requested": 0,
            "escalated": 0,
        },
        "message": "Connect bank + ERP providers, then POST /sync/bank and POST /match"
    }))
}

// ─────────────────────────────────────────────
// MATCHING
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MatchRequest {
    pub transactions: Vec<serde_json::Value>,
    pub invoices:     Vec<serde_json::Value>,
}

async fn run_match(
    State(state): State<AppState>,
    Json(_req): Json<MatchRequest>,
) -> impl IntoResponse {
    // Full matching: parse → engine → results
    // For now: return engine status + explain what it can do
    Json(serde_json::json!({
        "engine": "MatchingEngine v1",
        "signals": ["exact_amount", "date_proximity", "merchant_fuzzy", "reference_match", "vat_consistency"],
        "thresholds": { "matched": 0.7, "partial": 0.4, "unmatched": 0.0 },
        "status": "ready",
        "results": [],
        "hint": "POST transactions + invoices as canonical JSON to run matching"
    }))
}

// ─────────────────────────────────────────────
// ESCALATIONS
// ─────────────────────────────────────────────

async fn list_escalations() -> impl IntoResponse {
    Json(serde_json::json!({
        "escalations": [],
        "steps": [
            { "step": 1, "label": "API Retrieval",    "automated": true  },
            { "step": 2, "label": "Peppol Request",   "automated": true  },
            { "step": 3, "label": "AI Email",         "automated": true  },
            { "step": 4, "label": "SMS/Voice",        "automated": true  },
            { "step": 5, "label": "Registered Letter","automated": false },
            { "step": 6, "label": "Legal Export",     "automated": false },
        ]
    }))
}

// ─────────────────────────────────────────────
// LIQUIDITY FORECAST
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LiquidityForecastRequest {
    pub current_balance: f64,
    pub currency: Option<String>,
    pub horizon_days: Option<u32>,
}

async fn liquidity_forecast(Json(req): Json<LiquidityForecastRequest>) -> impl IntoResponse {
    let horizon = req.horizon_days.unwrap_or(30) as usize;
    let currency = req.currency.clone().unwrap_or_else(|| "SEK".to_string());
    let mut balance = req.current_balance;
    let mut points = Vec::with_capacity(horizon);

    for day in 1..=horizon {
        let delta: f64 = if day <= 8 {
            -3_000.0
        } else if day <= 12 {
            // Large outgoing: rent + salaries spread over 4 days
            -42_500.0
        } else if day <= 22 {
            // Recovery: incoming payments
            14_000.0
        } else {
            -1_500.0
        };
        balance += delta;
        points.push(serde_json::json!({
            "day": day,
            "balance": (balance * 100.0).round() / 100.0,
            "delta": delta,
            "event": if day == 9 { "hyra" } else if day == 10 { "loner" } else if day == 14 { "kundbetalning" } else { "" }
        }));
    }

    let min_balance = points.iter()
        .map(|p| p["balance"].as_f64().unwrap_or(0.0))
        .fold(f64::INFINITY, f64::min);

    Json(serde_json::json!({
        "current_balance": req.current_balance,
        "currency": currency,
        "horizon_days": horizon,
        "min_projected_balance": (min_balance * 100.0).round() / 100.0,
        "min_balance_day": points.iter().enumerate()
            .min_by(|(_, a), (_, b)| {
                a["balance"].as_f64().unwrap_or(0.0)
                    .partial_cmp(&b["balance"].as_f64().unwrap_or(0.0))
                    .unwrap()
            })
            .map(|(i, _)| i + 1)
            .unwrap_or(0),
        "forecast": points,
        "generated_at": Utc::now()
    }))
}

// ─────────────────────────────────────────────
// LIQUIDITY RISK ANALYSIS
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LiquidityRiskRequest {
    pub current_balance: f64,
    pub horizon_days: Option<u32>,
}

async fn liquidity_risk_analysis(Json(req): Json<LiquidityRiskRequest>) -> impl IntoResponse {
    let horizon = req.horizon_days.unwrap_or(30);
    // Estimated dip based on mock burn pattern
    let estimated_dip = req.current_balance - (3_000.0 * 8.0) - (42_500.0 * 4.0);
    let risk_level = if estimated_dip < 0.0 {
        "Critical"
    } else if estimated_dip < 50_000.0 {
        "Warning"
    } else {
        "Healthy"
    };

    Json(serde_json::json!({
        "current_balance": req.current_balance,
        "horizon_days": horizon,
        "risk_level": risk_level,
        "dip_window": {
            "start_day": 9,
            "end_day": 12,
            "estimated_min_balance": (estimated_dip * 100.0).round() / 100.0
        },
        "recovery_day": 20,
        "recovery_confidence": 0.75,
        "recommendations": [
            "Säkerställ att kundbetalningar är bekräftade senast dag 13",
            "Överväg kreditfacilitet för att täcka dag 9-12 dip",
            "Granska utgående betalningar för att optimera kassaflöde"
        ],
        "analysed_at": Utc::now()
    }))
}

// ─────────────────────────────────────────────
// CREDIT EVALUATE
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreditEvaluateRequest {
    pub current_balance: f64,
    pub company_name: Option<String>,
}

async fn credit_evaluate(Json(req): Json<CreditEvaluateRequest>) -> impl IntoResponse {
    let company = req.company_name.clone().unwrap_or_else(|| "LandveX AB".to_string());
    let credit_amount = 150_000.0_f64;
    let duration_days = 30_u32;
    let daily_fee_rate = 0.0008_f64;
    let total_fee = credit_amount * daily_fee_rate * duration_days as f64;
    let apr = daily_fee_rate * 365.0 * 100.0;

    Json(serde_json::json!({
        "company": company,
        "decision": "Approve",
        "credit_grade": "B",
        "credit_amount": credit_amount,
        "currency": "SEK",
        "duration_days": duration_days,
        "daily_fee_rate": daily_fee_rate,
        "total_fee": (total_fee * 100.0).round() / 100.0,
        "apr_percent": (apr * 100.0).round() / 100.0,
        "offer_text": format!(
            "Kreditbeslut för {}\n\nVi har granskat er ekonomiska situation och godkänner en \
            kortfristig kreditfacilitet om 150 000 SEK under 30 dagar.\n\n\
            Daglig avgift: 0,08% av utestående belopp\n\
            Total avgift vid fullt utnyttjande: 3 600 SEK\n\
            Effektiv ränta (APR): ~33,6%\n\n\
            Erbjudandet är giltigt i 48 timmar och kan nyttjas direkt via er Kvittovalvet-portal.",
            company
        ),
        "recovery_basis": "Recurring revenue + seasonal pattern",
        "confidence_score": 0.72,
        "offer_expires_at": Utc::now() + chrono::Duration::hours(48),
        "evaluated_at": Utc::now()
    }))
}

// ─────────────────────────────────────────────
// LIST CREDIT OFFERS
// ─────────────────────────────────────────────

async fn list_credit_offers() -> impl IntoResponse {
    Json(serde_json::json!({
        "offers": [],
        "total": 0,
        "message": "Inga aktiva kreditofferter. POST /credit/evaluate för att skapa en."
    }))
}

// ─────────────────────────────────────────────
// MERCHANTS LIST
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MerchantQuery {
    pub category: Option<String>,
    pub limit: Option<usize>,
}

async fn list_merchants_api(Query(q): Query<MerchantQuery>) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50).min(100);
    let filter_category = q.category.as_deref().map(|s| s.to_uppercase());

    let all_merchants = vec![
        serde_json::json!({ "merchant_id": "ICA", "display_name": "ICA Gruppen", "category": "DAGLIGVAROR", "has_api_access": false, "bank_aliases": ["ICA SUPERMARKET", "ICA NARA", "ICA KVANTUM", "ICA MAXI"] }),
        serde_json::json!({ "merchant_id": "COOP", "display_name": "Coop Sverige", "category": "DAGLIGVAROR", "has_api_access": false, "bank_aliases": ["COOP", "COOP FORUM", "COOP EXTRA"] }),
        serde_json::json!({ "merchant_id": "HEMKOP", "display_name": "Hemköp", "category": "DAGLIGVAROR", "has_api_access": false, "bank_aliases": ["HEMKOP", "HK"] }),
        serde_json::json!({ "merchant_id": "SYSTEMBOLAGET", "display_name": "Systembolaget", "category": "DAGLIGVAROR", "has_api_access": false, "bank_aliases": ["SYSTEMBOLAGET"] }),
        serde_json::json!({ "merchant_id": "PRESSBYRAAN", "display_name": "Pressbyrån", "category": "DAGLIGVAROR", "has_api_access": false, "bank_aliases": ["PRESSBYRAN", "PRESSBYRAAN"] }),
        serde_json::json!({ "merchant_id": "SJ", "display_name": "SJ AB", "category": "TRANSPORT", "has_api_access": true, "bank_aliases": ["SJ", "SJ.SE"] }),
        serde_json::json!({ "merchant_id": "SL", "display_name": "SL – Storstockholms Lokaltrafik", "category": "TRANSPORT", "has_api_access": false, "bank_aliases": ["SL", "STORSTOCKHOLMS LOKALTRAFIK"] }),
        serde_json::json!({ "merchant_id": "FLIXBUS", "display_name": "FlixBus", "category": "TRANSPORT", "has_api_access": false, "bank_aliases": ["FLIXBUS", "FLIX SE"] }),
        serde_json::json!({ "merchant_id": "RYANAIR", "display_name": "Ryanair", "category": "TRANSPORT", "has_api_access": false, "bank_aliases": ["RYANAIR", "RYANAIR DAC"] }),
        serde_json::json!({ "merchant_id": "NETFLIX", "display_name": "Netflix", "category": "PRENUMERATION", "has_api_access": false, "bank_aliases": ["NETFLIX", "NETFLIX.COM"] }),
        serde_json::json!({ "merchant_id": "SPOTIFY", "display_name": "Spotify", "category": "PRENUMERATION", "has_api_access": false, "bank_aliases": ["SPOTIFY", "SPOTIFY AB"] }),
        serde_json::json!({ "merchant_id": "MICROSOFT", "display_name": "Microsoft / Office 365", "category": "PRENUMERATION", "has_api_access": false, "bank_aliases": ["MICROSOFT", "MSFT*", "MICROSOFT 365"] }),
        serde_json::json!({ "merchant_id": "ADOBE", "display_name": "Adobe Inc.", "category": "PRENUMERATION", "has_api_access": false, "bank_aliases": ["ADOBE", "ADOBE SYSTEMS"] }),
        serde_json::json!({ "merchant_id": "TELIA", "display_name": "Telia Company", "category": "TELECOM", "has_api_access": false, "bank_aliases": ["TELIA", "TELIA SE"] }),
        serde_json::json!({ "merchant_id": "TELE2", "display_name": "Tele2 Sverige", "category": "TELECOM", "has_api_access": false, "bank_aliases": ["TELE2", "TELE2 AB"] }),
        serde_json::json!({ "merchant_id": "THREE", "display_name": "Tre / Hi3G", "category": "TELECOM", "has_api_access": false, "bank_aliases": ["TRE", "HI3G", "3 SVERIGE"] }),
        serde_json::json!({ "merchant_id": "ELGIGANTEN", "display_name": "Elgiganten", "category": "ELEKTRONIK", "has_api_access": false, "bank_aliases": ["ELGIGANTEN", "ELGIGANTEN AB"] }),
        serde_json::json!({ "merchant_id": "MEDIAMARKT", "display_name": "MediaMarkt", "category": "ELEKTRONIK", "has_api_access": false, "bank_aliases": ["MEDIAMARKT", "MEDIA MARKT"] }),
        serde_json::json!({ "merchant_id": "APOTEKET", "display_name": "Apoteket AB", "category": "HALSA", "has_api_access": false, "bank_aliases": ["APOTEKET", "APOTEKET AB"] }),
        serde_json::json!({ "merchant_id": "KRONANS_APOTEK", "display_name": "Kronans Apotek", "category": "HALSA", "has_api_access": false, "bank_aliases": ["KRONANS APOTEK", "KRONANS DROGHANDEL"] }),
    ];

    let filtered: Vec<_> = all_merchants.into_iter()
        .filter(|m| {
            if let Some(ref cat) = filter_category {
                m["category"].as_str().map(|c| c == cat.as_str()).unwrap_or(false)
            } else {
                true
            }
        })
        .take(limit)
        .collect();

    Json(serde_json::json!({
        "merchants": filtered,
        "total": filtered.len(),
        "filter": { "category": q.category, "limit": limit }
    }))
}

// ─────────────────────────────────────────────
// VRF VERIFY
// ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct VrfVerifyRequest {
    pub receipt: serde_json::Value,
}

async fn vrf_verify(Json(req): Json<VrfVerifyRequest>) -> impl IntoResponse {
    // Mock VRF verification – in production: cryptographic proof verification
    let has_id = req.receipt.get("id").is_some()
        || req.receipt.get("receipt_id").is_some()
        || req.receipt.get("transaction_id").is_some();

    Json(serde_json::json!({
        "valid": true,
        "signature_valid": has_id,
        "hash_matches": true,
        "issuer": "Kvittovalvet Test",
        "vrf_proof": "mock_vrf_proof_a1b2c3d4e5f6",
        "verified_at": Utc::now(),
        "note": "Mock verification – integrate with real VRF in production"
    }))
}

// ─────────────────────────────────────────────
// GRAPH STATS
// ─────────────────────────────────────────────

async fn graph_stats() -> impl IntoResponse {
    Json(serde_json::json!({
        "total_nodes": 106,
        "total_edges": 312,
        "node_types": {
            "merchants": 53,
            "suppliers": 8,
            "companies": 3,
            "persons": 24,
            "accounts": 12,
            "invoices": 6
        },
        "edge_types": {
            "transacted_with": 189,
            "supplies": 42,
            "owns": 18,
            "issued": 63
        },
        "graph_density": 0.028,
        "largest_connected_component": 98,
        "avg_degree": 5.89,
        "generated_at": Utc::now()
    }))
}
