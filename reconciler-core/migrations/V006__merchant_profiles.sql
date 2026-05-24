-- V006__merchant_profiles.sql
-- Kvittovalvet Merchant Intelligence Layer
-- Seed-data för Sveriges vanligaste merchants med bankaliaser, kvittostöd och kategorier

CREATE TABLE IF NOT EXISTS merchant_profiles (
    id                      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id             text UNIQUE NOT NULL,
    display_name            text NOT NULL,
    category                text NOT NULL,
    org_number              text,
    website                 text,
    receipt_portal          text,
    receipt_email_patterns  text[],
    bank_aliases            text[],
    receipt_support_channels text[],
    has_api_access          boolean DEFAULT false,
    notes                   text,
    country                 text DEFAULT 'SE',
    created_at              timestamptz DEFAULT now(),
    updated_at              timestamptz DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_merchant_category ON merchant_profiles(category);
CREATE INDEX IF NOT EXISTS idx_merchant_aliases  ON merchant_profiles USING GIN(bank_aliases);
CREATE INDEX IF NOT EXISTS idx_merchant_id       ON merchant_profiles(merchant_id);

-- ============================================================
-- DAGLIGVAROR
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'ICA',
    'ICA Gruppen',
    'DAGLIGVAROR',
    '556015-0875',
    'https://www.ica.se',
    'https://www.ica.se/mina-sidor/mina-kvitton/',
    ARRAY['kvitto@ica.se','no-reply@ica.se','noreply@ica.se','receipt@ica.se'],
    ARRAY['ICA','ICA MAXI','ICA KVANTUM','ICA SUPERMARKET','ICA NARA','ICA NÄRA',
          'ICA FOCUS','ICA TO GO','ICA TORG','ICA KVANTUM MALMO','ICA MAXI NACKA',
          'ICA MAXI BARKARBY','ICA MAXI KARLSTAD','ICA SUPERMARKET CITY',
          'ICA NARA VASASTAN','ICA SUPERMARKE','ICA GRUPPEN'],
    ARRAY['app','web','email'],
    false,
    'Digitala kvitton via ICA-appen om Stamkundskort/ICA-kort används vid köp. Historik sparas i appen under "Mina kvitton". Kvittokopia för äldre köp via kundtjänst kundservice@ica.se.'
),
(
    'COOP',
    'Coop Sverige',
    'DAGLIGVAROR',
    '702001-7798',
    'https://www.coop.se',
    'https://www.coop.se/medlemmar/mina-sidor/',
    ARRAY['noreply@coop.se','kvitto@coop.se','no-reply@coop.se'],
    ARRAY['COOP','COOP FORUM','COOP EXTRA','COOP NARA','COOP NÄRA','STORA COOP',
          'COOP KONSUM','COOP CITY','COOP SUPERMARKE','COOP BUTIK',
          'COOP ONLINE','COOP FORUM BARKARBY','COOP EXTRA MOBILIA',
          'KONSUM','COOP KONSUMENT'],
    ARRAY['app','web','email'],
    false,
    'Digitalt kvitto via Coop-appen med Coop-kort. Kvittokopia begärs via Mina sidor på coop.se eller kundtjänst 0771-26 00 00.'
),
(
    'WILLYS',
    'Willys',
    'DAGLIGVAROR',
    '556544-6244',
    'https://www.willys.se',
    'https://www.willys.se/mina-sidor/',
    ARRAY['noreply@willys.se','kvitto@willys.se'],
    ARRAY['WILLYS','WILLYS HEMMA','WILLYS PLUS','WILLYS CITY','WILLY S',
          'WILLYS STOCKHOLM','WILLYS GOTEBORG','WILLYS MALMO','WILLYS DRIVE'],
    ARRAY['app','web','email'],
    false,
    'Del av Axfood-koncernen. Digitalt kvitto via Willys-appen (Triss). Historik 24 månader bakåt. Kundtjänst: kundtjanst@willys.se.'
),
(
    'HEMKOP',
    'Hemköp',
    'DAGLIGVAROR',
    '556018-3215',
    'https://www.hemkop.se',
    'https://www.hemkop.se/mina-sidor/',
    ARRAY['noreply@hemkop.se','kvitto@hemkop.se'],
    ARRAY['HEMKOP','HEM KÖP','HEMKÖP','HEMKOP CITY','HEMKOP CENTRALEN',
          'HEMKOP OSTERMALM','HEMKOP SOLNA','HEMKOP LUND'],
    ARRAY['app','web','email'],
    false,
    'Del av Axfood. Digitalt kvitto via Hemköp-appen. Kundtjänst på hemkop.se/kundservice.'
),
(
    'CITYGROSS',
    'City Gross',
    'DAGLIGVAROR',
    '556021-9415',
    'https://www.citygross.se',
    'https://www.citygross.se/mina-sidor/',
    ARRAY['noreply@citygross.se','kundservice@citygross.se'],
    ARRAY['CITY GROSS','CITYGROSS','CITY GR','CITY GROSS HELSINGBORG',
          'CITY GROSS KRISTIANSTAD','CITY GROSS BORAS','CITY GROSS JONKOPING'],
    ARRAY['app','web','email'],
    false,
    'Fokus på södra Sverige. Digitala kvitton via appen med City Gross-kort. Kundtjänst: 0771-420 420.'
),
(
    'LIDL',
    'Lidl Sverige',
    'DAGLIGVAROR',
    '556453-3011',
    'https://www.lidl.se',
    NULL,
    ARRAY['noreply@lidl.se','receipt@lidl.se','kvitto@lidl.se'],
    ARRAY['LIDL','LIDL SVERIGE','LIDL STOCKHOLM','LIDL GOTEBORG','LIDL MALMO',
          'LIDL SE','LIDL FOOD'],
    ARRAY['app','email'],
    false,
    'Lidl Plus-appen ger digitala kvitton och erbjudanden. Utan app: papperskvitto. Kundtjänst: 0771-818 181.'
),
(
    'STORACOOP',
    'Stora Coop',
    'DAGLIGVAROR',
    '702001-7798',
    'https://www.coop.se/stora-coop/',
    'https://www.coop.se/medlemmar/mina-sidor/',
    ARRAY['noreply@coop.se','kvitto@coop.se'],
    ARRAY['STORA COOP','STORACOOP','COOP STOR','STORA COOP KISTA',
          'STORA COOP HANINGE','STORA COOP SUNDSVALL'],
    ARRAY['app','web','email'],
    false,
    'Samma system som Coop. Digitala kvitton via Coop-appen med Coop Mera-kort.'
),
(
    'AXFOOD',
    'Axfood',
    'DAGLIGVAROR',
    '556542-0824',
    'https://www.axfood.se',
    NULL,
    ARRAY['noreply@axfood.se'],
    ARRAY['AXFOOD','AXFOOD AB'],
    ARRAY['web','email'],
    false,
    'Moderbolag till Willys, Hemköp, Tempo, Snabbgross. Ibland syns "Axfood" på kortutdrag vid online-köp.'
);

