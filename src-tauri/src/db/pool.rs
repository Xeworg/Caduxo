//! Database connection pool configuration.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;

/// Type alias for the SQLite connection pool used throughout the application.
pub type DbPool = sqlx::Pool<sqlx::Sqlite>;

/// Returns `SqliteConnectOptions` pointing at the SQLite file at `db_path`.
pub fn sqlite_options(db_path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
}

/// Returns a configured `SqlitePoolOptions` with Caduxo defaults.
pub fn pool_options() -> SqlitePoolOptions {
    SqlitePoolOptions::new()
        .max_connections(4)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn sqlite_options_creates_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let opts = sqlite_options(&db_path);
        // Verify options are valid by connecting.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await;
        assert!(pool.is_ok());
    }
}
