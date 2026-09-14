//! Audit / banner service for unrecognized legacy units.
//!
//! Surfaces products whose `default_unit` text has no catalog link, computes
//! the SHA-256 signature for banner dismissal, and executes review actions
//! (map-to-preset, keep-as-custom, leave-for-later).

use crate::db::repositories::settings as settings_repo;
use crate::db::repositories::unit_definitions as units_repo;
use crate::db::DbPool;
use crate::dto::unit_definitions::{
    UnitAuditBannerState, UnitKind, UnitReviewAction, UnitReviewActionResult, UnrecognizedUnitGroup,
};
use crate::error::{AppError, DomainError};
use sha2::{Digest, Sha256};

/// Key in `app_settings` where the dismissed banner signature is persisted.
const SETTINGS_KEY_DISMISSED_SIGNATURE: &str = "unit_audit.dismissed_signature";

/// Alias → canonical kind mapping for the audit query.
/// These are recognized as "known but not in the catalog" — they never
/// silently back-fill `default_unit_id` in V3; they are surfaced here so
/// users can create them as proper catalog units.
#[allow(dead_code)]
pub const UNIT_AUDIT_ALIASES: &[(&str, &str)] = &[
    // Integer-family aliases
    ("units", "integer"),
    ("pcs", "integer"),
    ("cajas", "integer"),
    ("box", "integer"),
    ("boxes", "integer"),
    ("bottles", "integer"),
    ("bags", "integer"),
    ("packs", "integer"),
    ("u", "integer"),
    ("pza", "integer"),
    ("pzas", "integer"),
    ("und", "integer"),
    // Decimal-family aliases
    ("kg", "decimal"),
    ("g", "decimal"),
    ("mg", "decimal"),
    ("tn", "decimal"),
    ("l", "decimal"),
    ("ml", "decimal"),
    ("lb", "decimal"),
    ("oz", "decimal"),
];

/// Returns all unrecognized unit groups: products with `default_unit_id IS NULL`
/// and a non-empty raw text value.
pub async fn unrecognized_units(pool: &DbPool) -> Result<Vec<UnrecognizedUnitGroup>, AppError> {
    let rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT p.default_unit, COUNT(*) AS product_count, MIN(p.id) AS sample_product_id
         FROM products p
         WHERE p.default_unit_id IS NULL
           AND p.default_unit IS NOT NULL
           AND TRIM(p.default_unit) <> ''
         GROUP BY p.default_unit
         ORDER BY product_count DESC, p.default_unit ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(
            |(raw_value, product_count, sample_product_id)| UnrecognizedUnitGroup {
                raw_value,
                product_count: product_count as usize,
                sample_product_id,
            },
        )
        .collect())
}

/// Computes the current banner signature: SHA-256 of the sorted raw-value list,
/// truncated to 16 hex characters (64 bits).
pub async fn current_signature(pool: &DbPool) -> Result<String, AppError> {
    let groups = unrecognized_units(pool).await?;
    let sorted: Vec<String> = groups.into_iter().map(|g| g.raw_value).collect();
    let data = sorted.join("|");
    let hash = Sha256::digest(data.as_bytes());
    // Truncate to 8 bytes (16 hex chars).
    let hex: String = hash[..8].iter().map(|b| format!("{:02x}", b)).collect();
    Ok(hex)
}

/// Returns the dismissed signature from app_settings, if any.
async fn get_dismissed_signature(pool: &DbPool) -> Result<Option<String>, AppError> {
    settings_repo::get_setting(pool, SETTINGS_KEY_DISMISSED_SIGNATURE)
        .await
        .map_err(AppError::from)
}

/// Returns the banner visibility state.
pub async fn banner_state(pool: &DbPool) -> Result<UnitAuditBannerState, AppError> {
    let groups = unrecognized_units(pool).await?;
    let signature = current_signature(pool).await?;
    let dismissed = get_dismissed_signature(pool).await?;
    let visible = dismissed.as_ref() != Some(&signature);

    Ok(UnitAuditBannerState {
        visible,
        signature,
        group_count: groups.len(),
    })
}

/// Dismisses the banner by persisting the current signature.
pub async fn dismiss_banner(pool: &DbPool) -> Result<(), AppError> {
    let signature = current_signature(pool).await?;
    settings_repo::upsert_setting(pool, SETTINGS_KEY_DISMISSED_SIGNATURE, &signature)
        .await
        .map_err(AppError::from)
}

