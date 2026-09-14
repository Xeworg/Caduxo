//! Data access for the unit definitions catalog.
//!
//! All SQL for the `unit_definitions` table lives here.

#![allow(dead_code)]

use crate::db::DbPool;
use crate::dto::unit_definitions::{UnitDefinitionResponse, UnitKind};
use sqlx::FromRow;

/// Internal row shape from the `unit_definitions` table.
#[derive(Debug, FromRow)]
pub(super) struct UnitDefinitionRow {
    pub id: String,
    pub key: String,
    pub display_name: String,
    pub kind: String,
    pub is_preset: bool,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl UnitDefinitionRow {
    fn into_response(self) -> UnitDefinitionResponse {
        UnitDefinitionResponse {
            id: self.id,
            key: self.key,
            display_name: self.display_name,
            kind: match self.kind.as_str() {
                "integer" => UnitKind::Integer,
                _ => UnitKind::Decimal,
            },
            is_preset: self.is_preset,
            archived_at: self.archived_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Lists all non-archived unit definitions ordered by kind then display_name.
pub async fn list_active(pool: &DbPool) -> Result<Vec<UnitDefinitionResponse>, sqlx::Error> {
    let rows: Vec<UnitDefinitionRow> = sqlx::query_as(
        "SELECT id, key, display_name, kind, is_preset, archived_at, created_at, updated_at
         FROM unit_definitions
         WHERE archived_at IS NULL
         ORDER BY kind, display_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_response()).collect())
}

/// Finds a unit by its `key` (case-insensitive). Returns None if not found or archived.
pub async fn find_by_key(
    pool: &DbPool,
    key: &str,
) -> Result<Option<UnitDefinitionResponse>, sqlx::Error> {
    let row: Option<UnitDefinitionRow> = sqlx::query_as(
        "SELECT id, key, display_name, kind, is_preset, archived_at, created_at, updated_at
         FROM unit_definitions
         WHERE lower(key) = lower($1) AND archived_at IS NULL",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_response()))
}

/// Finds a unit by its `id`. Returns None if not found or archived.
pub async fn find_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<UnitDefinitionResponse>, sqlx::Error> {
    let row: Option<UnitDefinitionRow> = sqlx::query_as(
        "SELECT id, key, display_name, kind, is_preset, archived_at, created_at, updated_at
         FROM unit_definitions
         WHERE id = $1 AND archived_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_response()))
}

/// Inserts a new unit definition. Returns the inserted row.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &DbPool,
    id: &str,
    key: &str,
    display_name: &str,
    kind: UnitKind,
    is_preset: bool,
    created_at: &str,
    updated_at: &str,
) -> Result<UnitDefinitionResponse, sqlx::Error> {
    let kind_str = match kind {
        UnitKind::Integer => "integer",
        UnitKind::Decimal => "decimal",
    };

    let row: UnitDefinitionRow = sqlx::query_as(
        "INSERT INTO unit_definitions (id, key, display_name, kind, is_preset, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, key, display_name, kind, is_preset, archived_at, created_at, updated_at",
    )
    .bind(id)
    .bind(key)
    .bind(display_name)
    .bind(kind_str)
    .bind(is_preset)
    .bind(created_at)
    .bind(updated_at)
    .fetch_one(pool)
    .await?;

    Ok(row.into_response())
}

/// Updates the `display_name` of an existing unit. The `key` is immutable in this slice.
pub async fn rename(
    pool: &DbPool,
    id: &str,
    display_name: &str,
    updated_at: &str,
) -> Result<Option<UnitDefinitionResponse>, sqlx::Error> {
    let row: Option<UnitDefinitionRow> = sqlx::query_as(
        "UPDATE unit_definitions
         SET display_name = $2, updated_at = $3
         WHERE id = $1 AND archived_at IS NULL
         RETURNING id, key, display_name, kind, is_preset, archived_at, created_at, updated_at",
    )
    .bind(id)
    .bind(display_name)
    .bind(updated_at)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_response()))
}

/// Soft-archives a unit by setting `archived_at` to the current timestamp.
pub async fn archive(pool: &DbPool, id: &str, archived_at: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE unit_definitions SET archived_at = $2, updated_at = $2
         WHERE id = $1 AND archived_at IS NULL",
    )
    .bind(id)
    .bind(archived_at)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Returns true if any product references this unit via `default_unit_id`.
pub async fn is_referenced(pool: &DbPool, unit_id: &str) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products WHERE default_unit_id = $1")
        .bind(unit_id)
        .fetch_one(pool)
        .await?;

    Ok(row.0 > 0)
}

/// Suggests up to `limit` similar unit keys for the CSV preview badge.
/// Strategy: case-insensitive prefix match first, then contains match.
pub async fn suggest_similar(
    pool: &DbPool,
    raw: &str,
    limit: usize,
) -> Result<Vec<UnitDefinitionResponse>, sqlx::Error> {
    let raw_lower = raw.to_lowercase();
    let prefix = format!("{}%", raw_lower);

    // Try prefix match first.
    let rows: Vec<UnitDefinitionRow> = sqlx::query_as(
        "SELECT id, key, display_name, kind, is_preset, archived_at, created_at, updated_at
         FROM unit_definitions
         WHERE archived_at IS NULL
           AND (lower(key) LIKE $1 OR lower(display_name) LIKE $1)
         ORDER BY key
         LIMIT $2",
    )
    .bind(&prefix)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    if rows.len() >= limit {
        return Ok(rows.into_iter().map(|r| r.into_response()).collect());
    }

    // Fall back to contains match (excluding prefix matches already found).
    let contains = format!("%{}%", raw_lower);
    let rows2: Vec<UnitDefinitionRow> = sqlx::query_as(
        "SELECT id, key, display_name, kind, is_preset, archived_at, created_at, updated_at
         FROM unit_definitions
         WHERE archived_at IS NULL
           AND lower(key) LIKE $1
           AND lower(key) NOT LIKE $2
         ORDER BY key
         LIMIT $3",
    )
    .bind(&contains)
    .bind(&prefix)
    .bind((limit - rows.len()) as i64)
    .fetch_all(pool)
    .await?;

    let mut all: Vec<_> = rows;
    all.extend(rows2);
    Ok(all.into_iter().map(|r| r.into_response()).collect())
}

/// Returns all non-archived presets ordered by kind then display_name.
pub async fn list_presets(pool: &DbPool) -> Result<Vec<UnitDefinitionResponse>, sqlx::Error> {
    let rows: Vec<UnitDefinitionRow> = sqlx::query_as(
        "SELECT id, key, display_name, kind, is_preset, archived_at, created_at, updated_at
         FROM unit_definitions
         WHERE archived_at IS NULL AND is_preset = 1
         ORDER BY kind, display_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_response()).collect())
}

/// Returns the preset unit with key 'units' (the integer fallback for legacy lots).
pub async fn find_units_preset(
    pool: &DbPool,
) -> Result<Option<UnitDefinitionResponse>, sqlx::Error> {
    find_by_key(pool, "units").await
}
