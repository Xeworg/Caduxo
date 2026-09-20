//! Expiry lot service — create, update, archive, resolve.
//!
//! Business rules (pre-fill, validation, resolution) live here.
//! SQL is delegated to `db::repositories::expiry_lots`.
//! Product/store lookups use `db::repositories::products` and `stores`.
//! Lot movement emission (entry:initial) is handled here for atomicity with lot creation.

use chrono::NaiveDate;

use crate::db::repositories::expiry_lots as repo;
use crate::db::repositories::lot_movements as lm_repo;
use crate::db::repositories::products as product_repo;
use crate::db::repositories::settings as settings_repo;
use crate::db::repositories::stores as store_repo;
use crate::db::repositories::unit_definitions as unit_repo;
use crate::db::DbPool;
use crate::domain::lot_movements::{derive_batch_prefix, next_batch_candidate};
use crate::domain::lot_resolution::{
    compute_remaining_quantity, is_fully_resolved, is_valid_quantity,
};
use crate::dto::expiry_lots::{
    ArchiveLotInput, ExpiryLotCreate, ExpiryLotResolve, ExpiryLotResolveResult, ExpiryLotResponse,
    ExpiryLotUpdate, LotResolutionEventResponse,
};
use crate::error::{AppError, DomainError};
use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

/// Valid resolution type values.
const VALID_RESOLUTIONS: &[&str] = &["consumed", "sold", "discarded", "donated", "other"];

/// Returns true if the given resolution type is allowed.
fn is_valid_resolution_type(resolution: &str) -> bool {
    VALID_RESOLUTIONS.contains(&resolution.trim().to_ascii_lowercase().as_str())
}

/// Allow-listed archive reason codes. Mirrors the frontend `ARCHIVE_REASONS`
/// constant in `src/lib/expiry_lots.ts`. Keep both in sync.
const VALID_ARCHIVE_REASONS: &[&str] = &[
    "expired_unsold",
    "damaged",
    "returned_to_supplier",
    "recall",
    "lost",
    "internal_use",
    "administrative",
    "other",
];

/// Minimum trimmed length (in characters) for the archive justification notes.
const ARCHIVE_NOTES_MIN_CHARS: usize = 5;

/// Maximum trimmed length (in characters) for the archive justification notes.
const ARCHIVE_NOTES_MAX_CHARS: usize = 1000;

/// Returns true if the given reason is one of the allow-listed archive codes.
/// Comparison is case-insensitive and ignores surrounding whitespace.
fn is_valid_archive_reason(reason: &str) -> bool {
    VALID_ARCHIVE_REASONS.contains(&reason.trim().to_ascii_lowercase().as_str())
}

/// Validates that a quantity value is positive.
fn validate_quantity(qty: f64) -> Result<(), DomainError> {
    if !is_valid_quantity(qty) {
        return Err(DomainError::Validation {
            message: format!("Quantity must be positive, got {qty}"),
        });
    }
    Ok(())
}

/// Returns true when two quantities are considered equal for the purposes of
/// the metadata-only update guard. Uses a small absolute tolerance so that
/// innocuous float rounding from the UI never produces a false positive.
fn quantities_equal(a: f64, b: f64) -> bool {
    (a - b).abs() <= QUANTITY_EQ_TOLERANCE
}

/// Absolute tolerance used by [`quantities_equal`]. Far below any practical
/// product quantity (typical inputs are whole units or 2-decimal kg/L values).
const QUANTITY_EQ_TOLERANCE: f64 = 1e-9;

/// Validates an ISO-8601 date string (YYYY-MM-DD).
///
/// Emits the canonical English `InvalidDateFormat` variant so the command
/// layer can localise it via `localize_validation`. The label "expiry date"
/// matches the UserMessage catalog exactly; changing it here requires
/// updating `parse_user_message_kind` for round-trip coverage.
fn validate_expiry_date(date: &str) -> Result<(), DomainError> {
    let has_strict_shape = date.len() == 10
        && date.as_bytes()[4] == b'-'
        && date.as_bytes()[7] == b'-'
        && date
            .bytes()
            .enumerate()
            .all(|(idx, byte)| matches!(idx, 4 | 7) || byte.is_ascii_digit());
    if !has_strict_shape {
        return Err(DomainError::Validation {
            message: user_message(
                UserMessage::InvalidDateFormat {
                    label: "expiry date".into(),
                    value: date.into(),
                },
                Locale::En,
            ),
        });
    }

    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| DomainError::Validation {
            message: user_message(
                UserMessage::InvalidDateFormat {
                    label: "expiry date".into(),
                    value: date.into(),
                },
                Locale::En,
            ),
        })
}

/// Resolves the product's default unit from the catalog.
/// Priority: (1) product.default_unit_id → joined display_name from unit_definitions;
/// (2) product.default_unit raw text if no catalog link; (3) "Unidades" preset fallback.
async fn resolve_product_default_unit(
    pool: &DbPool,
    product: &crate::dto::products::ProductResponse,
) -> String {
    if let Some(ref unit_id) = product.default_unit_id {
        if let Ok(Some(unit)) = unit_repo::find_by_id(pool, unit_id).await {
            return unit.display_name;
        }
    }
    // No catalog link: fall back to raw text or the "Unidades" preset.
    if let Some(raw) = product.default_unit.as_deref() {
        if !raw.trim().is_empty() {
            return raw.trim().to_string();
        }
    }
    // Fallback to "Unidades" preset (key: "ud-units").
    unit_repo::find_by_id(pool, "ud-units")
        .await
        .ok()
        .flatten()
        .map(|u| u.display_name)
        .unwrap_or_else(|| "Unidades".to_string())
}

