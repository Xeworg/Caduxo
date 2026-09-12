//! Tauri commands for the dashboard: urgency counts, filtered lot table.
//!
//! Thin adapters only — business logic lives in `services::dashboard`.

use tauri::State;

use crate::dto::dashboard::{DashboardFilters, DashboardResponse};
use crate::error::{AppError, CommandError};
use crate::services::dashboard as service;
use crate::state::AppState;

/// Returns urgency counts and a sorted, urgency-filtered lot table.
///
/// Filters are applied in this order:
///   1. `store_id` / `location_id` — SQL filter in the repository
///   2. `preset` or `urgency` — in-memory urgency filter after classification
///
/// Sorting is always urgency-first (expired → today → alert_window →
/// next_30_days → future), then by expiry_date ASC within each group.
#[tauri::command]
pub async fn list_dashboard_lots(
    state: State<'_, AppState>,
    filters: DashboardFilters,
) -> Result<DashboardResponse, CommandError> {
    service::get_dashboard(&state.pool, filters)
        .await
        .map_err(AppError::into)
}
