-- =============================================================================
-- V004__bas_accounts.sql
-- BAS 2024 — Svenska standardkontoplanen (komplett urval, 130+ konton)
-- =============================================================================

INSERT INTO accounts (code, name, account_type, currency, parent_code, jurisdiction) VALUES

-- ===========================================================================
-- TILLGÅNGAR — Klass 1
-- ===========================================================================

-- 10xx – Immateriella anläggningstillgångar
('1000', 'Immateriella anläggningstillgångar',        'asset', 'SEK', NULL,   'SE'),
('1010', 'Balanserade utgifter för FoU',               'asset', 'SEK', '1000', 'SE'),
('1020', 'Koncessioner m.m.',                          'asset', 'SEK', '1000', 'SE'),
('1030', 'Patent',                                     'asset', 'SEK', '1000', 'SE'),
('1040', 'Licenser',                                   'asset', 'SEK', '1000', 'SE'),
('1050', 'Varumärken',                                 'asset', 'SEK', '1000', 'SE'),
('1060', 'Hyresrätter och liknande rättigheter',       'asset', 'SEK', '1000', 'SE'),
('1070', 'Goodwill',                                   'asset', 'SEK', '1000', 'SE'),
('1080', 'Förskott för immateriella anläggningstillg.','asset', 'SEK', '1000', 'SE'),

-- 11xx – Byggnader och mark
('1100', 'Byggnader och mark',                         'asset', 'SEK', NULL,   'SE'),
('1110', 'Byggnader',                                  'asset', 'SEK', '1100', 'SE'),
('1120', 'Förbättringsutgifter på annans fastighet',   'asset', 'SEK', '1100', 'SE'),
('1130', 'Markanläggningar',                           'asset', 'SEK', '1100', 'SE'),
('1150', 'Mark',                                       'asset', 'SEK', '1100', 'SE'),

-- 12xx – Maskiner och inventarier
('1200', 'Maskiner och inventarier',                   'asset', 'SEK', NULL,   'SE'),
('1210', 'Maskiner och andra tekniska anläggningar',   'asset', 'SEK', '1200', 'SE'),
('1220', 'Inventarier och verktyg',                    'asset', 'SEK', '1200', 'SE'),
('1230', 'Datorer och kringutrustning',                'asset', 'SEK', '1200', 'SE'),
('1240', 'Bilar och andra transportmedel',             'asset', 'SEK', '1200', 'SE'),
('1250', 'Leasade tillgångar (IFRS 16)',               'asset', 'SEK', '1200', 'SE'),
('1290', 'Ackumulerade avskrivningar maskiner',        'contra','SEK', '1200', 'SE'),

-- 13xx – Finansiella anläggningstillgångar
('1300', 'Finansiella anläggningstillgångar',          'asset', 'SEK', NULL,   'SE'),
('1310', 'Andelar i koncernföretag',                   'asset', 'SEK', '1300', 'SE'),
('1320', 'Andelar i intresseföretag',                  'asset', 'SEK', '1300', 'SE'),
('1380', 'Andra långfristiga fordringar',               'asset', 'SEK', '1300', 'SE'),

-- 14xx – Lager och varulager
('1400', 'Lager av råvaror och förnödenheter',         'asset', 'SEK', NULL,   'SE'),
('1410', 'Lager av råvaror',                           'asset', 'SEK', '1400', 'SE'),
('1420', 'Lager av handelsvaror',                      'asset', 'SEK', '1400', 'SE'),
('1440', 'Lager av produkter i arbete',                'asset', 'SEK', '1400', 'SE'),
('1460', 'Lager av färdiga varor',                     'asset', 'SEK', '1400', 'SE'),

-- 15xx – Kortfristiga fordringar
('1500', 'Kortfristiga fordringar',                    'asset', 'SEK', NULL,   'SE'),
('1510', 'Kundfordringar',                             'asset', 'SEK', '1500', 'SE'),
('1511', 'Kundfordringar (SEK)',                       'asset', 'SEK', '1510', 'SE'),
('1512', 'Kundfordringar (EUR)',                       'asset', 'EUR', '1510', 'SE'),
('1515', 'Osäkra kundfordringar',                      'asset', 'SEK', '1500', 'SE'),
('1516', 'Kundfordringar — koncernföretag',            'asset', 'SEK', '1500', 'SE'),
('1560', 'Momsfordran',                                'asset', 'SEK', '1500', 'SE'),
('1570', 'Skattefordran',                              'asset', 'SEK', '1500', 'SE'),
('1580', 'Övriga kortfristiga fordringar',             'asset', 'SEK', '1500', 'SE'),
('1590', 'Upplupna intäkter och förutbetalda kostnader','asset','SEK', '1500', 'SE'),

