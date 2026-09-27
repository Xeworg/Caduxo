//! Data access for the stock-out reasons catalog.
//!
//! All SQL for the `stock_out_reasons` table lives here.

#![allow(dead_code)]

use crate::db::DbPool;
use crate::dto::stock_out_reasons::StockOutReasonResponse;
use sqlx::FromRow;

/// Internal row shape from the `stock_out_reasons` table.
#[derive(Debug, FromRow)]
pub(super) struct StockOutReasonRow {
    pub id: String,
    pub display_name: String,
    pub movement_kind: String,
    pub sort_order: i32,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl StockOutReasonRow {
    fn into_response(self) -> StockOutReasonResponse {
        StockOutReasonResponse {
            id: self.id,
            display_name: self.display_name,
            movement_kind: self.movement_kind,
            sort_order: self.sort_order,
            archived_at: self.archived_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Lists all active (non-archived) reasons ordered by sort_order.
pub async fn list_active(pool: &DbPool) -> Result<Vec<StockOutReasonResponse>, sqlx::Error> {
    let rows: Vec<StockOutReasonRow> = sqlx::query_as(
        "SELECT id, display_name, movement_kind, sort_order, archived_at, created_at, updated_at
         FROM stock_out_reasons
         WHERE archived_at IS NULL
         ORDER BY sort_order, display_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_response()).collect())
}

/// Lists all reasons including archived ones, ordered by sort_order.
pub async fn list_all(pool: &DbPool) -> Result<Vec<StockOutReasonResponse>, sqlx::Error> {
    let rows: Vec<StockOutReasonRow> = sqlx::query_as(
        "SELECT id, display_name, movement_kind, sort_order, archived_at, created_at, updated_at
         FROM stock_out_reasons
         ORDER BY sort_order, display_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_response()).collect())
}

/// Finds a reason by its `id`. Returns `None` if not found (regardless of archived status).
pub async fn find_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<StockOutReasonResponse>, sqlx::Error> {
    let row: Option<StockOutReasonRow> = sqlx::query_as(
        "SELECT id, display_name, movement_kind, sort_order, archived_at, created_at, updated_at
         FROM stock_out_reasons
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_response()))
}

/// Finds an active reason by its display name (case-insensitive).
/// Returns `None` if not found or if the reason is archived.
pub async fn find_active_by_display_name_ci(
    pool: &DbPool,
    display_name: &str,
) -> Result<Option<StockOutReasonResponse>, sqlx::Error> {
    let row: Option<StockOutReasonRow> = sqlx::query_as(
        "SELECT id, display_name, movement_kind, sort_order, archived_at, created_at, updated_at
         FROM stock_out_reasons
         WHERE archived_at IS NULL AND lower(display_name) = lower($1)
         LIMIT 1",
    )
    .bind(display_name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_response()))
}

/// Inserts a new stock-out reason. Caller is responsible for validation and
/// duplicate-display-name check before insertion.
pub async fn insert(
    pool: &DbPool,
    id: &str,
    display_name: &str,
    movement_kind: &str,
    sort_order: i32,
    created_at: &str,
    updated_at: &str,
) -> Result<StockOutReasonResponse, sqlx::Error> {
    let row: StockOutReasonRow = sqlx::query_as(
        "INSERT INTO stock_out_reasons
             (id, display_name, movement_kind, sort_order, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, display_name, movement_kind, sort_order, archived_at, created_at, updated_at",
    )
    .bind(id)
    .bind(display_name)
    .bind(movement_kind)
    .bind(sort_order)
    .bind(created_at)
    .bind(updated_at)
    .fetch_one(pool)
    .await?;

    Ok(row.into_response())
}

/// Updates the `display_name` of an existing reason.
/// The `movement_kind` is immutable and NOT updated here.
pub async fn update_display_name(
    pool: &DbPool,
    id: &str,
    display_name: &str,
    updated_at: &str,
) -> Result<Option<StockOutReasonResponse>, sqlx::Error> {
    let row: Option<StockOutReasonRow> = sqlx::query_as(
        "UPDATE stock_out_reasons
         SET display_name = $2, updated_at = $3
         WHERE id = $1
         RETURNING id, display_name, movement_kind, sort_order, archived_at, created_at, updated_at",
    )
    .bind(id)
    .bind(display_name)
    .bind(updated_at)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_response()))
}

/// Soft-archives a reason by setting `archived_at` to the current timestamp.
/// Returns `true` if a row was updated.
pub async fn archive(pool: &DbPool, id: &str, archived_at: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE stock_out_reasons
         SET archived_at = $2, updated_at = $2
         WHERE id = $1 AND archived_at IS NULL",
    )
    .bind(id)
    .bind(archived_at)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Restores an archived reason by clearing `archived_at`.
/// Returns `true` if a row was updated.
pub async fn unarchive(pool: &DbPool, id: &str, updated_at: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE stock_out_reasons
         SET archived_at = NULL, updated_at = $2
         WHERE id = $1 AND archived_at IS NOT NULL",
    )
    .bind(id)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Returns `true` if any lot_movement references this reason via `exit_reason_id`.
pub async fn is_referenced(pool: &DbPool, reason_id: &str) -> Result<bool, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM lot_movements WHERE exit_reason_id = $1")
            .bind(reason_id)
            .fetch_one(pool)
            .await?;

    Ok(row.0 > 0)
}

/// Returns the maximum `sort_order` for a given movement kind (active reasons only).
/// Used to derive a default sort_order for new reasons.
pub async fn max_sort_order_for_kind(
    pool: &DbPool,
    movement_kind: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let row: Option<(i32,)> = sqlx::query_as(
        "SELECT MAX(sort_order) FROM stock_out_reasons
             WHERE movement_kind = $1 AND archived_at IS NULL",
    )
    .bind(movement_kind)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(v,)| v))
}
