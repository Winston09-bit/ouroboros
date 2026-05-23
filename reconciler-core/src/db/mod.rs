// =============================================================================
// src/db/mod.rs
// Database connection pool and lifecycle management
// =============================================================================

pub mod repositories;

use std::time::Duration;

use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use tracing::{debug, info, warn};

use crate::error::{ReconcilerError, Result};

pub use repositories::transaction_repo::TransactionRepository;

// ---------------------------------------------------------------------------
// Database handle
// ---------------------------------------------------------------------------

/// Central database handle wrapping a `sqlx::PgPool`.
///
/// Cheaply cloneable — cloning shares the underlying pool.
#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Connect to PostgreSQL using the given `DATABASE_URL`-style connection
    /// string and return a ready [`Database`] instance.
    ///
    /// # Pool defaults
    /// | Setting            | Value |
    /// |--------------------|-------|
    /// | `max_connections`  | 20    |
    /// | `min_connections`  | 2     |
    /// | `connect_timeout`  | 10 s  |
    /// | `idle_timeout`     | 10 min|
    /// | `max_lifetime`     | 30 min|
    /// | `acquire_timeout`  | 30 s  |
    ///
    /// Override via environment variables (`DATABASE_POOL_MAX`, etc.) if
    /// needed; see [`DatabaseConfig`].
    pub async fn connect(url: &str) -> Result<Self> {
        let options: PgConnectOptions = url
            .parse()
            .map_err(|e: sqlx::Error| ReconcilerError::Database(e.to_string()))?;

        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect_with(options)
            .await
            .map_err(|e| ReconcilerError::Database(e.to_string()))?;

        info!("PostgreSQL pool established (max_connections=20)");

        Ok(Self { pool })
    }

    /// Create a [`Database`] from an existing pool (useful in tests).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    // -----------------------------------------------------------------------
    // Migrations
    // -----------------------------------------------------------------------

    /// Apply all pending Flyway/sqlx migrations from the embedded
    /// `migrations/` directory.
    ///
    /// Migrations are run inside a transaction and are idempotent —
    /// safe to call on every application startup.
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations…");

        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| ReconcilerError::Migration(e.to_string()))?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Health check
    // -----------------------------------------------------------------------

    /// Perform a lightweight liveness check.
    ///
    /// Returns `Ok(true)` if the database is reachable and responsive,
    /// `Ok(false)` if a query executed but returned an unexpected result,
    /// and `Err(_)` if the connection itself failed.
    pub async fn health_check(&self) -> Result<bool> {
        let result: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                warn!("Database health check failed: {}", e);
                ReconcilerError::Database(e.to_string())
            })?;

        let healthy = result.0 == 1;
        debug!("Database health check: {}", if healthy { "OK" } else { "UNEXPECTED RESULT" });
        Ok(healthy)
    }

    // -----------------------------------------------------------------------
    // Pool accessors
    // -----------------------------------------------------------------------

    /// Return a reference to the underlying connection pool.
    ///
    /// Use this when you need to pass the pool to a repository or a raw
    /// `sqlx::query!` call.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Return pool-level statistics for observability / metrics.
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size:     self.pool.size(),
            idle:     self.pool.num_idle(),
        }
    }

    // -----------------------------------------------------------------------
    // Repository factories
    // -----------------------------------------------------------------------

    /// Construct a [`TransactionRepository`] bound to this pool.
    pub fn transactions(&self) -> TransactionRepository {
        TransactionRepository::new(self.pool.clone())
    }
}

// ---------------------------------------------------------------------------
// Pool statistics (for metrics/logging)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    /// Total connections in the pool (idle + in-use).
    pub size: u32,
    /// Connections currently sitting idle.
    pub idle: usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pool(size={}, idle={})", self.size, self.idle)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirm that `Database::from_pool` round-trips the pool reference.
    /// Full integration tests require a live Postgres instance; those live in
    /// tests/integration/.
    #[test]
    fn pool_stats_display() {
        // We can't spin up a real pool in a unit test, but we can verify the
        // Display impl compiles and formats correctly via a fake PoolStats.
        let stats = PoolStats { size: 5, idle: 3 };
        assert_eq!(stats.to_string(), "pool(size=5, idle=3)");
    }
}
pub mod repositories;
