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

/// One (location, quantity) pair within a distributed lot creation.
///
/// Allocations are applied atomically by
/// [`crate::services::expiry_lots::create_expiry_lot_distributed`]. The
/// service rejects empty lists, duplicate `location_id`s, quantities
/// `<= 0`, locations that are not part of the lot's `store_id`, and
/// inactive locations.
#[derive(Debug, Deserialize)]
pub struct ExpiryLotAllocationInput {
    /// The location to receive this slice of the lot. Must be an active
    /// location belonging to the lot's `store_id`.
    pub location_id: String,
    /// Quantity to assign to this location. Must be `> 0`; for
    /// integer-unit products must be a whole number; the sum of all
    /// allocations must equal the lot total.
    pub quantity: f64,
}

/// Input for creating one expiry lot whose initial stock is distributed
/// across multiple active locations in the same store.
///
/// Unlike [`ExpiryLotCreate`], this shape does **not** carry a single
/// `location_id`. The atomic backend flow writes one `entry:initial`
/// movement for the lot total to the first allocation's location, then
/// one `transfer` movement per remaining allocation, all in the same DB
/// transaction. The first allocation's location is recorded as the
/// lot's anchor `location_id` for backward compatibility with existing
/// listings; the per-location ledger (`lot_movements`) is the source of
/// truth for every allocation's balance.
///
/// The `total_quantity` field is the explicit expected lot total. The
/// service validates that the sum of allocation quantities equals it
/// (within [`DISTRIBUTED_TOTAL_EQ_TOLERANCE`]) so the caller can detect
/// under- or over-allocation before submission.
#[derive(Debug, Deserialize)]
pub struct ExpiryLotDistributedCreate {
    /// The product this lot belongs to.
    pub product_id: String,
    /// The store that owns every allocation's `location_id`.
    pub store_id: String,
    /// Explicit lot total. Must equal the sum of allocation quantities
    /// (within a tiny float tolerance); otherwise the request is rejected
    /// with `DistributionTotalMismatch`.
    pub total_quantity: f64,
    /// Non-empty list of `(location_id, quantity)` pairs. The sum of
    /// quantities must equal `total_quantity` exactly (no silent
    /// rounding).
    pub allocations: Vec<ExpiryLotAllocationInput>,
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
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ExpiryLotResponse {
    pub id: String,
    pub product_id: String,
    pub store_id: String,
    pub location_id: Option<String>,
    /// Human-readable name of the lot's store location, projected from
    /// `store_locations.name` via `LEFT JOIN`. `None` when the lot has no
    /// `location_id` or the referenced location row is missing. Inactive
    /// locations are included so historic lots remain interpretable.
    pub location_name: Option<String>,
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

/// Result of a distributed lot creation. Carries the persisted lot, the
/// anchor location (the first allocation's `location_id`), and the total
/// number of allocations written. The frontend can derive per-location
/// balances from `lot_movements` via the existing
/// `get_lot_location_balances` command.
#[derive(Debug, Serialize)]
pub struct ExpiryLotDistributedCreateResult {
    pub lot: ExpiryLotResponse,
    /// Location that received the single `entry:initial` movement for the
    /// full lot total. Persisted as the lot's anchor `location_id`.
    pub anchor_location_id: String,
    /// Total number of allocations applied (1 or more).
    pub allocation_count: usize,
}
