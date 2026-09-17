//! Application services — use-case orchestration.
//!
//! Services call domain functions and repositories. They handle transactions,
//! logging points, and business error selection. No direct SQL or UI code.

pub mod backup_restore;
pub mod categories;
pub mod csv_io;
pub mod dashboard;
pub mod expiry_lots;
pub mod lot_movements;
pub mod notifications;
pub mod products;
pub mod reports;
pub mod settings;
pub mod stores;
pub mod unit_audit;
pub mod unit_definitions;
