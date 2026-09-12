//! Application services — use-case orchestration.
//!
//! Services call domain functions and repositories. They handle transactions,
//! logging points, and business error selection. No direct SQL or UI code.

pub mod csv_io;
pub mod dashboard;
pub mod expiry_lots;
pub mod health;
pub mod notifications;
pub mod products;
pub mod settings;
pub mod stores;
