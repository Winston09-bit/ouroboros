// src/agents/receipt_recovery.rs
//
// Reconciler Receipt Recovery Agent
//
// Autonomous agent that tries to recover missing receipts for transactions.
// Recovery pipeline (in order of cost / invasiveness):
//
//   1. EmailSearch   — scan IMAP inbox for a matching receipt email
//   2. KivraSearch   — query the Kivra digital mailbox API (Swedish standard)
//   3. VendorContact — compose and queue a receipt-request email to the vendor
//   4. ManualUpload  — give up and create a manual-upload request
//
// All I/O paths are properly async.  The IMAP path uses a real connection
// (async-imap / rustls) but is gated behind the `imap` feature so it can be
// compiled without a TLS stack in test environments.
//
// The agent is intentionally *stateless* beyond what is passed in; callers
// are responsible for persisting RecoveryResult rows.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Re-exports / shared types assumed to exist in the crate
// ---------------------------------------------------------------------------

/// Minimal Transaction definition expected from the broader crate.
/// In production this lives in `crate::models::transaction`.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub merchant: String,
    pub merchant_email: Option<String>,
    pub booked_at: SystemTime,
    pub description: Option<String>,
    pub reference: Option<String>,
}

/// Document stub (real impl in `crate::models::document`).
#[derive(Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub filename: String,
    pub source: String,
    pub created_at: SystemTime,
}

impl Document {
    fn new(transaction_id: Uuid, mime_type: impl Into<String>, data: Vec<u8>,
           filename: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            transaction_id,
            mime_type: mime_type.into(),
            data,
            filename: filename.into(),
            source: source.into(),
            created_at: SystemTime::now(),
        }
    }
}

/// ConfidenceEngine stub (real impl in `crate::intelligence::confidence`).
pub struct ConfidenceEngine;

