//! Database module — SQLite pool, migrations, and repository boundaries.

pub mod migrations;
pub mod pool;
pub mod repositories;

// Re-export the most-used items so callers can import from `crate::db`.
pub use migrations::{open_pool, run_migrations};
pub use pool::DbPool;
