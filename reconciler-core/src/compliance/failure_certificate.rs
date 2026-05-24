use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::chain_of_custody::{CustodyAction, CustodyChain};

// ---------------------------------------------------------------------------
// RetrievalAttempt
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalAttempt {
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub target: String,
    pub outcome: String,
    pub evidence_hash: Option<String>,
}

// ---------------------------------------------------------------------------
// FailureCertificate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureCertificate {
    pub certificate_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub transaction_id: Uuid,
    pub merchant: String,
    pub amount: Decimal,
    pub currency: String,
    pub transaction_date: DateTime<Utc>,
    pub attempts: Vec<RetrievalAttempt>,
    pub final_determination: String,
    pub legal_basis: String,
    pub certificate_hash: String,
}

impl FailureCertificate {
    pub fn generate(chain: &CustodyChain, transaction: &serde_json::Value) -> Self {
        let transaction_id = transaction
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let merchant = transaction
            .get("merchant")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Merchant")
            .to_string();

        let amount = transaction
            .get("amount")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Decimal>().ok())
            .or_else(|| {
                transaction
                    .get("amount")
                    .and_then(|v| v.as_f64())
                    .map(|f| {
                        Decimal::try_from(f).unwrap_or(Decimal::ZERO)
                    })
            })
            .unwrap_or(Decimal::ZERO);

        let currency = transaction
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("SEK")
            .to_string();

        let transaction_date = transaction
            .get("date")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
            .unwrap_or_else(Utc::now);

        // Build retrieval attempts from chain events
        let mut attempts: Vec<RetrievalAttempt> = Vec::new();
        for event in chain.events() {
            let (method, target, outcome) = match &event.action {
                CustodyAction::ApiRetrievalAttempted => (
                    "api".to_string(),
                    event
                        .channel
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    "attempted".to_string(),
                ),
                CustodyAction::ApiRetrievalSucceeded => (
                    "api".to_string(),
                    event
                        .channel
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    "succeeded".to_string(),
                ),
                CustodyAction::ApiRetrievalFailed { reason } => (
                    "api".to_string(),
                    event
                        .channel
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    format!("failed: {}", reason),
                ),
                CustodyAction::PeppolRequestSent => (
                    "peppol".to_string(),
                    "peppol-network".to_string(),
                    "sent".to_string(),
                ),
                CustodyAction::EmailSent { to, subject } => (
                    "email".to_string(),
                    to.clone(),
                    format!("sent – {}", subject),
                ),
                CustodyAction::SmsSent { to } => {
                    ("sms".to_string(), to.clone(), "sent".to_string())
                }
                CustodyAction::VoiceCallInitiated => (
                    "voice".to_string(),
                    event
                        .channel
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    "initiated".to_string(),
                ),
                CustodyAction::PostalLetterSent { tracking } => (
                    "postal".to_string(),
                    tracking
                        .clone()
                        .unwrap_or_else(|| "no-tracking".to_string()),
                    "sent".to_string(),
                ),
                _ => continue,
            };

            attempts.push(RetrievalAttempt {
                timestamp: event.timestamp,
                method,
                target,
                outcome,
                evidence_hash: event.content_hash.clone(),
            });
        }

        let final_determination = if attempts.is_empty() {
            "Inga hämtningsförsök genomfördes.".to_string()
        } else {
            format!(
                "Kvitto/verifikation kunde ej erhållas efter {} hämtningsförsök via \
                 digitala kanaler (API, e-post, SMS) och eskaleringsprocessen. \
                 Verifikation bedöms ej tillgänglig.",
                attempts.len()
            )
        };

        let issued_at = Utc::now();
        let certificate_id = Uuid::new_v4();

        // Compute certificate hash
        let payload = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            certificate_id,
            issued_at.timestamp(),
            transaction_id,
            merchant,
            amount,
            currency,
            final_determination,
        );
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let certificate_hash = format!("{:x}", hasher.finalize());

        FailureCertificate {
            certificate_id,
            issued_at,
            transaction_id,
            merchant,
            amount,
            currency,
            transaction_date,
            attempts,
            final_determination,
            legal_basis:
                "Bokföringslagen 7 kap 2§ – verifikation ej tillgänglig".to_string(),
            certificate_hash,
        }
    }

    /// Generate a minimal plain-text PDF as raw bytes (uses simple text layout).
    pub fn to_pdf_bytes(&self) -> Vec<u8> {
        // Build a minimal hand-crafted PDF (no external PDF crate required).
        // Content stream contains the certificate as plain text.
        let lines: Vec<String> = vec![
            "INTYG OM EJ ATERFINBAR VERIFIKATION".into(),
            "Bokföringslagen 7 kap 2§".into(),
            "".into(),
            format!("Intygs-ID:       {}", self.certificate_id),
            format!(
                "Utfärdat:        {}",
                self.issued_at.format("%Y-%m-%d %H:%M:%S UTC")
            ),
            "".into(),
            format!("Transaktion-ID:  {}", self.transaction_id),
            format!("Handlare:        {}", self.merchant),
            format!("Belopp:          {} {}", self.amount, self.currency),
            format!(
                "Datum:           {}",
                self.transaction_date.format("%Y-%m-%d")
            ),
            "".into(),
            "HÄMTNINGSFÖRSÖK:".into(),
        ];

        let mut all_lines = lines;
        for (i, attempt) in self.attempts.iter().enumerate() {
            all_lines.push(format!(
                "  {}. {} – {} – {} ({})",
                i + 1,
                attempt.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                attempt.method,
                attempt.target,
                attempt.outcome,
            ));
        }

        all_lines.push("".into());
        all_lines.push("SLUTLIG BEDÖMNING:".into());
        all_lines.push(self.final_determination.clone());
        all_lines.push("".into());
        all_lines.push(format!("Rättslig grund:  {}", self.legal_basis));
        all_lines.push("".into());
        all_lines.push(format!("Intyg-hash (SHA-256):\n  {}", self.certificate_hash));

        // Build PDF content stream
        let mut stream_parts: Vec<String> = Vec::new();
        stream_parts.push("BT".into());
        stream_parts.push("/F1 11 Tf".into());
        stream_parts.push("50 800 Td".into());
        stream_parts.push("14 TL".into());

        for line in &all_lines {
            let safe = line.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
            stream_parts.push(format!("({}) Tj T*", safe));
        }
        stream_parts.push("ET".into());

        let stream_content = stream_parts.join("\n");
        let stream_bytes = stream_content.as_bytes();
        let stream_len = stream_bytes.len();

        let pdf = format!(
            "%PDF-1.4\n\
             1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R\n   /MediaBox [0 0 595 842]\n   /Contents 4 0 R\n   /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
             4 0 obj\n<< /Length {stream_len} >>\nstream\n{stream_content}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
             xref\n0 6\n0000000000 65535 f \n\
             trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n0\n%%EOF\n",
            stream_len = stream_len,
            stream_content = stream_content,
        );

        pdf.into_bytes()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}
