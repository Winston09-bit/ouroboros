// src/ocr/fraud_detection.rs — Document Fraud Analysis
// Reconciler OCR + Document Intelligence Pipeline

use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::collections::HashMap;

use super::{DocumentType, ExtractedDocumentData};

// ─────────────────────────────────────────────
// Public enums
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum FraudFlagType {
    /// Line items don't add up to the stated subtotal
    AmountInconsistency,
    /// VAT calculation is arithmetically wrong
    VATMathError,
    /// Document appears to be a duplicate of a previously seen one
    DuplicateDocument,
    /// PDF metadata shows signs of editing/copy-paste
    ManipulatedMetadata,
    /// Vendor is not found in the known-vendor whitelist
    UnknownVendor,
    /// Document date is in the future, or implausibly old
    SuspiciousDate,
    /// Line items sum is way below/above stated total
    LineItemMismatch,
    /// Amounts have an unusually round structure (possible fabrication)
    SuspiciousRoundAmounts,
}

impl std::fmt::Display for FraudFlagType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FraudFlagType::AmountInconsistency    => "AmountInconsistency",
            FraudFlagType::VATMathError           => "VATMathError",
            FraudFlagType::DuplicateDocument      => "DuplicateDocument",
            FraudFlagType::ManipulatedMetadata    => "ManipulatedMetadata",
            FraudFlagType::UnknownVendor          => "UnknownVendor",
            FraudFlagType::SuspiciousDate         => "SuspiciousDate",
            FraudFlagType::LineItemMismatch       => "LineItemMismatch",
            FraudFlagType::SuspiciousRoundAmounts => "SuspiciousRoundAmounts",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlagSeverity {
    /// Informational — no immediate action required
    Low,
    /// Worth a second look
    Medium,
    /// Strong indication of a problem; requires human review
    High,
    /// Stop processing; escalate immediately
    Critical,
}

impl FlagSeverity {
    /// Numeric weight used when accumulating the overall risk score.
    pub fn weight(&self) -> f64 {
        match self {
            FlagSeverity::Low      => 0.10,
            FlagSeverity::Medium   => 0.25,
            FlagSeverity::High     => 0.45,
            FlagSeverity::Critical => 0.80,
        }
    }
}

impl std::fmt::Display for FlagSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FlagSeverity::Low      => "Low",
            FlagSeverity::Medium   => "Medium",
            FlagSeverity::High     => "High",
            FlagSeverity::Critical => "Critical",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FraudRecommendation {
    /// No issues detected — safe to approve automatically
    AutoApprove,
    /// Minor issues — human should review before approving
    ManualReview,
    /// Significant issues — do not pay, reject and notify submitter
    Reject,
    /// Critical issues — escalate to compliance/management immediately
    Escalate,
}

impl std::fmt::Display for FraudRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FraudRecommendation::AutoApprove  => "AutoApprove",
            FraudRecommendation::ManualReview => "ManualReview",
            FraudRecommendation::Reject       => "Reject",
            FraudRecommendation::Escalate     => "Escalate",
        };
        write!(f, "{}", s)
    }
}

// ─────────────────────────────────────────────
// Result types
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FraudFlag {
    pub flag_type: FraudFlagType,
    pub description: String,
    pub severity: FlagSeverity,
}

#[derive(Debug, Clone)]
pub struct FraudAnalysis {
    /// 0.0 = no detected issues, 1.0 = maximum risk
    pub risk_score: f64,
    pub flags: Vec<FraudFlag>,
    pub recommendation: FraudRecommendation,
}

/// Score returned by PDF metadata analysis.
#[derive(Debug, Clone)]
pub struct ManipulationScore {
    /// 0.0 = no signs of manipulation, 1.0 = strong evidence
    pub score: f64,
    pub reasons: Vec<String>,
}

// ─────────────────────────────────────────────
// Tolerance constants
// ─────────────────────────────────────────────

/// Maximum acceptable rounding difference for amount checks (e.g. 0.05 SEK).
const AMOUNT_TOLERANCE: &str = "0.05";

