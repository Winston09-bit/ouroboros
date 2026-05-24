use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Timeline av ekonomiska händelser kring en transaktion
#[derive(Debug, Clone, Serialize)]
pub struct EconomicTimeline {
    pub anchor_transaction_id: Uuid,
    pub events: Vec<TimelineEvent>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

impl EconomicTimeline {
    /// Sorted events ascending by timestamp
    pub fn sorted_events(&self) -> Vec<&TimelineEvent> {
        let mut events: Vec<&TimelineEvent> = self.events.iter().collect();
        events.sort_by_key(|e| e.timestamp);
        events
    }

    /// All events of a given type
    pub fn events_of_type(&self, event_type: &str) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }

    /// True if all events are in "completed" status
    pub fn is_complete(&self) -> bool {
        !self.events.is_empty() && self.events.iter().all(|e| e.status == "completed")
    }

    /// Latest completed event timestamp
    pub fn last_completed_at(&self) -> Option<DateTime<Utc>> {
        self.events
            .iter()
            .filter(|e| e.status == "completed")
            .map(|e| e.timestamp)
            .max()
    }

    /// Returns events with "failed" status
    pub fn failures(&self) -> Vec<&TimelineEvent> {
        self.events.iter().filter(|e| e.status == "failed").collect()
    }
}

/// Known event types (as constants for convenience)
pub mod event_type {
    pub const PURCHASE_CREATED: &str = "purchase_created";
    pub const BANK_AUTHORIZED: &str = "bank_authorized";
    pub const INVOICE_SENT: &str = "invoice_sent";
    pub const VAT_VERIFIED: &str = "vat_verified";
    pub const RECEIPT_RETRIEVED: &str = "receipt_retrieved";
    pub const MATCHED: &str = "matched";
    pub const AUDITED: &str = "audited";
    pub const ARCHIVED: &str = "archived";
}

/// Known sources
pub mod source {
    pub const BANK: &str = "bank";
    pub const ERP: &str = "erp";
    pub const MERCHANT: &str = "merchant";
    pub const SYSTEM: &str = "system";
}

/// Known statuses
pub mod status {
    pub const COMPLETED: &str = "completed";
    pub const PENDING: &str = "pending";
    pub const FAILED: &str = "failed";
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    /// "purchase_created" | "bank_authorized" | "invoice_sent" | "vat_verified"
    /// | "receipt_retrieved" | "matched" | "audited" | "archived"
    pub event_type: String,
    pub description: String,
    /// "bank" | "erp" | "merchant" | "system"
    pub source: String,
    /// Länk till nod i grafen
    pub data_ref: Option<Uuid>,
    /// "completed" | "pending" | "failed"
    pub status: String,
}

impl TimelineEvent {
    pub fn new(
        timestamp: DateTime<Utc>,
        event_type: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            timestamp,
            event_type: event_type.into(),
            description: description.into(),
            source: source.into(),
            data_ref: None,
            status: status.into(),
        }
    }

    pub fn with_ref(mut self, data_ref: Uuid) -> Self {
        self.data_ref = Some(data_ref);
        self
    }

    pub fn is_completed(&self) -> bool {
        self.status == status::COMPLETED
    }

    pub fn is_pending(&self) -> bool {
        self.status == status::PENDING
    }

    pub fn is_failed(&self) -> bool {
        self.status == status::FAILED
    }
}

// ── Builder ─────────────────────────────────────────────────────────────────

pub struct TimelineBuilder;

impl TimelineBuilder {
    pub fn for_transaction(transaction_id: Uuid) -> TimelineBuilderState {
        TimelineBuilderState {
            transaction_id,
            events: Vec::new(),
        }
    }
}

pub struct TimelineBuilderState {
    transaction_id: Uuid,
    events: Vec<TimelineEvent>,
}

