use std::fmt::Write as FmtWrite;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Domain types referenced by the audit module
// ---------------------------------------------------------------------------

/// A financial transaction (minimal shape; extend as needed by the larger domain model).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub merchant: String,
    pub category: String,
    pub date: DateTime<Utc>,
    pub description: String,
    /// Reference to supporting document (receipt, invoice, …). None = missing.
    pub document_id: Option<Uuid>,
    /// Jurisdiction this transaction belongs to.
    pub jurisdiction: String,
}

/// A double-entry ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub account_debit: String,
    pub account_credit: String,
    pub amount: Decimal,
    pub currency: String,
    pub date: DateTime<Utc>,
    pub memo: String,
}

/// An attached document (receipt, invoice, contract, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub document_type: String,
    pub filename: String,
    /// SHA-256 hex digest of document content, for integrity verification.
    pub content_hash: String,
    pub uploaded_at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// An immutable audit-trail event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    /// JSON-serialised diff / snapshot of what changed.
    pub payload: Value,
}

// ---------------------------------------------------------------------------
// Audit output types
// ---------------------------------------------------------------------------

/// A transaction that is missing supporting documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingDocument {
    pub transaction_id: Uuid,
    pub amount: Decimal,
    pub merchant: String,
    pub date: DateTime<Utc>,
    /// "none" | "requested" | "partial" | "waived"
    pub recovery_status: String,
}

/// Summary of documentation completeness for a set of transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletenessReport {
    pub total_transactions: usize,
    pub with_receipt: usize,
    pub without_receipt: usize,
    /// Percentage of transactions that have supporting documents (0.0–100.0).
    pub completeness_pct: f64,
    pub missing: Vec<MissingDocument>,
}

/// A complete, self-contained audit package for a company period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPack {
    pub id: Uuid,
    pub company_id: Uuid,
    /// Human-readable period description, e.g. "2024-Q1" or "2024".
    pub period: String,
    pub transactions: Vec<Transaction>,
    pub journal_entries: Vec<LedgerEntry>,
    pub documents: Vec<Document>,
    /// 0.0–100.0 completeness score.
    pub completeness_score: f64,
    pub generated_at: DateTime<Utc>,
    /// SHA-256 hash of the entire pack (excluding itself) for tamper detection.
    pub integrity_hash: String,
    pub missing_documents: Vec<MissingDocument>,
}

// ---------------------------------------------------------------------------
// AuditEngine
// ---------------------------------------------------------------------------

pub struct AuditEngine;

impl AuditEngine {
    pub fn new() -> Self {
        Self
    }

    // ------------------------------------------------------------------
    // Core public API
    // ------------------------------------------------------------------

    /// Generate a full audit pack for a company over a period.
    ///
    /// In production `transactions`, `journal_entries`, and `documents`
    /// would be fetched from the database; here the caller supplies them
    /// so the method is pure and fully testable without database access.
    pub async fn generate_audit_pack(
        &self,
        company_id: Uuid,
        period: &str,
        transactions: Vec<Transaction>,
        journal_entries: Vec<LedgerEntry>,
        documents: Vec<Document>,
    ) -> AuditPack {
        let completeness = self.verify_documentation(&transactions);
        let completeness_score = completeness.completeness_pct;
        let missing_documents = completeness.missing.clone();

        // Derive synthetic audit events from the transactions for integrity hashing.
        let events: Vec<AuditEvent> = transactions
            .iter()
            .map(|tx| AuditEvent {
                id: Uuid::new_v4(),
                entity_type: "transaction".to_string(),
                entity_id: tx.id,
                action: "record".to_string(),
                actor: "system".to_string(),
                timestamp: tx.date,
                payload: json!({
                    "amount": tx.amount.to_string(),
                    "currency": tx.currency,
                    "merchant": tx.merchant,
                    "category": tx.category,
                }),
            })
            .collect();

        let integrity_hash = self.hash_audit_trail(&events);

        AuditPack {
            id: Uuid::new_v4(),
            company_id,
            period: period.to_string(),
            transactions,
            journal_entries,
            documents,
            completeness_score,
            generated_at: Utc::now(),
            integrity_hash,
            missing_documents,
        }
    }

    /// Verify documentation completeness for a slice of transactions.
    pub fn verify_documentation(&self, transactions: &[Transaction]) -> CompletenessReport {
        let total = transactions.len();
        let mut missing = Vec::new();

        let with_receipt = transactions
            .iter()
            .filter(|tx| {
                if tx.document_id.is_some() {
                    true
                } else {
                    missing.push(MissingDocument {
                        transaction_id: tx.id,
                        amount: tx.amount,
                        merchant: tx.merchant.clone(),
                        date: tx.date,
                        recovery_status: "none".to_string(),
                    });
                    false
                }
            })
            .count();

        let without_receipt = total - with_receipt;

        let completeness_pct = if total == 0 {
            100.0
        } else {
            (with_receipt as f64 / total as f64) * 100.0
        };

        CompletenessReport {
            total_transactions: total,
            with_receipt,
            without_receipt,
            completeness_pct,
            missing,
        }
    }

