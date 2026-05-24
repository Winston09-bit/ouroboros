// =============================================================================
// src/db/connection.rs
// Lightweight pool factory + migration runner + health probe
// =============================================================================

use std::time::Duration;

use anyhow::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Build a `PgPool` with sane defaults.
///
/// `database_url` must be a valid `postgresql://` connection string.
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .map_err(anyhow::Error::from)
}

/// Apply all pending migrations from the embedded `migrations/` directory.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// Returns `true` if the database responds to a trivial query.
pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}
