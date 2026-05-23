-- =============================================================================
-- V001__initial_schema.sql
-- Reconciler Canonical Data Model — Initial Schema
-- =============================================================================

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ---------------------------------------------------------------------------
-- ENUM types
-- ---------------------------------------------------------------------------

CREATE TYPE transaction_status AS ENUM (
    'pending',
    'matched',
    'unmatched',
    'duplicate',
    'excluded',
    'error'
);

CREATE TYPE invoice_status AS ENUM (
    'draft',
    'issued',
    'sent',
    'partially_paid',
    'paid',
    'overdue',
    'cancelled',
    'disputed'
);

CREATE TYPE journal_status AS ENUM (
    'draft',
    'pending_review',
    'posted',
    'reversed',
    'void'
);

CREATE TYPE payment_rail AS ENUM (
    'bank_transfer',
    'card',
    'swish',
    'klarna',
    'paypal',
    'stripe',
    'cash',
    'crypto',
    'direct_debit',
    'other'
);

CREATE TYPE account_type AS ENUM (
    'asset',
    'liability',
    'equity',
    'revenue',
    'expense',
    'contra'
);

CREATE TYPE tax_event_type AS ENUM (
    'sale',
    'purchase',
    'import',
    'export',
    'reverse_charge',
    'exempt',
    'zero_rated',
    'intra_eu'
);

CREATE TYPE document_type AS ENUM (
    'invoice',
    'receipt',
    'bank_statement',
    'contract',
    'credit_note',
    'customs_declaration',
    'other'
);

CREATE TYPE entity_type AS ENUM (
    'transaction',
    'invoice',
    'ledger_entry',
    'journal',
    'party',
    'account',
    'vendor',
    'tax_event',
    'document'
);

-- ---------------------------------------------------------------------------
-- accounts (chart of accounts — referenced by other tables, create first)
-- ---------------------------------------------------------------------------

CREATE TABLE accounts (
    code            VARCHAR(10)     PRIMARY KEY,
    name            TEXT            NOT NULL,
    account_type    account_type    NOT NULL,
    currency        CHAR(3)         NOT NULL DEFAULT 'SEK',
    parent_code     VARCHAR(10)     REFERENCES accounts(code),
    jurisdiction    VARCHAR(10)     NOT NULL DEFAULT 'SE',
    is_active       BOOLEAN         NOT NULL DEFAULT TRUE,

    CONSTRAINT accounts_code_format CHECK (code ~ '^[0-9]{4,10}$')
);

COMMENT ON TABLE  accounts IS 'Chart of accounts — BAS 2024 and custom extensions';
COMMENT ON COLUMN accounts.code         IS 'BAS account code, e.g. 1510';
COMMENT ON COLUMN accounts.account_type IS 'Asset/Liability/Equity/Revenue/Expense/Contra';
COMMENT ON COLUMN accounts.parent_code  IS 'Parent account for hierarchical grouping';

-- ---------------------------------------------------------------------------
-- parties (companies, persons — normalised entity)
-- ---------------------------------------------------------------------------

CREATE TABLE parties (
    id                  UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    name                TEXT            NOT NULL,
    normalized_name     TEXT            NOT NULL GENERATED ALWAYS AS (lower(trim(name))) STORED,
    registration_number VARCHAR(50),
    vat_number          VARCHAR(30),
    country             CHAR(2)         NOT NULL DEFAULT 'SE',
    entity_confidence   NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                            CHECK (entity_confidence BETWEEN 0 AND 1),
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT now(),

    CONSTRAINT parties_vat_unique UNIQUE (vat_number),
    CONSTRAINT parties_reg_unique UNIQUE (registration_number)
);

COMMENT ON TABLE  parties IS 'Normalised entity registry — vendors, customers, counterparties';
COMMENT ON COLUMN parties.entity_confidence IS '0.0–1.0 ML confidence that entity is correctly resolved';

-- ---------------------------------------------------------------------------
-- vendors (enrichment layer on top of parties)
-- ---------------------------------------------------------------------------

CREATE TABLE vendors (
    id                      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    party_id                UUID        NOT NULL REFERENCES parties(id) ON DELETE RESTRICT,
    default_account_code    VARCHAR(10) REFERENCES accounts(code),
    payment_terms_days      SMALLINT    NOT NULL DEFAULT 30
                                CHECK (payment_terms_days >= 0),
    preferred_currency      CHAR(3)     NOT NULL DEFAULT 'SEK',
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT vendors_party_unique UNIQUE (party_id)
);