/// Applies a review action. Returns the number of products updated.
pub async fn apply_review_action(
    pool: &DbPool,
    action: UnitReviewAction,
) -> Result<UnitReviewActionResult, AppError> {
    match action {
        UnitReviewAction::MapToPreset {
            raw_value,
            preset_id,
        } => map_to_preset(pool, &raw_value, &preset_id).await,
        UnitReviewAction::KeepAsCustom { raw_value } => keep_as_custom(pool, &raw_value).await,
        UnitReviewAction::LeaveForLater { .. } => {
            // No-op: leave products unlinked.
            Ok(UnitReviewActionResult {
                updated_product_count: 0,
            })
        }
    }
}

/// Updates all products matching `raw_value` to point to `preset_id`.
async fn map_to_preset(
    pool: &DbPool,
    raw_value: &str,
    preset_id: &str,
) -> Result<UnitReviewActionResult, AppError> {
    // Validate preset exists.
    let preset = units_repo::find_by_id(pool, preset_id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "unit_definition",
            id: preset_id.to_string(),
        })?;

    let kind_str = match preset.kind {
        UnitKind::Integer => "integer",
        UnitKind::Decimal => "decimal",
    };

    let result = sqlx::query(
        "UPDATE products
         SET default_unit_id = $1, unit_type = $2, default_unit = $3, updated_at = $4
         WHERE default_unit_id IS NULL
           AND default_unit IS NOT NULL
           AND TRIM(default_unit) <> ''
           AND TRIM(lower(default_unit)) = TRIM(lower($5))",
    )
    .bind(preset_id)
    .bind(kind_str)
    .bind(&preset.display_name)
    .bind(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    .bind(raw_value)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(UnitReviewActionResult {
        updated_product_count: result.rows_affected() as usize,
    })
}

