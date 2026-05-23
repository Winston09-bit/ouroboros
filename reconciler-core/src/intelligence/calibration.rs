/// calibration.rs — Adaptive Calibration Engine
///
/// Learns how each company books transactions over time and adapts
/// classification confidence, approval thresholds, and account mappings
/// per-company. Supports rule extraction, export/import, and confidence scoring.

use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::intelligence::merchant_graph::{MerchantIntelligence, normalize_merchant_name};

// ---------------------------------------------------------------------------
// Core domain types
// ---------------------------------------------------------------------------

/// A confirmed booking the engine can learn from.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub merchant: String,
    pub amount: Decimal,
    pub currency: String,
    pub description: Option<String>,
    pub raw_text: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl Transaction {
    pub fn new(merchant: impl Into<String>, amount: Decimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            merchant: merchant.into(),
            amount,
            currency: "SEK".to_string(),
            description: None,
            raw_text: None,
            occurred_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Approval thresholds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ApprovalThresholds {
    /// Below this limit the transaction is booked automatically without approval.
    pub auto_book_limit: Decimal,
    /// Requires one approver.
    pub single_approver_limit: Decimal,
    /// Requires two approvers.
    pub two_approver_limit: Decimal,
    /// Requires the CFO regardless of other approvers.
    pub cfo_required_above: Decimal,
}

impl Default for ApprovalThresholds {
    fn default() -> Self {
        Self {
            auto_book_limit: dec!(5000),
            single_approver_limit: dec!(25000),
            two_approver_limit: dec!(100000),
            cfo_required_above: dec!(500000),
        }
    }
}

impl ApprovalThresholds {
    /// Returns the required approval level for a given amount as a human-readable string.
    pub fn required_approval(&self, amount: Decimal) -> ApprovalLevel {
        if amount <= self.auto_book_limit {
            ApprovalLevel::Auto
        } else if amount <= self.single_approver_limit {
            ApprovalLevel::Single
        } else if amount <= self.two_approver_limit {
            ApprovalLevel::Double
        } else {
            ApprovalLevel::Cfo
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalLevel {
    Auto,
    Single,
    Double,
    Cfo,
}

impl ApprovalLevel {
    pub fn label(&self) -> &str {
        match self {
            Self::Auto => "Automatisk bokning",
            Self::Single => "En godkännare",
            Self::Double => "Två godkännare",
            Self::Cfo => "CFO krävs",
        }
    }
}

// ---------------------------------------------------------------------------
// VAT behaviour
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VatBehavior {
    /// Whether the company is VAT-registered (moms-registrerat).
    pub vat_registered: bool,
    /// Default VAT rate when no merchant-specific rate is found.
    pub default_rate: Decimal,
    /// Company-specific VAT overrides keyed by merchant canonical name.
    pub merchant_overrides: HashMap<String, Decimal>,
    /// Whether intra-EU reverse charge applies by default.
    pub reverse_charge_eu: bool,
}

impl Default for VatBehavior {
    fn default() -> Self {
        Self {
            vat_registered: true,
            default_rate: dec!(0.25),
            merchant_overrides: HashMap::new(),
            reverse_charge_eu: true,
        }
    }
}

impl VatBehavior {
    /// Resolves the effective VAT rate for a merchant name.
    pub fn effective_rate(&self, merchant: &str) -> Decimal {
        let key = normalize_merchant_name(merchant);
        *self.merchant_overrides.get(&key).unwrap_or(&self.default_rate)
    }
}

// ---------------------------------------------------------------------------
// Payment patterns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PaymentPatterns {
    /// Typical payment day-of-month (1–31), if applicable.
    pub typical_payment_day: Option<u8>,
    /// Typical number of days between invoice and payment.
    pub average_payment_days: f64,
    /// Whether the company uses direct debit (autogiro) for common merchants.
    pub uses_direct_debit: bool,
    /// Typical batch size for bulk payments.
    pub batch_frequency_days: u32,
}

impl Default for PaymentPatterns {
    fn default() -> Self {
        Self {
            typical_payment_day: None,
            average_payment_days: 30.0,
            uses_direct_debit: false,
            batch_frequency_days: 7,
        }
    }
}

// ---------------------------------------------------------------------------
// Confidence tuning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConfidenceTuning {
    /// Minimum confidence before auto-booking (default 0.85).
    pub auto_book_threshold: f64,
    /// Confidence above which account codes are shown without warning.
    pub high_confidence_threshold: f64,
    /// Confidence below which a manual review is always triggered.
    pub review_threshold: f64,
    /// Whether to penalise confidence for unusual amounts.
    pub penalise_unusual_amounts: bool,
    /// Extra confidence boost from company-specific observations.
    pub company_specific_boost: f64,
}

impl Default for ConfidenceTuning {
    fn default() -> Self {
        Self {
            auto_book_threshold: 0.85,
            high_confidence_threshold: 0.90,
            review_threshold: 0.50,
            penalise_unusual_amounts: true,
            company_specific_boost: 0.10,
        }
    }
}

// ---------------------------------------------------------------------------
// Learned rule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LearnedRule {
    pub id: Uuid,
    /// Human-readable condition, e.g. "merchant contains 'AWS'"
    pub condition: String,
    /// Human-readable action, e.g. "use account 6540, vat_rate 0.25"
    pub action: String,
    pub confidence: f64,
    pub observations: u32,
    pub created_at: DateTime<Utc>,
    pub last_triggered: DateTime<Utc>,
}

impl LearnedRule {
    fn new(condition: impl Into<String>, action: impl Into<String>, confidence: f64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            condition: condition.into(),
            action: action.into(),
            confidence,
            observations: 1,
            created_at: now,
            last_triggered: now,
        }
    }

    /// Reinforces the rule with a new observation and returns the new confidence.
    pub fn reinforce(&mut self) -> f64 {
        self.observations += 1;
        self.last_triggered = Utc::now();
        // Bayesian-like confidence update: converge toward 1.0 as observations grow
        self.confidence = 1.0 - (1.0 - self.confidence) * (1.0 / (1.0 + self.observations as f64).sqrt());
        self.confidence
    }
}

// ---------------------------------------------------------------------------
// Company calibration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CompanyCalibration {
    pub company_id: String,
    /// merchant canonical key → account code (company-specific overrides)
    pub account_mappings: HashMap<String, String>,
    pub approval_thresholds: ApprovalThresholds,
    pub vat_behavior: VatBehavior,
    pub payment_patterns: PaymentPatterns,
    pub confidence_tuning: ConfidenceTuning,
    pub learned_rules: Vec<LearnedRule>,
    /// 0.0–1.0 overall calibration quality
    pub calibration_score: f64,
    /// Total confirmed transactions seen for this company
    pub total_confirmed: u32,
}

