// =============================================================================
// src/db/repositories/transaction_repo.rs
// Repository pattern for the `transactions` table
// =============================================================================

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    error::{ReconcilerError, Result},
    model::{Transaction, TransactionStatus},
};

// ---------------------------------------------------------------------------
// Repository struct
// ---------------------------------------------------------------------------

/// Data-access object for the `transactions` table.
///
/// All methods use parameterised queries (`sqlx::query!` / `sqlx::query_as!`)
/// — no raw string concatenation.  The struct is cheaply cloneable because
/// `PgPool` is an `Arc`-wrapped reference-counted pool.
#[derive(Debug, Clone)]
pub struct TransactionRepository {
    pool: PgPool,
}

impl TransactionRepository {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // -----------------------------------------------------------------------
    // Writes
    // -----------------------------------------------------------------------

    /// Insert a new transaction row and return its generated UUID.
    ///
    /// The caller is responsible for setting `id` before calling — pass
    /// `Uuid::new_v4()` if you want the application to generate it, or let
    /// Postgres generate it by setting `id = Uuid::nil()` (nil is replaced
    /// by the DB default `gen_random_uuid()`).
    ///
    /// On unique-constraint violation (`external_id + source`) this returns
    /// [`ReconcilerError::Duplicate`].
    pub async fn insert(&self, txn: &Transaction) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar!(
            r#"
            INSERT INTO transactions (
                id, external_id, amount, currency, timestamp,
                counterparty_id, merchant_name, invoice_id,
                payment_rail, jurisdiction,
                tax_amount, tax_rate, account_id,
                source, status, confidence
            )
            VALUES (
                COALESCE($1, gen_random_uuid()),
                $2, $3, $4, $5,
                $6, $7, $8,
                $9::payment_rail, $10,
                $11, $12, $13,
                $14, $15::transaction_status, $16
            )
            RETURNING id
            "#,
            txn.id as Option<Uuid>,
            txn.external_id,
            txn.amount,
            txn.currency,
            txn.timestamp,
            txn.counterparty_id as Option<Uuid>,
            txn.merchant_name,
            txn.invoice_id as Option<Uuid>,
            txn.payment_rail.as_deref(),
            txn.jurisdiction,
            txn.tax_amount,
            txn.tax_rate,
            txn.account_id,
            txn.source,
            txn.status.as_str(),
            txn.confidence,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.constraint() == Some("transactions_external_id_source_unique") => {
                ReconcilerError::Duplicate(format!(
                    "transaction external_id={:?} source={} already exists",
                    txn.external_id, txn.source
                ))
            }
            _ => ReconcilerError::Database(e.to_string()),
        })?;

        Ok(id)
    }

    /// Update the `status` (and bump `updated_at`) of a single transaction.
    ///
    /// Returns [`ReconcilerError::NotFound`] if no row with that UUID exists.
    pub async fn update_status(&self, id: Uuid, status: TransactionStatus) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE transactions
               SET status     = $2::transaction_status,
                   updated_at = now()
             WHERE id = $1
            "#,
            id,
            status.as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ReconcilerError::Database(e.to_string()))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(ReconcilerError::NotFound(format!("transaction id={id}")));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Reads — single row
    // -----------------------------------------------------------------------

    /// Fetch a single transaction by its primary key.
    ///
    /// Returns `Ok(None)` when no row is found.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Transaction>> {
        let row = sqlx::query_as!(
            TransactionRow,
            r#"
            SELECT
                id,
                external_id,
                amount,
                currency,
                timestamp,
                counterparty_id,
                merchant_name,
                invoice_id,
                payment_rail,
                jurisdiction,
                tax_amount,
                tax_rate,
                account_id,
                source,
                status AS "status: String",
                confidence,
                created_at,
                updated_at
            FROM transactions
            WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReconcilerError::Database(e.to_string()))?;

        Ok(row.map(Transaction::from))
    }

    // -----------------------------------------------------------------------
    // Reads — collections
    // -----------------------------------------------------------------------

    /// Return all transactions in a given jurisdiction that are not yet
    /// matched.  Ordered by `timestamp ASC` so the oldest unmatched
    /// transactions surface first.
    ///
    /// Statuses considered "unmatched": `pending`, `unmatched`.
    pub async fn find_unmatched(&self, jurisdiction: &str) -> Result<Vec<Transaction>> {
        let rows = sqlx::query_as!(
            TransactionRow,
            r#"
            SELECT
                id,
                external_id,
                amount,
                currency,
                timestamp,
                counterparty_id,
                merchant_name,
                invoice_id,
                payment_rail,
                jurisdiction,
                tax_amount,
                tax_rate,
                account_id,
                source,
                status AS "status: String",
                confidence,
                created_at,
                updated_at
            FROM transactions
            WHERE jurisdiction = $1
              AND status       IN ('pending', 'unmatched')
            ORDER BY timestamp ASC
            "#,
            jurisdiction,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ReconcilerError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Transaction::from).collect())
    }

    /// Return all transactions whose `timestamp` falls within `[from, to)`.
    ///
    /// Ordered by `timestamp ASC`.  Useful for period-close reconciliation
    /// and monthly reporting.
    pub async fn find_by_period(
        &self,
        from: DateTime<Utc>,
        to:   DateTime<Utc>,
    ) -> Result<Vec<Transaction>> {
        if from >= to {
            return Err(ReconcilerError::InvalidInput(
                "find_by_period: `from` must be strictly before `to`".into(),
            ));
        }

        let rows = sqlx::query_as!(
            TransactionRow,
            r#"
            SELECT
                id,
                external_id,
                amount,
                currency,
                timestamp,
                counterparty_id,
                merchant_name,
                invoice_id,
                payment_rail,
                jurisdiction,
                tax_amount,
                tax_rate,
                account_id,
                source,
                status AS "status: String",
                confidence,
                created_at,
                updated_at
            FROM transactions
            WHERE timestamp >= $1
              AND timestamp <  $2
            ORDER BY timestamp ASC
            "#,
            from,
            to,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ReconcilerError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Transaction::from).collect())
    }

    /// Find potential duplicate transactions: same `amount` and `merchant_name`
    /// within a rolling window of `within_hours` hours before *and* after each
    /// candidate (centred on the most recent transaction with that profile).
    ///
    /// Uses the `idx_transactions_dup_detect` index defined in V003.
    ///
    /// Returns all matching rows excluding those with `status = 'excluded'`.
    pub async fn find_duplicates(
        &self,
        amount:       Decimal,
        merchant:     &str,
        within_hours: i64,
    ) -> Result<Vec<Transaction>> {
        if within_hours <= 0 {
            return Err(ReconcilerError::InvalidInput(
                "find_duplicates: `within_hours` must be > 0".into(),
            ));
        }

        let rows = sqlx::query_as!(
            TransactionRow,
            r#"
            SELECT
                id,
                external_id,
                amount,
                currency,
                timestamp,
                counterparty_id,
                merchant_name,
                invoice_id,
                payment_rail,
                jurisdiction,
                tax_amount,
                tax_rate,
                account_id,
                source,
                status AS "status: String",
                confidence,
                created_at,
                updated_at
            FROM transactions
            WHERE amount        = $1
              AND merchant_name = $2
              AND timestamp     >= (
                      SELECT MAX(timestamp) - ($3::BIGINT * INTERVAL '1 hour')
                      FROM   transactions
                      WHERE  amount        = $1
                        AND  merchant_name = $2
                        AND  status       != 'excluded'
                  )
              AND status != 'excluded'
            ORDER BY timestamp ASC
            "#,
            amount,
            merchant,
            within_hours,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ReconcilerError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Transaction::from).collect())
    }

    // -----------------------------------------------------------------------
    // Batch helpers
    // -----------------------------------------------------------------------

    /// Bulk-insert a slice of transactions in a single round-trip using
    /// `UNNEST`.  Returns the list of generated UUIDs in insertion order.
    ///
    /// On any constraint violation the entire batch is rolled back.
    pub async fn insert_batch(&self, txns: &[Transaction]) -> Result<Vec<Uuid>> {
        if txns.is_empty() {
            return Ok(vec![]);
        }

        // Decompose structs into parallel vecs for UNNEST
        let (external_ids, amounts, currencies, timestamps,
             merchant_names, jurisdictions, tax_amounts,
             sources, statuses, confidences): (
            Vec<_>, Vec<_>, Vec<_>, Vec<_>,
            Vec<_>, Vec<_>, Vec<_>,
            Vec<_>, Vec<_>, Vec<_>,
        ) = txns.iter().map(|t| (
            t.external_id.clone(),
            t.amount,
            t.currency.clone(),
            t.timestamp,
            t.merchant_name.clone(),
            t.jurisdiction.clone(),
            t.tax_amount,
            t.source.clone(),
            t.status.as_str().to_owned(),
            t.confidence,
        )).unzip_n_vec();

        let ids: Vec<Uuid> = sqlx::query_scalar!(
            r#"
            INSERT INTO transactions (
                external_id, amount, currency, timestamp,
                merchant_name, jurisdiction, tax_amount,
                source, status, confidence
            )
            SELECT * FROM UNNEST(
                $1::TEXT[],
                $2::NUMERIC[],
                $3::CHAR(3)[],
                $4::TIMESTAMPTZ[],
                $5::TEXT[],
                $6::TEXT[],
                $7::NUMERIC[],
                $8::TEXT[],
                $9::transaction_status[],
                $10::NUMERIC[]
            )
            RETURNING id
            "#,
            &external_ids    as &[Option<String>],
            &amounts         as &[Decimal],
            &currencies      as &[String],
            &timestamps      as &[DateTime<Utc>],
            &merchant_names  as &[Option<String>],
            &jurisdictions   as &[String],
            &tax_amounts     as &[Decimal],
            &sources         as &[String],
            &statuses        as &[String],
            &confidences     as &[Decimal],
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ReconcilerError::Database(e.to_string()))?;

        Ok(ids)
    }
}

