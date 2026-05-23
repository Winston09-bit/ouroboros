// src/ocr/parser.rs — Financial Document Parser
// Reconciler OCR + Document Intelligence Pipeline

use chrono::NaiveDate;
use regex::Regex;
use rust_decimal::Decimal;
use std::str::FromStr;

use super::{DocumentType, ExtractedDocumentData, ExtractedLineItem};

// ─────────────────────────────────────────────
// Supporting types
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AmountCandidate {
    pub value: Decimal,
    /// Surrounding text (up to ~40 chars each side)
    pub context: String,
    /// Is this likely the grand total?
    pub is_total: bool,
    /// Is this a tax / VAT line?
    pub is_tax: bool,
}

#[derive(Debug, Clone)]
pub struct VatInfo {
    pub amount: Option<Decimal>,
    pub rate: Option<Decimal>,
    /// VAT registration number (e.g. SE556123456701, DE123456789)
    pub number: Option<String>,
    pub jurisdiction: String,
}

// ─────────────────────────────────────────────
// Keyword tables
// ─────────────────────────────────────────────

/// Keywords that strongly indicate a TOTAL line (any jurisdiction).
const TOTAL_KEYWORDS: &[&str] = &[
    // Swedish
    "totalt", "total", "att betala", "summa", "belopp",
    "summa att betala", "totalbelopp",
    // English
    "total", "grand total", "amount due", "balance due",
    "total amount", "total due", "subtotal",
    // German / EU
    "gesamtbetrag", "rechnungsbetrag", "endbetrag", "bruttobetrag",
    // French
    "total ttc", "montant total",
];

/// Keywords indicating a TAX / VAT line.
const TAX_KEYWORDS: &[&str] = &[
    // Swedish
    "moms", "mervärdesskatt", "varav moms",
    // English
    "vat", "tax", "sales tax", "gst", "hst",
    // German
    "mwst", "mehrwertsteuer", "ust",
    // French
    "tva",
];

/// Keywords indicating an invoice (not a receipt).
const INVOICE_KEYWORDS: &[&str] = &[
    "faktura", "invoice", "rechnung", "facture",
    "fakturanummer", "invoice number", "fakturadatum",
];

/// Keywords indicating a bank statement.
const BANK_KEYWORDS: &[&str] = &[
    "kontoutdrag", "bank statement", "kontoauszug",
    "relevé de compte", "saldo", "balance",
    "transaction", "debit", "credit", "kredit",
];

// ─────────────────────────────────────────────
// FinancialDocumentParser
// ─────────────────────────────────────────────

pub struct FinancialDocumentParser;

impl FinancialDocumentParser {
    pub fn new() -> Self {
        Self
    }

    // ── Top-level parse ───────────────────────────────────────────────────────
    /// Parse raw OCR text into a fully structured `ExtractedDocumentData`.
    pub fn parse(&self, text: &str) -> ExtractedDocumentData {
        let lower = text.to_lowercase();

        let doc_type = Self::detect_document_type(&lower);
        let currency = Self::detect_currency(text);
        let jurisdiction = Self::infer_jurisdiction(&lower, currency.as_deref());
        let vat_info = Self::extract_vat(text, &jurisdiction);
        let date = Self::extract_date(text);
        let vendor_name = Self::extract_vendor(text);
        let invoice_number = Self::extract_invoice_number(text);
        let amounts = Self::extract_amount(text);

        let total_amount = Self::pick_total(&amounts, &lower);
        let tax_amount = vat_info.amount.or_else(|| Self::pick_tax(&amounts));

        let tax_rate = vat_info.rate.or_else(|| {
            // Derive rate from total + tax if possible
            if let (Some(total), Some(tax)) = (total_amount, tax_amount) {
                let net = total - tax;
                if !net.is_zero() {
                    let rate = (tax / net * Decimal::from(100))
                        .round_dp(2);
                    Some(rate)
                } else {
                    None
                }
            } else {
                None
            }
        });

        let line_items = Self::extract_line_items(text);

        let mut data = ExtractedDocumentData {
            doc_type,
            total_amount,
            tax_amount,
            tax_rate,
            currency,
            vendor_name,
            vendor_vat: vat_info.number,
            date,
            invoice_number,
            line_items,
            confidence: 0.0,
        };
        data.confidence = Self::confidence_score(&data);
        data
    }

