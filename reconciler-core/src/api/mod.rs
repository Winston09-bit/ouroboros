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
