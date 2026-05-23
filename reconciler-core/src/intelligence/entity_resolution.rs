// src/intelligence/entity_resolution.rs
//
// Reconciler Entity Resolution Engine
// Maps raw merchant strings to canonical entities via multi-stage matching:
//   1. Exact match on normalized form
//   2. Jaccard similarity on word sets
//   3. Levenshtein edit distance
//   4. Semantic / keyword heuristics (Semantic variant)
//
// All methods are pure functions except EntityResolver which holds an in-memory
// knowledge base.  Swap the knowledge base for a real DB by replacing
// `KnowledgeBase::built_in()` with a DB-backed implementation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Public enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchMethod {
    /// byte-for-byte match on normalised names
    Exact,
    /// cleaned-up prefix / suffix match after normalization
    Normalized,
    /// Jaccard + Levenshtein heuristic
    Fuzzy,
    /// keyword-based semantic bucket
    Semantic,
    /// could not determine
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerchantCategory {
    Groceries,
    Software,
    Travel,
    Utilities,
    Payroll,
    Tax,
    Restaurants,
    Healthcare,
    Entertainment,
    Finance,
    Other,
}

impl MerchantCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            MerchantCategory::Groceries => "Groceries",
            MerchantCategory::Software => "Software",
            MerchantCategory::Travel => "Travel",
            MerchantCategory::Utilities => "Utilities",
            MerchantCategory::Payroll => "Payroll",
            MerchantCategory::Tax => "Tax",
            MerchantCategory::Restaurants => "Restaurants",
            MerchantCategory::Healthcare => "Healthcare",
            MerchantCategory::Entertainment => "Entertainment",
            MerchantCategory::Finance => "Finance",
            MerchantCategory::Other => "Other",
        }
    }
}

// ---------------------------------------------------------------------------
// EntityMatch – result of a resolution attempt
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EntityMatch {
    pub raw_input: String,
    pub canonical_name: String,
    pub entity_id: Option<Uuid>,
    pub confidence: f64,
    pub match_method: MatchMethod,
    pub country: Option<String>,
    pub category: Option<String>,
}

impl EntityMatch {
    fn unknown(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let category = categorize_merchant(&raw).as_str().to_string();
        Self {
            canonical_name: raw.clone(),
            raw_input: raw,
            entity_id: None,
            confidence: 0.0,
            match_method: MatchMethod::Unknown,
            country: None,
            category: Some(category),
        }
    }
}

// ---------------------------------------------------------------------------
// Knowledge-base entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct MerchantEntry {
    id: Uuid,
    canonical_name: String,
    /// pre-normalized canonical name for fast lookup
    normalized_canonical: String,
    aliases: Vec<String>,
    /// pre-normalized aliases
    normalized_aliases: Vec<String>,
    country: Option<String>,
    category: MerchantCategory,
}

// ---------------------------------------------------------------------------
// Built-in knowledge base
// ---------------------------------------------------------------------------

struct KnowledgeBase {
    entries: Vec<MerchantEntry>,
    /// normalized_form -> entry index for O(1) exact lookup
    exact_index: HashMap<String, usize>,
}

impl KnowledgeBase {
    fn build(raw: Vec<(&str, Option<&str>, MerchantCategory, Vec<&str>)>) -> Self {
        let mut entries = Vec::with_capacity(raw.len());
        let mut exact_index = HashMap::new();

        for (canonical, country, category, aliases) in raw {
            let normalized_canonical = normalize_name(canonical);
            let normalized_aliases: Vec<String> = aliases.iter().map(|a| normalize_name(a)).collect();

            // index canonical
            let idx = entries.len();
            exact_index.insert(normalized_canonical.clone(), idx);

            // index all aliases
            for na in &normalized_aliases {
                exact_index.entry(na.clone()).or_insert(idx);
            }

            entries.push(MerchantEntry {
                id: Uuid::new_v4(),
                canonical_name: canonical.to_string(),
                normalized_canonical,
                aliases: aliases.iter().map(|s| s.to_string()).collect(),
                normalized_aliases,
                country: country.map(str::to_string),
                category,
            });
        }

        Self { entries, exact_index }
    }

