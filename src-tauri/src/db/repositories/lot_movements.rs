//! Repository for lot movement persistence.
//!
//! Provides data access for the lot_movements ledger table.

use sqlx::SqlitePool;

use crate::dto::lot_movements::{LotLocationBalance, LotMovementResponse};

/// Fetches a single movement by id, or `None` if it does not exist.
pub async fn get_movement(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<LotMovementResponse>, sqlx::Error> {
    sqlx::query_as::<_, LotMovementResponse>(
        r#"
        SELECT id, expiry_lot_id, movement_kind, direction, quantity,
               source_location_id, destination_location_id, reason, notes,
               actor, created_at
        FROM lot_movements
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

// ============================================================
// Movement listing
// ============================================================

/// Returns all movements for a lot, newest first.
pub async fn list_movements_by_lot(
    pool: &SqlitePool,
    lot_id: &str,
) -> Result<Vec<LotMovementResponse>, sqlx::Error> {
    sqlx::query_as::<_, LotMovementResponse>(
        r#"
        SELECT id, expiry_lot_id, movement_kind, direction, quantity,
               source_location_id, destination_location_id, reason, notes,
               actor, created_at
        FROM lot_movements
        WHERE expiry_lot_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(lot_id)
    .fetch_all(pool)
    .await
}

// ============================================================
// Per-location balances
// ============================================================

/// Returns per-location balances for a lot.
/// Only locations with balance > 0 are returned.
pub async fn location_balances_for_lot(
    pool: &SqlitePool,
    lot_id: &str,
) -> Result<Vec<LotLocationBalance>, sqlx::Error> {
    sqlx::query_as::<_, LotLocationBalance>(
        r#"
        SELECT location_id, SUM(balance) AS balance
        FROM (
            SELECT destination_location_id AS location_id, quantity AS balance
            FROM lot_movements
            WHERE expiry_lot_id = $1 AND destination_location_id IS NOT NULL
            UNION ALL
            SELECT source_location_id, -quantity
            FROM lot_movements
            WHERE expiry_lot_id = $1 AND source_location_id IS NOT NULL
        ) v
        GROUP BY location_id
        HAVING SUM(balance) > 0
        ORDER BY location_id
        "#,
    )
    .bind(lot_id)
    .fetch_all(pool)
    .await
}

// ============================================================
// Batch code generation helpers
// ============================================================

/// Finds the maximum NNN counter for a batch code prefix on a given date.
/// Returns `None` if no matching batch codes exist.
pub async fn find_max_nnn_for_prefix_on_date(
    pool: &SqlitePool,
    prefix: &str,
    date: &str,
) -> Result<Option<u32>, sqlx::Error> {
    // Pattern: PREFIX-DATE-NNN or PREFIX-DATE-NNN-M
    let pattern = format!("{}-{}%", prefix, date);
    let _prefix_len = prefix.len();
    let _date_len = date.len();

    let rows: Vec<BatchCodeRow> = sqlx::query_as(
        r#"
        SELECT batch_code
        FROM expiry_lots
        WHERE batch_code LIKE $1
        ORDER BY batch_code DESC
        LIMIT 10
        "#,
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let mut max_nnn: Option<u32> = None;
    for row in rows {
        if let Some(nnn) = extract_nnn_from_batch_code(&row.batch_code, prefix, date) {
            max_nnn = Some(max_nnn.map_or(nnn, |m| m.max(nnn)));
        }
    }

    Ok(max_nnn)
}

#[derive(Debug, sqlx::FromRow)]
struct BatchCodeRow {
    batch_code: String,
}

/// Extracts the NNN counter from a batch code matching PREFIX-DATE-NNN format.
fn extract_nnn_from_batch_code(batch_code: &str, prefix: &str, date: &str) -> Option<u32> {
    let expected_prefix = format!("{}-{}", prefix, date);
    if !batch_code.starts_with(&expected_prefix) {
        return None;
    }

    let suffix = &batch_code[expected_prefix.len()..];
    // Strip leading dash if present (for collision suffix format)
    let suffix = suffix.strip_prefix('-').unwrap_or(suffix);

    // Try to extract the first sequence of digits (NNN)
    let nnn_str: String = suffix.chars().take_while(|c| c.is_ascii_digit()).collect();
    nnn_str.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_nnn_from_batch_code_valid() {
        assert_eq!(
            extract_nnn_from_batch_code("SKU001-20261015-001", "SKU001", "20261015"),
            Some(1)
        );
        assert_eq!(
            extract_nnn_from_batch_code("SKU001-20261015-999", "SKU001", "20261015"),
            Some(999)
        );
    }

    #[test]
    fn extract_nnn_from_batch_code_with_collision_suffix() {
        assert_eq!(
            extract_nnn_from_batch_code("SKU001-20261015-999-2", "SKU001", "20261015"),
            Some(999)
        );
    }

    #[test]
    fn extract_nnn_from_batch_code_mismatch() {
        assert_eq!(
            extract_nnn_from_batch_code("OTHER-20261015-001", "SKU001", "20261015"),
            None
        );
        assert_eq!(
            extract_nnn_from_batch_code("SKU001-20261231-001", "SKU001", "20261015"),
            None
        );
    }
}
