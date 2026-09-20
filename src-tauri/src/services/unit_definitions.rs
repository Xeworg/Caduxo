//! Unit definitions catalog service.
//!
//! Business rules for listing, creating, renaming, and archiving units.

#![allow(dead_code)]

use crate::db::repositories::unit_definitions as repo;
use crate::db::DbPool;
use crate::dto::unit_definitions::{
    UnitDefinitionCreateInput, UnitDefinitionRenameInput, UnitDefinitionResponse,
};
use crate::error::{AppError, DomainError};
use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

/// Key validation: 1–16 chars, lowercase alphanumeric plus hyphen/underscore.
fn validate_unit_key(key: &str) -> Result<(), DomainError> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Validation {
            message: "Unit key cannot be empty".to_string(),
        });
    }
    if trimmed.len() > 16 {
        return Err(DomainError::Validation {
            message: "Unit key must be at most 16 characters".to_string(),
        });
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(DomainError::Validation {
            message: "Unit key must contain only letters, numbers, hyphens, and underscores"
                .to_string(),
        });
    }
    // Keys are stored lowercase.
    if trimmed.contains(|c: char| c.is_ascii_uppercase()) {
        return Err(DomainError::Validation {
            message: "Unit key must be lowercase".to_string(),
        });
    }
    Ok(())
}

/// Display name validation: non-empty when trimmed.
fn validate_display_name(name: &str) -> Result<(), DomainError> {
    if name.trim().is_empty() {
        return Err(DomainError::Validation {
            message: "Display name cannot be empty".to_string(),
        });
    }
    Ok(())
}

/// Lists all active (non-archived) unit definitions ordered by kind then display_name.
pub async fn list_active(pool: &DbPool) -> Result<Vec<UnitDefinitionResponse>, AppError> {
    repo::list_active(pool).await.map_err(AppError::from)
}

/// Finds a unit by key (case-insensitive).
pub async fn find_by_key(
    pool: &DbPool,
    key: &str,
) -> Result<Option<UnitDefinitionResponse>, AppError> {
    repo::find_by_key(pool, key).await.map_err(AppError::from)
}

/// Creates a new custom (non-preset) unit. Rejects duplicate keys (case-insensitive)
/// and validates key format and display name.
pub async fn create_custom_unit(
    pool: &DbPool,
    input: UnitDefinitionCreateInput,
) -> Result<UnitDefinitionResponse, AppError> {
    // Validate the raw input first (before lowercasing) to catch uppercase keys.
    validate_unit_key(&input.key).map_err(AppError::Domain)?;
    let key = input.key.trim().to_lowercase();
    let display_name = input.display_name.trim().to_string();
    validate_display_name(&display_name).map_err(AppError::Domain)?;

    // Check for duplicate key (case-insensitive).
    if let Some(existing) = repo::find_by_key(pool, &key)
        .await
        .map_err(AppError::from)?
    {
        return Err(AppError::Domain(DomainError::DuplicateField {
            field: "key",
            value: existing.key,
        }));
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = format!("ud-{}", &key);

    repo::insert(
        pool,
        &id,
        &key,
        &display_name,
        input.kind,
        false,
        &now,
        &now,
    )
    .await
    .map_err(AppError::from)
}

/// Renames a unit's display_name (key is immutable). Works for both presets and custom units.
pub async fn rename_unit(
    pool: &DbPool,
    input: UnitDefinitionRenameInput,
) -> Result<UnitDefinitionResponse, AppError> {
    let display_name = input.display_name.trim().to_string();
    validate_display_name(&display_name).map_err(AppError::Domain)?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let row = repo::rename(pool, &input.id, &display_name, &now)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "unit_definition",
            id: input.id,
        })?;

    Ok(row)
}

