//! Store and internal location management service.

use crate::db::repositories::stores as repo;
use crate::db::DbPool;
use crate::domain::validation::validate_name;
use crate::dto::stores::{
    StoreCreate, StoreLocationCreate, StoreLocationResponse, StoreLocationUpdate, StoreResponse,
    StoreUpdate,
};
use crate::error::{AppError, DomainError};

/// Returns true if the database has at least one active store.
pub async fn is_first_run(pool: &DbPool) -> Result<bool, AppError> {
    let count = repo::active_store_count(pool).await?;
    Ok(count == 0)
}

/// Creates a new store after validating the name.
pub async fn create_store(pool: &DbPool, input: StoreCreate) -> Result<StoreResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    repo::insert_store(pool, &input)
        .await
        .map_err(AppError::from)
}

/// Lists all active stores ordered by name.
pub async fn list_stores(pool: &DbPool) -> Result<Vec<StoreResponse>, AppError> {
    repo::list_active_stores(pool).await.map_err(AppError::from)
}

/// Lists all stores (active and inactive).
#[allow(dead_code)]
pub async fn list_all_stores(pool: &DbPool) -> Result<Vec<StoreResponse>, AppError> {
    repo::list_all_stores(pool).await.map_err(AppError::from)
}

/// Gets a single store by id.
#[allow(dead_code)]
pub async fn get_store(pool: &DbPool, id: &str) -> Result<Option<StoreResponse>, AppError> {
    repo::get_store(pool, id).await.map_err(AppError::from)
}

/// Updates an existing store.
pub async fn update_store(pool: &DbPool, input: StoreUpdate) -> Result<StoreResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    let row = repo::update_store(pool, &input)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "store",
            id: input.id,
        })?;
    Ok(row)
}

/// Creates a new internal location under a store.
pub async fn create_location(
    pool: &DbPool,
    input: StoreLocationCreate,
) -> Result<StoreLocationResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;

    let store = repo::get_store(pool, &input.store_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| DomainError::NotFound {
            resource: "store",
            id: input.store_id.clone(),
        })?;

    if !store.is_active {
        return Err(DomainError::BusinessRule {
            message: "Cannot add location to an inactive store".to_string(),
        }
        .into());
    }

    repo::insert_location(pool, &input)
        .await
        .map_err(AppError::from)
}

/// Lists active locations for a given store.
pub async fn list_locations(
    pool: &DbPool,
    store_id: &str,
) -> Result<Vec<StoreLocationResponse>, AppError> {
    repo::list_active_locations(pool, store_id)
        .await
        .map_err(AppError::from)
}

/// Lists all locations (active and inactive) for a given store.
#[allow(dead_code)]
pub async fn list_all_locations(
    pool: &DbPool,
    store_id: &str,
) -> Result<Vec<StoreLocationResponse>, AppError> {
    repo::list_all_locations(pool, store_id)
        .await
        .map_err(AppError::from)
}

/// Updates an existing location.
pub async fn update_location(
    pool: &DbPool,
    input: StoreLocationUpdate,
) -> Result<StoreLocationResponse, AppError> {
    validate_name(&input.name).map_err(|m| DomainError::Validation { message: m })?;
    let row = repo::update_location(pool, &input)
        .await
        .map_err(AppError::from)?
        .ok_or(DomainError::NotFound {
            resource: "store_location",
            id: input.id,
        })?;
    Ok(row)
}