/// Returns the effective alert days for a lot, preferring user-supplied overrides
/// (any non-negative value) and falling back to the product default.
fn resolve_alert_days(user_alert: Option<i32>, product_default: i32) -> i32 {
    match user_alert {
        Some(d) if d >= 0 => d,
        _ => product_default,
    }
}

// ============================================================
// Location helpers
// ============================================================

/// Ensures a sentinel location exists for the given store and returns its id.
async fn ensure_sentinel_for_store(pool: &DbPool, store_id: &str) -> Result<String, AppError> {
    let sentinel_id = format!("loc-sentinel-{}", store_id);
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM store_locations WHERE id = $1)")
            .bind(&sentinel_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::from)?;
    if exists {
        return Ok(sentinel_id);
    }
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO store_locations (id, store_id, name, notes, is_active, created_at, updated_at)
        VALUES ($1, $2, 'Sin ubicacion', NULL, 1, $3, $4)
        "#,
    )
    .bind(&sentinel_id)
    .bind(store_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::from)?;
    Ok(sentinel_id)
}

// ============================================================
// Batch code helpers
// ============================================================

/// Resolves batch_code: auto-generates if blank/None, preserves verbatim otherwise.
async fn resolve_batch_code(
    pool: &DbPool,
    input_batch_code: &Option<String>,
    product: &crate::dto::products::ProductResponse,
) -> Result<String, AppError> {
    if let Some(ref code) = input_batch_code {
        let trimmed = code.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    // Auto-generate
    let prefix = derive_batch_prefix(Some(product.sku.as_str()));
    let date = chrono::Local::now().format("%Y%m%d").to_string();
    let max_nnn = lm_repo::find_max_nnn_for_prefix_on_date(pool, &prefix, &date)
        .await
        .map_err(AppError::from)?
        .unwrap_or(0);
    let candidate = next_batch_candidate(&prefix, &date, max_nnn);
    // Check for collisions
    let mut batch_code = candidate.clone();
    let mut attempts = 0;
    while attempts < 100 {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM expiry_lots WHERE batch_code = $1)")
                .bind(&batch_code)
                .fetch_one(pool)
                .await
                .map_err(AppError::from)?;
        if !exists {
            return Ok(batch_code);
        }
        let current_nnn = max_nnn + attempts + 1;
        batch_code = next_batch_candidate(&prefix, &date, current_nnn);
        attempts += 1;
    }
    // Fallback: UUID suffix after 100 collisions
    Ok(format!(
        "{}-{}-{}-{}",
        prefix,
        date,
        999,
        uuid::Uuid::new_v4().to_string()[..6].to_uppercase()
    ))
}

// ============================================================
// Create
// ============================================================

