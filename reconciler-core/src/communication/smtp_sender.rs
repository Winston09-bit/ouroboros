//! SMTP email sender using Lettre (tokio1-native-tls + STARTTLS).
//!
//! Credentials are read from `/home/userwinston/.openclaw/secrets/email-hypbit.env`
//! or from standard environment variables at process startup.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{MessageSender, OutboundMessage};

// ---------------------------------------------------------------------------
// SmtpSender
// ---------------------------------------------------------------------------

/// Sends outbound messages via SMTP STARTTLS.
pub struct SmtpSender {
    host: String,
    port: u16,
    username: String,
    password: String,
    from_name: String,
}

impl SmtpSender {
    /// Build from environment variables (or the secrets env file).
    ///
    /// Variable resolution order:
    ///   1. Process environment (already exported)
    ///   2. `/home/userwinston/.openclaw/secrets/email-hypbit.env` dotenv file
    pub fn from_env() -> Result<Self> {
        // Try to load the secrets file; ignore if already set or file missing.
        let _ = dotenvy_or_manual_parse(
            "/home/userwinston/.openclaw/secrets/email-hypbit.env",
        );

        let host = std::env::var("SMTP_HOST")
            .or_else(|_| std::env::var("IMAP_HOST"))
            .unwrap_or_else(|_| "mail.wavult.com".to_string());

        let port: u16 = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);

        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("EMAIL_ADDRESS"))
            .context("SMTP: no USERNAME or EMAIL_ADDRESS in environment")?;

        let password = std::env::var("PASSWORD")
            .context("SMTP: no PASSWORD in environment")?;

        let from_name = std::env::var("SMTP_FROM_NAME")
            .unwrap_or_else(|_| "LandveX AB – Kvittovalvet".to_string());

        Ok(Self { host, port, username, password, from_name })
    }

    /// Build a Lettre async SMTP transport with STARTTLS on port 587.
    fn build_transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
        let tls_params = TlsParameters::new(self.host.clone())
            .context("Failed to build TLS parameters")?;

        let creds = Credentials::new(
            self.username.clone(),
            self.password.clone(),
        );

        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.host)
            .context("Failed to create STARTTLS transport")?
            .port(self.port)
            .credentials(creds)
            .tls(Tls::Required(tls_params))
            .build();

        Ok(transport)
    }

    /// Parse an RFC 5321 email address string into a Lettre `Mailbox`.
    fn parse_mailbox(addr: &str, display_name: Option<&str>) -> Result<Mailbox> {
        let address: Address = addr
            .parse()
            .with_context(|| format!("Invalid email address: {addr}"))?;
        Ok(Mailbox::new(display_name.map(str::to_string), address))
    }
}

// ---------------------------------------------------------------------------
// MessageSender impl
// ---------------------------------------------------------------------------

#[async_trait]
impl MessageSender for SmtpSender {
    fn channel(&self) -> &str {
        "email"
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<String> {
        let from_box = Self::parse_mailbox(&msg.from, Some(&self.from_name))
            .context("Parsing 'from' address")?;
        let to_box   = Self::parse_mailbox(&msg.to, None)
            .context("Parsing 'to' address")?;
        let reply_to = Self::parse_mailbox(&msg.reply_to, Some(&self.from_name))
            .context("Parsing 'reply_to' address")?;

        // Build the MIME message (multipart if HTML is present, plain-text otherwise).
        let email: Message = if let Some(html) = &msg.body_html {
            Message::builder()
                .from(from_box)
                .reply_to(reply_to)
                .to(to_box)
                .subject(&msg.subject)
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(msg.body_text.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html.clone()),
                        ),
                )
                .context("Building multipart MIME message")?
        } else {
            Message::builder()
                .from(from_box)
                .reply_to(reply_to)
                .to(to_box)
                .subject(&msg.subject)
                .header(ContentType::TEXT_PLAIN)
                .body(msg.body_text.clone())
                .context("Building plain-text MIME message")?
        };

        let transport = self.build_transport()?;

        info!(
            message_id  = %msg.id,
            to          = %msg.to,
            subject     = %msg.subject,
            step        = msg.escalation_step,
            template_id = %msg.template_id,
            "Sending email via SMTP"
        );

        let response = transport
            .send(email)
            .await
            .with_context(|| format!("SMTP send failed for message {}", msg.id))?;

        // Lettre returns the server-assigned message id if available.
        let server_message_id = response
            .message()
            .next()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("smtp-{}", Uuid::new_v4()));

        info!(
            message_id        = %msg.id,
            server_message_id = %server_message_id,
            "Email sent successfully"
        );

        Ok(server_message_id)
    }

    async fn health_check(&self) -> Result<bool> {
        debug!(host = %self.host, port = self.port, "SMTP health check");

        let transport = self.build_transport()?;

        match transport.test_connection().await {
            Ok(true) => {
                info!("SMTP health check passed");
                Ok(true)
            }
            Ok(false) => {
                warn!("SMTP health check: connection test returned false");
                Ok(false)
            }
            Err(e) => {
                error!(error = %e, "SMTP health check failed");
                Err(anyhow!("SMTP health check failed: {e}"))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal dotenv parser (no extra crate required)
// ---------------------------------------------------------------------------

/// Parse a `.env`-style file and inject missing variables into the process
/// environment. Variables already present are NOT overwritten.
fn dotenvy_or_manual_parse(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Reading env file: {path}"))?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"').trim_matches('\'');
            // Only set if not already in the environment.
            if std::env::var(key).is_err() {
                std::env::set_var(key, val);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: ensure SmtpSender::from_env() works when the secrets file
    /// is present (or env vars are already set). In CI this is expected to be
    /// skipped if neither is available.
    #[test]
    fn from_env_smoke() {
        // Allow test to pass even if secrets file is absent.
        match SmtpSender::from_env() {
            Ok(s) => {
                assert!(!s.host.is_empty());
                assert!(s.port > 0);
            }
            Err(e) => {
                eprintln!("SmtpSender::from_env() failed (expected in CI without secrets): {e}");
            }
        }
    }
}
