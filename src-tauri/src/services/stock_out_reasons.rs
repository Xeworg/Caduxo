//! Stock-out reasons catalog service.
//!
//! Business rules for listing, creating, renaming, archiving, and unarchiving reasons.

#![allow(dead_code)]

use uuid::Uuid;

use crate::db::repositories::stock_out_reasons as repo;
use crate::db::DbPool;
use crate::domain::stock_out_reasons as domain;
use crate::dto::stock_out_reasons::{
    StockOutReasonCreateInput, StockOutReasonRenameInput, StockOutReasonResponse,
};
use crate::error::{AppError, DomainError};

/// Maximum display name length enforced at the service layer.
const MAX_DISPLAY_NAME_LEN: usize = 100;

/// Lists all active (non-archived) reasons ordered by sort_order.
pub async fn list_active(pool: &DbPool) -> Result<Vec<StockOutReasonResponse>, AppError> {
    repo::list_active(pool).await.map_err(AppError::from)
}

/// Lists all reasons including archived ones, ordered by sort_order.
pub async fn list_all(pool: &DbPool) -> Result<Vec<StockOutReasonResponse>, AppError> {
    repo::list_all(pool).await.map_err(AppError::from)
}

/// Finds a reason by its `id`. Returns `None` if not found.
pub async fn find_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<StockOutReasonResponse>, AppError> {
    repo::find_by_id(pool, id).await.map_err(AppError::from)
}

/// Creates a new custom stock-out reason.
///
/// Validation rules:
/// - `display_name`: non-empty, max 100 chars
/// - `movement_kind`: must be one of the seven V20 stock-out kinds
/// - Display name must not conflict with any active reason (case-insensitive)
/// - ID uses UUID suffix to avoid collision with seeded `sor-*` IDs
pub async fn create_custom_reason(
    pool: &DbPool,
    input: StockOutReasonCreateInput,
) -> Result<StockOutReasonResponse, AppError> {
    // 1. Validate display_name.
    let display_name = input.display_name.trim().to_string();
    domain::validate_display_name(&display_name).map_err(AppError::Domain)?;
    domain::validate_display_name_max_len(&display_name, MAX_DISPLAY_NAME_LEN)
        .map_err(AppError::Domain)?;

    // 2. Validate movement_kind (closed enum).
    let movement_kind = input.movement_kind.trim().to_string();
    domain::validate_movement_kind_strict(&movement_kind).map_err(AppError::Domain)?;

    // 3. Check for duplicate active display name (case-insensitive).
    if let Some(existing) = repo::find_active_by_display_name_ci(pool, &display_name)
        .await
        .map_err(AppError::from)?
    {
        return Err(AppError::Domain(DomainError::DuplicateField {
            field: "display_name",
            value: existing.display_name,
        }));
    }

    // 4. Derive sort_order: use explicit value, or compute from max existing + 1.
    let sort_order = match input.sort_order {
        Some(n) => n,
        None => {
            repo::max_sort_order_for_kind(pool, &movement_kind)
                .await
                .map_err(AppError::from)?
                .unwrap_or(0)
                + 1
        }
    };

    // 5. Generate a collision-free custom ID using UUID suffix.
    //    UUID format ensures it cannot collide with `sor-*` seeded IDs.
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = format!("sor-{}", Uuid::new_v4());

    repo::insert(
        pool,
        &id,
        &display_name,
        &movement_kind,
        sort_order,
        &now,
        &now,
    )
    .await
    .map_err(AppError::from)
}