    fn built_in() -> Self {
        Self::build(vec![
            // Swedish groceries
            (
                "ICA",
                Some("SE"),
                MerchantCategory::Groceries,
                vec![
                    "ICA MAXI", "ICA Sverige AB", "ICA GBG", "ICA KVANTUM",
                    "ICA SUPERMARKET", "ICA Nära", "ica maxi göteborg",
                    "ICA GRUPPEN", "ICA MAXI GÖTEBORG",
                ],
            ),
            (
                "Willys",
                Some("SE"),
                MerchantCategory::Groceries,
                vec!["WILLYS AB", "Willys Hemma", "WILLYS PLUS"],
            ),
            (
                "Coop",
                Some("SE"),
                MerchantCategory::Groceries,
                vec!["COOP AB", "COOP BUTIKER", "Coop Forum", "Coop Konsum", "COOP NÄRA"],
            ),
            (
                "Lidl",
                Some("SE"),
                MerchantCategory::Groceries,
                vec!["LIDL SVERIGE", "LIDL SE", "Lidl AB"],
            ),
            (
                "Hemköp",
                Some("SE"),
                MerchantCategory::Groceries,
                vec!["HEMKÖP AB", "HEMKOP"],
            ),
            (
                "Matspar",
                Some("SE"),
                MerchantCategory::Groceries,
                vec!["MATSPAR.SE"],
            ),
            // Swedish fast food / restaurants
            (
                "McDonald's",
                None,
                MerchantCategory::Restaurants,
                vec!["MCDONALDS", "MC DONALDS", "McDonald's Sverige", "McDONALDS"],
            ),
            (
                "Max Burgers",
                Some("SE"),
                MerchantCategory::Restaurants,
                vec!["MAX", "Max Hamburgare", "MAX BURGER"],
            ),
            // Swedish utilities / telecom
            (
                "Telia",
                Some("SE"),
                MerchantCategory::Utilities,
                vec!["TELIA AB", "TELIA SVERIGE", "TeliaSonera"],
            ),
            (
                "Tele2",
                Some("SE"),
                MerchantCategory::Utilities,
                vec!["TELE2 AB", "TELE2 SVERIGE"],
            ),
            (
                "Vattenfall",
                Some("SE"),
                MerchantCategory::Utilities,
                vec!["VATTENFALL AB", "Vattenfall Eldistribution"],
            ),
            // Software / SaaS
            (
                "GitHub",
                Some("US"),
                MerchantCategory::Software,
                vec!["GITHUB INC", "Github.com", "GitHub.com"],
            ),
            (
                "Vercel",
                Some("US"),
                MerchantCategory::Software,
                vec!["VERCEL INC", "Vercel.com"],
            ),
            (
                "AWS",
                Some("US"),
                MerchantCategory::Software,
                vec![
                    "Amazon Web Services", "AMAZON WEB SERVICES INC",
                    "AWS EMEA SARL", "Amazon AWS",
                ],
            ),
            (
                "Cloudflare",
                Some("US"),
                MerchantCategory::Software,
                vec!["CLOUDFLARE INC", "Cloudflare.com"],
            ),
            (
                "Slack",
                Some("US"),
                MerchantCategory::Software,
                vec!["SLACK TECHNOLOGIES", "Slack Technologies Inc"],
            ),
            (
                "Notion",
                Some("US"),
                MerchantCategory::Software,
                vec!["NOTION LABS INC", "Notion.so"],
            ),
            (
                "OpenAI",
                Some("US"),
                MerchantCategory::Software,
                vec!["OPENAI LLC", "OpenAI Inc", "api.openai.com"],
            ),
            (
                "Anthropic",
                Some("US"),
                MerchantCategory::Software,
                vec!["ANTHROPIC PBC", "Anthropic LLC", "api.anthropic.com"],
            ),
            // Travel
            (
                "SJ",
                Some("SE"),
                MerchantCategory::Travel,
                vec!["SJ AB", "SJ.SE", "SJ RESOR"],
            ),
            (
                "Ryanair",
                Some("IE"),
                MerchantCategory::Travel,
                vec!["RYANAIR DAC", "RYANAIR LTD"],
            ),
            (
                "SAS",
                Some("SE"),
                MerchantCategory::Travel,
                vec!["Scandinavian Airlines", "SAS AB", "SCANDINAVIAN AIRLINES SYSTEM"],
            ),
            // Finance
            (
                "Revolut",
                Some("GB"),
                MerchantCategory::Finance,
                vec!["REVOLUT LTD", "Revolut Bank UAB"],
            ),
            (
                "Klarna",
                Some("SE"),
                MerchantCategory::Finance,
                vec!["KLARNA AB", "Klarna Bank AB"],
            ),
            // Swedish tax authority
            (
                "Skatteverket",
                Some("SE"),
                MerchantCategory::Tax,
                vec!["SKATTEVERKET", "Swedish Tax Agency", "SKV"],
            ),
        ])
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions (public so tests and other modules can use them)
// ---------------------------------------------------------------------------

/// Remove legal suffixes, store/branch numbers, extra punctuation and lower-case.
///
/// "ICA GBG 4412" → "ica gbg"
/// "Amazon Web Services Inc." → "amazon web services"
pub fn normalize_name(s: &str) -> String {
    // Legal-form suffixes to strip (word boundaries, case-insensitive)
    const LEGAL_SUFFIXES: &[&str] = &[
        "ab", "ltd", "llc", "inc", "gmbh", "bv", "nv", "sa", "sas",
        "ag", "plc", "oy", "as", "kb", "hb", "pbc", "dac", "sarl",
        "uab", "spa", "pvt",
    ];

    let lowered = s.to_lowercase();

    // Strip common punctuation
    let cleaned: String = lowered
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect();

    // Tokenize
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();

    // Drop pure numeric tokens (store branch numbers like "4412")
    let without_nums: Vec<&str> = tokens
        .iter()
        .copied()
        .filter(|t| !t.chars().all(|c| c.is_ascii_digit()))
        .collect();

    // Drop trailing legal suffixes (may appear in any position after the brand name)
    let without_legal: Vec<&str> = without_nums
        .iter()
        .copied()
        .filter(|t| !LEGAL_SUFFIXES.contains(t))
        .collect();

    let result = without_legal.join(" ");

    // If stripping everything left us empty, fall back to the lowered form
    if result.is_empty() {
        lowered.trim().to_string()
    } else {
        result
    }
}

/// Jaccard similarity on word sets.
///
/// J(A,B) = |A ∩ B| / |A ∪ B|
///
/// Returns a value in [0.0, 1.0].  Two identical strings → 1.0.
/// Two completely disjoint strings → 0.0.
pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;

    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Classic Wagner-Fischer Levenshtein distance.
///
/// Works on Unicode scalar values (not bytes).
/// Returns the minimum number of single-character edits (insertions,
/// deletions, substitutions) to transform `a` into `b`.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let la = a_chars.len();
    let lb = b_chars.len();

    if la == 0 {
        return lb;
    }
    if lb == 0 {
        return la;
    }

    // We only keep two rows at a time to save memory.
    let mut prev: Vec<usize> = (0..=lb).collect();
    let mut curr: Vec<usize> = vec![0; lb + 1];

    for i in 1..=la {
        curr[0] = i;
        for j in 1..=lb {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)             // deletion
                .min(curr[j - 1] + 1)           // insertion
                .min(prev[j - 1] + cost);        // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[lb]
}

/// Keyword-based category heuristic.
///
/// Operates on the *normalized* name so that "ICA GBG 4412" → "ica gbg" still
/// matches the "ica" grocery keyword.
pub fn categorize_merchant(name: &str) -> MerchantCategory {
    let n = normalize_name(name);

    // Keyword lists (all lowercase, already normalized)
    const GROCERY_KW: &[&str] = &[
        "ica", "willys", "coop", "lidl", "hemkop", "hemköp", "netto",
        "city gross", "citygross", "mathem", "matspar", "axfood", "tempo",
        "spar", "eurospar",
    ];
    const SOFTWARE_KW: &[&str] = &[
        "github", "gitlab", "vercel", "aws", "amazon web", "cloudflare",
        "digitalocean", "heroku", "stripe", "twilio", "sendgrid", "notion",
        "slack", "figma", "openai", "anthropic", "google cloud", "azure",
        "microsoft", "apple developer",
    ];
    const TRAVEL_KW: &[&str] = &[
        "sj", "ryanair", "sas", "norwegian", "klm", "lufthansa", "flixbus",
        "airbnb", "booking", "expedia", "hotels", "hyatt", "marriott",
        "hilton", "uber", "lyft", "taxi",
    ];
    const UTILITY_KW: &[&str] = &[
        "telia", "tele2", "telenor", "tre", "vattenfall", "eon", "fortum",
        "ellevio", "comhem", "bahnhof", "bredband", "halebop",
    ];
    const PAYROLL_KW: &[&str] = &["lön", "salary", "payroll", "swish lön"];
    const TAX_KW: &[&str] = &["skatteverket", "skv", "swedish tax", "moms", "vat refund"];
    const RESTAURANT_KW: &[&str] = &[
        "mcdonalds", "max burger", "max hamburgare", "burger king",
        "subway", "kfc", "pizza", "sushi", "restaurang", "cafe", "kafé",
        "coffee", "espresso house", "starbucks", "wayne", "wok",
    ];
    const HEALTHCARE_KW: &[&str] = &[
        "apotek", "pharmacy", "doktor", "clinic", "vårdcentral",
        "tandläkare", "dentist", "apoteket",
    ];
    const ENTERTAINMENT_KW: &[&str] = &[
        "spotify", "netflix", "hbo", "disney", "viaplay", "svtplay",
        "steam", "playstation", "xbox", "nintendo", "cinema", "bio ",
        "tickets",
    ];
    const FINANCE_KW: &[&str] = &[
        "revolut", "klarna", "swish", "paypal", "bankgiro", "plusgiro",
        "amex", "visa", "mastercard", "nordea", "swedbank", "seb",
        "handelsbanken", "länsförsäkringar",
    ];

    fn matches(n: &str, kws: &[&str]) -> bool {
        kws.iter().any(|kw| n.contains(kw))
    }

    if matches(&n, GROCERY_KW) {
        MerchantCategory::Groceries
    } else if matches(&n, SOFTWARE_KW) {
        MerchantCategory::Software
    } else if matches(&n, TRAVEL_KW) {
        MerchantCategory::Travel
    } else if matches(&n, UTILITY_KW) {
        MerchantCategory::Utilities
    } else if matches(&n, PAYROLL_KW) {
        MerchantCategory::Payroll
    } else if matches(&n, TAX_KW) {
        MerchantCategory::Tax
    } else if matches(&n, RESTAURANT_KW) {
        MerchantCategory::Restaurants
    } else if matches(&n, HEALTHCARE_KW) {
        MerchantCategory::Healthcare
    } else if matches(&n, ENTERTAINMENT_KW) {
        MerchantCategory::Entertainment
    } else if matches(&n, FINANCE_KW) {
        MerchantCategory::Finance
    } else {
        MerchantCategory::Other
    }
}

// ---------------------------------------------------------------------------
// Resolution thresholds
// ---------------------------------------------------------------------------

/// Minimum Jaccard similarity to accept a fuzzy match.
const JACCARD_THRESHOLD: f64 = 0.40;
/// Maximum Levenshtein distance (relative to the longer string length) to accept.
const LEVENSHTEIN_REL_THRESHOLD: f64 = 0.35;

// ---------------------------------------------------------------------------
// EntityResolver
// ---------------------------------------------------------------------------

pub struct EntityResolver {
    kb: Arc<KnowledgeBase>,
    /// LRU-style in-memory cache: normalized_input -> EntityMatch
    cache: Arc<RwLock<HashMap<String, EntityMatch>>>,
}

impl EntityResolver {
    /// Create a resolver backed by the built-in knowledge base.
    pub fn new() -> Self {
        Self {
            kb: Arc::new(KnowledgeBase::built_in()),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a resolver backed by a custom knowledge base (for testing or
    /// when loading from a database).
    pub fn with_entries(entries: Vec<(&str, Option<&str>, MerchantCategory, Vec<&str>)>) -> Self {
        Self {
            kb: Arc::new(KnowledgeBase::build(entries)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolve a single raw merchant string to its canonical entity.
    pub fn resolve(&self, raw: &str) -> EntityMatch {
        let normalized = normalize_name(raw);

        // ----------------------------------------------------------------
        // Stage 1: Exact lookup on normalized form
        // ----------------------------------------------------------------
        if let Some(&idx) = self.kb.exact_index.get(&normalized) {
            let entry = &self.kb.entries[idx];
            return self.make_match(raw, entry, 1.0, MatchMethod::Exact);
        }

        // ----------------------------------------------------------------
        // Stage 2: Normalized prefix / contains matching
        // ----------------------------------------------------------------
        for (i, entry) in self.kb.entries.iter().enumerate() {
            let nc = &entry.normalized_canonical;
            // Canonical is a prefix of input or vice-versa
            if normalized.starts_with(nc.as_str()) || nc.starts_with(normalized.as_str()) {
                let confidence = nc.len().min(normalized.len()) as f64
                    / nc.len().max(normalized.len()) as f64;
                if confidence >= 0.70 {
                    return self.make_match(raw, entry, confidence * 0.95, MatchMethod::Normalized);
                }
            }
            // Check aliases
            for na in &entry.normalized_aliases {
                if normalized.starts_with(na.as_str()) || na.starts_with(normalized.as_str()) {
                    let confidence = na.len().min(normalized.len()) as f64
                        / na.len().max(normalized.len()) as f64;
                    if confidence >= 0.70 {
                        return self.make_match(raw, entry, confidence * 0.92, MatchMethod::Normalized);
                    }
                }
            }
            let _ = i; // suppress unused warning
        }

        // ----------------------------------------------------------------
        // Stage 3: Fuzzy matching (Jaccard + Levenshtein combined score)
        // ----------------------------------------------------------------
        let mut best_score = 0.0_f64;
        let mut best_entry: Option<&MerchantEntry> = None;

        for entry in &self.kb.entries {
            let score = self.fuzzy_score(&normalized, &entry.normalized_canonical);
            if score > best_score {
                best_score = score;
                best_entry = Some(entry);
            }
            for na in &entry.normalized_aliases {
                let score = self.fuzzy_score(&normalized, na);
                if score > best_score {
                    best_score = score;
                    best_entry = Some(entry);
                }
            }
        }

        if best_score >= 0.55 {
            if let Some(entry) = best_entry {
                return self.make_match(raw, entry, best_score * 0.88, MatchMethod::Fuzzy);
            }
        }

        // ----------------------------------------------------------------
        // Stage 4: Semantic fallback – use category heuristics
        // ----------------------------------------------------------------
        let category = categorize_merchant(raw);
        if category != MerchantCategory::Other {
            // Return an unknown entity but with correct category from semantics
            let category_str = category.as_str().to_string();
            return EntityMatch {
                raw_input: raw.to_string(),
                canonical_name: Self::title_case(&normalized),
                entity_id: None,
                confidence: 0.30,
                match_method: MatchMethod::Semantic,
                country: None,
                category: Some(category_str),
            };
        }

        // ----------------------------------------------------------------
        // No match
        // ----------------------------------------------------------------
        EntityMatch::unknown(raw)
    }

    /// Batch resolve with async-safe caching.  Cache is checked before and
    /// written after resolution to avoid redundant work in hot paths.
    pub async fn resolve_batch(&self, raws: &[String]) -> Vec<EntityMatch> {
        let mut results = Vec::with_capacity(raws.len());

        for raw in raws {
            let key = normalize_name(raw);

            // Check cache (read lock, cheap)
            {
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(&key) {
                    results.push(cached.clone());
                    continue;
                }
            }

            // Resolve
            let matched = self.resolve(raw);

            // Store in cache (write lock)
            {
                let mut cache = self.cache.write().await;
                cache.insert(key, matched.clone());
            }

            results.push(matched);
        }

        results
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn make_match(&self, raw: &str, entry: &MerchantEntry, confidence: f64, method: MatchMethod) -> EntityMatch {
        EntityMatch {
            raw_input: raw.to_string(),
            canonical_name: entry.canonical_name.clone(),
            entity_id: Some(entry.id),
            confidence: confidence.clamp(0.0, 1.0),
            match_method: method,
            country: entry.country.clone(),
            category: Some(entry.category.as_str().to_string()),
        }
    }

    /// Combined Jaccard + normalised Levenshtein score in [0, 1].
    fn fuzzy_score(&self, a: &str, b: &str) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let j = jaccard_similarity(a, b);

        let max_len = a.chars().count().max(b.chars().count());
        let lev = levenshtein_distance(a, b);
        let lev_sim = 1.0 - (lev as f64 / max_len as f64).min(1.0);

        // Weighted combination: Jaccard 60%, Levenshtein 40%
        let combined = 0.6 * j + 0.4 * lev_sim;

        // Hard cutoffs: if neither measure clears their individual threshold,
        // consider it a non-match regardless of the weighted score.
        if j < JACCARD_THRESHOLD && (lev as f64 / max_len as f64) > LEVENSHTEIN_REL_THRESHOLD {
            return 0.0;
        }

        combined
    }

    /// Naive title-casing for display purposes.
    fn title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut c = word.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for EntityResolver {
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

    #[test]
    fn test_normalize_name_removes_legal_suffix() {
        assert_eq!(normalize_name("ICA Sverige AB"), "ica sverige");
    }

    #[test]
    fn test_normalize_name_removes_store_number() {
        assert_eq!(normalize_name("ICA GBG 4412"), "ica gbg");
    }

    #[test]
    fn test_normalize_name_lowercase() {
        assert_eq!(normalize_name("Ica Maxi Göteborg"), "ica maxi göteborg");
    }

    #[test]
    fn test_jaccard_identical() {
        assert!((jaccard_similarity("ica maxi", "ica maxi") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_disjoint() {
        assert!((jaccard_similarity("foo bar", "baz qux") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_partial() {
        // {"ica", "maxi"} ∩ {"ica", "gbg"} = {"ica"}
        // union = {"ica", "maxi", "gbg"} = 3
        let j = jaccard_similarity("ica maxi", "ica gbg");
        assert!((j - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_levenshtein_same() {
        assert_eq!(levenshtein_distance("kitten", "kitten"), 0);
    }

    #[test]
    fn test_levenshtein_classic() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_categorize_groceries() {
        assert_eq!(categorize_merchant("ICA MAXI GBG"), MerchantCategory::Groceries);
        assert_eq!(categorize_merchant("Willys AB"), MerchantCategory::Groceries);
    }

    #[test]
    fn test_categorize_software() {
        assert_eq!(categorize_merchant("GitHub Inc"), MerchantCategory::Software);
        assert_eq!(categorize_merchant("Amazon Web Services"), MerchantCategory::Software);
    }

    #[test]
    fn test_categorize_tax() {
        assert_eq!(categorize_merchant("Skatteverket"), MerchantCategory::Tax);
    }

    #[test]
    fn test_resolver_exact_alias() {
        let resolver = EntityResolver::new();
        let m = resolver.resolve("ICA MAXI");
        assert_eq!(m.canonical_name, "ICA");
        assert_eq!(m.match_method, MatchMethod::Exact);
        assert!(m.confidence > 0.99);
        assert_eq!(m.category.as_deref(), Some("Groceries"));
        assert_eq!(m.country.as_deref(), Some("SE"));
    }

    #[test]
    fn test_resolver_normalized_prefix() {
        let resolver = EntityResolver::new();
        // "ICA GBG 4412" normalizes to "ica gbg" which is a prefix of no alias
        // but starts with canonical "ica" — should be caught by prefix match
        let m = resolver.resolve("ICA GBG 4412");
        assert_eq!(m.canonical_name, "ICA");
    }

    #[test]
    fn test_resolver_fuzzy() {
        let resolver = EntityResolver::new();
        // Deliberate typo
        let m = resolver.resolve("Amazn Web Services");
        assert_eq!(m.canonical_name, "AWS");
    }

    #[test]
    fn test_resolver_unknown() {
        let resolver = EntityResolver::new();
        let m = resolver.resolve("Totally Unknown Vendor 9999");
        assert_eq!(m.match_method, MatchMethod::Unknown);
        assert!(m.entity_id.is_none());
    }

    #[tokio::test]
    async fn test_batch_resolve_caches() {
        let resolver = EntityResolver::new();
        let inputs: Vec<String> = vec![
            "ICA MAXI".to_string(),
            "ICA MAXI".to_string(), // duplicate — should hit cache on second pass
            "GitHub".to_string(),
        ];
        let results = resolver.resolve_batch(&inputs).await;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].canonical_name, results[1].canonical_name);
        assert_eq!(results[2].canonical_name, "GitHub");
    }
}
