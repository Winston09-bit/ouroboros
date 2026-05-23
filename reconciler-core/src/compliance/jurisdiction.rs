use std::collections::HashMap;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilingFrequency {
    Monthly,
    Quarterly,
    Annually,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilingType {
    VAT,
    IncomeTax,
    Payroll,
    Moms,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRules {
    pub code: String,
    pub name: String,
    pub currency: String,
    /// category slug → VAT/sales-tax rate (0.25 = 25 %)
    pub vat_rates: HashMap<String, Decimal>,
    /// Default document retention in years
    pub retention_years: u32,
    pub tax_authority: String,
    pub filing_frequency: FilingFrequency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeductibilityResult {
    pub deductible: bool,
    /// Fraction of expense that is deductible (0.0–1.0)
    pub rate: Decimal,
    pub requires_receipt: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilingDeadline {
    pub name: String,
    pub due_date: NaiveDate,
    pub jurisdiction: String,
    pub filing_type: FilingType,
}

// ---------------------------------------------------------------------------
// JurisdictionEngine
// ---------------------------------------------------------------------------

pub struct JurisdictionEngine {
    rules: HashMap<String, JurisdictionRules>,
}

impl JurisdictionEngine {
    /// Construct engine with built-in rules for SE, US, GB and EU.
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert("SE".to_string(), Self::rules_se());
        rules.insert("US".to_string(), Self::rules_us());
        rules.insert("GB".to_string(), Self::rules_gb());
        rules.insert("EU".to_string(), Self::rules_eu());
        Self { rules }
    }

    // ------------------------------------------------------------------
    // Built-in rule definitions
    // ------------------------------------------------------------------

    fn rules_se() -> JurisdictionRules {
        let mut vat_rates = HashMap::new();
        // Standard rate
        vat_rates.insert("standard".to_string(), dec!(0.25));
        vat_rates.insert("goods".to_string(), dec!(0.25));
        vat_rates.insert("services".to_string(), dec!(0.25));
        vat_rates.insert("software".to_string(), dec!(0.25));
        vat_rates.insert("electronics".to_string(), dec!(0.25));
        vat_rates.insert("clothing".to_string(), dec!(0.25));
        // Reduced 12 %
        vat_rates.insert("food".to_string(), dec!(0.12));
        vat_rates.insert("restaurant".to_string(), dec!(0.12));
        vat_rates.insert("hotel".to_string(), dec!(0.12));
        vat_rates.insert("accommodation".to_string(), dec!(0.12));
        // Reduced 6 %
        vat_rates.insert("books".to_string(), dec!(0.06));
        vat_rates.insert("newspapers".to_string(), dec!(0.06));
        vat_rates.insert("transport".to_string(), dec!(0.06));
        vat_rates.insert("culture".to_string(), dec!(0.06));
        vat_rates.insert("sport".to_string(), dec!(0.06));
        // Zero-rated
        vat_rates.insert("export".to_string(), dec!(0.00));
        vat_rates.insert("medical".to_string(), dec!(0.00));
        vat_rates.insert("education".to_string(), dec!(0.00));
        vat_rates.insert("financial".to_string(), dec!(0.00));
        vat_rates.insert("insurance".to_string(), dec!(0.00));

        JurisdictionRules {
            code: "SE".to_string(),
            name: "Sverige".to_string(),
            currency: "SEK".to_string(),
            vat_rates,
            retention_years: 7,
            tax_authority: "Skatteverket".to_string(),
            filing_frequency: FilingFrequency::Quarterly,
        }
    }

    fn rules_us() -> JurisdictionRules {
        let mut vat_rates = HashMap::new();
        // No federal VAT in the US; we model a blended average sales-tax range.
        // States with no sales tax: OR, MT, NH, DE, AK (0 %)
        vat_rates.insert("no_sales_tax_state".to_string(), dec!(0.00));
        // States ~5 %
        vat_rates.insert("low_rate_state".to_string(), dec!(0.05));
        // Most states cluster around 6–7 %
        vat_rates.insert("standard".to_string(), dec!(0.065));
        vat_rates.insert("goods".to_string(), dec!(0.065));
        vat_rates.insert("services".to_string(), dec!(0.00)); // Most states exempt services
        vat_rates.insert("software".to_string(), dec!(0.065));
        vat_rates.insert("electronics".to_string(), dec!(0.065));
        vat_rates.insert("clothing".to_string(), dec!(0.065));
        // California (CA) – 7.25 % base
        vat_rates.insert("ca".to_string(), dec!(0.0725));
        // Tennessee – 9.55 % (one of the highest)
        vat_rates.insert("tn".to_string(), dec!(0.0955));
        // Food – often reduced/exempt
        vat_rates.insert("food".to_string(), dec!(0.00));
        vat_rates.insert("medical".to_string(), dec!(0.00));
        vat_rates.insert("export".to_string(), dec!(0.00));

        JurisdictionRules {
            code: "US".to_string(),
            name: "United States".to_string(),
            currency: "USD".to_string(),
            vat_rates,
            retention_years: 7, // IRS: 7 years for bad-debt deductions; general 3 years
            tax_authority: "IRS / State Revenue Departments".to_string(),
            filing_frequency: FilingFrequency::Quarterly,
        }
    }

    fn rules_gb() -> JurisdictionRules {
        let mut vat_rates = HashMap::new();
        // Standard 20 %
        vat_rates.insert("standard".to_string(), dec!(0.20));
        vat_rates.insert("goods".to_string(), dec!(0.20));
        vat_rates.insert("services".to_string(), dec!(0.20));
        vat_rates.insert("software".to_string(), dec!(0.20));
        vat_rates.insert("electronics".to_string(), dec!(0.20));
        vat_rates.insert("clothing".to_string(), dec!(0.20));
        vat_rates.insert("hotel".to_string(), dec!(0.20));
        vat_rates.insert("accommodation".to_string(), dec!(0.20));
        vat_rates.insert("restaurant".to_string(), dec!(0.20));
        // Reduced 5 %
        vat_rates.insert("energy".to_string(), dec!(0.05));
        vat_rates.insert("children_car_seat".to_string(), dec!(0.05));
        vat_rates.insert("sanitary_products".to_string(), dec!(0.05));
        // Zero-rated
        vat_rates.insert("food".to_string(), dec!(0.00));
        vat_rates.insert("books".to_string(), dec!(0.00));
        vat_rates.insert("newspapers".to_string(), dec!(0.00));
        vat_rates.insert("children_clothing".to_string(), dec!(0.00));
        vat_rates.insert("medical".to_string(), dec!(0.00));
        vat_rates.insert("export".to_string(), dec!(0.00));
        vat_rates.insert("transport".to_string(), dec!(0.00));

        JurisdictionRules {
            code: "GB".to_string(),
            name: "United Kingdom".to_string(),
            currency: "GBP".to_string(),
            vat_rates,
            retention_years: 6, // HMRC: 6 years
            tax_authority: "HMRC".to_string(),
            filing_frequency: FilingFrequency::Quarterly,
        }
    }

    fn rules_eu() -> JurisdictionRules {
        // EU-wide OSS / cross-border digital services rules.
        // Member states set their own rates; we model the OSS destination-country principle.
        let mut vat_rates = HashMap::new();
        vat_rates.insert("standard".to_string(), dec!(0.21)); // EU average ≈21 %
        vat_rates.insert("digital_services".to_string(), dec!(0.21));
        vat_rates.insert("goods".to_string(), dec!(0.21));
        vat_rates.insert("software".to_string(), dec!(0.21));
        vat_rates.insert("food".to_string(), dec!(0.09));
        vat_rates.insert("books".to_string(), dec!(0.05));
        // Reverse-charge – B2B cross-border supplies: buyer accounts for VAT
        vat_rates.insert("reverse_charge".to_string(), dec!(0.00));
        vat_rates.insert("export".to_string(), dec!(0.00));
        vat_rates.insert("medical".to_string(), dec!(0.00));

        JurisdictionRules {
            code: "EU".to_string(),
            name: "European Union (OSS)".to_string(),
            currency: "EUR".to_string(),
            vat_rates,
            retention_years: 10, // EU VAT Directive Art. 244: 10 years for e-services records
            tax_authority: "EU Member State Tax Authorities / OSS".to_string(),
            filing_frequency: FilingFrequency::Quarterly,
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Return the VAT / sales-tax rate for a category in a jurisdiction.
    /// Falls back to the "standard" rate if the category is unknown.
    /// Returns Decimal::ZERO if the jurisdiction itself is unknown.
    pub fn vat_rate(&self, jurisdiction: &str, category: &str) -> Decimal {
        let jur = jurisdiction.to_uppercase();
        match self.rules.get(&jur) {
            None => Decimal::ZERO,
            Some(rules) => {
                let key = category.to_lowercase();
                rules
                    .vat_rates
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| {
                        rules
                            .vat_rates
                            .get("standard")
                            .copied()
                            .unwrap_or(Decimal::ZERO)
                    })
            }
        }
    }

    /// Validate a VAT registration number against jurisdiction-specific patterns.
    pub fn validate_vat_number(&self, number: &str, jurisdiction: &str) -> bool {
        let jur = jurisdiction.to_uppercase();
        let n = number.trim().replace(" ", "").to_uppercase();
        match jur.as_str() {
            // SE: SE + 12 digits, e.g. SE556012345601
            "SE" => {
                n.starts_with("SE")
                    && n.len() == 14
                    && n[2..].chars().all(|c| c.is_ascii_digit())
            }
            // US: EIN format 00-0000000 (9 digits)
            "US" => {
                let digits: String = n.chars().filter(|c| c.is_ascii_digit()).collect();
                digits.len() == 9
            }
            // GB: GB + 9 or 12 digits (post-Brexit VAT number), or GD/HA followed by 3 digits
            "GB" => {
                if n.starts_with("GB") {
                    let rest = &n[2..];
                    rest.chars().all(|c| c.is_ascii_digit())
                        && (rest.len() == 9 || rest.len() == 12)
                } else if n.starts_with("GD") || n.starts_with("HA") {
                    let rest = &n[2..];
                    rest.len() == 3 && rest.chars().all(|c| c.is_ascii_digit())
                } else {
                    false
                }
            }
            // EU generic: 2-letter country prefix + 2–13 alphanumeric chars
            "EU" => {
                n.len() >= 4
                    && n.len() <= 15
                    && n[..2].chars().all(|c| c.is_ascii_alphabetic())
                    && n[2..].chars().all(|c| c.is_ascii_alphanumeric())
            }
            // DE: DE + 9 digits
            "DE" => {
                n.starts_with("DE")
                    && n.len() == 11
                    && n[2..].chars().all(|c| c.is_ascii_digit())
            }
            // FR: FR + 2 chars (can be alpha or digit) + 9 digits
            "FR" => {
                n.starts_with("FR")
                    && n.len() == 13
                    && n[2..].chars().all(|c| c.is_ascii_alphanumeric())
            }
            // Fallback: any non-empty string with at least 4 characters
            _ => n.len() >= 4,
        }
    }

    /// Return the document retention period in years for a given document type
    /// and jurisdiction. Falls back to the jurisdiction default if doc_type is
    /// unrecognised, and to 7 years if the jurisdiction is unknown.
    pub fn retention_years(&self, doc_type: &str, jurisdiction: &str) -> u32 {
        let jur = jurisdiction.to_uppercase();
        let default = self
            .rules
            .get(&jur)
            .map(|r| r.retention_years)
            .unwrap_or(7);

        match doc_type.to_lowercase().as_str() {
            // Tax-related – use jurisdiction default
            "invoice" | "vat_return" | "sales_tax_return" | "moms" => default,
            // Payroll records
            "payroll" => match jur.as_str() {
                "SE" => 10, // Arbetsgivare: lönedokumentation 10 år
                "US" => 4,  // FLSA payroll: 3 years; IRS recommends 4
                "GB" => 3,  // HMRC PAYE: 3 years after tax year
                _ => default,
            },
            // Contracts
            "contract" => match jur.as_str() {
                "SE" => 10, // Preskriptionslag 10 år
                "US" => 7,
                "GB" => 6,
                _ => default,
            },
            // Bank statements
            "bank_statement" => match jur.as_str() {
                "SE" => 7,
                "US" => 7,
                "GB" => 6,
                _ => default,
            },
            // Corporate records – keep indefinitely; model as 99
            "articles_of_incorporation" | "board_minutes" | "shareholder_register" => 99,
            // Receipts / expense reports
            "receipt" | "expense_report" => default,
            // EU OSS records
            "oss_return" => 10,
            _ => default,
        }
    }

    /// Determine whether a category of expense is tax-deductible in the
    /// given jurisdiction and at what rate.
    pub fn is_deductible(&self, category: &str, jurisdiction: &str) -> DeductibilityResult {
        let jur = jurisdiction.to_uppercase();
        let cat = category.to_lowercase();

        match jur.as_str() {
            "SE" => Self::deductibility_se(&cat),
            "US" => Self::deductibility_us(&cat),
            "GB" => Self::deductibility_gb(&cat),
            "EU" => Self::deductibility_eu(&cat),
            _ => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: true,
                notes: format!("Unknown jurisdiction '{}'", jurisdiction),
            },
        }
    }

    fn deductibility_se(cat: &str) -> DeductibilityResult {
        match cat {
            "office_supplies" | "software" | "equipment" | "hardware" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Avdragsgill driftskostnad (IL 16:1)".to_string(),
            },
            "travel" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Tjänsteresor avdragsgilla; spara kvitto + syfte (IL 12:1)".to_string(),
            },
            "meals" | "restaurant" => DeductibilityResult {
                deductible: true,
                rate: dec!(0.5),    // Representation: 50 % + moms 0
                requires_receipt: true,
                notes: "Representation 50 % avdrag (IL 16:2); max 300 kr/person exkl moms".to_string(),
            },
            "car" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Tjänstebil: avdrag för faktiska kostnader eller schablon 25 öre/km".to_string(),
            },
            "marketing" | "advertising" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Marknadsföringskostnader fullt avdragsgilla (IL 16:1)".to_string(),
            },
            "salary" | "payroll" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: false,
                notes: "Löner och arbetsgivaravgifter fullt avdragsgilla".to_string(),
            },
            "fines" | "penalties" => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: false,
                notes: "Böter och skattetillägg ej avdragsgilla (IL 9:9)".to_string(),
            },
            "donations" => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: true,
                notes: "Gåvor till välgörenhet ej avdragsgilla för företag i SE".to_string(),
            },
            _ => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Presumerat avdragsgill driftskostnad; bekräfta med revisor".to_string(),
            },
        }
    }

    fn deductibility_us(cat: &str) -> DeductibilityResult {
        match cat {
            "office_supplies" | "equipment" | "hardware" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Ordinary & necessary business expense (IRC § 162)".to_string(),
            },
            "software" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Section 179 immediate expensing or 3-year amortisation (IRC § 167)".to_string(),
            },
            "travel" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Business travel deductible; document business purpose (IRC § 162)".to_string(),
            },
            "meals" | "restaurant" => DeductibilityResult {
                deductible: true,
                rate: dec!(0.5),
                requires_receipt: true,
                notes: "50 % meal deduction (IRC § 274(n)); document business purpose".to_string(),
            },
            "car" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "IRS standard mileage rate or actual-expense method (IRC § 179)".to_string(),
            },
            "marketing" | "advertising" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Ordinary advertising expense (IRC § 162)".to_string(),
            },
            "salary" | "payroll" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: false,
                notes: "Compensation deductible if reasonable (IRC § 162(a)(1))".to_string(),
            },
            "fines" | "penalties" => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: false,
                notes: "Fines/penalties to government not deductible (IRC § 162(f))".to_string(),
            },
            "donations" => DeductibilityResult {
                deductible: true,
                rate: dec!(0.10), // C-corps: up to 10 % of taxable income
                requires_receipt: true,
                notes: "Charitable contributions deductible up to 10 % of taxable income (IRC § 170)".to_string(),
            },
            _ => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Ordinary & necessary expense; confirm with CPA (IRC § 162)".to_string(),
            },
        }
    }

    fn deductibility_gb(cat: &str) -> DeductibilityResult {
        match cat {
            "office_supplies" | "equipment" | "hardware" | "software" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Wholly & exclusively for trade (ICTA 1988 s.74 / CTA 2009 s.54)".to_string(),
            },
            "travel" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Business travel deductible; document purpose (ITEPA 2003 s.337)".to_string(),
            },
            "meals" | "restaurant" => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: true,
                notes: "Entertainment/meals generally not deductible (CTA 2009 s.1298)".to_string(),
            },
            "car" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Capital allowances on business vehicles; HMRC mileage rates for cars".to_string(),
            },
            "marketing" | "advertising" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Wholly & exclusively trade advertising (CTA 2009 s.54)".to_string(),
            },
            "salary" | "payroll" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: false,
                notes: "Staff costs fully deductible (CTA 2009 s.54)".to_string(),
            },
            "fines" | "penalties" => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: false,
                notes: "Regulatory penalties not deductible (CTA 2009 s.1304)".to_string(),
            },
            "donations" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Gift Aid donations deductible (CTA 2010 s.189)".to_string(),
            },
            _ => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Wholly & exclusively rule applies; confirm with accountant".to_string(),
            },
        }
    }

    fn deductibility_eu(cat: &str) -> DeductibilityResult {
        // EU-level: use broadly accepted OECD / EU acquis principles.
        match cat {
            "office_supplies" | "equipment" | "hardware" | "software" | "marketing"
            | "advertising" | "salary" | "payroll" => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Business expense deductible under applicable member-state corporate tax law".to_string(),
            },
            "meals" | "restaurant" => DeductibilityResult {
                deductible: true,
                rate: dec!(0.5),
                requires_receipt: true,
                notes: "Entertainment typically 50 % deductible; varies by member state".to_string(),
            },
            "fines" | "penalties" => DeductibilityResult {
                deductible: false,
                rate: Decimal::ZERO,
                requires_receipt: false,
                notes: "Regulatory fines not deductible under EU member-state tax laws".to_string(),
            },
            _ => DeductibilityResult {
                deductible: true,
                rate: dec!(1.0),
                requires_receipt: true,
                notes: "Confirm with local member-state tax advisor".to_string(),
            },
        }
    }

    /// Return all material filing deadlines for a jurisdiction in a given calendar year.
    pub fn filing_deadlines(&self, jurisdiction: &str, year: i32) -> Vec<FilingDeadline> {
        let jur = jurisdiction.to_uppercase();
        match jur.as_str() {
            "SE" => Self::deadlines_se(year),
            "US" => Self::deadlines_us(year),
            "GB" => Self::deadlines_gb(year),
            "EU" => Self::deadlines_eu(year),
            _ => vec![],
        }
    }

    // SE – Swedish tax deadlines
    fn deadlines_se(year: i32) -> Vec<FilingDeadline> {
        vec![
            // Moms quarterly (momsperioder): deklaration + betalning senast den 12 i månaden
            // efter kvartalets slut (eller 26 feb för Q4 föregående år).
            FilingDeadline {
                name: "Moms Q1".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 5, 12).unwrap(),
                jurisdiction: "SE".to_string(),
                filing_type: FilingType::Moms,
            },
            FilingDeadline {
                name: "Moms Q2".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 8, 12).unwrap(),
                jurisdiction: "SE".to_string(),
                filing_type: FilingType::Moms,
            },
            FilingDeadline {
                name: "Moms Q3".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 11, 12).unwrap(),
                jurisdiction: "SE".to_string(),
                filing_type: FilingType::Moms,
            },
            // Q4 redovisas i februari nästa år
            FilingDeadline {
                name: "Moms Q4".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 2, 26).unwrap(),
                jurisdiction: "SE".to_string(),
                filing_type: FilingType::Moms,
            },
            // Inkomstdeklaration (INK2 för aktiebolag): senast 1 juli för kalenderår
            FilingDeadline {
                name: "Inkomstdeklaration".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 7, 1).unwrap(),
                jurisdiction: "SE".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            // Arbetsgivardeklaration (AGD): månadsvis, 12 per år – representeras av det
            // sista i december som årsmarkering.
            FilingDeadline {
                name: "Arbetsgivardeklaration (dec)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 1, 12).unwrap(),
                jurisdiction: "SE".to_string(),
                filing_type: FilingType::Payroll,
            },
        ]
    }

    // US – Federal tax deadlines
    fn deadlines_us(year: i32) -> Vec<FilingDeadline> {
        vec![
            // Quarterly estimated tax payments (Form 1040-ES / 1120-W)
            FilingDeadline {
                name: "Q1 Estimated Tax".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 4, 15).unwrap(),
                jurisdiction: "US".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            FilingDeadline {
                name: "Q2 Estimated Tax".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 6, 15).unwrap(),
                jurisdiction: "US".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            FilingDeadline {
                name: "Q3 Estimated Tax".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 9, 15).unwrap(),
                jurisdiction: "US".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            FilingDeadline {
                name: "Q4 Estimated Tax".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 1, 15).unwrap(),
                jurisdiction: "US".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            // C-Corp annual return Form 1120 (15 April for calendar-year corps)
            FilingDeadline {
                name: "Corporate Income Tax Return (Form 1120)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 4, 15).unwrap(),
                jurisdiction: "US".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            // Payroll deposits – quarterly Form 941 deadline
            FilingDeadline {
                name: "Payroll Form 941 Q4".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 1, 31).unwrap(),
                jurisdiction: "US".to_string(),
                filing_type: FilingType::Payroll,
            },
        ]
    }

    // GB – HMRC deadlines (Making Tax Digital)
    fn deadlines_gb(year: i32) -> Vec<FilingDeadline> {
        // UK tax year runs 6 April – 5 April. 
        // VAT quarters most commonly end Mar/Jun/Sep/Dec for calendar-aligned businesses.
        vec![
            FilingDeadline {
                name: "VAT Return Q1 (Jan–Mar)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 5, 7).unwrap(),
                jurisdiction: "GB".to_string(),
                filing_type: FilingType::VAT,
            },
            FilingDeadline {
                name: "VAT Return Q2 (Apr–Jun)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 8, 7).unwrap(),
                jurisdiction: "GB".to_string(),
                filing_type: FilingType::VAT,
            },
            FilingDeadline {
                name: "VAT Return Q3 (Jul–Sep)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 11, 7).unwrap(),
                jurisdiction: "GB".to_string(),
                filing_type: FilingType::VAT,
            },
            FilingDeadline {
                name: "VAT Return Q4 (Oct–Dec)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 2, 7).unwrap(),
                jurisdiction: "GB".to_string(),
                filing_type: FilingType::VAT,
            },
            // Corporation Tax: 9 months + 1 day after accounting period end
            // Modelled for a 31 Dec year-end → 1 Oct of following year
            FilingDeadline {
                name: "Corporation Tax Payment".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 10, 1).unwrap(),
                jurisdiction: "GB".to_string(),
                filing_type: FilingType::IncomeTax,
            },
            // Company Tax Return (CT600): 12 months after accounting period
            FilingDeadline {
                name: "Corporation Tax Return (CT600)".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 12, 31).unwrap(),
                jurisdiction: "GB".to_string(),
                filing_type: FilingType::IncomeTax,
            },
        ]
    }

    // EU – OSS quarterly returns
    fn deadlines_eu(year: i32) -> Vec<FilingDeadline> {
        // OSS returns are filed in the member state of identification, due by the last day
        // of the month following the quarter end (EU VAT Directive Art. 364 / 369g).
        vec![
            FilingDeadline {
                name: "OSS VAT Return Q1".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 4, 30).unwrap(),
                jurisdiction: "EU".to_string(),
                filing_type: FilingType::VAT,
            },
            FilingDeadline {
                name: "OSS VAT Return Q2".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 7, 31).unwrap(),
                jurisdiction: "EU".to_string(),
                filing_type: FilingType::VAT,
            },
            FilingDeadline {
                name: "OSS VAT Return Q3".to_string(),
                due_date: NaiveDate::from_ymd_opt(year, 10, 31).unwrap(),
                jurisdiction: "EU".to_string(),
                filing_type: FilingType::VAT,
            },
            FilingDeadline {
                name: "OSS VAT Return Q4".to_string(),
                due_date: NaiveDate::from_ymd_opt(year + 1, 1, 31).unwrap(),
                jurisdiction: "EU".to_string(),
                filing_type: FilingType::VAT,
            },
        ]
    }

    // ------------------------------------------------------------------
    // Utilities
    // ------------------------------------------------------------------

    /// Return all loaded jurisdiction codes.
    pub fn jurisdictions(&self) -> Vec<&str> {
        let mut codes: Vec<&str> = self.rules.keys().map(|s| s.as_str()).collect();
        codes.sort();
        codes
    }

    /// Retrieve the raw rules for a jurisdiction if present.
    pub fn rules_for(&self, jurisdiction: &str) -> Option<&JurisdictionRules> {
        self.rules.get(&jurisdiction.to_uppercase())
    }
}

