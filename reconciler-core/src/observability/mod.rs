use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{error, info, instrument, span, warn, Level, Span};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ReconcilerTracer
// ---------------------------------------------------------------------------

pub struct ReconcilerTracer;

impl ReconcilerTracer {
    /// Initialize OpenTelemetry tracing subscriber.
    ///
    /// Reads `OTEL_EXPORTER_OTLP_ENDPOINT` from the environment (falls back to
    /// stdout-based pretty printer for local dev).  Call once at process start.
    pub fn init() {
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{fmt, EnvFilter};

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // Build the subscriber.  In a real deployment you would add an
        // OpenTelemetry layer here (opentelemetry-otlp crate).  We wire up the
        // fmt layer so the crate compiles without optional OTEL feature flags
        // while still being instrumented correctly.
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        info!(
            service.name = "reconciler",
            service.version = env!("CARGO_PKG_VERSION"),
            "Tracing subscriber initialized"
        );
    }

    /// Creates and enters a span that covers one full reconciliation workflow.
    ///
    /// The caller is responsible for keeping the returned `Span` alive for the
    /// duration of the workflow.
    pub fn trace_reconciliation(txn_id: Uuid) -> Span {
        let span = span!(
            Level::INFO,
            "reconciliation",
            txn.id = %txn_id,
            otel.kind = "internal",
        );
        info!(parent: &span, txn.id = %txn_id, "Reconciliation workflow started");
        span
    }

    /// Creates a span that covers a single AI decision.
    pub fn trace_ai_decision(decision: &str, confidence: f64) -> Span {
        let span = span!(
            Level::INFO,
            "ai_decision",
            decision = decision,
            confidence = confidence,
            otel.kind = "internal",
        );
        if confidence < 0.5 {
            warn!(
                parent: &span,
                decision = decision,
                confidence = confidence,
                "Low-confidence AI decision — manual review may be required"
            );
        } else {
            info!(
                parent: &span,
                decision = decision,
                confidence = confidence,
                "AI decision recorded"
            );
        }
        span
    }

