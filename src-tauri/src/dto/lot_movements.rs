//! DTOs for lot movement management: create, list, and per-location balances.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================
// Movement kind and direction
// ============================================================

/// Direction for inventory_adjustment movements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Increase,
    Decrease,
}

// ============================================================
// Movement — inputs
// ============================================================

/// Input for creating a new lot movement.
#[derive(Debug, Deserialize)]
pub struct LotMovementCreate {
    /// The lot to record the movement against.
    pub lot_id: String,
    /// Movement kind (e.g., "transfer", "exit:sale", "inventory_adjustment").
    pub kind: String,
    /// Optional direction for `inventory_adjustment` kind.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Quantity being moved (always positive magnitude).
    pub quantity: f64,
    /// Source location for exits and transfers.
    #[serde(default)]
    pub source_location_id: Option<String>,
    /// Destination location for entries and transfers.
    #[serde(default)]
    pub destination_location_id: Option<String>,
    /// Optional notes (required for some kinds).
    #[serde(default)]
    pub notes: Option<String>,
}

// ============================================================
// Movement — responses
// ============================================================

/// Full movement response row.
#[derive(Debug, Serialize, FromRow)]
pub struct LotMovementResponse {
    pub id: String,
    pub expiry_lot_id: String,
    pub movement_kind: String,
    pub direction: Option<String>,
    pub quantity: f64,
    pub source_location_id: Option<String>,
    pub destination_location_id: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub actor: String,
    pub created_at: String,
}

/// Per-location balance for a lot.
#[derive(Debug, Serialize, FromRow)]
pub struct LotLocationBalance {
    pub location_id: String,
    pub balance: f64,
}
