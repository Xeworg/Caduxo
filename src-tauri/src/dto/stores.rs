//! DTOs for store and internal location management.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Input for creating a new store.
#[derive(Debug, Deserialize)]
pub struct StoreCreate {
    pub name: String,
    pub code: Option<String>,
    pub notes: Option<String>,
}

/// Input for updating an existing store.
#[derive(Debug, Deserialize)]
pub struct StoreUpdate {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
}

/// Response shape for a store.
#[derive(Debug, Serialize, FromRow)]
pub struct StoreResponse {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a new internal location under a store.
#[derive(Debug, Deserialize)]
pub struct StoreLocationCreate {
    pub store_id: String,
    pub name: String,
    pub notes: Option<String>,
}

/// Input for updating an existing internal location.
#[derive(Debug, Deserialize)]
pub struct StoreLocationUpdate {
    pub id: String,
    pub store_id: String,
    pub name: String,
    pub notes: Option<String>,
    pub is_active: bool,
}

/// Response shape for an internal location.
#[derive(Debug, Serialize, FromRow)]
pub struct StoreLocationResponse {
    pub id: String,
    pub store_id: String,
    pub name: String,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Application settings snapshot.
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub last_selected_store_id: Option<String>,
    /// When true, lot creation requires a location to be chosen.
    /// When false, an empty location picker uses the sentinel "Sin ubicacion".
    pub require_initial_location_on_lot_create: bool,
    /// Effective locale; one of {"en", "es"}. Falls back to "en" when no row
    /// is persisted so the type stays a non-nullable string for consumers.
    pub language: String,
    /// True only when an `app_settings.language` row was actually persisted.
    /// False on fresh installs lets the frontend call OS / WebView detection
    /// instead of treating the `"en"` fallback as a manual user preference.
    pub language_configured: bool,
}

/// Input for updating settings.
#[derive(Debug, Deserialize)]
pub struct SettingsUpdate {
    pub last_selected_store_id: Option<String>,
    /// Optional: toggles the require_initial_location_on_lot_create setting.
    pub require_initial_location_on_lot_create: Option<bool>,
    /// Optional: when present, sets the language preference to "en" or "es".
    pub language: Option<String>,
}
