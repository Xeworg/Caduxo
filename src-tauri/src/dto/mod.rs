//! Data Transfer Objects — command input/output structs.
//!
//! These types define the Tauri IPC boundary. They are serialized as JSON
//! and must not contain internal-only types (e.g. raw sqlx rows).

pub mod stores;
