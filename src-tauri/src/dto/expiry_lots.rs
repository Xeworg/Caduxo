//! DTOs for expiry lot management: create, update, resolve, and list.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::unit_definitions::UnitKind;

// ============================================================
// Expiry Lot — inputs
// ============================================================

/// Input for creating a new expiry lot. Product and store are required.
/// Unit and alert_days are pre-filled from the product default when omitted
/// (blank/None).
#[derive(Debug, Deserialize)]
pub struct ExpiryLotCreate {
    /// The product this lot belongs to.
    pub product_id: String,
    /// The store where this lot is stored.
    pub store_id: String,
    /// Optional internal location within the store.
    pub location_id: Option<String>,
    /// Number of units in this lot. Must be > 0.
    pub quantity: f64,
    /// Unit of measure (e.g. "L", "kg", "pcs"). Pre-filled from product
    /// `default_unit` when omitted (None or blank string in JSON).
    pub unit: Option<String>,
    /// Best-before / expiry date as an ISO-8601 date string (YYYY-MM-DD).
    pub expiry_date: String,
    /// Days before expiry to start alerting. Pre-filled from product
    /// `default_alert_days_before` when omitted (None or < 0).
    pub alert_days_before: Option<i32>,
    /// Optional batch code for internal tracking.
    pub batch_code: Option<String>,
    /// Optional free-text notes.
    pub notes: Option<String>,
}

/// Input for updating an existing expiry lot.
#[derive(Debug, Deserialize)]
pub struct ExpiryLotUpdate {
    pub id: String,
    /// Optional location override.
    pub location_id: Option<String>,
    /// Updated quantity. Must be > 0.
    pub quantity: f64,
    /// Unit of measure.
    pub unit: String,
    /// Updated expiry date.
    pub expiry_date: String,
    /// Updated alert days-before.
    pub alert_days_before: i32,
    /// Optional batch code.
    pub batch_code: Option<String>,
    /// Optional notes.
    pub notes: Option<String>,
}

/// Input for resolving (consuming / discarding) a quantity from a lot.
#[derive(Debug, Deserialize)]
pub struct ExpiryLotResolve {
    /// The lot to resolve quantity from.
    pub lot_id: String,
    /// Quantity to resolve. Must be > 0 and <= remaining quantity.
    pub quantity: f64,
    /// Resolution type: "consumed", "discarded", or "transferred".
    pub resolution: String,
    /// Optional notes about this resolution event.
    pub notes: Option<String>,
}

/// Input for archiving an active expiry lot with a required justification.
///
/// `reason` must be one of the allow-listed archive reason codes
/// (`expired_unsold`, `damaged`, `returned_to_supplier`, `recall`, `lost`,
/// `internal_use`, `administrative`, `other`). `notes` is trimmed and must be
/// at least 5 characters and at most 1000 characters.
#[derive(Debug, Deserialize)]
pub struct ArchiveLotInput {
    /// The lot id to archive. Must point to a lot whose status is `active`.
    pub id: String,
    /// Allow-listed reason code explaining why the lot is being archived.
    pub reason: String,
    /// Free-text justification. Validated server-side: trimmed length must be
    /// at least 5 characters and at most 1000 characters.
    pub notes: String,
}

// ============================================================
// Expiry Lot — responses
// ============================================================

/// Full expiry lot response row.
#[derive(Debug, Serialize, FromRow)]
pub struct ExpiryLotResponse {
    pub id: String,
    pub product_id: String,
    pub store_id: String,
    pub location_id: Option<String>,
    pub quantity: f64,
    pub unit: String,
    pub expiry_date: String,
    pub alert_days_before: i32,
    pub batch_code: Option<String>,
    pub status: String,
    pub resolution: Option<String>,
    pub resolved_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Unit kind from the product's catalog link (`integer` or `decimal`).
    /// `None` for legacy/uncatalogued products — treated as decimal by the
    /// backend. Propagated to the UI so modals can apply unit-aware input
    /// rules (min/step/validation) without an extra round-trip.
    pub unit_type: Option<UnitKind>,
}

/// Resolution event response row.
#[derive(Debug, Serialize, FromRow)]
pub struct LotResolutionEventResponse {
    pub id: String,
    pub expiry_lot_id: String,
    pub quantity: f64,
    pub resolution: String,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Result of a partial resolution: remaining quantity and whether the lot
/// is now fully resolved.
#[derive(Debug, Serialize)]
pub struct ExpiryLotResolveResult {
    pub lot_id: String,
    pub resolved_quantity: f64,
    pub remaining_quantity: f64,
    pub is_fully_resolved: bool,
    pub resolution_event_id: String,
}