    // ── Document type detection ───────────────────────────────────────────────
    fn detect_document_type(lower: &str) -> DocumentType {
        let invoice_score: usize = INVOICE_KEYWORDS
            .iter()
            .filter(|&&kw| lower.contains(kw))
            .count();
        let bank_score: usize = BANK_KEYWORDS
            .iter()
            .filter(|&&kw| lower.contains(kw))
            .count();

        if bank_score >= 2 {
            DocumentType::BankStatement
        } else if invoice_score >= 1 {
            DocumentType::Invoice
        } else {
            // If we see a total but no invoice markers → treat as receipt
            let has_total = TOTAL_KEYWORDS.iter().any(|&kw| lower.contains(kw));
            if has_total {
                DocumentType::Receipt
            } else {
                DocumentType::Unknown
            }
        }
    }

    // ── Amount extraction ─────────────────────────────────────────────────────
    /// Extract all monetary amounts with context clues.
    ///
    /// Handles formats:
    ///   1 234,56   (Swedish / European)
    ///   1,234.56   (Anglo-American)
    ///   1234.56    (plain decimal)
    ///   1 234      (integer with space-thousands)
    ///   kr 249:-   (Swedish short form)
    pub fn extract_amount(text: &str) -> Vec<AmountCandidate> {
        // Regex: optional currency prefix, then number in various formats
        // Group 1: optional currency prefix (kr, SEK, EUR, USD, GBP, €, $, £)
        // Group 2: the number itself
        let amount_re = Regex::new(
            r"(?xi)
            (?P<prefix>kr\.?\s*|sek\s*|eur\s*|usd\s*|gbp\s*|€\s*|\$\s*|£\s*)?
            (?P<amount>
                \d{1,3}(?:[\s\u00A0]\d{3})*[,\.]\d{2}   # 1 234,56 or 1,234.56
              | \d{1,3}(?:[\s\u00A0]\d{3})+              # 1 234 (no cents)
              | \d+[,\.]\d{2}                             # 123,45 or 123.45
              | \d+:-                                     # 249:- (Swedish)
            )
            (?P<suffix>\s*(?:kr|sek|eur|usd|gbp|€|\$|£))?
            ",
        )
        .expect("amount regex");

        let lower = text.to_lowercase();
        let mut results = Vec::new();

        for cap in amount_re.captures_iter(text) {
            let raw_amount = &cap["amount"];
            let Some(value) = Self::parse_decimal_str(raw_amount) else {
                continue;
            };

            // Gather context: up to 40 chars before and after the match
            let m = cap.get(0).unwrap();
            let start = m.start().saturating_sub(40);
            let end = (m.end() + 40).min(text.len());
            let context = text[start..end].to_string();
            let ctx_lower = context.to_lowercase();

            let is_total = TOTAL_KEYWORDS.iter().any(|&kw| ctx_lower.contains(kw));
            let is_tax = TAX_KEYWORDS.iter().any(|&kw| ctx_lower.contains(kw));

            results.push(AmountCandidate {
                value,
                context,
                is_total,
                is_tax,
            });
        }

        // Deduplicate by value+context similarity (keep first occurrence)
        results.dedup_by(|a, b| a.value == b.value && a.context == b.context);

        // Sort descending by value (totals tend to be the largest)
        results.sort_by(|a, b| b.value.cmp(&a.value));
        results
    }

    /// Parse a messy decimal string from OCR into a `Decimal`.
    fn parse_decimal_str(s: &str) -> Option<Decimal> {
        // Strip trailing :-
        let s = s.trim_end_matches(":-").trim();
        // Remove non-breaking spaces + regular spaces used as thousands sep
        let s = s.replace('\u{00A0}', "").replace(' ', "");

        // Determine separator convention:
        // If the string has both comma and dot, the last one is the decimal sep.
        let normalized = if s.contains(',') && s.contains('.') {
            // e.g. "1,234.56" → dot is decimal
            if s.rfind('.') > s.rfind(',') {
                s.replace(',', "")
            } else {
                // "1.234,56" → comma is decimal
                s.replace('.', "").replace(',', ".")
            }
        } else if s.contains(',') {
            // "1234,56" → comma is decimal (EU style)
            s.replace(',', ".")
        } else {
            s.to_string()
        };

        Decimal::from_str(&normalized).ok()
    }