impl TimelineBuilderState {
    pub fn add_event(mut self, event: TimelineEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Convenience: add a completed event from bank at `now`
    pub fn bank_authorized(self, description: impl Into<String>) -> Self {
        self.add_event(TimelineEvent::new(
            Utc::now(),
            event_type::BANK_AUTHORIZED,
            description,
            source::BANK,
            status::COMPLETED,
        ))
    }

    pub fn invoice_sent(self, description: impl Into<String>, data_ref: Option<Uuid>) -> Self {
        let ev = TimelineEvent::new(
            Utc::now(),
            event_type::INVOICE_SENT,
            description,
            source::MERCHANT,
            status::COMPLETED,
        );
        let ev = if let Some(r) = data_ref {
            ev.with_ref(r)
        } else {
            ev
        };
        self.add_event(ev)
    }

    pub fn receipt_retrieved(self, description: impl Into<String>, data_ref: Option<Uuid>) -> Self {
        let ev = TimelineEvent::new(
            Utc::now(),
            event_type::RECEIPT_RETRIEVED,
            description,
            source::MERCHANT,
            status::COMPLETED,
        );
        let ev = if let Some(r) = data_ref {
            ev.with_ref(r)
        } else {
            ev
        };
        self.add_event(ev)
    }

    pub fn matched(self, description: impl Into<String>) -> Self {
        self.add_event(TimelineEvent::new(
            Utc::now(),
            event_type::MATCHED,
            description,
            source::SYSTEM,
            status::COMPLETED,
        ))
    }

    pub fn vat_verified(self, description: impl Into<String>) -> Self {
        self.add_event(TimelineEvent::new(
            Utc::now(),
            event_type::VAT_VERIFIED,
            description,
            source::SYSTEM,
            status::COMPLETED,
        ))
    }

    pub fn audited(self, description: impl Into<String>) -> Self {
        self.add_event(TimelineEvent::new(
            Utc::now(),
            event_type::AUDITED,
            description,
            source::ERP,
            status::COMPLETED,
        ))
    }

    pub fn archived(self, description: impl Into<String>) -> Self {
        self.add_event(TimelineEvent::new(
            Utc::now(),
            event_type::ARCHIVED,
            description,
            source::SYSTEM,
            status::COMPLETED,
        ))
    }

    pub fn build(self) -> EconomicTimeline {
        let (window_start, window_end) = if self.events.is_empty() {
            let now = Utc::now();
            (now, now)
        } else {
            let min = self
                .events
                .iter()
                .map(|e| e.timestamp)
                .min()
                .unwrap_or_else(Utc::now);
            let max = self
                .events
                .iter()
                .map(|e| e.timestamp)
                .max()
                .unwrap_or_else(Utc::now);
            (min, max)
        };

        EconomicTimeline {
            anchor_transaction_id: self.transaction_id,
            events: self.events,
            window_start,
            window_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let tx_id = Uuid::new_v4();
        let timeline = TimelineBuilder::for_transaction(tx_id)
            .bank_authorized("Authorized SEK 450.00")
            .receipt_retrieved("Receipt from ICA", None)
            .matched("Matched to invoice INV-001")
            .vat_verified("25% VAT consistent")
            .build();

        assert_eq!(timeline.anchor_transaction_id, tx_id);
        assert_eq!(timeline.events.len(), 4);
        assert!(timeline.is_complete());
        assert!(timeline.failures().is_empty());
    }

    #[test]
    fn test_events_of_type() {
        let tx_id = Uuid::new_v4();
        let timeline = TimelineBuilder::for_transaction(tx_id)
            .bank_authorized("Auth 1")
            .bank_authorized("Auth 2")
            .matched("Match 1")
            .build();

        assert_eq!(timeline.events_of_type(event_type::BANK_AUTHORIZED).len(), 2);
        assert_eq!(timeline.events_of_type(event_type::MATCHED).len(), 1);
        assert_eq!(timeline.events_of_type(event_type::ARCHIVED).len(), 0);
    }

    #[test]
    fn test_sorted_events() {
        let tx_id = Uuid::new_v4();
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::seconds(30);
        let t3 = t2 + chrono::Duration::seconds(30);

        let timeline = TimelineBuilder::for_transaction(tx_id)
            .add_event(TimelineEvent::new(
                t3,
                event_type::ARCHIVED,
                "Archived",
                source::SYSTEM,
                status::COMPLETED,
            ))
            .add_event(TimelineEvent::new(
                t1,
                event_type::PURCHASE_CREATED,
                "Created",
                source::BANK,
                status::COMPLETED,
            ))
            .add_event(TimelineEvent::new(
                t2,
                event_type::MATCHED,
                "Matched",
                source::SYSTEM,
                status::COMPLETED,
            ))
            .build();

        let sorted = timeline.sorted_events();
        assert_eq!(sorted[0].timestamp, t1);
        assert_eq!(sorted[1].timestamp, t2);
        assert_eq!(sorted[2].timestamp, t3);
    }

    #[test]
    fn test_window() {
        let tx_id = Uuid::new_v4();
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::hours(2);

        let timeline = TimelineBuilder::for_transaction(tx_id)
            .add_event(TimelineEvent::new(
                t1,
                event_type::BANK_AUTHORIZED,
                "Start",
                source::BANK,
                status::COMPLETED,
            ))
            .add_event(TimelineEvent::new(
                t2,
                event_type::ARCHIVED,
                "End",
                source::SYSTEM,
                status::COMPLETED,
            ))
            .build();

        assert_eq!(timeline.window_start, t1);
        assert_eq!(timeline.window_end, t2);
    }

    #[test]
    fn test_failures() {
        let tx_id = Uuid::new_v4();
        let timeline = TimelineBuilder::for_transaction(tx_id)
            .add_event(TimelineEvent::new(
                Utc::now(),
                event_type::RECEIPT_RETRIEVED,
                "Retrieval failed",
                source::MERCHANT,
                status::FAILED,
            ))
            .build();

        assert!(!timeline.is_complete());
        assert_eq!(timeline.failures().len(), 1);
    }
}
