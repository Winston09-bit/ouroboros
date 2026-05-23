// vat_agent.rs — VAT / Moms Agent
//
// Analyserar transaktioner för en period, validerar momskoder, identifierar
// risker, beräknar nettoskuld/fordran och förbereder Skatteverket-filing.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared domain types (normally from a common crate)
// ---------------------------------------------------------------------------

/// A single financial transaction as it appears in the ledger.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub company_id: Uuid,
    pub date: chrono::NaiveDate,
    pub amount_excl_vat: Decimal,
    pub vat_amount: Decimal,
    pub currency: String,
    pub vat_code: String,
    pub description: String,
    pub vendor_vat_number: Option<String>,
    pub customer_vat_number: Option<String>,
    pub country_code: String, // ISO 3166-1 alpha-2 (e.g. "SE", "DE")
    pub is_purchase: bool,    // true = purchase (input VAT), false = sale (output VAT)
    pub receipt_attached: bool,
    pub account_code: String,
}

impl Transaction {
    pub fn total_amount(&self) -> Decimal {
        self.amount_excl_vat + self.vat_amount
    }

    pub fn effective_vat_rate(&self) -> Option<f64> {
        if self.amount_excl_vat.is_zero() {
            return None;
        }
        Some(
            (self.vat_amount / self.amount_excl_vat * dec!(100))
                .to_f64()
                .unwrap_or(0.0),
        )
    }
}

// ---------------------------------------------------------------------------
// VAT code catalogue (Swedish + EU rules)
// ---------------------------------------------------------------------------

