use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

pub mod imap_inbox;
pub mod receipt_parser;

#[async_trait]
pub trait MailInbox: Send + Sync {
    fn provider(&self) -> &str;
    async fn scan_for_receipts(&self, since_days: u32) -> Result<Vec<ParsedReceiptMail>>;
    async fn mark_processed(&self, uid: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ParsedReceiptMail {
    pub uid: String,
    pub from: String,
    pub subject: String,
    pub date: DateTime<Utc>,
    pub merchant_guess: Option<String>,
    pub amount_guess: Option<Decimal>,
    pub attachments: Vec<MailAttachment>,
    pub body_text: String,
}

#[derive(Debug, Clone)]
pub struct MailAttachment {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}
