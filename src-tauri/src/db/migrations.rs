//! Database migration definitions and runner.
//!
//! Migrations are embedded as inline SQL strings and applied sequentially.
//! The `_sqlx_migrations` tracking table (created by sqlx on first run)
//! records which migrations have been applied.
//!
//! ## Safety
//!
//! All migrations should be idempotent for the "already-applied" path.
//! Each migration runs in its own transaction and is rolled back on failure.

use sqlx::migrate::{Migration, MigrationType, Migrator};
use std::borrow::Cow;
use std::path::Path;

use super::pool::{pool_options, sqlite_options, DbPool};

/// All application migrations in order.
///
/// Each entry is `(version, description, SQL)`.
const MIGRATIONS: &[(i64, &str, &str)] = &[
    // V1 — smoke-test: create a marker table. This confirms the DB and
    // migrations subsystem are working before any real schema is added.
    (
        1,
        "create_schema_skeleton",
        r#"
        CREATE TABLE IF NOT EXISTS _caduxo_skeleton (
            id INTEGER PRIMARY KEY,
            marker TEXT NOT NULL DEFAULT 'caduxo-placeholder'
        );
        INSERT OR IGNORE INTO _caduxo_skeleton (id, marker) VALUES (1, 'caduxo-placeholder');
        "#,
    ),
    // V2 — Full application schema: stores, categories, products, barcodes,
    //       expiry lots, resolution events, notifications, settings.
    (
        2,
        "create_app_schema",
        r#"
        -- ============================================================
        -- stores
        -- ============================================================
        CREATE TABLE stores (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            code       TEXT UNIQUE,
            notes      TEXT,
            is_active  INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- ============================================================
        -- store_locations  (optional internal shelf/section per store)
        -- ============================================================
        CREATE TABLE store_locations (
            id         TEXT PRIMARY KEY,
            store_id   TEXT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
            name       TEXT NOT NULL,
            notes      TEXT,
            is_active  INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(store_id, name)
        );

        -- ============================================================
        -- categories
        -- ============================================================
        CREATE TABLE categories (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL UNIQUE,
            is_active  INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- ============================================================
        -- products
        -- ============================================================
        CREATE TABLE products (
            id                     TEXT PRIMARY KEY,
            sku                    TEXT NOT NULL UNIQUE,
            description            TEXT NOT NULL,
            category_id            TEXT REFERENCES categories(id),
            default_unit           TEXT,
            default_alert_days_before INTEGER NOT NULL DEFAULT 30,
            notes                  TEXT,
            is_active              INTEGER NOT NULL DEFAULT 1,
            created_at             TEXT NOT NULL,
            updated_at             TEXT NOT NULL
        );

        -- ============================================================
        -- product_barcodes
        -- ============================================================
        CREATE TABLE product_barcodes (
            id           TEXT PRIMARY KEY,
            product_id   TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            barcode      TEXT NOT NULL UNIQUE,
            barcode_type TEXT,
            is_primary   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL
        );

        -- ============================================================
        -- expiry_lots
        -- ============================================================
        CREATE TABLE expiry_lots (
            id                  TEXT PRIMARY KEY,
            product_id          TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            store_id            TEXT NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
            location_id         TEXT REFERENCES store_locations(id),
            quantity            REAL NOT NULL,
            unit                TEXT NOT NULL,
            expiry_date         TEXT NOT NULL,
            alert_days_before   INTEGER NOT NULL,
            batch_code          TEXT,
            status              TEXT NOT NULL DEFAULT 'active',
            resolution          TEXT,
            resolved_at         TEXT,
            notes               TEXT,
            created_at          TEXT NOT NULL,
            updated_at          TEXT NOT NULL,
            CHECK(quantity > 0),
            CHECK(alert_days_before >= 0)
        );

        -- ============================================================
        -- lot_resolution_events  (partial resolution audit trail)
        -- ============================================================
        CREATE TABLE lot_resolution_events (
            id            TEXT PRIMARY KEY,
            expiry_lot_id TEXT NOT NULL REFERENCES expiry_lots(id) ON DELETE CASCADE,
            quantity      REAL NOT NULL,
            resolution    TEXT NOT NULL,
            notes         TEXT,
            created_at    TEXT NOT NULL,
            CHECK(quantity > 0)
        );

        -- ============================================================
        -- notification_log  (prevents duplicate same-day notifications)
        -- ============================================================
        CREATE TABLE notification_log (
            id               TEXT PRIMARY KEY,
            expiry_lot_id    TEXT NOT NULL REFERENCES expiry_lots(id) ON DELETE CASCADE,
            notification_date TEXT NOT NULL,
            shown_at         TEXT NOT NULL,
            UNIQUE(expiry_lot_id, notification_date)
        );

        -- ============================================================
        -- app_settings  (key-value config, e.g. last_selected_store)
        -- ============================================================
        CREATE TABLE app_settings (
            key        TEXT PRIMARY KEY,
            value      TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- ============================================================
        -- Indexes
        -- ============================================================
        CREATE INDEX IF NOT EXISTS idx_products_sku         ON products(sku);
        CREATE INDEX IF NOT EXISTS idx_product_barcodes_barcode ON product_barcodes(barcode);
        CREATE INDEX IF NOT EXISTS idx_expiry_lots_expiry_date ON expiry_lots(expiry_date);
        CREATE INDEX IF NOT EXISTS idx_expiry_lots_store_expiry  ON expiry_lots(store_id, expiry_date);
        CREATE INDEX IF NOT EXISTS idx_expiry_lots_product      ON expiry_lots(product_id);
        CREATE INDEX IF NOT EXISTS idx_store_locations_store    ON store_locations(store_id);
        "#,
    ),
    // V3 — unit catalog + product catalog linkage.
    // - `unit_definitions` table with preset seed.
    // - `products.default_unit_id` (FK) + `unit_type` (TEXT integer|decimal).
    // - Case-insensitive back-fill for existing rows whose default_unit
    //   matches a preset key; unrecognized rows stay as text-only.
    //
    // NOTE: `default_unit` TEXT is preserved for compatibility; the
    // server-side contract echoes `display_name` from the catalog when
    // `default_unit_id` is set, and returns raw legacy text otherwise.
    (
        3,
        "add_unit_catalog",
        r#"
            -- Typed unit catalog.
            CREATE TABLE IF NOT EXISTS unit_definitions (
                id           TEXT PRIMARY KEY,
                key          TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                kind         TEXT NOT NULL CHECK(kind IN ('integer','decimal')),
                is_preset    INTEGER NOT NULL DEFAULT 0,
                archived_at  TEXT,
                created_at   TEXT NOT NULL,
                updated_at   TEXT NOT NULL
            );

            -- Partial index: active units by kind (for list queries).
            CREATE INDEX IF NOT EXISTS idx_unit_definitions_kind_active
                ON unit_definitions(kind)
                WHERE archived_at IS NULL;

            -- Product linkage: nullable FK + derived kind.
            -- Existing rows keep their default_unit text; back-fill runs below.
            ALTER TABLE products ADD COLUMN default_unit_id TEXT
                REFERENCES unit_definitions(id);
            ALTER TABLE products ADD COLUMN unit_type TEXT;

            -- Seed 14 preset units (idempotent via UNIQUE key).
            -- Keep SQL text stable: sqlx checks applied migration checksums.
            INSERT OR IGNORE INTO unit_definitions
                (id, key, display_name, kind, is_preset, created_at, updated_at)
            VALUES
                ('ud-units',   'units',  'Unidades',   'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-pcs',     'pcs',    'Piezas',     'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-cajas',   'cajas',  'Cajas',      'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-box',     'box',    'Caja',       'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-boxes',   'boxes',  'Cajas',      'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-bottles', 'bottles','Botellas',   'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-bags',    'bags',   'Bolsas',     'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-packs',   'packs',  'Paquetes',   'integer', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-kg',      'kg',     'Kilogramo',  'decimal', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-g',       'g',      'Gramo',      'decimal', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-mg',      'mg',     'Miligramo',  'decimal', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-tn',      'tn',     'Tonelada',   'decimal', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-l',       'L',      'Litro',      'decimal', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
                ('ud-ml',      'mL',     'Mililitro',  'decimal', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

            -- Case-insensitive back-fill for existing products whose default_unit
            -- matches a preset key. Rows that don't match remain text-only and will
            -- surface in the audit banner.
            UPDATE products
            SET
                default_unit_id = (
                    SELECT ud.id FROM unit_definitions ud
                    WHERE lower(ud.key) = lower(products.default_unit)
                    LIMIT 1
                ),
                unit_type = (
                    SELECT ud.kind FROM unit_definitions ud
                    WHERE lower(ud.key) = lower(products.default_unit)
                    LIMIT 1
                )
            WHERE default_unit IS NOT NULL
              AND default_unit_id IS NULL;
            "#,
    ),
];

/// Returns a `Migrator` built from the inline `MIGRATIONS` constant.
///
/// This avoids compile-time embedding path issues while keeping migrations
/// version-controlled alongside the source code.
///
fn build_migrator() -> Migrator {
    let migrations: Vec<Migration> = MIGRATIONS
        .iter()
        .map(|(version, description, sql)| {
            Migration::new(
                *version,
                Cow::Owned(description.to_string()),
                MigrationType::Simple,
                Cow::Owned(sql.to_string()),
                false, // no_tx — allow transactions
            )
        })
        .collect();

    // SAFETY: Migrator fields are semver-exempt and designed for this use.
    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
}

/// Opens a new SQLite pool connecting to `db_path`.
pub async fn open_pool(db_path: &Path) -> Result<DbPool, sqlx::Error> {
    let opts = sqlite_options(db_path);
    let pool_opts = pool_options();
    pool_opts.connect_with(opts).await
}

/// Runs all pending migrations on `pool`.
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::Error> {
    let migrator = build_migrator();
    let count = migrator.iter().count() as u32;
    migrator.run(pool).await?;
    tracing::info!(total_migrations = count, "Database migrations applied");
    Ok(())
}

/// Returns the count of applied migrations in the tracking table.
pub async fn applied_count(pool: &DbPool) -> Result<u32, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    Ok(row.0 as u32)
}

/// Creates a fresh SQLite pool backed by a temporary file and runs all
/// migrations against it.  Using a real file avoids in-memory SQLite
/// multi-connection schema-loss issues (SQLite :memory: databases are
/// per-connection; with pool max_connections > 1 the schema can be
/// invisible to other connections).
#[cfg(test)]
pub async fn fresh_test_pool() -> Result<DbPool, sqlx::Error> {
    // Use a path unique to this test invocation so concurrent tests don't
    // collide even within the same process.  Uses PID + a random suffix.
    let pid = std::process::id();
    let rand: u16 = rand::random();
    let db_path = std::path::PathBuf::from(format!("/tmp/caduxo_test_{pid}_{rand}.db"));
    // Remove any stale file from a previous run that wasn't cleaned up.
    let _ = std::fs::remove_file(&db_path);
    let pool = open_pool(&db_path).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {

    use super::*;

    // -------------------------------------------------------------------
    // Helper: count rows in a table.
    // -------------------------------------------------------------------
    async fn count_table(pool: &DbPool, table: &str) -> u32 {
        let query = format!("SELECT COUNT(*) FROM {table}");
        let row: Result<(i64,), _> = sqlx::query_as(&query).fetch_one(pool).await;
        row.map(|r| r.0 as u32).unwrap_or(0)
    }

    // -------------------------------------------------------------------
    // Helper: verify a table exists by querying sqlite_master.
    // -------------------------------------------------------------------
    async fn table_exists(pool: &DbPool, table: &str) -> bool {
        let query = "SELECT 1 FROM sqlite_master WHERE type='table' AND name=? LIMIT 1";
        sqlx::query(query)
            .bind(table)
            .fetch_optional(pool)
            .await
            .map(|r| r.is_some())
            .unwrap_or(false)
    }

    // -------------------------------------------------------------------
    // Helper: verify a named index exists.
    // -------------------------------------------------------------------
    async fn index_exists(pool: &DbPool, index_name: &str) -> bool {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='index' AND name=?")
                .bind(index_name)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
        row.is_some()
    }

    // -------------------------------------------------------------------
    // Test: all migrations apply cleanly on a fresh in-memory database.
    // V3 adds the unit_definitions table + product catalog linkage.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn v2_schema_applies_on_fresh_db() {
        let pool = fresh_test_pool().await.unwrap();
        assert_eq!(applied_count(&pool).await.unwrap(), 3);
    }

    // -------------------------------------------------------------------
    // Test: all required tables are created.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn all_required_tables_exist() {
        let pool = fresh_test_pool().await.unwrap();
        for table in [
            "stores",
            "store_locations",
            "categories",
            "products",
            "product_barcodes",
            "expiry_lots",
            "lot_resolution_events",
            "notification_log",
            "app_settings",
        ] {
            assert!(
                table_exists(&pool, table).await,
                "table '{table}' should exist"
            );
        }
    }

    // -------------------------------------------------------------------
    // Test: stores table has required columns and constraints.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn stores_table_structure() {
        let pool = fresh_test_pool().await.unwrap();

        // Insert a store
        let now = chrono::Utc::now().to_rfc3339();
        let r = sqlx::query(
            "INSERT INTO stores (id, name, code, is_active, created_at, updated_at)
                 VALUES ('s1', 'Main Store', 'MS001', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_ok());

        // Duplicate code is rejected
        let r = sqlx::query(
            "INSERT INTO stores (id, name, code, is_active, created_at, updated_at)
                 VALUES ('s2', 'Other', 'MS001', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_err(), "store code must be unique");
    }

    // -------------------------------------------------------------------
    // Test: store_locations enforces (store_id, name) uniqueness.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn store_locations_unique_per_store() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        // Seed store
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s1', 'Store', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        // Insert two different locations — OK
        for (lid, name) in [("l1", "Fridge A"), ("l2", "Shelf B")] {
            sqlx::query(
                    "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
                     VALUES (?, 's1', ?, 1, ?, ?)",
                )
                .bind(lid)
                .bind(name)
                .bind(&now)
                .bind(&now)
                .execute(&pool)
                .await
                .unwrap();
        }

        // Same name under same store — rejected
        let r = sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
                 VALUES ('l3', 's1', 'Fridge A', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_err(), "location name must be unique per store");

        // Same name under different store — OK
        let r = sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s2', 'Store 2', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_ok());

        let r = sqlx::query(
            "INSERT INTO store_locations (id, store_id, name, is_active, created_at, updated_at)
                 VALUES ('l4', 's2', 'Fridge A', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(
            r.is_ok(),
            "same location name is allowed in different stores"
        );
    }

    // -------------------------------------------------------------------
    // Test: categories enforces unique name.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn categories_unique_name() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at)
                 VALUES ('c1', 'Dairy', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        let r = sqlx::query(
            "INSERT INTO categories (id, name, is_active, created_at, updated_at)
                 VALUES ('c2', 'Dairy', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_err(), "category name must be unique");
    }

    // -------------------------------------------------------------------
    // Test: products enforces unique SKU.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn products_unique_sku() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Whole Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        let r = sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p2', 'SKU-001', 'Skim Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_err(), "product SKU must be unique");
    }

    // -------------------------------------------------------------------
    // Test: product_barcodes enforces unique barcode.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn barcodes_unique_across_products() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        // Seed two products
        for (pid, sku) in [("p1", "SKU-A"), ("p2", "SKU-B")] {
            sqlx::query(
                    "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                     VALUES (?, ?, 'Product', 30, 1, ?, ?)",
                )
                .bind(pid)
                .bind(sku)
                .bind(&now)
                .bind(&now)
                .execute(&pool)
                .await
                .unwrap();
        }

        // Add barcode to first product
        sqlx::query(
            "INSERT INTO product_barcodes (id, product_id, barcode, is_primary, created_at)
                 VALUES ('b1', 'p1', '1234567890', 1, ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        // Same barcode on second product — rejected
        let r = sqlx::query(
            "INSERT INTO product_barcodes (id, product_id, barcode, is_primary, created_at)
                 VALUES ('b2', 'p2', '1234567890', 0, ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_err(), "barcode must be unique across all products");
    }

    // -------------------------------------------------------------------
    // Test: expiry_lots rejects non-positive quantity.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn expiry_lots_quantity_check() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        // Seed store + product
        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s1', 'Store', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        // Valid lot
        let r = sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot1', 'p1', 's1', 5.0, 'L', '2025-12-31', 30, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_ok());

        // Zero quantity — rejected
        let r = sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot2', 'p1', 's1', 0.0, 'L', '2025-12-31', 30, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_err(), "quantity must be > 0");

        // Negative quantity — rejected
        let r = sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot3', 'p1', 's1', -1.0, 'L', '2025-12-31', 30, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_err(), "quantity must be > 0");
    }

    // -------------------------------------------------------------------
    // Test: expiry_lots rejects negative alert_days_before.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn expiry_lots_alert_days_non_negative() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s1', 'Store', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        let r = sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot1', 'p1', 's1', 5.0, 'L', '2025-12-31', -5, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_err(), "alert_days_before must be >= 0");
    }

    // -------------------------------------------------------------------
    // Test: lot_resolution_events enforces positive quantity.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn resolution_events_quantity_check() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s1', 'Store', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot1', 'p1', 's1', 10.0, 'L', '2025-12-31', 30, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        // Valid resolution event
        let r = sqlx::query(
                "INSERT INTO lot_resolution_events (id, expiry_lot_id, quantity, resolution, created_at)
                 VALUES ('ev1', 'lot1', 3.0, 'consumed', ?)",
            )
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_ok());

        // Zero or negative — rejected
        let r = sqlx::query(
                "INSERT INTO lot_resolution_events (id, expiry_lot_id, quantity, resolution, created_at)
                 VALUES ('ev2', 'lot1', 0.0, 'consumed', ?)",
            )
            .bind(&now)
            .execute(&pool)
            .await;
        assert!(r.is_err(), "resolution event quantity must be > 0");
    }

    // -------------------------------------------------------------------
    // Test: notification_log enforces (lot_id, date) uniqueness.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn notification_log_unique_per_day() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s1', 'Store', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot1', 'p1', 's1', 10.0, 'L', '2025-12-31', 30, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO notification_log (id, expiry_lot_id, notification_date, shown_at)
                 VALUES ('n1', 'lot1', '2025-06-01', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        // Same lot + same date — rejected
        let r = sqlx::query(
            "INSERT INTO notification_log (id, expiry_lot_id, notification_date, shown_at)
                 VALUES ('n2', 'lot1', '2025-06-01', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_err(), "notification_log: same lot+date must be unique");

        // Same lot + different date — OK
        let r = sqlx::query(
            "INSERT INTO notification_log (id, expiry_lot_id, notification_date, shown_at)
                 VALUES ('n3', 'lot1', '2025-06-02', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(
            r.is_ok(),
            "notification_log allows same lot on different days"
        );
    }

    // -------------------------------------------------------------------
    // Test: app_settings key is the primary key (upsert semantics).
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn app_settings_primary_key() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('theme', 'dark', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        // Duplicate key — rejected
        let r = sqlx::query(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('theme', 'light', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await;
        assert!(r.is_err(), "app_settings key must be unique");
    }

    // -------------------------------------------------------------------
    // Test: V3 migration applies on fresh db (RED — must fail before V3 entry).
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn v3_schema_applies_on_fresh_db() {
        let pool = fresh_test_pool().await.unwrap();
        assert_eq!(
            applied_count(&pool).await.unwrap(),
            3,
            "V3 should bring applied count to 3"
        );
    }

    // -------------------------------------------------------------------
    // Test: V3 migration is idempotent (RED — must fail before V3 entry).
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn v3_migration_is_idempotent() {
        let pool = fresh_test_pool().await.unwrap();
        let first_count = applied_count(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
        let second_count = applied_count(&pool).await.unwrap();
        assert_eq!(
            first_count, second_count,
            "re-running V3 migrations should add no new rows"
        );
    }

    // -------------------------------------------------------------------
    // Test: all required indexes exist after migration.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn all_required_indexes_exist() {
        let pool = fresh_test_pool().await.unwrap();

        for idx in [
            "idx_products_sku",
            "idx_product_barcodes_barcode",
            "idx_expiry_lots_expiry_date",
            "idx_expiry_lots_store_expiry",
            "idx_expiry_lots_product",
            "idx_store_locations_store",
        ] {
            assert!(index_exists(&pool, idx).await, "index '{idx}' should exist");
        }
    }

    // -------------------------------------------------------------------
    // Test: idempotency — running migrations twice is safe.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn migrations_are_idempotent() {
        let pool = fresh_test_pool().await.unwrap();
        let first_count = applied_count(&pool).await.unwrap();

        // Re-run migrations — should be a no-op (no new migrations).
        run_migrations(&pool).await.unwrap();
        let second_count = applied_count(&pool).await.unwrap();

        assert_eq!(
            first_count, second_count,
            "re-running migrations should add no new rows"
        );
    }

    // -------------------------------------------------------------------
    // Test: foreign-key cascade — deleting a product removes barcodes.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn product_barcodes_cascade_on_delete() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO product_barcodes (id, product_id, barcode, is_primary, created_at)
                 VALUES ('b1', 'p1', '1234567890', 1, ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(count_table(&pool, "product_barcodes").await, 1);

        sqlx::query("DELETE FROM products WHERE id = 'p1'")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            count_table(&pool, "product_barcodes").await,
            0,
            "barcodes should cascade-delete when product is deleted"
        );
    }

    // -------------------------------------------------------------------
    // V3 — seed + back-fill tests.
    // -------------------------------------------------------------------

    /// Returns the (id, key, display_name, kind) of all unit_definitions rows.
    async fn all_unit_definitions(pool: &DbPool) -> Vec<(String, String, String, String)> {
        let rows: Vec<(String, String, String, String)> =
            sqlx::query_as("SELECT id, key, display_name, kind FROM unit_definitions ORDER BY key")
                .fetch_all(pool)
                .await
                .unwrap();
        rows
    }

    #[tokio::test]
    async fn unit_definitions_seeded_with_presets() {
        let pool = fresh_test_pool().await.unwrap();
        let units = all_unit_definitions(&pool).await;

        let expected: std::collections::HashSet<(String, String, String, String)> = vec![
            (
                "ud-units".to_string(),
                "units".to_string(),
                "Unidades".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-pcs".to_string(),
                "pcs".to_string(),
                "Piezas".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-cajas".to_string(),
                "cajas".to_string(),
                "Cajas".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-box".to_string(),
                "box".to_string(),
                "Caja".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-boxes".to_string(),
                "boxes".to_string(),
                "Cajas".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-bottles".to_string(),
                "bottles".to_string(),
                "Botellas".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-bags".to_string(),
                "bags".to_string(),
                "Bolsas".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-packs".to_string(),
                "packs".to_string(),
                "Paquetes".to_string(),
                "integer".to_string(),
            ),
            (
                "ud-kg".to_string(),
                "kg".to_string(),
                "Kilogramo".to_string(),
                "decimal".to_string(),
            ),
            (
                "ud-g".to_string(),
                "g".to_string(),
                "Gramo".to_string(),
                "decimal".to_string(),
            ),
            (
                "ud-mg".to_string(),
                "mg".to_string(),
                "Miligramo".to_string(),
                "decimal".to_string(),
            ),
            (
                "ud-tn".to_string(),
                "tn".to_string(),
                "Tonelada".to_string(),
                "decimal".to_string(),
            ),
            (
                "ud-l".to_string(),
                "L".to_string(),
                "Litro".to_string(),
                "decimal".to_string(),
            ),
            (
                "ud-ml".to_string(),
                "mL".to_string(),
                "Mililitro".to_string(),
                "decimal".to_string(),
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(units.len(), 14, "14 preset units should be seeded");
        for (id, key, display_name, kind) in &units {
            let triple = (id.clone(), key.clone(), display_name.clone(), kind.clone());
            assert!(expected.contains(&triple), "unexpected unit: {:?}", triple);
        }
    }

    #[tokio::test]
    async fn products_default_unit_id_backfilled_for_known_keys() {
        // Use the same pattern as migrations_are_idempotent: create a V2-only
        // pool first, seed products, then run the full migrator and verify.
        use sqlx::migrate::{Migration, MigrationType, Migrator};
        use std::borrow::Cow;

        let now_ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // Build a V2-only migrator.
        let v2_migrations: Vec<Migration> = MIGRATIONS
            .iter()
            .filter(|(v, _, _)| *v <= 2)
            .map(|(version, description, sql)| {
                Migration::new(
                    *version,
                    Cow::Owned(description.to_string()),
                    MigrationType::Simple,
                    Cow::Owned(sql.to_string()),
                    false,
                )
            })
            .collect();
        let v2_migrator = Migrator {
            migrations: Cow::Owned(v2_migrations),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };

        // Open a fresh DB and apply V2 only.
        let pid = std::process::id();
        let rand: u16 = rand::random();
        let db_path = std::path::PathBuf::from(format!("/tmp/caduxo_test_v2_{pid}_{rand}.db"));
        let _ = std::fs::remove_file(&db_path);

        let pool = super::open_pool(&db_path).await.unwrap();
        v2_migrator.run(&pool).await.unwrap();

        // Seed V2 products before V3.
        sqlx::query(
                "INSERT INTO products (id, sku, description, default_unit, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p-kg', 'SKU-KG', 'Test product', 'kg', 30, 1, $1, $2)",
            )
            .bind(&now_ts)
            .bind(&now_ts)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_unit, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p-KG', 'SKU-KG2', 'Test product 2', 'KG', 30, 1, $1, $2)",
            )
            .bind(&now_ts)
            .bind(&now_ts)
            .execute(&pool)
            .await
            .unwrap();

        // Now run the full migrator (including V3).
        run_migrations(&pool).await.unwrap();

        // Verify back-fill for lowercase match.
        let row: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT default_unit_id, unit_type FROM products WHERE id = 'p-kg'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            row.0,
            Some("ud-kg".to_string()),
            "default_unit_id should be ud-kg"
        );
        assert_eq!(
            row.1,
            Some("decimal".to_string()),
            "unit_type should be decimal"
        );

        // Verify back-fill for uppercase match.
        let row2: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT default_unit_id, unit_type FROM products WHERE id = 'p-KG'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            row2.0,
            Some("ud-kg".to_string()),
            "default_unit_id should be ud-kg for 'KG'"
        );
        assert_eq!(
            row2.1,
            Some("decimal".to_string()),
            "unit_type should be decimal for 'KG'"
        );

        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn products_with_unknown_default_unit_remain_unlinked() {
        use sqlx::migrate::{Migration, MigrationType, Migrator};
        use std::borrow::Cow;

        let now_ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // Build a V2-only migrator (same pattern as products_default_unit_id_backfilled).
        let v2_migrations: Vec<Migration> = MIGRATIONS
            .iter()
            .filter(|(v, _, _)| *v <= 2)
            .map(|(version, description, sql)| {
                Migration::new(
                    *version,
                    Cow::Owned(description.to_string()),
                    MigrationType::Simple,
                    Cow::Owned(sql.to_string()),
                    false,
                )
            })
            .collect();
        let v2_migrator = Migrator {
            migrations: Cow::Owned(v2_migrations),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };

        let pid = std::process::id();
        let rand: u16 = rand::random();
        let db_path = std::path::PathBuf::from(format!("/tmp/caduxo_test_v2_unk_{pid}_{rand}.db"));
        let _ = std::fs::remove_file(&db_path);

        let pool = super::open_pool(&db_path).await.unwrap();
        v2_migrator.run(&pool).await.unwrap();

        // Seed V2 product with unrecognized key.
        sqlx::query(
                "INSERT INTO products (id, sku, description, default_unit, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p-foo', 'SKU-FOO', 'Test foo', 'foo', 30, 1, $1, $2)",
            )
            .bind(&now_ts)
            .bind(&now_ts)
            .execute(&pool)
            .await
            .unwrap();

        // Also seed with NULL default_unit.
        sqlx::query(
                "INSERT INTO products (id, sku, description, default_unit, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p-null', 'SKU-NULL', 'Test null', NULL, 30, 1, $1, $2)",
            )
            .bind(&now_ts)
            .bind(&now_ts)
            .execute(&pool)
            .await
            .unwrap();

        // Run full migrator (V3 applies back-fill).
        run_migrations(&pool).await.unwrap();

        // Verify unrecognized key: default_unit_id and unit_type are NULL.
        let row: (Option<String>, Option<String>, String) = sqlx::query_as(
            "SELECT default_unit_id, unit_type, default_unit FROM products WHERE id = 'p-foo'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            row.0.is_none(),
            "default_unit_id should be NULL for unrecognized key"
        );
        assert!(
            row.1.is_none(),
            "unit_type should be NULL for unrecognized key"
        );
        assert_eq!(row.2, "foo", "default_unit text should be preserved");

        // Verify NULL default_unit: stays NULL.
        let row2: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT default_unit_id, unit_type FROM products WHERE id = 'p-null'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            row2.0.is_none(),
            "default_unit_id should be NULL for NULL default_unit"
        );
        assert!(
            row2.1.is_none(),
            "unit_type should be NULL for NULL default_unit"
        );

        let _ = std::fs::remove_file(&db_path);
    }

    // -------------------------------------------------------------------
    // Test: notification_log cascade — deleting a lot removes log rows.
    // -------------------------------------------------------------------
    #[tokio::test]
    async fn notification_log_cascade_on_lot_delete() {
        let pool = fresh_test_pool().await.unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO stores (id, name, is_active, created_at, updated_at)
                 VALUES ('s1', 'Store', 1, ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, is_active, created_at, updated_at)
                 VALUES ('p1', 'SKU-001', 'Milk', 30, 1, ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, quantity, unit, expiry_date, alert_days_before, status, created_at, updated_at)
                 VALUES ('lot1', 'p1', 's1', 10.0, 'L', '2025-12-31', 30, 'active', ?, ?)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO notification_log (id, expiry_lot_id, notification_date, shown_at)
                 VALUES ('n1', 'lot1', '2025-06-01', ?)",
        )
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(count_table(&pool, "notification_log").await, 1);

        sqlx::query("DELETE FROM expiry_lots WHERE id = 'lot1'")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            count_table(&pool, "notification_log").await,
            0,
            "notification_log rows should cascade-delete when lot is deleted"
        );
    }
}