/// Creates a new expiry lot with an atomic entry:initial movement.
///
/// Location resolution:
/// - If `require_initial_location_on_lot_create` is true and `location_id` is None → reject.
/// - If `require_initial_location_on_lot_create` is false and `location_id` is None → use sentinel.
/// - If `location_id` is provided → use as-is.
///
/// Batch code:
/// - If `batch_code` is blank/None → auto-generate via `derive_batch_prefix` + counter.
/// - If `batch_code` is provided → preserve verbatim.
///
/// The entry:initial movement is written in the same transaction as the lot insert.
pub async fn create_expiry_lot(
    pool: &DbPool,
    input: ExpiryLotCreate,
) -> Result<ExpiryLotResponse, AppError> {
    // ── Precondition: at least one active store must exist. ──────────────────
    let store_exists = store_repo::has_active_store(pool).await?;
    if !store_exists {
        return Err(DomainError::BusinessRule {
            message: "Cannot create expiry lot: at least one store must exist first".to_string(),
        }
        .into());
    }

    // ── Validate input fields. ────────────────────────────────────────────────
    validate_quantity(input.quantity).map_err(AppError::Domain)?;
    validate_expiry_date(&input.expiry_date).map_err(AppError::Domain)?;

    // ── Look up the product to pre-fill defaults. ──────────────────────────
    let product = product_repo::get_product(pool, &input.product_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| DomainError::NotFound {
            resource: "product",
            id: input.product_id.clone(),
        })?;

    // ── Resolve unit: user-supplied wins; otherwise resolve via catalog. ──────
    let unit = match input.unit.as_deref() {
        Some(u) if !u.trim().is_empty() => u.trim().to_string(),
        _ => resolve_product_default_unit(pool, &product).await,
    };
    let alert_days_before =
        resolve_alert_days(input.alert_days_before, product.default_alert_days_before);

    // ── Check require_initial_location_on_lot_create setting. ─────────────────
    let require_location = settings_repo::get_require_initial_location_on_lot_create(pool)
        .await
        .map_err(AppError::from)?;

    // ── Resolve location_id: sentinel vs. explicit vs. required. ────────────
    let location_id = if let Some(ref loc) = input.location_id {
        Some(loc.clone())
    } else if require_location {
        // Emit the canonical English `LocationRequired` text so the command
        // layer can localise it via `localize_validation`. The persisted
        // sentinel name `Sin ubicacion` is unrelated — it stays in the
        // database as a stable identifier for unset-location lots.
        return Err(DomainError::Validation {
            message: user_message(UserMessage::LocationRequired, Locale::En),
        }
        .into());
    } else {
        // Setting is off and no location chosen → use/create sentinel
        Some(ensure_sentinel_for_store(pool, &input.store_id).await?)
    };

    // ── Resolve batch_code: auto-generate if blank, preserve if set. ───────────
    let batch_code = resolve_batch_code(pool, &input.batch_code, &product).await?;

    // ── Insert lot and emit entry:initial in a single transaction. ───────────
    let mut tx = pool.begin().await?;
    let lot_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO expiry_lots (
            id, product_id, store_id, location_id, quantity, unit,
            expiry_date, alert_days_before, batch_code, notes,
            status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'active', $11, $12)
        "#,
    )
    .bind(&lot_id)
    .bind(&input.product_id)
    .bind(&input.store_id)
    .bind(&location_id)
    .bind(input.quantity)
    .bind(&unit)
    .bind(&input.expiry_date)
    .bind(alert_days_before)
    .bind(&batch_code)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Emit entry:initial movement and reconcile lot.quantity in the same transaction.
    // entry:initial carries the lot's quantity (N) so the ledger records the initial stock.
    // The UPDATE then sets lot.quantity = ledger_sum = N (since the ledger has entry:initial(N)
    // and no other movements yet). This keeps lot.quantity in sync with the ledger
    // for runtime-created lots. The V5 migration path uses the same pattern:
    // entry:initial(N) + reconcile UPDATE → lot.quantity = N (or 0 for resolved lots).
    let movement_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO lot_movements (
            id, expiry_lot_id, movement_kind, direction, quantity,
            source_location_id, destination_location_id, notes, actor, created_at
        )
        VALUES ($1, $2, 'entry:initial', NULL, $3, NULL, $4, NULL, 'system', $5)
        "#,
    )
    .bind(&movement_id)
    .bind(&lot_id)
    .bind(input.quantity)
    .bind(&location_id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Reconcile lot.quantity from the ledger so lot.quantity == ledger_sum.
    // For a fresh lot with only entry:initial(N), this sets lot.quantity = N (no net change).
    // This step ensures runtime lots match the migrated-lots pattern where V5
    // reconciliation sets lot.quantity = ledger_sum after the backfill.
    sqlx::query(
        r#"
        UPDATE expiry_lots
        SET quantity = COALESCE(
            (SELECT SUM(
                CASE WHEN destination_location_id IS NOT NULL THEN lm.quantity
                     WHEN source_location_id IS NOT NULL THEN -lm.quantity
                     ELSE 0 END)
            FROM lot_movements lm WHERE lm.expiry_lot_id = expiry_lots.id
            ), 0),
            updated_at = $1
        WHERE id = $2
        "#,
    )
    .bind(&now)
    .bind(&lot_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Fetch and return the created lot
    repo::get_expiry_lot(pool, &lot_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::from(sqlx::Error::RowNotFound))
}

// ============================================================
// Update
// ============================================================

/// Updates the metadata of an existing active expiry lot. The `quantity`
/// field on the input is accepted but **must match the existing lot
/// quantity**; any attempt to change it through this path is rejected with
/// `BusinessRule`. Quantity changes must go through the movement /
/// adjustment / resolve flows so the `lot_movements` ledger stays the
/// source of truth.
///
/// Editable fields: `location_id`, `unit`, `expiry_date`,
/// `alert_days_before`, `batch_code`, `notes`.
///
/// Returns:
/// - `NotFound` if the lot does not exist.
/// - `BusinessRule` if the lot is not currently `active`.
/// - `BusinessRule` if `input.quantity` differs from the existing quantity.
/// - `Validation` for invalid quantity / expiry date shapes.
pub async fn update_expiry_lot(
    pool: &DbPool,
    input: ExpiryLotUpdate,
) -> Result<ExpiryLotResponse, AppError> {
    validate_quantity(input.quantity).map_err(AppError::Domain)?;
    validate_expiry_date(&input.expiry_date).map_err(AppError::Domain)?;

    // ── Fetch existing lot to enforce metadata-only updates. ────────────────
    let existing = repo::get_expiry_lot(pool, &input.id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: input.id.clone(),
        })?;

    if existing.status != "active" {
        return Err(DomainError::BusinessRule {
            message: format!("Cannot update lot: status is `{}`", existing.status),
        }
        .into());
    }

    // ── Quantity guard: update is metadata-only. Movement / resolve flows ────
    // remain the only supported way to change a lot's quantity so the
    // `lot_movements` ledger stays authoritative.
    if !quantities_equal(input.quantity, existing.quantity) {
        return Err(DomainError::BusinessRule {
            message: format!(
"Cannot change quantity of expiry lot directly: quantity must remain {:.2} {}. Use movement / adjustment / resolve actions to change it.",
existing.quantity, existing.unit
            ),
        }
        .into());
    }

    let row = repo::update_expiry_lot(pool, &input)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: input.id.clone(),
        })?;
    Ok(row)
}

// ============================================================
// Archive
// ============================================================

