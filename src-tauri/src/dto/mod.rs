//! Data Transfer Objects — command input/output structs.
//!
//! These types define the Tauri IPC boundary. They are serialized as JSON
//! and must not contain internal-only types (e.g. raw sqlx rows).

pub mod csv_io;
pub mod dashboard;
pub mod expiry_lots;
pub mod notifications;
pub mod products;
pub mod reports;
pub mod scanner;
pub mod stores;