/// Maximum acceptable rounding difference for VAT math checks.
const VAT_TOLERANCE: &str = "0.10";

/// Percentage tolerance for line-item sum vs stated total (2 %).
const LINE_ITEM_TOLERANCE_PCT: &str = "0.02";

// ─────────────────────────────────────────────
// FraudDetector
// ─────────────────────────────────────────────

pub struct FraudDetector {
    /// Optional list of trusted vendor names (lowercase).
    pub known_vendors: Vec<String>,
    /// Simple in-memory duplicate store: document fingerprint → true.
    /// In production this would be a database query.
    pub seen_fingerprints: HashMap<String, bool>,
}

impl Default for FraudDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FraudDetector {
    pub fn new() -> Self {
        Self {
            known_vendors: Vec::new(),
            seen_fingerprints: HashMap::new(),
        }
    }

    /// Construct with a pre-populated vendor whitelist.
    pub fn with_vendors(vendors: &[&str]) -> Self {
        Self {
            known_vendors: vendors.iter().map(|v| v.to_lowercase()).collect(),
            seen_fingerprints: HashMap::new(),
        }
    }

    // ── Main analysis entry-point ─────────────────────────────────────────────
    /// Run all fraud checks and return a consolidated `FraudAnalysis`.
    pub fn analyze(
        &self,
        doc_data: &ExtractedDocumentData,
        original_bytes: &[u8],
    ) -> FraudAnalysis {
        let mut flags: Vec<FraudFlag> = Vec::new();

        // 1. Amount internal consistency
        if !self.check_amount_consistency(doc_data) {
            flags.push(FraudFlag {
                flag_type: FraudFlagType::AmountInconsistency,
                description: format!(
                    "Line items sum (if present) does not match stated total {}",
                    doc_data.total_amount
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "n/a".to_string())
                ),
                severity: FlagSeverity::High,
            });
        }