impl CompanyCalibration {
    fn new(company_id: impl Into<String>) -> Self {
        Self {
            company_id: company_id.into(),
            account_mappings: HashMap::new(),
            approval_thresholds: ApprovalThresholds::default(),
            vat_behavior: VatBehavior::default(),
            payment_patterns: PaymentPatterns::default(),
            confidence_tuning: ConfidenceTuning::default(),
            learned_rules: Vec::new(),
            calibration_score: 0.0,
            total_confirmed: 0,
        }
    }

    /// Updates the calibration score based on total_confirmed count.
    fn recalculate_score(&mut self) {
        self.calibration_score = match self.total_confirmed {
            0..=9 => 0.0,
            10..=49 => 0.1 + (self.total_confirmed as f64 - 10.0) / 40.0 * 0.3,
            50..=199 => 0.4 + (self.total_confirmed as f64 - 50.0) / 150.0 * 0.4,
            _ => 0.8 + ((self.total_confirmed as f64 - 200.0) / 800.0 * 0.2).min(0.2),
        };
    }

    /// Upserts a learned rule for the given condition/action pair.
    fn upsert_rule(&mut self, condition: &str, action: &str, base_confidence: f64) {
        if let Some(rule) = self.learned_rules.iter_mut().find(|r| r.condition == condition) {
            rule.reinforce();
        } else {
            self.learned_rules.push(LearnedRule::new(condition, action, base_confidence));
        }
    }