-- ============================================================
-- DRIVMEDEL
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'CIRCLEK',
    'Circle K Sverige',
    'DRIVMEDEL',
    '556008-5674',
    'https://www.circlek.se',
    'https://www.circlek.se/mina-sidor/',
    ARRAY['noreply@circlek.se','receipt@circlek.se','no-reply@se.circlek.com'],
    ARRAY['CIRCLE K','CIRCLEK','CIRCLE K SVERIGE','CK','STATOIL','STATOIL FUEL',
          'CIRCLE K NACKA','CIRCLE K BROMMA','CIRCLE K STOCKHOLM',
          'CIRCLEK SE','CIRCLE K PUMP'],
    ARRAY['app','web','email'],
    false,
    'Tidigare Statoil. Circle K Extra-appen ger digitala kvitton och rabatter. Kvitto vid pump via e-post om registrerat.'
),
(
    'OKQ8',
    'OKQ8',
    'DRIVMEDEL',
    '556037-2859',
    'https://www.okq8.se',
    'https://www.okq8.se/mina-sidor/',
    ARRAY['noreply@okq8.se','kvitto@okq8.se','receipt@okq8.se'],
    ARRAY['OKQ8','OK Q8','OKQ8 BENSIN','OKQ8 DRIVE','OKQ8 STOCKHOLM',
          'OKQ8 GOTEBORG','OKQ8 MALMO','OK Q8 SE','Q8','OKQ8 PUMP'],
    ARRAY['app','web','email'],
    false,
    'OKQ8 Duo-kort ger kvittohistorik online. Kontanttankningar kräver e-post-registrering vid pump.'
),
(
    'ST1',
    'ST1',
    'DRIVMEDEL',
    '556669-5553',
    'https://www.st1.se',
    'https://www.st1.se/mina-sidor/',
    ARRAY['noreply@st1.se','kvitto@st1.se','receipt@st1.se'],
    ARRAY['ST1','ST 1','ST1 FUEL','ST1 BENSIN','ST1 SVERIGE','ST1 STOCKHOLM',
          'ST1 GOTEBORG','ST1 DRIVE','SHELL EXPRESS','SHELL'],
    ARRAY['app','web','email'],
    false,
    'ST1 driver även Shell i Sverige. Kvittoapp och e-postkvitto vid pump. Kundtjänst: 020-23 00 00.'
),
(
    'PREEM',
    'Preem',
    'DRIVMEDEL',
    '556072-6685',
    'https://www.preem.se',
    'https://www.preem.se/privat/mitt-preem/',
    ARRAY['noreply@preem.se','receipt@preem.se','kvitto@preem.se'],
    ARRAY['PREEM','PREEM BENSIN','PREEM STATION','PREEM DRIVE','PREEM OIL',
          'PREEM STOCKHOLM','PREEM GOTEBORG','PREEM AB'],
    ARRAY['app','web','email'],
    false,
    'Preem Mastercard/Preem-konto ger full kvittohistorik. Utan kort: e-postkvitto alternativ vid pump.'
),
(
    'INGO',
    'Ingo',
    'DRIVMEDEL',
    '556044-2751',
    'https://www.ingo.se',
    NULL,
    ARRAY['noreply@ingo.se','kvitto@ingo.se'],
    ARRAY['INGO','INGO BENSIN','INGO FUEL','INGO STATION','INGO DRIVE'],
    ARRAY['web','email'],
    false,
    'Lågprisstationer. Begränsad kvittofunktionalitet – papperskvitto primärt. Digital kopia via kundtjänst.'
),
(
    'QSTAR',
    'Q-Star',
    'DRIVMEDEL',
    '556457-1730',
    'https://www.q-star.se',
    NULL,
    ARRAY['noreply@q-star.se','info@q-star.se'],
    ARRAY['Q-STAR','QSTAR','Q STAR','Q-STAR BENSIN','QSTAR FUEL'],
    ARRAY['web','email'],
    false,
    'Franchisekedja av mindre bensinstationer. Kvitto via papper eller e-post vid registrerat konto.'
),
(
    'TESLA_SUPERCHARGING',
    'Tesla Supercharging',
    'DRIVMEDEL',
    NULL,
    'https://www.tesla.com/sv_SE/charging',
    'https://www.tesla.com/sv_SE/account',
    ARRAY['noreply@tesla.com','receipt@tesla.com','billing@tesla.com'],
    ARRAY['TESLA','TESLA SUPERCHARGING','TESLA MOTORS','TESLA INC',
          'TESLA CHARGING','TESLA SC','TESLA SUPER','TSLA CHARGING'],
    ARRAY['app','web','email'],
    true,
    'Kvitto automatiskt skickat till Tesla-kontots e-post efter varje laddning. Fullständig historik i Tesla-appen under "Laddning". API-åtkomst via Tesla Owner API.'
),
(
    'CHARGENODE',
    'ChargeNode',
    'DRIVMEDEL',
    '559038-6042',
    'https://www.chargenode.com',
    'https://app.chargenode.com/',
    ARRAY['noreply@chargenode.com','receipt@chargenode.com','billing@chargenode.com'],
    ARRAY['CHARGENODE','CHARGE NODE','CHARGENODE SE','CN CHARGING',
          'CHARGENODE AB','CHARGENODE LADDNING'],
    ARRAY['app','web','email'],
    true,
    'Nordisk laddnätverk. Kvitto via ChargeNode-appen och e-post. API tillgängligt för företagskunder.'
),
(
    'VATTENFALL_INCHARGE',
    'Vattenfall InCharge',
    'DRIVMEDEL',
    '556036-2316',
    'https://www.vattenfall.se/incharge/',
    'https://incharge.vattenfall.se/',
    ARRAY['noreply@vattenfall.se','incharge@vattenfall.se','receipt@incharge.vattenfall.se'],
    ARRAY['VATTENFALL','VATTENFALL INCHARGE','INCHARGE','IN CHARGE',
          'VATTENFALL LADDNING','VATTENFALL EV','INCHARGE VATTENFALL'],
    ARRAY['app','web','email'],
    true,
    'Vattenfalls laddnätverk för elbilar. Automatisk kvittoutskick per session. Fullständig historik i InCharge-appen.'
),
(
    'MER_CHARGING',
    'Mer (laddnätverk)',
    'DRIVMEDEL',
    '559024-1832',
    'https://www.mer.eco',
    'https://app.mer.eco/',
    ARRAY['noreply@mer.eco','receipt@mer.eco','billing@mer.eco'],
    ARRAY['MER','MER CHARGING','MER ECO','MER LADDNING','MER EV',
          'MER NORDICS','MER SE','MER CHARGE'],
    ARRAY['app','web','email'],
    true,
    'Nordiskt laddnätverk (f.d. Grønn Kontakt/Recharge). Automatiska kvitton per session via appen och e-post.'
);

