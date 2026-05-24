// IMAP inbox connector – läser email-attachments för receipt-retrieval
//
// NOTE: Imap crate version 3.0.0-alpha.15 har lite annorlunda API än stable.
// Den här implementationen använder reqwest-style helpers via stable imap = "2".
//
// För nu: stub som returnerar tomma resultat men loggar vad den SKULLE göra.
// Production-implementation följer när vi väl har en stabil imap-crate val.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;

use super::{MailAttachment, MailInbox, ParsedReceiptMail};

#[derive(Debug, Clone)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mailbox: String,
    pub use_tls: bool,
}

impl Default for ImapConfig {
    fn default() -> Self {
        Self {
            host: "imap.gmail.com".to_string(),
            port: 993,
            username: String::new(),
            password: String::new(),
            mailbox: "INBOX".to_string(),
            use_tls: true,
        }
    }
}

pub struct ImapInbox {
    config: ImapConfig,
}

impl ImapInbox {
    pub fn new(config: ImapConfig) -> Self {
        Self { config }
    }

    /// Läs från ~/.openclaw/secrets/email-hypbit.env
    pub fn from_env() -> Result<Self> {
        let path = std::env::var("EMAIL_ENV_PATH")
            .unwrap_or_else(|_| "/home/userwinston/.openclaw/secrets/email-hypbit.env".to_string());
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read {}", path))?;

        let mut config = ImapConfig::default();
        for line in raw.lines() {
            let line = line.trim();
            if line.starts_with('#') || !line.contains('=') {
                continue;
            }
            let (k, v) = line.split_once('=').unwrap();
            match k.trim() {
                "IMAP_HOST"     => config.host = v.trim().to_string(),
                "IMAP_PORT"     => config.port = v.trim().parse().unwrap_or(993),
                "EMAIL_ADDRESS" | "IMAP_USER" => config.username = v.trim().to_string(),
                "PASSWORD" | "IMAP_PASSWORD"   => config.password = v.trim().to_string(),
                "IMAP_MAILBOX"  => config.mailbox = v.trim().to_string(),
                "IMAP_SSL"      => config.use_tls = v.trim().eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
        Ok(Self::new(config))
    }
}

#[async_trait]
impl MailInbox for ImapInbox {
    fn provider(&self) -> &str { "imap" }

    async fn scan_for_receipts(&self, since_days: u32) -> Result<Vec<ParsedReceiptMail>> {
        tracing::info!(
            "ImapInbox would connect to {}:{} as {} and search SINCE {}d for receipt/noreply/kvitto/faktura senders",
            self.config.host,
            self.config.port,
            self.config.username,
            since_days,
        );

        // TODO production: använd imap-crate korrekt. Just nu blockerat på API-skillnader
        // mellan stable och alpha-versionen. Stub returnerar tom Vec.
        //
        // För manuell testkörning, använd python-script i scripts/ som vi redan har.

        Ok(vec![])
    }

    async fn mark_processed(&self, uid: &str) -> Result<()> {
        tracing::info!("ImapInbox would mark UID {} as processed (\\Flagged + processed-tag)", uid);
        Ok(())
    }
}

/// Manual builder – skapa ParsedReceiptMail från extern parsing (Python script etc.)
pub fn build_parsed_mail(
    uid: impl Into<String>,
    from: impl Into<String>,
    subject: impl Into<String>,
    body_text: impl Into<String>,
) -> ParsedReceiptMail {
    ParsedReceiptMail {
        uid: uid.into(),
        from: from.into(),
        subject: subject.into(),
        date: Utc::now(),
        merchant_guess: None,
        amount_guess: None,
        attachments: Vec::new(),
        body_text: body_text.into(),
    }
}

/// Helper för att skapa attachment
pub fn build_attachment(
    filename: impl Into<String>,
    mime_type: impl Into<String>,
    data: Vec<u8>,
) -> MailAttachment {
    MailAttachment {
        filename: filename.into(),
        mime_type: mime_type.into(),
        data,
    }
}