/// Soft-archives an active expiry lot and persists the archive
/// justification in `lot_movements` as an `exit:other` marker.
///
/// Validation:
/// - `reason` must be one of [`VALID_ARCHIVE_REASONS`] (case-insensitive).
/// - `notes` must be at least [`ARCHIVE_NOTES_MIN_CHARS`] and at most
///   [`ARCHIVE_NOTES_MAX_CHARS`] characters after trimming.
///
/// The soft-archive update and the ledger insert happen in the same DB
/// transaction. The ledger row uses `movement_kind = 'exit:other'`,
/// `direction = NULL`, `quantity = 0`, `source_location_id` from the lot,
/// `actor = 'system'`, and copies `reason` / `notes` from the input.
///
/// Returns `NotFound` if the lot does not exist; `BusinessRule` if the lot
/// is not currently `active`.
pub async fn archive_expiry_lot(pool: &DbPool, input: ArchiveLotInput) -> Result<(), AppError> {
    // ── Validate reason (allow-list, case-insensitive). ──────────────────────
    let reason = input.reason.trim().to_ascii_lowercase();
    if !is_valid_archive_reason(&input.reason) {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Reason must be one of {:?}, got `{}`",
                VALID_ARCHIVE_REASONS, input.reason
            ),
        }));
    }

    // ── Validate notes (trimmed char length). ───────────────────────────────
    let notes = input.notes.trim();
    let notes_len = notes.chars().count();
    if notes_len < ARCHIVE_NOTES_MIN_CHARS {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Notes must be at least {} characters (got {})",
                ARCHIVE_NOTES_MIN_CHARS, notes_len
            ),
        }));
    }
    if notes_len > ARCHIVE_NOTES_MAX_CHARS {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Notes must be at most {} characters (got {})",
                ARCHIVE_NOTES_MAX_CHARS, notes_len
            ),
        }));
    }

    // ── Fetch the lot to verify it exists and is active. ────────────────────
    let lot = repo::get_expiry_lot(pool, &input.id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| DomainError::NotFound {
            resource: "expiry_lot",
            id: input.id.clone(),
        })?;

    if lot.status != "active" {
        return Err(DomainError::BusinessRule {
            message: format!("Cannot archive lot: status is already `{}`", lot.status),
        }
        .into());
    }

    // ── Transactional archive + ledger insert. ───────────────────────────────
    let mut tx = pool.begin().await?;
    let now = chrono::Utc::now().to_rfc3339();

    let affected = sqlx::query(
        r#"
        UPDATE expiry_lots
        SET status = 'archived', updated_at = $1
        WHERE id = $2 AND status = 'active'
        "#,
    )
    .bind(&now)
    .bind(&input.id)
    .execute(&mut *tx)
    .await?;

    if affected.rows_affected() == 0 {
        // Race: another writer archived this lot between our pre-fetch and
        // the transactional UPDATE. Treat as no-longer-archiveable.
        return Err(DomainError::BusinessRule {
            message: "Lot is no longer active and cannot be archived".to_string(),
        }
        .into());
    }

    let movement_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO lot_movements (
            id, expiry_lot_id, movement_kind, direction, quantity,
            source_location_id, destination_location_id, reason, notes,
            actor, created_at
        )
        VALUES ($1, $2, 'exit:other', NULL, 1.0, $3, NULL, $4, $5, 'system', $6)
        "#,
    )
    .bind(&movement_id)
    .bind(&input.id)
    .bind(&lot.location_id)
    .bind(&reason)
    .bind(notes)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

// ============================================================
// Get / List
// ============================================================

/// Returns a single expiry lot by id.
pub async fn get_expiry_lot(pool: &DbPool, id: String) -> Result<ExpiryLotResponse, AppError> {
    let row = repo::get_expiry_lot(pool, &id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id,
        })?;
    Ok(row)
}

/// Returns all active expiry lots for a product, ordered by expiry date.
pub async fn list_expiry_lots_by_product(
    pool: &DbPool,
    product_id: String,
) -> Result<Vec<ExpiryLotResponse>, AppError> {
    repo::list_expiry_lots_by_product(pool, &product_id)
        .await
        .map_err(AppError::from)
}

/// Returns all active expiry lots for a store, ordered by expiry date.
pub async fn list_expiry_lots_by_store(
    pool: &DbPool,
    store_id: String,
) -> Result<Vec<ExpiryLotResponse>, AppError> {
    repo::list_expiry_lots_by_store(pool, &store_id)
        .await
        .map_err(AppError::from)
}

/// Returns all active expiry lots across all stores, ordered by expiry date.
pub async fn list_all_expiry_lots(pool: &DbPool) -> Result<Vec<ExpiryLotResponse>, AppError> {
    repo::list_all_active_expiry_lots(pool)
        .await
        .map_err(AppError::from)
}

// ============================================================
// Partial resolution
// ============================================================