    /// Pick the most likely grand-total from a list of candidates.
    fn pick_total(candidates: &[AmountCandidate], _lower: &str) -> Option<Decimal> {
        // Priority: explicit total keyword match, largest value otherwise
        candidates
            .iter()
            .find(|c| c.is_total && !c.is_tax)
            .or_else(|| candidates.first())
            .map(|c| c.value)
    }

    fn pick_tax(candidates: &[AmountCandidate]) -> Option<Decimal> {
        candidates
            .iter()
            .find(|c| c.is_tax)
            .map(|c| c.value)
    }

    // ── VAT extraction ────────────────────────────────────────────────────────
    /// Extract VAT information for the given jurisdiction hint.
    ///
    /// Jurisdiction examples: "SE", "DE", "GB", "US", "EU"
    pub fn extract_vat(text: &str, jurisdiction: &str) -> VatInfo {
        let lower = text.to_lowercase();

        // ── VAT registration number ──────────────────────────────────────────
        // Swedish: SE + 10 digits (e.g. SE556123456701)
        // EU generic: 2-letter country + 8-12 alphanumeric
        // UK: GB + 9 digits
        let vat_num_re = Regex::new(
            r"(?xi)
            (?:vat\s*(?:no\.?|number|nr\.?|reg\.?)?|moms(?:reg)?(?:nr)?\.?|orgnr\.?)
            \s*:?\s*
            (?P<vat>[A-Z]{2}[0-9A-Z]{8,12})
            ",
        )
        .expect("vat_num_re");

        let vat_number = vat_num_re
            .captures(text)
            .and_then(|c| c.name("vat"))
            .map(|m| m.as_str().to_uppercase());

        // ── VAT amount ───────────────────────────────────────────────────────
        // Lines like: "Moms 25%  123,45" or "VAT 20%  £24.99"
        let vat_amount_re = Regex::new(
            r"(?xi)
            (?:moms|vat|tva|mwst|iva)
            [\s:,]*
            (?:\d{1,2}[,\.]?\d*\s*%\s*)?   # optional rate
            (?:kr\.?\s*|sek\s*|eur?\s*|usd?\s*|gbp?\s*|€\s*|\$\s*|£\s*)?
            (?P<amount>
                \d{1,3}(?:[\s\u00A0]\d{3})*[,\.]\d{2}
              | \d+[,\.]\d{2}
              | \d{1,3}(?:[\s\u00A0]\d{3})+
            )
            ",
        )
        .expect("vat_amount_re");

        let vat_amount = vat_amount_re
            .captures(text)
            .and_then(|c| c.name("amount"))
            .and_then(|m| Self::parse_decimal_str(m.as_str()));

        // ── VAT rate ─────────────────────────────────────────────────────────
        // Patterns: "25 %", "VAT 20%", "incl. 19% MwSt", "moms 12%"
        let vat_rate_re = Regex::new(
            r"(?xi)
            (?:moms|vat|tva|mwst|iva|tax)\s+
            (?P<rate>\d{1,2}(?:[,\.]\d+)?)\s*%
            |
            (?P<rate2>\d{1,2}(?:[,\.]\d+)?)\s*%\s*
            (?:moms|vat|tva|mwst|iva|tax)
            ",
        )
        .expect("vat_rate_re");

        let vat_rate = vat_rate_re
            .captures(&lower)
            .and_then(|c| c.name("rate").or_else(|| c.name("rate2")))
            .and_then(|m| Self::parse_decimal_str(m.as_str()));

        // Jurisdiction-specific fallback rate hints
        let vat_rate = vat_rate.or_else(|| {
            if vat_amount.is_some() {
                // Guess common rates if we found an amount but not the rate
                Some(match jurisdiction {
                    "SE" => Decimal::from(25),
                    "DE" => Decimal::from(19),
                    "GB" => Decimal::from(20),
                    "FR" => Decimal::from(20),
                    _ => return None,
                })
            } else {
                None
            }
        });

        VatInfo {
            amount: vat_amount,
            rate: vat_rate,
            number: vat_number,
            jurisdiction: jurisdiction.to_string(),
        }
    }

