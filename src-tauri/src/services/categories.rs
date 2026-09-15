//! Category search service — prefix + substring search with pagination.

use crate::db::DbPool;
use crate::dto::products::{CategoryResponse, CategorySearchInput, CategorySearchPage};
use crate::error::AppError;

/// Explicit SELECT for the category row shape used by `CategoryResponse`.
/// sqlx 0.8 dropped the implicit `as_select()` macro; callers pass the
/// query string explicitly and bind positional parameters.
const CATEGORY_SELECT: &str = "SELECT id, name, is_active, created_at, updated_at \
    FROM categories WHERE is_active = 1 \
    AND lower(name) LIKE $1 \
    ORDER BY name ASC \
    LIMIT $2";

/// Searches active categories with case-insensitive prefix match first
/// and substring fallback when the prefix match returns zero rows.
/// Returns a page with up to `limit` items and a `has_more` flag.
pub async fn search(
    pool: &DbPool,
    input: CategorySearchInput,
) -> Result<CategorySearchPage, AppError> {
    let raw = input.query.trim();
    let limit = input.limit.unwrap_or(50).clamp(1, 500);

    // Empty query → list all active categories, ordered by name asc, capped.
    if raw.is_empty() {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories WHERE is_active = 1")
            .fetch_one(pool)
            .await
            .map_err(AppError::from)?;

        let items: Vec<CategoryResponse> = sqlx::query_as(CATEGORY_SELECT)
            .bind("%")
            .bind(limit as i64)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)?;

        return Ok(CategorySearchPage {
            has_more: (total.0 as usize) > items.len(),
            total: total.0 as usize,
            items,
        });
    }

    // Non-empty query: try prefix first.
    let needle = raw.to_lowercase();
    let prefix = format!("{needle}%");

    let prefix_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM categories WHERE is_active = 1 AND lower(name) LIKE $1",
    )
    .bind(&prefix)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    if prefix_count.0 > 0 {
        let items: Vec<CategoryResponse> = sqlx::query_as(CATEGORY_SELECT)
            .bind(&prefix)
            .bind(limit as i64)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)?;

        return Ok(CategorySearchPage {
            has_more: (prefix_count.0 as usize) > items.len(),
            total: prefix_count.0 as usize,
            items,
        });
    }

    // Fallback: substring match.
    let substring = format!("%{needle}%");
    let substring_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM categories WHERE is_active = 1 AND lower(name) LIKE $1",
    )
    .bind(&substring)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    let items: Vec<CategoryResponse> = sqlx::query_as(CATEGORY_SELECT)
        .bind(&substring)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)?;

    Ok(CategorySearchPage {
        has_more: (substring_count.0 as usize) > items.len(),
        total: substring_count.0 as usize,
        items,
    })
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::fresh_test_pool;
    use crate::dto::products::CategorySearchInput;
    use crate::services::categories::search;

    #[tokio::test]
    async fn search_empty_query_returns_active_categories() {
        let pool = fresh_test_pool().await.unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Dairy', 1, $2, $3)",
        )
        .bind("cat-dairy")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Bakery', 1, $2, $3)",
        )
        .bind("cat-bakery")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        // Archived category should not appear.
        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Legacy', 0, $2, $3)",
        )
        .bind("cat-legacy")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let result = search(
            &pool,
            CategorySearchInput {
                query: "".into(),
                limit: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 2);
        assert!(!result.has_more);
    }

    #[tokio::test]
    async fn search_prefix_match_for_partial_query() {
        let pool = fresh_test_pool().await.unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Dairy', 1, $2, $3)",
        )
        .bind("cat-1")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Dairy-Free', 1, $2, $3)",
        )
        .bind("cat-2")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Bakery', 1, $2, $3)",
        )
        .bind("cat-3")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let result = search(
            &pool,
            CategorySearchInput {
                query: "Dai".into(),
                limit: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 2);
        assert!(result.items.iter().all(|c| c.name.starts_with("Dai")));
    }

    #[tokio::test]
    async fn search_falls_back_to_substring_when_no_prefix() {
        let pool = fresh_test_pool().await.unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Dairy', 1, $2, $3)",
        )
        .bind("cat-1")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Bakery', 1, $2, $3)",
        )
        .bind("cat-2")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let result = search(
            &pool,
            CategorySearchInput {
                query: "airy".into(),
                limit: None,
            },
        )
        .await
        .unwrap();

        // "airy" is not a prefix of "Dairy", but it is a substring.
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "Dairy");
    }

    #[tokio::test]
    async fn search_excludes_archived_categories() {
        let pool = fresh_test_pool().await.unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Active', 1, $2, $3)",
        )
        .bind("cat-active")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, 'Archived', 0, $2, $3)",
        )
        .bind("cat-archived")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

        let result = search(
            &pool,
            CategorySearchInput {
                query: "".into(),
                limit: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "Active");
    }

    #[tokio::test]
    async fn search_respects_limit() {
        let pool = fresh_test_pool().await.unwrap();

        for i in 0..10 {
            sqlx::query(
                "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, $2, 1, $3, $4)",
            )
            .bind(format!("cat-{i}"))
            .bind(format!("Category {i}"))
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        }

        let result = search(
            &pool,
            CategorySearchInput {
                query: "Category".into(),
                limit: Some(3),
            },
        )
        .await
        .unwrap();

        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 10);
        assert!(result.has_more);
    }

    #[tokio::test]
    async fn search_has_more_true_when_total_exceeds_limit() {
        let pool = fresh_test_pool().await.unwrap();

        for i in 0..7 {
            sqlx::query(
                "INSERT INTO categories (id, name, is_active, created_at, updated_at) VALUES ($1, $2, 1, $3, $4)",
            )
            .bind(format!("cat-x{i}"))
            .bind(format!("X-{i}"))
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();
        }

        let result = search(
            &pool,
            CategorySearchInput {
                query: "X".into(),
                limit: Some(5),
            },
        )
        .await
        .unwrap();

        assert_eq!(result.items.len(), 5);
        assert_eq!(result.total, 7);
        assert!(result.has_more);
    }
}
