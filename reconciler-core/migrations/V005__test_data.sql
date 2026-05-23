-- =============================================================================
-- V005__test_data.sql
-- Sandbox seed data — 3 companies, 50 transactions, 20 invoices, vendors
-- =============================================================================

-- Only run in non-production environments
DO $$
BEGIN
    IF current_setting('app.environment', TRUE) = 'production' THEN
        RAISE EXCEPTION 'V005 seed data must NOT run in production!';
    END IF;
END;
$$;

-- ---------------------------------------------------------------------------
-- Helper: deterministic UUIDs for reproducible test data
-- ---------------------------------------------------------------------------

-- Company / party UUIDs
\set PARTY_LANDVEX     '\'a0000000-0000-0000-0000-000000000001\''
\set PARTY_ACME        '\'a0000000-0000-0000-0000-000000000002\''
\set PARTY_BYTECRAFT   '\'a0000000-0000-0000-0000-000000000003\''
\set PARTY_SUPPLIER_1  '\'a0000000-0000-0000-0000-000000000010\''
\set PARTY_SUPPLIER_2  '\'a0000000-0000-0000-0000-000000000011\''
\set PARTY_SUPPLIER_3  '\'a0000000-0000-0000-0000-000000000012\''

-- Vendor UUIDs
\set VENDOR_1          '\'b0000000-0000-0000-0000-000000000001\''
\set VENDOR_2          '\'b0000000-0000-0000-0000-000000000002\''
\set VENDOR_3          '\'b0000000-0000-0000-0000-000000000003\''

-- ---------------------------------------------------------------------------
-- Parties: 3 test companies + 3 suppliers
-- ---------------------------------------------------------------------------

INSERT INTO parties (id, name, registration_number, vat_number, country, entity_confidence) VALUES
    ('a0000000-0000-0000-0000-000000000001', 'LandveX AB',          '559141-7042', 'SE559141704201', 'SE', 1.0),
    ('a0000000-0000-0000-0000-000000000002', 'ACME Software AB',    '556800-1234', 'SE556800123401', 'SE', 1.0),
    ('a0000000-0000-0000-0000-000000000003', 'ByteCraft Solutions', '559300-5678', 'SE559300567801', 'SE', 1.0),
    ('a0000000-0000-0000-0000-000000000010', 'AWS EMEA SARL',       NULL,          'LU26375245',     'LU', 0.98),
    ('a0000000-0000-0000-0000-000000000011', 'Fortnox AB',          '556656-5975', 'SE556656597501', 'SE', 1.0),
    ('a0000000-0000-0000-0000-000000000012', 'Klarna Bank AB',      '556737-0431', 'SE556737043101', 'SE', 1.0)
ON CONFLICT DO NOTHING;

-- ---------------------------------------------------------------------------
-- Vendors
-- ---------------------------------------------------------------------------

INSERT INTO vendors (id, party_id, default_account_code, payment_terms_days, preferred_currency) VALUES
    ('b0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000010', '6710', 30, 'EUR'),
    ('b0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000011', '6700', 30, 'SEK'),
    ('b0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000012', '6550', 30, 'SEK')
ON CONFLICT DO NOTHING;

-- ---------------------------------------------------------------------------
-- Invoices (20) — mix of statuses, currencies, and companies
-- ---------------------------------------------------------------------------

INSERT INTO invoices (id, external_id, invoice_number, vendor_id, customer_id,
                      amount, tax_amount, currency, issued_at, due_at,
                      status, source, jurisdiction, confidence)