/// Renames a stock-out reason's display_name.
///
/// Validation rules:
/// - `id` must exist (active or archived)
/// - `display_name`: non-empty, max 100 chars
/// - New display name must not conflict with any active reason (case-insensitive),
///   excluding the reason being renamed (self-rename is always allowed).
pub async fn rename_reason(
    pool: &DbPool,
    input: StockOutReasonRenameInput,
) -> Result<StockOutReasonResponse, AppError> {
    // 1. Validate display_name.
    let display_name = input.display_name.trim().to_string();
    domain::validate_display_name(&display_name).map_err(AppError::Domain)?;
    domain::validate_display_name_max_len(&display_name, MAX_DISPLAY_NAME_LEN)
        .map_err(AppError::Domain)?;

    // 2. Ensure the reason exists.
    let existing = repo::find_by_id(pool, &input.id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "stock_out_reason",
            id: input.id.clone(),
        })?;

    // 3. Check for duplicate active display name, excluding self.
    if let Some(other) = repo::find_active_by_display_name_ci(pool, &display_name)
        .await
        .map_err(AppError::from)?
    {
        if other.id != existing.id {
            return Err(AppError::Domain(DomainError::DuplicateField {
                field: "display_name",
                value: other.display_name,
            }));
        }
    }

    // 4. Perform the update (movement_kind is NOT changed).
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let updated = repo::update_display_name(pool, &input.id, &display_name, &now)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "stock_out_reason",
            id: input.id,
        })?;

    Ok(updated)
}

