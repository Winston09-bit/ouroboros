// =============================================================================
// src/db/repositories/transactions.rs
// Repository for the `kv_transactions` table (V007 migration)
// =============================================================================

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TransactionRow {
    pub id: Uuid,
    pub external_id: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub timestamp: DateTime<Utc>,
    pub counterparty_name: Option<String>,
    pub merchant_id: Option<String>,
    pub merchant_display_name: Option<String>,
    pub category: Option<String>,
    pub source: String,
    pub status: String,
    pub confidence: f64,
    pub raw_data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Row → TransactionRow mapper (dynamic query)
// ---------------------------------------------------------------------------

fn map_row(row: sqlx::postgres::PgRow) -> Result<TransactionRow> {
    Ok(TransactionRow {
        id: row.try_get("id")?,
        external_id: row.try_get("external_id")?,
        amount: row.try_get("amount")?,
        currency: row.try_get("currency")?,
        timestamp: row.try_get("timestamp")?,
        counterparty_name: row.try_get("counterparty_name")?,
        merchant_id: row.try_get("merchant_id")?,
        merchant_display_name: row.try_get("merchant_display_name")?,
        category: row.try_get("category")?,
        source: row.try_get("source")?,
        status: row.try_get("status")?,
        confidence: row.try_get("confidence")?,
        raw_data: row.try_get("raw_data")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

pub struct TransactionRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> TransactionRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // -----------------------------------------------------------------------
    // Writes
    // -----------------------------------------------------------------------

    /// Insert a new row into `kv_transactions`. The `id` field in `tx` is
    /// used as-is; call with `Uuid::new_v4()` to generate one.
    pub async fn insert(&self, tx: &TransactionRow) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO kv_transactions (
                id, external_id, amount, currency, timestamp,
                counterparty_name, merchant_id, merchant_display_name,
                category, source, status, confidence, raw_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id
            "#,
        )
        .bind(tx.id)
        .bind(&tx.external_id)
        .bind(tx.amount)
        .bind(&tx.currency)
        .bind(tx.timestamp)
        .bind(&tx.counterparty_name)
        .bind(&tx.merchant_id)
        .bind(&tx.merchant_display_name)
        .bind(&tx.category)
        .bind(&tx.source)
        .bind(&tx.status)
        .bind(tx.confidence)
        .bind(&tx.raw_data)
        .fetch_one(self.pool)
        .await
        .map_err(|e| anyhow!("insert failed: {e}"))?;

        Ok(id)
    }

    /// Insert or update by `external_id`.
    ///
    /// When `external_id` is `None` the row is always inserted (PostgreSQL
    /// NULLs are never equal, so there is no conflict).
    pub async fn upsert_by_external_id(&self, tx: &TransactionRow) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO kv_transactions (
                id, external_id, amount, currency, timestamp,
                counterparty_name, merchant_id, merchant_display_name,
                category, source, status, confidence, raw_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (external_id) DO UPDATE SET
                amount                = EXCLUDED.amount,
                currency              = EXCLUDED.currency,
                timestamp             = EXCLUDED.timestamp,
                counterparty_name     = EXCLUDED.counterparty_name,
                merchant_id           = EXCLUDED.merchant_id,
                merchant_display_name = EXCLUDED.merchant_display_name,
                category              = EXCLUDED.category,
                source                = EXCLUDED.source,
                status                = EXCLUDED.status,
                confidence            = EXCLUDED.confidence,
                raw_data              = EXCLUDED.raw_data,
                updated_at            = now()
            RETURNING id
            "#,
        )
        .bind(tx.id)
        .bind(&tx.external_id)
        .bind(tx.amount)
        .bind(&tx.currency)
        .bind(tx.timestamp)
        .bind(&tx.counterparty_name)
        .bind(&tx.merchant_id)
        .bind(&tx.merchant_display_name)
        .bind(&tx.category)
        .bind(&tx.source)
        .bind(&tx.status)
        .bind(tx.confidence)
        .bind(&tx.raw_data)
        .fetch_one(self.pool)
        .await
        .map_err(|e| anyhow!("upsert_by_external_id failed: {e}"))?;

        Ok(id)
    }

    // -----------------------------------------------------------------------
    // Reads
    // -----------------------------------------------------------------------

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<TransactionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT id, external_id, amount, currency, timestamp,
                   counterparty_name, merchant_id, merchant_display_name,
                   category, source, status, confidence, raw_data,
                   created_at, updated_at
            FROM kv_transactions
            ORDER BY timestamp DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
        .map_err(|e| anyhow!("list failed: {e}"))?;

        rows.into_iter().map(map_row).collect()
    }

    pub async fn get(&self, id: &Uuid) -> Result<Option<TransactionRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, external_id, amount, currency, timestamp,
                   counterparty_name, merchant_id, merchant_display_name,
                   category, source, status, confidence, raw_data,
                   created_at, updated_at
            FROM kv_transactions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| anyhow!("get failed: {e}"))?;

        row.map(map_row).transpose()
    }

    pub async fn count(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kv_transactions")
            .fetch_one(self.pool)
            .await
            .map_err(|e| anyhow!("count failed: {e}"))?;
        Ok(n)
    }

    pub async fn count_by_status(&self, status: &str) -> Result<i64> {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM kv_transactions WHERE status = $1")
                .bind(status)
                .fetch_one(self.pool)
                .await
                .map_err(|e| anyhow!("count_by_status failed: {e}"))?;
        Ok(n)
    }

    /// Return transactions that are still `unmatched` — i.e. have no receipt
    /// evidence attached — ordered oldest-first up to `limit` rows.
    pub async fn missing_evidence(&self, limit: i64) -> Result<Vec<TransactionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT id, external_id, amount, currency, timestamp,
                   counterparty_name, merchant_id, merchant_display_name,
                   category, source, status, confidence, raw_data,
                   created_at, updated_at
            FROM kv_transactions
            WHERE status = 'unmatched'
            ORDER BY timestamp ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await
        .map_err(|e| anyhow!("missing_evidence failed: {e}"))?;

        rows.into_iter().map(map_row).collect()
    }
}