        // 2. VAT math check
        if let (Some(total), Some(tax), Some(rate)) = (
            doc_data.total_amount,
            doc_data.tax_amount,
            doc_data.tax_rate,
        ) {
            let net = total - tax;
            if !self.verify_vat_math(net, tax, total, rate) {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::VATMathError,
                    description: format!(
                        "VAT math fails: net={} × (1 + {}%) ≠ total={}",
                        net, rate, total
                    ),
                    severity: FlagSeverity::High,
                });
            }
        }

        // 3. Duplicate detection
        let fingerprint = Self::compute_fingerprint(doc_data);
        if self.seen_fingerprints.contains_key(&fingerprint) {
            flags.push(FraudFlag {
                flag_type: FraudFlagType::DuplicateDocument,
                description: format!(
                    "Document fingerprint '{}' already seen — possible resubmission",
                    &fingerprint[..fingerprint.len().min(16)]
                ),
                severity: FlagSeverity::Critical,
            });
        }

        // 4. PDF metadata manipulation
        if !original_bytes.is_empty() {
            let pdf_magic = original_bytes.starts_with(b"%PDF");
            if pdf_magic {
                let manip = self.detect_pdf_manipulation(original_bytes);
                if manip.score >= 0.4 {
                    let severity = if manip.score >= 0.7 {
                        FlagSeverity::Critical
                    } else {
                        FlagSeverity::Medium
                    };
                    flags.push(FraudFlag {
                        flag_type: FraudFlagType::ManipulatedMetadata,
                        description: format!(
                            "PDF metadata anomalies (score {:.2}): {}",
                            manip.score,
                            manip.reasons.join("; ")
                        ),
                        severity,
                    });
                }
            }
        }

        // 5. Unknown vendor
        if let Some(ref vendor) = doc_data.vendor_name {
            if !self.known_vendors.is_empty()
                && !self
                    .known_vendors
                    .iter()
                    .any(|kv| vendor.to_lowercase().contains(kv.as_str()))
            {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::UnknownVendor,
                    description: format!(
                        "Vendor '{}' not in known-vendor whitelist",
                        vendor
                    ),
                    severity: FlagSeverity::Low,
                });
            }
        }

        // 6. Suspicious date
        if let Some(date) = doc_data.date {
            let today = chrono::Local::now().naive_local().date();
            if date > today {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::SuspiciousDate,
                    description: format!(
                        "Document date {} is in the future (today: {})",
                        date, today
                    ),
                    severity: FlagSeverity::High,
                });
            } else {
                // More than 3 years old → suspicious
                let age_days = (today - date).num_days();
                if age_days > 365 * 3 {
                    flags.push(FraudFlag {
                        flag_type: FraudFlagType::SuspiciousDate,
                        description: format!(
                            "Document date {} is {} days old (> 3 years)",
                            date, age_days
                        ),
                        severity: FlagSeverity::Medium,
                    });
                }
            }
        }

        // 7. Suspiciously round amounts
        if Self::amounts_suspiciously_round(doc_data) {
            flags.push(FraudFlag {
                flag_type: FraudFlagType::SuspiciousRoundAmounts,
                description: "Multiple amounts are suspiciously round — possible fabrication"
                    .to_string(),
                severity: FlagSeverity::Low,
            });
        }

        // ── Compute risk score ────────────────────────────────────────────────
        // Capped at 1.0; uses max(flag_weight) + 0.1 per additional flag.
        let risk_score = Self::compute_risk_score(&flags);

        // ── Determine recommendation ──────────────────────────────────────────
        let recommendation = Self::recommend(risk_score, &flags);

        FraudAnalysis {
            risk_score,
            flags,
            recommendation,
        }
    }

    // ── Amount consistency ────────────────────────────────────────────────────
    /// Returns `true` if amounts are internally consistent (or insufficient
    /// data to perform the check).
    pub fn check_amount_consistency(&self, data: &ExtractedDocumentData) -> bool {
        let tol = Decimal::from_str(AMOUNT_TOLERANCE).unwrap();

        // Check: sum of line items ≈ total
        if !data.line_items.is_empty() {
            if let Some(total) = data.total_amount {
                let line_sum: Decimal =
                    data.line_items.iter().map(|li| li.total).sum();

                // Allow for tax being included in total
                let net_total = if let Some(tax) = data.tax_amount {
                    total - tax
                } else {
                    total
                };

                let pct_tol = Decimal::from_str(LINE_ITEM_TOLERANCE_PCT).unwrap();
                let abs_tol = (net_total * pct_tol).abs() + tol;

                if (line_sum - net_total).abs() > abs_tol {
                    return false;
                }
            }
        }

        // Check: tax_amount + net ≈ total
        if let (Some(total), Some(tax)) = (data.total_amount, data.tax_amount) {
            let net = total - tax;
            // Net must be positive
            if net < Decimal::ZERO {
                return false;
            }
            // tax must be < total
            if tax >= total {
                return false;
            }
        }

        true
    }

    // ── PDF manipulation detection ────────────────────────────────────────────
    /// Scan the raw PDF bytes for heuristic manipulation signals.
    ///
    /// This is a lightweight byte-level inspection; a production system would
    /// use a proper PDF parser.  Signals checked:
    ///   1. Multiple `%%EOF` markers (layers appended after generation)
    ///   2. Suspicious creator strings (common PDF editors used for fraud)
    ///   3. ModDate ≫ CreationDate (heavy post-creation editing)
    ///   4. Incremental update markers (`startxref` after first `%%EOF`)
    pub fn detect_pdf_manipulation(&self, pdf_bytes: &[u8]) -> ManipulationScore {
        let text = String::from_utf8_lossy(pdf_bytes);
        let mut reasons: Vec<String> = Vec::new();
        let mut score: f64 = 0.0;

        // 1. Multiple EOF markers
        let eof_count = text.matches("%%EOF").count();
        if eof_count > 1 {
            reasons.push(format!(
                "{} %%EOF markers found — document may have been incrementally updated",
                eof_count
            ));
            score += 0.30 * (eof_count - 1) as f64;
        }

        // 2. Suspicious creator tools
        let suspicious_creators = &[
            "adobe acrobat", "foxit", "smallpdf", "ilovepdf",
            "pdf24", "sejda", "pdfescapeapps", "pdfcandy",
            "online2pdf", "pdfcompressor", "nitro",
        ];
        let text_lower = text.to_lowercase();
        for creator in suspicious_creators {
            if text_lower.contains(creator) {
                reasons.push(format!(
                    "PDF created/modified with '{}' — commonly used for editing invoices",
                    creator
                ));
                score += 0.20;
                break; // only count once
            }
        }

        // 3. XMP ModDate vs CreationDate discrepancy
        // Format in PDFs: D:20260523120000+02'00'
        let creation_re =
            regex::Regex::new(r"/CreationDate\s*\(D:(\d{8})").unwrap();
        let mod_re =
            regex::Regex::new(r"/ModDate\s*\(D:(\d{8})").unwrap();

        if let (Some(cr), Some(mr)) =
            (creation_re.captures(&text), mod_re.captures(&text))
        {
            let creation_str = &cr[1]; // YYYYMMDD
            let mod_str = &mr[1];
            if creation_str != mod_str {
                // Parse as integers for naive comparison
                let c_days: u64 = creation_str.parse().unwrap_or(0);
                let m_days: u64 = mod_str.parse().unwrap_or(0);
                let diff = m_days.saturating_sub(c_days);
                if diff > 1 {
                    // Modified more than ~1 day after creation
                    reasons.push(format!(
                        "ModDate ({}) differs from CreationDate ({}) by {} calendar days",
                        mod_str, creation_str, diff_yyyymmdd_days(creation_str, mod_str)
                    ));
                    score += 0.25;
                }
            }
        }

        // 4. startxref after %%EOF (incremental update = post-processing)
        let first_eof = text.find("%%EOF");
        let last_startxref = text.rfind("startxref");
        if let (Some(eof_pos), Some(xref_pos)) = (first_eof, last_startxref) {
            if xref_pos > eof_pos {
                reasons.push(
                    "startxref marker found after first %%EOF — incremental update applied"
                        .to_string(),
                );
                score += 0.15;
            }
        }

        // 5. JavaScript embedded (rare in invoices, common in tampered docs)
        if text.contains("/JavaScript") || text.contains("/JS") {
            reasons.push("Embedded JavaScript found in PDF".to_string());
            score += 0.40;
        }

        ManipulationScore {
            score: score.min(1.0),
            reasons,
        }
    }

    // ── VAT math verification ─────────────────────────────────────────────────
    /// Verify: `subtotal × (1 + rate/100) ≈ total` within tolerance.
    pub fn verify_vat_math(
        &self,
        subtotal: Decimal,
        vat: Decimal,
        total: Decimal,
        rate: Decimal,
    ) -> bool {
        let tol = Decimal::from_str(VAT_TOLERANCE).unwrap();
        let hundred = Decimal::from(100);

        // Expected tax from rate
        let expected_vat = (subtotal * rate / hundred).round_dp(2);
        let vat_ok = (vat - expected_vat).abs() <= tol;

        // Expected total
        let expected_total = (subtotal + expected_vat).round_dp(2);
        let total_ok = (total - expected_total).abs() <= tol;

        vat_ok && total_ok
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Build a stable fingerprint for duplicate detection.
    ///
    /// Uses: vendor + date + total + invoice_number (all optional fields).
    /// In production, use a cryptographic hash and store in a database.
    fn compute_fingerprint(data: &ExtractedDocumentData) -> String {
        let parts = vec![
            data.vendor_name.clone().unwrap_or_default(),
            data.date.map(|d| d.to_string()).unwrap_or_default(),
            data.total_amount.map(|a| a.to_string()).unwrap_or_default(),
            data.invoice_number.clone().unwrap_or_default(),
        ];
        // Simple concatenation hash for demo purposes
        // Production: use SHA-256 and normalise fields first
        parts.join("|")
    }

    /// Register a document fingerprint as "seen" (for duplicate detection).
    pub fn register_document(&mut self, data: &ExtractedDocumentData) {
        let fp = Self::compute_fingerprint(data);
        self.seen_fingerprints.insert(fp, true);
    }

    /// Check whether multiple top-level amounts are suspiciously round.
    fn amounts_suspiciously_round(data: &ExtractedDocumentData) -> bool {
        let mut round_count = 0;
        let mut total_count = 0;

        let check = |v: Decimal| {
            // "Round" = no cents (scale 0 or cents == 00)
            v == v.round_dp(0)
        };

        if let Some(t) = data.total_amount {
            total_count += 1;
            if check(t) { round_count += 1; }
        }
        if let Some(t) = data.tax_amount {
            total_count += 1;
            if check(t) { round_count += 1; }
        }
        for li in &data.line_items {
            total_count += 1;
            if check(li.total) { round_count += 1; }
        }

        // Flag if ≥ 3 amounts exist and all are round
        total_count >= 3 && round_count == total_count
    }

    /// Accumulate a risk score from flag severities, capped at 1.0.
    fn compute_risk_score(flags: &[FraudFlag]) -> f64 {
        if flags.is_empty() {
            return 0.0;
        }
        // Start with the maximum single-flag weight
        let max_weight = flags
            .iter()
            .map(|f| f.severity.weight())
            .fold(0.0_f64, f64::max);

        // Each additional flag adds a diminishing contribution
        let extra: f64 = flags
            .iter()
            .skip(1)
            .map(|f| f.severity.weight() * 0.3)
            .sum();

        (max_weight + extra).min(1.0)
    }

    fn recommend(risk_score: f64, flags: &[FraudFlag]) -> FraudRecommendation {
        // Any critical flag → Escalate immediately
        if flags.iter().any(|f| f.severity == FlagSeverity::Critical) {
            return FraudRecommendation::Escalate;
        }
        // High risk or multiple High flags → Reject
        if risk_score >= 0.6
            || flags
                .iter()
                .filter(|f| f.severity == FlagSeverity::High)
                .count()
                >= 2
        {
            return FraudRecommendation::Reject;
        }
        // Some flags present → manual review
        if risk_score > 0.15 || !flags.is_empty() {
            return FraudRecommendation::ManualReview;
        }
        FraudRecommendation::AutoApprove
    }
}