/// Canonical VAT code definitions.
#[derive(Debug, Clone)]
pub struct VatCodeDef {
    pub code: String,
    pub description: String,
    pub rate: Decimal,
    pub is_reverse_charge: bool,
    pub is_intra_eu: bool,
    pub applies_to: VatApplicability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VatApplicability {
    Both,
    PurchasesOnly,
    SalesOnly,
}

/// Build the Swedish VAT code catalogue (SKV / Skatteverket).
fn swedish_vat_catalogue() -> HashMap<String, VatCodeDef> {
    let mut map = HashMap::new();

    let codes = vec![
        VatCodeDef {
            code: "MP1".to_owned(),
            description: "Utgående moms 25%".to_owned(),
            rate: dec!(25),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::SalesOnly,
        },
        VatCodeDef {
            code: "MP2".to_owned(),
            description: "Utgående moms 12% (livsmedel, hotell)".to_owned(),
            rate: dec!(12),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::SalesOnly,
        },
        VatCodeDef {
            code: "MP3".to_owned(),
            description: "Utgående moms 6% (böcker, tidningar, transport)".to_owned(),
            rate: dec!(6),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::SalesOnly,
        },
        VatCodeDef {
            code: "MF".to_owned(),
            description: "Momsfri omsättning".to_owned(),
            rate: dec!(0),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::Both,
        },
        VatCodeDef {
            code: "MP1I".to_owned(),
            description: "Ingående moms 25%".to_owned(),
            rate: dec!(25),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::PurchasesOnly,
        },
        VatCodeDef {
            code: "MP2I".to_owned(),
            description: "Ingående moms 12%".to_owned(),
            rate: dec!(12),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::PurchasesOnly,
        },
        VatCodeDef {
            code: "MP3I".to_owned(),
            description: "Ingående moms 6%".to_owned(),
            rate: dec!(6),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::PurchasesOnly,
        },
        VatCodeDef {
            code: "EU_VAROR_KOP".to_owned(),
            description: "Inköp av varor inom EU (omvänd skattskyldighet)".to_owned(),
            rate: dec!(25),
            is_reverse_charge: true,
            is_intra_eu: true,
            applies_to: VatApplicability::PurchasesOnly,
        },
        VatCodeDef {
            code: "EU_TJANST_KOP".to_owned(),
            description: "Inköp av tjänster inom EU (omvänd skattskyldighet)".to_owned(),
            rate: dec!(25),
            is_reverse_charge: true,
            is_intra_eu: true,
            applies_to: VatApplicability::PurchasesOnly,
        },
        VatCodeDef {
            code: "EU_VAROR_FORSALJ".to_owned(),
            description: "Försäljning av varor till köpare med VAT-nummer inom EU (0%)".to_owned(),
            rate: dec!(0),
            is_reverse_charge: false,
            is_intra_eu: true,
            applies_to: VatApplicability::SalesOnly,
        },
        VatCodeDef {
            code: "EXPORT".to_owned(),
            description: "Export utanför EU (0%)".to_owned(),
            rate: dec!(0),
            is_reverse_charge: false,
            is_intra_eu: false,
            applies_to: VatApplicability::SalesOnly,
        },
        VatCodeDef {
            code: "IMPORT_RC".to_owned(),
            description: "Import med omvänd skattskyldighet (25%)".to_owned(),
            rate: dec!(25),
            is_reverse_charge: true,
            is_intra_eu: false,
            applies_to: VatApplicability::PurchasesOnly,
        },
    ];

    for def in codes {
        map.insert(def.code.clone(), def);
    }
    map
}

// ---------------------------------------------------------------------------
// JurisdictionEngine
// ---------------------------------------------------------------------------

/// Encapsulates VAT rules for different jurisdictions (extended for multi-country).
pub struct JurisdictionEngine {
    catalogue: HashMap<String, VatCodeDef>,
    /// EU member state codes
    eu_member_states: Vec<String>,
}

impl JurisdictionEngine {
    pub fn new() -> Self {
        JurisdictionEngine {
            catalogue: swedish_vat_catalogue(),
            eu_member_states: vec![
                "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES", "FI",
                "FR", "GR", "HR", "HU", "IE", "IT", "LT", "LU", "LV", "MT",
                "NL", "PL", "PT", "RO", "SE", "SI", "SK",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    pub fn is_eu_country(&self, country_code: &str) -> bool {
        self.eu_member_states.contains(&country_code.to_uppercase().as_str().into())
            || self
                .eu_member_states
                .iter()
                .any(|c| c == &country_code.to_uppercase())
    }

    pub fn lookup_vat_code(&self, code: &str) -> Option<&VatCodeDef> {
        self.catalogue.get(code)
    }

    /// Determine the expected VAT treatment for a cross-border transaction.
    pub fn expected_vat_treatment(
        &self,
        is_purchase: bool,
        country_code: &str,
        has_buyer_vat_number: bool,
    ) -> ExpectedVatTreatment {
        let is_eu = self.is_eu_country(country_code);
        match (is_purchase, is_eu, has_buyer_vat_number) {
            // Purchase from EU supplier
            (true, true, _) => ExpectedVatTreatment::ReverseCharge,
            // Purchase from outside EU
            (true, false, _) => ExpectedVatTreatment::ImportReverseCharge,
            // Sale to EU with valid VAT number → zero-rated
            (false, true, true) => ExpectedVatTreatment::IntraEuZeroRated,
            // Sale to EU consumer (no VAT number) → Swedish VAT applies (OSS threshold may apply)
            (false, true, false) => ExpectedVatTreatment::StandardSwedishVat,
            // Sale outside EU → zero-rated export
            (false, false, _) => ExpectedVatTreatment::Export,
        }
    }
}

impl Default for JurisdictionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedVatTreatment {
    StandardSwedishVat,
    ReverseCharge,
    ImportReverseCharge,
    IntraEuZeroRated,
    Export,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VatTransaction {
    pub transaction_id: Uuid,
    pub date: chrono::NaiveDate,
    pub vat_code: String,
    pub amount_excl_vat: Decimal,
    pub vat_amount: Decimal,
    pub is_deductible: bool,
    pub treatment: VatTreatmentTag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VatTreatmentTag {
    StandardInput,
    StandardOutput,
    ReverseChargePurchase,
    IntraEuSale,
    Export,
    Exempt,
    Unknown,
}

impl fmt::Display for VatTreatmentTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            VatTreatmentTag::StandardInput => "STANDARD_INPUT",
            VatTreatmentTag::StandardOutput => "STANDARD_OUTPUT",
            VatTreatmentTag::ReverseChargePurchase => "REVERSE_CHARGE",
            VatTreatmentTag::IntraEuSale => "INTRA_EU_SALE",
            VatTreatmentTag::Export => "EXPORT",
            VatTreatmentTag::Exempt => "EXEMPT",
            VatTreatmentTag::Unknown => "UNKNOWN",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct VatReport {
    pub company_id: Uuid,
    pub period: String,
    pub jurisdiction: String,
    /// Total deductible input VAT (ingående moms)
    pub input_vat: Decimal,
    /// Total output VAT charged to customers (utgående moms)
    pub output_vat: Decimal,
    /// Net VAT position = output_vat - input_vat (positive = payable to Skatteverket)
    pub net_vat: Decimal,
    pub transactions: Vec<VatTransaction>,
    pub risks: Vec<VatRisk>,
    /// Percentage of transactions with all required fields present.
    pub completeness_pct: f64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VatBalance {
    pub net_vat: Decimal,
    /// Positive → company owes Skatteverket.
    pub payable: Decimal,
    /// Positive → Skatteverket owes company.
    pub refundable: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone)]
pub struct VatValidation {
    pub is_valid: bool,
    pub issues: Vec<VatIssue>,
    pub corrected_vat_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VatIssue {
    pub kind: VatIssueKind,
    pub message: String,
    pub severity: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VatIssueKind {
    UnknownVatCode,
    RateMismatch,
    ReverseChargeNotApplied,
    IntraEuNotFlagged,
    MissingVatNumber,
    ExemptCodeOnTaxableTransaction,
    WrongApplicability,
}

#[derive(Debug, Clone)]
pub struct VatRisk {
    pub risk_type: VatRiskType,
    pub severity: RiskLevel,
    pub description: String,
    pub transaction_id: Option<Uuid>,
    pub amount_at_risk: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VatRiskType {
    MissingVatNumber,
    IncorrectVatRate,
    MissingReceipt,
    ReverseChargeRequired,
    IntraEuTransaction,
    HighValueTransaction,
    UnknownVatCode,
    ExemptionRisk,
    DeductibilityDoubt,
}

impl fmt::Display for VatRiskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Prepared Skatteverket filing.
#[derive(Debug, Clone)]
pub struct VatFiling {
    pub period: String,
    pub company_id: Uuid,
    pub xml_payload: String,
    pub output_vat: Decimal,
    pub input_vat: Decimal,
    pub net_vat: Decimal,
    pub due_date: chrono::NaiveDate,
    pub generated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Main agent
// ---------------------------------------------------------------------------

pub struct VatAgent {
    pub jurisdiction_engine: Arc<JurisdictionEngine>,
    /// Threshold above which a single transaction is flagged as high-value.
    pub high_value_threshold: Decimal,
}

impl VatAgent {
    pub fn new(jurisdiction_engine: Arc<JurisdictionEngine>) -> Self {
        VatAgent {
            jurisdiction_engine,
            high_value_threshold: dec!(500_000),
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Build a full VAT report for a company and period (e.g. "2024-Q1" or "2024-01").
    pub async fn build_vat_report(
        &self,
        company_id: Uuid,
        transactions: &[Transaction],
        period: &str,
    ) -> VatReport {
        let mut vat_transactions: Vec<VatTransaction> = Vec::with_capacity(transactions.len());
        let mut input_vat = Decimal::ZERO;
        let mut output_vat = Decimal::ZERO;
        let mut completeness_issues = 0usize;

        for txn in transactions {
            if txn.company_id != company_id {
                continue;
            }

            let treatment = self.classify_treatment(txn);
            let is_deductible = matches!(
                treatment,
                VatTreatmentTag::StandardInput | VatTreatmentTag::ReverseChargePurchase
            );

            if is_deductible {
                input_vat += txn.vat_amount;
            } else if matches!(
                treatment,
                VatTreatmentTag::StandardOutput | VatTreatmentTag::IntraEuSale
            ) {
                output_vat += txn.vat_amount;
            }

            // Completeness check
            if txn.vendor_vat_number.is_none()
                && txn.amount_excl_vat > dec!(5000)
                && txn.is_purchase
            {
                completeness_issues += 1;
            }
            if !txn.receipt_attached && txn.is_purchase {
                completeness_issues += 1;
            }

            vat_transactions.push(VatTransaction {
                transaction_id: txn.id,
                date: txn.date,
                vat_code: txn.vat_code.clone(),
                amount_excl_vat: txn.amount_excl_vat,
                vat_amount: txn.vat_amount,
                is_deductible,
                treatment,
            });
        }

        let total = vat_transactions.len();
        let completeness_pct = if total == 0 {
            100.0
        } else {
            let problem_count = completeness_issues.min(total);
            ((total - problem_count) as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
        };

        let net_vat = output_vat - input_vat;
        let risks = self.identify_risks(transactions);

        VatReport {
            company_id,
            period: period.to_owned(),
            jurisdiction: "SE".to_owned(),
            input_vat,
            output_vat,
            net_vat,
            transactions: vat_transactions,
            risks,
            completeness_pct,
            generated_at: Utc::now(),
        }
    }

    /// Validate the VAT treatment of a single transaction.
    pub fn validate_vat_transaction(&self, txn: &Transaction) -> VatValidation {
        let mut issues = Vec::new();

        // 1. Check VAT code exists in catalogue
        let code_def = self.jurisdiction_engine.lookup_vat_code(&txn.vat_code);
        if code_def.is_none() {
            issues.push(VatIssue {
                kind: VatIssueKind::UnknownVatCode,
                message: format!("VAT code '{}' not found in catalogue", txn.vat_code),
                severity: RiskLevel::High,
            });
        }

        if let Some(def) = code_def {
            // 2. Applicability check
            if txn.is_purchase
                && def.applies_to == VatApplicability::SalesOnly
            {
                issues.push(VatIssue {
                    kind: VatIssueKind::WrongApplicability,
                    message: format!(
                        "VAT code '{}' is sales-only but applied to a purchase",
                        txn.vat_code
                    ),
                    severity: RiskLevel::High,
                });
            }
            if !txn.is_purchase && def.applies_to == VatApplicability::PurchasesOnly {
                issues.push(VatIssue {
                    kind: VatIssueKind::WrongApplicability,
                    message: format!(
                        "VAT code '{}' is purchases-only but applied to a sale",
                        txn.vat_code
                    ),
                    severity: RiskLevel::High,
                });
            }

            // 3. Rate sanity
            if !txn.amount_excl_vat.is_zero() {
                let computed_rate = txn.vat_amount / txn.amount_excl_vat * dec!(100);
                let expected_rate = def.rate;
                let diff = (computed_rate - expected_rate).abs();
                if diff > dec!(0.5) {
                    issues.push(VatIssue {
                        kind: VatIssueKind::RateMismatch,
                        message: format!(
                            "VAT rate mismatch: code '{}' expects {}%, computed {:.2}%",
                            txn.vat_code, expected_rate, computed_rate
                        ),
                        severity: RiskLevel::Medium,
                    });
                }
            }

            // 4. Reverse charge required but not flagged
            let is_eu = self.jurisdiction_engine.is_eu_country(&txn.country_code);
            let is_foreign = txn.country_code.to_uppercase() != "SE";
            if txn.is_purchase && is_foreign && is_eu && !def.is_reverse_charge {
                issues.push(VatIssue {
                    kind: VatIssueKind::ReverseChargeNotApplied,
                    message: format!(
                        "EU purchase from {} should use reverse-charge code, not '{}'",
                        txn.country_code, txn.vat_code
                    ),
                    severity: RiskLevel::High,
                });
            }

            // 5. Intra-EU sale without buyer VAT number
            if !txn.is_purchase && is_foreign && is_eu && def.is_intra_eu {
                if txn.customer_vat_number.is_none() {
                    issues.push(VatIssue {
                        kind: VatIssueKind::MissingVatNumber,
                        message: "Intra-EU sale flagged but buyer VAT number is missing".to_owned(),
                        severity: RiskLevel::High,
                    });
                }
            }
        }

        let corrected_vat_code: Option<String> = {
            let has_reverse_charge_issue = issues
                .iter()
                .any(|i| i.kind == VatIssueKind::ReverseChargeNotApplied);
            if has_reverse_charge_issue {
                Some("EU_TJANST_KOP".to_owned())
            } else {
                None
            }
        };

        VatValidation {
            is_valid: issues.is_empty(),
            issues,
            corrected_vat_code,
        }
    }

    /// Scan a batch of transactions and return all identified VAT risks.
    pub fn identify_risks(&self, transactions: &[Transaction]) -> Vec<VatRisk> {
        let mut risks: Vec<VatRisk> = Vec::new();

        for txn in transactions {
            // Missing VAT number on significant purchase
            if txn.is_purchase
                && txn.vendor_vat_number.is_none()
                && txn.amount_excl_vat > dec!(5000)
            {
                risks.push(VatRisk {
                    risk_type: VatRiskType::MissingVatNumber,
                    severity: RiskLevel::Medium,
                    description: format!(
                        "Purchase {} ({:.2} {}) has no vendor VAT number",
                        txn.id, txn.amount_excl_vat, txn.currency
                    ),
                    transaction_id: Some(txn.id),
                    amount_at_risk: txn.vat_amount,
                });
            }

            // Missing receipt
            if txn.is_purchase && !txn.receipt_attached {
                risks.push(VatRisk {
                    risk_type: VatRiskType::MissingReceipt,
                    severity: RiskLevel::Medium,
                    description: format!(
                        "Purchase {} is missing an attached receipt/invoice",
                        txn.id
                    ),
                    transaction_id: Some(txn.id),
                    amount_at_risk: txn.vat_amount,
                });
            }

            // High-value transaction
            if txn.amount_excl_vat >= self.high_value_threshold {
                risks.push(VatRisk {
                    risk_type: VatRiskType::HighValueTransaction,
                    severity: RiskLevel::High,
                    description: format!(
                        "High-value transaction {} ({:.2} {}); extra review required",
                        txn.id, txn.amount_excl_vat, txn.currency
                    ),
                    transaction_id: Some(txn.id),
                    amount_at_risk: txn.vat_amount,
                });
            }

            // Intra-EU flag
            let is_foreign = txn.country_code.to_uppercase() != "SE";
            let is_eu = self.jurisdiction_engine.is_eu_country(&txn.country_code);
            if is_foreign && is_eu {
                risks.push(VatRisk {
                    risk_type: VatRiskType::IntraEuTransaction,
                    severity: RiskLevel::Low,
                    description: format!(
                        "Transaction {} involves EU country {} – verify VAT treatment",
                        txn.id, txn.country_code
                    ),
                    transaction_id: Some(txn.id),
                    amount_at_risk: txn.vat_amount,
                });
            }

            // Reverse charge required check
            if txn.is_purchase && is_foreign && is_eu {
                let code_def = self.jurisdiction_engine.lookup_vat_code(&txn.vat_code);
                let is_rc = code_def.map(|d| d.is_reverse_charge).unwrap_or(false);
                if !is_rc {
                    risks.push(VatRisk {
                        risk_type: VatRiskType::ReverseChargeRequired,
                        severity: RiskLevel::High,
                        description: format!(
                            "Transaction {} is an EU purchase but does not use reverse-charge code",
                            txn.id
                        ),
                        transaction_id: Some(txn.id),
                        amount_at_risk: txn.vat_amount,
                    });
                }
            }

            // Incorrect VAT rate
            let validation = self.validate_vat_transaction(txn);
            if !validation.is_valid {
                let rate_issues: Vec<_> = validation
                    .issues
                    .iter()
                    .filter(|i| i.kind == VatIssueKind::RateMismatch)
                    .collect();
                for issue in rate_issues {
                    risks.push(VatRisk {
                        risk_type: VatRiskType::IncorrectVatRate,
                        severity: RiskLevel::High,
                        description: issue.message.clone(),
                        transaction_id: Some(txn.id),
                        amount_at_risk: txn.vat_amount,
                    });
                }
            }
        }

        // Sort by severity descending
        risks.sort_by(|a, b| b.severity.cmp(&a.severity));
        risks
    }

    /// Calculate the net VAT balance (payable or refundable).
    pub fn calculate_net_vat(&self, report: &VatReport) -> VatBalance {
        let net = report.output_vat - report.input_vat;
        if net >= Decimal::ZERO {
            VatBalance {
                net_vat: net,
                payable: net,
                refundable: Decimal::ZERO,
                currency: "SEK".to_owned(),
            }
        } else {
            VatBalance {
                net_vat: net,
                payable: Decimal::ZERO,
                refundable: net.abs(),
                currency: "SEK".to_owned(),
            }
        }
    }

    /// Generate a Skatteverket-compatible XML filing payload.
    ///
    /// Format follows SKV 2021 specification for electronic VAT returns.
    pub fn prepare_filing(&self, report: &VatReport) -> VatFiling {
        // Parse period for due-date calculation
        // Accepted formats: "YYYY-MM" (monthly) or "YYYY-QN"
        let (year, month) = parse_period_ym(&report.period);
        let due_date = skatteverket_due_date(year, month);

        let xml = self.build_xml_filing(report);

        VatFiling {
            period: report.period.clone(),
            company_id: report.company_id,
            xml_payload: xml,
            output_vat: report.output_vat,
            input_vat: report.input_vat,
            net_vat: report.net_vat,
            due_date,
            generated_at: Utc::now(),
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn classify_treatment(&self, txn: &Transaction) -> VatTreatmentTag {
        let code_def = self.jurisdiction_engine.lookup_vat_code(&txn.vat_code);
        let is_foreign = txn.country_code.to_uppercase() != "SE";
        let is_eu = self.jurisdiction_engine.is_eu_country(&txn.country_code);

        if txn.vat_code.is_empty() || txn.vat_code == "MF" {
            return VatTreatmentTag::Exempt;
        }

        match (txn.is_purchase, is_foreign, is_eu) {
            (true, true, true) => VatTreatmentTag::ReverseChargePurchase,
            (true, false, _) => VatTreatmentTag::StandardInput,
            (false, true, true) => {
                if txn.customer_vat_number.is_some() {
                    VatTreatmentTag::IntraEuSale
                } else {
                    VatTreatmentTag::StandardOutput
                }
            }
            (false, true, false) => VatTreatmentTag::Export,
            (false, false, _) => VatTreatmentTag::StandardOutput,
            _ => {
                if let Some(def) = code_def {
                    if def.is_reverse_charge {
                        VatTreatmentTag::ReverseChargePurchase
                    } else if def.is_intra_eu {
                        if txn.is_purchase {
                            VatTreatmentTag::ReverseChargePurchase
                        } else {
                            VatTreatmentTag::IntraEuSale
                        }
                    } else {
                        VatTreatmentTag::Unknown
                    }
                } else {
                    VatTreatmentTag::Unknown
                }
            }
        }
    }

    fn build_xml_filing(&self, report: &VatReport) -> String {
        // Skatteverket SKV 2021 simplified schema
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<MomsDeklaration xmlns="http://skatteverket.se/moms/2021"
                 xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                 xsi:schemaLocation="http://skatteverket.se/moms/2021 moms.xsd">
  <Organisationsnummer>{company_id}</Organisationsnummer>
  <Redovisningsperiod>{period}</Redovisningsperiod>
  <Jurisdiktion>{jurisdiction}</Jurisdiktion>
  <UtgaendeMoms>
    <Belopp>{output_vat}</Belopp>
  </UtgaendeMoms>
  <IngaendeMoms>
    <Belopp>{input_vat}</Belopp>
  </IngaendeMoms>
  <MomsSkuld>
    <Belopp>{net_vat}</Belopp>
    <Typ>{typ}</Typ>
  </MomsSkuld>
  <AntalTransaktioner>{count}</AntalTransaktioner>
  <Kompletthet>{completeness:.1}</Kompletthet>
  <Genererad>{generated_at}</Genererad>
</MomsDeklaration>"#,
            company_id = report.company_id,
            period = report.period,
            jurisdiction = report.jurisdiction,
            output_vat = report.output_vat,
            input_vat = report.input_vat,
            net_vat = report.net_vat.abs(),
            typ = if report.net_vat >= Decimal::ZERO {
                "SKULD"
            } else {
                "FORDRAN"
            },
            count = report.transactions.len(),
            completeness = report.completeness_pct,
            generated_at = report.generated_at.to_rfc3339(),
        )
    }
}

// ---------------------------------------------------------------------------
// Period helpers
// ---------------------------------------------------------------------------

/// Parse "YYYY-MM" or "YYYY-QN" → (year, representative_month)
fn parse_period_ym(period: &str) -> (i32, u32) {
    // Try YYYY-MM
    if let Some(parts) = period.split_once('-') {
        if let (Ok(y), Ok(m)) = (parts.0.parse::<i32>(), parts.1.parse::<u32>()) {
            return (y, m);
        }
        // Try YYYY-QN
        if parts.1.starts_with('Q') {
            if let (Ok(y), Ok(q)) = (
                parts.0.parse::<i32>(),
                parts.1.trim_start_matches('Q').parse::<u32>(),
            ) {
                let month = (q - 1) * 3 + 3; // last month of quarter
                return (y, month.min(12));
            }
        }
    }
    // Fallback
    let now = Utc::now();
    (now.year(), now.month())
}

use chrono::{Datelike, Duration, NaiveDate};

/// Skatteverket due date: 26th of the second month after the reporting period.
fn skatteverket_due_date(year: i32, period_month: u32) -> NaiveDate {
    let due_month = if period_month >= 11 {
        ((period_month as i32 + 2 - 12) as u32, year + 1)
    } else {
        (period_month + 2, year)
    };
    NaiveDate::from_ymd_opt(due_month.1, due_month.0, 26)
        .unwrap_or_else(|| {
            // 26th doesn't exist (shouldn't happen for month 1-12) — fallback to last day
            NaiveDate::from_ymd_opt(due_month.1, due_month.0 + 1, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_domestic_purchase(amount: Decimal, vat: Decimal, vat_code: &str) -> Transaction {
        Transaction {
            id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            amount_excl_vat: amount,
            vat_amount: vat,
            currency: "SEK".to_owned(),
            vat_code: vat_code.to_owned(),
            description: "Test purchase".to_owned(),
            vendor_vat_number: Some("SE556000000001".to_owned()),
            customer_vat_number: None,
            country_code: "SE".to_owned(),
            is_purchase: true,
            receipt_attached: true,
            account_code: "6540".to_owned(),
        }
    }

    #[test]
    fn test_validate_valid_25pct() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let txn = make_domestic_purchase(dec!(1000), dec!(250), "MP1I");
        let v = agent.validate_vat_transaction(&txn);
        assert!(v.is_valid, "Expected valid, issues: {:?}", v.issues);
    }

    #[test]
    fn test_validate_rate_mismatch() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let txn = make_domestic_purchase(dec!(1000), dec!(120), "MP1I"); // 12% on code that expects 25%
        let v = agent.validate_vat_transaction(&txn);
        assert!(!v.is_valid);
        assert!(v.issues.iter().any(|i| i.kind == VatIssueKind::RateMismatch));
    }

    #[test]
    fn test_validate_eu_purchase_wrong_code() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let mut txn = make_domestic_purchase(dec!(1000), dec!(250), "MP1I");
        txn.country_code = "DE".to_owned();
        let v = agent.validate_vat_transaction(&txn);
        assert!(!v.is_valid);
        assert!(v.issues.iter().any(|i| i.kind == VatIssueKind::ReverseChargeNotApplied));
        assert_eq!(v.corrected_vat_code, Some("EU_TJANST_KOP".to_owned()));
    }

    #[test]
    fn test_calculate_net_vat_payable() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let report = VatReport {
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            jurisdiction: "SE".to_owned(),
            input_vat: dec!(10000),
            output_vat: dec!(25000),
            net_vat: dec!(15000),
            transactions: vec![],
            risks: vec![],
            completeness_pct: 100.0,
            generated_at: Utc::now(),
        };
        let balance = agent.calculate_net_vat(&report);
        assert_eq!(balance.payable, dec!(15000));
        assert_eq!(balance.refundable, Decimal::ZERO);
    }

    #[test]
    fn test_calculate_net_vat_refundable() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let report = VatReport {
            company_id: Uuid::new_v4(),
            period: "2024-02".to_owned(),
            jurisdiction: "SE".to_owned(),
            input_vat: dec!(30000),
            output_vat: dec!(10000),
            net_vat: dec!(-20000),
            transactions: vec![],
            risks: vec![],
            completeness_pct: 100.0,
            generated_at: Utc::now(),
        };
        let balance = agent.calculate_net_vat(&report);
        assert_eq!(balance.refundable, dec!(20000));
        assert_eq!(balance.payable, Decimal::ZERO);
    }

    #[test]
    fn test_identify_risks_missing_receipt() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let mut txn = make_domestic_purchase(dec!(1000), dec!(250), "MP1I");
        txn.receipt_attached = false;
        let risks = agent.identify_risks(&[txn]);
        assert!(risks.iter().any(|r| r.risk_type == VatRiskType::MissingReceipt));
    }

    #[test]
    fn test_identify_risks_high_value() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let txn = make_domestic_purchase(dec!(1_000_000), dec!(250_000), "MP1I");
        let risks = agent.identify_risks(&[txn]);
        assert!(risks.iter().any(|r| r.risk_type == VatRiskType::HighValueTransaction));
    }

    #[test]
    fn test_skatteverket_due_date_january() {
        let d = skatteverket_due_date(2024, 1);
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 3, 26).unwrap());
    }

    #[test]
    fn test_skatteverket_due_date_november() {
        let d = skatteverket_due_date(2024, 11);
        // 11 + 2 = 13 → Jan 2025
        assert_eq!(d, NaiveDate::from_ymd_opt(2025, 1, 26).unwrap());
    }

    #[test]
    fn test_parse_period_quarterly() {
        let (y, m) = parse_period_ym("2024-Q1");
        assert_eq!(y, 2024);
        assert_eq!(m, 3); // last month of Q1
    }

    #[test]
    fn test_xml_filing_contains_required_fields() {
        let engine = Arc::new(JurisdictionEngine::new());
        let agent = VatAgent::new(engine);
        let report = VatReport {
            company_id: Uuid::new_v4(),
            period: "2024-01".to_owned(),
            jurisdiction: "SE".to_owned(),
            input_vat: dec!(5000),
            output_vat: dec!(12500),
            net_vat: dec!(7500),
            transactions: vec![],
            risks: vec![],
            completeness_pct: 98.5,
            generated_at: Utc::now(),
        };
        let filing = agent.prepare_filing(&report);
        assert!(filing.xml_payload.contains("<MomsDeklaration"));
        assert!(filing.xml_payload.contains("2024-01"));
        assert!(filing.xml_payload.contains("12500"));
        assert!(filing.xml_payload.contains("SKULD"));
        assert_eq!(filing.net_vat, dec!(7500));
    }

    #[test]
    fn test_eu_classification() {
        let engine = JurisdictionEngine::new();
        assert!(engine.is_eu_country("DE"));
        assert!(engine.is_eu_country("FR"));
        assert!(engine.is_eu_country("SE"));
        assert!(!engine.is_eu_country("US"));
        assert!(!engine.is_eu_country("GB")); // Brexit
        assert!(!engine.is_eu_country("NO"));
    }
}
