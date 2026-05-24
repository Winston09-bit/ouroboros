use once_cell::sync::Lazy;
use regex::Regex;
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Clone, Default)]
pub struct ParsedReceipt {
    pub merchant_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub date: Option<String>,
    pub vat_amount: Option<Decimal>,
    pub vat_rate: Option<f64>,
    pub items: Vec<String>,
    pub confidence: f64,
}

static AMOUNT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(?:total|summa|att betala|amount due)[^\d]*(\d+[,.]\d{2})\s*(SEK|kr|EUR)?",
    )
    .expect("Invalid amount regex")
});

static VAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:moms|vat|mva)[^\d]*(\d+[,.]\d{2})\s*(SEK|kr|EUR)?")
        .expect("Invalid VAT regex")
});

static DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{4}-\d{2}-\d{2}|\d{2}/\d{2}/\d{4}|\d{2}\.\d{2}\.\d{4})")
        .expect("Invalid date regex")
});

static MERCHANT_SUBJECT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:kvitto|receipt|order|faktura)\s+(?:från|from|hos|at)?\s*([A-Za-zÅÄÖåäö0-9 &]{2,40})")
        .expect("Invalid merchant regex")
});

static CURRENCY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(SEK|kr|EUR|USD|GBP)\b").expect("Invalid currency regex")
});

fn normalize_decimal(s: &str) -> String {
    s.replace(',', ".")
}

pub fn parse_email_body(body: &str) -> ParsedReceipt {
    let mut result = ParsedReceipt::default();
    let mut confidence: f64 = 0.0;

    if let Some(cap) = AMOUNT_RE.captures(body) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let normalized = normalize_decimal(raw);
        if let Ok(dec) = Decimal::from_str(&normalized) {
            result.total_amount = Some(dec);
            confidence += 0.4;
        }
        if let Some(cur) = cap.get(2) {
            let cur_str = cur.as_str().to_uppercase();
            result.currency = Some(if cur_str == "KR" {
                "SEK".to_string()
            } else {
                cur_str
            });
            confidence += 0.1;
        }
    }

    if result.currency.is_none() {
        if let Some(cap) = CURRENCY_RE.captures(body) {
            let cur = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_uppercase();
            result.currency = Some(if cur == "KR" {
                "SEK".to_string()
            } else {
                cur
            });
        }
    }

    if let Some(cap) = VAT_RE.captures(body) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let normalized = normalize_decimal(raw);
        if let Ok(dec) = Decimal::from_str(&normalized) {
            result.vat_amount = Some(dec);
            confidence += 0.1;
        }
    }

    if let Some(cap) = DATE_RE.captures(body) {
        result.date = Some(cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string());
        confidence += 0.1;
    }

    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed.len() > 3 && trimmed.len() < 80 {
            if trimmed.chars().any(|c| c.is_alphabetic()) {
                result.items.push(trimmed.to_string());
                if result.items.len() >= 10 {
                    break;
                }
            }
        }
    }

    result.confidence = confidence.min(1.0);
    result
}

pub fn parse_subject(subject: &str) -> ParsedReceipt {
    let mut result = ParsedReceipt::default();
    let mut confidence: f64 = 0.0;

    if let Some(cap) = MERCHANT_SUBJECT_RE.captures(subject) {
        let merchant = cap
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());
        if merchant.is_some() {
            result.merchant_name = merchant;
            confidence += 0.3;
        }
    }

    if let Some(cap) = AMOUNT_RE.captures(subject) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let normalized = normalize_decimal(raw);
        if let Ok(dec) = Decimal::from_str(&normalized) {
            result.total_amount = Some(dec);
            confidence += 0.3;
        }
        if let Some(cur) = cap.get(2) {
            let cur_str = cur.as_str().to_uppercase();
            result.currency = Some(if cur_str == "KR" {
                "SEK".to_string()
            } else {
                cur_str
            });
            confidence += 0.1;
        }
    }

    if result.currency.is_none() {
        if let Some(cap) = CURRENCY_RE.captures(subject) {
            let cur = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_uppercase();
            result.currency = Some(if cur == "KR" {
                "SEK".to_string()
            } else {
                cur
            });
        }
    }

    result.confidence = confidence.min(1.0);
    result
}