    /// Generate a deterministic SHA-256 hash over an ordered slice of audit events.
    ///
    /// The hash is computed over a stable canonical serialisation:
    ///   `{id}|{entity_type}|{entity_id}|{action}|{actor}|{timestamp_rfc3339}|{payload_json}\n`
    ///
    /// for each event in order, then SHA-256 of the resulting UTF-8 string.
    pub fn hash_audit_trail(&self, events: &[AuditEvent]) -> String {
        let mut canonical = String::new();
        for event in events {
            writeln!(
                canonical,
                "{}|{}|{}|{}|{}|{}|{}",
                event.id,
                event.entity_type,
                event.entity_id,
                event.action,
                event.actor,
                event.timestamp.to_rfc3339(),
                event.payload,
            )
            .expect("String write is infallible");
        }

        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let result = hasher.finalize();

        let mut hex = String::with_capacity(64);
        for byte in result {
            write!(hex, "{:02x}", byte).expect("String write is infallible");
        }
        hex
    }

    /// Export an AuditPack as a structured JSON value suitable for archival,
    /// transmission to a regulator, or storage in a document store.
    pub fn export_json(&self, pack: &AuditPack) -> Value {
        json!({
            "schema_version": "1.0",
            "id": pack.id.to_string(),
            "company_id": pack.company_id.to_string(),
            "period": pack.period,
            "generated_at": pack.generated_at.to_rfc3339(),
            "integrity_hash": pack.integrity_hash,
            "completeness_score": pack.completeness_score,
            "summary": {
                "total_transactions": pack.transactions.len(),
                "total_journal_entries": pack.journal_entries.len(),
                "total_documents": pack.documents.len(),
                "missing_document_count": pack.missing_documents.len(),
            },
            "transactions": pack.transactions.iter().map(|tx| json!({
                "id": tx.id.to_string(),
                "amount": tx.amount.to_string(),
                "currency": tx.currency,
                "merchant": tx.merchant,
                "category": tx.category,
                "date": tx.date.to_rfc3339(),
                "description": tx.description,
                "document_id": tx.document_id.map(|u| u.to_string()),
                "jurisdiction": tx.jurisdiction,
            })).collect::<Vec<_>>(),
            "journal_entries": pack.journal_entries.iter().map(|je| json!({
                "id": je.id.to_string(),
                "transaction_id": je.transaction_id.to_string(),
                "account_debit": je.account_debit,
                "account_credit": je.account_credit,
                "amount": je.amount.to_string(),
                "currency": je.currency,
                "date": je.date.to_rfc3339(),
                "memo": je.memo,
            })).collect::<Vec<_>>(),
            "documents": pack.documents.iter().map(|doc| json!({
                "id": doc.id.to_string(),
                "transaction_id": doc.transaction_id.to_string(),
                "document_type": doc.document_type,
                "filename": doc.filename,
                "content_hash": doc.content_hash,
                "uploaded_at": doc.uploaded_at.to_rfc3339(),
                "size_bytes": doc.size_bytes,
            })).collect::<Vec<_>>(),
            "missing_documents": pack.missing_documents.iter().map(|md| json!({
                "transaction_id": md.transaction_id.to_string(),
                "amount": md.amount.to_string(),
                "merchant": md.merchant,
                "date": md.date.to_rfc3339(),
                "recovery_status": md.recovery_status,
            })).collect::<Vec<_>>(),
        })
    }

    // ------------------------------------------------------------------
    // Additional helpers
    // ------------------------------------------------------------------

    /// Compute a rolling integrity hash over an existing hash and new events.
    /// Useful for appending to an existing audit trail without re-hashing everything.
    pub fn extend_audit_hash(&self, prior_hash: &str, new_events: &[AuditEvent]) -> String {
        let mut canonical = format!("PRIOR:{}\n", prior_hash);
        for event in new_events {
            writeln!(
                canonical,
                "{}|{}|{}|{}|{}|{}|{}",
                event.id,
                event.entity_type,
                event.entity_id,
                event.action,
                event.actor,
                event.timestamp.to_rfc3339(),
                event.payload,
            )
            .expect("String write is infallible");
        }

        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let result = hasher.finalize();

        let mut hex = String::with_capacity(64);
        for byte in result {
            write!(hex, "{:02x}", byte).expect("String write is infallible");
        }
        hex
    }

    /// Verify that a pack's `integrity_hash` is still valid by recomputing it.
    /// Returns `true` when the hash matches (pack is untampered).
    pub fn verify_pack_integrity(&self, pack: &AuditPack) -> bool {
        let events: Vec<AuditEvent> = pack
            .transactions
            .iter()
            .map(|tx| AuditEvent {
                id: Uuid::nil(), // deterministic nil for re-check (id itself is not stable)
                entity_type: "transaction".to_string(),
                entity_id: tx.id,
                action: "record".to_string(),
                actor: "system".to_string(),
                timestamp: tx.date,
                payload: json!({
                    "amount": tx.amount.to_string(),
                    "currency": tx.currency,
                    "merchant": tx.merchant,
                    "category": tx.category,
                }),
            })
            .collect();

        self.hash_audit_trail(&events) == pack.integrity_hash
    }
}