-- ============================================================
-- BYGG_ELEKTRONIK
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'CLASOHLSON',
    'Clas Ohlson',
    'BYGG_ELEKTRONIK',
    '556035-8231',
    'https://www.clasohlson.com/se/',
    'https://www.clasohlson.com/se/Mitt-konto/',
    ARRAY['noreply@clasohlson.com','kvitto@clasohlson.com','receipt@clasohlson.com'],
    ARRAY['CLAS OHLSON','CLASOHLSON','CLAS O','CLAS OHLSON SE',
          'CLAS OHLSON STOCKHOLM','CLAS OHLSON GOTEBORG','CLAS OHLSON MALMO',
          'CLASOHLSON.COM','CLAS OHLSON WEB'],
    ARRAY['app','web','email'],
    false,
    'Clas Ohlson Club-kortet ger digital kvittohistorik. Online-köp: e-postkvitto automatiskt. Butik utan kort: papper.'
),
(
    'BAUHAUS',
    'Bauhaus Sverige',
    'BYGG_ELEKTRONIK',
    '556212-9234',
    'https://www.bauhaus.se',
    'https://www.bauhaus.se/mina-sidor/',
    ARRAY['noreply@bauhaus.se','kvitto@bauhaus.se','receipt@bauhaus.se'],
    ARRAY['BAUHAUS','BAUHAUS SVERIGE','BAUHAUS BYGG','BAUHAUS STOCKHOLM',
          'BAUHAUS GOTEBORG','BAUHAUS MALMO','BAUHAUS AB','BAUHAUS BARKARBY',
          'BAUHAUS KUNGENS KURVA','BAUHAUS SOLNA'],
    ARRAY['app','web','email'],
    false,
    'Bauhaus Club-kort ger kvittokopia online. Företagskort: full historik. Kundtjänst: 020-700 700.'
),
(
    'BILTEMA',
    'Biltema',
    'BYGG_ELEKTRONIK',
    '556024-1006',
    'https://www.biltema.se',
    'https://www.biltema.se/konto/',
    ARRAY['noreply@biltema.se','kvitto@biltema.se','kundservice@biltema.se'],
    ARRAY['BILTEMA','BILTEMA AB','BILTEMA SVERIGE','BILTEMA STOCKHOLM',
          'BILTEMA GOTEBORG','BILTEMA MALMO','BILTEMA VASTRA','BILTEMA NORTH'],
    ARRAY['web','email'],
    false,
    'Biltema-konto ger kvittohistorik online. Inget lojalitetsprogram med app. E-postkvitto vid onlinebeställning.'
),
(
    'JULA',
    'Jula',
    'BYGG_ELEKTRONIK',
    '556045-9573',
    'https://www.jula.se',
    'https://www.jula.se/konto/',
    ARRAY['noreply@jula.se','kvitto@jula.se','receipt@jula.se'],
    ARRAY['JULA','JULA AB','JULA SVERIGE','JULA STOCKHOLM','JULA GOTEBORG',
          'JULA MALMO','JULA KUNGENS KURVA','JULA BARKARBY'],
    ARRAY['app','web','email'],
    false,
    'Jula Club-kort (gratis) ger digital kvittohistorik i Jula-appen. Utan kort: papperskvitto. 30 dagars bytesrätt.'
),
(
    'BEIJERBYGG',
    'Beijer Bygg',
    'BYGG_ELEKTRONIK',
    '556008-3448',
    'https://www.beijerbygg.se',
    'https://www.beijerbygg.se/mina-sidor/',
    ARRAY['noreply@beijerbygg.se','kvitto@beijerbygg.se','kundservice@beijerbygg.se'],
    ARRAY['BEIJER BYGG','BEIJERBYGG','BEIJER','BEIJER AB','BEIJER BYGGMATERIAL',
          'BEIJER BYGG STOCKHOLM','BEIJER BYGG GOTEBORG'],
    ARRAY['web','email'],
    false,
    'B2B-fokus. Kvittohistorik via Mina sidor. Bygghandelskedja med fokus på proffs. Kundtjänst: 020-280 280.'
),
(
    'BYGGMAX',
    'Byggmax',
    'BYGG_ELEKTRONIK',
    '556656-3093',
    'https://www.byggmax.se',
    'https://www.byggmax.se/min-sida/',
    ARRAY['noreply@byggmax.se','kvitto@byggmax.se','receipt@byggmax.se'],
    ARRAY['BYGGMAX','BYGGMAX AB','BYGGMAX SVERIGE','BYGGMAX STOCKHOLM',
          'BYGGMAX GOTEBORG','BYGGMAX MALMO','BYGGMAX SE'],
    ARRAY['web','email'],
    false,
    'Lågprisbygghandel. Kvitto via Mitt konto på byggmax.se vid registrerat konto. Online-köp: e-postkvitto.'
),
(
    'KRAUTA',
    'K-Rauta',
    'BYGG_ELEKTRONIK',
    '556015-2319',
    'https://www.k-rauta.se',
    'https://www.k-rauta.se/mina-sidor/',
    ARRAY['noreply@k-rauta.se','kvitto@k-rauta.se'],
    ARRAY['K-RAUTA','KRAUTA','K RAUTA','K-RAUTA AB','K-RAUTA SVERIGE',
          'K-RAUTA STOCKHOLM','K-RAUTA GOTEBORG','RAUTA'],
    ARRAY['web','email'],
    false,
    'Del av Kesko-koncernen. Kvittokopia via Mina sidor eller kundtjänst. E-postkvitto vid onlinebeställning.'
),
(
    'HORNBACH',
    'Hornbach',
    'BYGG_ELEKTRONIK',
    '556507-5753',
    'https://www.hornbach.se',
    'https://www.hornbach.se/shop/Mitt-konto/',
    ARRAY['noreply@hornbach.se','kvitto@hornbach.se','receipt@hornbach.se'],
    ARRAY['HORNBACH','HORNBACH AB','HORNBACH SVERIGE','HORNBACH STOCKHOLM',
          'HORNBACH GOTEBORG','HORNBACH BARKARBY','HORNBACH KUNGENS KURVA'],
    ARRAY['web','email'],
    false,
    'Tysk bygghandelskedja. Hornbach Club-kort: digital kvittohistorik. Kundtjänst: 08-584 606 00.'
),
(
    'ELGIGANTEN',
    'Elgiganten',
    'BYGG_ELEKTRONIK',
    '556286-3449',
    'https://www.elgiganten.se',
    'https://www.elgiganten.se/account/',
    ARRAY['noreply@elgiganten.se','kvitto@elgiganten.se','receipt@elgiganten.se','order@elgiganten.se'],
    ARRAY['ELGIGANTEN','ELGIGANTEN AB','ELGIGANTEN SVERIGE','ELGIGANTEN STOCKHOLM',
          'ELGIGANTEN GOTEBORG','ELGIGANTEN MALMO','GIGANTTI','ELKJOP',
          'ELGIGANTEN ONLINE','ELGIGANTEN WEB'],
    ARRAY['app','web','email'],
    false,
    'Del av Elkjøp Nordic (Dixons Carphone). Kvitto i appen eller via e-post. Returfönster 30 dagar. Kundtjänst: 020-800 800.'
),
(
    'POWER',
    'Power',
    'BYGG_ELEKTRONIK',
    '556661-8243',
    'https://www.power.se',
    'https://www.power.se/mitt-konto/',
    ARRAY['noreply@power.se','kvitto@power.se','order@power.se'],
    ARRAY['POWER','POWER SE','POWER SVERIGE','POWER ELEKTRONIK',
          'POWER STOCKHOLM','POWER GOTEBORG','POWER.SE'],
    ARRAY['web','email'],
    false,
    'Nordisk elektronikkedja. E-postkvitto vid onlineköp. Butik: papper eller e-post om registrerat.'
),
(
    'NETONNET',
    'NetOnNet',
    'BYGG_ELEKTRONIK',
    '556520-0558',
    'https://www.netonnet.se',
    'https://www.netonnet.se/art/mina-sidor/',
    ARRAY['noreply@netonnet.se','order@netonnet.se','kvitto@netonnet.se'],
    ARRAY['NETONNET','NET ON NET','NETONNET AB','NETONNET SE',
          'NETONNET LAGER','NETONNET LAGERSHOP'],
    ARRAY['web','email'],
    false,
    'Lagerbutik-koncept. E-postkvitto standard vid onlineköp och i lagerbutik. Kundtjänst: 033-700 12 00.'
),
(
    'WEBHALLEN',
    'Webhallen',
    'BYGG_ELEKTRONIK',
    '556541-3183',
    'https://www.webhallen.com',
    'https://www.webhallen.com/se/user/',
    ARRAY['noreply@webhallen.com','order@webhallen.com','kvitto@webhallen.com'],
    ARRAY['WEBHALLEN','WEBHALLEN AB','WEBHALLEN SE','WEBHALLEN STOCKHOLM',
          'WEBHALLEN GOTEBORG','WEBHALLEN MALMO','WEBHALLEN ONLINE'],
    ARRAY['web','email'],
    false,
    'Fokus på gaming/teknik. Automatiskt e-postkvitto. Fullständig orderhistorik online. Webhallen Plus-prenumeration tillgänglig.'
),
(
    'INET',
    'Inet',
    'BYGG_ELEKTRONIK',
    '556635-7039',
    'https://www.inet.se',
    'https://www.inet.se/konto/',
    ARRAY['noreply@inet.se','order@inet.se','support@inet.se'],
    ARRAY['INET','INET AB','INET SE','INET GOTEBORG','INET STOCKHOLM',
          'INET ONLINE','INETSE'],
    ARRAY['web','email'],
    false,
    'Göteborgsk elektronikhandel, populär bland entusiaster. E-postkvitto automatiskt. Orderhistorik på kontosidan.'
),
(
    'KOMPLETT',
    'Komplett',
    'BYGG_ELEKTRONIK',
    NULL,
    'https://www.komplett.se',
    'https://www.komplett.se/account/',
    ARRAY['noreply@komplett.se','order@komplett.se','kundeservice@komplett.no'],
    ARRAY['KOMPLETT','KOMPLETT SE','KOMPLETT AB','KOMPLETT NORGE',
          'KOMPLETT ONLINE','KOMPLETT.SE'],
    ARRAY['web','email'],
    false,
    'Norsk elektronikjätte med svensk verksamhet. E-postkvitto automatiskt. Kundtjänst: 020-120 99 99.'
),
(
    'MEDIAMARKT',
    'MediaMarkt',
    'BYGG_ELEKTRONIK',
    '556421-6693',
    'https://www.mediamarkt.se',
    'https://www.mediamarkt.se/sv/my-account/',
    ARRAY['noreply@mediamarkt.se','order@mediamarkt.se','receipt@mediamarkt.se'],
    ARRAY['MEDIAMARKT','MEDIA MARKT','MEDIAMARKT AB','MEDIAMARKT SE',
          'MEDIAMARKT STOCKHOLM','MEDIAMARKT GOTEBORG','MEDIAMARKT MALMO',
          'MM ELEKTRONIK','MEDIA MARKET'],
    ARRAY['app','web','email'],
    false,
    'Tysk elektronikkedja. MediaMarkt-appen ger digital kvittohistorik. E-postkvitto på begäran i butik.'
),
(
    'KJELL',
    'Kjell & Company',
    'BYGG_ELEKTRONIK',
    '556400-5773',
    'https://www.kjell.com/se/',
    'https://www.kjell.com/se/mina-sidor/',
    ARRAY['noreply@kjell.com','order@kjell.com','kvitto@kjell.com'],
    ARRAY['KJELL','KJELL AND CO','KJELL & CO','KJELL CO','KJELL COMPANY',
          'KJELL OCH CO','KJELL.COM','KJELL STOCKHOLM','KJELL GOTEBORG'],
    ARRAY['app','web','email'],
    false,
    'Kablar, tillbehör, elektronikkomponenter. Kvitto i Kjell & Company-appen. E-post automatiskt online. 365 dagars öppet köp med kvitto.'
);