VALUES
-- Paid invoices
('c0000000-0000-0000-0000-000000000001','INV-EXT-001','AWS-2024-01',    'b0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001',  1250.00, 312.50,'EUR','2024-01-01','2024-01-31','paid',  'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000002','INV-EXT-002','AWS-2024-02',    'b0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001',  1380.50, 345.13,'EUR','2024-02-01','2024-02-29','paid',  'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000003','INV-EXT-003','FORTNOX-Q1-24', 'b0000000-0000-0000-0000-000000000002','a0000000-0000-0000-0000-000000000001',  2400.00, 600.00,'SEK','2024-01-15','2024-02-14','paid',  'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000004','INV-EXT-004','FORTNOX-Q2-24', 'b0000000-0000-0000-0000-000000000002','a0000000-0000-0000-0000-000000000001',  2400.00, 600.00,'SEK','2024-04-15','2024-05-14','paid',  'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000005','INV-EXT-005','KL-FEE-JAN',    'b0000000-0000-0000-0000-000000000003','a0000000-0000-0000-0000-000000000001',   320.00,  80.00,'SEK','2024-01-31','2024-02-28','paid',  'klarna_api', 'SE',0.97),
-- Overdue invoices
('c0000000-0000-0000-0000-000000000006','INV-EXT-006','AWS-2024-05',    'b0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001',  1520.00, 380.00,'EUR','2024-05-01','2024-05-31','overdue','fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000007','INV-EXT-007','BYTECRAFT-001', NULL,                                  'a0000000-0000-0000-0000-000000000002', 15000.00,3750.00,'SEK','2024-03-01','2024-03-31','overdue','manual',     'SE',0.95),
-- Partially paid
('c0000000-0000-0000-0000-000000000008','INV-EXT-008','BYTECRAFT-002', NULL,                                  'a0000000-0000-0000-0000-000000000002', 28000.00,7000.00,'SEK','2024-04-01','2024-04-30','partially_paid','manual','SE',0.95),
-- Issued / sent
('c0000000-0000-0000-0000-000000000009','INV-EXT-009','LVX-OUT-001',   NULL,                                  'a0000000-0000-0000-0000-000000000003', 45000.00,11250.00,'SEK','2024-05-15','2024-06-14','issued', 'wavult_api','SE',1.0),
('c0000000-0000-0000-0000-000000000010','INV-EXT-010','LVX-OUT-002',   NULL,                                  'a0000000-0000-0000-0000-000000000002', 12500.00,3125.00,'SEK','2024-05-20','2024-06-19','sent',   'wavult_api','SE',1.0),
-- Drafts
('c0000000-0000-0000-0000-000000000011','INV-EXT-011','LVX-DRAFT-001', NULL,                                  'a0000000-0000-0000-0000-000000000002',  8000.00,2000.00,'SEK','2024-06-01', NULL,        'draft',  'wavult_api','SE',0.80),
('c0000000-0000-0000-0000-000000000012','INV-EXT-012','AWS-2024-03',    'b0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001',  1190.00, 297.50,'EUR','2024-03-01','2024-03-31','paid',  'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000013','INV-EXT-013','AWS-2024-04',    'b0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001',  1340.00, 335.00,'EUR','2024-04-01','2024-04-30','paid',  'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000014','INV-EXT-014','KL-FEE-FEB',    'b0000000-0000-0000-0000-000000000003','a0000000-0000-0000-0000-000000000001',   290.00,  72.50,'SEK','2024-02-28','2024-03-28','paid',  'klarna_api', 'SE',0.97),
('c0000000-0000-0000-0000-000000000015','INV-EXT-015','KL-FEE-MAR',    'b0000000-0000-0000-0000-000000000003','a0000000-0000-0000-0000-000000000001',   310.00,  77.50,'SEK','2024-03-31','2024-04-30','paid',  'klarna_api', 'SE',0.97),
('c0000000-0000-0000-0000-000000000016','INV-EXT-016','FORTNOX-Q3-24', 'b0000000-0000-0000-0000-000000000002','a0000000-0000-0000-0000-000000000001',  2400.00, 600.00,'SEK','2024-07-15','2024-08-14','issued', 'fortnox_api','SE',1.0),
('c0000000-0000-0000-0000-000000000017','INV-EXT-017','LVX-OUT-003',   NULL,                                  'a0000000-0000-0000-0000-000000000003', 67500.00,16875.00,'SEK','2024-06-01','2024-06-30','sent',   'wavult_api','SE',1.0),
('c0000000-0000-0000-0000-000000000018','INV-EXT-018','CREDIT-001',    NULL,                                  'a0000000-0000-0000-0000-000000000002', -5000.00,-1250.00,'SEK','2024-05-01', NULL,        'cancelled','wavult_api','SE',1.0),
('c0000000-0000-0000-0000-000000000019','INV-EXT-019','LVX-OUT-004',   NULL,                                  'a0000000-0000-0000-0000-000000000002', 19800.00,4950.00,'SEK','2024-06-10','2024-07-09','issued', 'wavult_api','SE',1.0),
('c0000000-0000-0000-0000-000000000020','INV-EXT-020','AWS-2024-06',    'b0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001',  1680.00, 420.00,'EUR','2024-06-01','2024-06-30','issued', 'fortnox_api','SE',1.0)
ON CONFLICT DO NOTHING;