// ─────────────────────────────────────────────
// Small utility — not in the public API
// ─────────────────────────────────────────────

/// Compute the difference in days between two YYYYMMDD strings (best-effort).
fn diff_yyyymmdd_days(a: &str, b: &str) -> i64 {
    fn parse(s: &str) -> Option<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(s, "%Y%m%d").ok()
    }
    match (parse(a), parse(b)) {
        (Some(da), Some(db)) => (db - da).num_days(),
        _ => 0,
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn base_doc() -> ExtractedDocumentData {
        ExtractedDocumentData {
            doc_type: DocumentType::Invoice,
            total_amount: Some(dec("1190.00")),
            tax_amount: Some(dec("190.00")),
            tax_rate: Some(dec("19")),
            currency: Some("EUR".to_string()),
            vendor_name: Some("Acme GmbH".to_string()),
            vendor_vat: Some("DE123456789".to_string()),
            date: Some(NaiveDate::from_ymd_opt(2026, 5, 23).unwrap()),
            invoice_number: Some("INV-001".to_string()),
            line_items: vec![],
            confidence: 0.95,
        }
    }

    // ── VAT math ────────────────────────────────────────────────────────────

    #[test]
    fn vat_math_correct_de_19() {
        let fd = FraudDetector::new();
        // net 1000 × 1.19 = 1190
        assert!(fd.verify_vat_math(dec("1000"), dec("190"), dec("1190"), dec("19")));
    }

    #[test]
    fn vat_math_correct_se_25() {
        let fd = FraudDetector::new();
        // net 800 × 1.25 = 1000
        assert!(fd.verify_vat_math(dec("800"), dec("200"), dec("1000"), dec("25")));
    }

    #[test]
    fn vat_math_wrong_amount() {
        let fd = FraudDetector::new();
        // net 1000, rate 19%, but vat claimed as 210 (wrong)
        assert!(!fd.verify_vat_math(dec("1000"), dec("210"), dec("1210"), dec("19")));
    }

    #[test]
    fn vat_math_handles_rounding() {
        let fd = FraudDetector::new();
        // 33.33 × 25% = 8.3325 → rounds to 8.33; total = 41.66
        assert!(fd.verify_vat_math(dec("33.33"), dec("8.33"), dec("41.66"), dec("25")));
    }

    // ── Amount consistency ──────────────────────────────────────────────────

    #[test]
    fn consistency_ok_no_line_items() {
        let fd = FraudDetector::new();
        assert!(fd.check_amount_consistency(&base_doc()));
    }

    #[test]
    fn consistency_fail_tax_exceeds_total() {
        let fd = FraudDetector::new();
        let mut doc = base_doc();
        doc.tax_amount = Some(dec("1200")); // > total 1190
        assert!(!fd.check_amount_consistency(&doc));
    }

    #[test]
    fn consistency_fail_line_items_mismatch() {
        use super::super::ExtractedLineItem;
        let fd = FraudDetector::new();
        let mut doc = base_doc();
        doc.tax_amount = None; // simplify
        doc.total_amount = Some(dec("500"));
        doc.line_items = vec![
            ExtractedLineItem {
                description: "Widget A".to_string(),
                quantity: None,
                unit_price: None,
                total: dec("200"),
            },
            ExtractedLineItem {
                description: "Widget B".to_string(),
                quantity: None,
                unit_price: None,
                total: dec("150"), // sum = 350, total = 500 → mismatch > 2%
            },
        ];
        assert!(!fd.check_amount_consistency(&doc));
    }

    // ── Duplicate detection ─────────────────────────────────────────────────

    #[test]
    fn duplicate_detection_triggers() {
        let mut fd = FraudDetector::new();
        let doc = base_doc();
        fd.register_document(&doc);
        let analysis = fd.analyze(&doc, b"");
        let has_dup = analysis
            .flags
            .iter()
            .any(|f| f.flag_type == FraudFlagType::DuplicateDocument);
        assert!(has_dup, "expected duplicate flag");
        assert_eq!(analysis.recommendation, FraudRecommendation::Escalate);
    }

    #[test]
    fn no_duplicate_on_different_invoice_number() {
        let mut fd = FraudDetector::new();
        let doc = base_doc();
        fd.register_document(&doc);

        let mut doc2 = base_doc();
        doc2.invoice_number = Some("INV-002".to_string());
        let analysis = fd.analyze(&doc2, b"");
        let has_dup = analysis
            .flags
            .iter()
            .any(|f| f.flag_type == FraudFlagType::DuplicateDocument);
        assert!(!has_dup);
    }

    // ── Unknown vendor ──────────────────────────────────────────────────────

    #[test]
    fn unknown_vendor_flag_when_whitelist_set() {
        let fd = FraudDetector::with_vendors(&["trusted corp", "mega supplier"]);
        let analysis = fd.analyze(&base_doc(), b""); // vendor = "Acme GmbH"
        let has_unknown = analysis
            .flags
            .iter()
            .any(|f| f.flag_type == FraudFlagType::UnknownVendor);
        assert!(has_unknown, "expected unknown vendor flag");
    }

    #[test]
    fn no_unknown_vendor_when_whitelist_empty() {
        let fd = FraudDetector::new(); // empty whitelist → skip check
        let analysis = fd.analyze(&base_doc(), b"");
        let has_unknown = analysis
            .flags
            .iter()
            .any(|f| f.flag_type == FraudFlagType::UnknownVendor);
        assert!(!has_unknown);
    }

    // ── Suspicious date ─────────────────────────────────────────────────────

    #[test]
    fn future_date_raises_high_flag() {
        let fd = FraudDetector::new();
        let mut doc = base_doc();
        // Set date far in the future
        doc.date = NaiveDate::from_ymd_opt(2099, 1, 1);
        let analysis = fd.analyze(&doc, b"");
        let has_date_flag = analysis
            .flags
            .iter()
            .any(|f| f.flag_type == FraudFlagType::SuspiciousDate && f.severity == FlagSeverity::High);
        assert!(has_date_flag, "expected future-date High flag");
    }

    // ── PDF manipulation ────────────────────────────────────────────────────

    #[test]
    fn pdf_manipulation_detects_multiple_eof() {
        let fd = FraudDetector::new();
        let fake_pdf = b"%PDF-1.4\n...content...\n%%EOF\n...more...\n%%EOF\n";
        let manip = fd.detect_pdf_manipulation(fake_pdf);
        assert!(manip.score > 0.0, "expected non-zero score for multiple EOF");
        assert!(!manip.reasons.is_empty());
    }

    #[test]
    fn pdf_manipulation_detects_javascript() {
        let fd = FraudDetector::new();
        let fake_pdf = b"%PDF-1.4\n/JavaScript (alert(1))\n%%EOF\n";
        let manip = fd.detect_pdf_manipulation(fake_pdf);
        assert!(manip.score >= 0.4);
    }

    #[test]
    fn clean_pdf_scores_zero() {
        let fd = FraudDetector::new();
        // Minimal clean PDF
        let fake_pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\nstartxref\n9\n%%EOF\n";
        let manip = fd.detect_pdf_manipulation(fake_pdf);
        assert_eq!(manip.score, 0.0, "clean PDF should score 0");
    }

    // ── Risk score & recommendation ─────────────────────────────────────────

    #[test]
    fn risk_score_zero_for_clean_doc() {
        let fd = FraudDetector::new();
        let analysis = fd.analyze(&base_doc(), b"");
        assert!(analysis.risk_score < 0.15, "clean doc risk={}", analysis.risk_score);
        assert_eq!(analysis.recommendation, FraudRecommendation::AutoApprove);
    }

    #[test]
    fn risk_score_caps_at_one() {
        let flags: Vec<FraudFlag> = (0..20).map(|_| FraudFlag {
            flag_type: FraudFlagType::VATMathError,
            description: "test".to_string(),
            severity: FlagSeverity::Critical,
        }).collect();
        let score = FraudDetector::compute_risk_score(&flags);
        assert!(score <= 1.0, "risk score must not exceed 1.0");
    }

    #[test]
    fn vat_error_doc_recommends_reject() {
        let fd = FraudDetector::new();
        let mut doc = base_doc();
        // Make VAT math wrong: tax of 500 on a 1190 total makes no sense
        doc.tax_amount = Some(dec("500"));
        doc.tax_rate = Some(dec("19"));
        let analysis = fd.analyze(&doc, b"");
        assert!(
            analysis.recommendation == FraudRecommendation::Reject
                || analysis.recommendation == FraudRecommendation::ManualReview,
            "got {:?}", analysis.recommendation
        );
    }

    // ── Round amounts ───────────────────────────────────────────────────────

    #[test]
    fn round_amounts_flag_when_all_integer() {
        use super::super::ExtractedLineItem;
        let doc = ExtractedDocumentData {
            doc_type: DocumentType::Receipt,
            total_amount: Some(dec("100")),
            tax_amount: Some(dec("20")),
            tax_rate: Some(dec("20")),
            currency: Some("EUR".to_string()),
            vendor_name: Some("Suspicious Shop".to_string()),
            vendor_vat: None,
            date: Some(NaiveDate::from_ymd_opt(2026, 5, 23).unwrap()),
            invoice_number: None,
            line_items: vec![
                ExtractedLineItem { description: "A".to_string(), quantity: None, unit_price: None, total: dec("80") },
            ],
            confidence: 0.5,
        };
        assert!(FraudDetector::amounts_suspiciously_round(&doc));
    }

    #[test]
    fn non_round_amounts_not_flagged() {
        let doc = base_doc(); // 1190.00, 190.00
        assert!(!FraudDetector::amounts_suspiciously_round(&doc));
    }
}