-- ============================================================
-- HOTELL_RESOR
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'SCANDIC',
    'Scandic Hotels',
    'HOTELL_RESOR',
    '556703-1702',
    'https://www.scandichotels.se',
    'https://www.scandichotels.se/mina-sidor/',
    ARRAY['noreply@scandichotels.com','receipt@scandichotels.com','folio@scandichotels.com'],
    ARRAY['SCANDIC','SCANDIC HOTELS','SCANDIC AB','SCANDIC STOCKHOLM',
          'SCANDIC GOTEBORG','SCANDIC MALMO','SCANDIC GRAND','SCANDIC HOTEL',
          'SCANDIC VICTORIA','SCANDIC SERGEL'],
    ARRAY['app','web','email'],
    false,
    'Kvitto (folio) skickas automatiskt till e-post vid utcheckning. Scandic Friends-app: full historik. Företagskonto: faktura.'
),
(
    'STRAWBERRY',
    'Strawberry Hotels',
    'HOTELL_RESOR',
    '915524-8600',
    'https://www.strawberry.no',
    'https://www.strawberry.no/mina-sidor/',
    ARRAY['noreply@strawberry.no','receipt@nordicchoice.com','folio@strawberry.no'],
    ARRAY['STRAWBERRY','NORDIC CHOICE','NORDICCHOICE','CLARION','QUALITY HOTEL',
          'COMFORT HOTEL','CLARION HOTEL','QUALITY INN','STRAWBERRY HOTELS',
          'NORDIC CHOICE HOTELS'],
    ARRAY['app','web','email'],
    false,
    'Tidigare Nordic Choice Hotels. Inkluderar Clarion, Quality Hotel, Comfort Hotel. Automatiskt e-postkvitto vid utcheckning.'
),
(
    'ELITE',
    'Elite Hotels',
    'HOTELL_RESOR',
    '556469-4997',
    'https://www.elite.se',
    'https://www.elite.se/mina-sidor/',
    ARRAY['noreply@elite.se','folio@elite.se','receipt@elite.se'],
    ARRAY['ELITE HOTELS','ELITE HOTEL','ELITE AB','ELITE STOCKHOLM',
          'ELITE PALACE','ELITE ADLON','ELITE PLAZA'],
    ARRAY['web','email'],
    false,
    'Svenska lyxhotell. Folio-kvitto till e-post vid utcheckning. Företag: månadsvis faktura.'
),
(
    'BESTWESTERN',
    'Best Western',
    'HOTELL_RESOR',
    NULL,
    'https://www.bestwestern.se',
    'https://www.bestwestern.com/en_US/account.html',
    ARRAY['noreply@bestwestern.com','receipt@bestwestern.com'],
    ARRAY['BEST WESTERN','BESTWESTERN','BW HOTEL','BEST WESTERN PLUS',
          'BEST WESTERN PREMIER','BWH HOTEL'],
    ARRAY['web','email'],
    false,
    'Globalt franchisekedjnätverk. Automatiskt kvitto via e-post. Best Western Rewards: full historik.'
),
(
    'RADISSONBLU',
    'Radisson Blu',
    'HOTELL_RESOR',
    NULL,
    'https://www.radissonhotels.com/sv-se/',
    'https://www.radissonhotels.com/sv-se/mitt-konto/',
    ARRAY['noreply@radissonhotels.com','receipt@radissonhotels.com','folio@radisson.com'],
    ARRAY['RADISSON BLU','RADISSONBLU','RADISSON','RADISSON HOTEL',
          'RADISSON BLU STOCKHOLM','RADISSON BLU GOTEBORG','RADISSON COLLECTION'],
    ARRAY['web','email'],
    false,
    'Del av Radisson Hotel Group. E-postkvitto automatiskt. Radisson Rewards-program.'
),
(
    'SJ',
    'SJ AB',
    'HOTELL_RESOR',
    '556388-3077',
    'https://www.sj.se',
    'https://www.sj.se/sv/hem/mitt-sj.html',
    ARRAY['noreply@sj.se','receipt@sj.se','biljett@sj.se','kvitto@sj.se'],
    ARRAY['SJ','SJ AB','SJ TÅGET','SJ RAIL','SJ SE','SJ TICKET',
          'SJ BILJETT','SJ RESEBOLAG','SJ SNABBTAG','SJ X2000'],
    ARRAY['app','web','email'],
    true,
    'Statligt järnvägsbolag. Digitala biljetter och kvitton i SJ-appen. Mitt SJ online: full historik. API för företagsintegrationer.'
),
(
    'MTREXPRESS',
    'MTR Express',
    'HOTELL_RESOR',
    '556951-8702',
    'https://www.mtrexpress.se',
    'https://www.mtrexpress.se/mitt-konto/',
    ARRAY['noreply@mtrexpress.se','receipt@mtrexpress.se','biljett@mtrexpress.se'],
    ARRAY['MTR EXPRESS','MTREXPRESS','MTR','MTR EXPRESS AB','MTR TRAIN',
          'MTR TÅGET','MTR BILJETT'],
    ARRAY['app','web','email'],
    false,
    'Privat höghastighetståg Stockholm-Göteborg. E-postkvitto automatiskt. App med historik.'
),
(
    'FLIXBUS',
    'FlixBus',
    'HOTELL_RESOR',
    NULL,
    'https://www.flixbus.se',
    'https://www.flixbus.se/konto',
    ARRAY['noreply@flixbus.com','booking@flixbus.com','receipt@flixbus.com'],
    ARRAY['FLIXBUS','FLIX BUS','FLIXBUS SE','FLIXBUS NORDIC','FLIX',
          'FLIXBUS TICKET','FLIXBUS AB'],
    ARRAY['app','web','email'],
    true,
    'Europeisk bussoperatör. E-postkvitto automatiskt. FlixBus-appen: full resehistorik. API för återförsäljare.'
),
(
    'SAS',
    'SAS (Scandinavian Airlines)',
    'HOTELL_RESOR',
    '556102-4223',
    'https://www.sas.se',
    'https://www.sas.se/sv/mitt-sas/',
    ARRAY['noreply@sas.se','receipt@sas.se','booking@sas.se','eurobonus@sas.se'],
    ARRAY['SAS','SCANDINAVIAN AIRLINES','SAS AB','SAS SE','SAS FLIGHT',
          'SCANDINAVIAN AIR','SAS BILJETT','SAS FLYGBOLAG','SAS TICKET'],
    ARRAY['app','web','email'],
    true,
    'Nordisk flygoperatör. E-postkvitto automatiskt. EuroBonus-app: full resehistorik. Månadsutdrag för företag.'
),
(
    'NORWEGIAN',
    'Norwegian Air Shuttle',
    'HOTELL_RESOR',
    NULL,
    'https://www.norwegian.com/se/',
    'https://www.norwegian.com/se/mitt-norwegian/',
    ARRAY['noreply@norwegian.com','receipt@norwegian.com','booking@norwegian.com'],
    ARRAY['NORWEGIAN','NORWEGIAN AIR','NORWEGIAN SHUTTLE','DY','NORWEGIAN SE',
          'NORWEGIAN FLIGHT','NORWEGIAN BILJETT'],
    ARRAY['app','web','email'],
    false,
    'Norskt lågprisflygbolag. E-postkvitto automatiskt. Norwegian Reward-program med historik.'
),
(
    'RYANAIR',
    'Ryanair',
    'HOTELL_RESOR',
    NULL,
    'https://www.ryanair.com/se/sv/',
    'https://www.ryanair.com/se/sv/useful-info/my-ryanair/',
    ARRAY['noreply@ryanair.com','receipt@ryanair.com','donotreply@ryanair.com'],
    ARRAY['RYANAIR','RYAN AIR','RYANAIR FR','RYANAIR DAC','RYANAIR BILJETT',
          'RYANAIR FLIGHT','FR RYANAIR'],
    ARRAY['web','email'],
    false,
    'Irländskt lågprisflygbolag. E-postkvitto automatiskt. Mina resor på ryanair.com.'
),
(
    'BOOKINGCOM',
    'Booking.com',
    'HOTELL_RESOR',
    NULL,
    'https://www.booking.com',
    'https://account.booking.com/',
    ARRAY['noreply@booking.com','confirmation@booking.com','receipts@booking.com'],
    ARRAY['BOOKING.COM','BOOKING COM','BOOKING','BOOKINGCOM','BOOKING BV',
          'BOOKING HOLDINGS','PRICELINE'],
    ARRAY['web','email'],
    true,
    'Global hotellbokningsplattform. Kvitto/faktura via Mina bokningar på booking.com. API för partners.'
),
(
    'AIRBNB',
    'Airbnb',
    'HOTELL_RESOR',
    NULL,
    'https://www.airbnb.se',
    'https://www.airbnb.se/account-settings/payments/',
    ARRAY['noreply@airbnb.com','automated@airbnb.com','receipts@airbnb.com'],
    ARRAY['AIRBNB','AIR BNB','AIRBNB INC','AIRBNB IRELAND','AIRBNB SE',
          'AIRBNB PAYMENT','AIRBNB*'],
    ARRAY['web','email'],
    true,
    'Korttidsbokning. Automatiskt e-postkvitto. Fullständig transaktionshistorik i kontoinställningar. API för värdar/partners.'
),
(
    'HOTELSCOM',
    'Hotels.com',
    'HOTELL_RESOR',
    NULL,
    'https://sv.hotels.com',
    'https://se.hotels.com/account/',
    ARRAY['noreply@hotels.com','receipt@hotels.com','booking@hotels.com'],
    ARRAY['HOTELS.COM','HOTELSCOM','HOTELS COM','EXPEDIA','HOTELS.COM SE',
          'HOTELS INC'],
    ARRAY['web','email'],
    false,
    'Del av Expedia Group. E-postkvitto automatiskt. Hotels.com Rewards: historik.'
),
(
    'HERTZ',
    'Hertz',
    'HOTELL_RESOR',
    NULL,
    'https://www.hertz.se',
    'https://www.hertz.se/rentacar/member/',
    ARRAY['noreply@hertz.com','receipt@hertz.com','rental@hertz.com'],
    ARRAY['HERTZ','HERTZ SE','HERTZ RENTAL','HERTZ CAR','HERTZ SWEDEN',
          'HERTZ BILUTHYRNING','HERTZ STATION'],
    ARRAY['web','email'],
    false,
    'Global biluthyrning. E-postkvitto automatiskt vid återlämning. Gold Plus Rewards: historik online.'
),
(
    'SIXT',
    'Sixt',
    'HOTELL_RESOR',
    NULL,
    'https://www.sixt.se',
    'https://www.sixt.se/mina-sidor/',
    ARRAY['noreply@sixt.com','receipt@sixt.com','rental@sixt.com'],
    ARRAY['SIXT','SIXT SE','SIXT RENTAL','SIXT CAR','SIXT SWEDEN',
          'SIXT BILUTHYRNING','SIXT AB'],
    ARRAY['web','email'],
    false,
    'Tysk biluthyrning med premium-fokus. E-postkvitto automatiskt. Sixt-appen: resehistorik.'
),
(
    'AVIS',
    'Avis',
    'HOTELL_RESOR',
    NULL,
    'https://www.avis.se',
    'https://www.avis.se/en/car-rental/account/',
    ARRAY['noreply@avis.com','receipt@avis.com','rental@avis.com'],
    ARRAY['AVIS','AVIS SE','AVIS CAR','AVIS RENTAL','AVIS SWEDEN',
          'AVIS BILUTHYRNING','AVIS BUDGET'],
    ARRAY['web','email'],
    false,
    'Del av Avis Budget Group. E-postkvitto automatiskt. Avis Preferred: historik online.'
),
(
    'EUROPCAR',
    'Europcar',
    'HOTELL_RESOR',
    NULL,
    'https://www.europcar.se',
    'https://www.europcar.com/en-gb/account/',
    ARRAY['noreply@europcar.com','receipt@europcar.com'],
    ARRAY['EUROPCAR','EUROPCAR SE','EUROPCAR RENTAL','EUROPCAR SWEDEN',
          'EUROPCAR BILUTHYRNING','EUROPCAR AB'],
    ARRAY['web','email'],
    false,
    'Europeisk biluthyrning. E-postkvitto vid återlämning. Europcar Club: historik.'
);

