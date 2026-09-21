//! Application-wide shared state registered with Tauri.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::dto::stores::CloseBehavior;

/// Holds all application-level dependencies that live for the lifetime of the app.
#[derive(Clone)]
pub struct AppState {
    /// The active database connection pool. Replaced atomically on restore.
    pub pool: Arc<Mutex<DbPool>>,
    /// The absolute path to the active SQLite database file.
    pub db_path: PathBuf,
    /// Cached close-window behaviour read at startup and refreshed by the
    /// `update_settings` IPC command. The desktop close-window handler
    /// (`crate::lifecycle::install_close_handler`) reads this value
    /// synchronously on every `WindowEvent::CloseRequested` so a user
    /// change via ConfigurationPage takes effect on the next close
    /// without forcing the handler to await the DB pool. PR 3 of
    /// `scanner-quick-operations`.
    pub close_behavior: Arc<RwLock<CloseBehavior>>,
}

impl AppState {
    pub fn new(pool: Arc<Mutex<DbPool>>, db_path: PathBuf, close_behavior: CloseBehavior) -> Self {
        Self {
            pool,
            db_path,
            close_behavior: Arc::new(RwLock::new(close_behavior)),
        }
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

    /// Returns the cached close-window behaviour. Falls back to
    /// [`CloseBehavior::MinimizeToTray`] if the cache lock is poisoned,
    /// which would only happen after a panic in another thread that held
    /// the write guard — the safe default matches the documented default
    /// so a poisoned cache can never strand the user with a hidden
    /// process.
    pub fn close_behavior(&self) -> CloseBehavior {
        match self.close_behavior.read() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                tracing::warn!(
                    "close_behavior cache lock poisoned; reading from the recovered guard"
                );
                *poisoned.into_inner()
            }
        }
    }

    /// Replaces the cached close-window behaviour. Called by the
    /// `update_settings` IPC command after the persisted row has been
    /// updated successfully so the next close-window event observes the
    /// new behaviour without a DB round-trip.
    pub fn set_close_behavior(&self, value: CloseBehavior) {
        match self.close_behavior.write() {
            Ok(mut guard) => *guard = value,
            Err(poisoned) => {
                tracing::warn!("close_behavior cache lock poisoned; recovering the guard to write");
                *poisoned.into_inner() = value;
            }
        }
    }
}
