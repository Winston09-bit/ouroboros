//! Email templates for Kvittovalvet communication engine.
//!
//! All templates are available in Swedish (primary) and English (fallback).
//! Tone escalates through the five collection steps; legal language is
//! introduced at step 5 in accordance with Swedish law (Inkassolagen SFS 1974:182).

use anyhow::{anyhow, Result};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemplateId {
    /// Step 1 – automated API lookup attempt ("Vi söker automatiskt…")
    Step1ApiRequest,
    /// Step 2 – first direct email request
    Step2EmailFirst,
    /// Step 3 – polite reminder after 3 working days of silence
    Step3Reminder,
    /// Step 4 – final notice with explicit deadline
    Step4FinalNotice,
    /// Step 5 – formal legal escalation notice
    Step5LegalEscalation,
    /// Clarification request: we need the order number before we can proceed
    MissingOrderNumber,
    /// Acknowledgement: receipt/invoice successfully received
    ReceiptConfirmation,
}

impl TemplateId {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateId::Step1ApiRequest       => "step1_api_request",
            TemplateId::Step2EmailFirst       => "step2_email_first",
            TemplateId::Step3Reminder         => "step3_reminder",
            TemplateId::Step4FinalNotice      => "step4_final_notice",
            TemplateId::Step5LegalEscalation  => "step5_legal_escalation",
            TemplateId::MissingOrderNumber    => "missing_order_number",
            TemplateId::ReceiptConfirmation   => "receipt_confirmation",
        }
    }
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TemplateVars {
    pub merchant_name: String,
    pub amount: String,
    pub currency: String,
    pub transaction_date: String,
    pub reference: Option<String>,
    /// Legal sender entity, e.g. "LandveX AB"
    pub company_name: String,
    pub escalation_step: u8,
    /// ISO date string for response deadline, e.g. "2026-06-03"
    pub deadline_date: Option<String>,
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmailTemplate {
    pub subject: String,
    pub body_sv: String,
    pub body_en: String,
    /// One of "professional", "formal", "legal"
    pub tone: &'static str,
}

impl EmailTemplate {
    /// Returns the Swedish body; falls back to English.
    pub fn body_primary(&self) -> &str {
        &self.body_sv
    }

    /// Build a plain-text body with a bilingual block (Swedish + English).
    pub fn body_bilingual(&self) -> String {
        format!(
            "{}\n\n---\n\n[English below]\n\n{}",
            self.body_sv, self.body_en
        )
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Render a template with the given variables.
///
/// Returns an error only when a required variable is missing for the chosen
/// template (e.g. `deadline_date` for Step 4/5).
pub fn get_template(id: TemplateId, vars: &TemplateVars) -> Result<EmailTemplate> {
    let reference_str = vars
        .reference
        .as_deref()
        .unwrap_or("ej angivet / not provided");

    match id {
        // ------------------------------------------------------------------
        TemplateId::Step1ApiRequest => Ok(EmailTemplate {
            subject: format!(
                "Automatisk kvittobegäran – {} {} ({})",
                vars.amount, vars.currency, vars.transaction_date
            ),
            body_sv: format!(
                "Hej,\n\n\
                 {company} genomför en automatisk sökning i era system efter underlag \
                 (kvitto/faktura) för nedanstående transaktion. Ni behöver inte vidta \
                 någon åtgärd om ert system tillhandahåller digitala kvitton automatiskt.\n\n\
                 Transaktionsdetaljer:\n\
                 - Handlare: {merchant}\n\
                 - Datum: {date}\n\
                 - Belopp: {amount} {currency}\n\
                 - Referens: {reference}\n\n\
                 Om er plattform kräver manuell hantering, vänligen förse oss med \
                 underlaget på detta e-postmeddelande.\n\n\
                 Med vänlig hälsning,\n\
                 {company}",
                company   = vars.company_name,
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
            ),
            body_en: format!(
                "Hello,\n\n\
                 {company} is performing an automated query to your systems for a \
                 receipt or invoice relating to the transaction below. No action is \
                 required if your platform provides digital receipts automatically.\n\n\
                 Transaction details:\n\
                 - Merchant: {merchant}\n\
                 - Date: {date}\n\
                 - Amount: {amount} {currency}\n\
                 - Reference: {reference}\n\n\
                 If your platform requires manual handling, please reply to this email \
                 with the supporting document.\n\n\
                 Best regards,\n\
                 {company}",
                company   = vars.company_name,
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
            ),
            tone: "professional",
        }),

        // ------------------------------------------------------------------
        TemplateId::Step2EmailFirst => Ok(EmailTemplate {
            subject: format!(
                "Förfrågan om kvitto/underlag – transaktion {} {} {}",
                vars.transaction_date, vars.amount, vars.currency
            ),
            body_sv: format!(
                "Hej,\n\n\
                 Vi kontaktar er angående en transaktion som genomfördes {date} på \
                 {amount} {currency} hos {merchant}.\n\n\
                 I samband med vår löpande bokföring och revision saknar vi underlag \
                 (kvitto/faktura) för denna transaktion. Vi ber er vänligen att förse \
                 oss med:\n\n\
                 \t• Kvitto eller faktura för transaktionen\n\
                 \t• Alternativt bekräftelse via er kundportal\n\n\
                 Transaktionsdetaljer:\n\
                 - Datum:    {date}\n\
                 - Belopp:   {amount} {currency}\n\
                 - Referens: {reference}\n\n\
                 Vänligen återkom till oss inom 5 arbetsdagar.\n\n\
                 Med vänlig hälsning,\n\
                 {company}",
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                merchant  = vars.merchant_name,
                reference = reference_str,
                company   = vars.company_name,
            ),
            body_en: format!(
                "Hello,\n\n\
                 We are writing regarding a transaction dated {date} for {amount} {currency} \
                 at {merchant}.\n\n\
                 In the course of our routine bookkeeping and audit process, we are \
                 missing supporting documentation (receipt/invoice) for this transaction. \
                 We kindly ask you to provide us with:\n\n\
                 \t• A receipt or invoice for the transaction\n\
                 \t• Alternatively, confirmation via your customer portal\n\n\
                 Transaction details:\n\
                 - Date:      {date}\n\
                 - Amount:    {amount} {currency}\n\
                 - Reference: {reference}\n\n\
                 Please respond within 5 working days.\n\n\
                 Best regards,\n\
                 {company}",
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                merchant  = vars.merchant_name,
                reference = reference_str,
                company   = vars.company_name,
            ),
            tone: "professional",
        }),

        // ------------------------------------------------------------------
        TemplateId::Step3Reminder => Ok(EmailTemplate {
            subject: format!(
                "Påminnelse: Kvitto saknas – {} {} {}",
                vars.transaction_date, vars.amount, vars.currency
            ),
            body_sv: format!(
                "Hej,\n\n\
                 Vi skickar härmed en påminnelse angående vår tidigare förfrågan om \
                 kvitto/underlag för transaktionen nedan. Vi har ännu inte mottagit \
                 något svar eller underlag.\n\n\
                 Transaktionsdetaljer:\n\
                 - Handlare: {merchant}\n\
                 - Datum:    {date}\n\
                 - Belopp:   {amount} {currency}\n\
                 - Referens: {reference}\n\n\
                 Vi ber er snarast möjligt, och senast inom 3 arbetsdagar, att:\n\n\
                 \t1. Skicka kvitto eller faktura som svar på detta meddelande, eller\n\
                 \t2. Kontakta oss om ni behöver ytterligare information för att \
                 behandla vår begäran.\n\n\
                 Om underlaget redan har skickats, var vänlig att vidarebefordra det \
                 igen så att vi kan bekräfta mottagandet.\n\n\
                 Med vänlig hälsning,\n\
                 {company}",
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
                company   = vars.company_name,
            ),
            body_en: format!(
                "Hello,\n\n\
                 This is a reminder regarding our previous request for a receipt or \
                 invoice for the transaction listed below. We have not yet received \
                 a response or any supporting documentation.\n\n\
                 Transaction details:\n\
                 - Merchant:  {merchant}\n\
                 - Date:      {date}\n\
                 - Amount:    {amount} {currency}\n\
                 - Reference: {reference}\n\n\
                 We kindly ask you to, within 3 working days:\n\n\
                 \t1. Reply to this email with the receipt or invoice, or\n\
                 \t2. Contact us if you need further information to process our request.\n\n\
                 If the documentation has already been sent, please forward it again so \
                 we can confirm receipt.\n\n\
                 Best regards,\n\
                 {company}",
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
                company   = vars.company_name,
            ),
            tone: "professional",
        }),

        // ------------------------------------------------------------------
        TemplateId::Step4FinalNotice => {
            let deadline = vars
                .deadline_date
                .as_deref()
                .ok_or_else(|| anyhow!("Step4FinalNotice requires deadline_date"))?
                .to_owned();

            Ok(EmailTemplate {
                subject: format!(
                    "SLUTLIG PÅMINNELSE: Kvitto/underlag krävs senast {} – {} {} {}",
                    deadline, vars.transaction_date, vars.amount, vars.currency
                ),
                body_sv: format!(
                    "Hej,\n\n\
                     Trots tidigare kontakt har vi ännu inte mottagit begärt underlag \
                     (kvitto/faktura) för nedanstående transaktion. Vi kontaktar er nu \
                     för sista gången innan ärendet hanteras vidare.\n\n\
                     Transaktionsdetaljer:\n\
                     - Handlare: {merchant}\n\
                     - Datum:    {date}\n\
                     - Belopp:   {amount} {currency}\n\
                     - Referens: {reference}\n\n\
                     Vi ber er att senast {deadline} sända oss:\n\n\
                     \t• Kvitto eller faktura för transaktionen, eller\n\
                     \t• Skriftligt svar med förklaring till varför underlaget inte \
                     kan tillhandahållas.\n\n\
                     Om vi inte har mottagit svar inom angiven tid kommer ärendet att \
                     hanteras enligt vår interna eskaleringsprocess, vilket kan innebära \
                     ytterligare administrativa åtgärder.\n\n\
                     Vänligen behandla detta ärende med hög prioritet.\n\n\
                     Med vänlig hälsning,\n\
                     {company}",
                    merchant  = vars.merchant_name,
                    date      = vars.transaction_date,
                    amount    = vars.amount,
                    currency  = vars.currency,
                    reference = reference_str,
                    deadline  = deadline,
                    company   = vars.company_name,
                ),
                body_en: format!(
                    "Hello,\n\n\
                     Despite previous correspondence, we have not yet received the \
                     requested supporting documentation (receipt/invoice) for the \
                     transaction below. This is our final notice before the matter is \
                     escalated internally.\n\n\
                     Transaction details:\n\
                     - Merchant:  {merchant}\n\
                     - Date:      {date}\n\
                     - Amount:    {amount} {currency}\n\
                     - Reference: {reference}\n\n\
                     We require that by {deadline} you provide us with:\n\n\
                     \t• A receipt or invoice for the transaction, or\n\
                     \t• A written explanation as to why the documentation cannot \
                     be provided.\n\n\
                     Failure to respond by the stated deadline will result in the \
                     matter being handled through our internal escalation process, \
                     which may involve further administrative action.\n\n\
                     Please treat this matter with high priority.\n\n\
                     Best regards,\n\
                     {company}",
                    merchant  = vars.merchant_name,
                    date      = vars.transaction_date,
                    amount    = vars.amount,
                    currency  = vars.currency,
                    reference = reference_str,
                    deadline  = deadline,
                    company   = vars.company_name,
                ),
                tone: "formal",
            })
        }

        // ------------------------------------------------------------------
        TemplateId::Step5LegalEscalation => {
            let deadline = vars
                .deadline_date
                .as_deref()
                .ok_or_else(|| anyhow!("Step5LegalEscalation requires deadline_date"))?
                .to_owned();

            Ok(EmailTemplate {
                subject: format!(
                    "Formellt krav på bokföringsunderlag – lagstadgad skyldighet – {} {} {}",
                    vars.transaction_date, vars.amount, vars.currency
                ),
                body_sv: format!(
                    "Till behörig befattningshavare hos {merchant},\n\n\
                     {company} (org.nr 559141-7042) riktar härmed ett formellt krav \
                     avseende kvitto/faktura för transaktionen specificerad nedan.\n\n\
                     Transaktionsdetaljer:\n\
                     - Handlare: {merchant}\n\
                     - Datum:    {date}\n\
                     - Belopp:   {amount} {currency}\n\
                     - Referens: {reference}\n\n\
                     RÄTTSLIG GRUND\n\
                     Enligt bokföringslagen (SFS 1999:1078) 5 kap. 6–7 §§ är säljaren \
                     skyldig att på begäran utfärda verifikation (kvitto/faktura) vid \
                     försäljning till annan juridisk person. Underlåtelse att tillhandahålla \
                     sådant underlag kan utgöra bokföringsbrott enligt brottsbalken \
                     (SFS 1962:700) 11 kap. 5 §.\n\n\
                     KRAV\n\
                     Vi kräver att ni senast {deadline} tillhandahåller ett av följande:\n\n\
                     \t1. Originalkvitto eller faktura för transaktionen, eller\n\
                     \t2. Duplett/kopia med angivande av originaldatum och löpnummer, eller\n\
                     \t3. Skriftlig förklaring på brevhuvud med namnteckning av behörig \
                     person, som anger varför underlag inte kan tillhandahållas.\n\n\
                     KONSEKVENSER VID UNDERLÅTELSE\n\
                     Om vi inte erhåller tillfredsställande svar inom angiven tid förbehåller \
                     sig {company} rätten att:\n\n\
                     \t• Anmäla ärendet till Skatteverket för granskning\n\
                     \t• Vidarebefordra ärendet till juridiskt ombud för vidare hantering\n\
                     \t• Ansöka om tillsyn hos Bolagsverket\n\n\
                     Detta är ett formellt juridiskt meddelande. Vänligen behandla det \
                     i enlighet därmed och säkerställ att det når behörig befattningshavare \
                     inom er organisation.\n\n\
                     Med hänvisning till gällande lagstiftning,\n\
                     {company}\n\
                     org.nr 559141-7042",
                    merchant  = vars.merchant_name,
                    date      = vars.transaction_date,
                    amount    = vars.amount,
                    currency  = vars.currency,
                    reference = reference_str,
                    deadline  = deadline,
                    company   = vars.company_name,
                ),
                body_en: format!(
                    "To the responsible officer at {merchant},\n\n\
                     {company} (reg. no. 559141-7042) hereby submits a formal demand \
                     for a receipt or invoice relating to the transaction specified below.\n\n\
                     Transaction details:\n\
                     - Merchant:  {merchant}\n\
                     - Date:      {date}\n\
                     - Amount:    {amount} {currency}\n\
                     - Reference: {reference}\n\n\
                     LEGAL BASIS\n\
                     Under the Swedish Bookkeeping Act (SFS 1999:1078), Chapter 5, \
                     Sections 6–7, a seller is obligated upon request to issue a \
                     verification document (receipt/invoice) for sales to other legal \
                     entities. Failure to provide such documentation may constitute \
                     a bookkeeping offence under the Swedish Penal Code (SFS 1962:700), \
                     Chapter 11, Section 5.\n\n\
                     DEMAND\n\
                     We require that by {deadline} you provide one of the following:\n\n\
                     \t1. Original receipt or invoice for the transaction, or\n\
                     \t2. Duplicate/copy stating the original date and serial number, or\n\
                     \t3. A written explanation on company letterhead, signed by an \
                     authorised officer, explaining why documentation cannot be provided.\n\n\
                     CONSEQUENCES OF NON-COMPLIANCE\n\
                     Should we not receive a satisfactory response within the stated \
                     time, {company} reserves the right to:\n\n\
                     \t• Report the matter to the Swedish Tax Agency (Skatteverket)\n\
                     \t• Refer the matter to legal counsel for further action\n\
                     \t• File a supervisory complaint with the Swedish Companies \
                     Registration Office (Bolagsverket)\n\n\
                     This is a formal legal notice. Please handle it accordingly and \
                     ensure it reaches the relevant authorised officer within your \
                     organisation.\n\n\
                     With reference to applicable legislation,\n\
                     {company}\n\
                     Reg. no. 559141-7042",
                    merchant  = vars.merchant_name,
                    date      = vars.transaction_date,
                    amount    = vars.amount,
                    currency  = vars.currency,
                    reference = reference_str,
                    deadline  = deadline,
                    company   = vars.company_name,
                ),
                tone: "legal",
            })
        }

        // ------------------------------------------------------------------
        TemplateId::MissingOrderNumber => Ok(EmailTemplate {
            subject: format!(
                "Svar: Vi behöver ordernummer – {} {} {}",
                vars.transaction_date, vars.amount, vars.currency
            ),
            body_sv: format!(
                "Hej,\n\n\
                 Tack för ert svar angående transaktionen nedan.\n\n\
                 För att vi ska kunna lokalisera och tillhandahålla rätt underlag \
                 behöver vi ett ordernummer eller bokningsnummer kopplat till \
                 transaktionen.\n\n\
                 Transaktionsdetaljer:\n\
                 - Handlare: {merchant}\n\
                 - Datum:    {date}\n\
                 - Belopp:   {amount} {currency}\n\
                 - Referens: {reference}\n\n\
                 Vänligen återkom med:\n\n\
                 \t• Ordernummer / bokningsnummer\n\
                 \t• Alternativt kortnummrets fyra sista siffror vid köptillfället\n\n\
                 När vi mottagit denna information behandlar vi er begäran snarast.\n\n\
                 Med vänlig hälsning,\n\
                 {company}",
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
                company   = vars.company_name,
            ),
            body_en: format!(
                "Hello,\n\n\
                 Thank you for your reply regarding the transaction below.\n\n\
                 In order to locate and provide the correct documentation, we need \
                 an order number or booking reference associated with the transaction.\n\n\
                 Transaction details:\n\
                 - Merchant:  {merchant}\n\
                 - Date:      {date}\n\
                 - Amount:    {amount} {currency}\n\
                 - Reference: {reference}\n\n\
                 Please provide us with:\n\n\
                 \t• Order number / booking reference\n\
                 \t• Alternatively, the last four digits of the card used at the \
                 time of purchase\n\n\
                 Once we receive this information, we will process your request promptly.\n\n\
                 Best regards,\n\
                 {company}",
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
                company   = vars.company_name,
            ),
            tone: "professional",
        }),

        // ------------------------------------------------------------------
        TemplateId::ReceiptConfirmation => Ok(EmailTemplate {
            subject: format!(
                "Bekräftelse: Underlag mottaget – {} {} {}",
                vars.transaction_date, vars.amount, vars.currency
            ),
            body_sv: format!(
                "Hej,\n\n\
                 Vi bekräftar härmed att vi har mottagit begärt underlag \
                 (kvitto/faktura) för transaktionen nedan.\n\n\
                 Transaktionsdetaljer:\n\
                 - Handlare: {merchant}\n\
                 - Datum:    {date}\n\
                 - Belopp:   {amount} {currency}\n\
                 - Referens: {reference}\n\n\
                 Dokumentet har registrerats i vårt bokföringssystem. \
                 Ärendet är härmed avslutat och ingen ytterligare åtgärd krävs \
                 från er sida.\n\n\
                 Tack för er samarbetsvilja.\n\n\
                 Med vänlig hälsning,\n\
                 {company}",
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
                company   = vars.company_name,
            ),
            body_en: format!(
                "Hello,\n\n\
                 We hereby confirm that we have received the requested documentation \
                 (receipt/invoice) for the transaction below.\n\n\
                 Transaction details:\n\
                 - Merchant:  {merchant}\n\
                 - Date:      {date}\n\
                 - Amount:    {amount} {currency}\n\
                 - Reference: {reference}\n\n\
                 The document has been registered in our accounting system. \
                 This matter is now closed and no further action is required on your part.\n\n\
                 Thank you for your cooperation.\n\n\
                 Best regards,\n\
                 {company}",
                merchant  = vars.merchant_name,
                date      = vars.transaction_date,
                amount    = vars.amount,
                currency  = vars.currency,
                reference = reference_str,
                company   = vars.company_name,
            ),
            tone: "professional",
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vars() -> TemplateVars {
        TemplateVars {
            merchant_name:    "Acme Hotel AB".to_string(),
            amount:           "1 234,00".to_string(),
            currency:         "SEK".to_string(),
            transaction_date: "2026-05-10".to_string(),
            reference:        Some("TXN-20260510-001".to_string()),
            company_name:     "LandveX AB".to_string(),
            escalation_step:  2,
            deadline_date:    None,
        }
    }

    #[test]
    fn step2_renders() {
        let tpl = get_template(TemplateId::Step2EmailFirst, &sample_vars()).unwrap();
        assert!(tpl.body_sv.contains("1 234,00"));
        assert!(tpl.body_sv.contains("Acme Hotel AB"));
        assert!(tpl.body_en.contains("1 234,00"));
        assert_eq!(tpl.tone, "professional");
    }

    #[test]
    fn step4_requires_deadline() {
        let vars = sample_vars();
        assert!(get_template(TemplateId::Step4FinalNotice, &vars).is_err());
    }

    #[test]
    fn step4_renders_with_deadline() {
        let mut vars = sample_vars();
        vars.deadline_date = Some("2026-06-03".to_string());
        let tpl = get_template(TemplateId::Step4FinalNotice, &vars).unwrap();
        assert!(tpl.body_sv.contains("2026-06-03"));
        assert_eq!(tpl.tone, "formal");
    }

    #[test]
    fn step5_legal_tone() {
        let mut vars = sample_vars();
        vars.deadline_date = Some("2026-06-10".to_string());
        let tpl = get_template(TemplateId::Step5LegalEscalation, &vars).unwrap();
        assert!(tpl.body_sv.contains("SFS 1999:1078"));
        assert_eq!(tpl.tone, "legal");
    }

    #[test]
    fn confirmation_renders() {
        let tpl = get_template(TemplateId::ReceiptConfirmation, &sample_vars()).unwrap();
        assert!(tpl.body_sv.contains("avslutat"));
    }
}
