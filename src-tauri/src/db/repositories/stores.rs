//! Repository for store and store_location persistence.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::dto::stores::{
    StoreCreate, StoreLocationCreate, StoreLocationResponse, StoreLocationUpdate, StoreResponse,
    StoreUpdate,
};

/// Inserts a new store and returns the populated response.
pub async fn insert_store(
    pool: &SqlitePool,
    input: &StoreCreate,
) -> Result<StoreResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO stores (id, name, code, notes, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 1, $5, $6)
        "#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.code)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(StoreResponse {
        id,
        name: input.name.clone(),
        code: input.code.clone(),
        notes: input.notes.clone(),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Fetches all active stores ordered by name.
pub async fn list_active_stores(pool: &SqlitePool) -> Result<Vec<StoreResponse>, sqlx::Error> {
    sqlx::query_as::<_, StoreResponse>(
        r#"
        SELECT id, name, code, notes, is_active, created_at, updated_at
        FROM stores
        WHERE is_active = 1
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Fetches all stores (active and inactive) ordered by name.
pub async fn list_all_stores(pool: &SqlitePool) -> Result<Vec<StoreResponse>, sqlx::Error> {
    sqlx::query_as::<_, StoreResponse>(
        r#"
        SELECT id, name, code, notes, is_active, created_at, updated_at
        FROM stores
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Fetches a single store by id.
pub async fn get_store(pool: &SqlitePool, id: &str) -> Result<Option<StoreResponse>, sqlx::Error> {
    sqlx::query_as::<_, StoreResponse>(
        r#"
        SELECT id, name, code, notes, is_active, created_at, updated_at
        FROM stores
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Updates an existing store and returns the updated row.
pub async fn update_store(
    pool: &SqlitePool,
    input: &StoreUpdate,
) -> Result<Option<StoreResponse>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let affected = sqlx::query(
        r#"
        UPDATE stores
        SET name = $1, code = $2, notes = $3, is_active = $4, updated_at = $5
        WHERE id = $6
        "#,
    )
    .bind(&input.name)
    .bind(&input.code)
    .bind(&input.notes)
    .bind(i32::from(input.is_active))
    .bind(&now)
    .bind(&input.id)
    .execute(pool)
    .await?;

    if affected.rows_affected() == 0 {
        return Ok(None);
    }

    get_store(pool, &input.id).await
}

/// Inserts a new internal location under a store.
pub async fn insert_location(
    pool: &SqlitePool,
    input: &StoreLocationCreate,
) -> Result<StoreLocationResponse, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO store_locations (id, store_id, name, notes, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 1, $5, $6)
        "#,
    )
    .bind(&id)
    .bind(&input.store_id)
    .bind(&input.name)
    .bind(&input.notes)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(StoreLocationResponse {
        id,
        store_id: input.store_id.clone(),
        name: input.name.clone(),
        notes: input.notes.clone(),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Lists all active locations for a given store.
pub async fn list_active_locations(
    pool: &SqlitePool,
    store_id: &str,
) -> Result<Vec<StoreLocationResponse>, sqlx::Error> {
    sqlx::query_as::<_, StoreLocationResponse>(
        r#"
        SELECT id, store_id, name, notes, is_active, created_at, updated_at
        FROM store_locations
        WHERE store_id = $1 AND is_active = 1
        ORDER BY name ASC
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await
}

/// Lists all locations for a given store (active and inactive).
pub async fn list_all_locations(
    pool: &SqlitePool,
    store_id: &str,
) -> Result<Vec<StoreLocationResponse>, sqlx::Error> {
    sqlx::query_as::<_, StoreLocationResponse>(
        r#"
        SELECT id, store_id, name, notes, is_active, created_at, updated_at
        FROM store_locations
        WHERE store_id = $1
        ORDER BY name ASC
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await
}

/// Updates an existing location.
pub async fn update_location(
    pool: &SqlitePool,
    input: &StoreLocationUpdate,
) -> Result<Option<StoreLocationResponse>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    let affected = sqlx::query(
        r#"
        UPDATE store_locations
        SET name = $1, notes = $2, is_active = $3, updated_at = $4
        WHERE id = $5 AND store_id = $6
        "#,
    )
    .bind(&input.name)
    .bind(&input.notes)
    .bind(i32::from(input.is_active))
    .bind(&now)
    .bind(&input.id)
    .bind(&input.store_id)
    .execute(pool)
    .await?;

    if affected.rows_affected() == 0 {
        return Ok(None);
    }

    sqlx::query_as::<_, StoreLocationResponse>(
        r#"
        SELECT id, store_id, name, notes, is_active, created_at, updated_at
        FROM store_locations
        WHERE id = $1
        "#,
    )
    .bind(&input.id)
    .fetch_optional(pool)
    .await
}

/// Returns true if at least one active store exists.
pub async fn has_active_store(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stores WHERE is_active = 1")
        .fetch_one(pool)
        .await?;
    Ok(row.0 > 0)
}

/// Returns the count of active stores.
pub async fn active_store_count(pool: &SqlitePool) -> Result<u32, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stores WHERE is_active = 1")
        .fetch_one(pool)
        .await?;
    Ok(row.0 as u32)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::db::migrations::fresh_test_pool;
    use crate::dto::stores::{StoreCreate, StoreLocationCreate, StoreLocationUpdate, StoreUpdate};

    // NOTE: no `use super::*` — avoids shadowing `fresh_test_pool`.
    // Call repository functions via `super::` explicitly.

    async fn seed_store(pool: &SqlitePool) -> Result<String, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Test Store', 1, $2, $3)",
        )
        .bind(&id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    async fn seed_other_store(pool: &SqlitePool) -> Result<String, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
             VALUES ($1, 'Other Store', 1, $2, $3)",
        )
        .bind(&id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(id)
    }

    #[tokio::test]
    async fn insert_and_list_stores() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let input = StoreCreate {
            name: "My Store".into(),
            code: Some("MS001".into()),
            notes: None,
        };
        let store = super::insert_store(&pool, &input).await?;
        assert_eq!(store.name, "My Store");
        assert_eq!(store.code.as_deref(), Some("MS001"));

        let all = super::list_active_stores(&pool).await?;
        assert_eq!(all.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn update_store() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let id = seed_store(&pool).await?;
        let updated = super::update_store(
            &pool,
            &StoreUpdate {
                id: id.clone(),
                name: "Renamed".into(),
                code: Some("MS002".into()),
                notes: Some("Updated".into()),
                is_active: true,
            },
        )
        .await?;
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().name, "Renamed");
        Ok(())
    }

    #[tokio::test]
    async fn insert_and_list_locations() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool).await?;

        let loc = super::insert_location(
            &pool,
            &StoreLocationCreate {
                store_id: store_id.clone(),
                name: "Fridge A".into(),
                notes: None,
            },
        )
        .await?;
        assert_eq!(loc.name, "Fridge A");

        let all = super::list_active_locations(&pool, &store_id).await?;
        assert_eq!(all.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn update_location() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool).await?;
        let loc = super::insert_location(
            &pool,
            &StoreLocationCreate {
                store_id: store_id.clone(),
                name: "Shelf 1".into(),
                notes: None,
            },
        )
        .await?;

        let updated = super::update_location(
            &pool,
            &StoreLocationUpdate {
                id: loc.id.clone(),
                store_id: store_id.clone(),
                name: "Shelf One".into(),
                notes: None,
                is_active: true,
            },
        )
        .await?;
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().name, "Shelf One");
        Ok(())
    }

    #[tokio::test]
    async fn has_active_store() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        assert!(!super::has_active_store(&pool).await?);
        seed_store(&pool).await?;
        assert!(super::has_active_store(&pool).await?);
        Ok(())
    }

    #[tokio::test]
    async fn active_store_count() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        assert_eq!(super::active_store_count(&pool).await?, 0);

        let s1 = seed_store(&pool).await?;
        assert_eq!(super::active_store_count(&pool).await?, 1);

        super::update_store(
            &pool,
            &StoreUpdate {
                id: s1,
                name: "Test".into(),
                code: None,
                notes: None,
                is_active: false,
            },
        )
        .await?;
        assert_eq!(super::active_store_count(&pool).await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn location_name_unique_per_store_enforced() -> Result<(), Box<dyn std::error::Error>> {
        let pool = fresh_test_pool().await?;
        let store_id = seed_store(&pool).await?;
        let other_store_id = seed_other_store(&pool).await?;

        super::insert_location(
            &pool,
            &StoreLocationCreate {
                store_id: store_id.clone(),
                name: "Fridge A".into(),
                notes: None,
            },
        )
        .await?;

        // Same name in same store — must fail (schema UNIQUE constraint)
        let r = super::insert_location(
            &pool,
            &StoreLocationCreate {
                store_id: store_id.clone(),
                name: "Fridge A".into(),
                notes: None,
            },
        )
        .await;
        assert!(r.is_err(), "location name must be unique per store");

        // Same name in different store — OK
        let r = super::insert_location(
            &pool,
            &StoreLocationCreate {
                store_id: other_store_id,
                name: "Fridge A".into(),
                notes: None,
            },
        )
        .await;
        assert!(
            r.is_ok(),
            "same location name is allowed in different stores"
        );
        Ok(())
    }
}