/// Resolves a quantity from an expiry lot, records a resolution event, and
/// marks the lot as fully resolved when remaining quantity reaches zero.
///
/// Returns the resolution event id, resolved quantity, remaining quantity, and
/// whether the lot is now fully resolved.
pub async fn resolve_expiry_lot(
    pool: &DbPool,
    input: ExpiryLotResolve,
) -> Result<ExpiryLotResolveResult, AppError> {
    // ── Validate. ────────────────────────────────────────────────────────────
    validate_quantity(input.quantity).map_err(AppError::Domain)?;
    if !is_valid_resolution_type(&input.resolution) {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Resolution must be one of {:?}, got `{}`",
                VALID_RESOLUTIONS, input.resolution
            ),
        }));
    }

    // ── Fetch the lot and validate it exists and is active. ─────────────────
    let lot = repo::get_expiry_lot(pool, &input.lot_id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: input.lot_id.clone(),
        })?;

    if lot.status != "active" {
        return Err(DomainError::BusinessRule {
            message: format!("Cannot resolve lot: lot is already `{}`", lot.status),
        }
        .into());
    }

    // ── Validate resolved_qty does not exceed remaining quantity. ────────────
    if input.quantity > lot.quantity {
        return Err(AppError::Domain(DomainError::Validation {
            message: format!(
                "Cannot resolve {:.2} {}: only {:.2} {} remain",
                input.quantity, lot.unit, lot.quantity, lot.unit
            ),
        }));
    }

    // ── Record the resolution event first (event is the source of truth). ──
    let event = repo::insert_resolution_event(pool, &input)
        .await
        .map_err(AppError::from)?;

    // ── Compute remaining quantity and update the lot. ───────────────────────
    // SAFETY: `input.quantity <= lot.quantity` is enforced by the guard above,
    // so `compute_remaining_quantity` is guaranteed to return `Some`.
    let remaining = match compute_remaining_quantity(lot.quantity, input.quantity) {
        Some(v) => v,
        None => unreachable!(),
    };
    let now = chrono::Utc::now();
    let resolved_at = now.to_rfc3339();

    repo::apply_partial_resolution(
        pool,
        &input.lot_id,
        input.quantity,
        &input.resolution,
        &resolved_at,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ExpiryLotResolveResult {
        lot_id: input.lot_id,
        resolved_quantity: input.quantity,
        remaining_quantity: remaining,
        is_fully_resolved: is_fully_resolved(remaining),
        resolution_event_id: event.id,
    })
}