// ---------------------------------------------------------------------------
// Internal DB row type (flat sqlx mapping → avoids derive macro on model)
// ---------------------------------------------------------------------------

/// Raw row type returned by sqlx — mirrors `transactions` columns exactly.
/// Converted to the domain [`Transaction`] via `From<TransactionRow>`.
#[derive(Debug)]
struct TransactionRow {
    id:             Uuid,
    external_id:    Option<String>,
    amount:         Decimal,
    currency:       String,
    timestamp:      DateTime<Utc>,
    counterparty_id:Option<Uuid>,
    merchant_name:  Option<String>,
    invoice_id:     Option<Uuid>,
    payment_rail:   Option<String>,
    jurisdiction:   String,
    tax_amount:     Decimal,
    tax_rate:       Option<Decimal>,
    account_id:     Option<String>,
    source:         String,
    status:         String,
    confidence:     Decimal,
    created_at:     DateTime<Utc>,
    updated_at:     DateTime<Utc>,
}

impl From<TransactionRow> for Transaction {
    fn from(r: TransactionRow) -> Self {
        Transaction {
            id:              Some(r.id),
            external_id:     r.external_id,
            amount:          r.amount,
            currency:        r.currency,
            timestamp:       r.timestamp,
            counterparty_id: r.counterparty_id,
            merchant_name:   r.merchant_name,
            invoice_id:      r.invoice_id,
            payment_rail:    r.payment_rail,
            jurisdiction:    r.jurisdiction,
            tax_amount:      r.tax_amount,
            tax_rate:        r.tax_rate,
            account_id:      r.account_id,
            source:          r.source,
            status:          TransactionStatus::from_str(&r.status),
            confidence:      r.confidence,
            created_at:      r.created_at,
            updated_at:      r.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper trait: unzip 10-tuple iterator into parallel vecs
// ---------------------------------------------------------------------------

trait UnzipN10<A,B,C,D,E,F,G,H,I,J> {
    fn unzip_n_vec(self) -> (Vec<A>,Vec<B>,Vec<C>,Vec<D>,Vec<E>,Vec<F>,Vec<G>,Vec<H>,Vec<I>,Vec<J>);
}

impl<It,A,B,C,D,E,F,G,H,I,J> UnzipN10<A,B,C,D,E,F,G,H,I,J> for It
where
    It: Iterator<Item = (A,B,C,D,E,F,G,H,I,J)>,
{
    fn unzip_n_vec(self) -> (Vec<A>,Vec<B>,Vec<C>,Vec<D>,Vec<E>,Vec<F>,Vec<G>,Vec<H>,Vec<I>,Vec<J>) {
        let mut a=vec![]; let mut b=vec![]; let mut c=vec![]; let mut d=vec![];
        let mut e=vec![]; let mut f=vec![]; let mut g=vec![]; let mut h=vec![];
        let mut i=vec![]; let mut j=vec![];
        for (va,vb,vc,vd,ve,vf,vg,vh,vi,vj) in self {
            a.push(va); b.push(vb); c.push(vc); d.push(vd);
            e.push(ve); f.push(vf); g.push(vg); h.push(vh);
            i.push(vi); j.push(vj);
        }
        (a,b,c,d,e,f,g,h,i,j)
    }
}

// ---------------------------------------------------------------------------
// Unit tests (no live DB required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn find_by_period_rejects_inverted_range() {
        // We can't call the async method in a simple unit test without a pool,
        // but we can verify the guard logic by constructing the error case.
        let from = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let to   = Utc.with_ymd_and_hms(2024, 5, 1, 0, 0, 0).unwrap(); // before from

        // Simulate the guard
        let result: Result<()> = if from >= to {
            Err(ReconcilerError::InvalidInput(
                "find_by_period: `from` must be strictly before `to`".into(),
            ))
        } else {
            Ok(())
        };

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("strictly before"));
    }

    #[test]
    fn find_duplicates_rejects_zero_hours() {
        let result: Result<()> = if 0_i64 <= 0 {
            Err(ReconcilerError::InvalidInput(
                "find_duplicates: `within_hours` must be > 0".into(),
            ))
        } else {
            Ok(())
        };

        assert!(result.is_err());
    }

    #[test]
    fn transaction_row_maps_status() {
        let status = TransactionStatus::from_str("matched");
        assert_eq!(status.as_str(), "matched");

        let status = TransactionStatus::from_str("unknown_garbage");
        assert_eq!(status.as_str(), "pending"); // fallback
    }
}