-- 16xx – Förutbetalda kostnader och upplupna intäkter
('1600', 'Förutbetalda kostnader och upplupna intäkter','asset','SEK', NULL,   'SE'),
('1610', 'Förutbetalda hyreskostnader',                'asset', 'SEK', '1600', 'SE'),
('1620', 'Förutbetalda leasingavgifter',               'asset', 'SEK', '1600', 'SE'),
('1640', 'Upplupna ränteintäkter',                     'asset', 'SEK', '1600', 'SE'),

-- 19xx – Kassa och bank
('1900', 'Kassa och bank',                             'asset', 'SEK', NULL,   'SE'),
('1910', 'Kassa',                                      'asset', 'SEK', '1900', 'SE'),
('1920', 'PlusGiro',                                   'asset', 'SEK', '1900', 'SE'),
('1930', 'Företagskonto / checkkonto',                 'asset', 'SEK', '1900', 'SE'),
('1931', 'Nordea Företagskonto SEK',                   'asset', 'SEK', '1930', 'SE'),
('1940', 'Bankkonto utländsk valuta',                  'asset', 'EUR', '1900', 'SE'),
('1941', 'Revolut Business EUR',                       'asset', 'EUR', '1940', 'SE'),
('1942', 'Revolut Business SEK',                       'asset', 'SEK', '1940', 'SE'),
('1960', 'Kortfristiga placeringar',                   'asset', 'SEK', '1900', 'SE'),

-- ===========================================================================
-- SKULDER OCH EGET KAPITAL — Klass 2
-- ===========================================================================

-- 20xx – Eget kapital
('2000', 'Eget kapital',                               'equity','SEK', NULL,   'SE'),
('2010', 'Aktiekapital',                               'equity','SEK', '2000', 'SE'),
('2020', 'Bundna reserver',                            'equity','SEK', '2000', 'SE'),
('2030', 'Fria reserver',                              'equity','SEK', '2000', 'SE'),
('2040', 'Balanserat resultat',                        'equity','SEK', '2000', 'SE'),
('2050', 'Årets resultat',                             'equity','SEK', '2000', 'SE'),

-- 22xx – Obeskattade reserver
('2200', 'Obeskattade reserver',                       'liability','SEK','NULL','SE'),
('2210', 'Periodiseringsfonder',                       'liability','SEK','2200','SE'),

-- 23xx – Avsättningar
('2300', 'Avsättningar',                               'liability','SEK',NULL,  'SE'),
('2310', 'Avsättningar för pensioner',                 'liability','SEK','2300','SE'),
('2350', 'Övriga avsättningar',                        'liability','SEK','2300','SE'),

-- 24xx – Långfristiga skulder
('2400', 'Långfristiga skulder',                       'liability','SEK',NULL,  'SE'),
('2410', 'Banklån',                                    'liability','SEK','2400','SE'),
('2420', 'Checkräkningskredit',                        'liability','SEK','2400','SE'),
('2440', 'Fastighetslån',                              'liability','SEK','2400','SE'),
('2480', 'Leasingskulder (IFRS 16)',                   'liability','SEK','2400','SE'),

-- 25xx – Kortfristiga skulder
('2500', 'Kortfristiga skulder',                       'liability','SEK',NULL,  'SE'),
('2510', 'Leverantörsskulder',                         'liability','SEK','2500','SE'),
('2511', 'Leverantörsskulder SEK',                     'liability','SEK','2510','SE'),
('2512', 'Leverantörsskulder EUR',                     'liability','EUR','2510','SE'),
('2516', 'Leverantörsskulder koncernföretag',          'liability','SEK','2500','SE'),
('2550', 'Skatteskulder',                              'liability','SEK','2500','SE'),
('2560', 'Momsskuld',                                  'liability','SEK','2500','SE'),
('2561', 'Utgående moms 25%',                          'liability','SEK','2560','SE'),
('2562', 'Utgående moms 12%',                          'liability','SEK','2560','SE'),
('2563', 'Utgående moms 6%',                           'liability','SEK','2560','SE'),
('2640', 'Ingående moms',                              'asset',   'SEK','1500','SE'),
('2650', 'Momsredovisningskonto',                      'liability','SEK','2560','SE'),
('2710', 'Personalskatt',                              'liability','SEK','2500','SE'),
('2730', 'Lagstadgade arbetsgivaravgifter',            'liability','SEK','2500','SE'),
('2790', 'Övriga kortfristiga skulder',                'liability','SEK','2500','SE'),
('2900', 'Upplupna kostnader och förutbetalda intäkter','liability','SEK',NULL, 'SE'),
('2910', 'Upplupna löner',                             'liability','SEK','2900','SE'),
('2920', 'Upplupna semesterlöner',                     'liability','SEK','2900','SE'),
('2940', 'Upplupna räntekostnader',                    'liability','SEK','2900','SE'),

