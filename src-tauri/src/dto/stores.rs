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
}

/// Input for updating settings.
#[derive(Debug, Deserialize)]
pub struct SettingsUpdate {
    pub last_selected_store_id: Option<String>,
}
