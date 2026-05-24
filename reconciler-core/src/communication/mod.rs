use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub id: Uuid,
    pub to: String,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub from: String,
    pub reply_to: String,
    pub escalation_step: u8,
    pub transaction_id: Uuid,
    pub merchant_id: String,
    pub template_id: String,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub opened_at: Option<DateTime<Utc>>,
    pub replied_at: Option<DateTime<Utc>>,
    pub status: MessageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Opened,
    Replied,
    Bounced,
    Failed,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Pending   => write!(f, "pending"),
            MessageStatus::Sent      => write!(f, "sent"),
            MessageStatus::Delivered => write!(f, "delivered"),
            MessageStatus::Opened    => write!(f, "opened"),
            MessageStatus::Replied   => write!(f, "replied"),
            MessageStatus::Bounced   => write!(f, "bounced"),
            MessageStatus::Failed    => write!(f, "failed"),
        }
    }
}

#[async_trait]
pub trait MessageSender: Send + Sync {
    /// Identifier for the transport channel, e.g. "email" or "sms".
    fn channel(&self) -> &str;

    /// Send the message and return the provider-assigned message id.
    async fn send(&self, msg: &OutboundMessage) -> Result<String>;

    /// Lightweight connectivity / auth check.
    async fn health_check(&self) -> Result<bool>;
}

pub mod email_templates;
pub mod smtp_sender;
pub mod response_parser;