    /// Creates a span for a single outbound connector call.
    pub fn trace_connector(provider: &str, operation: &str) -> Span {
        let span = span!(
            Level::INFO,
            "connector_call",
            connector.provider = provider,
            connector.operation = operation,
            otel.kind = "client",
        );
        info!(
            parent: &span,
            connector.provider = provider,
            connector.operation = operation,
            "Connector call started"
        );
        span
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metrics {
    pub transactions_processed: u64,
    pub invoices_matched: u64,
    pub auto_booked: u64,
    pub anomalies_detected: u64,
    pub receipts_recovered: u64,
    pub connector_errors: HashMap<String, u64>,
    /// Running mean of confidence scores (Welford online algorithm).
    pub avg_confidence_score: f64,
    /// Percentage of transactions that were auto-booked without human review.
    pub autonomy_percentage: f64,

    // Internal state for Welford mean
    #[serde(skip)]
    confidence_count: u64,
    #[serde(skip)]
    confidence_sum: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Metrics {
            transactions_processed: 0,
            invoices_matched: 0,
            auto_booked: 0,
            anomalies_detected: 0,
            receipts_recovered: 0,
            connector_errors: HashMap::new(),
            avg_confidence_score: 0.0,
            autonomy_percentage: 0.0,
            confidence_count: 0,
            confidence_sum: 0.0,
        }
    }

    /// Record one processed transaction.
    ///
    /// - `auto_booked`: whether it was booked without human intervention.
    /// - `confidence`: the AI confidence score in [0.0, 1.0].
    #[instrument(skip(self), fields(auto_booked, confidence))]
    pub fn record_transaction(&mut self, auto_booked: bool, confidence: f64) {
        self.transactions_processed += 1;

        if auto_booked {
            self.auto_booked += 1;
        }

        // Welford online mean
        self.confidence_count += 1;
        self.confidence_sum += confidence;
        self.avg_confidence_score = self.confidence_sum / self.confidence_count as f64;

        self.autonomy_percentage = self.autonomy_pct();

        info!(
            transactions_processed = self.transactions_processed,
            auto_booked = self.auto_booked,
            avg_confidence = self.avg_confidence_score,
            autonomy_pct = self.autonomy_percentage,
            "Transaction recorded"
        );
    }

    /// Record that an anomaly was detected.
    #[instrument(skip(self))]
    pub fn record_anomaly(&mut self) {
        self.anomalies_detected += 1;
        warn!(
            anomalies_detected = self.anomalies_detected,
            "Anomaly detected and recorded"
        );
    }

    /// Record an attempt to recover a missing receipt.
    #[instrument(skip(self), fields(success))]
    pub fn record_receipt_recovery(&mut self, success: bool) {
        if success {
            self.receipts_recovered += 1;
            info!(receipts_recovered = self.receipts_recovered, "Receipt recovered");
        } else {
            error!("Receipt recovery failed");
        }
    }

    /// Record a connector-level error for the named provider.
    #[instrument(skip(self), fields(provider))]
    pub fn record_connector_error(&mut self, provider: &str) {
        let counter = self.connector_errors.entry(provider.to_string()).or_insert(0);
        *counter += 1;
        error!(
            connector.provider = provider,
            connector.errors = *counter,
            "Connector error recorded"
        );
    }

    /// Compute the autonomy percentage.
    ///
    /// Returns `0.0` when no transactions have been processed yet.
    pub fn autonomy_pct(&self) -> f64 {
        if self.transactions_processed == 0 {
            return 0.0;
        }
        (self.auto_booked as f64 / self.transactions_processed as f64) * 100.0
    }

    /// Serialize the current snapshot to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "transactions_processed": self.transactions_processed,
            "invoices_matched": self.invoices_matched,
            "auto_booked": self.auto_booked,
            "anomalies_detected": self.anomalies_detected,
            "receipts_recovered": self.receipts_recovered,
            "connector_errors": self.connector_errors,
            "avg_confidence_score": self.avg_confidence_score,
            "autonomy_percentage": self.autonomy_percentage,
        })
    }
}

// ---------------------------------------------------------------------------
// FinancialDecisionLog
// ---------------------------------------------------------------------------

/// Thread-safe, append-only ledger of every financial decision made by the
/// reconciler or its AI agents.
#[derive(Debug, Default)]
pub struct FinancialDecisionLog {
    entries: Arc<Mutex<Vec<DecisionLogEntry>>>,
}

impl FinancialDecisionLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Append a new entry.  Panics if the internal mutex is poisoned.
    pub fn append(&self, entry: DecisionLogEntry) {
        let mut guard = self.entries.lock().expect("FinancialDecisionLog mutex poisoned");
        info!(
            decision_type = %entry.decision_type,
            entity_id = %entry.entity_id,
            confidence = entry.confidence,
            outcome = ?entry.outcome,
            agent = %entry.agent,
            "Financial decision logged"
        );
        guard.push(entry);
    }

    /// Returns a snapshot of all entries (cloned).
    pub fn snapshot(&self) -> Vec<DecisionLogEntry> {
        self.entries
            .lock()
            .expect("FinancialDecisionLog mutex poisoned")
            .clone()
    }

    /// Returns the total number of logged decisions.
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .expect("FinancialDecisionLog mutex poisoned")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns all entries where `outcome` matches the supplied predicate.
    pub fn filter<F>(&self, predicate: F) -> Vec<DecisionLogEntry>
    where
        F: Fn(&DecisionLogEntry) -> bool,
    {
        self.entries
            .lock()
            .expect("FinancialDecisionLog mutex poisoned")
            .iter()
            .filter(|e| predicate(e))
            .cloned()
            .collect()
    }

    /// Serialize the full log to a JSON array.
    pub fn to_json(&self) -> serde_json::Value {
        let entries = self.snapshot();
        serde_json::to_value(&entries).unwrap_or(serde_json::Value::Array(vec![]))
    }
}

