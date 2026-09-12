//! Tauri command adapters — thin IPC boundary only.
//!
//! Each command module exposes one group of related commands.
//! Business logic lives in `../services/` and `../domain/`.

pub mod health;
