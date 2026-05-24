use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// Schemaläggare för automatiska uppföljningar
pub struct RetrySchedule {
    pub transaction_id: Uuid,
    pub scheduled_at: DateTime<Utc>,
    pub action: String,
    pub attempts: u8,
    pub max_attempts: u8,
}

impl RetrySchedule {
    pub fn new(transaction_id: Uuid, action: impl Into<String>) -> Self {
        Self {
            transaction_id,
            scheduled_at: Utc::now(),
            action: action.into(),
            attempts: 0,
            max_attempts: 3,
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.scheduled_at = Utc::now() + delay;
        self
    }

    pub fn is_due(&self) -> bool {
        Utc::now() >= self.scheduled_at
    }

    pub fn increment(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
    }

    pub fn is_exhausted(&self) -> bool {
        self.attempts >= self.max_attempts
    }
}
