use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Transaction, Invoice, AuditEvent};

// ─────────────────────────────────────────────
// CONFIDENCE SCORE — every decision has one
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub score: f64,               // 0.0–1.0
    pub reasons: Vec<String>,     // human-readable explanation
    pub rollback_possible: bool,
    pub requires_human_review: bool,
    pub auto_book_threshold: f64, // default 0.95
}

impl ConfidenceScore {
    pub fn high(reasons: Vec<String>) -> Self {
        Self {
            score: 0.97,
            reasons,
            rollback_possible: true,
            requires_human_review: false,
            auto_book_threshold: 0.95,
        }
    }

    pub fn low(score: f64, reasons: Vec<String>) -> Self {
        Self {
            score,
            reasons,
            rollback_possible: true,
            requires_human_review: true,
            auto_book_threshold: 0.95,
        }
    }

    pub fn should_auto_book(&self) -> bool {
        self.score >= self.auto_book_threshold && !self.requires_human_review
    }
}

// ─────────────────────────────────────────────
// ANOMALY
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: Uuid,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub confidence: f64,
    pub affected_transaction_id: Option<Uuid>,
    pub affected_invoice_id: Option<Uuid>,
    pub recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    DuplicateTransaction,
    VatMismatch,
    UnusualAmount,
    UnknownMerchant,
    CurrencyMismatch,
    TimingAnomaly,
    FraudSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity { Low, Medium, High, Critical }

// ─────────────────────────────────────────────
// DECISION EXPLANATION
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionExplanation {
    pub decision: String,
    pub confidence: ConfidenceScore,
    pub evidence: Vec<Evidence>,
    pub audit_event: AuditEvent,
    pub rollback_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: String,
    pub description: String,
    pub weight: f64,
    pub source: String,
}

// ─────────────────────────────────────────────
// CONFIDENCE ENGINE
// ─────────────────────────────────────────────
pub struct ConfidenceEngine {
    auto_book_threshold: f64,
    anomaly_threshold: f64,
}

impl ConfidenceEngine {
    pub fn new() -> Self {
        Self {
            auto_book_threshold: 0.95,
            anomaly_threshold: 0.80,
        }
    }

    /// Match a bank transaction to an invoice.
    /// Returns confidence score with full explanation.
    pub fn match_transaction_to_invoice(
        &self,
        txn: &Transaction,
        invoice: &Invoice,
    ) -> ConfidenceScore {
        let mut score: f64 = 0.0;
        let mut reasons = Vec::new();
        let mut evidence = Vec::new();

        // 1. Exact amount match (weight: 0.40)
        if txn.amount == invoice.amount {
            score += 0.40;
            reasons.push("Exact amount match".to_string());
            evidence.push(("amount", 0.40));
        } else {
            // Check if amount matches with tax included
            let amount_with_tax = invoice.amount + invoice.tax_amount;
            if txn.amount == amount_with_tax {
                score += 0.35;
                reasons.push("Amount matches invoice total including VAT".to_string());
                evidence.push(("amount_with_vat", 0.35));
            } else {
                let diff = (txn.amount - invoice.amount).abs();
                let tolerance = invoice.amount * Decimal::new(2, 2); // 2%
                if diff <= tolerance {
                    score += 0.20;
                    reasons.push(format!("Amount within 2% tolerance ({} vs {})", txn.amount, invoice.amount));
                    evidence.push(("amount_approximate", 0.20));
                }
            }
        }

        // 2. Currency match (weight: 0.10)
        if txn.currency == invoice.currency {
            score += 0.10;
            reasons.push("Currency match".to_string());
        }

        // 3. Vendor/merchant name match (weight: 0.25)
        if let (Some(merchant), Some(vendor)) = (&txn.merchant, &invoice.vendor) {
            let similarity = name_similarity(&merchant.raw_name, &vendor.name);
            let vendor_score = similarity * 0.25;
            score += vendor_score;
            if similarity > 0.8 {
                reasons.push(format!("Vendor name match: {}% similarity", (similarity * 100.0) as u32));
            }
        }

        // 4. Timing proximity (weight: 0.15)
        let days_diff = (txn.timestamp - invoice.issued_at).num_days().abs();
        if days_diff <= 3 {
            score += 0.15;
            reasons.push(format!("Transaction within {} days of invoice date", days_diff));
        } else if days_diff <= 30 {
            let timing_score = 0.10 * (1.0 - (days_diff as f64 / 30.0));
            score += timing_score;
            reasons.push(format!("Transaction {} days from invoice date", days_diff));
        }

        // 5. Invoice reference in transaction (weight: 0.10)
        if let (Some(merchant), Some(ext_id)) = (&txn.merchant, &invoice.external_id) {
            if merchant.raw_name.contains(ext_id.as_str()) {
                score += 0.10;
                reasons.push("Invoice reference found in transaction".to_string());
            }
        }

        let capped = score.min(1.0);

        ConfidenceScore {
            score: capped,
            reasons,
            rollback_possible: true,
            requires_human_review: capped < self.auto_book_threshold,
            auto_book_threshold: self.auto_book_threshold,
        }
    }