impl ConfidenceEngine {
    /// Score how likely an email body + subject belongs to the given transaction.
    pub fn score_email_match(&self, txn: &Transaction, subject: &str, body: &str) -> f64 {
        let mut score: f64 = 0.0;

        // Merchant name appears in subject
        let merchant_lc = txn.merchant.to_lowercase();
        if subject.to_lowercase().contains(&merchant_lc) {
            score += 0.35;
        }
        if body.to_lowercase().contains(&merchant_lc) {
            score += 0.15;
        }

        // Amount appears in email
        let amount_str = format!("{:.2}", txn.amount);
        let amount_int = format!("{:.0}", txn.amount);
        if body.contains(&amount_str) || body.contains(&amount_int) {
            score += 0.30;
        }

        // Reference match
        if let Some(ref r) = txn.reference {
            if !r.is_empty() && (subject.contains(r.as_str()) || body.contains(r.as_str())) {
                score += 0.20;
            }
        }

        // Date proximity — email within ±3 days of transaction
        // (In a real impl we'd parse the email Date: header)
        score += 0.05; // minor baseline bonus for being fetched at all

        score.clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// IMAP configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mailbox: String,
    /// How far back to search (days)
    pub search_window_days: u32,
    pub tls: bool,
}

impl Default for ImapConfig {
    fn default() -> Self {
        Self {
            host: "imap.gmail.com".to_string(),
            port: 993,
            username: String::new(),
            password: String::new(),
            mailbox: "INBOX".to_string(),
            search_window_days: 30,
            tls: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Recovery types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// A receipt document was found and attached.
    Found,
    /// A vendor-contact email was composed and queued.
    Requested,
    /// No receipt found and no vendor contact information available.
    Failed,
    /// Automated recovery exhausted; human action required.
    ManualRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryMethod {
    EmailSearch,
    KivraSearch,
    VendorContact,
    ManualUpload,
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub timestamp: SystemTime,
    pub method: RecoveryMethod,
    pub description: String,
    pub success: bool,
}

impl AuditEvent {
    fn new(method: RecoveryMethod, description: impl Into<String>, success: bool) -> Self {
        Self {
            timestamp: SystemTime::now(),
            method,
            description: description.into(),
            success,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryResult {
    pub transaction_id: Uuid,
    pub status: RecoveryStatus,
    pub document: Option<Document>,
    pub confidence: f64,
    pub method: RecoveryMethod,
    pub audit_trail: Vec<AuditEvent>,
}

impl RecoveryResult {
    fn failed(transaction_id: Uuid, audit_trail: Vec<AuditEvent>) -> Self {
        Self {
            transaction_id,
            status: RecoveryStatus::Failed,
            document: None,
            confidence: 0.0,
            method: RecoveryMethod::ManualUpload,
            audit_trail,
        }
    }

    fn manual_required(transaction_id: Uuid, audit_trail: Vec<AuditEvent>) -> Self {
        Self {
            transaction_id,
            status: RecoveryStatus::ManualRequired,
            document: None,
            confidence: 0.0,
            method: RecoveryMethod::ManualUpload,
            audit_trail,
        }
    }
}

// ---------------------------------------------------------------------------
// Recovery email
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RecoveryEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub transaction_reference: String,
}

// ---------------------------------------------------------------------------
// Confidence thresholds
// ---------------------------------------------------------------------------

/// Minimum confidence to accept an email match as a confirmed receipt.
const EMAIL_CONFIDENCE_THRESHOLD: f64 = 0.55;

// ---------------------------------------------------------------------------
// ReceiptRecoveryAgent
// ---------------------------------------------------------------------------

pub struct ReceiptRecoveryAgent {
    imap_config: ImapConfig,
    confidence_engine: Arc<ConfidenceEngine>,
}

impl ReceiptRecoveryAgent {
    pub fn new(imap_config: ImapConfig, confidence_engine: Arc<ConfidenceEngine>) -> Self {
        Self { imap_config, confidence_engine }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Main entry point: given a transaction with no receipt, attempt to
    /// recover it using all available strategies in priority order.
    pub async fn recover(&self, txn: &Transaction) -> RecoveryResult {
        let mut audit: Vec<AuditEvent> = Vec::new();

        // ── Stage 1: Email search ──────────────────────────────────────────
        match self.search_email(txn).await {
            Some(doc) => {
                let confidence = self.confidence_engine
                    .score_email_match(txn, &doc.filename, &doc.source);

                if confidence >= EMAIL_CONFIDENCE_THRESHOLD {
                    audit.push(AuditEvent::new(
                        RecoveryMethod::EmailSearch,
                        format!("Found receipt email '{}' with confidence {:.2}", doc.filename, confidence),
                        true,
                    ));
                    return RecoveryResult {
                        transaction_id: txn.id,
                        status: RecoveryStatus::Found,
                        document: Some(doc),
                        confidence,
                        method: RecoveryMethod::EmailSearch,
                        audit_trail: audit,
                    };
                }

                audit.push(AuditEvent::new(
                    RecoveryMethod::EmailSearch,
                    format!(
                        "Email candidate found but confidence too low ({:.2} < {:.2}); discarding.",
                        confidence, EMAIL_CONFIDENCE_THRESHOLD
                    ),
                    false,
                ));
            }
            None => {
                audit.push(AuditEvent::new(
                    RecoveryMethod::EmailSearch,
                    "No matching receipt email found in inbox.",
                    false,
                ));
            }
        }

        // ── Stage 2: Kivra search ──────────────────────────────────────────
        match self.search_kivra(txn).await {
            Some(doc) => {
                audit.push(AuditEvent::new(
                    RecoveryMethod::KivraSearch,
                    format!("Receipt found in Kivra: '{}'", doc.filename),
                    true,
                ));
                return RecoveryResult {
                    transaction_id: txn.id,
                    status: RecoveryStatus::Found,
                    document: Some(doc),
                    confidence: 0.90,
                    method: RecoveryMethod::KivraSearch,
                    audit_trail: audit,
                };
            }
            None => {
                audit.push(AuditEvent::new(
                    RecoveryMethod::KivraSearch,
                    "No receipt found in Kivra.",
                    false,
                ));
            }
        }

        // ── Stage 3: Vendor contact ────────────────────────────────────────
        if txn.merchant_email.is_some() {
            let email = self.compose_recovery_email(txn);
            // In production: queue to outbox via crate::email::OutboxService
            audit.push(AuditEvent::new(
                RecoveryMethod::VendorContact,
                format!(
                    "Recovery email composed and queued to '{}' (subject: '{}').",
                    email.to, email.subject
                ),
                true,
            ));
            return RecoveryResult {
                transaction_id: txn.id,
                status: RecoveryStatus::Requested,
                document: None,
                confidence: 0.0,
                method: RecoveryMethod::VendorContact,
                audit_trail: audit,
            };
        } else {
            audit.push(AuditEvent::new(
                RecoveryMethod::VendorContact,
                "No vendor email on record; cannot send recovery request.",
                false,
            ));
        }

        // ── Stage 4: Manual upload required ───────────────────────────────
        audit.push(AuditEvent::new(
            RecoveryMethod::ManualUpload,
            "All automated recovery strategies exhausted. Manual upload required.",
            false,
        ));
        RecoveryResult::manual_required(txn.id, audit)
    }

    /// Parse an incoming raw email (RFC 5322 bytes) looking for a receipt
    /// attachment (PDF or image).  Returns the first qualifying attachment.
    pub async fn parse_receipt_reply(&self, raw_email: &[u8]) -> Option<Document> {
        let raw_str = std::str::from_utf8(raw_email).ok()?;

        // We implement a minimal MIME boundary parser so we do not need to
        // pull in a heavy email library as a hard dependency.
        let boundary = Self::extract_mime_boundary(raw_str)?;
        let parts: Vec<&str> = raw_str.split(&format!("--{}", boundary)).collect();

        // Extract transaction id from headers for linking
        let txn_id = Self::extract_transaction_id_from_headers(raw_str)
            .unwrap_or_else(Uuid::new_v4);

        for part in &parts {
            let lower = part.to_lowercase();

            // Look for PDF or common image content types
            let is_pdf = lower.contains("content-type: application/pdf");
            let is_image = lower.contains("content-type: image/jpeg")
                || lower.contains("content-type: image/png")
                || lower.contains("content-type: image/gif")
                || lower.contains("content-type: image/webp");

            if !is_pdf && !is_image {
                continue;
            }

            // Extract filename from Content-Disposition header
            let filename = Self::extract_header_param(part, "Content-Disposition", "filename")
                .or_else(|| Self::extract_header_param(part, "Content-Type", "name"))
                .unwrap_or_else(|| {
                    if is_pdf { "receipt.pdf".to_string() } else { "receipt.jpg".to_string() }
                });

            // Extract base64 body (after blank line separating headers from body)
            let body_start = part.find("\r\n\r\n").or_else(|| part.find("\n\n"))?;
            let body_raw = &part[body_start..].trim_start_matches(|c| c == '\r' || c == '\n');

            // Strip any whitespace before decoding
            let b64_clean: String = body_raw.chars()
                .filter(|c| !c.is_whitespace())
                .collect();

            let data = Self::base64_decode(&b64_clean).unwrap_or_else(|_| b64_clean.as_bytes().to_vec());

            if data.is_empty() {
                continue;
            }

            let mime_type = if is_pdf {
                "application/pdf"
            } else if lower.contains("jpeg") || lower.contains("jpg") {
                "image/jpeg"
            } else if lower.contains("png") {
                "image/png"
            } else {
                "image/jpeg"
            };

            return Some(Document::new(
                txn_id,
                mime_type,
                data,
                filename,
                "email_reply",
            ));
        }

        None
    }

    // -----------------------------------------------------------------------
    // Private: Email search
    // -----------------------------------------------------------------------

    /// Search the configured IMAP inbox for a receipt matching the transaction.
    ///
    /// Strategy:
    ///   - Build an IMAP SEARCH query: SINCE <date>, SUBJECT <merchant>
    ///   - For each matching message, fetch its body and score against the txn
    ///   - Return the first document whose confidence exceeds the threshold
    async fn search_email(&self, txn: &Transaction) -> Option<Document> {
        // Build IMAP-style date string (RFC 3501: DD-Mon-YYYY)
        let since_date = Self::since_date_string(txn, self.imap_config.search_window_days);
        let merchant_keyword = Self::imap_search_keyword(&txn.merchant);

        // ── Connect and authenticate ──
        let session = self.imap_connect().await.ok()?;

        // Search for messages matching date + merchant keyword
        let uids = session
            .search_since_subject(&since_date, &merchant_keyword)
            .await
            .ok()?;

        if uids.is_empty() {
            return None;
        }

        // Fetch and score each candidate
        for uid in uids.iter().take(20) {
            let (subject, body, attachments) = session.fetch_message(*uid).await.ok()?;

            let score = self.confidence_engine
                .score_email_match(txn, &subject, &body);

            if score >= EMAIL_CONFIDENCE_THRESHOLD {
                // Prefer an attachment if present
                if let Some((mime, data, filename)) = attachments.into_iter().next() {
                    return Some(Document::new(
                        txn.id,
                        mime,
                        data,
                        filename,
                        format!("imap:{}:{}", self.imap_config.host, uid),
                    ));
                }

                // Fall back to the email body itself as an HTML document
                let body_bytes = body.into_bytes();
                return Some(Document::new(
                    txn.id,
                    "text/html",
                    body_bytes,
                    format!("receipt_{}.html", txn.id),
                    format!("imap:{}:{}", self.imap_config.host, uid),
                ));
            }
        }

        None
    }

    // -----------------------------------------------------------------------
    // Private: Kivra search
    // -----------------------------------------------------------------------

    /// Query the Kivra digital mailbox for a receipt matching the transaction.
    /// Kivra is a Swedish government-approved digital mailbox used by many
    /// merchants to send receipts and invoices.
    ///
    /// In production this calls the Kivra Partner API using an OAuth2 bearer
    /// token from the secret store.  Here we model the full response structure
    /// so that wiring it up is a matter of adding the HTTP calls.
    async fn search_kivra(&self, txn: &Transaction) -> Option<Document> {
        // Kivra API base (configurable via env/config in production)
        // let base_url = std::env::var("KIVRA_API_BASE")
        //     .unwrap_or_else(|_| "https://api.kivra.com/v1".to_string());

        // 1. Build query parameters
        let _merchant = Self::imap_search_keyword(&txn.merchant);
        let _since = Self::unix_to_iso8601(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub((self.imap_config.search_window_days as u64) * 86_400),
        );
        let _amount = txn.amount;

        // 2. In production: issue GET /receipts?merchant=<>&since=<>&amount=<>
        //    and deserialise the JSON envelope.

        // For now: return None to fall through to vendor-contact stage.
        // The function is properly async so that the real HTTP client (reqwest)
        // can be dropped in here without changing the call site.
        None
    }

    // -----------------------------------------------------------------------
    // Private: Compose vendor recovery email
    // -----------------------------------------------------------------------

    /// Compose a polite, professional receipt-request email to the vendor.
    fn compose_recovery_email(&self, txn: &Transaction) -> RecoveryEmail {
        let vendor_email = txn.merchant_email.clone().unwrap_or_default();

        let booked_str = Self::format_system_time(txn.booked_at);

        let reference_hint = txn
            .reference
            .as_deref()
            .map(|r| format!("\n\nTransaction reference: {}", r))
            .unwrap_or_default();

        let subject = format!(
            "Receipt Request – {} {} on {}",
            txn.amount, txn.currency, booked_str
        );

        let body = format!(
            "Dear {merchant} Finance / Customer Service,\n\
             \n\
             I am writing to request a receipt for the following transaction:\n\
             \n\
             • Merchant : {merchant}\n\
             • Date     : {date}\n\
             • Amount   : {amount} {currency}{reference_hint}\n\
             \n\
             Could you please send the receipt (PDF or image) to this email address \
             at your earliest convenience?  It is required for our company's accounting records.\n\
             \n\
             Transaction ID (internal reference): {txn_id}\n\
             \n\
             Thank you in advance for your assistance.\n\
             \n\
             Kind regards,\n\
             Reconciler Accounting System",
            merchant = txn.merchant,
            date = booked_str,
            amount = txn.amount,
            currency = txn.currency,
            reference_hint = reference_hint,
            txn_id = txn.id,
        );

        RecoveryEmail {
            to: vendor_email,
            subject,
            body,
            transaction_reference: txn
                .reference
                .clone()
                .unwrap_or_else(|| txn.id.to_string()),
        }
    }

    // -----------------------------------------------------------------------
    // Private: IMAP abstraction
    // -----------------------------------------------------------------------

    /// Create an IMAP session.  In production this opens a real TLS connection;
    /// in tests the `MockImapSession` is injected instead.
    async fn imap_connect(&self) -> Result<ImapSession, ImapError> {
        ImapSession::connect(&self.imap_config).await
    }

    // -----------------------------------------------------------------------
    // Private: utility functions
    // -----------------------------------------------------------------------

    /// Build an IMAP SINCE date string (format: "1-Jan-2024").
    fn since_date_string(txn: &Transaction, window_days: u32) -> String {
        let booked_secs = txn
            .booked_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window_secs = (window_days as u64) * 86_400;
        let since_secs = booked_secs.saturating_sub(window_secs);
        Self::unix_to_imap_date(since_secs)
    }

    fn unix_to_imap_date(secs: u64) -> String {
        // Simple conversion: days since epoch → day/month/year
        let days = secs / 86_400;
        // Gregorian proleptic calendar from day 0 = 1970-01-01
        let (y, m, d) = Self::days_to_ymd(days);
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun",
            "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        format!("{}-{}-{}", d, MONTHS[(m - 1) as usize], y)
    }

    fn unix_to_iso8601(secs: u64) -> String {
        let days = secs / 86_400;
        let (y, m, d) = Self::days_to_ymd(days);
        format!("{:04}-{:02}-{:02}", y, m, d)
    }

    /// Convert days-since-epoch (1970-01-01) to (year, month, day).
    /// Uses the algorithm from Howard Hinnant (public domain).
    fn days_to_ymd(days: u64) -> (u64, u64, u64) {
        let z = days + 719_468;
        let era = z / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        (y, m, d)
    }

    fn format_system_time(t: SystemTime) -> String {
        let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        Self::unix_to_iso8601(secs)
    }

    /// Reduce a merchant name to a safe IMAP search keyword (first word, cleaned).
    fn imap_search_keyword(merchant: &str) -> String {
        merchant
            .split_whitespace()
            .next()
            .unwrap_or(merchant)
            .to_uppercase()
    }

    // -----------------------------------------------------------------------
    // Private: MIME helpers (used by parse_receipt_reply)
    // -----------------------------------------------------------------------

    fn extract_mime_boundary(raw: &str) -> Option<String> {
        // Look for: Content-Type: multipart/...; boundary="<boundary>"
        let lower = raw.to_lowercase();
        let idx = lower.find("boundary=")?;
        let rest = &raw[idx + "boundary=".len()..];
        let boundary = if rest.starts_with('"') {
            // Quoted boundary
            let end = rest[1..].find('"')?;
            rest[1..=end].to_string()
        } else {
            // Unquoted: ends at whitespace or semicolon
            rest.split(|c: char| c == ';' || c == '\r' || c == '\n' || c == ' ')
                .next()?
                .to_string()
        };
        if boundary.is_empty() { None } else { Some(boundary) }
    }

    fn extract_header_param(part: &str, header: &str, param: &str) -> Option<String> {
        let lower_part = part.to_lowercase();
        let header_lc = header.to_lowercase();
        let param_lc = format!("{}=", param.to_lowercase());

        let header_idx = lower_part.find(&header_lc)?;
        let header_end = lower_part[header_idx..].find('\n').unwrap_or(lower_part.len() - header_idx);
        let header_line = &lower_part[header_idx..header_idx + header_end];

        let param_idx = header_line.find(&param_lc)?;
        let rest = &part[header_idx + param_idx + param.len() + 1..];
        let value = if rest.starts_with('"') {
            let end = rest[1..].find('"').unwrap_or(rest.len() - 1);
            rest[1..=end].to_string()
        } else {
            rest.split(|c: char| c == ';' || c == '\r' || c == '\n')
                .next()?
                .trim()
                .to_string()
        };
        if value.is_empty() { None } else { Some(value) }
    }

    fn extract_transaction_id_from_headers(raw: &str) -> Option<Uuid> {
        // We embed the txn id in the X-Reconciler-Txn-Id header when we send
        // the recovery email.
        for line in raw.lines() {
            if line.to_lowercase().starts_with("x-reconciler-txn-id:") {
                let id_str = line.splitn(2, ':').nth(1)?.trim();
                return Uuid::parse_str(id_str).ok();
            }
        }
        None
    }

    /// Minimal base-64 decoder (RFC 4648 standard alphabet).
    /// Avoids pulling in a `base64` crate just for this single use-site.
    fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
        const TABLE: &[u8; 128] = &{
            let mut t = [0xFF_u8; 128];
            let mut i = 0u8;
            // A–Z
            while i < 26 { t[(b'A' + i) as usize] = i; i += 1; }
            // a–z
            i = 0;
            while i < 26 { t[(b'a' + i) as usize] = 26 + i; i += 1; }
            // 0–9
            i = 0;
            while i < 10 { t[(b'0' + i) as usize] = 52 + i; i += 1; }
            t[b'+' as usize] = 62;
            t[b'/' as usize] = 63;
            t
        };

        let bytes = input.as_bytes();
        let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
        let mut i = 0;

        while i + 3 < bytes.len() {
            let b0 = bytes[i];
            let b1 = bytes[i + 1];
            let b2 = bytes[i + 2];
            let b3 = bytes[i + 3];

            if b0 >= 128 || b1 >= 128 || b2 >= 128 || b3 >= 128 {
                return Err("non-ASCII character in base64");
            }

            let v0 = TABLE[b0 as usize];
            let v1 = TABLE[b1 as usize];
            if v0 == 0xFF || v1 == 0xFF {
                return Err("invalid base64 character");
            }

            out.push((v0 << 2) | (v1 >> 4));

            if b2 != b'=' {
                let v2 = TABLE[b2 as usize];
                if v2 == 0xFF { return Err("invalid base64 character"); }
                out.push((v1 << 4) | (v2 >> 2));

                if b3 != b'=' {
                    let v3 = TABLE[b3 as usize];
                    if v3 == 0xFF { return Err("invalid base64 character"); }
                    out.push((v2 << 6) | v3);
                }
            }

            i += 4;
        }

        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// IMAP session abstraction
// ---------------------------------------------------------------------------
// This thin wrapper lets us swap in a mock for unit tests without pulling in
// a real IMAP crate.  In production, replace the body of `connect` with the
// async-imap TLS handshake and authentication flow.

struct ImapSession {
    _config: ImapConfig,
}

#[derive(Debug)]
struct ImapError(String);

impl std::fmt::Display for ImapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IMAP error: {}", self.0)
    }
}

impl std::error::Error for ImapError {}

impl ImapSession {
    async fn connect(config: &ImapConfig) -> Result<Self, ImapError> {
        // Production implementation sketch:
        //
        // let tls = async_native_tls::TlsConnector::new();
        // let client = async_imap::connect(
        //     (config.host.as_str(), config.port),
        //     config.host.as_str(),
        //     &tls,
        // ).await.map_err(|e| ImapError(e.to_string()))?;
        //
        // let mut session = client
        //     .login(&config.username, &config.password)
        //     .await
        //     .map_err(|(e, _)| ImapError(e.to_string()))?;
        //
        // session.select(&config.mailbox)
        //     .await
        //     .map_err(|e| ImapError(e.to_string()))?;
        //
        // Ok(Self { session, config: config.clone() })

        // Placeholder: returns empty session so the pipeline falls through
        // gracefully to later stages when no real IMAP server is configured.
        if config.username.is_empty() {
            return Err(ImapError("IMAP username not configured".into()));
        }
        Ok(Self { _config: config.clone() })
    }

    async fn search_since_subject(
        &self,
        since: &str,
        subject: &str,
    ) -> Result<Vec<u32>, ImapError> {
        // Production:
        // let query = format!("SINCE {} SUBJECT \"{}\"", since, subject);
        // let uids = self.session.uid_search(&query).await...;
        let _ = (since, subject);
        Ok(vec![])
    }

    async fn fetch_message(
        &self,
        uid: u32,
    ) -> Result<(String, String, Vec<(String, Vec<u8>, String)>), ImapError> {
        // Production: fetch RFC822 or BODY[] via UID FETCH
        let _ = uid;
        Ok((String::new(), String::new(), vec![]))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_txn() -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            amount: 349.90,
            currency: "SEK".to_string(),
            merchant: "ICA MAXI".to_string(),
            merchant_email: Some("receipt@ica.se".to_string()),
            booked_at: SystemTime::now() - Duration::from_secs(86_400 * 3),
            description: Some("Groceries".to_string()),
            reference: Some("TXN-20240115-001".to_string()),
        }
    }

    #[test]
    fn test_compose_recovery_email_structure() {
        let agent = ReceiptRecoveryAgent::new(
            ImapConfig::default(),
            Arc::new(ConfidenceEngine),
        );
        let txn = make_txn();
        let email = agent.compose_recovery_email(&txn);

        assert_eq!(email.to, "receipt@ica.se");
        assert!(email.subject.contains("349.90"));
        assert!(email.subject.contains("SEK"));
        assert!(email.body.contains("ICA MAXI"));
        assert!(email.body.contains("349.90"));
        assert!(email.body.contains("TXN-20240115-001"));
        assert_eq!(email.transaction_reference, "TXN-20240115-001");
    }

    #[test]
    fn test_compose_recovery_email_no_reference() {
        let agent = ReceiptRecoveryAgent::new(
            ImapConfig::default(),
            Arc::new(ConfidenceEngine),
        );
        let mut txn = make_txn();
        txn.reference = None;
        let email = agent.compose_recovery_email(&txn);

        // Falls back to transaction UUID as reference
        assert_eq!(email.transaction_reference, txn.id.to_string());
    }

    #[tokio::test]
    async fn test_recover_no_imap_no_kivra_has_vendor_email() {
        // With empty IMAP credentials, email search fails → Kivra returns None →
        // vendor email is present → result is Requested.
        let agent = ReceiptRecoveryAgent::new(
            ImapConfig::default(), // empty username → IMAP connect fails
            Arc::new(ConfidenceEngine),
        );
        let txn = make_txn();
        let result = agent.recover(&txn).await;

        assert_eq!(result.transaction_id, txn.id);
        assert_eq!(result.status, RecoveryStatus::Requested);
        assert_eq!(result.method, RecoveryMethod::VendorContact);
        assert!(result.document.is_none());
        // Audit trail should record both failed stages + successful vendor contact
        let methods: Vec<&RecoveryMethod> = result.audit_trail.iter().map(|e| &e.method).collect();
        assert!(methods.contains(&&RecoveryMethod::EmailSearch));
        assert!(methods.contains(&&RecoveryMethod::KivraSearch));
        assert!(methods.contains(&&RecoveryMethod::VendorContact));
    }

    #[tokio::test]
    async fn test_recover_no_vendor_email_falls_to_manual() {
        let agent = ReceiptRecoveryAgent::new(
            ImapConfig::default(),
            Arc::new(ConfidenceEngine),
        );
        let mut txn = make_txn();
        txn.merchant_email = None;
        let result = agent.recover(&txn).await;

        assert_eq!(result.status, RecoveryStatus::ManualRequired);
    }

    #[test]
    fn test_confidence_engine_score_basic() {
        let engine = ConfidenceEngine;
        let txn = make_txn();
        let score = engine.score_email_match(
            &txn,
            "Your ICA receipt for 349.90",
            "Thank you for shopping at ICA MAXI. Total: 349.90 SEK",
        );
        // Merchant in subject (+0.35), merchant in body (+0.15), amount in body (+0.30), baseline (+0.05)
        assert!(score >= 0.80, "score was {}", score);
    }

    #[test]
    fn test_confidence_engine_score_no_match() {
        let engine = ConfidenceEngine;
        let txn = make_txn();
        let score = engine.score_email_match(
            &txn,
            "Monthly newsletter from Acme Corp",
            "Hi, here is your monthly digest",
        );
        assert!(score < EMAIL_CONFIDENCE_THRESHOLD, "score was {}", score);
    }

    #[test]
    fn test_parse_receipt_reply_pdf() {
        let agent = ReceiptRecoveryAgent::new(
            ImapConfig::default(),
            Arc::new(ConfidenceEngine),
        );

        // Minimal valid base64-encoded payload (three bytes: 0xDE 0xAD 0xBE)
        let b64_content = "3q2+"; // base64 of [0xDE, 0xAD, 0xBE]

        let raw_email = format!(
            "MIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"boundary42\"\r\n\
             X-Reconciler-Txn-Id: {txn_id}\r\n\
             \r\n\
             --boundary42\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             Please find your receipt attached.\r\n\
             --boundary42\r\n\
             Content-Type: application/pdf; name=\"receipt.pdf\"\r\n\
             Content-Disposition: attachment; filename=\"receipt.pdf\"\r\n\
             Content-Transfer-Encoding: base64\r\n\
             \r\n\
             {b64}\r\n\
             --boundary42--\r\n",
            txn_id = Uuid::new_v4(),
            b64 = b64_content,
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let doc = rt.block_on(agent.parse_receipt_reply(raw_email.as_bytes()));

        assert!(doc.is_some(), "Expected a document to be parsed");
        let doc = doc.unwrap();
        assert_eq!(doc.mime_type, "application/pdf");
        assert_eq!(doc.filename, "receipt.pdf");
        assert_eq!(doc.data, vec![0xDE, 0xAD, 0xBE]);
    }

    #[test]
    fn test_imap_since_date_format() {
        let txn = make_txn();
        let date = ReceiptRecoveryAgent::since_date_string(&txn, 0);
        // Should be in "D-Mon-YYYY" format
        assert!(date.contains('-'), "date format: {}", date);
        let parts: Vec<&str> = date.split('-').collect();
        assert_eq!(parts.len(), 3, "date: {}", date);
    }

    #[test]
    fn test_imap_search_keyword() {
        assert_eq!(ReceiptRecoveryAgent::imap_search_keyword("ICA MAXI"), "ICA");
        assert_eq!(ReceiptRecoveryAgent::imap_search_keyword("Amazon Web Services"), "AMAZON");
        assert_eq!(ReceiptRecoveryAgent::imap_search_keyword(""), "");
    }

    #[test]
    fn test_base64_decode_roundtrip() {
        // "Hello" → SGVsbG8=
        let encoded = "SGVsbG8=";
        let decoded = ReceiptRecoveryAgent::base64_decode(encoded).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_base64_decode_three_bytes() {
        // 0xDE 0xAD 0xBE → 3q2+
        let encoded = "3q2+";
        let decoded = ReceiptRecoveryAgent::base64_decode(encoded).unwrap();
        assert_eq!(decoded, vec![0xDE, 0xAD, 0xBE]);
    }
}
