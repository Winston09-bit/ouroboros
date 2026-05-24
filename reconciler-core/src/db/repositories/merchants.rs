// =============================================================================
// src/db/repositories/merchants.rs
// Repository for the `merchant_profiles` table (V006 migration)
// =============================================================================

use anyhow::{anyhow, Result};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MerchantRow {
    pub id: Uuid,
    pub merchant_id: String,
    pub display_name: String,
    pub category: String,
    pub org_number: Option<String>,
    pub website: Option<String>,
    pub bank_aliases: Vec<String>,
    pub has_api_access: bool,
}

// ---------------------------------------------------------------------------
// Internal mapper
// ---------------------------------------------------------------------------

fn map_row(row: sqlx::postgres::PgRow) -> Result<MerchantRow> {
    Ok(MerchantRow {
        id: row.try_get("id")?,
        merchant_id: row.try_get("merchant_id")?,
        display_name: row.try_get("display_name")?,
        category: row.try_get("category")?,
        org_number: row.try_get("org_number")?,
        website: row.try_get("website")?,
        bank_aliases: row
            .try_get::<Option<Vec<String>>, _>("bank_aliases")?
            .unwrap_or_default(),
        has_api_access: row
            .try_get::<Option<bool>, _>("has_api_access")?
            .unwrap_or(false),
    })
}

// ---------------------------------------------------------------------------
// Seed struct (matches seeds/merchants.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MerchantSeed {
    merchant_id: String,
    display_name: String,
    category: String,
    org_number: Option<String>,
    website: Option<String>,
    receipt_portal: Option<String>,
    receipt_email_patterns: Option<Vec<String>>,
    bank_aliases: Option<Vec<String>>,
    receipt_support_channels: Option<Vec<String>>,
    has_api_access: Option<bool>,
    notes: Option<String>,
    country: Option<String>,
}

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

pub struct MerchantRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MerchantRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Fetch a single merchant by its text `merchant_id` (e.g. `"ICA"`).
    pub async fn by_id(&self, merchant_id: &str) -> Result<Option<MerchantRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, merchant_id, display_name, category,
                   org_number, website, bank_aliases, has_api_access
            FROM merchant_profiles
            WHERE merchant_id = $1
            "#,
        )
        .bind(merchant_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| anyhow!("by_id failed: {e}"))?;

        row.map(map_row).transpose()
    }

    /// Return every merchant ordered alphabetically by `merchant_id`.
    pub async fn all(&self) -> Result<Vec<MerchantRow>> {
        let rows = sqlx::query(
            r#"
            SELECT id, merchant_id, display_name, category,
                   org_number, website, bank_aliases, has_api_access
            FROM merchant_profiles
            ORDER BY merchant_id ASC
            "#,
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| anyhow!("all failed: {e}"))?;

        rows.into_iter().map(map_row).collect()
    }

    /// Return all merchants in a given category.
    pub async fn by_category(&self, category: &str) -> Result<Vec<MerchantRow>> {
        let rows = sqlx::query(
            r#"
            SELECT id, merchant_id, display_name, category,
                   org_number, website, bank_aliases, has_api_access
            FROM merchant_profiles
            WHERE category = $1
            ORDER BY merchant_id ASC
            "#,
        )
        .bind(category)
        .fetch_all(self.pool)
        .await
        .map_err(|e| anyhow!("by_category failed: {e}"))?;

        rows.into_iter().map(map_row).collect()
    }

    /// Total number of merchant profiles in the table.
    pub async fn count(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM merchant_profiles")
            .fetch_one(self.pool)
            .await
            .map_err(|e| anyhow!("count failed: {e}"))?;
        Ok(n)
    }

    /// Read `path` (JSON array of merchant objects) and insert any new
    /// merchants. Existing rows (same `merchant_id`) are left untouched.
    ///
    /// Returns the number of rows actually inserted.
    pub async fn seed_from_json(&self, path: &str) -> Result<usize> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("cannot read seed file {path}: {e}"))?;

        let seeds: Vec<MerchantSeed> = serde_json::from_str(&content)
            .map_err(|e| anyhow!("cannot parse seed file {path}: {e}"))?;

        let mut inserted = 0usize;

        for seed in &seeds {
            let rows_affected = sqlx::query(
                r#"
                INSERT INTO merchant_profiles (
                    merchant_id, display_name, category,
                    org_number, website, receipt_portal,
                    receipt_email_patterns, bank_aliases,
                    receipt_support_channels, has_api_access,
                    notes, country
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (merchant_id) DO NOTHING
                "#,
            )
            .bind(&seed.merchant_id)
            .bind(&seed.display_name)
            .bind(&seed.category)
            .bind(&seed.org_number)
            .bind(&seed.website)
            .bind(&seed.receipt_portal)
            .bind(&seed.receipt_email_patterns)
            .bind(&seed.bank_aliases)
            .bind(&seed.receipt_support_channels)
            .bind(seed.has_api_access.unwrap_or(false))
            .bind(&seed.notes)
            .bind(seed.country.as_deref().unwrap_or("SE"))
            .execute(self.pool)
            .await
            .map_err(|e| anyhow!("seed insert failed for {}: {e}", seed.merchant_id))?
            .rows_affected();

            inserted += rows_affected as usize;
        }

        Ok(inserted)
    }
}
