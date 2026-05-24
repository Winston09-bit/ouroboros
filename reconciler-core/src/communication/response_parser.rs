//! Inbound email response parser for Kvittovalvet.
//!
//! Classifies merchant replies into actionable intents using keyword matching
//! for both Swedish and English. Extracts useful data fragments (order numbers,
//! amounts, forwarding addresses) that downstream workflow steps can act on.

use regex::Regex;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedResponse {
    pub intent: ResponseIntent,
    pub extracted_data: ExtractedData,
    /// 0.0–1.0 confidence score based on number and weight of matched signals.
    pub confidence: f64,
    /// Raw input text (body + subject concatenated) used for matching.
    pub raw_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseIntent {
    /// Merchant confirms they have the receipt and will/did send it.
    HasReceipt,
    /// Merchant needs more information before they can help.
    NeedsMoreInfo { info_needed: String },
    /// Merchant searched but cannot locate the receipt/transaction.
    CannotFind,
    /// We have been directed to a different department or email.
    WrongDepartment { forward_to: String },
    /// Auto-reply: sender is out of the office.
    OutOfOffice { return_date: Option<String> },
    /// Intent could not be determined.
    Unclassified,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtractedData {
    pub order_number: Option<String>,
    pub receipt_numbers: Vec<String>,
    pub amounts: Vec<String>,
    pub contact_email: Option<String>,
    pub attachments_mentioned: bool,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Parse a response email body + subject into a [`ParsedResponse`].
pub fn parse_response(body: &str, subject: &str) -> ParsedResponse {
    let combined = format!("{} {}", subject, body);
    let lower    = combined.to_lowercase();

    // --- Extract structured data first (order-independent) -----------------
    let extracted = extract_data(&combined, &lower);

    // --- Classify intent ----------------------------------------------------
    let (intent, confidence) = classify(&lower, &extracted);

    ParsedResponse {
        intent,
        extracted_data: extracted,
        confidence,
        raw_text: combined,
    }
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

fn classify(lower: &str, data: &ExtractedData) -> (ResponseIntent, f64) {
    // Each matcher returns (Option<ResponseIntent>, weight).
    // We collect all signals and pick the highest-weighted hit.

    struct Signal {
        intent: ResponseIntent,
        weight: f64,
    }

    let mut signals: Vec<Signal> = Vec::new();

    // ── Out of Office ────────────────────────────────────────────────────────
    let ooo_keywords = [
        "out of office",
        "automatic reply",
        "auto-reply",
        "frånvaromeddelande",
        "automatiskt svar",
        "semester",
        "föräldraledig",
        "tjänstledighet",
        "frånvaro",
        "abwesend",           // German (occasionally seen)
    ];
    if ooo_keywords.iter().any(|k| lower.contains(k)) {
        let return_date = extract_return_date(lower);
        signals.push(Signal {
            intent: ResponseIntent::OutOfOffice { return_date },
            weight: 0.90,
        });
    }

    // ── Wrong Department / Forwarding ────────────────────────────────────────
    let fwd_keywords = [
        "ekonomiavdelningen",
        "accounts payable",
        "fakturering",
        "leverantörsreskontra",
        "reskontran",
        "contact our finance",
        "please contact finance",
        "invoice department",
        "maila",
        "kontakta istället",
        "vidarebefordrar",
        "forward",
        "transferring your",
    ];
    if fwd_keywords.iter().any(|k| lower.contains(k)) {
        let forward_email = data.contact_email.clone().unwrap_or_default();
        signals.push(Signal {
            intent: ResponseIntent::WrongDepartment { forward_to: forward_email },
            weight: 0.80,
        });
    }

    // ── Needs More Info ──────────────────────────────────────────────────────
    let info_patterns: &[(&str, &str)] = &[
        ("ordernummer",           "ordernummer"),
        ("order number",          "order number"),
        ("beställningsnummer",    "beställningsnummer"),
        ("purchase order",        "purchase order number"),
        ("po number",             "PO number"),
        ("referensnummer",        "referensnummer"),
        ("booking reference",     "booking reference"),
        ("reservation number",    "reservation number"),
        ("vilket kort",           "kortnummer"),
        ("card number",           "card number"),
        ("mer information",       "ytterligare information"),
        ("more information",      "more information"),
        ("vilket datum",          "transaktionsdatum"),
        ("vilken butik",          "butiksidentifiering"),
    ];
    for (pattern, label) in info_patterns {
        if lower.contains(pattern) {
            signals.push(Signal {
                intent: ResponseIntent::NeedsMoreInfo {
                    info_needed: label.to_string(),
                },
                weight: 0.75,
            });
        }
    }

    // ── Cannot Find ──────────────────────────────────────────────────────────
    let not_found_keywords = [
        "hittar inte",
        "kan inte hitta",
        "saknar",
        "finns inte",
        "har inga uppgifter",
        "cannot find",
        "unable to find",
        "could not locate",
        "no record",
        "not found",
        "no receipt",
        "no invoice",
        "hittade inga",
        "inga resultat",
        "no matching",
        "unable to locate",
        "we do not have",
        "we don't have",
    ];
    if not_found_keywords.iter().any(|k| lower.contains(k)) {
        signals.push(Signal {
            intent: ResponseIntent::CannotFind,
            weight: 0.70,
        });
    }

    // ── Has Receipt ──────────────────────────────────────────────────────────
    let receipt_keywords = [
        "kvitto bifogas",
        "faktura bifogas",
        "bifogat finner ni",
        "bifogar kvitto",
        "bifogar faktura",
        "skickar kvitto",
        "skickar faktura",
        "receipt attached",
        "invoice attached",
        "please find attached",
        "find the receipt",
        "find the invoice",
        "kvitto skickat",
        "faktura skickad",
        "skickat till er",
        "sent to you",
        "sending the receipt",
        "sending the invoice",
        "will send",
        "skickar inom",
        "comes attached",
        "see attachment",
        "in the attachment",
    ];
    // Also count mentioned attachments as a supporting signal.
    let attachment_weight_bonus = if data.attachments_mentioned { 0.10 } else { 0.0 };
    if receipt_keywords.iter().any(|k| lower.contains(k)) || data.attachments_mentioned {
        signals.push(Signal {
            intent: ResponseIntent::HasReceipt,
            weight: 0.85 + attachment_weight_bonus,
        });
    }

    // ── Pick highest-weight signal ──────────────────────────────────────────
    if let Some(best) = signals.into_iter().max_by(|a, b| {
        a.weight.partial_cmp(&b.weight).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        (best.intent, best.weight.min(1.0))
    } else {
        (ResponseIntent::Unclassified, 0.0)
    }
}

// ---------------------------------------------------------------------------
// Data extraction helpers
// ---------------------------------------------------------------------------

fn extract_data(raw: &str, lower: &str) -> ExtractedData {
    ExtractedData {
        order_number:          extract_order_number(raw, lower),
        receipt_numbers:       extract_receipt_numbers(raw),
        amounts:               extract_amounts(raw),
        contact_email:         extract_email_address(raw),
        attachments_mentioned: lower.contains("bifogad")
            || lower.contains("bifogar")
            || lower.contains("attach")
            || lower.contains("se bilaga")
            || lower.contains("see attached"),
    }
}

fn extract_order_number(raw: &str, lower: &str) -> Option<String> {
    // Look for common order-number patterns after Swedish/English labels.
    let label_re = Regex::new(
        r"(?i)(?:ordernummer|order\s*(?:nr|number|#|no\.?)|beställningsnummer|po\s*(?:number|#|no\.?))[:\s#]*([A-Z0-9\-]{4,30})"
    ).expect("order_number regex");

    if let Some(cap) = label_re.captures(raw) {
        return Some(cap[1].trim().to_string());
    }

    // Fallback: bare order-ish tokens (e.g. "ORD-20260510-0042")
    let bare_re = Regex::new(r"\b(ORD|PO|REF|BKN|RES)-[\dA-Z]{4,20}\b").expect("bare_order regex");
    bare_re.captures(raw).map(|c| c[0].to_string())
}

fn extract_receipt_numbers(raw: &str) -> Vec<String> {
    let re = Regex::new(
        r"(?i)(?:kvittonummer|receipt\s*(?:nr|number|#)|fakturanummer|invoice\s*(?:nr|number|#))[:\s#]*([A-Z0-9\-]{3,30})"
    ).expect("receipt_number regex");

    re.captures_iter(raw)
        .map(|c| c[1].trim().to_string())
        .collect()
}

fn extract_amounts(raw: &str) -> Vec<String> {
    // Match numbers like "1 234,56 SEK", "€ 99.00", "1234.56 EUR"
    let re = Regex::new(
        r"(?i)(?:[€$£][\s]?)?(\d{1,3}(?:[\s,\.]\d{3})*(?:[,\.]\d{1,2})?)\s*(?:SEK|EUR|USD|GBP|kr)?"
    ).expect("amount regex");

    re.captures_iter(raw)
        .filter_map(|c| {
            let amount = c[0].trim().to_string();
            // Filter out noise (single short digits, years, etc.)
            if amount.len() > 3 && !amount.chars().all(|c| c == '0' || c == ',') {
                Some(amount)
            } else {
                None
            }
        })
        .take(10)
        .collect()
}

fn extract_email_address(raw: &str) -> Option<String> {
    let re = Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").expect("email regex");
    re.find(raw).map(|m| m.as_str().to_lowercase())
}

/// Extract a return date from OOO messages.
///
/// Tries common Swedish and English patterns, returns ISO-style string when found.
fn extract_return_date(lower: &str) -> Option<String> {
    // Patterns: "tillbaka den 3 juni", "return on 2026-06-03", "back on June 3"
    let patterns: &[&str] = &[
        r"(?:tillbaka|återvänder|åter)\s+(?:den\s+)?(\d{1,2}[/\-\.]\d{1,2}(?:[/\-\.]\d{2,4})?)",
        r"(?:return(?:ing)?|back)\s+(?:on\s+)?(\d{4}-\d{2}-\d{2})",
        r"(?:return(?:ing)?|back)\s+(?:on\s+)?(\d{1,2}[/\-\.]\d{1,2}(?:[/\-\.]\d{2,4})?)",
        r"(?:from\s+)?(\d{4}-\d{2}-\d{2})\s+(?:onwards|forward)",
        r"(?:fr\.?o\.?m\.?|from)\s+(\d{1,2}[/\-\.]\d{1,2}(?:[/\-\.]\d{2,4})?)",
    ];

    for pat in patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(cap) = re.captures(lower) {
                return Some(cap[1].to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_has_receipt() {
        let body    = "Hej, kvitto bifogas i detta svar. MVH Ekonomi";
        let subject = "SV: Förfrågan om kvitto";
        let result  = parse_response(body, subject);
        assert_eq!(result.intent, ResponseIntent::HasReceipt);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn detects_cannot_find_sv() {
        let body    = "Hej, vi har sökt i våra system men hittar inte den aktuella transaktionen.";
        let subject = "SV: Kvitto saknas";
        let result  = parse_response(body, subject);
        assert_eq!(result.intent, ResponseIntent::CannotFind);
    }

    #[test]
    fn detects_cannot_find_en() {
        let body    = "Hello, we searched our system but are unable to find any matching transaction.";
        let subject = "Re: Receipt request";
        let result  = parse_response(body, subject);
        assert_eq!(result.intent, ResponseIntent::CannotFind);
    }

    #[test]
    fn detects_ooo_sv() {
        let body    = "Automatiskt svar: Jag är på semester till och med 10 juni.";
        let subject = "Automatiskt svar";
        let result  = parse_response(body, subject);
        assert!(matches!(result.intent, ResponseIntent::OutOfOffice { .. }));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn detects_ooo_en() {
        let body    = "Out of office: I will return on 2026-06-03. Please contact my colleague.";
        let subject = "Automatic reply";
        let result  = parse_response(body, subject);
        match &result.intent {
            ResponseIntent::OutOfOffice { return_date } => {
                assert_eq!(return_date.as_deref(), Some("2026-06-03"));
            }
            other => panic!("Expected OutOfOffice, got {:?}", other),
        }
    }

    #[test]
    fn detects_needs_order_number() {
        let body    = "Hej, vi behöver ett ordernummer för att kunna söka upp transaktionen.";
        let subject = "SV: Kvittobegäran";
        let result  = parse_response(body, subject);
        assert!(matches!(result.intent, ResponseIntent::NeedsMoreInfo { .. }));
    }

    #[test]
    fn detects_wrong_department() {
        let body    = "Maila ekonomiavdelningen på faktura@example.com så hjälper de er.";
        let subject = "Re: Förfrågan";
        let result  = parse_response(body, subject);
        assert!(matches!(result.intent, ResponseIntent::WrongDepartment { .. }));
    }

    #[test]
    fn extracts_email_from_wrong_department() {
        let body    = "Please contact our accounts payable team at ap@example.com.";
        let subject = "Re: Invoice request";
        let result  = parse_response(body, subject);
        assert_eq!(
            result.extracted_data.contact_email,
            Some("ap@example.com".to_string())
        );
    }

    #[test]
    fn extracts_order_number_labelled() {
        let body    = "Ordernummer: ORD-20260510-0042. Kvitto skickas separat.";
        let subject = "SV: Kvittobegäran";
        let result  = parse_response(body, subject);
        assert_eq!(
            result.extracted_data.order_number,
            Some("ORD-20260510-0042".to_string())
        );
    }

    #[test]
    fn unclassified_returns_zero_confidence() {
        let body    = "Tack för ditt mail.";
        let subject = "Re: Test";
        let result  = parse_response(body, subject);
        assert_eq!(result.intent, ResponseIntent::Unclassified);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn attachment_mentioned_flag() {
        let body    = "Se bifogad faktura för transaktionen.";
        let subject = "SV: Faktura";
        let result  = parse_response(body, subject);
        assert!(result.extracted_data.attachments_mentioned);
    }
}