impl Default for JurisdictionEngine {
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

    fn engine() -> JurisdictionEngine {
        JurisdictionEngine::new()
    }

    #[test]
    fn test_se_vat_standard() {
        let e = engine();
        assert_eq!(e.vat_rate("SE", "goods"), dec!(0.25));
    }

    #[test]
    fn test_se_vat_food() {
        let e = engine();
        assert_eq!(e.vat_rate("SE", "food"), dec!(0.12));
    }

    #[test]
    fn test_se_vat_books() {
        let e = engine();
        assert_eq!(e.vat_rate("SE", "books"), dec!(0.06));
    }

    #[test]
    fn test_gb_vat_standard() {
        let e = engine();
        assert_eq!(e.vat_rate("GB", "services"), dec!(0.20));
    }

    #[test]
    fn test_gb_vat_zero_food() {
        let e = engine();
        assert_eq!(e.vat_rate("GB", "food"), dec!(0.00));
    }

    #[test]
    fn test_validate_se_vat_number() {
        let e = engine();
        assert!(e.validate_vat_number("SE556012345601", "SE"));
        assert!(!e.validate_vat_number("556012345601", "SE")); // missing prefix
    }

    #[test]
    fn test_validate_gb_vat_number() {
        let e = engine();
        assert!(e.validate_vat_number("GB123456789", "GB"));
        assert!(!e.validate_vat_number("123456789", "GB"));
    }

