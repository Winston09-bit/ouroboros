use chrono::Duration;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::models::{Invoice, Transaction};

// ─────────────────────────────────────────────
// RESULT TYPES
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub transaction_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub status: MatchStatus,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub evidence_state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchStatus {
    Matched,
    Partial,
    Unmatched,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceState {
    Verified,
    Missing,
    Requested,
    Escalated,
}

// ─────────────────────────────────────────────
// SCORING CONSTANTS
// ─────────────────────────────────────────────

const SCORE_EXACT_AMOUNT: f64 = 20.0;
const SCORE_DATE_PROXIMITY: f64 = 15.0;
const SCORE_MERCHANT_MATCH: f64 = 25.0;
const SCORE_REFERENCE_MATCH: f64 = 30.0;
const SCORE_VAT_CONSISTENCY: f64 = 10.0;
const SCORE_MAX: f64 =
    SCORE_EXACT_AMOUNT + SCORE_DATE_PROXIMITY + SCORE_MERCHANT_MATCH + SCORE_REFERENCE_MATCH + SCORE_VAT_CONSISTENCY;

const THRESHOLD_MATCHED: f64 = 0.7;
const THRESHOLD_PARTIAL: f64 = 0.4;

const AMOUNT_TOLERANCE_SEK: f64 = 1.0;
const DATE_WINDOW_DAYS: i64 = 7;
const VAT_RATE_SE: f64 = 0.25;
const MERCHANT_LEVENSHTEIN_THRESHOLD: usize = 4;

// ─────────────────────────────────────────────
// LEVENSHTEIN (inline, no extra dep)
// ─────────────────────────────────────────────

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

fn normalize_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn names_match(a: &str, b: &str) -> bool {
    let na = normalize_name(a);
    let nb = normalize_name(b);

    if na == nb {
        return true;
    }
    if na.contains(&nb) || nb.contains(&na) {
        return true;
    }
    levenshtein(&na, &nb) <= MERCHANT_LEVENSHTEIN_THRESHOLD
}

// ─────────────────────────────────────────────
// CANDIDATE SCORER
// ─────────────────────────────────────────────

struct CandidateScore {
    raw: f64,
    reasons: Vec<String>,
}

fn score_pair(txn: &Transaction, inv: &Invoice) -> CandidateScore {
    let mut raw = 0.0f64;
    let mut reasons: Vec<String> = Vec::new();

    // 1. Exact amount (±1 SEK)
    let amount_diff = (txn.amount - inv.amount).abs();
    let tolerance = Decimal::try_from(AMOUNT_TOLERANCE_SEK).unwrap_or_default();
    if amount_diff <= tolerance {
        raw += SCORE_EXACT_AMOUNT;
        reasons.push(format!(
            "Amount match: txn={} inv={} diff={}",
            txn.amount, inv.amount, amount_diff
        ));
    }

    // 2. Date proximity (within 7 days)
    let delta = (txn.timestamp - inv.issued_at).num_days().abs();
    if delta <= DATE_WINDOW_DAYS {
        let proximity_factor = 1.0 - (delta as f64 / (DATE_WINDOW_DAYS as f64 + 1.0));
        let date_score = SCORE_DATE_PROXIMITY * proximity_factor;
        raw += date_score;
        reasons.push(format!(
            "Date proximity: delta={}d score={:.1}",
            delta, date_score
        ));
    }

    // 3. Merchant / supplier match
    let txn_merchant = txn
        .merchant
        .as_ref()
        .map(|m| {
            m.normalized_name
                .clone()
                .unwrap_or_else(|| m.raw_name.clone())
        })
        .or_else(|| txn.counterparty.as_ref().map(|p| p.name.clone()));

    let inv_supplier = inv.vendor.as_ref().map(|p| {
        p.normalized_name
            .clone()
            .unwrap_or_else(|| p.name.clone())
    });

    if let (Some(tm), Some(is)) = (&txn_merchant, &inv_supplier) {
        if names_match(tm, is) {
            let dist = levenshtein(&normalize_name(tm), &normalize_name(is));
            let merchant_conf = if dist == 0 { 1.0 } else { 1.0 - (dist as f64 / 20.0).min(0.9) };
            let merchant_score = SCORE_MERCHANT_MATCH * merchant_conf;
            raw += merchant_score;
            reasons.push(format!(
                "Merchant match: '{}' ~ '{}' lev={} score={:.1}",
                tm, is, dist, merchant_score
            ));
        }
    }

    // 4. Reference / OCR match
    let txn_ref = txn.external_id.as_deref().or_else(|| {
        // Some adapters surface OCR references on merchant raw_name
        None
    });

    if let Some(tref) = txn_ref {
        let inv_num = inv.invoice_number.trim();
        let tref_norm = tref.trim().to_lowercase();
        let inv_norm = inv_num.to_lowercase();

        if tref_norm == inv_norm || tref_norm.contains(&inv_norm) || inv_norm.contains(&tref_norm) {
            raw += SCORE_REFERENCE_MATCH;
            reasons.push(format!(
                "Reference match: txn_ref='{}' inv_num='{}'",
                tref, inv_num
            ));
        }
    }

    // 5. VAT consistency (25 % SE standard)
    let expected_vat = inv.amount * Decimal::try_from(VAT_RATE_SE).unwrap_or_default();
    let vat_diff = (inv.tax_amount - expected_vat).abs();
    let one_sek = Decimal::try_from(1.0).unwrap_or_default();
    if vat_diff <= one_sek {
        raw += SCORE_VAT_CONSISTENCY;
        reasons.push(format!(
            "VAT consistent: invoice_tax={} expected_25%={}",
            inv.tax_amount, expected_vat
        ));
    }

    CandidateScore { raw, reasons }
}

// ─────────────────────────────────────────────
// MATCHING ENGINE
// ─────────────────────────────────────────────

pub struct MatchingEngine;

impl MatchingEngine {
    pub fn new() -> Self {
        MatchingEngine
    }

    pub fn match_batch(&self, txns: &[Transaction], invoices: &[Invoice]) -> Vec<MatchResult> {
        let mut results: Vec<MatchResult> = Vec::with_capacity(txns.len());

        for txn in txns {
            let best = invoices
                .iter()
                .map(|inv| {
                    let s = score_pair(txn, inv);
                    (inv, s)
                })
                .filter(|(_, s)| s.raw > 0.0)
                .max_by(|(_, a), (_, b)| a.raw.partial_cmp(&b.raw).unwrap_or(std::cmp::Ordering::Equal));

            let result = match best {
                None => {
                    tracing::debug!(
                        transaction_id = %txn.id,
                        "No candidate invoices found – Unmatched"
                    );
                    MatchResult {
                        transaction_id: txn.id,
                        invoice_id: None,
                        status: MatchStatus::Unmatched,
                        confidence: 0.0,
                        reasons: vec!["No scoring candidates found".to_string()],
                        evidence_state: EvidenceState::Missing,
                    }
                }
                Some((inv, score)) => {
                    let confidence = (score.raw / SCORE_MAX).clamp(0.0, 1.0);

                    let (status, evidence_state) = if confidence >= THRESHOLD_MATCHED {
                        (MatchStatus::Matched, EvidenceState::Verified)
                    } else if confidence >= THRESHOLD_PARTIAL {
                        (MatchStatus::Partial, EvidenceState::Requested)
                    } else {
                        (MatchStatus::Unmatched, EvidenceState::Missing)
                    };

                    tracing::info!(
                        transaction_id = %txn.id,
                        invoice_id = %inv.id,
                        confidence,
                        status = ?status,
                        reasons = ?score.reasons,
                        "Match decision"
                    );

                    MatchResult {
                        transaction_id: txn.id,
                        invoice_id: Some(inv.id),
                        status,
                        confidence,
                        reasons: score.reasons,
                        evidence_state,
                    }
                }
            };

            results.push(result);
        }

        results
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────
// TESTS
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_basics() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("ikea", "IKEA"), 0); // after normalization
    }

    #[test]
    fn names_match_exact() {
        assert!(names_match("IKEA Sverige AB", "ikea sverige ab"));
    }

    #[test]
    fn names_match_contains() {
        assert!(names_match("IKEA", "IKEA Sverige AB"));
    }

    #[test]
    fn names_match_fuzzy() {
        assert!(names_match("Staples Nordic", "Staples Nordik")); // lev=1
    }

    #[test]
    fn engine_constructs() {
        let _engine = MatchingEngine::new();
    }
}
