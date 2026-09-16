//! Domain modules — pure business rules and calculations.
//!
//! Domain functions must have no dependencies on Tauri, SQLite, or Svelte.
//! They are the primary target for fast unit tests.

pub mod expiry_status;
pub mod lot_movements;
pub mod lot_resolution;
pub mod validation;
