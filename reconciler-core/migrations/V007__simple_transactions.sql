CREATE TABLE IF NOT EXISTS kv_transactions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    external_id text UNIQUE,
    amount numeric(18,4) NOT NULL,
    currency text NOT NULL DEFAULT 'SEK',
    timestamp timestamptz NOT NULL,
    counterparty_name text,
    merchant_id text,
    merchant_display_name text,
    category text,
    source text NOT NULL,
    status text NOT NULL DEFAULT 'unmatched',
    confidence double precision DEFAULT 0.0,
    raw_data jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_kv_tx_timestamp ON kv_transactions(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_kv_tx_status ON kv_transactions(status);
CREATE INDEX IF NOT EXISTS idx_kv_tx_merchant ON kv_transactions(merchant_id);
CREATE INDEX IF NOT EXISTS idx_kv_tx_source ON kv_transactions(source);
