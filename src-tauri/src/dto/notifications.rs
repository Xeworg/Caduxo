//! DTOs for local notification candidates and `notification_log` rows.
//!
//! These types are the IPC boundary for the notification query and the
//! "mark shown" command. They mirror the SQL projection used by the repository
//! and the `notification_log` table shape.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single lot that is a candidate for a local OS notification on `today`.
///
/// Fields are chosen for the notification payload: product identity
/// (`sku`, `description`), store identity (`store_name`), location
/// (optional `location_name`), quantity/unit, and the alert window metadata.
/// `notification_date` is the day the notification is being computed for
/// (passed in by the caller — usually today).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DueNotificationLot {
    pub lot_id: String,
    pub product_id: String,
    pub sku: String,
    pub description: String,
    pub store_id: String,
    pub store_name: String,
    pub location_id: Option<String>,
    pub location_name: Option<String>,
    pub quantity: f64,
    pub unit: String,
    pub expiry_date: String,
    pub alert_days_before: i32,
    pub notification_date: String,
}

/// One row from the `notification_log` dedup table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationLogResponse {
    pub id: String,
    pub expiry_lot_id: String,
    pub notification_date: String,
    pub shown_at: String,
}

/// Input for `mark_notification_shown`.
///
/// `notification_date` is `Option<String>` so the frontend may pass an explicit
/// date (e.g. when back-filling missed notifications) or `None` to use today.
/// When provided, it must be a strict YYYY-MM-DD string.
#[derive(Debug, Clone, Deserialize)]
pub struct MarkNotificationShownInput {
    pub expiry_lot_id: String,
    /// Optional ISO date (YYYY-MM-DD). Defaults to today (UTC) when omitted.
    pub notification_date: Option<String>,
}