/// Archives a unit. Blocked when any product still references it.
pub async fn archive_unit(pool: &DbPool, id: String) -> Result<(), AppError> {
    // Check if the unit exists.
    let found = repo::find_by_id(pool, &id).await.map_err(AppError::from)?;
    if found.is_none() {
        return Err(AppError::Domain(DomainError::NotFound {
            resource: "unit_definition",
            id,
        }));
    }

    // Check if referenced.
    if repo::is_referenced(pool, &id)
        .await
        .map_err(AppError::from)?
    {
        return Err(AppError::Domain(DomainError::BusinessRule {
            message: user_message(UserMessage::UnitStillReferenced, Locale::En),
        }));
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    repo::archive(pool, &id, &now)
        .await
        .map_err(AppError::from)?;

    Ok(())
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::unit_definitions::{
        UnitDefinitionCreateInput, UnitDefinitionRenameInput, UnitKind,
    };
    use crate::error::AppError;
    use crate::services::unit_definitions as svc;

    #[tokio::test]
    async fn create_custom_unit_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let unit = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "bandejas".into(),
                display_name: "Bandejas".into(),
                kind: UnitKind::Integer,
            },
        )
        .await?;

        assert_eq!(unit.key, "bandejas");
        assert_eq!(unit.display_name, "Bandejas");
        assert_eq!(unit.kind, UnitKind::Integer);
        assert!(!unit.is_preset);
        assert!(unit.archived_at.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_unit_rejects_duplicate_key() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Create a custom unit with key "bandejas".
        svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "bandejas".into(),
                display_name: "Bandejas".into(),
                kind: UnitKind::Integer,
            },
        )
        .await?;

        // Try to create another with the same key (case-insensitive).
        // Use a lowercase duplicate to avoid the uppercase validation catching it first.
        let err = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "bandejas".into(), // exact duplicate key
                display_name: "Bandejas 2".into(),
                kind: UnitKind::Decimal,
            },
        )
        .await
        .expect_err("duplicate key must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::DuplicateField { field, value }) => {
                assert_eq!(field, "key");
                assert_eq!(value, "bandejas");
            }
            other => panic!("expected DuplicateField, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_unit_rejects_empty_display_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "bandejas".into(),
                display_name: "   ".into(),
                kind: UnitKind::Integer,
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
    async fn create_custom_unit_rejects_uppercase_key() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "Bandejas".into(), // uppercase
                display_name: "Bandejas".into(),
                kind: UnitKind::Integer,
            },
        )
        .await
        .expect_err("uppercase key must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn create_custom_unit_rejects_too_long_key() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let err = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "this-key-is-way-too-long".into(), // > 16 chars
                display_name: "Too Long".into(),
                kind: UnitKind::Integer,
            },
        )
        .await
        .expect_err("too-long key must be rejected");
        assert!(matches!(
            err,
            AppError::Domain(crate::error::DomainError::Validation { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn rename_unit_updates_display_name() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let unit = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "bandejas".into(),
                display_name: "Bandejas".into(),
                kind: UnitKind::Integer,
            },
        )
        .await?;

        let renamed = svc::rename_unit(
            &pool,
            UnitDefinitionRenameInput {
                id: unit.id.clone(),
                display_name: "Bandejas de cartón".into(),
            },
        )
        .await?;

        assert_eq!(renamed.display_name, "Bandejas de cartón");
        assert_eq!(
            renamed.key, "bandejas",
            "key must remain stable after rename"
        );
        Ok(())
    }

    #[tokio::test]
    async fn rename_unit_presets_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Rename a preset unit.
        let renamed = svc::rename_unit(
            &pool,
            UnitDefinitionRenameInput {
                id: "ud-kg".into(),
                display_name: "Kilogramo (kg)".into(),
            },
        )
        .await?;

        assert_eq!(renamed.id, "ud-kg");
        assert_eq!(renamed.display_name, "Kilogramo (kg)");
        assert!(renamed.is_preset);
        Ok(())
    }

    #[tokio::test]
    async fn archive_unit_succeeds_when_unreferenced() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let unit = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "temperatura".into(),
                display_name: "Temperatura".into(),
                kind: UnitKind::Decimal,
            },
        )
        .await?;

        svc::archive_unit(&pool, unit.id.clone()).await?;

        // Archived unit should not appear in active list.
        let active = svc::list_active(&pool).await?;
        assert!(
            active.iter().all(|u| u.id != unit.id),
            "archived unit should not appear in active list"
        );
        Ok(())
    }

    #[tokio::test]
    async fn archive_unit_referenced_returns_business_rule_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;

        // Create a custom unit and assign it to a product.
        let unit = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "bandejas2".into(),
                display_name: "Bandejas 2".into(),
                kind: UnitKind::Integer,
            },
        )
        .await?;

        // Create a product using this unit.
        crate::services::products::create_product(
            &pool,
            crate::dto::products::ProductCreate {
                sku: "SKU-BANDEJAS".into(),
                description: "Test product with custom unit".into(),
                category_ids: None,
                default_unit: None,
                default_unit_id: Some(unit.id.clone()),
                default_alert_days_before: 30,
                notes: None,
            },
        )
        .await?;

        let err = svc::archive_unit(&pool, unit.id.clone())
            .await
            .expect_err("archiving referenced unit must be rejected");

        match err {
            AppError::Domain(crate::error::DomainError::BusinessRule { message }) => {
                assert!(message.contains("still referenced"));
            }
            other => panic!("expected BusinessRule, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test]
    async fn list_active_units_excludes_archived() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let unit = svc::create_custom_unit(
            &pool,
            UnitDefinitionCreateInput {
                key: "to-archive".into(),
                display_name: "To Archive".into(),
                kind: UnitKind::Decimal,
            },
        )
        .await?;

        let count_before = svc::list_active(&pool).await?.len();
        svc::archive_unit(&pool, unit.id.clone()).await?;
        let count_after = svc::list_active(&pool).await?.len();

        assert_eq!(count_after, count_before - 1);
        Ok(())
    }

    #[tokio::test]
    async fn suggest_similar_debug() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        // Test what suggestions each misspelling actually produces.
        for input in ["litro", "gram", "grams", "kgram", "liter", "litre", "milli"] {
            let suggestions =
                crate::db::repositories::unit_definitions::suggest_similar(&pool, input, 3).await?;
            eprintln!("Input: {input}, Suggestions: {suggestions:?}");
        }
        // Always pass — this is just a diagnostic.
        Ok(())
    }
}
