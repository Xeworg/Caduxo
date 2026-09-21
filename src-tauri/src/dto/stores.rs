//! DTOs for store and internal location management.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// FEFO (First-Expired, First-Out) lot-selection policy persisted in
/// `app_settings.scanner_fefo_policy`. Wire-serialised as the snake_case
/// strings the frontend expects (`suggest_fefo`, `require_fefo`,
/// `manual_lot_choice`); defaults to `SuggestFefo` when the row is absent
/// or unrecognised (mirroring the existing `language` fallback pattern).
///
/// The Scanner tab honours the policy for `ProductMatch` results only;
/// `LotMatch` results bypass FEFO because the scanned code already names
/// the lot — see `services::scanner::resolve_scanner_code` and the
/// `scanner-quick-operations` spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FefoPolicy {
    /// Default. Pre-selects the FEFO lot but lets the user override.
    SuggestFefo,
    /// Pre-selects the FEFO lot and disables the lot picker so the user
    /// cannot override while that lot has stock.
    RequireFefo,
    /// Does not pre-select a lot; the user must choose before the
    /// confirm button becomes enabled.
    ManualLotChoice,
}

impl FefoPolicy {
    /// Parses a raw persisted value into a [`FefoPolicy`]. Returns
    /// `SuggestFefo` (the documented default) when the value is unknown,
    /// absent, or whitespace, mirroring the `get_language_setting`
    /// fallback pattern.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(|s| s.trim()) {
            Some("suggest_fefo") => Self::SuggestFefo,
            Some("require_fefo") => Self::RequireFefo,
            Some("manual_lot_choice") => Self::ManualLotChoice,
            _ => Self::SuggestFefo,
        }
    }

    /// Canonical snake_case wire value the IPC boundary emits and accepts.
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::SuggestFefo => "suggest_fefo",
            Self::RequireFefo => "require_fefo",
            Self::ManualLotChoice => "manual_lot_choice",
        }
    }
}

/// Close-window behaviour persisted in `app_settings.close_behavior`.
/// Wire-serialised as the snake_case strings the frontend expects
/// (`minimize_to_tray`, `exit_application`); defaults to `MinimizeToTray`
/// when the row is absent or unrecognised.
///
/// PR 3 (`scanner-quick-operations`) wires the Tauri runtime so the
/// configured behaviour takes effect from the very first close attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    /// Default. Closes the window hide to the system tray; the process
    /// stays alive so a tray `Restore` action can show the window again.
    MinimizeToTray,
    /// Closes the window and tears down the process through the existing
    /// Tauri shutdown path; no tray icon is shown afterwards.
    ExitApplication,
}

impl CloseBehavior {
    /// Parses a raw persisted value into a [`CloseBehavior`]. Returns
    /// `MinimizeToTray` (the documented default) when the value is unknown,
    /// absent, or whitespace, mirroring the `get_language_setting`
    /// fallback pattern.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(|s| s.trim()) {
            Some("minimize_to_tray") => Self::MinimizeToTray,
            Some("exit_application") => Self::ExitApplication,
            _ => Self::MinimizeToTray,
        }
    }

    /// Canonical snake_case wire value the IPC boundary emits and accepts.
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::MinimizeToTray => "minimize_to_tray",
            Self::ExitApplication => "exit_application",
        }
    }
}

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
    /// Effective active theme; one of {"caduxo-light", "dark"}. Falls back to
    /// `"caduxo-light"` when no row is persisted so the type stays a
    /// non-nullable string for consumers. The frontend treats the fallback
    /// as "no manual preference" via `theme_configured` and runs OS detection
    /// (`prefers-color-scheme`) before locking to the fallback string.
    pub theme: String,
    /// True only when an `app_settings.theme` row was actually persisted AND
    /// its value is in the supported set (`caduxo-light` or `dark`). False on
    /// fresh installs OR when a stored row carries an unsupported value, so
    /// the frontend can run detection / fall back to the default theme
    /// instead of honouring an invalid row.
    pub theme_configured: bool,
    /// Effective FEFO lot-selection policy for the Scanner tab. Defaults to
    /// `SuggestFefo` on fresh installs (matching the spec) and when a stored
    /// row carries an unrecognised value (mirroring the `language` fallback
    /// pattern).
    pub scanner_fefo_policy: FefoPolicy,
    /// Effective close-window behaviour. Defaults to `MinimizeToTray` on
    /// fresh installs (matching the spec) and when a stored row carries an
    /// unrecognised value.
    pub close_behavior: CloseBehavior,
}

/// Input for updating settings.
#[derive(Debug, Deserialize)]
pub struct SettingsUpdate {
    pub last_selected_store_id: Option<String>,
    /// Optional: toggles the require_initial_location_on_lot_create setting.
    pub require_initial_location_on_lot_create: Option<bool>,
    /// Optional: when present, sets the language preference to "en" or "es".
    pub language: Option<String>,
    /// Optional: when present, sets the active theme preference. The command
    /// boundary rejects values outside the curated v1 set
    /// (`caduxo-light`, `dark`); the service layer accepts any non-empty
    /// string and the repository trusts it.
    pub theme: Option<String>,
    /// Optional: when present, sets the Scanner FEFO lot-selection policy.
    /// The command boundary rejects values outside the curated v1 set
    /// (`suggest_fefo`, `require_fefo`, `manual_lot_choice`); the persisted
    /// row stays untouched on rejection.
    pub scanner_fefo_policy: Option<FefoPolicy>,
    /// Optional: when present, sets the close-window behaviour. The command
    /// boundary rejects values outside the curated v1 set
    /// (`minimize_to_tray`, `exit_application`); the persisted row stays
    /// untouched on rejection.
    pub close_behavior: Option<CloseBehavior>,
}