    /// Finds the best matching rule for a transaction (simple substring evaluation).
    fn find_matching_rule(&self, txn: &Transaction) -> Option<&LearnedRule> {
        let merchant_lower = txn.merchant.to_lowercase();
        let description_lower = txn
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase();

        let mut best: Option<&LearnedRule> = None;
        for rule in &self.learned_rules {
            if !rule_matches(rule, &merchant_lower, &description_lower, txn.amount) {
                continue;
            }
            if best.map_or(true, |b: &LearnedRule| rule.confidence > b.confidence) {
                best = Some(rule);
            }
        }
        best
    }
}

// ---------------------------------------------------------------------------
// Rule matching helper
// ---------------------------------------------------------------------------

/// Evaluates whether a rule's condition string matches the transaction.
///
/// Supports patterns:
/// - `merchant contains '<token>'`
/// - `description contains '<token>'`
/// - `amount > <N>` / `amount < <N>`
/// - `merchant is '<name>'`
fn rule_matches(rule: &LearnedRule, merchant: &str, description: &str, amount: Decimal) -> bool {
    let cond = rule.condition.to_lowercase();
    if let Some(rest) = cond.strip_prefix("merchant contains '") {
        let token = rest.trim_end_matches('\'');
        return merchant.contains(token);
    }
    if let Some(rest) = cond.strip_prefix("merchant is '") {
        let token = rest.trim_end_matches('\'');
        return merchant == token;
    }
    if let Some(rest) = cond.strip_prefix("description contains '") {
        let token = rest.trim_end_matches('\'');
        return description.contains(token);
    }
    if let Some(rest) = cond.strip_prefix("amount > ") {
        if let Ok(threshold) = rest.trim().parse::<Decimal>() {
            return amount > threshold;
        }
    }
    if let Some(rest) = cond.strip_prefix("amount < ") {
        if let Ok(threshold) = rest.trim().parse::<Decimal>() {
            return amount < threshold;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Parse account + vat from action string
// ---------------------------------------------------------------------------

/// Extracts account code from action strings like "use account 6540, vat_rate 0.25".
fn parse_account_from_action(action: &str) -> Option<String> {
    let lower = action.to_lowercase();
    if let Some(rest) = lower.find("account ").map(|i| &action[i + 8..]) {
        let code: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !code.is_empty() {
            return Some(code);
        }
    }
    None
}

/// Extracts VAT rate from action strings like "use account 6540, vat_rate 0.25".
fn parse_vat_from_action(action: &str) -> Option<Decimal> {
    let lower = action.to_lowercase();
    if let Some(pos) = lower.find("vat_rate ") {
        let rest = &action[pos + 9..];
        let rate_str: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if let Ok(rate) = rate_str.parse::<Decimal>() {
            return Some(rate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Calibration status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationStatus {
    /// Fewer than 10 confirmed transactions.
    Uncalibrated,
    /// 10–49 confirmed transactions.
    LearningPhase,
    /// 50–199 confirmed transactions.
    Calibrated,
    /// More than 200 confirmed transactions.
    FullyCalibrated,
}

impl CalibrationStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Uncalibrated => "Okalibrerad (< 10 transaktioner)",
            Self::LearningPhase => "Inlärningsfas (10–49 transaktioner)",
            Self::Calibrated => "Kalibrerad (50–199 transaktioner)",
            Self::FullyCalibrated => "Fullständigt kalibrerad (> 200 transaktioner)",
        }
    }
}

// ---------------------------------------------------------------------------
// Calibrated classification output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CalibratedClassification {
    pub account_code: String,
    pub vat_rate: Decimal,
    pub confidence: f64,
    pub based_on_observations: u32,
    pub is_company_specific: bool,
    pub override_reason: Option<String>,
    pub approval_level: ApprovalLevel,
    pub requires_review: bool,
}

// ---------------------------------------------------------------------------
// CalibrationEngine
// ---------------------------------------------------------------------------

pub struct CalibrationEngine {
    company_profiles: HashMap<String, CompanyCalibration>,
    /// Shared merchant intelligence used as a fallback classifier.
    merchant_intel: MerchantIntelligence,
}

impl CalibrationEngine {
    /// Creates a new engine with an empty company registry and pre-loaded merchant intelligence.
    pub fn new() -> Self {
        Self {
            company_profiles: HashMap::new(),
            merchant_intel: MerchantIntelligence::new(),
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Registers a new company and returns a reference to its calibration profile.
    pub fn new_company(&mut self, company_id: &str) -> &CompanyCalibration {
        self.company_profiles
            .entry(company_id.to_string())
            .or_insert_with(|| CompanyCalibration::new(company_id))
    }

    /// Learn from a confirmed booking: updates account mappings, rules and merchant intel.
    pub fn learn_from_booking(&mut self, company_id: &str, txn: &Transaction, account: &str) {
        // Ensure company profile exists
        if !self.company_profiles.contains_key(company_id) {
            self.company_profiles
                .insert(company_id.to_string(), CompanyCalibration::new(company_id));
        }

        // Derive VAT rate from the global merchant intel (best available prior)
        let global_class = self.merchant_intel.classify(&txn.merchant, txn.amount);
        let vat_rate = global_class.vat_rate;

        // Update global merchant intelligence
        self.merchant_intel
            .learn(&txn.merchant, txn.amount, account, vat_rate);

        let cal = self.company_profiles.get_mut(company_id).unwrap();

        // 1. Update account mapping
        let key = normalize_merchant_name(&txn.merchant);
        cal.account_mappings.insert(key.clone(), account.to_string());

        // 2. Upsert a learned rule
        let condition = format!("merchant contains '{}'", key);
        let action = format!("use account {}, vat_rate {}", account, vat_rate);
        cal.upsert_rule(&condition, &action, 0.6);

        // 3. If the transaction text provides additional signals, create description rule too
        if let Some(ref desc) = txn.description {
            let desc_key = desc.to_lowercase();
            if !desc_key.is_empty() && desc_key != key {
                let desc_condition = format!("description contains '{}'", &desc_key[..desc_key.len().min(30)]);
                cal.upsert_rule(&desc_condition, &action, 0.5);
            }
        }

        // 4. Update VAT override for this merchant
        cal.vat_behavior
            .merchant_overrides
            .insert(key, vat_rate);

        // 5. Increment confirmed transaction counter and recalculate calibration score
        cal.total_confirmed += 1;
        cal.recalculate_score();
    }

    /// Returns a calibrated classification for a company + transaction.
    pub fn classify_for_company(
        &self,
        company_id: &str,
        txn: &Transaction,
    ) -> CalibratedClassification {
        // Fall back to global merchant intelligence if no company profile exists
        let Some(cal) = self.company_profiles.get(company_id) else {
            return self.fallback_classification(txn);
        };

        let merchant_key = normalize_merchant_name(&txn.merchant);
        let tuning = &cal.confidence_tuning;

        // ----------------------------------------------------------------
        // Step 1: Company-specific account mapping (highest priority)
        // ----------------------------------------------------------------
        if let Some(account) = cal.account_mappings.get(&merchant_key) {
            let observations = cal
                .learned_rules
                .iter()
                .find(|r| r.condition.contains(&merchant_key))
                .map(|r| r.observations)
                .unwrap_or(1);

            let vat_rate = cal.vat_behavior.effective_rate(&txn.merchant);
            let base_confidence = 0.70
                + (observations as f64 / (observations as f64 + 10.0)) * 0.25
                + tuning.company_specific_boost;
            let confidence = base_confidence.min(0.99);
            let approval = cal
                .approval_thresholds
                .required_approval(txn.amount);

            return CalibratedClassification {
                account_code: account.clone(),
                vat_rate,
                confidence,
                based_on_observations: observations,
                is_company_specific: true,
                override_reason: Some(format!(
                    "Bolagsspecifik mappning ({} observationer)",
                    observations
                )),
                approval_level: approval,
                requires_review: confidence < tuning.review_threshold,
            };
        }

        // ----------------------------------------------------------------
        // Step 2: Learned rule matching
        // ----------------------------------------------------------------
        if let Some(rule) = cal.find_matching_rule(txn) {
            let account = parse_account_from_action(&rule.action)
                .unwrap_or_else(|| "6990".to_string());
            let vat_rate = parse_vat_from_action(&rule.action)
                .unwrap_or_else(|| cal.vat_behavior.default_rate);

            let confidence = (rule.confidence + tuning.company_specific_boost * 0.5).min(0.99);
            let approval = cal.approval_thresholds.required_approval(txn.amount);

            return CalibratedClassification {
                account_code: account,
                vat_rate,
                confidence,
                based_on_observations: rule.observations,
                is_company_specific: true,
                override_reason: Some(format!("Lärd regel: {}", rule.condition)),
                approval_level: approval,
                requires_review: confidence < tuning.review_threshold,
            };
        }

        // ----------------------------------------------------------------
        // Step 3: Global merchant intelligence fallback
        // ----------------------------------------------------------------
        let global = self.merchant_intel.classify(&txn.merchant, txn.amount);
        let mut confidence = global.confidence;

        if tuning.penalise_unusual_amounts && global.amount_is_unusual {
            confidence *= 0.85;
        }
        // Boost slightly from calibration score
        confidence = (confidence + cal.calibration_score * 0.05).min(0.99);

        let approval = cal.approval_thresholds.required_approval(txn.amount);

        CalibratedClassification {
            account_code: global.suggested_account,
            vat_rate: global.vat_rate,
            confidence,
            based_on_observations: 0,
            is_company_specific: false,
            override_reason: None,
            approval_level: approval,
            requires_review: confidence < tuning.review_threshold,
        }
    }

    /// Returns the calibration status for a company.
    pub fn calibration_confidence(&self, company_id: &str) -> CalibrationStatus {
        let total = self
            .company_profiles
            .get(company_id)
            .map(|c| c.total_confirmed)
            .unwrap_or(0);

        match total {
            0..=9 => CalibrationStatus::Uncalibrated,
            10..=49 => CalibrationStatus::LearningPhase,
            50..=199 => CalibrationStatus::Calibrated,
            _ => CalibrationStatus::FullyCalibrated,
        }
    }

    /// Exports all learned rules for a company.
    pub fn export_rules(&self, company_id: &str) -> Vec<LearnedRule> {
        self.company_profiles
            .get(company_id)
            .map(|c| c.learned_rules.clone())
            .unwrap_or_default()
    }

    /// Returns a reference to a company calibration, if it exists.
    pub fn company_calibration(&self, company_id: &str) -> Option<&CompanyCalibration> {
        self.company_profiles.get(company_id)
    }

    /// Mutable access to a company calibration.
    pub fn company_calibration_mut(&mut self, company_id: &str) -> Option<&mut CompanyCalibration> {
        self.company_profiles.get_mut(company_id)
    }

    /// Updates approval thresholds for a specific company.
    pub fn set_approval_thresholds(
        &mut self,
        company_id: &str,
        thresholds: ApprovalThresholds,
    ) {
        if let Some(cal) = self.company_profiles.get_mut(company_id) {
            cal.approval_thresholds = thresholds;
        }
    }

    /// Returns a snapshot of all company calibration scores.
    pub fn all_calibration_scores(&self) -> HashMap<String, f64> {
        self.company_profiles
            .iter()
            .map(|(k, v)| (k.clone(), v.calibration_score))
            .collect()
    }

    /// Bulk-imports learned rules for a company (e.g., restored from DB).
    pub fn import_rules(&mut self, company_id: &str, rules: Vec<LearnedRule>) {
        let cal = self
            .company_profiles
            .entry(company_id.to_string())
            .or_insert_with(|| CompanyCalibration::new(company_id));
        for rule in rules {
            // Avoid duplicates: replace if condition already exists
            if let Some(existing) = cal.learned_rules.iter_mut().find(|r| r.condition == rule.condition) {
                *existing = rule;
            } else {
                cal.learned_rules.push(rule);
            }
        }
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn fallback_classification(&self, txn: &Transaction) -> CalibratedClassification {
        let global = self.merchant_intel.classify(&txn.merchant, txn.amount);
        CalibratedClassification {
            account_code: global.suggested_account,
            vat_rate: global.vat_rate,
            confidence: global.confidence * 0.8,
            based_on_observations: 0,
            is_company_specific: false,
            override_reason: None,
            approval_level: ApprovalLevel::Auto,
            requires_review: global.confidence < 0.5,
        }
    }
}

impl Default for CalibrationEngine {
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

    fn sample_txn(merchant: &str, amount: Decimal) -> Transaction {
        Transaction::new(merchant, amount)
    }

    #[test]
    fn test_new_company_returns_fresh_calibration() {
        let mut engine = CalibrationEngine::new();
        let cal = engine.new_company("bolag-001");
        assert_eq!(cal.company_id, "bolag-001");
        assert_eq!(cal.total_confirmed, 0);
        assert!((cal.calibration_score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_uncalibrated_status() {
        let mut engine = CalibrationEngine::new();
        engine.new_company("bolag-002");
        assert_eq!(
            engine.calibration_confidence("bolag-002"),
            CalibrationStatus::Uncalibrated
        );
    }

    #[test]
    fn test_learn_from_booking_updates_mapping() {
        let mut engine = CalibrationEngine::new();
        let txn = sample_txn("AWS", dec!(5000));
        engine.learn_from_booking("bolag-abc", &txn, "6540");
        let cal = engine.company_profiles.get("bolag-abc").unwrap();
        assert_eq!(cal.account_mappings.get("amazon web services").map(|s| s.as_str()), Some("6540"));
        assert_eq!(cal.total_confirmed, 1);
    }

    #[test]
    fn test_classify_company_specific_mapping() {
        let mut engine = CalibrationEngine::new();
        let txn = sample_txn("Telia", dec!(600));
        engine.learn_from_booking("bolag-tel", &txn, "6210");
        let result = engine.classify_for_company("bolag-tel", &txn);
        assert!(result.is_company_specific);
        assert_eq!(result.account_code, "6210");
        assert!(result.confidence > 0.7);
    }

    #[test]
    fn test_classify_falls_back_to_merchant_intel() {
        let engine = CalibrationEngine::new();
        let txn = sample_txn("Vattenfall", dec!(2000));
        let result = engine.classify_for_company("okänt-bolag", &txn);
        assert!(!result.is_company_specific);
        assert_eq!(result.account_code, "6220");
    }

    #[test]
    fn test_approval_level_auto() {
        let thresholds = ApprovalThresholds::default();
        assert_eq!(thresholds.required_approval(dec!(1000)), ApprovalLevel::Auto);
    }

    #[test]
    fn test_approval_level_single() {
        let thresholds = ApprovalThresholds::default();
        assert_eq!(
            thresholds.required_approval(dec!(10000)),
            ApprovalLevel::Single
        );
    }

    #[test]
    fn test_approval_level_double() {
        let thresholds = ApprovalThresholds::default();
        assert_eq!(
            thresholds.required_approval(dec!(50000)),
            ApprovalLevel::Double
        );
    }

    #[test]
    fn test_approval_level_cfo() {
        let thresholds = ApprovalThresholds::default();
        assert_eq!(
            thresholds.required_approval(dec!(600000)),
            ApprovalLevel::Cfo
        );
    }

    #[test]
    fn test_calibration_status_learning_phase() {
        let mut engine = CalibrationEngine::new();
        engine.new_company("bolag-lrn");
        // Simulate 15 confirmed bookings
        for i in 0..15 {
            let txn = sample_txn(&format!("Merchant{}", i), dec!(100));
            engine.learn_from_booking("bolag-lrn", &txn, "6540");
        }
        assert_eq!(
            engine.calibration_confidence("bolag-lrn"),
            CalibrationStatus::LearningPhase
        );
    }

    #[test]
    fn test_calibration_status_calibrated() {
        let mut engine = CalibrationEngine::new();
        engine.new_company("bolag-cal");
        for i in 0..60 {
            let txn = sample_txn(&format!("Vendor{}", i % 10), dec!(200));
            engine.learn_from_booking("bolag-cal", &txn, "6540");
        }
        assert_eq!(
            engine.calibration_confidence("bolag-cal"),
            CalibrationStatus::Calibrated
        );
    }

    #[test]
    fn test_calibration_status_fully_calibrated() {
        let mut engine = CalibrationEngine::new();
        engine.new_company("bolag-full");
        for i in 0..205 {
            let txn = sample_txn(&format!("Vendor{}", i % 20), dec!(300));
            engine.learn_from_booking("bolag-full", &txn, "6540");
        }
        assert_eq!(
            engine.calibration_confidence("bolag-full"),
            CalibrationStatus::FullyCalibrated
        );
    }

    #[test]
    fn test_export_import_rules() {
        let mut engine = CalibrationEngine::new();
        let txn = sample_txn("Stripe", dec!(500));
        engine.learn_from_booking("bolag-exp", &txn, "6590");
        let rules = engine.export_rules("bolag-exp");
        assert!(!rules.is_empty());

        let mut engine2 = CalibrationEngine::new();
        engine2.import_rules("bolag-exp", rules.clone());
        let imported = engine2.export_rules("bolag-exp");
        assert_eq!(imported.len(), rules.len());
    }

    #[test]
    fn test_set_approval_thresholds() {
        let mut engine = CalibrationEngine::new();
        engine.new_company("bolag-thr");
        let custom = ApprovalThresholds {
            auto_book_limit: dec!(1000),
            single_approver_limit: dec!(10000),
            two_approver_limit: dec!(50000),
            cfo_required_above: dec!(200000),
        };
        engine.set_approval_thresholds("bolag-thr", custom);
        let txn = sample_txn("ICA", dec!(1500));
        let result = engine.classify_for_company("bolag-thr", &txn);
        assert_eq!(result.approval_level, ApprovalLevel::Single);
    }

    #[test]
    fn test_learned_rule_reinforcement() {
        let mut rule = LearnedRule::new("merchant contains 'aws'", "use account 6540, vat_rate 0.25", 0.6);
        let after_first_reinforce = rule.reinforce();
        assert!(after_first_reinforce > 0.6);
        assert_eq!(rule.observations, 2);
    }

    #[test]
    fn test_all_calibration_scores() {
        let mut engine = CalibrationEngine::new();
        engine.new_company("a");
        engine.new_company("b");
        let scores = engine.all_calibration_scores();
        assert!(scores.contains_key("a"));
        assert!(scores.contains_key("b"));
    }

    #[test]
    fn test_vat_behavior_effective_rate_override() {
        let mut vat = VatBehavior::default();
        vat.merchant_overrides.insert("sj".to_string(), dec!(0.06));
        assert_eq!(vat.effective_rate("SJ"), dec!(0.06));
        assert_eq!(vat.effective_rate("Unknown"), dec!(0.25));
    }

    #[test]
    fn test_rule_matches_merchant_contains() {
        let rule = LearnedRule::new("merchant contains 'aws'", "use account 6540, vat_rate 0.25", 0.9);
        assert!(rule_matches(&rule, "amazon web services", "", dec!(100)));
        assert!(!rule_matches(&rule, "ica supermarket", "", dec!(100)));
    }

    #[test]
    fn test_rule_matches_amount_greater_than() {
        let rule = LearnedRule::new("amount > 10000", "use account 6540, vat_rate 0.25", 0.7);
        assert!(rule_matches(&rule, "", "", dec!(15000)));
        assert!(!rule_matches(&rule, "", "", dec!(5000)));
    }

    #[test]
    fn test_classify_unknown_company_fallback() {
        let engine = CalibrationEngine::new();
        let txn = sample_txn("Klarna", dec!(800));
        let result = engine.classify_for_company("totally-unknown", &txn);
        assert!(!result.is_company_specific);
    }

    #[test]
    fn test_parse_account_from_action() {
        assert_eq!(
            parse_account_from_action("use account 6540, vat_rate 0.25"),
            Some("6540".to_string())
        );
        assert_eq!(
            parse_account_from_action("use account 7320, vat_rate 0.06"),
            Some("7320".to_string())
        );
    }

    #[test]
    fn test_parse_vat_from_action() {
        assert_eq!(
            parse_vat_from_action("use account 6540, vat_rate 0.25"),
            Some(dec!(0.25))
        );
        assert_eq!(
            parse_vat_from_action("use account 7320, vat_rate 0.06"),
            Some(dec!(0.06))
        );
    }
}