COMMENT ON TABLE  vendors IS 'Vendor-specific settings layered over the parties entity';

-- ---------------------------------------------------------------------------
-- transactions
-- ---------------------------------------------------------------------------

CREATE TABLE transactions (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    external_id     TEXT,
    amount          NUMERIC(20,6)   NOT NULL,
    currency        CHAR(3)         NOT NULL DEFAULT 'SEK',
    timestamp       TIMESTAMPTZ     NOT NULL,
    counterparty_id UUID            REFERENCES parties(id),
    merchant_name   TEXT,
    invoice_id      UUID,           -- FK added after invoices table exists
    payment_rail    payment_rail,
    jurisdiction    VARCHAR(10)     NOT NULL DEFAULT 'SE',
    tax_amount      NUMERIC(20,6)   NOT NULL DEFAULT 0,
    tax_rate        NUMERIC(7,4),
    account_id      VARCHAR(10)     REFERENCES accounts(code),
    source          TEXT            NOT NULL,
    status          transaction_status NOT NULL DEFAULT 'pending',
    confidence      NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                        CHECK (confidence BETWEEN 0 AND 1),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT now(),

    CONSTRAINT transactions_external_id_source_unique UNIQUE (external_id, source)
);

COMMENT ON TABLE  transactions IS 'Raw bank / payment-rail transactions';
COMMENT ON COLUMN transactions.external_id  IS 'ID in originating system (bank, Stripe, etc.)';
COMMENT ON COLUMN transactions.confidence   IS 'ML reconciliation confidence 0.0–1.0';
COMMENT ON COLUMN transactions.source       IS 'Ingestion source identifier, e.g. nordea_api, revolut_csv';

-- updated_at auto-touch trigger (shared helper, applied per-table below)
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

CREATE TRIGGER transactions_updated_at
    BEFORE UPDATE ON transactions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ---------------------------------------------------------------------------
-- invoices
-- ---------------------------------------------------------------------------

CREATE TABLE invoices (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    external_id     TEXT,
    invoice_number  TEXT            NOT NULL,
    vendor_id       UUID            REFERENCES vendors(id),
    customer_id     UUID            REFERENCES parties(id),
    amount          NUMERIC(20,6)   NOT NULL,
    tax_amount      NUMERIC(20,6)   NOT NULL DEFAULT 0,
    currency        CHAR(3)         NOT NULL DEFAULT 'SEK',
    issued_at       TIMESTAMPTZ     NOT NULL,
    due_at          TIMESTAMPTZ,
    status          invoice_status  NOT NULL DEFAULT 'draft',
    source          TEXT            NOT NULL,
    jurisdiction    VARCHAR(10)     NOT NULL DEFAULT 'SE',
    confidence      NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                        CHECK (confidence BETWEEN 0 AND 1),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT now(),

    CONSTRAINT invoices_number_source_unique UNIQUE (invoice_number, source)
);

COMMENT ON TABLE invoices IS 'Inbound and outbound invoices';

CREATE TRIGGER invoices_updated_at
    BEFORE UPDATE ON invoices
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Now add the FK that requires invoices to exist
ALTER TABLE transactions
    ADD CONSTRAINT transactions_invoice_id_fk
    FOREIGN KEY (invoice_id) REFERENCES invoices(id);

-- ---------------------------------------------------------------------------
-- journals
-- ---------------------------------------------------------------------------

CREATE TABLE journals (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    description     TEXT            NOT NULL,
    date            DATE            NOT NULL,
    status          journal_status  NOT NULL DEFAULT 'draft',
    posted_by       TEXT,
    confidence      NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                        CHECK (confidence BETWEEN 0 AND 1),
    is_reversed     BOOLEAN         NOT NULL DEFAULT FALSE,
    reversed_by     UUID            REFERENCES journals(id),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT now()
);

COMMENT ON TABLE  journals IS 'Journal vouchers — groups of double-entry ledger lines';
COMMENT ON COLUMN journals.reversed_by IS 'UUID of the reversing journal entry if this has been reversed';

-- ---------------------------------------------------------------------------
-- ledger_entries
-- ---------------------------------------------------------------------------

