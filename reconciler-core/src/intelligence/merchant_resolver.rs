//! Merchant Intelligence Layer – resolves raw bank descriptions to known Swedish merchants.
//!
//! # Usage
//! ```rust
//! use crate::intelligence::merchant_resolver::MerchantResolver;
//!
//! let resolver = MerchantResolver::from_seed()?;
//! if let Some(result) = resolver.resolve("ICA MAXI NACKA 4392") {
//!     println!("{}: {} (conf={:.2})", result.merchant_id, result.display_name, result.confidence);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MerchantResolverError {
    #[error("Failed to read seed file at {path}: {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse merchant seed JSON: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Seed file not found; tried: {0:?}")]
    SeedNotFound(Vec<String>),
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single merchant entry, matching the seeds/merchants.json schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantProfile {
    pub merchant_id: String,
    pub display_name: String,
    pub category: String,
    pub org_number: Option<String>,
    pub website: Option<String>,
    pub receipt_portal: Option<String>,
    pub receipt_email_patterns: Vec<String>,
    pub bank_aliases: Vec<String>,
    pub receipt_support_channels: Vec<String>,
    pub has_api_access: bool,
    pub notes: Option<String>,
    pub country: String,
}

/// Result of a single resolution attempt.
#[derive(Debug, Clone, Serialize)]
pub struct ResolutionResult {
    pub merchant_id: String,
    pub display_name: String,
    pub category: String,
    pub confidence: f64,
    /// Which alias string caused the match, if any.
    pub matched_alias: Option<String>,
}

// ---------------------------------------------------------------------------
// Normalisation helpers
// ---------------------------------------------------------------------------

/// Normalise a bank transaction description for fuzzy matching.
///
/// Strategy:
/// 1. Uppercase
/// 2. Remove trailing digits (e.g. terminal IDs "ICA MAXI 4392" → "ICA MAXI")
/// 3. Strip common noise characters (*, #, /)
/// 4. Collapse multiple spaces
/// 5. Trim
fn normalise(s: &str) -> String {
    // Step 1: uppercase
    let upper = s.to_uppercase();

    // Step 2: remove common noise characters
    let cleaned: String = upper
        .chars()
        .map(|c| match c {
            '*' | '#' | '/' | '\\' | '.' | ',' | '\'' => ' ',
            _ => c,
        })
        .collect();

    // Step 3: strip trailing digit sequences (terminal IDs / store numbers)
    // We keep all non-trailing digits so "TELE2" stays "TELE2"
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let trimmed_words: Vec<&str> = words
        .iter()
        .rev()
        .skip_while(|w| w.chars().all(|c| c.is_ascii_digit() || c == '-'))
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    trimmed_words.join(" ")
}

// ---------------------------------------------------------------------------
// Resolver
// ---------------------------------------------------------------------------

pub struct MerchantResolver {
    profiles: Vec<MerchantProfile>,
    /// Pre-built lookup: normalised_alias → (profile index, alias string)
    alias_index: HashMap<String, (usize, String)>,
}