    #[test]
    fn test_retention_se_invoice() {
        let e = engine();
        assert_eq!(e.retention_years("invoice", "SE"), 7);
    }

    #[test]
    fn test_retention_se_payroll() {
        let e = engine();
        assert_eq!(e.retention_years("payroll", "SE"), 10);
    }

    #[test]
    fn test_retention_gb_payroll() {
        let e = engine();
        assert_eq!(e.retention_years("payroll", "GB"), 3);
    }

    #[test]
    fn test_deductible_se_meals() {
        let e = engine();
        let r = e.is_deductible("meals", "SE");
        assert!(r.deductible);
        assert_eq!(r.rate, dec!(0.5));
    }

    #[test]
    fn test_deductible_us_fines() {
        let e = engine();
        let r = e.is_deductible("fines", "US");
        assert!(!r.deductible);
    }

    #[test]
    fn test_filing_deadlines_se_count() {
        let e = engine();
        let deadlines = e.filing_deadlines("SE", 2025);
        // 4 moms + 1 inkomst + 1 payroll = 6
        assert_eq!(deadlines.len(), 6);
    }

    #[test]
    fn test_filing_deadlines_eu_q1() {
        let e = engine();
        let deadlines = e.filing_deadlines("EU", 2025);
        let q1 = deadlines.iter().find(|d| d.name.contains("Q1")).unwrap();
        assert_eq!(q1.due_date, NaiveDate::from_ymd_opt(2025, 4, 30).unwrap());
    }

    #[test]
    fn test_unknown_jurisdiction_returns_zero_vat() {
        let e = engine();
        assert_eq!(e.vat_rate("XX", "goods"), Decimal::ZERO);
    }
}