-- ===========================================================================
-- INTÄKTER — Klass 3
-- ===========================================================================

('3000', 'Rörelsens intäkter',                         'revenue','SEK', NULL,  'SE'),
('3010', 'Försäljning av varor, Sverige 25% moms',     'revenue','SEK', '3000','SE'),
('3020', 'Försäljning av varor, Sverige 12% moms',     'revenue','SEK', '3000','SE'),
('3030', 'Försäljning av varor, Sverige 6% moms',      'revenue','SEK', '3000','SE'),
('3040', 'Försäljning av varor, momsfri',              'revenue','SEK', '3000','SE'),
('3100', 'Försäljning av tjänster, Sverige 25% moms',  'revenue','SEK', '3000','SE'),
('3110', 'Försäljning av tjänster, Sverige 12%',       'revenue','SEK', '3000','SE'),
('3120', 'Försäljning av tjänster, momsfri',           'revenue','SEK', '3000','SE'),
('3200', 'Försäljning, export utanför EU',             'revenue','SEK', '3000','SE'),
('3210', 'Försäljning av varor till EU (OSS)',         'revenue','EUR', '3000','SE'),
('3300', 'Hyresintäkter',                              'revenue','SEK', '3000','SE'),
('3400', 'Övriga rörelseintäkter',                     'revenue','SEK', '3000','SE'),
('3500', 'Faktureringsavgifter',                       'revenue','SEK', '3000','SE'),
('3590', 'Kreditnotor och returer',                    'contra', 'SEK', '3000','SE'),
('3740', 'Öres- och kronutjämning',                    'revenue','SEK', '3000','SE'),
('3960', 'Valutakursvinster rörelsefordringar',        'revenue','SEK', '3000','SE'),

-- ===========================================================================
-- DIREKTA KOSTNADER — Klass 4
-- ===========================================================================

('4000', 'Inköp och direkta kostnader',                'expense','SEK', NULL,  'SE'),
('4010', 'Inköp av varor',                             'expense','SEK', '4000','SE'),
('4020', 'Inköp av råvaror',                           'expense','SEK', '4000','SE'),
('4100', 'Frakt och distribution',                     'expense','SEK', '4000','SE'),
('4200', 'Importtullar och importavgifter',             'expense','SEK', '4000','SE'),
('4400', 'Förbrukningsmaterial och förnödenheter',     'expense','SEK', '4000','SE'),
('4500', 'Underkonsulter / köpta tjänster',            'expense','SEK', '4000','SE'),
('4600', 'Förändring lager',                           'expense','SEK', '4000','SE'),
('4900', 'Övriga direkta kostnader',                   'expense','SEK', '4000','SE'),

-- ===========================================================================
-- PERSONALKOSTNADER — Klass 5–7 (urval)
-- ===========================================================================

('5000', 'Personalkostnader',                          'expense','SEK', NULL,  'SE'),
('5010', 'Löner tjänstemän',                           'expense','SEK', '5000','SE'),
('5020', 'Löner arbetare',                             'expense','SEK', '5000','SE'),
('5060', 'Semesterlöner och semesterersättningar',     'expense','SEK', '5000','SE'),
('5070', 'Övertidsersättningar',                       'expense','SEK', '5000','SE'),
('5090', 'Övriga löner och ersättningar',              'expense','SEK', '5000','SE'),
('5310', 'Arbetsgivaravgifter',                        'expense','SEK', '5000','SE'),
('5320', 'Särskild löneskatt',                         'expense','SEK', '5000','SE'),
('5400', 'Traktamenten och reseersättningar',          'expense','SEK', '5000','SE'),
('5500', 'Sjuk- och hälsovård',                        'expense','SEK', '5000','SE'),
('5600', 'Utbildning och kurser',                      'expense','SEK', '5000','SE'),
('5810', 'Pensionskostnader',                          'expense','SEK', '5000','SE'),
('5820', 'Gruppförsäkringar',                          'expense','SEK', '5000','SE'),

