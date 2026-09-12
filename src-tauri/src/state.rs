//! Application-wide shared state registered with Tauri.

use std::sync::Arc;

use crate::db::DbPool;

/// Holds all application-level dependencies that live for the lifetime of the app.
#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<DbPool>,
}

impl AppState {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    /// Ping the database to verify the connection is alive.
    pub async fn ping_db(&self) -> Result<bool, sqlx::Error> {
        sqlx::query("SELECT 1")
            .fetch_one(self.pool.as_ref())
            .await?;
        Ok(true)
    }
}