    /// Detect anomalies in a transaction.
    pub fn detect_anomaly(&self, txn: &Transaction, history: &[Transaction]) -> Option<Anomaly> {
        // 1. Duplicate detection
        let duplicates: Vec<_> = history.iter().filter(|h| {
            h.id != txn.id
                && h.amount == txn.amount
                && h.currency == txn.currency
                && (h.timestamp - txn.timestamp).num_hours().abs() < 24
        }).collect();

        if !duplicates.is_empty() {
            return Some(Anomaly {
                id: uuid::Uuid::new_v4(),
                anomaly_type: AnomalyType::DuplicateTransaction,
                severity: AnomalySeverity::High,
                description: format!("Possible duplicate: {} {} within 24h", txn.amount, txn.currency),
                confidence: 0.92,
                affected_transaction_id: Some(txn.id),
                affected_invoice_id: None,
                recommended_action: "Review and confirm uniqueness before booking".to_string(),
            });
        }

        // 2. Unusual amount (3x historical average)
        if history.len() > 5 {
            let avg: f64 = history.iter()
                .filter_map(|h| {
                    let f: f64 = h.amount.try_into().ok()?;
                    Some(f)
                })
                .sum::<f64>() / history.len() as f64;
            
            let txn_f: f64 = txn.amount.try_into().unwrap_or(0.0);
            if txn_f > avg * 3.0 {
                return Some(Anomaly {
                    id: uuid::Uuid::new_v4(),
                    anomaly_type: AnomalyType::UnusualAmount,
                    severity: AnomalySeverity::Medium,
                    description: format!("Amount {} is {}x higher than average {:.0}", txn.amount, (txn_f / avg) as u32, avg),
                    confidence: 0.85,
                    affected_transaction_id: Some(txn.id),
                    affected_invoice_id: None,
                    recommended_action: "Verify with approver before booking".to_string(),
                });
            }
        }

        None
    }

    /// Full decision explanation with audit trail.
    pub fn explain_decision(
        &self,
        decision: &str,
        confidence: ConfidenceScore,
        transaction_id: Option<uuid::Uuid>,
    ) -> DecisionExplanation {
        let audit = AuditEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            actor: "ai-confidence-engine".to_string(),
            action: decision.to_string(),
            reason: confidence.reasons.join("; "),
            confidence: confidence.score,
            source: "reconciler-ai".to_string(),
            payload: serde_json::json!({
                "auto_booked": confidence.should_auto_book(),
                "transaction_id": transaction_id,
            }),
        };

        let evidence: Vec<Evidence> = confidence.reasons.iter().map(|r| Evidence {
            evidence_type: "matching_signal".to_string(),
            description: r.clone(),
            weight: confidence.score / confidence.reasons.len() as f64,
            source: "reconciler-ai".to_string(),
        }).collect();

        let rollback = if confidence.rollback_possible {
            Some(format!("POST /decisions/{}/rollback", transaction_id.unwrap_or_default()))
        } else {
            None
        };

        DecisionExplanation {
            decision: decision.to_string(),
            confidence,
            evidence,
            audit_event: audit,
            rollback_instructions: rollback,
        }
    }
}

// Simple normalized string similarity (0.0–1.0)
fn name_similarity(a: &str, b: &str) -> f64 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    
    if a == b { return 1.0; }
    if a.contains(&b) || b.contains(&a) { return 0.9; }
    
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}