-- ===========================================================================
-- RÖRELSEKOSTNADER — Klass 6
-- ===========================================================================

('6000', 'Övriga rörelsekostnader',                    'expense','SEK', NULL,  'SE'),
('6010', 'Lokalhyra',                                  'expense','SEK', '6000','SE'),
('6020', 'El, vatten, värme',                          'expense','SEK', '6000','SE'),
('6100', 'Resekostnader',                              'expense','SEK', '6000','SE'),
('6110', 'Flygbiljetter och tåg',                      'expense','SEK', '6100','SE'),
('6120', 'Hotell och logi',                            'expense','SEK', '6100','SE'),
('6130', 'Bilersättning',                              'expense','SEK', '6100','SE'),
('6200', 'Marknadsföring och reklam',                  'expense','SEK', '6000','SE'),
('6250', 'Mässor och utställningar',                   'expense','SEK', '6000','SE'),
('6300', 'Kontorsmateriel och trycksaker',             'expense','SEK', '6000','SE'),
('6400', 'Telekommunikation och porto',                'expense','SEK', '6000','SE'),
('6410', 'Mobil och bredband',                         'expense','SEK', '6400','SE'),
('6500', 'Försäkringar',                               'expense','SEK', '6000','SE'),
('6550', 'Bankkostnader och kortavgifter',             'expense','SEK', '6000','SE'),
('6600', 'Revision och redovisning',                   'expense','SEK', '6000','SE'),
('6610', 'Juridiska kostnader',                        'expense','SEK', '6000','SE'),
('6700', 'IT-tjänster och programvarulicenser',        'expense','SEK', '6000','SE'),
('6710', 'Molntjänster (AWS, Azure, GCP)',             'expense','SEK', '6700','SE'),
('6720', 'SaaS-licenser',                              'expense','SEK', '6700','SE'),
('6800', 'Avskrivningar inventarier',                  'expense','SEK', '6000','SE'),
('6820', 'Avskrivningar immateriella tillgångar',      'expense','SEK', '6000','SE'),
('6900', 'Övriga rörelsekostnader',                    'expense','SEK', '6000','SE'),
('6960', 'Valutakursförluster rörelsefordringar',      'expense','SEK', '6000','SE'),
('6970', 'Kundförluster',                              'expense','SEK', '6000','SE'),

-- ===========================================================================
-- FINANSIELLA POSTER — Klass 8
-- ===========================================================================

('8000', 'Finansiella poster',                         'expense','SEK', NULL,  'SE'),
('8010', 'Ränteintäkter bankkonto',                    'revenue','SEK', '8000','SE'),
('8020', 'Utdelningar på andelar',                     'revenue','SEK', '8000','SE'),
('8040', 'Valutakursvinster finansiella poster',       'revenue','SEK', '8000','SE'),
('8300', 'Räntekostnader',                             'expense','SEK', '8000','SE'),
('8310', 'Räntekostnader banklån',                     'expense','SEK', '8300','SE'),
('8340', 'Räntekostnader leasingskulder',              'expense','SEK', '8300','SE'),
('8400', 'Valutakursförluster finansiella poster',     'expense','SEK', '8000','SE'),
('8700', 'Bokslutsdispositioner',                      'expense','SEK', '8000','SE'),
('8800', 'Inkomstskatt',                               'expense','SEK', '8000','SE'),
('8810', 'Aktuell skatt',                              'expense','SEK', '8800','SE'),
('8820', 'Uppskjuten skatt',                           'expense','SEK', '8800','SE'),
('8999', 'Årets resultat (bokslutskonto)',             'equity', 'SEK', NULL,  'SE')

ON CONFLICT (code) DO UPDATE
    SET name         = EXCLUDED.name,
        account_type = EXCLUDED.account_type,
        currency     = EXCLUDED.currency,
        parent_code  = EXCLUDED.parent_code,
        jurisdiction = EXCLUDED.jurisdiction,
        is_active    = TRUE;

-- Verify count
DO $$
DECLARE
    v_count INT;
BEGIN
    SELECT COUNT(*) INTO v_count FROM accounts WHERE jurisdiction = 'SE';
    RAISE NOTICE 'BAS 2024 loaded: % accounts', v_count;
    IF v_count < 100 THEN
        RAISE EXCEPTION 'Expected ≥100 BAS accounts, got %', v_count;
    END IF;
END;
$$;