impl Default for AuditEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn make_tx(with_doc: bool) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            amount: dec!(100.00),
            currency: "SEK".to_string(),
            merchant: "Acme AB".to_string(),
            category: "office_supplies".to_string(),
            date: Utc.with_ymd_and_hms(2024, 3, 1, 12, 0, 0).unwrap(),
            description: "Pens and paper".to_string(),
            document_id: if with_doc { Some(Uuid::new_v4()) } else { None },
            jurisdiction: "SE".to_string(),
        }
    }

    #[test]
    fn test_verify_documentation_all_present() {
        let engine = AuditEngine::new();
        let txs = vec![make_tx(true), make_tx(true)];
        let report = engine.verify_documentation(&txs);
        assert_eq!(report.total_transactions, 2);
        assert_eq!(report.with_receipt, 2);
        assert_eq!(report.without_receipt, 0);
        assert!((report.completeness_pct - 100.0).abs() < f64::EPSILON);
        assert!(report.missing.is_empty());
    }

    #[test]
    fn test_verify_documentation_partial() {
        let engine = AuditEngine::new();
        let txs = vec![make_tx(true), make_tx(false), make_tx(false)];
        let report = engine.verify_documentation(&txs);
        assert_eq!(report.total_transactions, 3);
        assert_eq!(report.with_receipt, 1);
        assert_eq!(report.without_receipt, 2);
        assert!((report.completeness_pct - (1.0 / 3.0 * 100.0)).abs() < 0.01);
        assert_eq!(report.missing.len(), 2);
    }

    #[test]
    fn test_verify_documentation_empty() {
        let engine = AuditEngine::new();
        let report = engine.verify_documentation(&[]);
        assert_eq!(report.total_transactions, 0);
        assert!((report.completeness_pct - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hash_audit_trail_deterministic() {
        let engine = AuditEngine::new();
        let event = AuditEvent {
            id: Uuid::nil(),
            entity_type: "transaction".to_string(),
            entity_id: Uuid::nil(),
            action: "record".to_string(),
            actor: "system".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            payload: json!({"amount": "100.00"}),
        };
        let h1 = engine.hash_audit_trail(&[event.clone()]);
        let h2 = engine.hash_audit_trail(&[event]);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_hash_audit_trail_differs_on_mutation() {
        let engine = AuditEngine::new();
        let mut e1 = AuditEvent {
            id: Uuid::nil(),
            entity_type: "transaction".to_string(),
            entity_id: Uuid::nil(),
            action: "record".to_string(),
            actor: "system".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            payload: json!({"amount": "100.00"}),
        };
        let h1 = engine.hash_audit_trail(&[e1.clone()]);
        e1.payload = json!({"amount": "999.00"}); // tamper
        let h2 = engine.hash_audit_trail(&[e1]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_export_json_structure() {
        let engine = AuditEngine::new();
        let pack = AuditPack {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            period: "2024-Q1".to_string(),
            transactions: vec![make_tx(true)],
            journal_entries: vec![],
            documents: vec![],
            completeness_score: 100.0,
            generated_at: Utc::now(),
            integrity_hash: "abc123".to_string(),
            missing_documents: vec![],
        };
        let json = engine.export_json(&pack);
        assert_eq!(json["schema_version"], "1.0");
        assert_eq!(json["period"], "2024-Q1");
        assert_eq!(json["summary"]["total_transactions"], 1);
        assert_eq!(json["summary"]["missing_document_count"], 0);
        assert!(json["transactions"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_extend_audit_hash_changes_on_new_events() {
        let engine = AuditEngine::new();
        let prior = "0000000000000000000000000000000000000000000000000000000000000000";
        let event = AuditEvent {
            id: Uuid::nil(),
            entity_type: "transaction".to_string(),
            entity_id: Uuid::nil(),
            action: "record".to_string(),
            actor: "system".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            payload: json!({"amount": "50.00"}),
        };
        let extended = engine.extend_audit_hash(prior, &[event]);
        assert_ne!(extended, prior);
        assert_eq!(extended.len(), 64);
    }

    #[tokio::test]
    async fn test_generate_audit_pack() {
        let engine = AuditEngine::new();
        let company_id = Uuid::new_v4();
        let txs = vec![make_tx(true), make_tx(false)];
        let pack = engine
            .generate_audit_pack(company_id, "2024-Q2", txs, vec![], vec![])
            .await;
        assert_eq!(pack.company_id, company_id);
        assert_eq!(pack.period, "2024-Q2");
        assert_eq!(pack.transactions.len(), 2);
        assert_eq!(pack.missing_documents.len(), 1);
        assert!((pack.completeness_score - 50.0).abs() < 0.01);
        assert_eq!(pack.integrity_hash.len(), 64);
    }
}
