use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Account type in the double-entry system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
    CostOfGoods,
}

/// Normal balance direction for an account (debit or credit increases balance)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalBalance {
    Debit,
    Credit,
}

/// VAT classification for Swedish BAS 2024
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VatClass {
    Standard25,
    Reduced12,
    Reduced6,
    Zero,
    Exempt,
    NotApplicable,
}

impl VatClass {
    pub fn rate(&self) -> Decimal {
        match self {
            VatClass::Standard25 => Decimal::new(25, 2),
            VatClass::Reduced12 => Decimal::new(12, 2),
            VatClass::Reduced6 => Decimal::new(6, 2),
            VatClass::Zero => Decimal::ZERO,
            VatClass::Exempt => Decimal::ZERO,
            VatClass::NotApplicable => Decimal::ZERO,
        }
    }
}

/// A single account in the Chart of Accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub normal_balance: NormalBalance,
    pub vat_class: VatClass,
    pub parent_code: Option<String>,
    pub is_active: bool,
    pub description: Option<String>,
}

impl Account {
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        account_type: AccountType,
        normal_balance: NormalBalance,
        vat_class: VatClass,
        parent_code: Option<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            account_type,
            normal_balance,
            vat_class,
            parent_code,
            is_active: true,
            description,
        }
    }
}

/// Full BAS 2024 Chart of Accounts registry
pub struct ChartOfAccounts {
    accounts: HashMap<String, Account>,
}