-- ---------------------------------------------------------------------------
-- Transactions (50) — generated via generate_series for brevity
-- ---------------------------------------------------------------------------

-- 30 bank transactions (Nordea)
INSERT INTO transactions (external_id, amount, currency, timestamp,
                          counterparty_id, merchant_name, payment_rail,
                          jurisdiction, tax_amount, tax_rate,
                          account_id, source, status, confidence)
SELECT
    'NORDEA-' || LPAD(gs::TEXT, 5, '0'),
    CASE (gs % 8)
        WHEN 0 THEN  -12500.00
        WHEN 1 THEN   -2400.00
        WHEN 2 THEN   -1250.00 * 10.89   -- EUR converted ~10.89
        WHEN 3 THEN   45000.00
        WHEN 4 THEN     -320.00
        WHEN 5 THEN  -28000.00
        WHEN 6 THEN   15000.00
        ELSE           -1680.00 * 10.89
    END,
    CASE (gs % 8) WHEN 2 THEN 'EUR' WHEN 7 THEN 'EUR' ELSE 'SEK' END,
    TIMESTAMPTZ '2024-01-15 09:00:00 UTC' + (gs - 1) * INTERVAL '7 days',
    CASE (gs % 3)
        WHEN 0 THEN 'a0000000-0000-0000-0000-000000000010'::UUID
        WHEN 1 THEN 'a0000000-0000-0000-0000-000000000011'::UUID
        ELSE         'a0000000-0000-0000-0000-000000000012'::UUID
    END,
    CASE (gs % 8)
        WHEN 0 THEN 'ByteCraft Solutions'
        WHEN 1 THEN 'Fortnox AB'
        WHEN 2 THEN 'AWS EMEA SARL'
        WHEN 3 THEN 'LandveX AB – inbetalning'
        WHEN 4 THEN 'Klarna Bank AB'
        WHEN 5 THEN 'ByteCraft Solutions – delfaktura'
        WHEN 6 THEN 'ACME Software AB'
        ELSE         'Amazon AWS'
    END,
    'bank_transfer'::payment_rail,
    'SE',
    CASE (gs % 8)
        WHEN 0 THEN  3125.00
        WHEN 1 THEN   600.00
        WHEN 2 THEN  312.50
        WHEN 3 THEN 11250.00
        WHEN 4 THEN    80.00
        WHEN 5 THEN  7000.00
        WHEN 6 THEN  3750.00
        ELSE          420.00
    END,
    0.25,
    '1931',
    'nordea_api',
    CASE (gs % 5)
        WHEN 0 THEN 'matched'::transaction_status
        WHEN 1 THEN 'matched'::transaction_status
        WHEN 2 THEN 'unmatched'::transaction_status
        ELSE         'pending'::transaction_status
    END,
    0.9 + (gs % 10) * 0.01
FROM generate_series(1, 30) AS gs
ON CONFLICT (external_id, source) DO NOTHING;

