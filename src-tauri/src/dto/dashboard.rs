//! DTOs for the dashboard query: urgency counts, filtered lot rows,
//! and the row-level detail needed by the urgency cards and lot table.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Optional filters for the dashboard lot query.
#[derive(Debug, Default, Deserialize)]
pub struct DashboardFilters {
    /// When provided, return only lots in this store.
    pub store_id: Option<String>,
    /// When provided, return only lots in this location.
    pub location_id: Option<String>,
    /// Quick-filter preset. Mutually exclusive with `urgency` below.
    pub preset: Option<DashboardPreset>,
    /// Explicit urgency filter (overrides `preset` when set).
    pub urgency: Option<String>,
}

/// Quick-filter preset that maps to an urgency-based subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardPreset {
    /// Expired lots only.
    Expired,
    /// Lots expiring today only.
    Today,
    /// Lots in their alert window (not expired, not today).
    AlertWindow,
    /// Lots expiring in the next 7 calendar days.
    #[serde(rename = "next_7_days")]
    Next7Days,
    /// Lots expiring in the next 30 calendar days (alert window already covered by AlertWindow).
    #[serde(rename = "next_30_days")]
    Next30Days,
    /// All active lots — no filter.
    All,
}

/// Urgency bucket counts shown on the urgency cards.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrgencyCounts {
    pub expired: u32,
    pub today: u32,
    pub alert_window: u32,
    pub next_30_days: u32,
}

/// A single row in the dashboard lot table.
///
/// Fields are chosen to be useful for display without leaking sensitive
/// product data into logs: we include `sku` and `description` for display
/// but they will not be logged by the command/service layers.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DashboardLotRow {
    pub lot_id: String,
    pub product_id: String,
    /// SKU for display.
    pub sku: String,
    /// Product description for display.
    pub description: String,
    pub store_id: String,
    pub store_name: String,
    pub location_id: Option<String>,
    pub location_name: Option<String>,
    pub quantity: f64,
    pub unit: String,
    pub expiry_date: String,
    pub alert_days_before: i32,
    pub batch_code: Option<String>,
    pub status: String,
    pub urgency: String,
    pub days_remaining: i64,
}

/// Full dashboard response bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub counts: UrgencyCounts,
    pub lots: Vec<DashboardLotRow>,
}