    // ── Date extraction ───────────────────────────────────────────────────────
    /// Extract date from multiple locale-aware formats.
    ///
    /// Supported:
    ///   - 2026-05-23        (ISO 8601)
    ///   - 23/05/2026        (EU day-first)
    ///   - 05/23/2026        (US month-first, heuristic)
    ///   - 23.05.2026        (German)
    ///   - May 23 2026       (English long)
    ///   - 23 maj 2026       (Swedish long)
    ///   - 23 mai 2026       (French long)
    ///   - 23. Mai 2026      (German long)
    pub fn extract_date(text: &str) -> Option<NaiveDate> {
        // ── ISO 8601 ────────────────────────────────────────────────────────
        let iso_re =
            Regex::new(r"\b(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})\b").unwrap();
        if let Some(cap) = iso_re.captures(text) {
            if let Ok(date) = NaiveDate::from_ymd_opt(
                cap["y"].parse().ok()?,
                cap["m"].parse().ok()?,
                cap["d"].parse().ok()?,
            )
            .ok_or(())
            {
                return Some(date);
            }
        }

        // ── EU numeric: DD/MM/YYYY or DD.MM.YYYY ────────────────────────────
        let eu_re = Regex::new(
            r"\b(?P<d>\d{1,2})[/\.](?P<m>\d{1,2})[/\.](?P<y>\d{4})\b",
        )
        .unwrap();
        if let Some(cap) = eu_re.captures(text) {
            let d: u32 = cap["d"].parse().ok()?;
            let m: u32 = cap["m"].parse().ok()?;
            let y: i32 = cap["y"].parse().ok()?;
            if d <= 31 && m <= 12 {
                if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                    return Some(date);
                }
            }
        }

        // ── Wordy formats ────────────────────────────────────────────────────
        // "23 May 2026", "May 23, 2026", "23 maj 2026", etc.
        let month_map: &[(&str, u32)] = &[
            ("january", 1), ("jan", 1),
            ("february", 2), ("feb", 2),
            ("march", 3), ("mar", 3),
            ("april", 4), ("apr", 4),
            ("may", 5),
            ("june", 6), ("jun", 6),
            ("july", 7), ("jul", 7),
            ("august", 8), ("aug", 8),
            ("september", 9), ("sep", 9), ("sept", 9),
            ("october", 10), ("oct", 10),
            ("november", 11), ("nov", 11),
            ("december", 12), ("dec", 12),
            // Swedish
            ("januari", 1), ("februari", 2), ("mars", 3),
            ("maj", 5), ("juni", 6), ("juli", 7),
            ("augusti", 8), ("oktober", 10), ("december", 12),
            // German
            ("januar", 1), ("februar", 2), ("märz", 3),
            ("mai", 5), ("oktober", 10), ("dezember", 12),
            // French
            ("janvier", 1), ("février", 2), ("mars", 3),
            ("avril", 4), ("mai", 5), ("juin", 6),
            ("juillet", 7), ("août", 8), ("septembre", 9),
            ("octobre", 10), ("novembre", 11), ("décembre", 12),
        ];

        // Build alternation pattern dynamically
        let month_names: String = month_map
            .iter()
            .map(|(n, _)| regex::escape(n))
            .collect::<Vec<_>>()
            .join("|");

        let wordy_re = Regex::new(&format!(
            r"(?xi)
            (?:
                (?P<d1>\d{{1,2}})\.?\s+(?P<mon1>{months})\s+(?P<y1>\d{{4}})   # 23 May 2026
              | (?P<mon2>{months})\s+(?P<d2>\d{{1,2}})[,\s]+(?P<y2>\d{{4}})   # May 23 2026
            )
            ",
            months = month_names
        ))
        .unwrap();