/// Returns all resolution events for a given expiry lot.
pub async fn list_resolution_events(
    pool: &DbPool,
    lot_id: String,
) -> Result<Vec<LotResolutionEventResponse>, AppError> {
    // Verify the lot exists first.
    let _ = repo::get_expiry_lot(pool, &lot_id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "expiry_lot",
            id: lot_id.clone(),
        })?;

    repo::list_resolution_events_by_lot(pool, &lot_id)
        .await
        .map_err(AppError::from)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::expiry_lots::{
        ArchiveLotInput, ExpiryLotCreate, ExpiryLotResolve, ExpiryLotUpdate,
    };
    use crate::dto::products::ProductCreate;
    use crate::dto::stores::StoreCreate;
    use crate::services::expiry_lots as svc;
    use crate::services::stores::create_store as create_store_svc;

    /// Helper: creates a store + product, returning (store_id, product_id).
    async fn seed_product(
        pool: &crate::db::DbPool,
    ) -> Result<(String, String, String), Box<dyn std::error::Error>> {
        // Create a store first (required by lot creation).
        let store = create_store_svc(
            pool,
            StoreCreate {
                name: "Test Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        // Create a location for the store.
        let location = crate::db::repositories::stores::insert_location(
            pool,
            &crate::dto::stores::StoreLocationCreate {
                store_id: store.id.clone(),
                name: "Test Location".into(),
                notes: None,
            },
        )
        .await?;

        // Disable require_initial_location_on_lot_create so existing tests
        // that pass location_id: None continue to work with the sentinel.
        // Tests that exercise the location-requirement feature should instead
        // pass a location_id explicitly.
        crate::db::repositories::settings::set_require_initial_location_on_lot_create(pool, false)
            .await?;

        // Create a product with a known default_unit and alert_days.
        let product = crate::services::products::create_product(
            pool,
            ProductCreate {
                sku: "SEED-SKU-001".into(),
                description: "Test Product".into(),
                category_ids: None,
                default_unit: Some("kg".into()),
                default_unit_id: None,
                default_alert_days_before: 14,
                notes: None,
            },
        )
        .await?;
        Ok((store.id, product.id, location.id))
    }

    // ------------------------------------------------------------------
    // Create — precondition: store must exist
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn create_lot_requires_store() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // No store → must fail with BusinessRule.
        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: "fake".into(),
                store_id: "no-such-store".into(),
                location_id: None,
                quantity: 5.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("creating lot without a store must fail");

        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::BusinessRule { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(), // pool has one store seeded
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(lot.product_id, product_id);
        assert_eq!(lot.quantity, 10.0);
        assert_eq!(lot.unit, "L");
        assert_eq!(lot.alert_days_before, 7);
        assert_eq!(lot.status, "active");
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_pre_fills_unit_from_product() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        // User omits unit → product default "kg" should be used.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 2.0,
                unit: None, // not provided
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.unit, "Kilogramo",
            "unit should be pre-filled from product default via catalog display_name"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_user_unit_overrides_product_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        // Product has default "kg" but user specifies "g" → user wins.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 2.0,
                unit: Some("g".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.unit, "g",
            "user-supplied unit must override product default"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_pre_fills_alert_days_from_product() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        // Product has default 14 alert days; user omits → should use 14.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None, // not provided
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.alert_days_before, 14,
            "alert_days should be pre-filled from product default"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_user_alert_days_overrides_product_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        // Product has default 14 but user specifies 30 → user wins.
        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(30),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(
            lot.alert_days_before, 30,
            "user-supplied alert_days must override product default"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_rejects_negative_quantity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: -1.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("negative quantity must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_rejects_zero_quantity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 0.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("zero quantity must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_rejects_invalid_expiry_date() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        for bad_date in ["not-a-date", "2025/12/31", "25-12-31", ""] {
            let err = svc::create_expiry_lot(
                &pool,
                ExpiryLotCreate {
                    product_id: product_id.clone(),
                    store_id: store_id.clone(),
                    location_id: None,
                    quantity: 5.0,
                    unit: None,
                    expiry_date: bad_date.into(),
                    alert_days_before: None,
                    batch_code: None,
                    notes: None,
                },
            )
            .await
            .expect_err(&format!("invalid date `{bad_date}` must be rejected"));
            assert!(matches!(
                err,
                crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Update
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn update_lot_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Metadata-only update: quantity must be preserved.
        let updated = svc::update_expiry_lot(
            &pool,
            ExpiryLotUpdate {
                id: lot.id.clone(),
                location_id: None,
                quantity: lot.quantity,
                unit: "mL".into(),
                expiry_date: "2026-01-15".into(),
                alert_days_before: 14,
                batch_code: Some("BATCH-001".into()),
                notes: Some("Updated notes".into()),
            },
        )
        .await?;

        assert_eq!(
            updated.quantity, 10.0,
            "quantity must be preserved on metadata-only update"
        );
        assert_eq!(updated.unit, "mL");
        assert_eq!(updated.expiry_date, "2026-01-15");
        assert_eq!(updated.alert_days_before, 14);
        assert_eq!(updated.batch_code.as_deref(), Some("BATCH-001"));
        Ok(())
    }

    #[tokio::test]
    async fn update_lot_rejects_quantity_change() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Attempt to change the quantity through update — must be rejected.
        let err = svc::update_expiry_lot(
            &pool,
            ExpiryLotUpdate {
                id: lot.id.clone(),
                location_id: None,
                quantity: 8.0,
                unit: lot.unit.clone(),
                expiry_date: lot.expiry_date.clone(),
                alert_days_before: lot.alert_days_before,
                batch_code: lot.batch_code.clone(),
                notes: lot.notes.clone(),
            },
        )
        .await
        .expect_err("changing quantity through update must be rejected");

        assert!(matches!(
            err,
            crate::error::AppError::Domain(
                crate::error::DomainError::BusinessRule { ref message }
            ) if message.contains("Cannot change quantity of expiry lot directly")
        ));

        // The lot must remain untouched: same quantity, same status.
        let after = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(after.quantity, 10.0);
        assert_eq!(after.status, "active");
        Ok(())
    }

    #[tokio::test]
    async fn update_lot_metadata_only_with_same_quantity_succeeds(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: Some("kg".into()),
                expiry_date: "2025-06-30".into(),
                alert_days_before: Some(14),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Only metadata changes; quantity mirrors the existing lot value.
        let updated = svc::update_expiry_lot(
            &pool,
            ExpiryLotUpdate {
                id: lot.id.clone(),
                location_id: None,
                quantity: lot.quantity,
                unit: "g".into(),
                expiry_date: "2026-06-30".into(),
                alert_days_before: 60,
                batch_code: Some("META-ONLY".into()),
                notes: Some("metadata only".into()),
            },
        )
        .await?;

        assert_eq!(updated.quantity, 5.0);
        assert_eq!(updated.unit, "g");
        assert_eq!(updated.expiry_date, "2026-06-30");
        assert_eq!(updated.alert_days_before, 60);
        assert_eq!(updated.batch_code.as_deref(), Some("META-ONLY"));
        assert_eq!(updated.notes.as_deref(), Some("metadata only"));
        Ok(())
    }

    #[tokio::test]
    async fn update_missing_lot_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::update_expiry_lot(
            &pool,
            ExpiryLotUpdate {
                id: "nope".into(),
                location_id: None,
                quantity: 5.0,
                unit: "kg".into(),
                expiry_date: "2025-12-31".into(),
                alert_days_before: 30,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("updating missing lot must be NotFound");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Archive
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn archive_lot_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        svc::archive_expiry_lot(
            &pool,
            ArchiveLotInput {
                id: lot.id.clone(),
                reason: "expired_unsold".into(),
                notes: "Past expiry date, no buyer".into(),
            },
        )
        .await?;

        // Archived lot should no longer appear in the active list.
        let lots = svc::list_expiry_lots_by_product(&pool, lot.product_id.clone()).await?;
        assert!(
            lots.is_empty(),
            "archived lot should not appear in active list"
        );
        Ok(())
    }

    #[tokio::test]
    async fn archive_lot_records_exit_other_movement() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let reason = "damaged";
        let notes = "Water damage in storage, discarded for safety.";
        svc::archive_expiry_lot(
            &pool,
            ArchiveLotInput {
                id: lot.id.clone(),
                reason: reason.into(),
                notes: notes.into(),
            },
        )
        .await?;

        // Verify exactly one exit:other row was persisted for this lot.
        let rows: Vec<(
            String,
            Option<String>,
            f64,
            Option<String>,
            Option<String>,
            String,
        )> = sqlx::query_as(
            r#"
                SELECT movement_kind, direction, quantity, source_location_id, reason, notes
                FROM lot_movements
                WHERE expiry_lot_id = $1
                "#,
        )
        .bind(&lot.id)
        .fetch_all(&pool)
        .await?;

        // Lots carry an entry:initial movement from creation plus the archive's exit:other.
        let exit_rows: Vec<_> = rows
            .iter()
            .filter(|(k, _, _, _, _, _)| k == "exit:other")
            .collect();
        assert_eq!(
            exit_rows.len(),
            1,
            "exactly one exit:other movement should be persisted on archive"
        );

        let (_kind, direction, qty, source, stored_reason, stored_notes) = exit_rows[0];
        assert!(direction.is_none(), "exit:other direction should be NULL");
        // V17 CHECK constraint requires quantity > 0 for all movements.
        // The archive marker uses quantity = 1 (minimum positive) to satisfy the constraint.
        // The archive operation sets lot.status = 'archived' (not 'resolved'), so the
        // legacy archive path does not affect the ledger balance in the same way as
        // the ledger path. A quantity = 1 marker is a minimal stock reduction indicator.
        assert_eq!(
            *qty, 1.0,
            "archive marker should have quantity = 1 (minimum positive)"
        );
        // Lot was created with location_id: None → service used the sentinel location
        // (because require_initial_location_on_lot_create defaults off in tests).
        assert!(
            source.is_some(),
            "source_location_id should be populated from the lot location"
        );
        assert_eq!(stored_reason.as_deref(), Some(reason));
        assert_eq!(stored_notes.as_str(), notes);
        Ok(())
    }

    #[tokio::test]
    async fn archive_lot_rejects_blank_or_short_notes() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        for bad_notes in ["", "   ", "\t\n", "abcd"] {
            let err = svc::archive_expiry_lot(
                &pool,
                ArchiveLotInput {
                    id: lot.id.clone(),
                    reason: "expired_unsold".into(),
                    notes: bad_notes.into(),
                },
            )
            .await
            .expect_err(&format!("blank/short notes `{bad_notes}` must be rejected"));
            assert!(matches!(
                err,
                crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
            ));
        }

        // Lot must remain active because every attempt was rejected.
        let after = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(after.status, "active");
        Ok(())
    }

    #[tokio::test]
    async fn archive_lot_rejects_invalid_reason() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::archive_expiry_lot(
            &pool,
            ArchiveLotInput {
                id: lot.id.clone(),
                reason: "because_i_said_so".into(),
                notes: "This should never persist.".into(),
            },
        )
        .await
        .expect_err("unknown reason must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));

        // Lot must remain active.
        let after = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(after.status, "active");
        Ok(())
    }

    #[tokio::test]
    async fn archive_missing_lot_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::archive_expiry_lot(
            &pool,
            ArchiveLotInput {
                id: "nope".into(),
                reason: "expired_unsold".into(),
                notes: "Trying to archive something that doesn't exist".into(),
            },
        )
        .await
        .expect_err("archiving missing lot must be NotFound");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::NotFound { .. })
        ));
        Ok(())
    }

    // ------------------------------------------------------------------
    // List
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn list_lots_by_product_ordered_by_expiry() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        // Create two lots with different expiry dates.
        let _early = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-06-01".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;
        let _late = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 3.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let lots = svc::list_expiry_lots_by_product(&pool, product_id).await?;
        assert_eq!(lots.len(), 2);
        assert_eq!(lots[0].expiry_date, "2025-06-01", "early expiry first");
        assert_eq!(lots[1].expiry_date, "2025-12-31", "late expiry second");
        Ok(())
    }

    // ------------------------------------------------------------------
    // Partial resolution
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn partial_resolution_reduces_quantity_and_records_event(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: Some("L".into()),
                expiry_date: "2025-12-31".into(),
                alert_days_before: Some(7),
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Resolve 3 units.
        let result = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 3.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;

        assert_eq!(result.resolved_quantity, 3.0);
        assert_eq!(result.remaining_quantity, 7.0);
        assert!(!result.is_fully_resolved);

        // Verify the lot quantity was updated.
        let updated = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(updated.quantity, 7.0);
        assert_eq!(updated.status, "active");

        // Verify the resolution event was recorded.
        let events = svc::list_resolution_events(&pool, lot.id.clone()).await?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].quantity, 3.0);
        assert_eq!(events[0].resolution, "consumed");
        Ok(())
    }

    #[tokio::test]
    async fn full_resolution_marks_lot_resolved() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Resolve all 5 units at once.
        let result = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 5.0,
                resolution: "discarded".into(),
                notes: Some("All expired".into()),
            },
        )
        .await?;

        assert_eq!(result.remaining_quantity, 0.0);
        assert!(result.is_fully_resolved);

        let updated = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(updated.status, "resolved");
        // DB clamps quantity to 1.0 to satisfy CHECK(quantity > 0); remaining is tracked via events.
        assert_eq!(updated.quantity, 1.0);
        assert_eq!(updated.resolution.as_deref(), Some("discarded"));
        assert!(updated.resolved_at.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn cannot_resolve_more_than_remaining() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 10.0, // more than the 5 remaining
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await
        .expect_err("resolving more than remaining must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn cannot_resolve_archived_or_resolved_lot() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Resolve fully → status becomes "resolved".
        svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 5.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;

        // Second resolution attempt must fail.
        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 1.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await
        .expect_err("resolving already-resolved lot must fail");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::BusinessRule { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn resolve_rejects_invalid_resolution_type() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 2.0,
                resolution: "eaten".into(), // not a valid type
                notes: None,
            },
        )
        .await
        .expect_err("invalid resolution type must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn resolve_rejects_zero_quantity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        let err = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 0.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await
        .expect_err("zero resolution quantity must be rejected");
        assert!(matches!(
            err,
            crate::error::AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn multiple_partial_resolutions_accumulate() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // First partial resolution.
        let r1 = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 3.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;
        assert_eq!(r1.remaining_quantity, 7.0);

        // Second partial resolution.
        let r2 = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 4.0,
                resolution: "discarded".into(),
                notes: None,
            },
        )
        .await?;
        assert_eq!(r2.remaining_quantity, 3.0);
        assert!(!r2.is_fully_resolved);

        // Both events recorded.
        let events = svc::list_resolution_events(&pool, lot.id.clone()).await?;
        assert_eq!(events.len(), 2);

        // Final resolution.
        let r3 = svc::resolve_expiry_lot(
            &pool,
            ExpiryLotResolve {
                lot_id: lot.id.clone(),
                quantity: 3.0,
                resolution: "consumed".into(),
                notes: None,
            },
        )
        .await?;
        assert!(r3.is_fully_resolved);
        assert_eq!(r3.remaining_quantity, 0.0);

        let updated = svc::get_expiry_lot(&pool, lot.id.clone()).await?;
        assert_eq!(updated.status, "resolved");
        Ok(())
    }

    // ------------------------------------------------------------------
    // Phase 2: lot movement emission, location requirement, auto batch code
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn create_lot_emits_entry_initial_movement() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id: product_id.clone(),
                store_id: store_id.clone(),
                location_id: None,
                quantity: 10.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        // Verify an entry:initial movement was created for this lot.
        let movements: Vec<(String, String)> =
            sqlx::query_as("SELECT id, movement_kind FROM lot_movements WHERE expiry_lot_id = $1")
                .bind(&lot.id)
                .fetch_all(&pool)
                .await?;

        assert_eq!(movements.len(), 1, "exactly one movement should exist");
        assert_eq!(movements[0].1, "entry:initial");
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_requires_location_when_setting_is_on(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        // Enable the require-location setting.
        crate::db::repositories::settings::set_require_initial_location_on_lot_create(&pool, true)
            .await?;

        let err = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id,
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await
        .expect_err("location_id: None with require=true must be rejected");

        assert!(matches!(
                err,
                crate::error::AppError::Domain(crate::error::DomainError::Validation {
        message,
                }) if message == "Please select a location"
            ));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_uses_explicit_location_when_provided(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, location_id) = seed_product(&pool).await?;

        // Enable require-location setting (explicit location should bypass requirement).
        crate::db::repositories::settings::set_require_initial_location_on_lot_create(&pool, true)
            .await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id: store_id.clone(),
                location_id: Some(location_id.clone()),
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None,
                notes: None,
            },
        )
        .await?;

        assert_eq!(lot.location_id.as_deref(), Some(location_id.as_str()));
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_auto_generates_batch_code() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id,
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: None, // blank → auto-generate
                notes: None,
            },
        )
        .await?;

        // Batch code must be non-empty and contain today's date (format: PREFIX-YYYYMMDD-NNN).
        let batch = lot.batch_code.as_deref().unwrap();
        assert!(!batch.is_empty(), "batch_code should not be empty");
        let today = chrono::Local::now().format("%Y%m%d").to_string();
        assert!(
            batch.contains(&today),
            "batch_code should contain today's date ({today}), got: {batch}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_lot_preserves_explicit_batch_code() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let (store_id, product_id, _location_id) = seed_product(&pool).await?;

        let lot = svc::create_expiry_lot(
            &pool,
            ExpiryLotCreate {
                product_id,
                store_id,
                location_id: None,
                quantity: 5.0,
                unit: None,
                expiry_date: "2025-12-31".into(),
                alert_days_before: None,
                batch_code: Some("MY-BATCH-42".into()),
                notes: None,
            },
        )
        .await?;

        assert_eq!(lot.batch_code.as_deref(), Some("MY-BATCH-42"));
        Ok(())
    }

    #[tokio::test]
    async fn settings_require_initial_location_defaults_to_true(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let settings = crate::services::settings::get_settings(&pool).await?;
        assert!(
            settings.require_initial_location_on_lot_create,
            "require_initial_location_on_lot_create should default to true"
        );
        Ok(())
    }

    #[tokio::test]
    async fn settings_update_toggles_require_initial_location(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        let s1 = crate::services::settings::get_settings(&pool).await?;
        assert!(s1.require_initial_location_on_lot_create);

        let s2 = crate::services::settings::update_settings(
            &pool,
            crate::dto::stores::SettingsUpdate {
                last_selected_store_id: None,
                require_initial_location_on_lot_create: Some(false),
                language: None,
            },
        )
        .await?;
        assert!(!s2.require_initial_location_on_lot_create);

        let s3 = crate::services::settings::update_settings(
            &pool,
            crate::dto::stores::SettingsUpdate {
                last_selected_store_id: None,
                require_initial_location_on_lot_create: Some(true),
                language: None,
            },
        )
        .await?;
        assert!(s3.require_initial_location_on_lot_create);
        Ok(())
    }
}
