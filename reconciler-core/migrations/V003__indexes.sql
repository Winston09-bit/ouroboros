-- =============================================================================
-- V003__indexes.sql
-- Performance indexes — Reconciler Canonical Data Model
-- =============================================================================

-- ---------------------------------------------------------------------------
-- transactions
-- ---------------------------------------------------------------------------

-- Primary reconciliation query: unmatched transactions in a time window
CREATE INDEX idx_transactions_timestamp_status
    ON transactions (timestamp DESC, status);

-- Merchant deduplication / fuzzy lookup
CREATE INDEX idx_transactions_merchant_name
    ON transactions (merchant_name)
    WHERE merchant_name IS NOT NULL;

-- Join path: transaction → invoice
CREATE INDEX idx_transactions_invoice_id
    ON transactions (invoice_id)
    WHERE invoice_id IS NOT NULL;

-- Jurisdiction-partitioned reconciliation queries
CREATE INDEX idx_transactions_jurisdiction
    ON transactions (jurisdiction, status, timestamp DESC);

-- Duplicate detection: same amount + merchant in a window
CREATE INDEX idx_transactions_dup_detect
    ON transactions (amount, merchant_name, timestamp DESC)
    WHERE status != 'excluded';

-- Source-based ingestion deduplication
CREATE INDEX idx_transactions_external_id_source
    ON transactions (external_id, source)
    WHERE external_id IS NOT NULL;

-- Counterparty lookups
CREATE INDEX idx_transactions_counterparty
    ON transactions (counterparty_id)
    WHERE counterparty_id IS NOT NULL;

-- ---------------------------------------------------------------------------
-- invoices
-- ---------------------------------------------------------------------------

-- AR/AP aging: overdue + near-due dashboard queries
CREATE INDEX idx_invoices_status_due_at
    ON invoices (status, due_at)
    WHERE status NOT IN ('cancelled', 'paid');

-- Vendor invoice list
CREATE INDEX idx_invoices_vendor_id
    ON invoices (vendor_id, issued_at DESC)
    WHERE vendor_id IS NOT NULL;

-- Invoice number search / deduplication
CREATE INDEX idx_invoices_invoice_number
    ON invoices (invoice_number);

-- Customer receivables
CREATE INDEX idx_invoices_customer_id
    ON invoices (customer_id, issued_at DESC)
    WHERE customer_id IS NOT NULL;

-- Jurisdiction reporting
CREATE INDEX idx_invoices_jurisdiction_period
    ON invoices (jurisdiction, issued_at DESC);

-- ---------------------------------------------------------------------------
-- ledger_entries
-- ---------------------------------------------------------------------------

-- Journal-level queries (all lines for a voucher)
CREATE INDEX idx_ledger_entries_journal_id
    ON ledger_entries (journal_id);

-- Trial balance / period close: sum by account per period
CREATE INDEX idx_ledger_entries_account_period
    ON ledger_entries (account_code, period);

-- Transaction → ledger drill-down
CREATE INDEX idx_ledger_entries_transaction_id
    ON ledger_entries (transaction_id)
    WHERE transaction_id IS NOT NULL;

-- Invoice → ledger drill-down
CREATE INDEX idx_ledger_entries_invoice_id
    ON ledger_entries (invoice_id)
    WHERE invoice_id IS NOT NULL;

-- Jurisdiction + period for statutory reporting
CREATE INDEX idx_ledger_entries_jurisdiction_period
    ON ledger_entries (jurisdiction, period);

-- ---------------------------------------------------------------------------
-- audit_events
-- ---------------------------------------------------------------------------

-- Entity timeline: all events for a specific entity ordered by time
CREATE INDEX idx_audit_events_entity_timestamp
    ON audit_events (entity_id, timestamp DESC)
    WHERE entity_id IS NOT NULL;

-- Actor activity log
CREATE INDEX idx_audit_events_actor_timestamp
    ON audit_events (actor, timestamp DESC);

-- Action-type filtering (e.g. find all 'post_journal' events)
CREATE INDEX idx_audit_events_action
    ON audit_events (action, timestamp DESC);

-- Entity type + action cross-filter
CREATE INDEX idx_audit_events_entity_type_action
    ON audit_events (entity_type, action, timestamp DESC)
    WHERE entity_type IS NOT NULL;

-- ---------------------------------------------------------------------------
-- journals
-- ---------------------------------------------------------------------------

-- Open journals by date (batch posting workflows)
CREATE INDEX idx_journals_status_date
    ON journals (status, date DESC);

-- ---------------------------------------------------------------------------
-- tax_events
-- ---------------------------------------------------------------------------

-- VAT return period aggregation
CREATE INDEX idx_tax_events_jurisdiction_period
    ON tax_events (jurisdiction, period, event_type);

-- ---------------------------------------------------------------------------
-- parties
-- ---------------------------------------------------------------------------

-- Entity resolution: normalised name lookup (case-insensitive already via generated col)
CREATE INDEX idx_parties_normalized_name
    ON parties (normalized_name);

-- VAT number lookup (deduplication)
CREATE INDEX idx_parties_vat_number
    ON parties (vat_number)
    WHERE vat_number IS NOT NULL;

-- ---------------------------------------------------------------------------
-- documents
-- ---------------------------------------------------------------------------

-- Full-text search on OCR text
CREATE INDEX idx_documents_ocr_text_fts
    ON documents USING gin (to_tsvector('swedish', coalesce(ocr_text, '')));

-- JSONB field extraction queries
CREATE INDEX idx_documents_extracted_data
    ON documents USING gin (extracted_data)
    WHERE extracted_data IS NOT NULL;