impl MerchantResolver {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Build a resolver from a JSON seed file.
    ///
    /// Tries several paths relative to the binary / current directory so it
    /// works both in development (`cargo run`) and deployed contexts.
    pub fn from_seed() -> Result<Self, MerchantResolverError> {
        let candidates: Vec<PathBuf> = vec![
            // Canonical location relative to the crate root
            PathBuf::from("seeds/merchants.json"),
            PathBuf::from("reconciler-core/seeds/merchants.json"),
            // Runtime: next to the binary
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("seeds/merchants.json")))
                .unwrap_or_default(),
            // Absolute path for the Kvittovalvet workspace
            PathBuf::from(
                "/home/userwinston/.openclaw/workspace/ouroboros/reconciler-core/seeds/merchants.json",
            ),
        ];

        for path in &candidates {
            if path.exists() {
                let json = std::fs::read_to_string(path).map_err(|e| {
                    MerchantResolverError::IoError {
                        path: path.display().to_string(),
                        source: e,
                    }
                })?;
                let profiles: Vec<MerchantProfile> = serde_json::from_str(&json)?;
                return Ok(Self::from_profiles(profiles));
            }
        }

        Err(MerchantResolverError::SeedNotFound(
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
        ))
    }

    /// Build a resolver directly from a JSON string (useful in tests / WASM).
    pub fn from_json(json: &str) -> Result<Self, MerchantResolverError> {
        let profiles: Vec<MerchantProfile> = serde_json::from_str(json)?;
        Ok(Self::from_profiles(profiles))
    }

    /// Build a resolver from a pre-parsed slice of profiles.
    pub fn from_profiles(profiles: Vec<MerchantProfile>) -> Self {
        let mut alias_index: HashMap<String, (usize, String)> = HashMap::new();

        for (idx, profile) in profiles.iter().enumerate() {
            // Index every bank alias
            for alias in &profile.bank_aliases {
                let key = normalise(alias);
                alias_index.entry(key).or_insert_with(|| (idx, alias.clone()));
            }
            // Also index the display name as a fallback alias
            let display_key = normalise(&profile.display_name);
            alias_index
                .entry(display_key)
                .or_insert_with(|| (idx, profile.display_name.clone()));
        }

        Self {
            profiles,
            alias_index,
        }
    }

    // -----------------------------------------------------------------------
    // Resolution
    // -----------------------------------------------------------------------

    /// Resolve a raw bank transaction description to the best matching merchant.
    ///
    /// Matching pipeline (highest confidence first):
    ///
    /// | Score | Strategy |
    /// |-------|----------|
    /// | 1.00  | Exact match on normalised alias |
    /// | 0.92  | Normalised input **equals** a normalised alias |
    /// | 0.85  | Normalised alias is a prefix of normalised input |
    /// | 0.75  | Normalised alias is contained in normalised input |
    /// | 0.60  | Normalised input contains normalised alias word-boundary match |
    /// | 0.45  | First-word match (input starts with alias first word) |
    pub fn resolve(&self, bank_description: &str) -> Option<ResolutionResult> {
        let input_norm = normalise(bank_description);

        if input_norm.is_empty() {
            return None;
        }

        let mut best: Option<(f64, usize, String)> = None;

        // --- Strategy 1: exact alias key lookup (O(1)) ---
        if let Some((idx, alias)) = self.alias_index.get(&input_norm) {
            best = Some((1.0, *idx, alias.clone()));
        }

        // Only run the O(n) scan if we didn't get a perfect hit
        if best.as_ref().map(|(c, _, _)| *c).unwrap_or(0.0) < 1.0 {
            for (idx, profile) in self.profiles.iter().enumerate() {
                for alias in &profile.bank_aliases {
                    let alias_norm = normalise(alias);

                    // Strategy 2: input equals alias (already covered by index, skip)
                    // Strategy 3: alias is a prefix of input
                    let confidence = if input_norm.starts_with(&alias_norm) && !alias_norm.is_empty()
                    {
                        // Longer prefix = higher confidence
                        let ratio = alias_norm.len() as f64 / input_norm.len() as f64;
                        0.75 + 0.17 * ratio
                    } else if alias_norm.starts_with(&input_norm) && !input_norm.is_empty() {
                        // Input is a prefix of alias
                        let ratio = input_norm.len() as f64 / alias_norm.len() as f64;
                        0.65 + 0.15 * ratio
                    } else if input_norm.contains(&alias_norm) && !alias_norm.is_empty() {
                        // Alias is a substring of input
                        let ratio = alias_norm.len() as f64 / input_norm.len() as f64;
                        0.55 + 0.20 * ratio
                    } else if alias_norm.contains(&input_norm) && !input_norm.is_empty() {
                        // Input is a substring of alias
                        0.50
                    } else {
                        // Strategy 4: word-boundary first-word match
                        let alias_first = alias_norm.split_whitespace().next().unwrap_or("");
                        let input_first = input_norm.split_whitespace().next().unwrap_or("");
                        if !alias_first.is_empty()
                            && !input_first.is_empty()
                            && alias_first == input_first
                        {
                            0.40
                        } else {
                            continue;
                        }
                    };

                    if confidence > best.as_ref().map(|(c, _, _)| *c).unwrap_or(0.0) {
                        best = Some((confidence, idx, alias.clone()));
                    }
                }
            }
        }

        best.map(|(confidence, idx, alias)| {
            let profile = &self.profiles[idx];
            ResolutionResult {
                merchant_id: profile.merchant_id.clone(),
                display_name: profile.display_name.clone(),
                category: profile.category.clone(),
                confidence,
                matched_alias: Some(alias),
            }
        })
    }

    /// Resolve and return `None` if confidence is below the given threshold.
    pub fn resolve_confident(
        &self,
        bank_description: &str,
        min_confidence: f64,
    ) -> Option<ResolutionResult> {
        self.resolve(bank_description)
            .filter(|r| r.confidence >= min_confidence)
    }

    /// Return all profiles for a given category.
    pub fn by_category(&self, category: &str) -> Vec<&MerchantProfile> {
        let upper = category.to_uppercase();
        self.profiles
            .iter()
            .filter(|p| p.category.to_uppercase() == upper)
            .collect()
    }

    /// Return all profiles that support a specific receipt channel.
    pub fn by_receipt_channel(&self, channel: &str) -> Vec<&MerchantProfile> {
        let lower = channel.to_lowercase();
        self.profiles
            .iter()
            .filter(|p| p.receipt_support_channels.iter().any(|c| c.to_lowercase() == lower))
            .collect()
    }

    /// Return all profiles with API access.
    pub fn with_api_access(&self) -> Vec<&MerchantProfile> {
        self.profiles.iter().filter(|p| p.has_api_access).collect()
    }

    /// Look up a profile by exact merchant_id.
    pub fn by_id(&self, merchant_id: &str) -> Option<&MerchantProfile> {
        let upper = merchant_id.to_uppercase();
        self.profiles
            .iter()
            .find(|p| p.merchant_id.to_uppercase() == upper)
    }

    /// Returns the total number of loaded profiles.
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Returns all loaded profiles.
    pub fn all(&self) -> &[MerchantProfile] {
        &self.profiles
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SEED_JSON: &str = include_str!("../../../seeds/merchants.json");

    fn resolver() -> MerchantResolver {
        MerchantResolver::from_json(SEED_JSON).expect("Failed to load seed")
    }

    #[test]
    fn test_load_seed() {
        let r = resolver();
        assert!(r.len() >= 80, "Expected ≥80 merchants, got {}", r.len());
    }

    #[test]
    fn test_exact_match_ica() {
        let r = resolver();
        let result = r.resolve("ICA MAXI").unwrap();
        assert_eq!(result.merchant_id, "ICA");
        assert!(result.confidence >= 0.85);
    }

    #[test]
    fn test_ica_with_terminal_id() {
        let r = resolver();
        // Terminal ID suffix should be stripped by normalisation
        let result = r.resolve("ICA MAXI NACKA 4392").unwrap();
        assert_eq!(result.merchant_id, "ICA");
        assert!(result.confidence >= 0.75);
    }

    #[test]
    fn test_circle_k_legacy_statoil() {
        let r = resolver();
        let result = r.resolve("STATOIL FUEL").unwrap();
        assert_eq!(result.merchant_id, "CIRCLEK");
    }

    #[test]
    fn test_spotify_asterisk() {
        let r = resolver();
        // Bank descriptions sometimes include asterisk
        let result = r.resolve("SPOTIFY*").unwrap();
        assert_eq!(result.merchant_id, "SPOTIFY");
    }

    #[test]
    fn test_by_category_dagligvaror() {
        let r = resolver();
        let cat = r.by_category("DAGLIGVAROR");
        assert!(!cat.is_empty());
        assert!(cat.iter().all(|p| p.category == "DAGLIGVAROR"));
    }

    #[test]
    fn test_bolt_vs_other() {
        let r = resolver();
        let result = r.resolve("BOLT EU").unwrap();
        assert_eq!(result.merchant_id, "BOLT");
        assert!(result.confidence >= 0.85);
    }

    #[test]
    fn test_uber_eats() {
        let r = resolver();
        let result = r.resolve("UBER EATS").unwrap();
        assert_eq!(result.merchant_id, "UBER");
    }

    #[test]
    fn test_unknown_returns_none() {
        let r = resolver();
        // A completely made-up string should return None or very low confidence
        let result = r.resolve_confident("ZZZZZZZFAKEMERCHANT9999", 0.5);
        assert!(result.is_none());
    }

    #[test]
    fn test_by_receipt_channel_app() {
        let r = resolver();
        let app_merchants = r.by_receipt_channel("app");
        assert!(!app_merchants.is_empty());
        // ICA and Spotify should be in app-channel merchants
        assert!(app_merchants.iter().any(|p| p.merchant_id == "ICA"));
        assert!(app_merchants.iter().any(|p| p.merchant_id == "SPOTIFY"));
    }

    #[test]
    fn test_api_access() {
        let r = resolver();
        let api = r.with_api_access();
        // Spotify, AWS, GitHub should all have API access
        assert!(api.iter().any(|p| p.merchant_id == "SPOTIFY"));
        assert!(api.iter().any(|p| p.merchant_id == "AWS"));
        assert!(api.iter().any(|p| p.merchant_id == "GITHUB"));
    }

    #[test]
    fn test_normalise_strips_noise() {
        assert_eq!(normalise("NETFLIX*"), "NETFLIX");
        assert_eq!(normalise("ICA MAXI 4392"), "ICA MAXI");
        assert_eq!(normalise("spotify.com"), "SPOTIFYCOM");
    }

    #[test]
    fn test_by_id() {
        let r = resolver();
        let profile = r.by_id("KLARNA").unwrap();
        assert_eq!(profile.merchant_id, "KLARNA");
        assert_eq!(profile.category, "BANK_FINANS");
    }

    #[test]
    fn test_scandic_variants() {
        let r = resolver();
        for desc in &["SCANDIC STOCKHOLM", "SCANDIC GRAND CENTRAL", "SCANDIC HOTEL"] {
            let result = r.resolve(desc).unwrap();
            assert_eq!(result.merchant_id, "SCANDIC", "Failed for: {}", desc);
        }
    }

    #[test]
    fn test_tele2_comviq_distinct() {
        let r = resolver();
        let t2 = r.resolve("TELE2 MOBIL").unwrap();
        assert_eq!(t2.merchant_id, "TELE2");
        let cq = r.resolve("COMVIQ").unwrap();
        assert_eq!(cq.merchant_id, "COMVIQ");
    }

    #[test]
    fn test_resolve_confident_threshold() {
        let r = resolver();
        // High confidence hit
        let high = r.resolve_confident("ICA MAXI", 0.8);
        assert!(high.is_some());
        // Low confidence threshold should accept
        let low = r.resolve_confident("ICA MAXI", 0.1);
        assert!(low.is_some());
    }
}