CREATE TABLE ledger_entries (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    journal_id      UUID            NOT NULL REFERENCES journals(id) ON DELETE RESTRICT,
    account_code    VARCHAR(10)     NOT NULL REFERENCES accounts(code),
    account_name    TEXT            NOT NULL,
    debit           NUMERIC(20,6)   NOT NULL DEFAULT 0 CHECK (debit  >= 0),
    credit          NUMERIC(20,6)   NOT NULL DEFAULT 0 CHECK (credit >= 0),
    currency        CHAR(3)         NOT NULL DEFAULT 'SEK',
    description     TEXT,
    transaction_id  UUID            REFERENCES transactions(id),
    invoice_id      UUID            REFERENCES invoices(id),
    jurisdiction    VARCHAR(10)     NOT NULL DEFAULT 'SE',
    period          CHAR(7)         NOT NULL,   -- YYYY-MM
    confidence      NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                        CHECK (confidence BETWEEN 0 AND 1),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT now(),

    CONSTRAINT ledger_entries_debit_or_credit CHECK (
        (debit > 0 AND credit = 0) OR (credit > 0 AND debit = 0)
    ),
    CONSTRAINT ledger_entries_period_format CHECK (period ~ '^\d{4}-(0[1-9]|1[0-2])$')
);

COMMENT ON TABLE  ledger_entries IS 'Double-entry bookkeeping lines — immutable once journal is posted';
COMMENT ON COLUMN ledger_entries.period IS 'Accounting period in YYYY-MM format';

-- ---------------------------------------------------------------------------
-- tax_events
-- ---------------------------------------------------------------------------

CREATE TABLE tax_events (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type      tax_event_type  NOT NULL,
    amount          NUMERIC(20,6)   NOT NULL,
    tax_amount      NUMERIC(20,6)   NOT NULL DEFAULT 0,
    tax_rate        NUMERIC(7,4),
    jurisdiction    VARCHAR(10)     NOT NULL DEFAULT 'SE',
    period          CHAR(7)         NOT NULL,   -- YYYY-MM
    transaction_id  UUID            REFERENCES transactions(id),
    invoice_id      UUID            REFERENCES invoices(id),
    confidence      NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                        CHECK (confidence BETWEEN 0 AND 1),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT now(),

    CONSTRAINT tax_events_period_format CHECK (period ~ '^\d{4}-(0[1-9]|1[0-2])$')
);

COMMENT ON TABLE tax_events IS 'VAT and tax obligation events derived from transactions/invoices';

-- ---------------------------------------------------------------------------
-- documents
-- ---------------------------------------------------------------------------

CREATE TABLE documents (
    id              UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    doc_type        document_type   NOT NULL,
    filename        TEXT            NOT NULL,
    storage_url     TEXT            NOT NULL,
    ocr_text        TEXT,
    extracted_data  JSONB,
    confidence      NUMERIC(5,4)    NOT NULL DEFAULT 1.0
                        CHECK (confidence BETWEEN 0 AND 1),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT now()
);

COMMENT ON TABLE  documents IS 'Raw documents: PDFs, images — stores OCR text and ML-extracted fields';
COMMENT ON COLUMN documents.extracted_data IS 'Structured data extracted by OCR/ML pipeline (amounts, dates, etc.)';

-- ---------------------------------------------------------------------------
-- audit_events (append-only event log)
-- ---------------------------------------------------------------------------

CREATE TABLE audit_events (
    id          UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp   TIMESTAMPTZ     NOT NULL DEFAULT now(),
    actor       TEXT            NOT NULL,
    action      TEXT            NOT NULL,
    reason      TEXT,
    confidence  NUMERIC(5,4)    DEFAULT NULL
                    CHECK (confidence IS NULL OR confidence BETWEEN 0 AND 1),
    source      TEXT,
    payload     JSONB,
    entity_id   UUID,
    entity_type entity_type
);

COMMENT ON TABLE  audit_events IS 'Immutable append-only audit log — no UPDATE or DELETE allowed';
COMMENT ON COLUMN audit_events.actor      IS 'User, service-account, or system that caused the event';
COMMENT ON COLUMN audit_events.entity_id  IS 'UUID of the affected entity';
COMMENT ON COLUMN audit_events.payload    IS 'Full before/after state snapshot as JSON';