        let lower = text.to_lowercase();
        if let Some(cap) = wordy_re.captures(&lower) {
            let (day, month_str, year) = if let Some(d) = cap.name("d1") {
                (
                    d.as_str(),
                    cap.name("mon1").unwrap().as_str(),
                    cap.name("y1").unwrap().as_str(),
                )
            } else {
                (
                    cap.name("d2").unwrap().as_str(),
                    cap.name("mon2").unwrap().as_str(),
                    cap.name("y2").unwrap().as_str(),
                )
            };

            let d: u32 = day.parse().ok()?;
            let y: i32 = year.parse().ok()?;
            let m = month_map
                .iter()
                .find(|(name, _)| *name == month_str)
                .map(|(_, num)| *num)?;

            if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                return Some(date);
            }
        }

        None
    }

    // ── Vendor name ───────────────────────────────────────────────────────────
    /// Extract vendor name — typically the first meaningful non-numeric line.
    pub fn extract_vendor(text: &str) -> Option<String> {
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        for line in lines.iter().take(5) {
            let lower = line.to_lowercase();
            // Skip lines that look like addresses, phone numbers, dates, or totals
            if lower.starts_with("tel") || lower.starts_with("phone") {
                continue;
            }
            if line.contains('@') {
                continue; // email address
            }
            if Regex::new(r"^\d{1,2}[/\.\-]\d{1,2}").unwrap().is_match(line) {
                continue; // date-looking
            }
            if line.chars().filter(|c| c.is_alphabetic()).count() < 3 {
                continue; // too few letters
            }
            // Skip keyword lines
            let is_keyword = TOTAL_KEYWORDS
                .iter()
                .chain(TAX_KEYWORDS.iter())
                .chain(INVOICE_KEYWORDS.iter())
                .any(|&kw| lower == kw);
            if is_keyword {
                continue;
            }
            return Some(line.to_string());
        }
        None
    }

    // ── Invoice number ────────────────────────────────────────────────────────
    fn extract_invoice_number(text: &str) -> Option<String> {
        let re = Regex::new(
            r"(?xi)
            (?:faktura(?:nr|nummer)?|invoice\s*(?:no\.?|number|#)|rechnung(?:s?nr\.?)?|facture\s*n[o°]\.?)
            \s*:?\s*
            (?P<num>[A-Z0-9][\w\-/\.]{1,20})
            ",
        )
        .expect("invoice_num_re");
        re.captures(text)
            .and_then(|c| c.name("num"))
            .map(|m| m.as_str().to_string())
    }

    // ── Line items ────────────────────────────────────────────────────────────
    /// Attempt to parse tabular line items.
    ///
    /// Heuristic: look for lines that contain a description + a number at the
    /// end separated by whitespace (common receipt/invoice row format).
    pub fn extract_line_items(text: &str) -> Vec<ExtractedLineItem> {
        // Pattern: "   Description     qty x price   total"
        // We accept any line with at least one decimal amount at the end.
        let line_re = Regex::new(
            r"(?x)
            ^
            (?P<desc>.{3,40}?)          # description (non-greedy, 3-40 chars)
            \s{2,}                       # at least 2 spaces before amount
            (?P<qty>\d+(?:[,\.]\d+)?\s*[xX]\s*)?  # optional qty × price
            (?P<total>
                \d{1,3}(?:[\s]\d{3})*[,\.]\d{2}
              | \d+[,\.]\d{2}
            )
            \s*$
            ",
        )
        .expect("line_item_re");

        let mut items = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip header/footer keywords
            let lower = trimmed.to_lowercase();
            if TOTAL_KEYWORDS.iter().any(|&k| lower.contains(k))
                || TAX_KEYWORDS.iter().any(|&k| lower.contains(k))
            {
                continue;
            }

            if let Some(cap) = line_re.captures(trimmed) {
                let description = cap["desc"].trim().to_string();
                let total_str = &cap["total"];
                let Some(total) = Self::parse_decimal_str(total_str) else {
                    continue;
                };

                // Try to parse qty × unit_price
                let (quantity, unit_price) = if let Some(qty_match) = cap.name("qty") {
                    let raw = qty_match.as_str().replace(['x', 'X'], "*");
                    let parts: Vec<&str> = raw.split('*').collect();
                    if parts.len() == 2 {
                        let qty = Self::parse_decimal_str(parts[0].trim());
                        let up = Self::parse_decimal_str(parts[1].trim());
                        (qty, up)
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                if !description.is_empty() {
                    items.push(ExtractedLineItem {
                        description,
                        quantity,
                        unit_price,
                        total,
                    });
                }
            }
        }

        items
    }

    // ── Language detection ────────────────────────────────────────────────────
    /// Very lightweight language detection based on vocabulary fingerprinting.
    ///
    /// Returns a BCP-47-ish code: "sv", "en", "de", "fr", "unknown".
    pub fn detect_language(text: &str) -> String {
        let lower = text.to_lowercase();

        // Weighted word lists per language
        let sv_words = &[
            "och", "att", "det", "är", "en", "ett", "för", "på", "av",
            "moms", "kvitto", "faktura", "betalning", "totalt", "datum",
            "maj", "juni", "juli", "januari", "februari",
        ];
        let en_words = &[
            "the", "and", "for", "invoice", "receipt", "payment",
            "total", "tax", "amount", "date", "thank", "you",
        ];
        let de_words = &[
            "und", "der", "die", "das", "für", "rechnung", "mwst",
            "mehrwertsteuer", "betrag", "gesamt", "datum",
        ];
        let fr_words = &[
            "et", "le", "la", "les", "des", "facture", "tva",
            "montant", "total", "date", "merci",
        ];

        let score = |words: &[&str]| -> usize {
            words.iter().filter(|&&w| {
                // whole-word match
                let pattern = format!(r"\b{}\b", regex::escape(w));
                Regex::new(&pattern).map(|r| r.is_match(&lower)).unwrap_or(false)
            }).count()
        };

        let sv = score(sv_words);
        let en = score(en_words);
        let de = score(de_words);
        let fr = score(fr_words);

        let max = sv.max(en).max(de).max(fr);
        if max == 0 {
            return "unknown".to_string();
        }
        if sv == max { "sv".to_string() }
        else if en == max { "en".to_string() }
        else if de == max { "de".to_string() }
        else { "fr".to_string() }
    }

    // ── Currency detection ────────────────────────────────────────────────────
    /// Detect the document's primary currency from symbols and ISO codes.
    pub fn detect_currency(text: &str) -> Option<String> {
        let lower = text.to_lowercase();

        // Check explicit ISO codes first (avoid symbol ambiguity)
        let iso_re =
            Regex::new(r"\b(SEK|NOK|DKK|EUR|USD|GBP|CHF|CAD|AUD|JPY)\b").unwrap();
        if let Some(cap) = iso_re.captures(text) {
            return Some(cap[1].to_string());
        }

        // Symbol → ISO mapping (frequency-ranked)
        if lower.contains("kr") || text.contains("kr.") {
            // "kr" is used by SEK, NOK, DKK — default to SEK unless other hints
            return Some("SEK".to_string());
        }
        if text.contains('€') || lower.contains("eur") {
            return Some("EUR".to_string());
        }
        if text.contains('$') || lower.contains("usd") {
            return Some("USD".to_string());
        }
        if text.contains('£') || lower.contains("gbp") {
            return Some("GBP".to_string());
        }

        None
    }

    // ── Jurisdiction inference ────────────────────────────────────────────────
    fn infer_jurisdiction(lower: &str, currency: Option<&str>) -> String {
        if lower.contains("moms") || currency == Some("SEK") {
            return "SE".to_string();
        }
        if lower.contains("mwst") || lower.contains("mehrwertsteuer") {
            return "DE".to_string();
        }
        if lower.contains("tva") {
            return "FR".to_string();
        }
        if lower.contains("gst") || lower.contains("hst") {
            return "CA".to_string();
        }
        if currency == Some("GBP") {
            return "GB".to_string();
        }
        if currency == Some("EUR") {
            return "EU".to_string();
        }
        "US".to_string()
    }

    // ── Confidence score ──────────────────────────────────────────────────────
    /// Compute extraction quality score in [0, 1].
    ///
    /// Each successfully extracted field contributes a weight; mandatory
    /// fields (total, vendor, date) carry the most weight.
    pub fn confidence_score(data: &ExtractedDocumentData) -> f64 {
        let mut score = 0.0_f64;
        let mut max = 0.0_f64;

        macro_rules! field {
            ($weight:expr, $present:expr) => {
                max += $weight;
                if $present {
                    score += $weight;
                }
            };
        }

        field!(0.25, data.total_amount.is_some());
        field!(0.20, data.vendor_name.is_some());
        field!(0.15, data.date.is_some());
        field!(0.10, data.tax_amount.is_some());
        field!(0.10, data.currency.is_some());
        field!(0.08, data.tax_rate.is_some());
        field!(0.07, data.vendor_vat.is_some());
        field!(0.05, !matches!(data.doc_type, DocumentType::Unknown));

        if max == 0.0 { 0.0 } else { score / max }
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Amount parsing ──────────────────────────────────────────────────────

    #[test]
    fn parses_swedish_amount_kr_suffix() {
        let amounts = FinancialDocumentParser::extract_amount("Totalt  249 kr");
        assert!(!amounts.is_empty());
        assert_eq!(amounts[0].value, Decimal::from(249));
        assert!(amounts[0].is_total);
    }

    #[test]
    fn parses_eu_decimal_comma() {
        let amounts = FinancialDocumentParser::extract_amount("Total: 1 234,56 EUR");
        assert!(!amounts.is_empty());
        assert_eq!(
            amounts[0].value,
            Decimal::from_str("1234.56").unwrap()
        );
    }

    #[test]
    fn parses_american_decimal_dot() {
        let amounts = FinancialDocumentParser::extract_amount("Grand Total: $1,234.56");
        assert!(!amounts.is_empty());
        assert_eq!(
            amounts[0].value,
            Decimal::from_str("1234.56").unwrap()
        );
    }

    #[test]
    fn parses_swedish_colon_minus() {
        let amounts = FinancialDocumentParser::extract_amount("Pris: 99:-");
        assert!(!amounts.is_empty());
        assert_eq!(amounts[0].value, Decimal::from(99));
    }

    #[test]
    fn marks_vat_line_as_tax() {
        let amounts = FinancialDocumentParser::extract_amount("Moms 25%  62,00 kr");
        let tax = amounts.iter().find(|c| c.is_tax);
        assert!(tax.is_some(), "expected a tax candidate");
    }

    // ── Date extraction ─────────────────────────────────────────────────────

    #[test]
    fn parses_iso_date() {
        let d = FinancialDocumentParser::extract_date("Datum: 2026-05-23");
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 5, 23));
    }

    #[test]
    fn parses_eu_slash_date() {
        let d = FinancialDocumentParser::extract_date("Date: 23/05/2026");
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 5, 23));
    }

    #[test]
    fn parses_german_dot_date() {
        let d = FinancialDocumentParser::extract_date("Datum: 23.05.2026");
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 5, 23));
    }

    #[test]
    fn parses_english_long_date() {
        let d = FinancialDocumentParser::extract_date("May 23 2026");
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 5, 23));
    }

    #[test]
    fn parses_swedish_long_date() {
        let d = FinancialDocumentParser::extract_date("23 maj 2026");
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 5, 23));
    }

    #[test]
    fn parses_day_first_english() {
        let d = FinancialDocumentParser::extract_date("23 May 2026");
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 5, 23));
    }

    // ── Currency detection ──────────────────────────────────────────────────

    #[test]
    fn detects_sek() {
        assert_eq!(
            FinancialDocumentParser::detect_currency("Totalt 249 kr"),
            Some("SEK".to_string())
        );
    }

    #[test]
    fn detects_eur_symbol() {
        assert_eq!(
            FinancialDocumentParser::detect_currency("Total: €1 234,56"),
            Some("EUR".to_string())
        );
    }

    #[test]
    fn detects_iso_gbp() {
        assert_eq!(
            FinancialDocumentParser::detect_currency("Amount due: GBP 99.99"),
            Some("GBP".to_string())
        );
    }

    // ── Language detection ──────────────────────────────────────────────────

    #[test]
    fn detects_swedish() {
        let text = "Kvitto\nDatum: 2026-05-23\nMoms 25%  62 kr\nTotalt  249 kr";
        assert_eq!(FinancialDocumentParser::detect_language(text), "sv");
    }

    #[test]
    fn detects_english() {
        let text = "RECEIPT\nDate: May 23 2026\nTax: $12.50\nTotal: $62.50";
        assert_eq!(FinancialDocumentParser::detect_language(text), "en");
    }

    // ── VAT extraction ──────────────────────────────────────────────────────

    #[test]
    fn extracts_swedish_vat_rate() {
        let vat = FinancialDocumentParser::extract_vat("Moms 25%  62,00 kr\nTotalt 310,00 kr", "SE");
        assert_eq!(vat.rate, Some(Decimal::from(25)));
    }

    #[test]
    fn extracts_vat_number() {
        let vat = FinancialDocumentParser::extract_vat(
            "VAT No. SE556123456701\nTotal: 1000 EUR",
            "SE",
        );
        assert_eq!(vat.number, Some("SE556123456701".to_string()));
    }

    // ── Vendor extraction ───────────────────────────────────────────────────

    #[test]
    fn extracts_vendor_first_line() {
        let text = "Acme Supplies AB\nOrgnr: 556123-4567\nDatum: 2026-05-23";
        let vendor = FinancialDocumentParser::extract_vendor(text);
        assert_eq!(vendor, Some("Acme Supplies AB".to_string()));
    }

    // ── Confidence score ────────────────────────────────────────────────────

    #[test]
    fn confidence_all_fields() {
        let data = ExtractedDocumentData {
            doc_type: DocumentType::Invoice,
            total_amount: Some(Decimal::from(100)),
            tax_amount: Some(Decimal::from(20)),
            tax_rate: Some(Decimal::from(20)),
            currency: Some("EUR".to_string()),
            vendor_name: Some("Acme".to_string()),
            vendor_vat: Some("DE123456789".to_string()),
            date: Some(NaiveDate::from_ymd_opt(2026, 5, 23).unwrap()),
            invoice_number: Some("INV-001".to_string()),
            line_items: vec![],
            confidence: 0.0,
        };
        let score = FinancialDocumentParser::confidence_score(&data);
        assert!(score > 0.9, "score was {}", score);
    }

    #[test]
    fn confidence_empty_document() {
        let data = ExtractedDocumentData::default();
        let score = FinancialDocumentParser::confidence_score(&data);
        assert_eq!(score, 0.0);
    }

    // ── Full parse integration ──────────────────────────────────────────────

    #[test]
    fn full_parse_swedish_receipt() {
        let text = r#"
ICA Supermarket
Hornsgatan 100, Stockholm

Datum: 2026-05-23

Mjölk 3L          25,90 kr
Bröd              18,50 kr
Kaffe 500g        49,00 kr

Totalt           93,40 kr
Varav moms 12%   10,00 kr

Tack för ditt köp!
"#;
        let parser = FinancialDocumentParser::new();
        let data = parser.parse(text);
        assert_eq!(data.doc_type, DocumentType::Receipt);
        assert_eq!(data.currency, Some("SEK".to_string()));
        assert!(data.total_amount.is_some());
        assert!(data.date.is_some());
        assert!(data.vendor_name.is_some());
    }

    #[test]
    fn full_parse_eu_invoice() {
        let text = r#"
ACME Consulting GmbH
VAT No. DE123456789

Invoice Number: INV-2026-0042
Date: 23.05.2026

Consulting services   8 hours x 150,00 EUR   1.200,00 EUR

Subtotal:    1.200,00 EUR
VAT 19%:       228,00 EUR
Total:       1.428,00 EUR
"#;
        let parser = FinancialDocumentParser::new();
        let data = parser.parse(text);
        assert_eq!(data.doc_type, DocumentType::Invoice);
        assert_eq!(data.currency, Some("EUR".to_string()));
        assert!(data.total_amount.is_some());
        assert!(data.vendor_vat.is_some());
        assert_eq!(data.invoice_number, Some("INV-2026-0042".to_string()));
    }
}
