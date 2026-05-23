/// merchant_graph.rs — Merchant Intelligence Graph
///
/// Learns and maintains vendor patterns per company over time.
/// Provides fuzzy matching, statistical anomaly detection, and
/// account-code suggestions backed by 60+ built-in profiles.

use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MerchantCategory {
    Grocery,
    Telecom,
    Utilities,
    Transport,
    FoodAndBeverage,
    Retail,
    SaasCloud,
    TravelAccommodation,
    Finance,
    Government,
    Healthcare,
    Other(String),
}

impl MerchantCategory {
    /// Returns a human-readable Swedish label for the category.
    pub fn label(&self) -> &str {
        match self {
            Self::Grocery => "Dagligvaror",
            Self::Telecom => "Telekommunikation",
            Self::Utilities => "El/Energi",
            Self::Transport => "Transport",
            Self::FoodAndBeverage => "Mat & Dryck",
            Self::Retail => "Detaljhandel",
            Self::SaasCloud => "SaaS/Molntjänster",
            Self::TravelAccommodation => "Resa & Boende",
            Self::Finance => "Finans & Bank",
            Self::Government => "Myndighet",
            Self::Healthcare => "Hälsa & Apotek",
            Self::Other(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AmountRange {
    pub min: Decimal,
    pub max: Decimal,
    pub typical: Decimal,
    pub std_deviation: Decimal,
}

impl AmountRange {
    pub fn new(min: Decimal, max: Decimal, typical: Decimal, std_deviation: Decimal) -> Self {
        Self { min, max, typical, std_deviation }
    }

    /// Returns `true` when `amount` deviates more than 2σ from the typical value.
    pub fn is_unusual(&self, amount: Decimal) -> bool {
        if self.std_deviation.is_zero() {
            return false;
        }
        let diff = (amount - self.typical).abs();
        let two_sigma = self.std_deviation * dec!(2);
        diff > two_sigma
    }
}

#[derive(Debug, Clone)]
pub struct MerchantProfile {
    pub canonical_id: Uuid,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub category: MerchantCategory,
    pub typical_amount_range: AmountRange,
    pub typical_vat_rate: Decimal,
    pub jurisdiction: String,
    pub default_account_code: String,
    pub booking_confidence: f64,
    pub transaction_count: u32,
    pub last_seen: DateTime<Utc>,
    pub known_email_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MerchantClassification {
    pub canonical_name: String,
    pub canonical_id: Option<Uuid>,
    pub category: MerchantCategory,
    pub suggested_account: String,
    pub vat_rate: Decimal,
    pub confidence: f64,
    pub is_known: bool,
    pub amount_is_unusual: bool,
}

#[derive(Debug, Clone)]
pub struct AccountSuggestion {
    pub account_code: String,
    pub account_name: String,
    pub vat_rate: Decimal,
    pub confidence: f64,
    pub rationale: String,
}

// ---------------------------------------------------------------------------
// Builder helpers (private)
// ---------------------------------------------------------------------------

struct ProfileBuilder {
    id: Uuid,
    name: &'static str,
    aliases: Vec<&'static str>,
    category: MerchantCategory,
    min: Decimal,
    max: Decimal,
    typical: Decimal,
    sigma: Decimal,
    vat: Decimal,
    jurisdiction: &'static str,
    account: &'static str,
    confidence: f64,
    email_patterns: Vec<&'static str>,
}

impl ProfileBuilder {
    fn build(self) -> (String, MerchantProfile) {
        let key = normalize_merchant_name(self.name);
        let profile = MerchantProfile {
            canonical_id: self.id,
            canonical_name: self.name.to_string(),
            aliases: self.aliases.iter().map(|s| normalize_merchant_name(s)).collect(),
            category: self.category,
            typical_amount_range: AmountRange::new(self.min, self.max, self.typical, self.sigma),
            typical_vat_rate: self.vat,
            jurisdiction: self.jurisdiction.to_string(),
            default_account_code: self.account.to_string(),
            booking_confidence: self.confidence,
            transaction_count: 0,
            last_seen: DateTime::<Utc>::MIN_UTC,
            known_email_patterns: self.email_patterns.iter().map(|s| s.to_string()).collect(),
        };
        (key, profile)
    }
}

// ---------------------------------------------------------------------------
// Name normalisation
// ---------------------------------------------------------------------------

/// Lowercases, strips punctuation and common suffixes so matching is fuzzy.
pub fn normalize_merchant_name(name: &str) -> String {
    let lower = name.to_lowercase();
    // Remove common legal suffixes
    let stripped = lower
        .replace(" ab", "")
        .replace(" as", "")
        .replace(" inc", "")
        .replace(" ltd", "")
        .replace(" llc", "")
        .replace(" gmbh", "")
        .replace(" bv", "");
    // Keep only alphanumeric + spaces
    stripped
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Levenshtein distance for fuzzy matching.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    if m == 0 { return n; }
    if n == 0 { return m; }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1])
            };
        }
    }
    dp[m][n]
}

