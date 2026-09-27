//! DTOs for the stock-out reasons catalog.
//!
//! Types here define the Tauri IPC boundary for stock-out reason management:
//! listing active/all reasons, creating custom reasons, renaming, archiving,
//! and unarchiving. No hard deletes.

use serde::{Deserialize, Serialize};

/// Full stock-out reason returned by catalog commands.
#[derive(Debug, Clone, Serialize)]
pub struct StockOutReasonResponse {
    /// Stable identifier (seeded `sor-*` or custom).
    pub id: String,
    /// User-facing label. Editable after creation.
    pub display_name: String,
    /// The V20 stock-out movement kind (e.g. `exit:waste`).
    pub movement_kind: String,
    /// Sort order within movement kind group.
    pub sort_order: i32,
    /// Null when active; timestamp when archived.
    pub archived_at: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Input for creating a new custom stock-out reason.
#[derive(Debug, Deserialize)]
pub struct StockOutReasonCreateInput {
    /// Display name for the new reason.
    pub display_name: String,
    /// The V20 stock-out movement kind (must be one of the seven allowed kinds).
    pub movement_kind: String,
    /// Optional sort order override; defaults to 0.
    #[serde(default)]
    pub sort_order: Option<i32>,
}

/// Input for renaming a stock-out reason's display_name.
/// The `movement_kind` is immutable in the update API.
#[derive(Debug, Deserialize)]
pub struct StockOutReasonRenameInput {
    /// ID of the reason to rename.
    pub id: String,
    /// New display name.
    pub display_name: String,
}
