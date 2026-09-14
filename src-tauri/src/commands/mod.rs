//! Tauri command adapters — thin IPC boundary only.
//!
//! Each command module exposes one group of related commands.
//! Business logic lives in `../services/` and `../domain/`.

pub mod backup_restore;
pub mod csv_io;
pub mod dashboard;
pub mod expiry_lots;
pub mod health;
pub mod notifications;
pub mod products;
pub mod reports;
pub mod stores;
pub mod unit_definitions;
