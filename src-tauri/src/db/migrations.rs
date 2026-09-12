//! Database migration definitions and runner.
//!
//! Migrations are embedded as inline SQL strings and applied sequentially.
//! The `_sqlx_migrations` tracking table (created by sqlx on first run)
//! records which migrations have been applied.
//!
//! ## Safety
//!
//! All migrations should be idempotent for the "already-applied" path.
//! Each migration runs in its own transaction and is rolled back on failure.

use sqlx::migrate::{Migration, MigrationType, Migrator};
use std::borrow::Cow;
use std::path::Path;

use super::pool::{pool_options, sqlite_options, DbPool};

/// All application migrations in order.
///
/// Each entry is `(version, description, SQL)`.
const MIGRATIONS: &[(i64, &str, &str)] = &[
    // V1 — smoke-test: create a marker table. This confirms the DB and
    // migrations subsystem are working before any real schema is added.
    (
        1,
        "create_schema_skeleton",
        r#"
        CREATE TABLE IF NOT EXISTS _caduxo_skeleton (
            id INTEGER PRIMARY KEY,
            marker TEXT NOT NULL DEFAULT 'caduxo-placeholder'
        );
        INSERT OR IGNORE INTO _caduxo_skeleton (id, marker) VALUES (1, 'caduxo-placeholder');
        "#,
    ),
];

/// Returns a `Migrator` built from the inline `MIGRATIONS` constant.
///
/// This avoids compile-time embedding path issues while keeping migrations
/// version-controlled alongside the source code.
fn build_migrator() -> Migrator {
    let migrations: Vec<Migration> = MIGRATIONS
        .iter()
        .map(|(version, description, sql)| {
            Migration::new(
                *version,
                Cow::Owned(description.to_string()),
                MigrationType::Simple,
                Cow::Owned(sql.to_string()),
                false, // no_tx — allow transactions
            )
        })
        .collect();

    // SAFETY: Migrator fields are semver-exempt and designed for this use.
    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

/// Opens a new SQLite pool connecting to `db_path`.
pub async fn open_pool(db_path: &Path) -> Result<DbPool, sqlx::Error> {
    let opts = sqlite_options(db_path);
    let pool_opts = pool_options();
    pool_opts.connect_with(opts).await
}

/// Runs all pending migrations on `pool`.
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::Error> {
    let migrator = build_migrator();
    let count = migrator.iter().count() as u32;
    migrator.run(pool).await?;
    tracing::info!(total_migrations = count, "Database migrations applied");
    Ok(())
}

/// Returns the count of applied migrations in the tracking table.
pub async fn applied_count(pool: &DbPool) -> Result<u32, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    Ok(row.0 as u32)
}

/// Smoke-test helper: creates a fresh in-memory SQLite pool and runs all
/// migrations against it.
#[cfg(test)]
pub async fn fresh_test_pool() -> Result<DbPool, sqlx::Error> {
    let pool = open_pool(std::path::Path::new(":memory:")).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}