impl ChartOfAccounts {
    /// Build the standard BAS 2024 chart of accounts
    pub fn bas_2024() -> Self {
        let mut accounts = HashMap::new();

        let entries: Vec<Account> = vec![
            // ── Class 1: Assets ──────────────────────────────────────────
            Account::new("1", "Tillgångar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, None, Some("Tillgångar (klass 1)")),

            // Immateriella anläggningstillgångar
            Account::new("10", "Immateriella anläggningstillgångar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1010", "Balanserade utgifter för forskning och utveckling", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1020", "Koncessioner m.m.", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1030", "Patent", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1040", "Licenser", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1050", "Varumärken", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1060", "Hyresrätter och liknande rättigheter", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1070", "Goodwill", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),
            Account::new("1080", "Förskott för immateriella anläggningstillgångar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("10".into()), None),

            // Byggnader och mark
            Account::new("11", "Byggnader och mark", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1110", "Byggnader", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("11".into()), None),
            Account::new("1111", "Byggnader, anskaffningsvärde", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("11".into()), None),
            Account::new("1119", "Ackumulerade avskrivningar på byggnader", AccountType::Asset, NormalBalance::Credit, VatClass::NotApplicable, Some("11".into()), None),
            Account::new("1130", "Markanläggningar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("11".into()), None),
            Account::new("1150", "Mark", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("11".into()), None),
            Account::new("1180", "Pågående nyanläggningar och förskott för byggnader och mark", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("11".into()), None),

            // Maskiner och inventarier
            Account::new("12", "Maskiner och inventarier", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1210", "Maskiner och andra tekniska anläggningar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("12".into()), None),
            Account::new("1220", "Inventarier och verktyg", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("12".into()), None),
            Account::new("1229", "Ackumulerade avskrivningar på inventarier och verktyg", AccountType::Asset, NormalBalance::Credit, VatClass::NotApplicable, Some("12".into()), None),
            Account::new("1230", "Datorer", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("12".into()), None),
            Account::new("1240", "Bilar och andra transportmedel", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("12".into()), None),
            Account::new("1250", "Leasade tillgångar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("12".into()), None),
            Account::new("1280", "Pågående nyanläggningar och förskott för maskiner och inventarier", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("12".into()), None),

            // Finansiella anläggningstillgångar
            Account::new("13", "Finansiella anläggningstillgångar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1310", "Andelar i koncernföretag", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("13".into()), None),
            Account::new("1320", "Långfristiga fordringar hos koncernföretag", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("13".into()), None),
            Account::new("1330", "Andelar i intresseföretag", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("13".into()), None),
            Account::new("1350", "Ägarintressen i övriga företag", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("13".into()), None),
            Account::new("1380", "Andra långfristiga fordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("13".into()), None),

            // Varulager
            Account::new("14", "Lager, produkter i arbete och pågående arbeten", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1410", "Lager av råvaror och förnödenheter", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("14".into()), None),
            Account::new("1420", "Lager av handelsvaror", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("14".into()), None),
            Account::new("1430", "Produkter i arbete", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("14".into()), None),
            Account::new("1440", "Lager av färdiga varor", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("14".into()), None),
            Account::new("1470", "Pågående arbeten", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("14".into()), None),
            Account::new("1480", "Förskott till leverantörer", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("14".into()), None),

            // Kortfristiga fordringar
            Account::new("15", "Kundfordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1500", "Kundfordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("15".into()), Some("Kundfordringar")),
            Account::new("1510", "Kundfordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("15".into()), None),
            Account::new("1515", "Kundfordringar, osäkra", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("15".into()), None),
            Account::new("1519", "Nedskrivning av kundfordringar", AccountType::Asset, NormalBalance::Credit, VatClass::NotApplicable, Some("15".into()), None),

            Account::new("16", "Övriga kortfristiga fordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1610", "Fordran hos moderföretag", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), None),
            Account::new("1620", "Fordran hos dotterföretag", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), None),
            Account::new("1630", "Avräkning för skatter och avgifter (skattekonto)", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), Some("Skattekonto")),
            Account::new("1640", "Skattefordran", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), None),
            Account::new("1650", "Momsfordran", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), Some("Ingående moms > utgående moms")),
            Account::new("1660", "Kortfristiga fordringar hos anställda", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), None),
            Account::new("1680", "Övriga kortfristiga fordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), None),
            Account::new("1690", "Övriga fordringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("16".into()), None),

            Account::new("17", "Förutbetalda kostnader och upplupna intäkter", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1710", "Förutbetalda hyreskostnader", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("17".into()), None),
            Account::new("1720", "Förutbetalda leasingavgifter", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("17".into()), None),
            Account::new("1730", "Förutbetalda försäkringspremier", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("17".into()), None),
            Account::new("1740", "Förutbetalda räntekostnader", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("17".into()), None),
            Account::new("1790", "Övriga förutbetalda kostnader och upplupna intäkter", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("17".into()), None),

            // Kassa och bank
            Account::new("19", "Kassa och bank", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("1".into()), None),
            Account::new("1910", "Kassa", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), Some("Kontantkassa")),
            Account::new("1920", "PlusGiro", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), None),
            Account::new("1930", "Företagskonto / affärskonto", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), Some("Bankkonto")),
            Account::new("1940", "Övriga bankkonton", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), None),
            Account::new("1950", "Bankkonto med bindningstid", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), None),
            Account::new("1960", "Kortfristiga placeringar", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), None),
            Account::new("1980", "Valutakonton", AccountType::Asset, NormalBalance::Debit, VatClass::NotApplicable, Some("19".into()), None),

            // ── Class 2: Liabilities & Equity ────────────────────────────
            Account::new("2", "Eget kapital och skulder", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, None, Some("Skulder och eget kapital (klass 2)")),

            // Eget kapital
            Account::new("20", "Eget kapital", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2010", "Aktiekapital", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),
            Account::new("2020", "Ej registrerat aktiekapital", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),
            Account::new("2030", "Överkursfond", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),
            Account::new("2040", "Reservfond", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),
            Account::new("2050", "Balanserat resultat", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), Some("Balanserade vinstmedel")),
            Account::new("2060", "Årets resultat", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),
            Account::new("2070", "Insatskapital / Fritt eget kapital", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),
            Account::new("2080", "Kapitalandelsfond", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("20".into()), None),

            // Obeskattade reserver
            Account::new("21", "Obeskattade reserver", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2110", "Periodiseringsfonder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("21".into()), None),
            Account::new("2120", "Ackumulerade överavskrivningar", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("21".into()), None),
            Account::new("2150", "Ersättningsfonder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("21".into()), None),

            // Avsättningar
            Account::new("22", "Avsättningar", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2210", "Avsättningar för pensioner och liknande", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("22".into()), None),
            Account::new("2220", "Avsättningar för skatter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("22".into()), None),
            Account::new("2290", "Övriga avsättningar", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("22".into()), None),

            // Långfristiga skulder
            Account::new("23", "Långfristiga skulder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2310", "Obligationslån", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("23".into()), None),
            Account::new("2320", "Checkräkningskredit", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("23".into()), None),
            Account::new("2330", "Byggnadskreditiv", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("23".into()), None),
            Account::new("2350", "Skulder till kreditinstitut", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("23".into()), None),
            Account::new("2360", "Skulder till koncernföretag", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("23".into()), None),
            Account::new("2390", "Övriga långfristiga skulder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("23".into()), None),

            // Kortfristiga skulder
            Account::new("24", "Kortfristiga skulder till kreditinstitut", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2410", "Kortfristiga skulder till kreditinstitut", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("24".into()), None),
            Account::new("2420", "Checkkrediter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("24".into()), None),

            Account::new("25", "Leverantörsskulder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2440", "Leverantörsskulder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("25".into()), Some("Accounts payable")),
            Account::new("2441", "Leverantörsskulder i SEK", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("25".into()), None),
            Account::new("2442", "Leverantörsskulder i utländsk valuta", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("25".into()), None),

            // Skatter och avgifter
            Account::new("26", "Skatteskulder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2610", "Utgående moms, 25 %", AccountType::Liability, NormalBalance::Credit, VatClass::Standard25, Some("26".into()), Some("Utgående moms 25%")),
            Account::new("2611", "Utgående moms, försäljning inom Sverige 25 %", AccountType::Liability, NormalBalance::Credit, VatClass::Standard25, Some("26".into()), None),
            Account::new("2620", "Utgående moms, 12 %", AccountType::Liability, NormalBalance::Credit, VatClass::Reduced12, Some("26".into()), Some("Utgående moms 12%")),
            Account::new("2630", "Utgående moms, 6 %", AccountType::Liability, NormalBalance::Credit, VatClass::Reduced6, Some("26".into()), Some("Utgående moms 6%")),
            Account::new("2640", "Ingående moms", AccountType::Asset, NormalBalance::Debit, VatClass::Standard25, Some("26".into()), Some("Ingående moms")),
            Account::new("2641", "Ingående moms, 25 %", AccountType::Asset, NormalBalance::Debit, VatClass::Standard25, Some("26".into()), None),
            Account::new("2642", "Ingående moms, 12 %", AccountType::Asset, NormalBalance::Debit, VatClass::Reduced12, Some("26".into()), None),
            Account::new("2643", "Ingående moms, 6 %", AccountType::Asset, NormalBalance::Debit, VatClass::Reduced6, Some("26".into()), None),
            Account::new("2650", "Redovisningskonto för moms", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),
            Account::new("2710", "Personalskatt", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),
            Account::new("2720", "Lagstadgade sociala avgifter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),
            Account::new("2730", "Avdragen skatt vid utbetalning", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),
            Account::new("2750", "Momsskuld", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),
            Account::new("2760", "Skatteskuld", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),
            Account::new("2790", "Övriga skatteskulder", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("26".into()), None),

            // Upplupna kostnader
            Account::new("28", "Upplupna kostnader och förutbetalda intäkter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("2".into()), None),
            Account::new("2820", "Upplupna löner", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),
            Account::new("2830", "Upplupna semesterlöner och semesterersättningar", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),
            Account::new("2840", "Upplupna lagstadgade sociala avgifter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),
            Account::new("2850", "Upplupna avtalsenliga sociala avgifter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),
            Account::new("2860", "Upplupna räntekostnader", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),
            Account::new("2870", "Förutbetalda hyresintäkter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),
            Account::new("2890", "Övriga upplupna kostnader och förutbetalda intäkter", AccountType::Liability, NormalBalance::Credit, VatClass::NotApplicable, Some("28".into()), None),

            // ── Class 3: Revenue ─────────────────────────────────────────
            Account::new("3", "Rörelsens inkomster / intäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, None, Some("Intäkter (klass 3)")),

            Account::new("30", "Huvudintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("3".into()), None),
            Account::new("3010", "Försäljning av varor, momspliktig 25 %", AccountType::Revenue, NormalBalance::Credit, VatClass::Standard25, Some("30".into()), Some("Varuförsäljning 25% moms")),
            Account::new("3011", "Försäljning av varor, momspliktig 12 %", AccountType::Revenue, NormalBalance::Credit, VatClass::Reduced12, Some("30".into()), None),
            Account::new("3012", "Försäljning av varor, momspliktig 6 %", AccountType::Revenue, NormalBalance::Credit, VatClass::Reduced6, Some("30".into()), None),
            Account::new("3040", "Försäljning av varor, momsfri", AccountType::Revenue, NormalBalance::Credit, VatClass::Exempt, Some("30".into()), None),
            Account::new("3051", "Försäljning av varor till länder utanför EU", AccountType::Revenue, NormalBalance::Credit, VatClass::Zero, Some("30".into()), None),
            Account::new("3060", "Försäljning av varor till länder inom EU", AccountType::Revenue, NormalBalance::Credit, VatClass::Zero, Some("30".into()), None),

            Account::new("31", "Tjänsteintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("3".into()), None),
            Account::new("3100", "Tjänsteintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::Standard25, Some("31".into()), Some("Tjänsteförsäljning 25% moms")),
            Account::new("3110", "Försäljning av tjänster, momspliktig 25 %", AccountType::Revenue, NormalBalance::Credit, VatClass::Standard25, Some("31".into()), None),
            Account::new("3120", "Försäljning av tjänster, momspliktig 12 %", AccountType::Revenue, NormalBalance::Credit, VatClass::Reduced12, Some("31".into()), None),
            Account::new("3130", "Försäljning av tjänster, momspliktig 6 %", AccountType::Revenue, NormalBalance::Credit, VatClass::Reduced6, Some("31".into()), None),
            Account::new("3140", "Försäljning av tjänster, momsfri", AccountType::Revenue, NormalBalance::Credit, VatClass::Exempt, Some("31".into()), None),
            Account::new("3160", "Försäljning av tjänster till länder inom EU", AccountType::Revenue, NormalBalance::Credit, VatClass::Zero, Some("31".into()), None),
            Account::new("3170", "Försäljning av tjänster till länder utanför EU", AccountType::Revenue, NormalBalance::Credit, VatClass::Zero, Some("31".into()), None),

            Account::new("38", "Övriga rörelseintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("3".into()), None),
            Account::new("3810", "Hyresintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("38".into()), None),
            Account::new("3850", "Provisionsintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::Standard25, Some("38".into()), None),
            Account::new("3890", "Övriga rörelseintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("38".into()), None),

            Account::new("39", "Rabatter, returer och kreditnotor", AccountType::Revenue, NormalBalance::Debit, VatClass::NotApplicable, Some("3".into()), None),
            Account::new("3910", "Lämnade rabatter", AccountType::Revenue, NormalBalance::Debit, VatClass::Standard25, Some("39".into()), None),
            Account::new("3920", "Returvaror", AccountType::Revenue, NormalBalance::Debit, VatClass::Standard25, Some("39".into()), None),

            // ── Class 4: Cost of Goods ───────────────────────────────────
            Account::new("4", "Kostnader för varor, material och tjänster", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::NotApplicable, None, Some("Rörelsekostnader (klass 4)")),

            Account::new("40", "Handelsvaror", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::NotApplicable, Some("4".into()), None),
            Account::new("4000", "Inköp av varor", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Standard25, Some("40".into()), Some("Varuinköp 25% moms")),
            Account::new("4010", "Inköp av varor, 25 %", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Standard25, Some("40".into()), None),
            Account::new("4011", "Inköp av varor, 12 %", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Reduced12, Some("40".into()), None),
            Account::new("4012", "Inköp av varor, 6 %", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Reduced6, Some("40".into()), None),
            Account::new("4060", "Inköp av varor från länder inom EU", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Zero, Some("40".into()), None),
            Account::new("4069", "Inköp av varor från länder utanför EU", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Zero, Some("40".into()), None),
            Account::new("4090", "Förändring av lager av handelsvaror", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::NotApplicable, Some("40".into()), None),

            Account::new("44", "Underentreprenörer och inhyrda tjänster", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::NotApplicable, Some("4".into()), None),
            Account::new("4400", "Underentreprenörer", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Standard25, Some("44".into()), None),
            Account::new("4420", "Inhyrda tjänster", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Standard25, Some("44".into()), None),

            Account::new("49", "Övriga rörelsekostnader (klass 4)", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::NotApplicable, Some("4".into()), None),
            Account::new("4900", "Övriga kostnader för tjänsteproduktion", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Standard25, Some("49".into()), None),
            Account::new("4910", "Förpackningsmaterial", AccountType::CostOfGoods, NormalBalance::Debit, VatClass::Standard25, Some("49".into()), None),

            // ── Class 5: Other Operating Costs ───────────────────────────
            Account::new("5", "Övriga externa rörelsekostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, None, Some("Övriga rörelsekostnader (klass 5)")),

            Account::new("51", "Lokalkostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5010", "Lokalhyra", AccountType::Expense, NormalBalance::Debit, VatClass::Exempt, Some("51".into()), Some("Hyra lokaler")),
            Account::new("5020", "El för belysning", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("51".into()), None),
            Account::new("5060", "Städning och renhållning", AccountType::Expense, NormalBalance::Debit, VatClass::Reduced12, Some("51".into()), None),
            Account::new("5090", "Övriga lokalkostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("51".into()), None),

            Account::new("52", "Fastighetskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5210", "Hyra av anläggningstillgångar", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("52".into()), None),

            Account::new("54", "Fordonskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5410", "Drivmedel", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("54".into()), None),
            Account::new("5420", "Försäkring och skatt för fordon", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("54".into()), None),
            Account::new("5460", "Bilersättningar", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("54".into()), None),
            Account::new("5490", "Övriga fordonskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("54".into()), None),

            Account::new("55", "Kostnader för IT, telefon och kommunikation", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5510", "Förbrukningsinventarier och förbrukningsmaterial", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("55".into()), None),
            Account::new("5610", "Programvaror och IT-tjänster", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("55".into()), Some("Programvaror, SaaS")),
            Account::new("5620", "Molntjänster och hosting", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("55".into()), Some("AWS, Azure, GCP")),
            Account::new("5630", "Telefon och fax", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("55".into()), None),
            Account::new("5640", "Datakommunikation", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("55".into()), None),
            Account::new("5690", "Övriga kommunikationskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("55".into()), None),

            Account::new("57", "Reklam och PR", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5710", "Reklam och annonsering", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("57".into()), None),
            Account::new("5720", "Utställningar och mässor", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("57".into()), None),
            Account::new("5730", "Trycksaker och kataloger", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("57".into()), None),
            Account::new("5790", "Övriga marknadsföringskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("57".into()), None),

            Account::new("58", "Resekostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5800", "Resekostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("58".into()), None),
            Account::new("5810", "Resekostnader vid tjänsteresa", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("58".into()), None),
            Account::new("5820", "Hotellkostnader vid tjänsteresa", AccountType::Expense, NormalBalance::Debit, VatClass::Reduced12, Some("58".into()), None),

            Account::new("59", "Övriga externa tjänster", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("5".into()), None),
            Account::new("5900", "Övriga externa tjänster", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("59".into()), None),
            Account::new("5910", "Revisionsarvode", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("59".into()), None),
            Account::new("5920", "Redovisningskonsult", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("59".into()), None),
            Account::new("5930", "Juridisk rådgivning", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("59".into()), None),
            Account::new("5940", "Övriga externa konsulttjänster", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("59".into()), None),

            // ── Class 6: Other Operating Costs cont. ─────────────────────
            Account::new("6", "Övriga externa rörelsekostnader (forts.)", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, None, Some("Övriga rörelsekostnader (klass 6)")),

            Account::new("61", "Kontorskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("6".into()), None),
            Account::new("6100", "Kontorsmaterial", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("61".into()), None),
            Account::new("6110", "Kontorsmaterial och trycksaker", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("61".into()), None),
            Account::new("6150", "Tidningar, tidskrifter och facklitteratur", AccountType::Expense, NormalBalance::Debit, VatClass::Reduced6, Some("61".into()), None),
            Account::new("6190", "Övriga kontorskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("61".into()), None),

            Account::new("62", "Tele och post", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("6".into()), None),
            Account::new("6210", "Telekommunikation", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("62".into()), None),
            Account::new("6250", "Postbefordran", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("62".into()), None),

            Account::new("63", "Försäkringspremier", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("6".into()), None),
            Account::new("6310", "Förmögenhetsskadeförsäkring", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("63".into()), None),
            Account::new("6320", "Transportförsäkring", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("63".into()), None),
            Account::new("6390", "Övriga försäkringspremier", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("63".into()), None),

            Account::new("64", "Förvaltningskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("6".into()), None),
            Account::new("6410", "Styrelsearvoden", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("64".into()), None),
            Account::new("6420", "Kostnader för bolagsstämma", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("64".into()), None),
            Account::new("6490", "Övriga förvaltningskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("64".into()), None),

            Account::new("65", "Övriga externa kostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("6".into()), None),
            Account::new("6500", "Övriga externa kostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("65".into()), None),
            Account::new("6510", "Licensavgifter och royalties", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("65".into()), None),
            Account::new("6520", "Kostnader för patent och varumärken", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("65".into()), None),
            Account::new("6530", "Bankkostnader och kortavgifter", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("65".into()), Some("Bankkostnader")),
            Account::new("6540", "Fakturaavgifter", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("65".into()), None),
            Account::new("6590", "Övriga externa kostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("65".into()), None),

            // ── Class 7: Personnel ───────────────────────────────────────
            Account::new("7", "Personalkostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, None, Some("Personalkostnader (klass 7)")),

            Account::new("70", "Löner till kollektivanställda", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("7".into()), None),
            Account::new("7010", "Löner till tjänstemän och företagsledare", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("70".into()), None),
            Account::new("7020", "Löner till kollektivanställda", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("70".into()), None),
            Account::new("7030", "Löner till tillfällig personal", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("70".into()), None),

            Account::new("72", "Förmåner", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("7".into()), None),
            Account::new("7210", "Löneförmåner, bil", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("72".into()), None),
            Account::new("7220", "Löneförmåner, bostad", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("72".into()), None),
            Account::new("7230", "Kost och logi", AccountType::Expense, NormalBalance::Debit, VatClass::Reduced12, Some("72".into()), None),
            Account::new("7290", "Övriga löneförmåner", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("72".into()), None),

            Account::new("73", "Kostnadsersättningar och förmåner", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("7".into()), None),
            Account::new("7310", "Kostnadsersättningar", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("73".into()), None),
            Account::new("7320", "Traktamenten vid tjänsteresa", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("73".into()), None),
            Account::new("7330", "Milersättningar", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("73".into()), None),
            Account::new("7380", "Representationskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("73".into()), None),
            Account::new("7390", "Övriga kostnadsersättningar", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("73".into()), None),

            Account::new("74", "Pensionskostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("7".into()), None),
            Account::new("7410", "Pensionskostnader, lagstadgade", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("74".into()), None),
            Account::new("7420", "Pensionskostnader, avtalsenliga", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("74".into()), None),

            Account::new("75", "Sociala avgifter", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("7".into()), None),
            Account::new("7510", "Lagstadgade sociala avgifter", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("75".into()), None),
            Account::new("7520", "Avtalsenliga sociala avgifter", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("75".into()), None),

            Account::new("76", "Övriga personalkostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("7".into()), None),
            Account::new("7610", "Utbildning", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("76".into()), None),
            Account::new("7620", "Företagshälsovård", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("76".into()), None),
            Account::new("7630", "Sjukförsäkring", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("76".into()), None),
            Account::new("7690", "Övriga personalkostnader", AccountType::Expense, NormalBalance::Debit, VatClass::Standard25, Some("76".into()), None),

            // ── Class 8: Financial items ─────────────────────────────────
            Account::new("8", "Finansiella poster och skatt", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, None, Some("Finansiella poster (klass 8)")),

            Account::new("82", "Ränteintäkter", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("8".into()), None),
            Account::new("8210", "Ränteintäkter från bank", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("82".into()), None),
            Account::new("8220", "Ränteintäkter, övriga", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("82".into()), None),

            Account::new("83", "Valutakursdifferenser", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("8".into()), None),
            Account::new("8310", "Valutakursvinster", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("83".into()), None),
            Account::new("8320", "Valutakursförluster", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("83".into()), None),

            Account::new("84", "Räntekostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("8".into()), None),
            Account::new("8410", "Räntekostnader för långfristiga skulder", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("84".into()), None),
            Account::new("8420", "Räntekostnader för kortfristiga skulder", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("84".into()), None),
            Account::new("8430", "Dröjsmålsräntor", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("84".into()), None),
            Account::new("8490", "Övriga räntekostnader", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("84".into()), None),

            Account::new("88", "Bokslutsdispositioner", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("8".into()), None),
            Account::new("8810", "Avsättningar till periodiseringsfonder", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("88".into()), None),
            Account::new("8820", "Återföring av periodiseringsfonder", AccountType::Revenue, NormalBalance::Credit, VatClass::NotApplicable, Some("88".into()), None),

            Account::new("89", "Skatter och årets resultat", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("8".into()), None),
            Account::new("8910", "Aktuell skattekostnad", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("89".into()), None),
            Account::new("8920", "Uppskjuten skattekostnad", AccountType::Expense, NormalBalance::Debit, VatClass::NotApplicable, Some("89".into()), None),
            Account::new("8990", "Årets resultat", AccountType::Equity, NormalBalance::Credit, VatClass::NotApplicable, Some("89".into()), None),
        ];

        for account in entries {
            accounts.insert(account.code.clone(), account);
        }

        Self { accounts }
    }

    /// Look up an account by its code
    pub fn get(&self, code: &str) -> Option<&Account> {
        self.accounts.get(code)
    }

    /// Check whether a code exists and is active
    pub fn is_valid_code(&self, code: &str) -> bool {
        self.accounts.get(code).map(|a| a.is_active).unwrap_or(false)
    }

    /// Return all accounts under a parent prefix
    pub fn children_of(&self, parent_code: &str) -> Vec<&Account> {
        self.accounts
            .values()
            .filter(|a| {
                a.parent_code
                    .as_deref()
                    .map(|p| p == parent_code)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Return all leaf accounts (4-digit codes)
    pub fn leaf_accounts(&self) -> Vec<&Account> {
        self.accounts
            .values()
            .filter(|a| a.code.len() == 4)
            .collect()
    }
}

// ── Classification ──────────────────────────────────────────────────────────

/// A suggested account code with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSuggestion {
    pub account_code: String,
    pub account_name: String,
    pub confidence: f64,
    pub reason: String,
}

/// Keyword rule for account classification
struct ClassificationRule {
    keywords: Vec<&'static str>,
    account_code: &'static str,
    reason: &'static str,
}

/// Classifies transaction descriptions to BAS 2024 account codes
pub struct AccountClassifier {
    chart: ChartOfAccounts,
    rules: Vec<ClassificationRule>,
}

impl AccountClassifier {
    pub fn new(chart: ChartOfAccounts) -> Self {
        let rules = vec![
            ClassificationRule {
                keywords: vec!["aws", "amazon web services", "ec2", "s3", "lambda", "rds", "cloudfront"],
                account_code: "5620",
                reason: "AWS molntjänster",
            },
            ClassificationRule {
                keywords: vec!["azure", "microsoft azure", "google cloud", "gcp", "digitalocean", "linode", "hetzner"],
                account_code: "5620",
                reason: "Molntjänster/hosting",
            },
            ClassificationRule {
                keywords: vec!["github", "gitlab", "bitbucket", "jira", "confluence", "notion", "slack", "figma"],
                account_code: "5610",
                reason: "Programvaror/SaaS-tjänster",
            },
            ClassificationRule {
                keywords: vec!["stripe", "adyen", "klarna", "paypal", "braintree", "kortavgift", "betalningsavgift"],
                account_code: "6530",
                reason: "Betalningsprocessingkostnader",
            },
            ClassificationRule {
                keywords: vec!["hyra", "hyresavgift", "lokalhyra", "kontorshyra"],
                account_code: "5010",
                reason: "Lokalhyra",
            },
            ClassificationRule {
                keywords: vec!["el ", "elräkning", "elförbrukning", "vattenförbrukning"],
                account_code: "5020",
                reason: "El/vatten",
            },
            ClassificationRule {
                keywords: vec!["telefon", "mobil", "tele2", "telia", "tre ", "telenor", "comviq"],
                account_code: "5630",
                reason: "Telefon och kommunikation",
            },
            ClassificationRule {
                keywords: vec!["bredband", "internet", "bredbandsleverantör", "fiber"],
                account_code: "5640",
                reason: "Datakommunikation",
            },
            ClassificationRule {
                keywords: vec!["revisor", "revision", "revisionsarvode", "pwc", "kpmg", "deloitte", "ey "],
                account_code: "5910",
                reason: "Revisionsarvode",
            },
            ClassificationRule {
                keywords: vec!["bokföring", "redovisning", "redovisningsbyrå", "bokföringsbyrå", "fortnox"],
                account_code: "5920",
                reason: "Redovisningskonsult",
            },
            ClassificationRule {
                keywords: vec!["advokat", "jurist", "juridik", "rättslig rådgivning"],
                account_code: "5930",
                reason: "Juridisk rådgivning",
            },
            ClassificationRule {
                keywords: vec!["konsult", "konsulttjänst", "uppdrag"],
                account_code: "5940",
                reason: "Övriga konsulttjänster",
            },
            ClassificationRule {
                keywords: vec!["reklam", "annons", "google ads", "facebook ads", "meta ads", "marknadsföring"],
                account_code: "5710",
                reason: "Reklam och annonsering",
            },
            ClassificationRule {
                keywords: vec!["trycksak", "katalog", "visitkort", "broschyr"],
                account_code: "5730",
                reason: "Trycksaker och kataloger",
            },
            ClassificationRule {
                keywords: vec!["hotell", "boende", "övernattning", "airbnb"],
                account_code: "5820",
                reason: "Hotell vid tjänsteresa",
            },
            ClassificationRule {
                keywords: vec!["flyg", "tåg", "resa", "taxi", "sas ", "norwegian", "ryanair", "sl ", "transport"],
                account_code: "5810",
                reason: "Resor vid tjänsteresa",
            },
            ClassificationRule {
                keywords: vec!["lön", "löner", "lönekostnad"],
                account_code: "7010",
                reason: "Löner",
            },
            ClassificationRule {
                keywords: vec!["sociala avgifter", "arbetsgivaravgift"],
                account_code: "7510",
                reason: "Sociala avgifter",
            },
            ClassificationRule {
                keywords: vec!["pension", "tjänstepension", "itp"],
                account_code: "7420",
                reason: "Pensionskostnader",
            },
            ClassificationRule {
                keywords: vec!["utbildning", "kurs", "konferens", "certifiering"],
                account_code: "7610",
                reason: "Utbildning",
            },
            ClassificationRule {
                keywords: vec!["försäkring", "sakförsäkring", "ansvarsförsäkring"],
                account_code: "6310",
                reason: "Försäkringspremier",
            },
            ClassificationRule {
                keywords: vec!["drivmedel", "bensin", "diesel", "tankning"],
                account_code: "5410",
                reason: "Drivmedel",
            },
            ClassificationRule {
                keywords: vec!["ränta", "räntekostnad", "räntebetalning"],
                account_code: "8490",
                reason: "Räntekostnader",
            },
            ClassificationRule {
                keywords: vec!["ränteintäkt", "ränta intäkt"],
                account_code: "8210",
                reason: "Ränteintäkter",
            },
            ClassificationRule {
                keywords: vec!["faktura", "invoice", "försäljning tjänst", "tjänsteuppdrag"],
                account_code: "3110",
                reason: "Tjänsteintäkt",
            },
            ClassificationRule {
                keywords: vec!["varuförsäljning", "produktförsäljning", "sålda varor"],
                account_code: "3010",
                reason: "Varuförsäljning",
            },
            ClassificationRule {
                keywords: vec!["varuinköp", "inköp av varor", "handelsvaror"],
                account_code: "4010",
                reason: "Varuinköp",
            },
            ClassificationRule {
                keywords: vec!["kontor", "kontorsmaterial", "papper", "penna", "post-it"],
                account_code: "6110",
                reason: "Kontorsmaterial",
            },
            ClassificationRule {
                keywords: vec!["prenumeration", "tidskrift", "tidning", "böcker", "litteratur"],
                account_code: "6150",
                reason: "Tidskrifter och facklitteratur",
            },
            ClassificationRule {
                keywords: vec!["licens", "licensavgift", "royalty"],
                account_code: "6510",
                reason: "Licensavgifter och royalties",
            },
            ClassificationRule {
                keywords: vec!["domän", "domain", "ssl", "tls", "certifikat"],
                account_code: "5610",
                reason: "IT-tjänster (domän/certifikat)",
            },
        ];

        Self { chart, rules }
    }

    /// Suggest the best matching account code for a transaction description.
    /// Returns up to `max_suggestions` candidates sorted by confidence descending.
    pub fn classify(
        &self,
        description: &str,
        amount: Option<Decimal>,
        max_suggestions: usize,
    ) -> Vec<AccountSuggestion> {
        let normalized = description.to_lowercase();
        let mut scores: HashMap<String, (f64, String)> = HashMap::new();

        for rule in &self.rules {
            let mut match_count = 0usize;
            let mut keyword_len_sum = 0usize;

            for kw in &rule.keywords {
                if normalized.contains(kw) {
                    match_count += 1;
                    keyword_len_sum += kw.len();
                }
            }

            if match_count > 0 {
                // Score: proportion of description matched + keyword specificity bonus
                let coverage = keyword_len_sum as f64 / normalized.len().max(1) as f64;
                let specificity = 1.0 - (1.0 / rule.keywords.len() as f64).min(0.5);
                let base_score = (coverage * 0.6 + specificity * 0.4).min(1.0);

                // Slight boost when amount is provided and large (more confidence for costly items)
                let amount_factor = amount
                    .map(|a| if a > Decimal::new(10000, 0) { 1.05 } else { 1.0 })
                    .unwrap_or(1.0);

                let score = (base_score * amount_factor).min(1.0);

                scores
                    .entry(rule.account_code.to_string())
                    .and_modify(|(s, _)| {
                        if score > *s {
                            *s = score;
                        }
                    })
                    .or_insert((score, rule.reason.to_string()));
            }
        }

        let mut results: Vec<AccountSuggestion> = scores
            .into_iter()
            .filter_map(|(code, (confidence, reason))| {
                self.chart.get(&code).map(|account| AccountSuggestion {
                    account_code: code,
                    account_name: account.name.clone(),
                    confidence,
                    reason,
                })
            })
            .collect();

        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        results.truncate(max_suggestions);
        results
    }

    /// Return the top suggestion, or None if nothing matches
    pub fn best_match(&self, description: &str, amount: Option<Decimal>) -> Option<AccountSuggestion> {
        self.classify(description, amount, 1).into_iter().next()
    }

    pub fn chart(&self) -> &ChartOfAccounts {
        &self.chart
    }
}
