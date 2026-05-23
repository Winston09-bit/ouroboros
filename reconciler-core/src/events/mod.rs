use async_nats::Client as NatsClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::models::{Invoice, Transaction};
use crate::ai::confidence::Anomaly;

// ─────────────────────────────────────────────
// FINANCIAL EVENTS — the nervous system
// Every significant action creates an event.
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum FinancialEvent {
    InvoiceReceived(Invoice),
    PaymentMatched {
        transaction_id: Uuid,
        invoice_id: Uuid,
        confidence: f64,
        auto_booked: bool,
    },
    BankSyncCompleted {
        provider: String,
        account_id: String,
        count: usize,
        unmatched: usize,
    },
    AnomalyDetected(Anomaly),
    VatMismatchFound {
        transaction_id: Uuid,
        expected: Decimal,
        actual: Decimal,
        jurisdiction: String,
    },
    AuditExportGenerated {
        audit_id: String,
        company_id: Uuid,
        period: String,
        record_count: usize,
    },
    ReceiptRecoveryStarted {
        transaction_id: Uuid,
        merchant: String,
        amount: Decimal,
    },
    ReceiptRecovered {
        transaction_id: Uuid,
        document_id: Uuid,
        source: String,
        confidence: f64,
    },
    EscalationRequired {
        entity_id: Uuid,
        reason: String,
        assigned_to: Option<String>,
    },
    LedgerEntryCreated {
        voucher_id: Uuid,
        provider: String,
        amount: Decimal,
        confidence: f64,
    },
}

impl FinancialEvent {
    pub fn subject(&self) -> &str {
        match self {
            Self::InvoiceReceived(_) => "finance.invoice.received",
            Self::PaymentMatched { .. } => "finance.payment.matched",
            Self::BankSyncCompleted { .. } => "finance.bank.sync_completed",
            Self::AnomalyDetected(_) => "finance.anomaly.detected",
            Self::VatMismatchFound { .. } => "finance.vat.mismatch",
            Self::AuditExportGenerated { .. } => "finance.audit.export_generated",
            Self::ReceiptRecoveryStarted { .. } => "finance.receipt.recovery_started",
            Self::ReceiptRecovered { .. } => "finance.receipt.recovered",
            Self::EscalationRequired { .. } => "finance.escalation.required",
            Self::LedgerEntryCreated { .. } => "finance.ledger.entry_created",
        }
    }
}

// ─────────────────────────────────────────────
// EVENT BUS
// ─────────────────────────────────────────────
pub struct EventBus {
    client: NatsClient,
}

impl EventBus {
    pub async fn connect(nats_url: &str) -> Result<Self> {
        let client = async_nats::connect(nats_url).await?;
        Ok(Self { client })
    }

    pub async fn publish(&self, event: &FinancialEvent) -> Result<()> {
        let subject = event.subject();
        let payload = serde_json::to_vec(event)?;
        self.client.publish(subject, payload.into()).await?;
        tracing::info!("Published event: {}", subject);
        Ok(())
    }

    pub async fn subscribe(
        &self,
        subject: &str,
    ) -> Result<async_nats::Subscriber> {
        Ok(self.client.subscribe(subject).await?)
    }

    pub async fn subscribe_all(&self) -> Result<async_nats::Subscriber> {
        Ok(self.client.subscribe("finance.>").await?)
    }
}

// ─────────────────────────────────────────────
// EVENT HANDLER TRAIT
// ─────────────────────────────────────────────
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    fn handles(&self) -> Vec<&str>;
    async fn handle(&self, event: FinancialEvent) -> Result<()>;
}