// ---------------------------------------------------------------------------
// DecisionLogEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionLogEntry {
    pub timestamp: DateTime<Utc>,
    pub decision_type: String,
    pub entity_id: Uuid,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub outcome: DecisionOutcome,
    pub agent: String,
    pub rollback_id: Option<Uuid>,
}

impl DecisionLogEntry {
    pub fn new(
        decision_type: impl Into<String>,
        entity_id: Uuid,
        confidence: f64,
        reasons: Vec<String>,
        outcome: DecisionOutcome,
        agent: impl Into<String>,
    ) -> Self {
        DecisionLogEntry {
            timestamp: Utc::now(),
            decision_type: decision_type.into(),
            entity_id,
            confidence,
            reasons,
            outcome,
            agent: agent.into(),
            rollback_id: None,
        }
    }

    pub fn with_rollback(mut self, rollback_id: Uuid) -> Self {
        self.rollback_id = Some(rollback_id);
        self
    }
}

// ---------------------------------------------------------------------------
// DecisionOutcome
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOutcome {
    AutoApproved,
    ManualReview,
    Escalated,
    Rejected,
    RolledBack,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_autonomy_zero_when_no_transactions() {
        let m = Metrics::new();
        assert_eq!(m.autonomy_pct(), 0.0);
    }

    #[test]
    fn metrics_record_and_autonomy() {
        let mut m = Metrics::new();
        m.record_transaction(true, 0.9);
        m.record_transaction(false, 0.6);
        m.record_transaction(true, 0.8);
        assert_eq!(m.transactions_processed, 3);
        assert_eq!(m.auto_booked, 2);
        let pct = m.autonomy_pct();
        assert!((pct - 66.666_666).abs() < 0.01);
    }

    #[test]
    fn metrics_avg_confidence() {
        let mut m = Metrics::new();
        m.record_transaction(true, 0.8);
        m.record_transaction(false, 0.6);
        assert!((m.avg_confidence_score - 0.7).abs() < 1e-10);
    }

    #[test]
    fn metrics_to_json_fields() {
        let mut m = Metrics::new();
        m.record_transaction(true, 0.95);
        m.record_anomaly();
        let json = m.to_json();
        assert_eq!(json["transactions_processed"], 1);
        assert_eq!(json["anomalies_detected"], 1);
    }

    #[test]
    fn decision_log_append_and_snapshot() {
        let log = FinancialDecisionLog::new();
        let entry = DecisionLogEntry::new(
            "invoice_match",
            Uuid::new_v4(),
            0.92,
            vec!["vendor match".into(), "amount within tolerance".into()],
            DecisionOutcome::AutoApproved,
            "reconciler-agent-v1",
        );
        log.append(entry);
        assert_eq!(log.len(), 1);
        let snapshot = log.snapshot();
        assert_eq!(snapshot[0].outcome, DecisionOutcome::AutoApproved);
    }

    #[test]
    fn decision_log_filter_manual_review() {
        let log = FinancialDecisionLog::new();
        let id = Uuid::new_v4();
        log.append(DecisionLogEntry::new(
            "invoice_match",
            id,
            0.45,
            vec![],
            DecisionOutcome::ManualReview,
            "agent",
        ));
        log.append(DecisionLogEntry::new(
            "invoice_match",
            Uuid::new_v4(),
            0.99,
            vec![],
            DecisionOutcome::AutoApproved,
            "agent",
        ));
        let manual = log.filter(|e| e.outcome == DecisionOutcome::ManualReview);
        assert_eq!(manual.len(), 1);
        assert_eq!(manual[0].entity_id, id);
    }

    #[test]
    fn connector_error_accumulates() {
        let mut m = Metrics::new();
        m.record_connector_error("fortnox");
        m.record_connector_error("fortnox");
        m.record_connector_error("revolut");
        assert_eq!(*m.connector_errors.get("fortnox").unwrap(), 2);
        assert_eq!(*m.connector_errors.get("revolut").unwrap(), 1);
    }
}