/// Archives a stock-out reason. Sets `archived_at`; does NOT hard delete.
pub async fn archive_reason(pool: &DbPool, id: String) -> Result<(), AppError> {
    // 1. Ensure the reason exists and is active.
    let found = repo::find_by_id(pool, &id).await.map_err(AppError::from)?;
    match found {
        None => {
            return Err(AppError::Domain(DomainError::NotFound {
                resource: "stock_out_reason",
                id,
            }));
        }
        Some(r) if r.archived_at.is_some() => {
            return Err(AppError::Domain(DomainError::NotFound {
                resource: "stock_out_reason",
                id,
            }));
        }
        Some(_) => {}
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    repo::archive(pool, &id, &now)
        .await
        .map_err(AppError::from)?;

    Ok(())
}

/// Unarchives a stock-out reason. Clears `archived_at`.
///
/// Validation:
/// - `id` must be an archived reason
/// - Display name must not conflict with any active reason (case-insensitive)
pub async fn unarchive_reason(
    pool: &DbPool,
    id: String,
) -> Result<StockOutReasonResponse, AppError> {
    // 1. Fetch the archived reason.
    let archived = repo::find_by_id(pool, &id)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "stock_out_reason",
            id: id.clone(),
        })?;

    // Must be archived.
    if archived.archived_at.is_none() {
        return Err(AppError::Domain(DomainError::NotFound {
            resource: "stock_out_reason",
            id,
        }));
    }

    // 2. Check for display_name conflict with active reasons.
    if let Some(active) = repo::find_active_by_display_name_ci(pool, &archived.display_name)
        .await
        .map_err(AppError::from)?
    {
        return Err(AppError::Domain(DomainError::DuplicateField {
            field: "display_name",
            value: active.display_name,
        }));
    }

    // 3. Restore.
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    repo::unarchive(pool, &archived.id, &now)
        .await
        .map_err(AppError::from)?;

    // Return the restored reason.
    Ok(StockOutReasonResponse {
        archived_at: None,
        ..archived
    })
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::stock_out_reasons::{StockOutReasonCreateInput, StockOutReasonRenameInput};
    use crate::error::AppError;
    use crate::services::stock_out_reasons as svc;

    // ── Create ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_custom_reason_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let reason = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Descarte fino".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        assert!(reason.id.starts_with("sor-"));
        assert_eq!(reason.display_name, "Descarte fino");
        assert_eq!(reason.movement_kind, "exit:waste");
        assert!(reason.archived_at.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_reason_with_explicit_sort_order(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let reason = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Otro descarte".into(),
                movement_kind: "exit:waste".into(),
                sort_order: Some(999),
            },
        )
        .await?;
        assert_eq!(reason.sort_order, 999);
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_reason_rejects_invalid_movement_kind(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Test".into(),
                movement_kind: "exit:sale".into(), // not a stock-out kind
                sort_order: None,
            },
        )
        .await
        .expect_err("invalid movement_kind must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_reason_rejects_unknown_movement_kind(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Test".into(),
                movement_kind: "unknown:kind".into(),
                sort_order: None,
            },
        )
        .await
        .expect_err("unknown movement_kind must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_reason_rejects_empty_display_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "   ".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await
        .expect_err("empty display_name must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_reason_rejects_duplicate_active_display_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Create first reason.
        svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón duplicada".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        // Try to create second with same display name (case-insensitive).
        let err = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón Duplicada".into(), // same name, different case
                movement_kind: "exit:expired".into(),
                sort_order: None,
            },
        )
        .await
        .expect_err("duplicate active display_name must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "display_name");
                assert_eq!(value, "Razón duplicada");
            }
            other => panic!("expected DuplicateField, got {other:?}"),
        }
        Ok(())
    }

    // ── Rename ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn rename_reason_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Viejo nombre".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        let renamed = svc::rename_reason(
            &pool,
            StockOutReasonRenameInput {
                id: created.id.clone(),
                display_name: "Nuevo nombre".into(),
            },
        )
        .await?;

        assert_eq!(renamed.display_name, "Nuevo nombre");
        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.movement_kind, "exit:waste"); // kind unchanged
        Ok(())
    }

    #[tokio::test]
    async fn rename_reason_kind_immutable() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón waste".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        // Rename — movement_kind stays the same.
        let renamed = svc::rename_reason(
            &pool,
            StockOutReasonRenameInput {
                id: created.id.clone(),
                display_name: "Razón renombrada".into(),
            },
        )
        .await?;

        assert_eq!(renamed.movement_kind, "exit:waste");
        Ok(())
    }

    #[tokio::test]
    async fn rename_reason_missing_id_returns_not_found() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let err = svc::rename_reason(
            &pool,
            StockOutReasonRenameInput {
                id: "nonexistent-id".into(),
                display_name: "New Name".into(),
            },
        )
        .await
        .expect_err("rename missing ID must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::NotFound { resource, id }) => {
                assert_eq!(resource, "stock_out_reason");
                assert_eq!(id, "nonexistent-id");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn rename_reason_duplicate_active_display_name() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;

        let r1 = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón uno".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        let r2 = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón dos".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        // Try to rename r2 to r1's display name.
        let err = svc::rename_reason(
            &pool,
            StockOutReasonRenameInput {
                id: r2.id.clone(),
                display_name: "Razón uno".into(), // conflicts with r1
            },
        )
        .await
        .expect_err("duplicate active display_name must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "display_name");
                assert_eq!(value, "Razón uno");
            }
            other => panic!("expected DuplicateField, got {other:?}"),
        }
        Ok(())
    }

    // ── Archive ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn archive_reason_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Para archivar".into(),
                movement_kind: "exit:expired".into(),
                sort_order: None,
            },
        )
        .await?;

        svc::archive_reason(&pool, created.id.clone()).await?;

        // Archived reason must not appear in active list.
        let active = svc::list_active(&pool).await?;
        assert!(
            active.iter().all(|r| r.id != created.id),
            "archived reason should not appear in active list"
        );
        Ok(())
    }

    #[tokio::test]
    async fn archive_reason_missing_id_returns_not_found() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;
        let err = svc::archive_reason(&pool, "nonexistent-id".into())
            .await
            .expect_err("archive missing ID must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::NotFound { resource, id }) => {
                assert_eq!(resource, "stock_out_reason");
                assert_eq!(id, "nonexistent-id");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn archive_reason_already_archived_returns_not_found(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Ya archivada".into(),
                movement_kind: "exit:damaged".into(),
                sort_order: None,
            },
        )
        .await?;

        svc::archive_reason(&pool, created.id.clone()).await?;
        let err = svc::archive_reason(&pool, created.id.clone())
            .await
            .expect_err("archive already-archived reason must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
        Ok(())
    }

    // ── Unarchive ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn unarchive_reason_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Para desarchivar".into(),
                movement_kind: "exit:damaged".into(),
                sort_order: None,
            },
        )
        .await?;

        svc::archive_reason(&pool, created.id.clone()).await?;
        let restored = svc::unarchive_reason(&pool, created.id.clone()).await?;

        assert!(restored.archived_at.is_none());
        assert_eq!(restored.id, created.id);

        // Reason reappears in active list.
        let active = svc::list_active(&pool).await?;
        assert!(active.iter().any(|r| r.id == created.id));
        Ok(())
    }

    #[tokio::test]
    async fn unarchive_reason_missing_id_returns_not_found(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::unarchive_reason(&pool, "nonexistent-id".into())
            .await
            .expect_err("unarchive missing ID must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::NotFound { resource, id }) => {
                assert_eq!(resource, "stock_out_reason");
                assert_eq!(id, "nonexistent-id");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn unarchive_reason_active_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Ya activa".into(),
                movement_kind: "exit:damaged".into(),
                sort_order: None,
            },
        )
        .await?;

        // Unarchiving an active (non-archived) reason must fail.
        let err = svc::unarchive_reason(&pool, created.id.clone())
            .await
            .expect_err("unarchive active reason must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::NotFound { .. }) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn unarchive_reason_duplicate_active_display_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Create two reasons.
        let r1 = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón activa".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        let r2 = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón archivada".into(),
                movement_kind: "exit:waste".into(),
                sort_order: None,
            },
        )
        .await?;

        // Archive r2.
        svc::archive_reason(&pool, r2.id.clone()).await?;

        // Rename r1 to match r2's display name.
        svc::rename_reason(
            &pool,
            StockOutReasonRenameInput {
                id: r1.id.clone(),
                display_name: "Razón archivada".into(),
            },
        )
        .await?;

        // Now unarchiving r2 should fail because "Razón archivada" is taken.
        let err = svc::unarchive_reason(&pool, r2.id.clone())
            .await
            .expect_err("unarchive with conflicting active display_name must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "display_name");
                assert_eq!(value, "Razón archivada");
            }
            other => panic!("expected DuplicateField, got {other:?}"),
        }
        Ok(())
    }

    // ── List ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_active_excludes_archived() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón listar".into(),
                movement_kind: "exit:damaged".into(),
                sort_order: None,
            },
        )
        .await?;

        let count_before = svc::list_active(&pool).await?.len();
        svc::archive_reason(&pool, created.id.clone()).await?;
        let count_after = svc::list_active(&pool).await?.len();

        assert_eq!(count_after, count_before - 1);
        Ok(())
    }

    #[tokio::test]
    async fn list_all_includes_archived() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón listar todas".into(),
                movement_kind: "exit:damaged".into(),
                sort_order: None,
            },
        )
        .await?;

        svc::archive_reason(&pool, created.id.clone()).await?;

        let all = svc::list_all(&pool).await?;
        assert!(
            all.iter()
                .any(|r| r.id == created.id && r.archived_at.is_some()),
            "archived reason must appear in list_all"
        );
        Ok(())
    }

    #[tokio::test]
    async fn find_by_id_returns_none_for_missing() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let found = svc::find_by_id(&pool, "nonexistent").await?;
        assert!(found.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn find_by_id_returns_archived() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = svc::create_custom_reason(
            &pool,
            StockOutReasonCreateInput {
                display_name: "Razón encontrar".into(),
                movement_kind: "exit:damaged".into(),
                sort_order: None,
            },
        )
        .await?;

        svc::archive_reason(&pool, created.id.clone()).await?;
        let found = svc::find_by_id(&pool, &created.id).await?;
        assert!(found.is_some());
        assert!(found.unwrap().archived_at.is_some());
        Ok(())
    }
}