-- ============================================================
-- RESTAURANG
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'MCDONALDS',
    'McDonald''s Sverige',
    'RESTAURANG',
    '556015-3541',
    'https://www.mcdonalds.com/se/sv-se.html',
    NULL,
    ARRAY['noreply@mcdonalds.com','receipt@mcdonalds.com'],
    ARRAY['MCDONALDS','MC DONALDS','MCDONALD''S','MCD','MCDO','MC D',
          'MCDONALDS SE','MCDONALDS SVERIGE','MCDONALDS STOCKHOLM',
          'MCDONALDS GOTEBORG','MCF','GOLDEN ARCHES'],
    ARRAY['app'],
    false,
    'McDonald''s-appen ger digitala kvitton och erbjudanden. Utan app: papperskvitto. Kundtjänst via mcdonalds.se.'
),
(
    'MAXBURGERS',
    'Max Burgers',
    'RESTAURANG',
    '556020-4219',
    'https://www.max.se',
    'https://www.max.se/mitt-konto/',
    ARRAY['noreply@max.se','kvitto@max.se','receipt@max.se'],
    ARRAY['MAX','MAX BURGERS','MAXBURGERS','MAX HAMBURGARE','MAX AB',
          'MAX BURGER','MAX FAST FOOD','MAX SVERIGE','MAX GOTEBORG',
          'MAX STOCKHOLM','MAX MALMO'],
    ARRAY['app','web','email'],
    false,
    'Svensk hamburgerkedja. Max-appen (MAX Plus) ger digitala kvitton. Kundtjänst: info@max.se.'
),
(
    'ESPRESSOHOUSE',
    'Espresso House',
    'RESTAURANG',
    '556583-7428',
    'https://www.espressohouse.com/se/',
    NULL,
    ARRAY['noreply@espressohouse.com','receipt@espressohouse.com'],
    ARRAY['ESPRESSO HOUSE','ESPRESSOHOUSE','EH','ESPRESSO HOUSE SE',
          'ESPRESSO HOUSE AB','ESPRESSO HSE','ESPRESSO H'],
    ARRAY['app'],
    false,
    'Nordisk kaffekedja. Espresso House-appen (Stars-programmet): digitala kvitton och poäng. Utan app: papper.'
),
(
    'WAYNESCOFFEE',
    'Wayne''s Coffee',
    'RESTAURANG',
    '556516-0497',
    'https://www.waynescoffee.se',
    NULL,
    ARRAY['noreply@waynescoffee.se','kvitto@waynescoffee.se'],
    ARRAY['WAYNES COFFEE','WAYNE''S COFFEE','WAYNES','WAYNES COFFEE SE',
          'WAYNE COFFEE','WANYES COFFEE'],
    ARRAY['app'],
    false,
    'Svensk kaffekedja. Wayne''s Coffee-appen: lojalitetspoäng och digitala kvitton. Utan app: papper.'
),
(
    'BURGERKING',
    'Burger King Sverige',
    'RESTAURANG',
    '556019-7889',
    'https://www.burgerking.se',
    NULL,
    ARRAY['noreply@burgerking.se','receipt@burgerking.com'],
    ARRAY['BURGER KING','BURGERKING','BK','BURGER KING SE','BURGER KING AB',
          'BURGER KING SVERIGE','BK SWEDEN'],
    ARRAY['app'],
    false,
    'Burger King Royal Perks-appen: digitala kvitton och belöningar. Utan app: papperskvitto.'
),
(
    'PIZZAHUT',
    'Pizza Hut Sverige',
    'RESTAURANG',
    NULL,
    'https://www.pizzahut.se',
    'https://www.pizzahut.se/mitt-konto/',
    ARRAY['noreply@pizzahut.se','receipt@pizzahut.com','order@pizzahut.se'],
    ARRAY['PIZZA HUT','PIZZAHUT','PIZZA HUT SE','PIZZAHUT SVERIGE',
          'PH PIZZA','PIZZA HUT DELIVERY'],
    ARRAY['web','email'],
    false,
    'Internationell pizzakedja med begränsad svensk närvaro. E-postkvitto vid onlinebeställning.'
),
(
    'SUBWAY',
    'Subway',
    'RESTAURANG',
    NULL,
    'https://www.subway.com/sv-SE',
    NULL,
    ARRAY['noreply@subway.com','receipt@subway.com'],
    ARRAY['SUBWAY','SUBWAY SE','SUBWAY SANDWICH','SUBWAY SVERIGE',
          'SUBWAY RESTAURANT','SUB'],
    ARRAY['app'],
    false,
    'Globalt sandwichfranchise. Subway-appen: digitala kvitton och rabatter. Utan app: papper.'
),
(
    'OLEARYS',
    'O''Learys',
    'RESTAURANG',
    '556295-1434',
    'https://www.olearys.se',
    NULL,
    ARRAY['noreply@olearys.se','receipt@olearys.se'],
    ARRAY['O''LEARYS','OLEARYS','O LEARYS','O''LEARY''S','OLEARYS SE',
          'OLEARYS STOCKHOLM','OLEARYS GOTEBORG'],
    ARRAY['web','email'],
    false,
    'Sportbar/restaurangkedja. Kvitto primärt via papper. Digital kopia möjlig via kundtjänst.'
),
(
    'SUSHIYAMA',
    'Sushi Yama',
    'RESTAURANG',
    '559052-0442',
    'https://www.sushiyama.se',
    NULL,
    ARRAY['noreply@sushiyama.se','order@sushiyama.se'],
    ARRAY['SUSHI YAMA','SUSHIYAMA','SUSHI YAMA SE','YAMA SUSHI',
          'SUSHI YAMA STOCKHOLM','SUSHI YAMA GOTEBORG'],
    ARRAY['app','web','email'],
    false,
    'Svensk sushikedja med take-away-fokus. E-postkvitto via app-beställning. Kundtjänst: info@sushiyama.se.'
);

