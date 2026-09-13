//! Application-wide shared state registered with Tauri.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::db::DbPool;

/// Holds all application-level dependencies that live for the lifetime of the app.
#[derive(Clone)]
pub struct AppState {
    /// The active database connection pool. Replaced atomically on restore.
    pub pool: Arc<Mutex<DbPool>>,
    /// The absolute path to the active SQLite database file.
    pub db_path: PathBuf,
}

impl AppState {
    pub fn new(pool: Arc<Mutex<DbPool>>, db_path: PathBuf) -> Self {
        Self { pool, db_path }
    }

    /// Ping the database to verify the connection is alive.
    pub async fn ping_db(&self) -> Result<bool, sqlx::Error> {
        let pool = self.pool.lock().await;
        sqlx::query("SELECT 1").fetch_one(&*pool).await?;
        Ok(true)
    }

    /// Returns the absolute path to the active SQLite database file.
    pub fn pool_path(&self) -> std::path::PathBuf {
        self.db_path.clone()
    }

    /// Acquires a lock on the pool and returns a reference for the duration of the
    /// call site. Commands use this to pass `&DbPool` to services.
    pub async fn pool(&self) -> tokio::sync::MutexGuard<'_, DbPool> {
        self.pool.lock().await
    }
}