/// Creates a custom unit from `raw_value` and maps all matching products to it.
async fn keep_as_custom(
    pool: &DbPool,
    raw_value: &str,
) -> Result<UnitReviewActionResult, AppError> {
    let key = slugify(raw_value);
    let display_name = raw_value.trim().to_string();

    // Check if a custom unit with this key already exists.
    let existing = units_repo::find_by_key(pool, &key)
        .await
        .map_err(AppError::from)?;

    let (unit_id, kind) = match existing {
        Some(unit) => {
            // Reuse existing custom unit.
            let kind_str = match unit.kind {
                UnitKind::Integer => "integer",
                UnitKind::Decimal => "decimal",
            };
            (unit.id, kind_str.to_string())
        }
        None => {
            // Create a new custom unit. Default to "integer" kind (count-like).
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let id = format!("ud-{}", &key);
            let inserted = units_repo::insert(
                pool,
                &id,
                &key,
                &display_name,
                UnitKind::Integer,
                false,
                &now,
                &now,
            )
            .await
            .map_err(AppError::from)?;

            let kind_str = match inserted.kind {
                UnitKind::Integer => "integer",
                UnitKind::Decimal => "decimal",
            };
            (inserted.id, kind_str.to_string())
        }
    };

    let result = sqlx::query(
        "UPDATE products
         SET default_unit_id = $1, unit_type = $2, default_unit = $3, updated_at = $4
         WHERE default_unit_id IS NULL
           AND default_unit IS NOT NULL
           AND TRIM(default_unit) <> ''
           AND TRIM(lower(default_unit)) = TRIM(lower($5))",
    )
    .bind(&unit_id)
    .bind(&kind)
    .bind(&display_name)
    .bind(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    .bind(raw_value)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(UnitReviewActionResult {
        updated_product_count: result.rows_affected() as usize,
    })
}

/// Converts a string to a lowercase slug suitable as a unit key.
fn slugify(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(16)
        .collect()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::unit_definitions::UnitReviewAction;
    use crate::services::unit_audit as svc;

    async fn seed_product_with_raw_unit(
        pool: &crate::db::DbPool,
        id: &str,
        sku: &str,
        raw_unit: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "INSERT INTO products (id, sku, description, default_unit, default_alert_days_before, is_active, created_at, updated_at)
             VALUES ($1, $2, 'Test', $3, 30, 1, $4, $5)",
        )
        .bind(id)
        .bind(sku)
        .bind(raw_unit)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn unrecognized_units_groups_by_raw_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        seed_product_with_raw_unit(&pool, "p1", "SKU-1", "kg.").await?;
        seed_product_with_raw_unit(&pool, "p2", "SKU-2", "kg.").await?;
        seed_product_with_raw_unit(&pool, "p3", "SKU-3", "litross").await?;

        let groups = svc::unrecognized_units(&pool).await?;
        assert_eq!(groups.len(), 2);

        let kg_dot = groups.iter().find(|g| g.raw_value == "kg.").unwrap();
        assert_eq!(kg_dot.product_count, 2);

        let litross = groups.iter().find(|g| g.raw_value == "litross").unwrap();
        assert_eq!(litross.product_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn current_signature_changes_when_new_unknown_unit_appears(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        seed_product_with_raw_unit(&pool, "p1", "SKU-1", "kg.").await?;
        let sig1 = svc::current_signature(&pool).await?;

        seed_product_with_raw_unit(&pool, "p2", "SKU-2", "litross").await?;
        let sig2 = svc::current_signature(&pool).await?;

        assert_ne!(
            sig1, sig2,
            "signature must change when unrecognized units change"
        );
        assert_eq!(sig1.len(), 16);
        assert_eq!(sig2.len(), 16);
        Ok(())
    }

    #[tokio::test]
    async fn banner_is_hidden_when_signature_matches() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        seed_product_with_raw_unit(&pool, "p1", "SKU-1", "kg.").await?;

        // Initially visible (no dismissal).
        let state = svc::banner_state(&pool).await?;
        assert!(state.visible);

        // Dismiss the banner.
        svc::dismiss_banner(&pool).await?;

        // Now hidden.
        let state2 = svc::banner_state(&pool).await?;
        assert!(!state2.visible, "banner should be hidden after dismissal");
        Ok(())
    }

    #[tokio::test]
    async fn dismiss_banner_persists_signature() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        seed_product_with_raw_unit(&pool, "p1", "SKU-1", "kg.").await?;

        let sig = svc::current_signature(&pool).await?;
        svc::dismiss_banner(&pool).await?;

        // Banner state should show hidden.
        let state = svc::banner_state(&pool).await?;
        assert_eq!(state.signature, sig);
        assert!(!state.visible);
        Ok(())
    }

    #[tokio::test]
    async fn apply_review_action_map_to_preset_assigns_default_unit_id(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Product with unrecognized "kg." maps to preset "ud-kg".
        seed_product_with_raw_unit(&pool, "p1", "SKU-KG", "kg.").await?;

        let result = svc::apply_review_action(
            &pool,
            UnitReviewAction::MapToPreset {
                raw_value: "kg.".into(),
                preset_id: "ud-kg".into(),
            },
        )
        .await?;

        assert_eq!(result.updated_product_count, 1);

        // Verify the product is now linked.
        let row: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT default_unit_id, unit_type FROM products WHERE id = 'p1'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(row.0, Some("ud-kg".to_string()));
        assert_eq!(row.1, Some("decimal".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn apply_review_action_keep_as_custom_creates_and_assigns_unit(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        seed_product_with_raw_unit(&pool, "p1", "SKU-BAND", "bandejas").await?;
        seed_product_with_raw_unit(&pool, "p2", "SKU-BAND2", "bandejas").await?;

        let result = svc::apply_review_action(
            &pool,
            UnitReviewAction::KeepAsCustom {
                raw_value: "bandejas".into(),
            },
        )
        .await?;

        assert_eq!(result.updated_product_count, 2);

        // Verify a custom unit was created and both products are linked.
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM unit_definitions WHERE key = 'bandejas'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(count.0, 1);

        let linked: (Option<String>,) =
            sqlx::query_as("SELECT default_unit_id FROM products WHERE id = 'p1'")
                .fetch_one(&pool)
                .await?;
        assert!(linked.0.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn apply_review_action_leave_for_later_is_noop() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        seed_product_with_raw_unit(&pool, "p1", "SKU-X", "xyz-unknown").await?;

        let result = svc::apply_review_action(
            &pool,
            UnitReviewAction::LeaveForLater {
                raw_value: "xyz-unknown".into(),
            },
        )
        .await?;

        assert_eq!(result.updated_product_count, 0);

        // Product is still unlinked.
        let row: (Option<String>,) =
            sqlx::query_as("SELECT default_unit_id FROM products WHERE id = 'p1'")
                .fetch_one(&pool)
                .await?;
        assert!(row.0.is_none());
        Ok(())
    }
}