-- 15 Revolut card transactions
INSERT INTO transactions (external_id, amount, currency, timestamp,
                          merchant_name, payment_rail,
                          jurisdiction, tax_amount, tax_rate,
                          account_id, source, status, confidence)
SELECT
    'REVOLUT-' || LPAD(gs::TEXT, 5, '0'),
    -(50 + gs * 17.5),
    'EUR',
    TIMESTAMPTZ '2024-01-20 12:00:00 UTC' + gs * INTERVAL '5 days',
    CASE (gs % 5)
        WHEN 0 THEN 'Slack Technologies'
        WHEN 1 THEN 'GitHub Inc'
        WHEN 2 THEN 'Google Cloud'
        WHEN 3 THEN 'Zoom Video Communications'
        ELSE         'HubSpot Inc'
    END,
    'card'::payment_rail,
    'SE',
    (50 + gs * 17.5) * 0.25,
    0.25,
    '1941',
    'revolut_csv',
    CASE (gs % 3)
        WHEN 0 THEN 'matched'::transaction_status
        WHEN 1 THEN 'pending'::transaction_status
        ELSE         'unmatched'::transaction_status
    END,
    0.85 + (gs % 15) * 0.01
FROM generate_series(1, 15) AS gs
ON CONFLICT (external_id, source) DO NOTHING;

-- 5 Swish transactions (incoming payments)
INSERT INTO transactions (external_id, amount, currency, timestamp,
                          merchant_name, payment_rail,
                          jurisdiction, tax_amount, tax_rate,
                          account_id, source, status, confidence)
VALUES
    ('SWISH-00001',  5000.00,'SEK','2024-02-01 10:00:00 UTC','Privatkund A', 'swish','SE',1250.0,0.25,'1930','swish_api','matched',  0.98),
    ('SWISH-00002',  3500.00,'SEK','2024-02-15 14:30:00 UTC','Privatkund B', 'swish','SE', 875.0,0.25,'1930','swish_api','matched',  0.97),
    ('SWISH-00003',  7200.00,'SEK','2024-03-10 09:15:00 UTC','Privatkund C', 'swish','SE',1800.0,0.25,'1930','swish_api','unmatched',0.88),
    ('SWISH-00004',  1800.00,'SEK','2024-03-22 16:45:00 UTC','Privatkund D', 'swish','SE', 450.0,0.25,'1930','swish_api','pending',  0.91),
    ('SWISH-00005', 12000.00,'SEK','2024-04-05 11:00:00 UTC','Privatkund E', 'swish','SE',3000.0,0.25,'1930','swish_api','matched',  0.99)
ON CONFLICT (external_id, source) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Verify counts
-- ---------------------------------------------------------------------------

DO $$
DECLARE
    v_parties      INT;
    v_vendors      INT;
    v_invoices     INT;
    v_transactions INT;
BEGIN
    SELECT COUNT(*) INTO v_parties      FROM parties      WHERE id::TEXT LIKE 'a0000000%';
    SELECT COUNT(*) INTO v_vendors      FROM vendors       WHERE id::TEXT LIKE 'b0000000%';
    SELECT COUNT(*) INTO v_invoices     FROM invoices      WHERE external_id LIKE 'INV-EXT%';
    SELECT COUNT(*) INTO v_transactions FROM transactions  WHERE source IN ('nordea_api','revolut_csv','swish_api');

    RAISE NOTICE 'Seed data: % parties, % vendors, % invoices, % transactions',
        v_parties, v_vendors, v_invoices, v_transactions;

    IF v_parties < 6 THEN
        RAISE EXCEPTION 'Expected ≥6 parties, got %', v_parties;
    END IF;
    IF v_invoices < 20 THEN
        RAISE EXCEPTION 'Expected 20 invoices, got %', v_invoices;
    END IF;
    IF v_transactions < 45 THEN
        RAISE EXCEPTION 'Expected ≥45 transactions, got %', v_transactions;
    END IF;
END;
$$;