/// Returns true if at least one active store exists.
pub async fn has_store(pool: &DbPool) -> Result<bool, AppError> {
    repo::has_active_store(pool).await.map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    // NOTE: avoid `use super::*` — it would bring `fresh_test_pool` (if any)
    // into scope from the parent module.
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::stores::StoreLocationCreate;
    use crate::dto::stores::{StoreCreate, StoreLocationUpdate, StoreUpdate};
    use crate::services::stores::{
        create_location, create_store, has_store, is_first_run, list_locations, list_stores,
        update_location, update_store,
    };

    #[tokio::test]
    async fn is_first_run_true_when_no_stores() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let result = is_first_run(&pool).await?;
        assert!(result, "should be first run when no stores exist");
        Ok(())
    }

    #[tokio::test]
    async fn is_first_run_false_after_store_created() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        create_store(
            &pool,
            StoreCreate {
                name: "My Shop".into(),
                code: Some("SHOP-01".into()),
                notes: None,
            },
        )
        .await?;
        let result = is_first_run(&pool).await?;
        assert!(!result, "should not be first run after store is created");
        Ok(())
    }

    #[tokio::test]
    async fn create_store_validates_empty_name() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let result = create_store(
            &pool,
            StoreCreate {
                name: "".into(),
                code: None,
                notes: None,
            },
        )
        .await;
        assert!(result.is_err(), "creating store with empty name must fail");
        Ok(())
    }

    #[tokio::test]
    async fn create_and_list_stores() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        create_store(
            &pool,
            StoreCreate {
                name: "Alpha".into(),
                code: Some("A".into()),
                notes: None,
            },
        )
        .await?;
        create_store(
            &pool,
            StoreCreate {
                name: "Beta".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        let stores = list_stores(&pool).await?;
        assert_eq!(stores.len(), 2);
        assert_eq!(stores[0].name, "Alpha");
        assert_eq!(stores[1].name, "Beta");
        Ok(())
    }

    #[tokio::test]
    async fn update_store_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let created = create_store(
            &pool,
            StoreCreate {
                name: "Original".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        let updated = update_store(
            &pool,
            StoreUpdate {
                id: created.id,
                name: "Renamed".into(),
                code: Some("NEW-CODE".into()),
                notes: None,
                is_active: true,
            },
        )
        .await?;
        assert_eq!(updated.name, "Renamed");
        Ok(())
    }

    #[tokio::test]
    async fn create_and_list_locations() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store = create_store(
            &pool,
            StoreCreate {
                name: "Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        create_location(
            &pool,
            StoreLocationCreate {
                store_id: store.id.clone(),
                name: "Fridge".into(),
                notes: None,
            },
        )
        .await?;
        create_location(
            &pool,
            StoreLocationCreate {
                store_id: store.id.clone(),
                name: "Freezer".into(),
                notes: None,
            },
        )
        .await?;

        let locations = list_locations(&pool, &store.id).await?;
        assert_eq!(locations.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn create_location_validates_empty_name() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store = create_store(
            &pool,
            StoreCreate {
                name: "Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        let result = create_location(
            &pool,
            StoreLocationCreate {
                store_id: store.id,
                name: "".into(),
                notes: None,
            },
        )
        .await;
        assert!(result.is_err(), "location with empty name must be rejected");
        Ok(())
    }

    #[tokio::test]
    async fn create_location_rejects_inactive_store() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store = create_store(
            &pool,
            StoreCreate {
                name: "Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        // Deactivate the store
        update_store(
            &pool,
            StoreUpdate {
                id: store.id.clone(),
                name: "Store".into(),
                code: None,
                notes: None,
                is_active: false,
            },
        )
        .await?;

        let result = create_location(
            &pool,
            StoreLocationCreate {
                store_id: store.id,
                name: "Section".into(),
                notes: None,
            },
        )
        .await;
        assert!(result.is_err(), "cannot add location to inactive store");
        Ok(())
    }

    #[tokio::test]
    async fn update_location_works() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store = create_store(
            &pool,
            StoreCreate {
                name: "Store".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        let loc = create_location(
            &pool,
            StoreLocationCreate {
                store_id: store.id.clone(),
                name: "Section 1".into(),
                notes: None,
            },
        )
        .await?;

        let updated = update_location(
            &pool,
            StoreLocationUpdate {
                id: loc.id,
                store_id: store.id,
                name: "Section One".into(),
                notes: None,
                is_active: true,
            },
        )
        .await?;
        assert_eq!(updated.name, "Section One");
        Ok(())
    }

    /// Full first-run setup flow regression test.
    ///
    /// Exercises the complete onboarding sequence:
    /// 1. Fresh DB → is_first_run = true, has_store = false
    /// 2. First store created → is_first_run = false, has_store = true
    /// 3. Settings remain empty (no auto-selection on first run)
    #[tokio::test]
    async fn first_run_setup_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
        use crate::dto::stores::SettingsUpdate;
        use crate::services::settings::{get_settings, update_settings};

        let pool = fresh_test_pool().await?;

        // Step 1: fresh database is first run
        assert!(is_first_run(&pool).await?, "fresh DB should be first run");
        assert!(!has_store(&pool).await?, "fresh DB has no store");

        // Settings are empty on fresh DB
        let settings = get_settings(&pool).await?;
        assert!(
            settings.last_selected_store_id.is_none(),
            "fresh DB settings should have no selected store"
        );

        // Step 2: creating the first store ends the first-run state
        let store = create_store(
            &pool,
            StoreCreate {
                name: "My First Shop".into(),
                code: Some("SHOP-001".into()),
                notes: None,
            },
        )
        .await?;

        assert!(
            !is_first_run(&pool).await?,
            "first run ends after store creation"
        );
        assert!(has_store(&pool).await?, "store now exists");

        // Step 3: settings are still empty (no auto-selection)
        let settings = get_settings(&pool).await?;
        assert!(
            settings.last_selected_store_id.is_none(),
            "settings should not auto-select store on first run"
        );

        // Step 4: manually selecting the store updates settings
        update_settings(
            &pool,
            SettingsUpdate {
                last_selected_store_id: Some(store.id.clone()),
                require_initial_location_on_lot_create: None,
                language: None,
            },
        )
        .await?;

        let settings = get_settings(&pool).await?;
        assert_eq!(
            settings.last_selected_store_id,
            Some(store.id),
            "settings should persist the selected store"
        );

        Ok(())
    }

    /// Regression: has_store only counts active stores.
    /// Archiving the only store makes has_store return false and is_first_run true.
    /// This is intentional — active-only counting prevents blocked lot creation
    /// on what the user considers an "empty" store state.
    #[tokio::test]
    async fn archived_store_no_longer_satisfies_has_store() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = fresh_test_pool().await?;

        let store = create_store(
            &pool,
            StoreCreate {
                name: "Archived Shop".into(),
                code: None,
                notes: None,
            },
        )
        .await?;

        assert!(has_store(&pool).await?, "active store satisfies has_store");

        update_store(
            &pool,
            StoreUpdate {
                id: store.id,
                name: "Archived Shop".into(),
                code: None,
                notes: None,
                is_active: false,
            },
        )
        .await?;

        // Archived store does NOT satisfy has_store (only active stores count)
        assert!(
            !has_store(&pool).await?,
            "archived store should not satisfy has_store"
        );
        // is_first_run also returns true because no active stores remain
        assert!(
            is_first_run(&pool).await?,
            "no active stores → is_first_run is true again"
        );
        Ok(())
    }

    #[tokio::test]
    async fn has_store_returns_correct_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        assert!(!has_store(&pool).await?, "no store initially");

        create_store(
            &pool,
            StoreCreate {
                name: "Shop".into(),
                code: None,
                notes: None,
            },
        )
        .await?;
        assert!(has_store(&pool).await?, "store exists after creation");
        Ok(())
    }
}