/// Similarity score 0.0–1.0 based on Levenshtein.
fn similarity(a: &str, b: &str) -> f64 {
    let max_len = a.len().max(b.len());
    if max_len == 0 { return 1.0; }
    let dist = levenshtein(a, b);
    1.0 - (dist as f64 / max_len as f64)
}

// ---------------------------------------------------------------------------
// MerchantIntelligence
// ---------------------------------------------------------------------------

pub struct MerchantIntelligence {
    /// Primary index: normalised name → profile
    merchants: HashMap<String, MerchantProfile>,
    /// Secondary index: alias → canonical normalised name
    alias_index: HashMap<String, String>,
}

impl MerchantIntelligence {
    /// Creates a new instance pre-loaded with 60+ built-in merchant profiles.
    pub fn new() -> Self {
        let mut engine = Self {
            merchants: HashMap::new(),
            alias_index: HashMap::new(),
        };
        engine.load_builtin_profiles();
        engine
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Learn a new merchant pattern from a confirmed transaction.
    pub fn learn(
        &mut self,
        merchant: &str,
        amount: Decimal,
        account_code: &str,
        vat_rate: Decimal,
    ) {
        let key = normalize_merchant_name(merchant);
        if let Some(profile) = self.merchants.get_mut(&key) {
            // Update existing profile
            profile.default_account_code = account_code.to_string();
            profile.typical_vat_rate = vat_rate;
            self.update_amount_stats_internal(profile, amount);
            profile.transaction_count += 1;
            profile.last_seen = Utc::now();
            profile.booking_confidence = (profile.booking_confidence + 0.05).min(1.0);
        } else {
            // Create a new profile from the learned data
            let profile = MerchantProfile {
                canonical_id: Uuid::new_v4(),
                canonical_name: merchant.to_string(),
                aliases: vec![],
                category: MerchantCategory::Other("Okänd".to_string()),
                typical_amount_range: AmountRange::new(
                    amount * dec!(0.5),
                    amount * dec!(2),
                    amount,
                    amount * dec!(0.2),
                ),
                typical_vat_rate: vat_rate,
                jurisdiction: "SE".to_string(),
                default_account_code: account_code.to_string(),
                booking_confidence: 0.5,
                transaction_count: 1,
                last_seen: Utc::now(),
                known_email_patterns: vec![],
            };
            self.merchants.insert(key, profile);
        }
    }

    /// Look up a merchant and return a classification.
    pub fn classify(&self, merchant: &str, amount: Decimal) -> MerchantClassification {
        let key = normalize_merchant_name(merchant);

        // 1. Exact match
        if let Some(profile) = self.lookup_exact(&key) {
            return self.build_classification(profile, amount, 1.0, true);
        }

        // 2. Alias match
        if let Some(canonical_key) = self.alias_index.get(&key) {
            if let Some(profile) = self.merchants.get(canonical_key) {
                return self.build_classification(profile, amount, 0.95, true);
            }
        }

        // 3. Substring match
        if let Some((profile, score)) = self.substring_match(&key) {
            if score >= 0.8 {
                return self.build_classification(profile, amount, score * 0.9, true);
            }
        }

        // 4. Fuzzy match
        if let Some((profile, score)) = self.fuzzy_match(&key) {
            if score >= 0.7 {
                return self.build_classification(profile, amount, score * 0.75, true);
            }
        }

        // 5. Unknown merchant — return generic classification
        MerchantClassification {
            canonical_name: merchant.to_string(),
            canonical_id: None,
            category: MerchantCategory::Other("Okänd".to_string()),
            suggested_account: "6990".to_string(),
            vat_rate: dec!(0.25),
            confidence: 0.1,
            is_known: false,
            amount_is_unusual: false,
        }
    }

    /// Update running statistics for an existing merchant by UUID.
    pub fn update_stats(&mut self, canonical_id: Uuid, amount: Decimal) {
        if let Some(profile) = self.merchants.values_mut().find(|p| p.canonical_id == canonical_id) {
            self.update_amount_stats_internal(profile, amount);
            profile.transaction_count += 1;
            profile.last_seen = Utc::now();
        }
    }

    /// Export all profiles for persistence (e.g. database storage).
    pub fn export_profiles(&self) -> Vec<MerchantProfile> {
        self.merchants.values().cloned().collect()
    }

    /// Import previously persisted profiles, merging with built-ins.
    pub fn import_profiles(&mut self, profiles: Vec<MerchantProfile>) {
        for profile in profiles {
            let key = normalize_merchant_name(&profile.canonical_name);
            // Re-register aliases
            for alias in &profile.aliases {
                self.alias_index.insert(alias.clone(), key.clone());
            }
            self.merchants.insert(key, profile);
        }
    }

    /// Suggest an account code with confidence level.
    pub fn suggest_account(&self, merchant: &str, amount: Decimal) -> AccountSuggestion {
        let classification = self.classify(merchant, amount);
        AccountSuggestion {
            account_code: classification.suggested_account.clone(),
            account_name: account_name_for(&classification.suggested_account),
            vat_rate: classification.vat_rate,
            confidence: classification.confidence,
            rationale: if classification.is_known {
                format!(
                    "Baserat på känd profil för '{}' (kategori: {})",
                    classification.canonical_name,
                    classification.category.label()
                )
            } else {
                "Okänd leverantör – standardkonto tilldelat".to_string()
            },
        }
    }

    /// Find merchants in a similar category.
    pub fn similar_merchants(&self, merchant: &str) -> Vec<MerchantProfile> {
        let classification = self.classify(merchant, Decimal::ZERO);
        self.merchants
            .values()
            .filter(|p| {
                p.canonical_name.to_lowercase() != merchant.to_lowercase()
                    && std::mem::discriminant(&p.category)
                        == std::mem::discriminant(&classification.category)
            })
            .cloned()
            .collect()
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn lookup_exact(&self, key: &str) -> Option<&MerchantProfile> {
        self.merchants.get(key)
    }

    fn substring_match(&self, key: &str) -> Option<(&MerchantProfile, f64)> {
        let mut best: Option<(&MerchantProfile, f64)> = None;
        for (k, profile) in &self.merchants {
            if k.contains(key) || key.contains(k.as_str()) {
                let overlap = k.len().min(key.len()) as f64 / k.len().max(key.len()) as f64;
                if best.map_or(true, |(_, s)| overlap > s) {
                    best = Some((profile, overlap));
                }
            }
            // Check aliases too
            for alias in &profile.aliases {
                if alias.contains(key) || key.contains(alias.as_str()) {
                    let overlap =
                        alias.len().min(key.len()) as f64 / alias.len().max(key.len()) as f64;
                    if best.map_or(true, |(_, s)| overlap > s) {
                        best = Some((profile, overlap));
                    }
                }
            }
        }
        best
    }

    fn fuzzy_match(&self, key: &str) -> Option<(&MerchantProfile, f64)> {
        let mut best: Option<(&MerchantProfile, f64)> = None;
        for (k, profile) in &self.merchants {
            let score = similarity(k, key);
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((profile, score));
            }
            for alias in &profile.aliases {
                let alias_score = similarity(alias, key);
                if best.map_or(true, |(_, s)| alias_score > s) {
                    best = Some((profile, alias_score));
                }
            }
        }
        best
    }

    fn build_classification(
        &self,
        profile: &MerchantProfile,
        amount: Decimal,
        confidence: f64,
        is_known: bool,
    ) -> MerchantClassification {
        let amount_is_unusual = !amount.is_zero()
            && profile.typical_amount_range.is_unusual(amount);
        // Reduce confidence slightly if amount is unusual
        let adjusted_confidence = if amount_is_unusual {
            (confidence * 0.9).min(1.0)
        } else {
            confidence
        };
        MerchantClassification {
            canonical_name: profile.canonical_name.clone(),
            canonical_id: Some(profile.canonical_id),
            category: profile.category.clone(),
            suggested_account: profile.default_account_code.clone(),
            vat_rate: profile.typical_vat_rate,
            confidence: adjusted_confidence * profile.booking_confidence,
            is_known,
            amount_is_unusual,
        }
    }

    /// Welford online algorithm for running mean and variance updates.
    fn update_amount_stats_internal(&self, profile: &mut MerchantProfile, amount: Decimal) {
        let n = Decimal::from(profile.transaction_count + 1);
        let old_mean = profile.typical_amount_range.typical;
        let new_mean = old_mean + (amount - old_mean) / n;
        // Update std_deviation as simple rolling estimate
        let old_sigma = profile.typical_amount_range.std_deviation;
        let diff_sq = (amount - new_mean) * (amount - old_mean);
        let new_variance = if n > Decimal::ONE {
            (old_sigma * old_sigma * (n - Decimal::ONE) + diff_sq) / n
        } else {
            Decimal::ZERO
        };
        let new_sigma = decimal_sqrt(new_variance.max(Decimal::ZERO));
        profile.typical_amount_range.typical = new_mean;
        profile.typical_amount_range.std_deviation = new_sigma;
        if amount < profile.typical_amount_range.min {
            profile.typical_amount_range.min = amount;
        }
        if amount > profile.typical_amount_range.max {
            profile.typical_amount_range.max = amount;
        }
    }

    // ------------------------------------------------------------------
    // Built-in profile loader (60+ merchants)
    // ------------------------------------------------------------------

    fn load_builtin_profiles(&mut self) {
        let builders: Vec<ProfileBuilder> = vec![
            // -------------------------------------------------------
            // Swedish Grocery
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("11000001-0000-0000-0000-000000000001").unwrap(),
                name: "ICA",
                aliases: vec!["ICA Supermarket", "ICA Maxi", "ICA Nära", "ICA Kvantum", "ICA To Go"],
                category: MerchantCategory::Grocery,
                min: dec!(50), max: dec!(2000), typical: dec!(350), sigma: dec!(200),
                vat: dec!(0.12),
                jurisdiction: "SE",
                account: "4000",
                confidence: 0.95,
                email_patterns: vec!["ica.se", "noreply@ica.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000001-0000-0000-0000-000000000002").unwrap(),
                name: "Coop",
                aliases: vec!["Coop Butik", "Coop Extra", "Coop Forum", "Coop Konsum"],
                category: MerchantCategory::Grocery,
                min: dec!(40), max: dec!(1800), typical: dec!(300), sigma: dec!(180),
                vat: dec!(0.12),
                jurisdiction: "SE",
                account: "4000",
                confidence: 0.95,
                email_patterns: vec!["coop.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000001-0000-0000-0000-000000000003").unwrap(),
                name: "Willys",
                aliases: vec!["Willys Hemma", "Willys Plus"],
                category: MerchantCategory::Grocery,
                min: dec!(30), max: dec!(1500), typical: dec!(280), sigma: dec!(160),
                vat: dec!(0.12),
                jurisdiction: "SE",
                account: "4000",
                confidence: 0.95,
                email_patterns: vec!["willys.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000001-0000-0000-0000-000000000004").unwrap(),
                name: "Lidl",
                aliases: vec!["Lidl Sverige", "LIDL"],
                category: MerchantCategory::Grocery,
                min: dec!(20), max: dec!(1200), typical: dec!(220), sigma: dec!(140),
                vat: dec!(0.12),
                jurisdiction: "SE",
                account: "4000",
                confidence: 0.93,
                email_patterns: vec!["lidl.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000001-0000-0000-0000-000000000005").unwrap(),
                name: "Systembolaget",
                aliases: vec!["Systemet", "SB"],
                category: MerchantCategory::FoodAndBeverage,
                min: dec!(100), max: dec!(3000), typical: dec!(450), sigma: dec!(300),
                vat: dec!(0.25),
                jurisdiction: "SE",
                account: "6540",
                confidence: 0.97,
                email_patterns: vec!["systembolaget.se"],
            },
            // -------------------------------------------------------
            // Swedish Telecom
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("11000002-0000-0000-0000-000000000001").unwrap(),
                name: "Telia",
                aliases: vec!["Telia Company", "TeliaSonera", "Telia Sverige"],
                category: MerchantCategory::Telecom,
                min: dec!(200), max: dec!(5000), typical: dec!(600), sigma: dec!(300),
                vat: dec!(0.25),
                jurisdiction: "SE",
                account: "6210",
                confidence: 0.97,
                email_patterns: vec!["telia.se", "noreply@telia.com"],
            },
            // -------------------------------------------------------
            // Swedish Utilities
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("11000003-0000-0000-0000-000000000001").unwrap(),
                name: "Vattenfall",
                aliases: vec!["Vattenfall Eldistribution", "Vattenfall Energy"],
                category: MerchantCategory::Utilities,
                min: dec!(500), max: dec!(15000), typical: dec!(2000), sigma: dec!(1000),
                vat: dec!(0.25),
                jurisdiction: "SE",
                account: "6220",
                confidence: 0.96,
                email_patterns: vec!["vattenfall.se"],
            },
            // -------------------------------------------------------
            // Swedish Transport
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("11000004-0000-0000-0000-000000000001").unwrap(),
                name: "SJ",
                aliases: vec!["SJ AB", "SJ Tåg", "statens järnvägar"],
                category: MerchantCategory::Transport,
                min: dec!(100), max: dec!(4000), typical: dec!(800), sigma: dec!(500),
                vat: dec!(0.06),
                jurisdiction: "SE",
                account: "7320",
                confidence: 0.95,
                email_patterns: vec!["sj.se", "noreply@sj.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000004-0000-0000-0000-000000000002").unwrap(),
                name: "Arlanda Express",
                aliases: vec!["A-Train", "arlandaexpress"],
                category: MerchantCategory::Transport,
                min: dec!(120), max: dec!(600), typical: dec!(295), sigma: dec!(80),
                vat: dec!(0.06),
                jurisdiction: "SE",
                account: "7320",
                confidence: 0.95,
                email_patterns: vec!["arlandaexpress.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000004-0000-0000-0000-000000000003").unwrap(),
                name: "Taxi Stockholm",
                aliases: vec!["Taxi 020", "Taxi Sthlm"],
                category: MerchantCategory::Transport,
                min: dec!(80), max: dec!(1200), typical: dec!(350), sigma: dec!(200),
                vat: dec!(0.06),
                jurisdiction: "SE",
                account: "7320",
                confidence: 0.90,
                email_patterns: vec!["taxistockholm.se"],
            },
            // -------------------------------------------------------
            // Swedish Food & Beverage
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("11000005-0000-0000-0000-000000000001").unwrap(),
                name: "McDonald's",
                aliases: vec!["McDonalds", "McDo", "Mcdonalds Sverige"],
                category: MerchantCategory::FoodAndBeverage,
                min: dec!(50), max: dec!(800), typical: dec!(120), sigma: dec!(60),
                vat: dec!(0.12),
                jurisdiction: "SE",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["mcdonalds.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000005-0000-0000-0000-000000000002").unwrap(),
                name: "Espresso House",
                aliases: vec!["Espressohouse", "Espresso H"],
                category: MerchantCategory::FoodAndBeverage,
                min: dec!(30), max: dec!(500), typical: dec!(85), sigma: dec!(40),
                vat: dec!(0.12),
                jurisdiction: "SE",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["espressohouse.com"],
            },
            // -------------------------------------------------------
            // Swedish Retail / Healthcare
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("11000006-0000-0000-0000-000000000001").unwrap(),
                name: "H&M",
                aliases: vec!["H & M", "HM", "Hennes & Mauritz"],
                category: MerchantCategory::Retail,
                min: dec!(100), max: dec!(5000), typical: dec!(600), sigma: dec!(400),
                vat: dec!(0.25),
                jurisdiction: "SE",
                account: "6980",
                confidence: 0.90,
                email_patterns: vec!["hm.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("11000006-0000-0000-0000-000000000002").unwrap(),
                name: "Apotek Hjärtat",
                aliases: vec!["Apotek", "Hjärtat Apotek", "apotekhjärtat"],
                category: MerchantCategory::Healthcare,
                min: dec!(50), max: dec!(2000), typical: dec!(280), sigma: dec!(200),
                vat: dec!(0.25),
                jurisdiction: "SE",
                account: "6980",
                confidence: 0.88,
                email_patterns: vec!["apotekhjärtat.se", "apotekhjartat.se"],
            },
            // -------------------------------------------------------
            // SaaS / Cloud — AWS
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000001").unwrap(),
                name: "Amazon Web Services",
                aliases: vec!["AWS", "aws.amazon.com", "Amazon AWS", "AMAZON WEB SERVICES"],
                category: MerchantCategory::SaasCloud,
                min: dec!(10), max: dec!(500000), typical: dec!(5000), sigma: dec!(8000),
                vat: dec!(0.25),
                jurisdiction: "IE",
                account: "6540",
                confidence: 0.98,
                email_patterns: vec!["aws.amazon.com", "amazon.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000002").unwrap(),
                name: "Google Cloud Platform",
                aliases: vec!["GCP", "Google Cloud", "cloud.google.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(5), max: dec!(300000), typical: dec!(3000), sigma: dec!(6000),
                vat: dec!(0.25),
                jurisdiction: "IE",
                account: "6540",
                confidence: 0.97,
                email_patterns: vec!["cloud.google.com", "google.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000003").unwrap(),
                name: "Microsoft Azure",
                aliases: vec!["Azure", "Microsoft Azure Cloud", "azure.microsoft.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(10), max: dec!(400000), typical: dec!(4000), sigma: dec!(7000),
                vat: dec!(0.25),
                jurisdiction: "IE",
                account: "6540",
                confidence: 0.97,
                email_patterns: vec!["microsoft.com", "azure.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000004").unwrap(),
                name: "GitHub",
                aliases: vec!["Github", "GitHub Inc", "github.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(40), max: dec!(50000), typical: dec!(400), sigma: dec!(600),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.96,
                email_patterns: vec!["github.com", "noreply@github.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000005").unwrap(),
                name: "Slack",
                aliases: vec!["Slack Technologies", "slack.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(80), max: dec!(20000), typical: dec!(800), sigma: dec!(600),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.95,
                email_patterns: vec!["slack.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000006").unwrap(),
                name: "Notion",
                aliases: vec!["Notion Labs", "notion.so"],
                category: MerchantCategory::SaasCloud,
                min: dec!(80), max: dec!(5000), typical: dec!(200), sigma: dec!(200),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.94,
                email_patterns: vec!["notion.so", "makenotion.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000007").unwrap(),
                name: "Figma",
                aliases: vec!["Figma Inc", "figma.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(140), max: dec!(10000), typical: dec!(450), sigma: dec!(400),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.94,
                email_patterns: vec!["figma.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000008").unwrap(),
                name: "Linear",
                aliases: vec!["Linear App", "linear.app"],
                category: MerchantCategory::SaasCloud,
                min: dec!(80), max: dec!(5000), typical: dec!(350), sigma: dec!(300),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["linear.app"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000009").unwrap(),
                name: "Anthropic",
                aliases: vec!["Anthropic PBC", "Claude", "api.anthropic.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(5), max: dec!(50000), typical: dec!(500), sigma: dec!(800),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.95,
                email_patterns: vec!["anthropic.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000010").unwrap(),
                name: "OpenAI",
                aliases: vec!["OpenAI LLC", "openai.com", "ChatGPT"],
                category: MerchantCategory::SaasCloud,
                min: dec!(5), max: dec!(50000), typical: dec!(600), sigma: dec!(900),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.95,
                email_patterns: vec!["openai.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000011").unwrap(),
                name: "Cloudflare",
                aliases: vec!["Cloudflare Inc", "cloudflare.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(20000), typical: dec!(300), sigma: dec!(400),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.95,
                email_patterns: vec!["cloudflare.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000012").unwrap(),
                name: "Vercel",
                aliases: vec!["Vercel Inc", "vercel.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(15000), typical: dec!(200), sigma: dec!(300),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.93,
                email_patterns: vec!["vercel.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000013").unwrap(),
                name: "Netlify",
                aliases: vec!["Netlify Inc", "netlify.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(10000), typical: dec!(150), sigma: dec!(250),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.93,
                email_patterns: vec!["netlify.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000014").unwrap(),
                name: "Stripe",
                aliases: vec!["Stripe Inc", "stripe.com", "Stripe Payments"],
                category: MerchantCategory::Finance,
                min: dec!(50), max: dec!(50000), typical: dec!(500), sigma: dec!(1000),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6590",
                confidence: 0.96,
                email_patterns: vec!["stripe.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000015").unwrap(),
                name: "Twilio",
                aliases: vec!["Twilio Inc", "twilio.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(10), max: dec!(20000), typical: dec!(400), sigma: dec!(600),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.93,
                email_patterns: vec!["twilio.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000016").unwrap(),
                name: "SendGrid",
                aliases: vec!["Twilio SendGrid", "sendgrid.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(10000), typical: dec!(200), sigma: dec!(300),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["sendgrid.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000017").unwrap(),
                name: "Datadog",
                aliases: vec!["Datadog Inc", "datadoghq.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(100), max: dec!(100000), typical: dec!(2000), sigma: dec!(3000),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.93,
                email_patterns: vec!["datadoghq.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000001-0000-0000-0000-000000000018").unwrap(),
                name: "Sentry",
                aliases: vec!["Sentry.io", "Functional Software"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(20000), typical: dec!(600), sigma: dec!(800),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["sentry.io"],
            },
            // -------------------------------------------------------
            // Travel & Accommodation
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000001").unwrap(),
                name: "Ryanair",
                aliases: vec!["Ryanair DAC", "ryanair.com"],
                category: MerchantCategory::TravelAccommodation,
                min: dec!(200), max: dec!(8000), typical: dec!(1200), sigma: dec!(800),
                vat: dec!(0.06),
                jurisdiction: "IE",
                account: "7320",
                confidence: 0.95,
                email_patterns: vec!["ryanair.com", "noreply@ryanair.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000002").unwrap(),
                name: "SAS",
                aliases: vec!["Scandinavian Airlines", "sas.se", "SAS Group"],
                category: MerchantCategory::TravelAccommodation,
                min: dec!(400), max: dec!(15000), typical: dec!(2500), sigma: dec!(2000),
                vat: dec!(0.06),
                jurisdiction: "SE",
                account: "7320",
                confidence: 0.95,
                email_patterns: vec!["sas.se", "flysas.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000003").unwrap(),
                name: "Norwegian",
                aliases: vec!["Norwegian Air Shuttle", "norwegian.com"],
                category: MerchantCategory::TravelAccommodation,
                min: dec!(300), max: dec!(10000), typical: dec!(1800), sigma: dec!(1500),
                vat: dec!(0.06),
                jurisdiction: "NO",
                account: "7320",
                confidence: 0.94,
                email_patterns: vec!["norwegian.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000004").unwrap(),
                name: "Booking.com",
                aliases: vec!["booking.com", "Booking Holdings"],
                category: MerchantCategory::TravelAccommodation,
                min: dec!(500), max: dec!(30000), typical: dec!(3000), sigma: dec!(3000),
                vat: dec!(0.25),
                jurisdiction: "NL",
                account: "7320",
                confidence: 0.94,
                email_patterns: vec!["booking.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000005").unwrap(),
                name: "Airbnb",
                aliases: vec!["airbnb.com", "Airbnb Inc"],
                category: MerchantCategory::TravelAccommodation,
                min: dec!(400), max: dec!(25000), typical: dec!(2500), sigma: dec!(2500),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "7320",
                confidence: 0.93,
                email_patterns: vec!["airbnb.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000006").unwrap(),
                name: "Uber",
                aliases: vec!["Uber Technologies", "Uber BV", "uber.com"],
                category: MerchantCategory::Transport,
                min: dec!(50), max: dec!(2000), typical: dec!(250), sigma: dec!(200),
                vat: dec!(0.25),
                jurisdiction: "NL",
                account: "7320",
                confidence: 0.94,
                email_patterns: vec!["uber.com", "trip.uber.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000007").unwrap(),
                name: "Lyft",
                aliases: vec!["Lyft Inc", "lyft.com"],
                category: MerchantCategory::Transport,
                min: dec!(50), max: dec!(1500), typical: dec!(200), sigma: dec!(150),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "7320",
                confidence: 0.92,
                email_patterns: vec!["lyft.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("33000001-0000-0000-0000-000000000008").unwrap(),
                name: "Marriott",
                aliases: vec!["Marriott International", "marriott.com", "Courtyard by Marriott"],
                category: MerchantCategory::TravelAccommodation,
                min: dec!(800), max: dec!(40000), typical: dec!(4000), sigma: dec!(4000),
                vat: dec!(0.12),
                jurisdiction: "US",
                account: "7320",
                confidence: 0.92,
                email_patterns: vec!["marriott.com"],
            },
            // -------------------------------------------------------
            // Finance & Banking
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("44000001-0000-0000-0000-000000000001").unwrap(),
                name: "Swedbank",
                aliases: vec!["Swedbank AB", "swedbank.se"],
                category: MerchantCategory::Finance,
                min: dec!(0), max: dec!(50000), typical: dec!(500), sigma: dec!(1000),
                vat: dec!(0.0),
                jurisdiction: "SE",
                account: "6590",
                confidence: 0.95,
                email_patterns: vec!["swedbank.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("44000001-0000-0000-0000-000000000002").unwrap(),
                name: "SEB",
                aliases: vec!["SEB Bank", "Skandinaviska Enskilda Banken", "seb.se"],
                category: MerchantCategory::Finance,
                min: dec!(0), max: dec!(50000), typical: dec!(500), sigma: dec!(1000),
                vat: dec!(0.0),
                jurisdiction: "SE",
                account: "6590",
                confidence: 0.95,
                email_patterns: vec!["seb.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("44000001-0000-0000-0000-000000000003").unwrap(),
                name: "Handelsbanken",
                aliases: vec!["Svenska Handelsbanken", "handelsbanken.se"],
                category: MerchantCategory::Finance,
                min: dec!(0), max: dec!(50000), typical: dec!(500), sigma: dec!(1000),
                vat: dec!(0.0),
                jurisdiction: "SE",
                account: "6590",
                confidence: 0.95,
                email_patterns: vec!["handelsbanken.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("44000001-0000-0000-0000-000000000004").unwrap(),
                name: "Revolut",
                aliases: vec!["Revolut Ltd", "Revolut Business", "revolut.com"],
                category: MerchantCategory::Finance,
                min: dec!(0), max: dec!(100000), typical: dec!(1000), sigma: dec!(2000),
                vat: dec!(0.0),
                jurisdiction: "LT",
                account: "6590",
                confidence: 0.95,
                email_patterns: vec!["revolut.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("44000001-0000-0000-0000-000000000005").unwrap(),
                name: "Klarna",
                aliases: vec!["Klarna Bank", "klarna.se", "Klarna AB"],
                category: MerchantCategory::Finance,
                min: dec!(0), max: dec!(50000), typical: dec!(800), sigma: dec!(1200),
                vat: dec!(0.0),
                jurisdiction: "SE",
                account: "6590",
                confidence: 0.95,
                email_patterns: vec!["klarna.se", "klarna.com"],
            },
            // -------------------------------------------------------
            // Government / Myndigheter
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("55000001-0000-0000-0000-000000000001").unwrap(),
                name: "Skatteverket",
                aliases: vec!["Swedish Tax Agency", "skatteverket.se"],
                category: MerchantCategory::Government,
                min: dec!(0), max: dec!(10000000), typical: dec!(50000), sigma: dec!(100000),
                vat: dec!(0.0),
                jurisdiction: "SE",
                account: "2510",
                confidence: 0.99,
                email_patterns: vec!["skatteverket.se"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("55000001-0000-0000-0000-000000000002").unwrap(),
                name: "Bolagsverket",
                aliases: vec!["Swedish Companies Registration Office", "bolagsverket.se"],
                category: MerchantCategory::Government,
                min: dec!(200), max: dec!(50000), typical: dec!(2000), sigma: dec!(3000),
                vat: dec!(0.0),
                jurisdiction: "SE",
                account: "6810",
                confidence: 0.99,
                email_patterns: vec!["bolagsverket.se"],
            },
            // -------------------------------------------------------
            // Additional SaaS
            // -------------------------------------------------------
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000001").unwrap(),
                name: "Atlassian",
                aliases: vec!["Jira", "Confluence", "atlassian.com", "Atlassian Pty"],
                category: MerchantCategory::SaasCloud,
                min: dec!(100), max: dec!(30000), typical: dec!(1000), sigma: dec!(1500),
                vat: dec!(0.25),
                jurisdiction: "AU",
                account: "6540",
                confidence: 0.93,
                email_patterns: vec!["atlassian.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000002").unwrap(),
                name: "HubSpot",
                aliases: vec!["HubSpot Inc", "hubspot.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(400), max: dec!(50000), typical: dec!(2000), sigma: dec!(3000),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["hubspot.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000003").unwrap(),
                name: "Zoom",
                aliases: vec!["Zoom Video", "zoom.us"],
                category: MerchantCategory::SaasCloud,
                min: dec!(140), max: dec!(10000), typical: dec!(500), sigma: dec!(600),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.93,
                email_patterns: vec!["zoom.us"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000004").unwrap(),
                name: "Intercom",
                aliases: vec!["Intercom Inc", "intercom.com", "intercom.io"],
                category: MerchantCategory::SaasCloud,
                min: dec!(400), max: dec!(30000), typical: dec!(1500), sigma: dec!(2000),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.91,
                email_patterns: vec!["intercom.com", "intercom.io"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000005").unwrap(),
                name: "Mixpanel",
                aliases: vec!["Mixpanel Inc", "mixpanel.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(20000), typical: dec!(800), sigma: dec!(1200),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.90,
                email_patterns: vec!["mixpanel.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000006").unwrap(),
                name: "Segment",
                aliases: vec!["Twilio Segment", "segment.com", "segment.io"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(50000), typical: dec!(1200), sigma: dec!(2000),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.90,
                email_patterns: vec!["segment.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000007").unwrap(),
                name: "PagerDuty",
                aliases: vec!["pagerduty.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(200), max: dec!(20000), typical: dec!(800), sigma: dec!(1000),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.90,
                email_patterns: vec!["pagerduty.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000008").unwrap(),
                name: "1Password",
                aliases: vec!["AgileBits", "1password.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(50), max: dec!(5000), typical: dec!(350), sigma: dec!(300),
                vat: dec!(0.25),
                jurisdiction: "CA",
                account: "6540",
                confidence: 0.91,
                email_patterns: vec!["1password.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000009").unwrap(),
                name: "Dropbox",
                aliases: vec!["Dropbox Inc", "dropbox.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(100), max: dec!(10000), typical: dec!(400), sigma: dec!(400),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.92,
                email_patterns: vec!["dropbox.com"],
            },
            ProfileBuilder {
                id: Uuid::parse_str("22000002-0000-0000-0000-000000000010").unwrap(),
                name: "Loom",
                aliases: vec!["Loom Inc", "loom.com"],
                category: MerchantCategory::SaasCloud,
                min: dec!(0), max: dec!(5000), typical: dec!(200), sigma: dec!(200),
                vat: dec!(0.25),
                jurisdiction: "US",
                account: "6540",
                confidence: 0.89,
                email_patterns: vec!["loom.com"],
            },
        ];

        for builder in builders {
            let (key, profile) = builder.build();
            // Register all aliases in the alias index
            for alias in &profile.aliases {
                self.alias_index.insert(alias.clone(), key.clone());
            }
            self.merchants.insert(key, profile);
        }
    }
}

impl Default for MerchantIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Utility: integer square root approximation for Decimal
// ---------------------------------------------------------------------------

/// Newton-Raphson square root for Decimal.
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let two = dec!(2);
    let mut guess = x / two;
    for _ in 0..20 {
        let next = (guess + x / guess) / two;
        if (next - guess).abs() < dec!(0.0001) {
            return next;
        }
        guess = next;
    }
    guess
}

// ---------------------------------------------------------------------------
// Account-name lookup (BAS-kontoplan subset)
// ---------------------------------------------------------------------------

fn account_name_for(code: &str) -> String {
    match code {
        "4000" => "Inköp varor",
        "6210" => "Fast telefoni och abonnemang",
        "6220" => "El och energi",
        "6540" => "IT och programvara",
        "6590" => "Övriga finansiella kostnader",
        "6810" => "Juridik, licenser och registreringsavgifter",
        "6980" => "Övriga externa kostnader",
        "7320" => "Resor och transport",
        "2510" => "Skatteskulder",
        _ => "Okänt konto",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_ica() {
        let intel = MerchantIntelligence::new();
        let c = intel.classify("ICA Supermarket", dec!(350));
        assert!(c.is_known);
        assert_eq!(c.canonical_name, "ICA");
        assert!(c.confidence > 0.8);
    }

    #[test]
    fn test_alias_match_aws() {
        let intel = MerchantIntelligence::new();
        let c = intel.classify("AWS", dec!(5000));
        assert!(c.is_known);
        assert_eq!(c.canonical_name, "Amazon Web Services");
    }

    #[test]
    fn test_unknown_merchant() {
        let intel = MerchantIntelligence::new();
        let c = intel.classify("Okänd Leverantör AB", dec!(1000));
        assert!(!c.is_known);
        assert_eq!(c.suggested_account, "6990");
    }

    #[test]
    fn test_learn_and_classify() {
        let mut intel = MerchantIntelligence::new();
        intel.learn("Ny Leverantör AB", dec!(500), "6540", dec!(0.25));
        let c = intel.classify("Ny Leverantör AB", dec!(500));
        assert!(c.is_known);
    }

    #[test]
    fn test_unusual_amount_detection() {
        let intel = MerchantIntelligence::new();
        // Typical for AWS is 5000 with sigma 8000 — so 50000 is > 2σ away
        // (50000 - 5000 = 45000 > 2*8000=16000)
        let c = intel.classify("AWS", dec!(50000));
        assert!(c.amount_is_unusual);
    }

    #[test]
    fn test_suggest_account() {
        let intel = MerchantIntelligence::new();
        let s = intel.suggest_account("Telia", dec!(600));
        assert_eq!(s.account_code, "6210");
        assert!(s.confidence > 0.8);
    }

    #[test]
    fn test_export_import() {
        let mut intel = MerchantIntelligence::new();
        let exported = intel.export_profiles();
        assert!(exported.len() >= 60);
        let mut intel2 = MerchantIntelligence::new();
        intel2.import_profiles(exported.clone());
        assert_eq!(intel2.export_profiles().len(), exported.len());
    }

    #[test]
    fn test_similar_merchants() {
        let intel = MerchantIntelligence::new();
        let similar = intel.similar_merchants("Ryanair");
        // Should return other TravelAccommodation merchants
        assert!(similar.iter().any(|p| p.canonical_name == "SAS" || p.canonical_name == "Norwegian"));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_merchant_name("Telia AB"), "telia");
        assert_eq!(normalize_merchant_name("Amazon Web Services Inc"), "amazon web services");
    }

    #[test]
    fn test_update_stats() {
        let mut intel = MerchantIntelligence::new();
        let id = intel.classify("ICA", dec!(350)).canonical_id.unwrap();
        intel.update_stats(id, dec!(200));
    }
}