-- ============================================================
-- TRANSPORT
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'BOLT',
    'Bolt',
    'TRANSPORT',
    NULL,
    'https://bolt.eu/sv-se/',
    'https://bolt.eu/sv-se/account/',
    ARRAY['noreply@bolt.eu','receipt@bolt.eu','receipts@bolt.eu','support@bolt.eu'],
    ARRAY['BOLT','BOLT EU','BOLT TRANSPORT','BOLT RIDES','BOLT TAXI',
          'BOLT*','BOLT SE','TAXIFY','BOLT APP'],
    ARRAY['app','web','email'],
    true,
    'Estländsk ride-hailing. Automatiskt e-postkvitto efter varje resa. Full historik i Bolt-appen. API för företagskonton.'
),
(
    'UBER',
    'Uber',
    'TRANSPORT',
    NULL,
    'https://www.uber.com/se/sv/',
    'https://riders.uber.com/',
    ARRAY['noreply@uber.com','receipts@uber.com','no-reply@uber.com'],
    ARRAY['UBER','UBER BV','UBER TAXI','UBER RIDES','UBER*','UBER SE',
          'UBER TECHNOLOGIES','UBER TRIP','UBER EATS'],
    ARRAY['app','web','email'],
    true,
    'Globalt ride-hailing. Automatiskt e-postkvitto. Full resehistorik i Uber-appen och på riders.uber.com. API för företag.'
),
(
    'TAXISTOCKHOLM',
    'Taxi Stockholm',
    'TRANSPORT',
    '556044-7347',
    'https://www.taxistockholm.se',
    'https://www.taxistockholm.se/foretag/',
    ARRAY['noreply@taxistockholm.se','kvitto@taxistockholm.se'],
    ARRAY['TAXI STOCKHOLM','TAXISTOCKHOLM','TAXI STHLM','TAXI STOCKHOLM AB',
          'TAXI 020','020-TAXI'],
    ARRAY['app','web','email'],
    false,
    'Stockholms största taxibolag. Kvitto via Taxi Stockholm-appen eller e-post. Företagskonto: månadsvis faktura.'
),
(
    'TAXIKURIR',
    'Taxi Kurir',
    'TRANSPORT',
    '556053-1611',
    'https://www.taxikurir.se',
    'https://www.taxikurir.se/foretag/',
    ARRAY['noreply@taxikurir.se','kvitto@taxikurir.se'],
    ARRAY['TAXI KURIR','TAXIKURIR','TAXI KURIR AB','TAXIKURIR STOCKHOLM',
          'KURIR TAXI'],
    ARRAY['app','web','email'],
    false,
    'Taxibolag med rikstäckning. Kvitto via app eller e-post. Företagskunder: faktura.'
),
(
    'SL',
    'SL (Storstockholms Lokaltrafik)',
    'TRANSPORT',
    '556013-0683',
    'https://sl.se',
    'https://sl.se/resenar/app-och-digitala-tjanster/',
    ARRAY['noreply@sl.se','kvitto@sl.se','sl@sl.se'],
    ARRAY['SL','SL AB','STORSTOCKHOLMS LOKALTRAFIK','SL TRAFIK',
          'SL STOCKHOLM','SL BILJETT','SL KORT','SL ACCESS'],
    ARRAY['app','web'],
    true,
    'Kollektivtrafik i Storstockholmsregionen. SL-appen: kvitton för app-köp. SL Access-korthistorik via sl.se. API för reseplanerare.'
),
(
    'VASTTRAFIK',
    'Västtrafik',
    'TRANSPORT',
    '556558-5873',
    'https://www.vasttrafik.se',
    'https://www.vasttrafik.se/kundservice/',
    ARRAY['noreply@vasttrafik.se','kvitto@vasttrafik.se'],
    ARRAY['VASTTRAFIK','VÄSTTRAFIK','VASTTRAFIK AB','VASTTRAFIK SE',
          'VT GOTEBORG','VÄSTTRAFIK GOTEBORG'],
    ARRAY['app','web'],
    true,
    'Kollektivtrafik i Västra Götalandsregionen. Västtrafik To Go-appen: digitala biljetter och kvitton. API för partners.'
),
(
    'SKANETRAFIKEN',
    'Skånetrafiken',
    'TRANSPORT',
    '556337-4191',
    'https://www.skanetrafiken.se',
    'https://www.skanetrafiken.se/kundservice/',
    ARRAY['noreply@skanetrafiken.se','kvitto@skanetrafiken.se'],
    ARRAY['SKANETRAFIKEN','SKÅNETRAFIKEN','SKANE TRAFIKEN','SKANETRAFIKEN AB',
          'REGIONTRAFIKEN SKANE'],
    ARRAY['app','web'],
    true,
    'Kollektivtrafik i Skåne. Skånetrafiken-appen: digitala biljetter. Jojo-kort: historik online.'
),
(
    'SUNFLEET',
    'Sunfleet/M (bilpool)',
    'TRANSPORT',
    '556700-9765',
    'https://www.sunfleet.com',
    'https://www.sunfleet.com/mina-sidor/',
    ARRAY['noreply@sunfleet.com','kvitto@sunfleet.com','receipt@sunfleet.com'],
    ARRAY['SUNFLEET','SUNFLEET AB','M BILPOOL','M MOBILITY','SUNFLEET BILPOOL',
          'VOLVO SUNFLEET','M CARSHARING'],
    ARRAY['web','email'],
    false,
    'Volvos bilpool-tjänst. E-postkvitto automatiskt per bokning. Full historik i Sunfleet-portalen.'
);

-- ============================================================
-- TELEKOM
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'TELIA',
    'Telia',
    'TELEKOM',
    '556103-4249',
    'https://www.telia.se',
    'https://www.telia.se/privat/mitt-telia/',
    ARRAY['noreply@telia.se','faktura@telia.se','kvitto@telia.se','invoice@telia.se'],
    ARRAY['TELIA','TELIA SE','TELIA AB','TELIA COMPANY','TELIA SVERIGE',
          'TELIASONERA','TELIA MOBIL','TELIA BREDBAND','TELIA TV'],
    ARRAY['app','web','email'],
    true,
    'Telekommunikation, mobil, bredband. Faktura/kvitto via Mitt Telia och e-post. Företag: API-integration möjlig.'
),
(
    'TELE2',
    'Tele2',
    'TELEKOM',
    '556274-7826',
    'https://www.tele2.se',
    'https://www.tele2.se/mina-sidor/',
    ARRAY['noreply@tele2.se','faktura@tele2.se','invoice@tele2.se'],
    ARRAY['TELE2','TELE 2','TELE2 AB','TELE2 SE','TELE2 MOBIL',
          'TELE2 BREDBAND','TELE2 FORETAG'],
    ARRAY['web','email'],
    false,
    'Mobil och bredband. Faktura via Mina sidor och e-post. Pappersfaktura möjlig mot avgift.'
),
(
    'COMVIQ',
    'Comviq',
    'TELEKOM',
    '556274-7826',
    'https://www.comviq.se',
    'https://www.comviq.se/mina-sidor/',
    ARRAY['noreply@comviq.se','faktura@comviq.se'],
    ARRAY['COMVIQ','COMVIQ SE','COMVIQ AB','COMVIQ MOBIL','COMVIQ TELE2'],
    ARRAY['web','email'],
    false,
    'Lågprisvarumärke ägt av Tele2. Faktura via Mina sidor. Kontantkort: papperskvitto.'
),
(
    'TRE',
    'Tre (Hi3G)',
    'TELEKOM',
    '556534-1109',
    'https://www.tre.se',
    'https://www.tre.se/mina-sidor/',
    ARRAY['noreply@tre.se','faktura@tre.se','invoice@tre.se'],
    ARRAY['TRE','3 SVERIGE','HI3G','TRE MOBIL','TRE SE','3SE',
          '3 MOBIL','TRE BREDBAND','TRE AB'],
    ARRAY['app','web','email'],
    false,
    'Del av CK Hutchison. Faktura via Mina sidor och e-post.'
),
(
    'TELENOR',
    'Telenor Sverige',
    'TELEKOM',
    '556421-0250',
    'https://www.telenor.se',
    'https://www.telenor.se/mina-sidor/',
    ARRAY['noreply@telenor.se','faktura@telenor.se','invoice@telenor.se'],
    ARRAY['TELENOR','TELENOR SE','TELENOR AB','TELENOR SVERIGE',
          'TELENOR MOBIL','DJUICE','TELENOR BREDBAND'],
    ARRAY['app','web','email'],
    false,
    'Norskt telekombolag med stor svensk närvaro. Faktura via Mina sidor och e-post.'
),
(
    'HALLON',
    'Hallon',
    'TELEKOM',
    '556534-1109',
    'https://www.hallon.se',
    'https://www.hallon.se/mina-sidor/',
    ARRAY['noreply@hallon.se','faktura@hallon.se'],
    ARRAY['HALLON','HALLON SE','HALLON MOBIL','HALLON AB','HALLON TRE'],
    ARRAY['web','email'],
    false,
    'Lågprisvarumärke ägt av Tre. Digital faktura standard. Ingen pappersfaktura.'
);

-- ============================================================
-- STREAMING
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'NETFLIX',
    'Netflix',
    'STREAMING',
    NULL,
    'https://www.netflix.com/se/',
    'https://www.netflix.com/YourAccount',
    ARRAY['info@mailer.netflix.com','noreply@netflix.com','payment@netflix.com'],
    ARRAY['NETFLIX','NETFLIX SE','NETFLIX INC','NETFLIX.COM','NETFLIX SUBSCRIPTION',
          'NETFLIX*','NF*NETFLIX'],
    ARRAY['web','email'],
    true,
    'Strömning. Månadsvis kvitto/faktura skickas till e-post. Full betalningshistorik i kontoinstallningar.'
),
(
    'SPOTIFY',
    'Spotify',
    'STREAMING',
    '556703-7485',
    'https://www.spotify.com/se/',
    'https://www.spotify.com/se/account/subscription/receipts/',
    ARRAY['noreply@spotify.com','receipts@spotify.com','no-reply@spotify.com'],
    ARRAY['SPOTIFY','SPOTIFY AB','SPOTIFY SE','SPOTIFY PREMIUM',
          'SPOTIFY SUBSCRIPTION','SPOTIFY*','SP*SPOTIFY'],
    ARRAY['web','email'],
    true,
    'Svensk musikströmning. Kvitto skickas till e-post månadsvis. Kontoinstallningar: ladda ner alla kvitton. API för partners.'
),
(
    'HBOMAX',
    'Max (HBO Max)',
    'STREAMING',
    NULL,
    'https://www.max.com/se/sv',
    'https://www.max.com/se/sv/account',
    ARRAY['noreply@max.com','receipts@max.com','billing@hbomax.com'],
    ARRAY['HBO MAX','HBOMAX','HBO','MAX STREAMING','MAX TV','HBO NORDIC',
          'HBO*','WARNERMEDIA','MAX.COM'],
    ARRAY['web','email'],
    false,
    'Warner Bros Discovery-strömning (tidigare HBO Nordic). Månadskvitto via e-post.'
),
(
    'DISNEYPLUS',
    'Disney+',
    'STREAMING',
    NULL,
    'https://www.disneyplus.com/sv-se',
    'https://www.disneyplus.com/account',
    ARRAY['noreply@mail.disneyplus.com','receipts@disneyplus.com'],
    ARRAY['DISNEY+','DISNEY PLUS','DISNEYPLUS','DISNEY+ SE',
          'DISNEY*','THE WALT DISNEY','DISNEY STREAMING'],
    ARRAY['web','email'],
    false,
    'Disneys strömtjänst. E-postkvitto månadsvis. Fakturahistorik i kontoinstitallningar.'
),
(
    'VIAPLAY',
    'Viaplay',
    'STREAMING',
    '556442-7564',
    'https://viaplay.se',
    'https://viaplay.se/account/',
    ARRAY['noreply@viaplay.com','receipts@viaplay.com','billing@viaplay.se'],
    ARRAY['VIAPLAY','VIAPLAY SE','VIASAT','VIAPLAY AB','NENT GROUP',
          'NORDIC ENTERTAINMENT','VIAPLAY*','V* VIAPLAY'],
    ARRAY['web','email'],
    false,
    'Nordisk strömtjänst. E-postkvitto månadsvis. Fakturahistorik i Mitt konto på viaplay.se.'
),
(
    'APPLETVPLUS',
    'Apple TV+',
    'STREAMING',
    NULL,
    'https://tv.apple.com/se',
    'https://reportaproblem.apple.com/',
    ARRAY['noreply@email.apple.com','no_reply@email.apple.com','receipt@apple.com'],
    ARRAY['APPLE TV','APPLE TV PLUS','APPLE TV+','APPLE.COM/BILL',
          'APPLE*','APPLE SERVICES','APPLE SUBSCRIPTION'],
    ARRAY['web','email'],
    false,
    'Del av Apples prenumerationstjänster. Kvitto via Apple-ID e-post månadsvis. Historik i Inköpshistorik på Apple-kontot.'
),
(
    'AMAZONPRIME',
    'Amazon Prime',
    'STREAMING',
    NULL,
    'https://www.amazon.se',
    'https://www.amazon.se/gp/css/order-history/',
    ARRAY['noreply@amazon.se','auto-confirm@amazon.se','receipts@amazon.co.uk'],
    ARRAY['AMAZON PRIME','AMAZONPRIME','AMAZON*PRIME','AMAZON DIGITAL',
          'AMAZON EU','AMAZON SERVICES','AMAZON.SE','AMZN'],
    ARRAY['web','email'],
    true,
    'Amazons prenumerationstjänst (video+shopping-förmåner). Kvitto till e-post månadsvis. Orderhistorik på amazon.se. API för säljare/partners.'
),
(
    'YOUTUBEPREMIUM',
    'YouTube Premium',
    'STREAMING',
    NULL,
    'https://www.youtube.com/premium',
    'https://myaccount.google.com/payments-and-subscriptions',
    ARRAY['noreply@accounts.google.com','receipts@google.com'],
    ARRAY['YOUTUBE PREMIUM','YOUTUBE','GOOGLE*YOUTUBE','YOUTUBE*',
          'YT PREMIUM','GOOGLE YOUTUBE','YOUTUBEPREMIUM'],
    ARRAY['web','email'],
    false,
    'Googles premiumtjänst för YouTube. Kvitto till e-post månadsvis. Betalningshistorik via Google Payments-kontot.'
);

-- ============================================================
-- KONTOR
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'STAPLES',
    'Staples',
    'KONTOR',
    '556419-1706',
    'https://www.staples.se',
    'https://www.staples.se/mina-sidor/',
    ARRAY['noreply@staples.se','order@staples.se','receipt@staples.se'],
    ARRAY['STAPLES','STAPLES SE','STAPLES AB','STAPLES SVERIGE',
          'STAPLES OFFICE','STAPLES KONTORSMATERIAL'],
    ARRAY['web','email'],
    false,
    'Kontorsvaror och IT-tillbehör. E-postkvitto standard. Företagskonton: fakturahantering.'
),
(
    'OFFICEDEPOT',
    'Office Depot',
    'KONTOR',
    '556097-6895',
    'https://www.officedepot.se',
    'https://www.officedepot.se/account/',
    ARRAY['noreply@officedepot.se','order@officedepot.se','receipt@officedepot.se'],
    ARRAY['OFFICE DEPOT','OFFICEDEPOT','OFFICE DEPOT SE','OFFICE DEPOT AB',
          'VIKING DIRECT'],
    ARRAY['web','email'],
    false,
    'Del av Office Depot Europe. E-postkvitto automatiskt. Fullständig orderhistorik online.'
),
(
    'KONTORSGIGANTEN',
    'Kontorsgiganten',
    'KONTOR',
    '556645-4671',
    'https://www.kontorsgiganten.se',
    'https://www.kontorsgiganten.se/konto/',
    ARRAY['noreply@kontorsgiganten.se','order@kontorsgiganten.se'],
    ARRAY['KONTORSGIGANTEN','KONTOR GIGANTEN','KONTORSGIGANTEN SE',
          'KONTORSGIGANTEN AB'],
    ARRAY['web','email'],
    false,
    'Kontorsmöbler och -utrustning. E-postkvitto på beställningar. Företagskunder: fakturaalternativ.'
),
(
    'IKEA',
    'IKEA',
    'KONTOR',
    '556084-9806',
    'https://www.ikea.com/se/sv/',
    'https://www.ikea.com/se/sv/customer-service/orders-deliveries/',
    ARRAY['noreply@ikea.com','kvitto@ikea.com','receipt@ikea.com','order@ikea.com'],
    ARRAY['IKEA','IKEA AB','IKEA SVERIGE','IKEA SE','IKEA STOCKHOLM',
          'IKEA GOTEBORG','IKEA MALMO','IKEA KUNGENS KURVA','IKEA BARKARBY',
          'IKEA BÄCKEBOL','IKEA KÅLLERED','IKEA ONLINE'],
    ARRAY['app','web','email'],
    false,
    'Möbler och heminredning. IKEA Family-kort: kvittohistorik. App: digitalt kvitto i butik. Online: e-postkvitto automatiskt.'
),
(
    'ADOBE',
    'Adobe',
    'KONTOR',
    NULL,
    'https://www.adobe.com/se/',
    'https://account.adobe.com/',
    ARRAY['noreply@adobe.com','receipts@adobe.com','mail@mail.adobe.com'],
    ARRAY['ADOBE','ADOBE INC','ADOBE*','ADOBE SYSTEMS','ADOBE SE',
          'ADOBE SUBSCRIPTION','ADOBE CREATIVE CLOUD','ADOBE CC'],
    ARRAY['web','email'],
    true,
    'Creative Cloud-prenumeration. Månadskvitto till e-post. Full betalningshistorik på account.adobe.com. API för partners.'
),
(
    'MICROSOFT',
    'Microsoft',
    'KONTOR',
    NULL,
    'https://www.microsoft.com/sv-se/',
    'https://account.microsoft.com/billing/',
    ARRAY['noreply@microsoft.com','receipts@microsoft.com','msa@communication.microsoft.com'],
    ARRAY['MICROSOFT','MICROSOFT*','MSFT','MICROSOFT SE','MSOFT',
          'MICROSOFT SUBSCRIPTION','MICROSOFT 365','OFFICE 365',
          'MS*','AZURE MICROSOFT','MICROSOFT AZURE'],
    ARRAY['web','email'],
    true,
    'Microsoft 365, Azure, etc. Kvitto till e-post och i Microsoft-kontoportalen. API för företagsintegrationer via Azure.'
),
(
    'GOOGLE_WORKSPACE',
    'Google Workspace',
    'KONTOR',
    NULL,
    'https://workspace.google.com/',
    'https://admin.google.com/',
    ARRAY['noreply@accounts.google.com','receipts@google.com','googlecloud@google.com'],
    ARRAY['GOOGLE','GOOGLE WORKSPACE','GOOGLE*','GOOGLE LLC','GOOGLE CLOUD',
          'GOOGLE PLAY','G SUITE','GOOGLE SERVICES','GOOGLECLOUD'],
    ARRAY['web','email'],
    true,
    'Google Workspace (GMail, Drive, etc). Faktura/kvitto via Google Admin-konsolen. Full API-integration via Google Workspace API.'
),
(
    'AWS',
    'Amazon Web Services',
    'KONTOR',
    NULL,
    'https://aws.amazon.com/',
    'https://console.aws.amazon.com/billing/',
    ARRAY['no-reply@sns.amazonaws.com','receipts@amazon.com','aws-billing@amazon.com'],
    ARRAY['AWS','AMAZON WEB SERVICES','AMAZON AWS','AWS*','AMAZON EC2',
          'AWS AMAZON','AMAZON CLOUD','AMZN AWS','AWS MARKETPLACE'],
    ARRAY['web','email'],
    true,
    'Cloud-infrastruktur. Månadsvis faktura via AWS Billing Console och e-post. Full API via AWS Cost Explorer API.'
),
(
    'GITHUB',
    'GitHub',
    'KONTOR',
    NULL,
    'https://github.com',
    'https://github.com/settings/billing',
    ARRAY['noreply@github.com','billing@github.com','receipts@github.com'],
    ARRAY['GITHUB','GITHUB INC','GITHUB*','GITHUB SUBSCRIPTION',
          'GITHUB COM','GITHUB COPILOT'],
    ARRAY['web','email'],
    true,
    'Versionskontroll och CI/CD. Kvitto till e-post månadsvis. Full betalningshistorik i GitHub-inställningar. API för organisations-management.'
),
(
    'NOTION',
    'Notion',
    'KONTOR',
    NULL,
    'https://www.notion.so',
    'https://www.notion.so/my-account',
    ARRAY['noreply@makenotion.com','receipts@notion.so','billing@notion.so'],
    ARRAY['NOTION','NOTION LABS','NOTION*','NOTION SO','NOTION APP',
          'NOTION SUBSCRIPTION'],
    ARRAY['web','email'],
    true,
    'Workspace/anteckningsverktyg. Kvitto till e-post månadsvis/årsvis. Billing-historik på kontosidan. API för integrationer.'
);

-- ============================================================
-- BANK_FINANS
-- ============================================================

INSERT INTO merchant_profiles
    (merchant_id, display_name, category, org_number, website, receipt_portal,
     receipt_email_patterns, bank_aliases, receipt_support_channels, has_api_access, notes)
VALUES
(
    'SWEDBANK',
    'Swedbank',
    'BANK_FINANS',
    '502017-7753',
    'https://www.swedbank.se',
    'https://www.swedbank.se/privat/internetbanken.html',
    ARRAY['noreply@swedbank.se','internetbank@swedbank.se','info@swedbank.se'],
    ARRAY['SWEDBANK','SWEDBANK AB','SWEDBANK SE','SPARBANKEN','FORENINGSSPARBANKEN',
          'SWEDBANK PRIVAT','SWEDBANK FORETAG'],
    ARRAY['app','web','email'],
    true,
    'Storbank. Kontoutdrag och kvitton via internetbanken och Swish. API via Open Banking PSD2.'
),
(
    'HANDELSBANKEN',
    'Handelsbanken',
    'BANK_FINANS',
    '502007-7862',
    'https://www.handelsbanken.se',
    'https://www.handelsbanken.se/sv/privat/internet-mobil/',
    ARRAY['noreply@handelsbanken.se','internetbank@handelsbanken.se'],
    ARRAY['HANDELSBANKEN','SHB','SVENSKA HANDELSBANKEN','HANDELSBANKEN AB',
          'HANDELSBANKEN SE','SHB PRIVAT'],
    ARRAY['app','web','email'],
    true,
    'Storbank. Kontoutdrag via internetbank. Open Banking API (PSD2). Månadsstatement.'
),
(
    'SEB',
    'SEB',
    'BANK_FINANS',
    '502032-9537',
    'https://www.seb.se',
    'https://www.seb.se/privat/internetbanken/',
    ARRAY['noreply@seb.se','internetbank@seb.se','seb@seb.se'],
    ARRAY['SEB','SEB AB','SKANDINAVISKA ENSKILDA','SEB PRIVAT','SEB FORETAG',
          'SE-BANKEN','SEB SE'],
    ARRAY['app','web','email'],
    true,
    'Storbank. Kontoutdrag via SEBs internetbank. Open Banking API.'
),
(
    'NORDEA',
    'Nordea',
    'BANK_FINANS',
    '516406-0120',
    'https://www.nordea.se',
    'https://www.nordea.se/privat/internetbank/',
    ARRAY['noreply@nordea.se','internetbank@nordea.se'],
    ARRAY['NORDEA','NORDEA AB','NORDEA BANK','NORDEA SE','NORDEA PRIVAT',
          'NORDEA FORETAG','POSTGIROT'],
    ARRAY['app','web','email'],
    true,
    'Nordisk storbank. Kontoutdrag via Nordea-appen och internetbanken. Open Banking API (PSD2).'
),
(
    'REVOLUT',
    'Revolut',
    'BANK_FINANS',
    NULL,
    'https://www.revolut.com/sv-SE/',
    'https://app.revolut.com/',
    ARRAY['noreply@revolut.com','receipts@revolut.com','support@revolut.com'],
    ARRAY['REVOLUT','REVOLUT LTD','REVOLUT*','REVOLUT SE','REVOLUT BANK',
          'REVOLUT PAYMENTS','REVOLUT BUSINESS'],
    ARRAY['app','web','email'],
    true,
    'Neobank. Full transaktionshistorik och kvitton i appen. Export till CSV/Excel. API via Revolut Business API.'
),
(
    'WISE',
    'Wise (TransferWise)',
    'BANK_FINANS',
    NULL,
    'https://wise.com/se/',
    'https://wise.com/account/activity/',
    ARRAY['noreply@wise.com','receipts@wise.com','no-reply@wise.com'],
    ARRAY['WISE','TRANSFERWISE','WISE LTD','WISE PAYMENTS','WISE*',
          'WISE SE','WISE EUROPE','TRANSFERWISE LTD'],
    ARRAY['web','email'],
    true,
    'Internationella överföringar och multi-valutakonto. Kvitto per transaktion via e-post. CSV-export. API för betaltjänster.'
),
(
    'KLARNA',
    'Klarna',
    'BANK_FINANS',
    '556737-0431',
    'https://www.klarna.com/se/',
    'https://app.klarna.com/',
    ARRAY['noreply@klarna.com','receipts@klarna.com','info@klarna.com'],
    ARRAY['KLARNA','KLARNA AB','KLARNA BANK','KLARNA SE','KLARNA PAYMENTS',
          'KLARNA*','SOFORT','KLARNA CHECKOUT'],
    ARRAY['app','web','email'],
    true,
    'Betaltjänst och neobank. Full köphistorik i Klarna-appen. Automatiska kvitton per köp. API för merchants och partners.'
),
(
    'RESURSBANK',
    'Resurs Bank',
    'BANK_FINANS',
    '516401-0208',
    'https://www.resursbank.se',
    'https://www.resursbank.se/mina-sidor/',
    ARRAY['noreply@resursbank.se','faktura@resursbank.se'],
    ARRAY['RESURS BANK','RESURSBANK','RESURS BANK AB','RESURS SE',
          'RESURS KREDIT','RESURSFINANS'],
    ARRAY['web','email'],
    false,
    'Konsumentkreditbank. Fakturor och kvitton via Mina sidor. Kundtjänst: 0771-33 00 00.'
),
(
    'LENDO',
    'Lendo',
    'BANK_FINANS',
    '556685-6192',
    'https://www.lendo.se',
    NULL,
    ARRAY['noreply@lendo.se','kvitto@lendo.se'],
    ARRAY['LENDO','LENDO AB','LENDO SE','LENDO KREDIT'],
    ARRAY['web','email'],
    false,
    'Låneförmedlare. Ingen direkt transaktionsrole – kopplar till underliggande banker.'
),
(
    'TRUSTLY',
    'Trustly',
    'BANK_FINANS',
    '556754-8989',
    'https://www.trustly.com/se',
    NULL,
    ARRAY['noreply@trustly.com','receipts@trustly.com'],
    ARRAY['TRUSTLY','TRUSTLY GROUP','TRUSTLY AB','TRUSTLY SE','TRUSTLY*',
          'TRUSTLY PAYMENTS'],
    ARRAY['web','email'],
    true,
    'Betalningsplattform för bankbetalningar online. Kvitto via e-post per transaktion. API för merchants.'
),
(
    'TINK',
    'Tink',
    'BANK_FINANS',
    '556898-8887',
    'https://tink.com/se/',
    NULL,
    ARRAY['noreply@tink.com','support@tink.com'],
    ARRAY['TINK','TINK AB','TINK SE','TINK PAYMENTS','VISA TINK'],
    ARRAY['web','email'],
    true,
    'Open Banking-plattform (nu ägt av Visa). Infrastrukturbolag – syns sällan direkt på kontoutdrag. Komplett API.'
),
(
    'STRIPE',
    'Stripe',
    'BANK_FINANS',
    NULL,
    'https://stripe.com/se',
    'https://dashboard.stripe.com/',
    ARRAY['noreply@stripe.com','receipts@stripe.com','no-reply@stripe.com'],
    ARRAY['STRIPE','STRIPE INC','STRIPE*','STRIPE PAYMENTS','STRIPE SE',
          'STRIPE IRELAND','STRIPE TECHNOLOGY'],
    ARRAY['web','email'],
    true,
    'Betalningsinfrastruktur för utvecklare. Kvitto/faktura via Stripe Dashboard. Komplett API.'
),
(
    'SQUARE',
    'Square (Block)',
    'BANK_FINANS',
    NULL,
    'https://squareup.com/se/',
    'https://squareup.com/dashboard/',
    ARRAY['noreply@squareup.com','receipts@squareup.com'],
    ARRAY['SQUARE','SQ*','SQUARE INC','BLOCK INC','SQUARE PAYMENTS',
          'SQ SQUARE','SQUAREUP'],
    ARRAY['web','email'],
    true,
    'Betalterminal och betalningslösning. Kvitto till köpare via e-post eller SMS. Säljare: full historik i Dashboard. API för POS-integrationer.'
);

-- Trigger för updated_at
CREATE OR REPLACE FUNCTION update_merchant_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER merchant_profiles_updated_at
    BEFORE UPDATE ON merchant_profiles
    FOR EACH ROW EXECUTE FUNCTION update_merchant_updated_at();
