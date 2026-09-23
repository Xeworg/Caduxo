# Design — product-lifecycle-reusable-identifiers

## 1. Overview

This change adds a third, terminal product lifecycle state — `retired` — alongside the existing reversible `archived` state, so operators can release SKUs and barcodes for reuse while preserving every history row (`expiry_lots`, `lot_movements`, `lot_resolution_events`, `notification_log`, `product_categories`, `product_barcodes`) for the retired product. The design pivots `products.sku` and `product_barcodes.barcode` from whole-table UNIQUE constraints to partial UNIQUE indexes scoped to `lifecycle != 'retired'`, persists a new `product_lifecycle_events` audit table, and wires the lifecycle through the backend services, the scanner resolver, the dashboard scan/search surface, the CSV import preview, and the catalog / detail / dashboard UI surfaces. Every mutation flows through one Rust service boundary that writes both the `products.lifecycle` change and the audit row in a single transaction.

The implementation is a chained PR slice because the change touches schema, services, multiple UI surfaces, and i18n parity. The canonical review budget for this session is **400 changed lines**. Section 10 sizes every slice and proposes the chain; section 12 records the chain-strategy decision the parent should confirm before `tasks.md`.

## 2. Model decisions

### 2.1 Lifecycle vocabulary

Persist a `lifecycle TEXT NOT NULL DEFAULT 'active'` column on `products` with the CHECK constraint:

```sql
CHECK(lifecycle IN ('active', 'archived', 'retired'))
```

`lifecycle` is canonical. The legacy `is_active INTEGER NOT NULL DEFAULT 1` column is **preserved** for the transition window so existing read paths that still consult it keep working unchanged. See §3 for the dual-read window and the retirement plan for `is_active`.

**Barcode lifecycle mirror column.** The `product_barcodes` table gains a sibling `lifecycle TEXT NOT NULL DEFAULT 'active' CHECK(lifecycle IN ('active', 'archived', 'retired'))` column. SQLite partial-index `WHERE` clauses cannot reference other tables (they are restricted to columns of the indexed table — see https://www.sqlite.org/partialindex.html §7), so a partial unique index over `product_barcodes(barcode) WHERE <parent product is not retired>` is not expressible in SQL. The mirror column makes the index a single-table predicate (`WHERE lifecycle != 'retired'`), which is the only legal shape. The invariant is **service-layer-only**: every code path that mutates `products.lifecycle` updates every `product_barcodes` row for that product in the same database transaction. The mirror is therefore denormalised data whose value is governed by an explicit, transactional invariant, not by a SQLite trigger or generated column. Triggers were considered and rejected because they fire outside the calling service's transaction boundaries and make the audit-event atomicity harder to reason about. Generated columns were rejected because they are read-only in SQLite and cannot be reassigned when a barcode migrates from one product to another (the only path that could cause a barcode row to outlive its parent's lifecycle is `remove_barcode`, which deletes the row outright — see §6.4).

The mirror column carries the same value as the parent's lifecycle at every commit boundary, with three rules:

1. **Insert.** When `add_barcode` writes a new row, the row's `lifecycle` mirrors the parent product's current `lifecycle`.
2. **Lifecycle transition on the parent.** When `apply_lifecycle_transition` updates `products.lifecycle`, the same transaction issues an `UPDATE product_barcodes SET lifecycle = <new> WHERE product_id = <id>`. This covers all three transitions (`active → archived`, `archived → active`, `active|archived → retired`).
3. **Removal.** `remove_barcode` deletes the row outright; no historical `lifecycle` row is retained because the runtime model treats barcodes as current state, not history (see explore evidence §2.1).

Allowed transitions (spec §product lifecycle states):

| From | To | Trigger | Audit `event_type` | Reason required |
|---|---|---|---|---|
| `active` | `archived` | `archive_product` IPC | `archived` | no |
| `archived` | `active` | `unarchive_product` IPC | `unarchived` | no |
| `active` | `retired` | `retire_product` IPC | `retired` | yes |
| `archived` | `retired` | `retire_product` IPC | `retired` | yes |
| `retired` | *any* | — (blocked at service layer) | — | — |

Retire is one-way. Every transition writes exactly one `product_lifecycle_events` row in the same database transaction as the `products` mutation.

### 2.2 SKU / barcode uniqueness carve-out

Replace the V2 constraints:

```sql
products.sku TEXT NOT NULL UNIQUE                  -- line 84
product_barcodes.barcode TEXT NOT NULL UNIQUE      -- line 101
```

with same-table partial UNIQUE indexes that exclude retired rows:

```sql
CREATE UNIQUE INDEX uq_products_sku_active
    ON products(sku) WHERE lifecycle != 'retired';

CREATE UNIQUE INDEX uq_product_barcodes_barcode_active
    ON product_barcodes(barcode) WHERE lifecycle != 'retired';
```

Both predicates reference only the indexed table's own columns, which is the **only** SQLite-legal shape for a partial index. SQLite's `partialindex.html` documentation explicitly forbids subqueries and references to other tables in the `WHERE` clause of a partial index (the entire predicate must be evaluable using only the columns of the table being indexed). A cross-table `WHERE EXISTS (SELECT 1 FROM products WHERE ...)` form, although superficially appealing, is **not legal** and is rejected by SQLite's parser.

The `product_barcodes.barcode` index therefore constrains the per-row `lifecycle` mirror described in §2.1. The mirror's service-layer invariant — every `product_barcodes` row tracks the parent product's lifecycle — is what makes the same-table predicate correct: rows where the parent was retired have `lifecycle = 'retired'` and are excluded from uniqueness, so the same barcode value can be reused by a future active product. The invariant is the contract; the partial index is the enforcement.

The error string the Rust service relies on — `UNIQUE constraint failed: products.sku` / `…product_barcodes.barcode` — is unchanged because the partial-index name never appears in SQLite's error message; only the table name and column name do. `services::products::product_unique_error` and `barcode_unique_error` therefore continue to work without translation changes.

### 2.3 Legacy `is_active` reads during the transition window

For at least one release after this change lands, **read paths may consult either `is_active` or `lifecycle`** depending on what the column was at write time. The new V19 migration projects every row at backfill time:

- `is_active = 0` → `lifecycle = 'archived'`
- `is_active = 1` → `lifecycle = 'active'`

No row projects to `retired` at backfill time. The legacy `is_active` column stays on disk so:

1. Existing code that still filters `is_active = 1` keeps producing the correct result for legacy rows.
2. New code filters on `lifecycle` and is the canonical contract.
3. The migration is fully reversible: dropping the column would require no service-layer re-projection.

The compatibility rule for service-layer reads (see §6.2) is: **`lifecycle` is the source of truth; `is_active` is consulted only inside the dual-read window before the explicit retirement commit lands in a later change.** Every read helper added by this change filters on `lifecycle` exclusively. Every read helper that already filters on `is_active` continues to work because V19 keeps the projection consistent.

A follow-up change after the transition window can drop `is_active` and rewrite the few remaining read sites; that change is **explicitly out of scope** for `product-lifecycle-reusable-identifiers`.

### 2.4 Scanner resolver vs. dashboard search

Two distinct read paths today:

| Path | File | Current filter | New filter |
|---|---|---|---|
| Scanner `resolve_scanner_code` | `services/scanner.rs` | `is_active = 1` (via `find_active_product_by_barcode_exact` / `…by_sku_exact`) | `lifecycle = 'active'` (drop both archived AND retired) |
| Dashboard `find_product_by_scan` | `services/products.rs:465` | none — includes archived | none — continues to include archived AND retired; UI badge makes state clear |
| Dashboard product-create quick modal | `DashboardPage.svelte` quick-create | n/a — creates new product from SKU/barcode input | rejects SKU/barcode that match only `retired` rows via the same active helper the Scanner uses |

Lot resolution (`expiry_lots.batch_code` lookup in `services/scanner.rs`) is **unaffected**: a lot that was created before the parent was retired continues to resolve by its batch code. The lot detail view renders a `Retired product` badge on the parent product header so the operator knows the parent is terminal. This matches the spec's `lot-code scans for retired products` requirement.

### 2.5 CSV import preview — released-SKU/barcode distinction

The current `CsvPreviewRowStatus` enum has two collision variants — `DuplicateSku` and `DuplicateBarcode` — both of which hard-block the import. The new behavior is **three** states, distinguishable at preview time:

| Match surface | Variant | Block? |
|---|---|---|
| SKU matches `active` or `archived` row | `DuplicateSku` | yes |
| SKU matches only `retired` rows | `ReleasedSku` | no — passes through with advisory notice |
| Barcode matches `active` or `archived` row | `DuplicateBarcode` | yes |
| Barcode matches only `retired` rows | `ReleasedBarcode` | no — passes through with advisory notice |
| SKU or barcode matches both `active/archived` and `retired` | `DuplicateSku` / `DuplicateBarcode` (the active-or-archived match wins) | yes |

The `ReleasedSku` / `ReleasedBarcode` variants carry the `existing_product_id` of the retired row (so the UI can render a "previously associated with product X" tooltip) but **do not block import**. The frontend renders a per-row badge and a localized "this identifier was previously associated with a retired product" notice.

The classify-row helper in `services/csv_io.rs:417` is the single insertion point for the new variants — see §6.6.

## 3. Schema / migration strategy

### 3.1 Next fresh migration index: V19

`src-tauri/src/db/migrations.rs` ends at V18 (`app_settings_theme_key_milestone`). Per the V15 comment, sqlx pins a SHA-384 checksum on every migration; existing migrations MUST NOT be edited. The new migration lands as **V19** at the next fresh index, with the description `add_product_lifecycle_reusable_identifiers_v19`.

The migration is idempotent on its own (it adds columns with `DEFAULT`, back-fills from `is_active`, drops the legacy UNIQUE constraints, and creates the partial indexes), so a re-run on a partially-applied database is safe. The `MIGRATIONS` constant entry:

```rust
(
    19,
    "add_product_lifecycle_reusable_identifiers_v19",
    r#"
    -- See §3.2 for the full body. The migration rebuilds `products` and
    -- `product_barcodes` (table-rebuild pattern, mirrors V17) to add the
    -- `lifecycle` mirror column to both, drops the implicit V2 UNIQUE
    -- autoindexes, and creates explicit same-table partial UNIQUE indexes
    -- on `products(sku) WHERE lifecycle != 'retired'` and
    -- `product_barcodes(barcode) WHERE lifecycle != 'retired'`. The
    -- audit table `product_lifecycle_events` is created last.
    "#,
),
```

`build_migrator()` already wires the `(version, description, sql)` tuple into sqlx's `Migrator` with `MigrationType::Simple` and `no_tx: false`. V19 therefore commits or rolls back atomically inside the migration transaction — the explore evidence on `migrations.rs` lines 439–445 confirms the harness behaviour. The `PRAGMA defer_foreign_keys = ON` at the top of the V19 body (see §3.2) is transaction-local and lets the rebuild's `DROP TABLE products` step run while FK enforcement is enabled at the pool level; the deferred FK check at COMMIT validates every reference against the renamed table.

### 3.2 V19 transaction body

The single-transaction V19 SQL rebuilds `products` and `product_barcodes` to add the `lifecycle` column to both and to drop the implicit `UNIQUE` constraints that back the V2 autoindexes. The rebuild mirrors the V17 `lot_movements` table-rebuild pattern (`migrations.rs` V17, lines 604–671): create the new shape, copy every row, drop the old table, rename, recreate indexes. The legacy V2 autoindexes for `products.sku` and `product_barcodes.barcode` cannot be dropped directly via `DROP INDEX sqlite_autoindex_*` because SQLite refuses to drop autoindexes that back table-level `UNIQUE` or `PRIMARY KEY` constraints; the rebuild pattern is the supported workaround and is exactly what V17 did for `lot_movements`.

```sql
-- Defer foreign-key checks until COMMIT so the DROP TABLE products step is
-- legal. expiry_lots.product_id, product_categories.product_id, and
-- product_barcodes.product_id all reference products(id); with FK
-- enforcement enabled (pool.rs line 18-19 sets PRAGMA foreign_keys = ON for
-- every connection), DROP TABLE products would otherwise fail with
-- "foreign key constraint failed". defer_foreign_keys is transaction-local,
-- so it applies only to this migration's transaction and the FK check runs
-- at COMMIT, when products exists again under the same name with every
-- copied row.
PRAGMA defer_foreign_keys = ON;

-- 1. Rebuild products without the inline UNIQUE on `sku` and add `lifecycle`.
CREATE TABLE products_v19 (
    id                     TEXT PRIMARY KEY,
    sku                    TEXT NOT NULL,
    description            TEXT NOT NULL,
    category_id            TEXT REFERENCES categories(id),
    default_unit           TEXT,
    default_alert_days_before INTEGER NOT NULL DEFAULT 30,
    notes                  TEXT,
    is_active              INTEGER NOT NULL DEFAULT 1,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL,
    lifecycle              TEXT NOT NULL DEFAULT 'active'
                           CHECK(lifecycle IN ('active', 'archived', 'retired'))
);

INSERT INTO products_v19 (
    id, sku, description, category_id, default_unit,
    default_alert_days_before, notes, is_active, created_at, updated_at
)
SELECT id, sku, description, category_id, default_unit,
       default_alert_days_before, notes, is_active, created_at, updated_at
FROM products;

-- 2. Back-fill lifecycle from the legacy is_active column. Every existing
--    row projects to 'active' or 'archived'; no row projects to 'retired'
--    at backfill time (the spec scenario in §product lifecycle states).
UPDATE products_v19
SET lifecycle = CASE
    WHEN is_active = 0 THEN 'archived'
    ELSE 'active'
END
WHERE lifecycle = 'active';

-- 3. Swap products over to the new shape. The DROP drops the V2 implicit
--    autoindex `sqlite_autoindex_products_1` along with the table; the
--    RENAME keeps the table name stable so every FK from expiry_lots,
--    product_categories, product_barcodes, and product_lifecycle_events
--    resolves to the new table by name.
DROP TABLE products;
ALTER TABLE products_v19 RENAME TO products;

-- 4. Create the partial UNIQUE index on the new products. The legacy
--    whole-table UNIQUE is gone, replaced by an explicit partial index
--    whose predicate is single-table and therefore SQLite-legal.
CREATE UNIQUE INDEX uq_products_sku_active
    ON products(sku) WHERE lifecycle != 'retired';

-- 5. Rebuild product_barcodes without the inline UNIQUE on `barcode` and
--    add the sibling `lifecycle` column that mirrors the parent product.
--    No other table references product_barcodes by FK, so the DROP here
--    succeeds without defer_foreign_keys.
CREATE TABLE product_barcodes_v19 (
    id           TEXT PRIMARY KEY,
    product_id   TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    barcode      TEXT NOT NULL,
    barcode_type TEXT,
    is_primary   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    lifecycle    TEXT NOT NULL DEFAULT 'active'
                 CHECK(lifecycle IN ('active', 'archived', 'retired'))
);

INSERT INTO product_barcodes_v19 (
    id, product_id, barcode, barcode_type, is_primary, created_at
)
SELECT id, product_id, barcode, barcode_type, is_primary, created_at
FROM product_barcodes;

-- 6. Back-fill product_barcodes.lifecycle from the parent product's
--    lifecycle at V19 apply time. No row projects to 'retired' here
--    because no product has been retired yet (mirrors step 2).
UPDATE product_barcodes_v19
SET lifecycle = (SELECT lifecycle
                 FROM products
                 WHERE products.id = product_barcodes_v19.product_id);

-- 7. Swap product_barcodes over and create its partial UNIQUE index.
DROP TABLE product_barcodes;
ALTER TABLE product_barcodes_v19 RENAME TO product_barcodes;
CREATE UNIQUE INDEX uq_product_barcodes_barcode_active
    ON product_barcodes(barcode) WHERE lifecycle != 'retired';

-- 8. Audit table.
CREATE TABLE product_lifecycle_events (
    id         TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL
               CHECK(event_type IN ('archived', 'unarchived', 'retired')),
    from_state TEXT NOT NULL
               CHECK(from_state IN ('active', 'archived', 'retired')),
    to_state   TEXT NOT NULL
               CHECK(to_state IN ('active', 'archived', 'retired')),
    actor      TEXT,
    reason     TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_product_lifecycle_events_product_created
    ON product_lifecycle_events(product_id, created_at DESC);
```

Notes:

- **Why a rebuild and not `ALTER TABLE … ADD COLUMN`.** SQLite supports `ALTER TABLE … ADD COLUMN` with a constant `DEFAULT`, so adding `lifecycle` to `products` and `product_barcodes` is technically possible without a rebuild. However, the V2 inline `UNIQUE` constraints are baked into `sqlite_autoindex_products_1` and `sqlite_autoindex_product_barcodes_1`, and SQLite does not support dropping those autoindexes from a live table — the `UNIQUE` clause can only be removed by recreating the table without the clause (V17 already established this pattern for `lot_movements`). Doing a single rebuild for both the `lifecycle` column add and the `UNIQUE` constraint removal keeps V19 to one transaction.
- **`PRAGMA defer_foreign_keys = ON`.** This is a transaction-local pragma; the FK check is deferred until COMMIT. With FK enforcement enabled in every pool connection (`pool.rs:18-19`), `DROP TABLE products` would otherwise fail because `expiry_lots.product_id`, `product_categories.product_id`, and `product_barcodes.product_id` all reference `products(id)`. The deferred check at COMMIT verifies that every FK reference points at a valid `products.id` value, which holds because the RENAME step preserves the table name and every copied row.
- **Single-table predicates for partial indexes.** Both partial indexes reference only the indexed table's own columns, satisfying the only SQLite-legal shape for a partial index predicate (see `partialindex.html` §7).
- **`ON DELETE CASCADE` on `product_lifecycle_events.product_id`.** Precautionary; matches the existing pattern on `lot_movements.expiry_lot_id`. Hard delete is not shipped by this change.
- **The migration does not drop `is_active`.** See §2.3. The dual-read window keeps legacy read paths working until a follow-up change retires the column.

### 3.3 Migration tests

V19 tests live next to the existing `migrations.rs` tests at the bottom of that file. Required cases (each test uses `fresh_test_pool()` for a clean apply):

1. **`v19_applies_on_fresh_db`** — total migration count after a fresh apply is **19** (matches `MIGRATIONS.len()`).
2. **`v19_adds_lifecycle_column`** — `PRAGMA table_info(products)` includes `lifecycle` with type `TEXT`, `NOT NULL`, default `'active'`.
3. **`v19_partial_unique_indexes_exist`** — `sqlite_master` rows include `uq_products_sku_active` and `uq_product_barcodes_barcode_active`; `sqlite_master` no longer contains `sqlite_autoindex_products_1` or `sqlite_autoindex_product_barcodes_1` (the rebuild drops them implicitly because they lived on the inline `UNIQUE` clauses the rebuild removed).
4. **`v19_creates_product_lifecycle_events`** — table exists with the documented columns and CHECKs.
5. **`v19_backfill_is_active_zero_to_archived`** — insert a row with `is_active = 0` via direct SQL (bypassing the service layer), run V19, verify `lifecycle = 'archived'`.
6. **`v19_backfill_is_active_one_to_active`** — symmetric case.
7. **`v19_backfill_product_barcodes_lifecycle_mirrors_parent`** — after V19, every `product_barcodes.lifecycle` value matches its parent product's `lifecycle` at V19 apply time.
8. **`v19_backfill_idempotent`** — running V19 twice does not double-projection (sqlx refuses re-application once `_sqlx_migrations` records V19, so this test asserts that the partial-index predicate keeps existing rows compatible across re-runs and that re-applied migrations don't disturb existing data).
9. **`v19_preserves_expiry_lots_fk_after_products_rebuild`** — after V19, every `expiry_lots.product_id` resolves to a valid `products.id` (no orphaned rows). This is the regression guard for the deferred-FK path used during the table rebuild.
10. **`v19_preserves_product_categories_fk_after_products_rebuild`** — symmetric guard for `product_categories`.
11. **`v19_sku_unique_against_active_but_reusable_after_retire`** — create a product, verify duplicate SKU is blocked; retire it; create a new product with the same SKU and verify success.
12. **`v19_barcode_unique_against_active_but_reusable_after_parent_retire`** — attach a barcode to an active product, verify duplicate barcode is blocked; retire the parent; attach the same barcode to a different active product and verify success.
13. **`v19_audit_row_atomic_with_mutation`** — insert + lifecycle update must roll back together when the audit insert fails (test by stubbing the audit insert with an invalid `to_state`).

## 4. `product_lifecycle_events` schema and service/write boundaries

### 4.1 Schema (frozen by V19)

```sql
CREATE TABLE product_lifecycle_events (
    id         TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK(event_type IN ('archived', 'unarchived', 'retired')),
    from_state TEXT NOT NULL CHECK(from_state IN ('active', 'archived', 'retired')),
    to_state   TEXT NOT NULL CHECK(to_state IN ('active', 'archived', 'retired')),
    actor      TEXT,
    reason     TEXT,
    created_at TEXT NOT NULL
);
```

### 4.2 DTOs

New types in `src-tauri/src/dto/products.rs`:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductLifecycle {
    Active,
    Archived,
    Retired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductLifecycleEventType {
    Archived,
    Unarchived,
    Retired,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductLifecycleEventResponse {
    pub id: String,
    pub product_id: String,
    pub event_type: ProductLifecycleEventType,
    pub from_state: ProductLifecycle,
    pub to_state: ProductLifecycle,
    pub actor: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RetireProductInput {
    pub id: String,
    pub reason: String,                // required, non-blank after trim
    pub actor: Option<String>,
}
```

### 4.3 Write boundaries

Every lifecycle mutation runs through a single transactional helper in `services/products.rs`. The helper updates `products.lifecycle`, fans out the change to every `product_barcodes` row owned by that product (per §2.1's mirror invariant), and writes the audit row — all in the same database transaction.

```rust
async fn apply_lifecycle_transition(
    pool: &DbPool,
    product_id: &str,
    expected_from: Option<ProductLifecycle>,   // None = no precondition
    target: ProductLifecycle,
    event_type: ProductLifecycleEventType,
    actor: Option<String>,
    reason: Option<String>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    // 1. SELECT current lifecycle FOR UPDATE (sqlite single-writer model
    //    gives us this implicitly inside the transaction; we still
    //    re-SELECT to validate the precondition atomically).
    let current = repo::get_product_lifecycle(&mut *tx, product_id).await?;
    let current = current.ok_or(DomainError::NotFound { resource: "product", id: product_id.clone() })?;

    // 2. Validate precondition.
    if let Some(expected) = expected_from {
        if current != expected {
            return Err(DomainError::Conflict {
                message: format!("product lifecycle is {current:?}, expected {expected:?}"),
            }.into());
        }
    }
    if current == ProductLifecycle::Retired {
        return Err(DomainError::BusinessRule {
            message: user_message(UserMessage::ProductRetiredForMutation, Locale::En),
        }.into());
    }
    if target == ProductLifecycle::Retired && reason.as_deref().map(str::trim).map_or(true, str::is_empty) {
        return Err(DomainError::Validation {
            message: user_message(UserMessage::RetireReasonRequired, Locale::En),
        }.into());
    }

    // 3. Apply the lifecycle update on products.
    repo::set_product_lifecycle(&mut *tx, product_id, target).await?;

    // 4. Mirror the lifecycle change onto every product_barcodes row for
    //    this product (the §2.1 service-layer invariant). One UPDATE, no
    //    per-row reads — the partial index on product_barcodes(barcode)
    //    is unaffected because we are not changing any barcode value.
    repo::sync_product_barcodes_lifecycle(&mut *tx, product_id, target).await?;

    // 5. Write the audit row.
    let event_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO product_lifecycle_events
            (id, product_id, event_type, from_state, to_state, actor, reason, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&event_id)
    .bind(product_id)
    .bind(event_type)
    .bind(current)
    .bind(target)
    .bind(&actor)
    .bind(&reason)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
```

**Barcode lifecycle semantics per transition.** The mirror step in step 4 propagates the parent's new lifecycle to every `product_barcodes` row owned by that product, with the following rules:

- `active → archived`: every owned `product_barcodes.lifecycle` becomes `archived`.
- `archived → active` (unarchive): every owned `product_barcodes.lifecycle` becomes `active`. (Retired barcode rows under other products are untouched.)
- `active|archived → retired`: every owned `product_barcodes.lifecycle` becomes `retired`. The partial index then excludes those rows from barcode uniqueness, releasing the values for reuse.

Retired barcode rows are terminal for their own lifecycle because their parent product is retired and `apply_lifecycle_transition` rejects any attempt to mutate a retired product (step 2 above). A barcode can therefore never escape `retired` once set.

Three thin service-layer wrappers call into this helper:

```rust
pub async fn archive_product(pool: &DbPool, id: String)
    -> Result<(), AppError>;
pub async fn unarchive_product(pool: &DbPool, id: String)
    -> Result<(), AppError>;
pub async fn retire_product(pool: &DbPool, input: RetireProductInput)
    -> Result<(), AppError>;
```

The existing `archive_product` service retains its current `repo::archive_product` call path during the transition window so legacy `is_active` projection stays consistent. After the helper is in place, `repo::archive_product` becomes a one-liner that calls the same `UPDATE products SET lifecycle = 'archived', is_active = 0` and writes the audit row in the same transaction. `repo::archive_product` continues to set `is_active = 0` for the dual-read window.

The `update_product` flow continues to use its own transaction (categories + product row). If a lifecycle-changing `is_active` toggle arrives via the edit form, the service branches into `apply_lifecycle_transition` after the metadata UPDATE commits; this preserves the existing single-row edit semantics while recording the audit event.

### 4.4 Read boundaries

- `repo::get_product_lifecycle(pool, id) -> Option<ProductLifecycle>` — single-column SELECT for use inside transactions.
- `repo::list_lifecycle_events(pool, product_id) -> Vec<ProductLifecycleEventResponse>` — chronological DESC, paginated by 50 in the UI.
- The lifecycle field rides along on every existing `ProductResponse` and `ProductSearchResult` (already serialized into JSON via `is_active` — see §6.2 for the dual-read mapping).

### 4.5 IPC wiring

`src-tauri/src/commands/products.rs` adds:

```rust
#[tauri::command]
pub async fn unarchive_product(state: State<'_, AppState>, id: String)
    -> Result<(), CommandError>;

#[tauri::command]
pub async fn retire_product(state: State<'_, AppState>, input: RetireProductInput)
    -> Result<(), CommandError>;

#[tauri::command]
pub async fn list_product_lifecycle_events(
    state: State<'_, AppState>,
    product_id: String,
) -> Result<Vec<ProductLifecycleEventResponse>, CommandError>;
```

`lib.rs` `invoke_handler` registers the three new commands alongside the existing `archive_product` (which is preserved verbatim for the transition window — see §6.2). The frontend invokes them via three new typed wrappers in `src/lib/products.ts`:

```ts
export async function unarchiveProduct(id: string): Promise<void>;
export async function retireProduct(input: RetireProductInput): Promise<void>;
export async function listProductLifecycleEvents(productId: string):
    Promise<ProductLifecycleEventResponse[]>;
```

### 4.6 User message additions

`src-tauri/src/services/user_messages.rs` gets two new variants:

```rust
/// Product catalog business rule: a retired product cannot be mutated
/// other than by read.
ProductRetiredForMutation,
/// Product catalog business rule: retire requires a non-blank reason.
RetireReasonRequired,
/// Product catalog business rule: a barcode cannot be added to a
/// retired product (parallel to ProductArchivedForBarcode).
ProductRetiredForBarcode,
```

Each variant gets English + Spanish translations and a stable string code (`product_retired_for_mutation`, `retire_reason_required`, `product_retired_for_barcode`). The string-code table is the same shape as `CHECK_CODE_REQUIRED_TABLES_PRESENT` etc. on the backup/restore side.

## 5. Compatibility strategy between legacy `is_active` and new `lifecycle`

### 5.1 Backfill is a literal projection

V19 back-fills `lifecycle` from `is_active` per §3.2 step 2, then back-fills `product_barcodes.lifecycle` from the parent product's `lifecycle` per §3.2 step 6. Every read path that uses `lifecycle` after V19 applies sees a row whose `lifecycle` exactly mirrors its `is_active` projection (for products) and whose barcode rows exactly mirror the parent's `lifecycle` (for `product_barcodes`). The columns therefore agree at the moment V19 finishes, which means:

- New code reading `lifecycle` produces the same set of rows as legacy code reading `is_active = 1`.
- New code reading `lifecycle = 'retired'` produces the empty set until the first retire IPC fires.
- `is_active = 0` legacy rows are reachable as `lifecycle = 'archived'` AND `is_active = 0` simultaneously, so the existing archived-product filter on the legacy scanner helpers continues to match them.
- Every existing `product_barcodes` row inherits `lifecycle = 'active'` or `lifecycle = 'archived'` from its parent product at V19 apply time, so the partial unique index on barcode (`uq_product_barcodes_barcode_active`) does not suddenly admit previously-blocked duplicates — the backfill preserves the same uniqueness behaviour the inline `UNIQUE` constraint enforced.

### 5.2 Dual-read window

The transition window runs from the moment V19 applies until the explicit follow-up change drops `is_active`. During the window:

| Path | Behaviour |
|---|---|
| New code (added by this change) | Reads `lifecycle` exclusively. |
| Legacy code (untouched) | Reads `is_active` exclusively. |
| `repo::archive_product` | Sets BOTH `lifecycle = 'archived'` AND `is_active = 0` so both projections agree; also propagates `lifecycle = 'archived'` to every owned `product_barcodes` row (the §2.1 mirror invariant). |
| `repo::unarchive_product` | Sets BOTH `lifecycle = 'active'` AND `is_active = 1`; also propagates `lifecycle = 'active'` to every owned `product_barcodes` row. Retired barcode rows under other products are not touched. |
| `repo::retire_product` | Sets `lifecycle = 'retired'` AND propagates `lifecycle = 'retired'` to every owned `product_barcodes` row (the partial index excludes these rows from barcode uniqueness). DOES NOT touch `is_active` (so legacy code that filters on `is_active = 1` still sees the retired row, which is correct because the spec says legacy read paths may continue to operate). |

A single helper keeps the two columns consistent:

```rust
fn set_lifecycle_and_legacy_is_active(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    product_id: &str,
    lifecycle: ProductLifecycle,
    legacy_is_active: Option<bool>,   // None = don't touch is_active (retire)
) -> Result<(), sqlx::Error> {
    match legacy_is_active {
        Some(flag) => {
            sqlx::query(
                "UPDATE products
                 SET lifecycle = $1, is_active = $2, updated_at = $3
                 WHERE id = $4",
            )
            .bind(lifecycle)
            .bind(i32::from(flag))
            .bind(Utc::now().to_rfc3339())
            .bind(product_id)
            .execute(&mut **tx)
            .await?;
        }
        None => {
            sqlx::query(
                "UPDATE products
                 SET lifecycle = $1, updated_at = $2
                 WHERE id = $3",
            )
            .bind(lifecycle)
            .bind(Utc::now().to_rfc3339())
            .bind(product_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}
```

`retire_product` calls with `legacy_is_active = None`; `archive_product` and `unarchive_product` call with the matching `is_active` flag.

### 5.3 Frontend compatibility

The frontend's `ProductSearchResult` and `ProductResponse` already carry `is_active`. The new code adds an optional `lifecycle` field on the wire:

```ts
// src/lib/products.ts
export type ProductLifecycle = "active" | "archived" | "retired";

export interface ProductResponse {
    // ... existing fields ...
    is_active: boolean;
    lifecycle?: ProductLifecycle;   // present after this change lands
}

export interface ProductSearchResult {
    // ... existing fields ...
    is_active: boolean;
    lifecycle?: ProductLifecycle;
}
```

The optional field lets the older frontend binary (without this change) keep reading the same `is_active` field without runtime errors. The catalog / detail / dashboard UI surfaces map `lifecycle ?? (is_active ? "active" : "archived")` for the badge so the UX is correct regardless of which backend the user happens to be talking to during a rolling deployment.

### 5.4 Retirement of `is_active`

Out of scope. A separate change after the transition window will:

1. Drop the `is_active` column from `products`.
2. Rewrite the four or five legacy read paths that still filter on `is_active`.
3. Update the `ProductResponse` / `ProductSearchResult` shape to remove `is_active`.

The current change does **not** schedule that follow-up.

## 6. Backend service / repository / API changes

### 6.1 Lifecycle service (`services/products.rs`)

Additions:

```rust
pub async fn unarchive_product(pool: &DbPool, id: String) -> Result<(), AppError>
pub async fn retire_product(pool: &DbPool, input: RetireProductInput) -> Result<(), AppError>
pub async fn list_lifecycle_events(pool: &DbPool, product_id: String)
    -> Result<Vec<ProductLifecycleEventResponse>, AppError>
```

Refactor of the existing `archive_product` service to call the new transactional helper; the public signature stays identical so `commands/products.rs::archive_product` does not need to change.

Validation rules:

- `archive_product` rejects products already in `archived` (idempotent no-op) and `retired` (business rule `ProductRetiredForMutation`).
- `unarchive_product` rejects products already in `active` (idempotent no-op) and `retired` (business rule).
- `retire_product` rejects products already in `retired` (idempotent no-op) and rejects `reason` strings that are empty after trim.

### 6.2 Repository (`db/repositories/products.rs`)

Additions:

```rust
/// Single-column read used inside lifecycle transactions.
pub async fn get_product_lifecycle(
    pool: &SqlitePool, id: &str,
) -> Result<Option<ProductLifecycle>, sqlx::Error>;

/// Mutator used inside lifecycle transactions.
pub async fn set_product_lifecycle(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    lifecycle: ProductLifecycle,
) -> Result<bool, sqlx::Error>;

/// Mirror the parent product's lifecycle onto every product_barcodes row
/// for that product. Called inside `apply_lifecycle_transition` so the
/// §2.1 service-layer invariant holds at every commit boundary.
pub async fn sync_product_barcodes_lifecycle(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    product_id: &str,
    lifecycle: ProductLifecycle,
) -> Result<u64, sqlx::Error>;

/// Reads lifecycle events for a product, newest first.
pub async fn list_lifecycle_events(
    pool: &SqlitePool, product_id: &str,
) -> Result<Vec<ProductLifecycleEventResponse>, sqlx::Error>;
```

The `sync_product_barcodes_lifecycle` helper executes one statement:

```sql
UPDATE product_barcodes
SET lifecycle = $1
WHERE product_id = $2;
```

The partial unique index on `product_barcodes(barcode) WHERE lifecycle != 'retired'` is unaffected by this UPDATE because the index entry's value (`barcode`) does not change; only the indexed predicate's eligibility can change. SQLite invalidates affected index entries lazily on the next access, so the UPDATE does not incur a full index rebuild.

Modifications:

- `archive_product` is rewritten to use `set_lifecycle_and_legacy_is_active` (§5.2) so both columns agree, calls `sync_product_barcodes_lifecycle` with `archived`, and writes an audit row in the same transaction.
- `unarchive_product` calls `set_lifecycle_and_legacy_is_active` with `active` and `is_active = 1`, then calls `sync_product_barcodes_lifecycle` with `active`. Retired barcode rows under other products are not touched.
- `retire_product` sets `lifecycle = 'retired'` (without touching `is_active` per §5.2), calls `sync_product_barcodes_lifecycle` with `retired`, and writes the audit row in the same transaction. The barcode partial index then excludes those rows from uniqueness, releasing the values for reuse.
- `find_active_product_by_barcode_exact` / `find_active_product_by_sku_exact` change `AND products.is_active = 1` to `AND products.lifecycle = 'active'` so retired products are also excluded. The scanner resolver continues to read from `products` only; `product_barcodes.lifecycle` is consulted by SQLite via the partial index for the `add_barcode` UNIQUE check, not by scanner read paths.
- `search_products` (used by both dashboard and catalog) gains an optional `lifecycle` filter — `None` returns every state, `Some(active)` returns only active rows. The dashboard's `find_product_by_scan` path keeps `lifecycle = None` so it includes archived and retired rows.
- `RawProductRow` gains a `lifecycle: String` field. `into_response` and `into_search_result` project it to the wire DTO.
- `RawProductBarcodeRow` gains a `lifecycle: String` field. `into_barcode_response` projects it to the wire DTO so callers (the lifecycle history pane, the barcodes list inside the product detail view) can render the lifecycle badge per barcode row.

### 6.3 Scanner resolver (`services/scanner.rs`)

Because the repository helpers already filter on `lifecycle = 'active'`, the scanner service requires **no code change** — the rewrite of `find_active_product_by_barcode_exact` / `…by_sku_exact` is enough. The lot-code branch stays unchanged. The spec scenarios for retired-product barcode and retired-product SKU resolve to `Unknown`, which is the existing return path.

### 6.4 Barcode attach (`services/products.rs::add_barcode`)

The `if !product.is_active` check at `services/products.rs:454` is replaced by:

```rust
let product = repo::get_product(pool, &input.product_id).await?
    .ok_or_else(|| DomainError::NotFound { resource: "product", id: input.product_id.clone() })?;
match product.lifecycle {
    Some(ProductLifecycle::Retired) | Some(ProductLifecycle::Archived) => {
        return Err(DomainError::BusinessRule {
            message: user_message(UserMessage::ProductRetiredForBarcode, Locale::En),
        }.into());
    }
    _ => {}
}
```

The `Some(...)` matching on `Option<ProductLifecycle>` keeps backward compatibility with older backends that do not return the field; legacy behaviour (`is_active = false` blocks) is preserved.

### 6.5 Dashboard scan/search (`services/products.rs::find_product_by_scan`)

No service-layer change. The retired product is surfaced through `find_by_barcode_exact` / `find_by_sku_exact` (which include retired rows) so the dashboard's search result still returns the row. The frontend renders a `Retired` badge on the row and prevents the dashboard's quick product-create modal from routing the scanned value into a new product's SKU or barcode slot when the matched row is retired.

### 6.6 CSV import preview (`services/csv_io.rs::classify_row`)

The two collision checks at `services/csv_io.rs:432` (`DuplicateSku`) and `:462` (`DuplicateBarcode`) become three-state classifiers. The new logic per row:

```rust
// SKU collision check.
match crate::db::repositories::products::find_by_sku_exact(pool, &sku).await {
    Ok(Some(existing)) => {
        match existing.lifecycle {
            Some(ProductLifecycle::Retired) => {
                // Released SKU — advisory notice, NOT a block.
                return (CsvPreviewRowStatus::ReleasedSku {
                    existing_product_id: existing.id,
                    existing_sku: existing.sku,
                }, /* ... */);
            }
            _ => {
                return (CsvPreviewRowStatus::DuplicateSku {
                    existing_product_id: existing.id,
                    existing_sku: existing.sku,
                }, /* ... */);
            }
        }
    }
    Ok(None) => {}
    Err(e) => { /* existing DB-error branch */ }
}
```

The barcode check follows the same pattern. The new `CsvPreviewRowStatus` variants are added to `src-tauri/src/dto/csv_io.rs`:

```rust
pub enum CsvPreviewRowStatus {
    Ok,
    DuplicateSku { existing_product_id: String, existing_sku: String },
    DuplicateBarcode { existing_product_id: String, existing_barcode: String },
    ReleasedSku { existing_product_id: String, existing_sku: String },
    ReleasedBarcode { existing_product_id: String, existing_barcode: String },
    MissingRequired { field: String },
    Invalid { reason: String, reason_code: Option<String> },
    UnknownUnit { raw_value: String, suggested_keys: Vec<String> },
}
```

The `CsvPreviewResponse` summary gains two new counters — `released_sku_rows` and `released_barcode_rows` — and the `valid_rows` / `invalid_rows` accounting shifts: a `ReleasedSku` row counts as `valid_rows` (it does not block import) but not as `Ok` (the UI must render the advisory badge).

### 6.7 Backup / restore (`services/backup_restore.rs`)

`REQUIRED_TABLES` at `services/backup_restore.rs:30` adds `"product_lifecycle_events"`. The new migration adds the table to the same database that `VACUUM INTO` snapshots, so the round-trip is automatic; the explicit `REQUIRED_TABLES` entry is the only change required to make validation accept backups that contain the new table.

`export_backup` and `validate_backup` are otherwise untouched — the audit table rides along with every backup because `VACUUM INTO` snapshots the whole database.

### 6.8 Update product form (`update_product` flow)

The existing flow at `services/products.rs::update_product` reads `is_active` from the input, which the user can flip via a checkbox on the form. The change:

- If the user flips `is_active` from `true` to `false`, the service additionally calls the lifecycle helper to record an `archived` audit row (with `actor = None`, `reason = None`) so the audit table reflects the form-driven archive.
- If the user flips `is_active` from `false` to `true`, the service records an `unarchived` audit row.
- A retire toggle is **NOT** added to the update form — retire is its own dedicated two-step UI flow (see §7.2).

This keeps the form path simple while still leaving an audit trail.

## 7. Frontend UI surfaces

### 7.1 `ProductCatalogPage.svelte`

Existing: hide-archived toggle persisted at `localStorage["caduxo.products.catalog.hideArchived.v1"]` (line 65).

Additions:

- New `showRetired` toggle persisted at `localStorage["caduxo.products.catalog.showRetired.v1"]`. Default `false`.
- New `lifecycle` field on each product row (computed as `lifecycle ?? (is_active ? "active" : "archived")`).
- New `Retired` badge rendered for `lifecycle === "retired"` rows alongside the existing `Archived` badge.
- When `showRetired` is on, retired rows are listed **after** every active and archived row in the default ordering. The existing `ORDER BY description ASC` at `services/products.rs` becomes `ORDER BY (lifecycle = 'retired') ASC, description ASC` so the SQL handles the bottom-sort directly without an in-memory re-sort.
- The existing `displayedProducts` derived value (lines 348–362) adds a parallel filter for `lifecycle === "retired"` against the toggle.

No layout restructuring. The hide-archived and show-retired toggles sit side by side; one is the inverse of the other for archived, but they are independent axes (a user can hide archived but show retired, or vice versa).

### 7.2 `ProductDetailPage.svelte`

Existing: single `confirmArchive` flow at line 156; banner copy at lines 305–319; `confirmingArchive` state variable.

Additions:

- New `confirmingUnarchive` and `confirmingRetire` state variables.
- New "Unarchive" button rendered conditionally on `detail.product.lifecycle === "archived"`. Single-step confirm (no reason required). On click, calls `unarchiveProduct(detail.product.id)`, hides the banner, reloads the detail.
- New "Retire" button rendered conditionally on `detail.product.lifecycle === "active"` or `=== "archived"`. Two-step confirm:
  1. First modal asks for a free-text reason (`retireReason`). The submit button is disabled until the trimmed reason is non-blank.
  2. Second modal asks for explicit "I understand this is irreversible" confirmation (button labelled with the i18n key `products.retireConfirm`). The submit button calls `retireProduct({ id, reason, actor: null })`.
- New "Lifecycle history" section rendered conditionally when `lifecycle !== "retired"` (the section is always present for retired rows so the user can see when the retire happened). Each entry shows event type, from→to states, timestamp, actor when present, reason when present. Read-only.
- The existing "Archived" banner copy is replaced by conditional copy keyed on lifecycle: `products.detail.bannerArchived`, `products.detail.bannerRetired`. The button row beneath the banner adapts: archived shows `[Unarchive] [Retire]`, retired shows no buttons (the only action is read-only history), active shows `[Archive] [Retire]`.

### 7.3 `ProductForm.svelte`

The existing `isActive` checkbox on line 92 stays as-is. Two new affordances:

- The form renders a `lifecycle` field below the existing `isActive` checkbox so the user can see the projected state. The field is read-only.
- The form's "Archive" quick-action (if it exists; explore notes no such button, only the detail page archive) is left untouched.

No new lifecycle controls on the create form — newly created products always start at `active` (spec §product lifecycle states, scenario `new product starts active`).

### 7.4 `DashboardPage.svelte`

Existing: product modal at lines 435–444 and 795; quick product-create path that does not consult lifecycle.

Additions:

- The product modal renders both `Archived` and `Retired` badges (parallel to the catalog change).
- The quick product-create modal rejects SKU or barcode values that match only retired rows: it uses the same scanner-style active-only helpers the ScannerPage uses, so a scan of a retired-only barcode returns `Unknown` and the user is forced to create a brand-new SKU/barcode. The spec scenario `dashboard scan/search surface includes retired products with a Retired badge` covers the search side; the create path is symmetric.

### 7.5 `ScannerPage.svelte`

No code change. The scanner resolver already filters on `is_active = 1` today; the rewrite of the two `find_active_product_by_*` helpers to filter on `lifecycle = 'active'` is enough to make the spec scenarios pass. Lot-code scans stay unchanged.

### 7.6 `CsvImportPage.svelte`

Additions:

- New per-row badge for `ReleasedSku` and `ReleasedBarcode` statuses with the localized notice: "this SKU was previously associated with a retired product — the new product will be created".
- The "Commit import" button now admits `ReleasedSku` / `ReleasedBarcode` rows alongside `Ok` rows.
- The summary panel shows `released_sku_rows` and `released_barcode_rows` counts alongside the existing `duplicate_sku_rows` / `duplicate_barcode_rows`.

### 7.7 i18n

`src/i18n/en/index.ts` and `src/i18n/es/index.ts` additions (parity required):

```text
products.unarchive                   (en line 365 already exists; wire to unarchive IPC)
products.retire
products.retired
products.retireReason
products.retireReasonLabel
products.retireConfirm
products.retireConfirmBody
products.retireIrreversible
products.lifecycleEvents
products.lifecycleEventArchived
products.lifecycleEventUnarchived
products.lifecycleEventRetired
products.retireHistoryNotice
products.lifecycleActive
products.lifecycleArchived
products.lifecycleRetired
products.catalog.showRetired
products.detail.bannerArchived
products.detail.bannerRetired
products.detail.unarchiveConfirm
products.detail.retireConfirm
csv.releasedSku
csv.releasedBarcode
csv.releasedSkuNotice
csv.releasedBarcodeNotice
dashboard.scanRetired
```

The existing `products.unarchive` key in `src/i18n/en/index.ts:365` ("Unarchive") has no consumer today — it gets wired to the new `unarchiveProduct` IPC. The Spanish locale tree must add the matching key (current explore evidence shows it does not exist there).

## 8. Test strategy

### 8.1 Backend automated tests

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

New test files / modules:

- `db/repositories/products.rs` — `archive_product_writes_lifecycle_event`, `unarchive_product_writes_lifecycle_event`, `retire_product_writes_lifecycle_event_with_reason`, `retire_product_rejects_blank_reason`, `retire_product_rejects_retired_product`, `set_lifecycle_and_legacy_is_active_keeps_columns_consistent`, `find_active_*_excludes_retired`, `sync_product_barcodes_lifecycle_propagates_to_owned_rows`, `sync_product_barcodes_lifecycle_does_not_touch_other_products`, `sync_product_barcodes_lifecycle_preserves_retired_rows`.
- `services/products.rs` — `unarchive_product_round_trip`, `retire_product_round_trip`, `retire_product_rejects_retired_product`, `add_barcode_rejects_retired`, `add_barcode_rejects_archived`, `archive_mirrors_lifecycle_to_owned_barcodes`, `unarchive_mirrors_active_to_owned_barcodes`, `retire_mirrors_retired_to_owned_barcodes_and_releases_value_for_reuse`, `reuse_released_barcode_succeeds_against_active_parent`.
- `services/csv_io.rs` — `classify_row_released_sku`, `classify_row_released_barcode`, `classify_row_active_duplicate_sku_wins_over_released`, `preview_summary_counts_released_rows_as_valid`.
- `services/scanner.rs` — `retired_product_barcode_does_not_resolve`, `retired_product_sku_does_not_resolve`, `lot_scan_for_retired_product_resolves_with_badge_signal` (signal = the resolved product carries the lifecycle field).
- `db/migrations.rs` — the V19 test cases from §3.3.
- `db/repositories/products.rs::tests` — `lifecycle_event_atomicity`: a forced failure inside the audit insert must roll back the lifecycle UPDATE **and** the `sync_product_barcodes_lifecycle` UPDATE, proving all three writes share a single transaction.

### 8.2 Frontend checks

Run:

```bash
npx svelte-check --workspace . --threshold error
npm run build
```

The `typesafe-i18n` build is the parity gate for the new keys listed in §7.7. Any missing key in `en` or `es` fails the build.

### 8.3 Manual smoke (recorded in `verify-report.md` after apply)

Required scenarios (one row per spec `#### Scenario` block):

- Archive via UI → lifecycle = archived, audit row written.
- Unarchive via UI → lifecycle = active, audit row written.
- Retire active product with reason → lifecycle = retired, audit row with reason.
- Retire archived product with reason → lifecycle = retired, audit row with reason.
- Retire rejected with blank reason (UI submit button disabled).
- Retired product cannot be edited / archived / unarchived / barcode-attached.
- Catalog list with `Show retired` off → retired products hidden.
- Catalog list with `Show retired` on → retired products listed at bottom with badge.
- Dashboard search of retired SKU → row surfaces with `Retired` badge.
- Scanner resolution of retired SKU → `Unknown`.
- Scanner resolution of retired barcode → `Unknown`.
- Scanner resolution of lot under retired parent → `LotMatch` with `Retired product` badge on the parent header.
- CSV import preview of retired SKU → `ReleasedSku` advisory notice; import commits new product.
- CSV import preview of active SKU collision → `DuplicateSku` blocks import.
- Backup / restore round-trip → audit table present after restore; lifecycle values intact.
- Migration on a database with existing archived and active rows → every archived row projects to `lifecycle = 'archived'`, every active row projects to `lifecycle = 'active'`, no row projects to `retired`.
- Migration on a fresh database → all V1–V19 apply cleanly, total migration count = 19.

## 9. File-by-file change summary

| File | Type of change |
|---|---|
| `src-tauri/src/db/migrations.rs` | Add V19 inline SQL (table-rebuild for `products` and `product_barcodes` with the §2.1 mirror column, partial unique indexes, audit table) + V19 tests |
| `src-tauri/src/db/repositories/products.rs` | Add lifecycle helpers, rewrite archive/unarchive/retire, add `sync_product_barcodes_lifecycle`, update active-only filters, add `lifecycle` to `RawProductRow` and `RawProductBarcodeRow` |
| `src-tauri/src/services/products.rs` | Add `unarchive_product` / `retire_product` / `list_lifecycle_events` services, refactor `archive_product` to use transactional helper, update `add_barcode` retired guard |
| `src-tauri/src/services/scanner.rs` | No code change; relies on repo filter rewrite |
| `src-tauri/src/services/csv_io.rs` | Update `classify_row` to emit `ReleasedSku` / `ReleasedBarcode`, update `CsvPreviewResponse` counters |
| `src-tauri/src/services/backup_restore.rs` | Add `product_lifecycle_events` to `REQUIRED_TABLES` |
| `src-tauri/src/services/user_messages.rs` | Add `ProductRetiredForMutation`, `RetireReasonRequired`, `ProductRetiredForBarcode` variants and translations |
| `src-tauri/src/dto/products.rs` | Add `ProductLifecycle`, `ProductLifecycleEventType`, `ProductLifecycleEventResponse`, `RetireProductInput` |
| `src-tauri/src/dto/csv_io.rs` | Add `ReleasedSku` / `ReleasedBarcode` variants to `CsvPreviewRowStatus`, add counters to `CsvPreviewResponse` |
| `src-tauri/src/commands/products.rs` | Add `unarchive_product`, `retire_product`, `list_product_lifecycle_events` IPC handlers |
| `src-tauri/src/lib.rs` | Register three new commands in `invoke_handler` |
| `src/lib/products.ts` | Add `unarchiveProduct`, `retireProduct`, `listProductLifecycleEvents`, `ProductLifecycleEventResponse`, `RetireProductInput` types |
| `src/components/ProductCatalogPage.svelte` | Add `showRetired` toggle, add `Retired` badge, bottom-sort |
| `src/components/ProductDetailPage.svelte` | Add `unarchive` and `retire` actions, lifecycle history section, conditional banner copy |
| `src/components/ProductForm.svelte` | Read-only `lifecycle` field |
| `src/components/DashboardPage.svelte` | Render `Retired` badge on product modal, retire-aware quick-create |
| `src/components/CsvImportPage.svelte` | Render `ReleasedSku` / `ReleasedBarcode` badges, admit them in the commit count |
| `src/i18n/en/index.ts` | Wire `products.unarchive`; add keys listed in §7.7 |
| `src/i18n/es/index.ts` | Add the matching keys for parity |
| `openspec/changes/product-lifecycle-reusable-identifiers/specs/caduxo-expiry-tracker/spec.md` | Already drafted (see input); the spec language around the partial-index carve-out is implementation-neutral and the corrected design satisfies it |
| `openspec/changes/product-lifecycle-reusable-identifiers/design.md` | This file (corrected for SQLite feasibility) |

## 10. Delivery split (review budget 400 changed lines)

The canonical session review budget is **400 changed lines** per `proposal.md`. The full feature is structurally above that ceiling: schema migration + lifecycle column + audit table + four new IPC handlers + scanner / barcode / CSV / dashboard / catalog / detail / form UI + i18n parity on two locales is comfortably **~900–1100 net** human-edited lines, ignoring tests. Auto-chain is therefore non-optional; `tasks.md` MUST be written as a chained plan.

The chain proposed for `tasks.md` is **3 stacked PRs**:

### PR 1 — Backend foundation (~360–420 net)

Files touched:

- `src-tauri/src/db/migrations.rs` (V19 + 10 V19 tests)
- `src-tauri/src/db/repositories/products.rs` (lifecycle helpers + active-only filter rewrite + audit read helper + new tests)
- `src-tauri/src/services/products.rs` (`unarchive_product`, `retire_product`, `list_lifecycle_events`, transactional `archive_product` refactor, `add_barcode` retired guard, new tests)
- `src-tauri/src/services/scanner.rs` (no code; one test that asserts retired SKU/barcode → `Unknown`)
- `src-tauri/src/services/csv_io.rs` (new `ReleasedSku` / `ReleasedBarcode` classify branches + new counter fields + tests)
- `src-tauri/src/services/backup_restore.rs` (`REQUIRED_TABLES` + tests)
- `src-tauri/src/services/user_messages.rs` (3 new variants + translations + tests)
- `src-tauri/src/dto/products.rs` (new types)
- `src-tauri/src/dto/csv_io.rs` (new enum variants)
- `src-tauri/src/commands/products.rs` (3 new IPC handlers)
- `src-tauri/src/lib.rs` (`invoke_handler` registration)

Forecast: ~360 net production code + ~60 net test code = ~420 net. **At the upper edge of the 400-line budget.** If PR 1's net diff exceeds 400 during implementation, split into PR 1a (schema + repository helpers + service-layer lifecycle methods + audit) and PR 1b (DTOs, IPC wiring, CSV preview, backup/restore, message variants). The split boundary is mechanical: any file that does not import from `dto::products::ProductLifecycle` ships in 1a.

### PR 2 — Catalog / detail / dashboard UI + i18n parity (~340–420 net)

Files touched:

- `src/components/ProductCatalogPage.svelte` (`showRetired` toggle, `Retired` badge, bottom-sort)
- `src/components/ProductDetailPage.svelte` (`unarchive` + `retire` flows + lifecycle history section + conditional banner)
- `src/components/ProductForm.svelte` (read-only `lifecycle` field)
- `src/components/DashboardPage.svelte` (badge rendering + retire-aware quick-create)
- `src/components/CsvImportPage.svelte` (per-row badges, counter display, commit admission)
- `src/lib/products.ts` (typed wrappers)
- `src/i18n/en/index.ts` + `src/i18n/es/index.ts` (full key parity per §7.7)

Forecast: ~300 net UI + ~100 net i18n + ~20 net lib wrappers = ~420 net. **At the upper edge of the 400-line budget.** Likely-split point: `CsvImportPage.svelte` is the largest single file; if its net exceeds 200 lines, escalate via `deliveryStrategy: ask-on-risk` and split into PR 2a (catalog / detail / dashboard / form / `lib/products.ts` / i18n keys) and PR 2b (CSV import page only — wire to the backend contract shipped in PR 1).

### PR 3 — Verification, review fixes, and verify-report (~80–150 net)

Files touched:

- `openspec/changes/product-lifecycle-reusable-identifiers/verify-report.md` (manual smoke records per §8.3)
- Test fixes discovered during PR 1 / PR 2 review
- i18n parity fixes discovered during `svelte-check`
- Possibly a small frontend follow-up if PR 2 surfaces a UX gap

Forecast: ~120 net. Comfortably under 400.

### Chain strategy

Recommended: **stacked-to-main** — PR 1 lands to `main`; PR 2 stacks onto PR 1's branch and lands after PR 1 is merged; PR 3 stacks onto PR 2's branch and lands after PR 2 is merged. The chain strategy matches the precedent in `scanner-quick-operations` (same `stacked-to-main` shape) and keeps each PR individually reviewable at ~400 net lines.

The parent must **confirm the chain strategy** before `tasks.md` is generated. The proposal's "Chain strategy: deferred" entry resolves to `stacked-to-main` once the parent approves this design. If the parent prefers `feature-branch-chain` (one feature branch holding all three PRs), `tasks.md` reflects that instead.

## 11. Rollout

1. Land PR 1 (backend) to `main`. **No user-visible change yet** — every frontend call into the new IPCs is wired in PR 2.
2. Land PR 2 (UI + i18n) on top of PR 1.
3. Land PR 3 (verify-report + review fixes) on top of PR 2.
4. The legacy `is_active` column stays on disk through the entire rollout and is dropped by a separate, later change (§5.4).
5. The `product_lifecycle_events` table is invisible to operators who never retire a product; it accumulates audit rows only as transitions happen.

## 12. Chain strategy decision (request to parent)

This design proposes a 3-PR chain at the upper edge of the 400-line budget per slice. The parent must confirm the chain strategy before `tasks.md` is generated:

- **Recommended**: `stacked-to-main` with the slice boundaries in §10 (matches `scanner-quick-operations` precedent).
- **Alternative**: `feature-branch-chain` (one feature branch with three PRs off it; matches if the project later needs a single merge event).
- **Alternative**: `single-pr` is **not viable** — the change is ~900+ net lines and the budget is 400.
- **`size:exception`** is **not recommended** — splitting into three reviewable slices is the lower-risk path and matches the existing project precedent.

If the parent does not confirm the chain strategy, `tasks.md` MUST be written with `ask-on-risk` flagged for the chain boundary and the `deliveryStrategy: ask-on-risk` pause reused. The proposal's `auto-chain` delivery strategy resolves the chain question before `tasks.md` ships.

## 13. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Partial UNIQUE index predicate mis-formed | High | Both partial indexes use single-table predicates (`products(sku) WHERE lifecycle != 'retired'` and `product_barcodes(barcode) WHERE lifecycle != 'retired'`). The mirror invariant in §2.1 makes the `product_barcodes` predicate correct without a cross-table `EXISTS`. V19 tests cover SKU and barcode uniqueness after retire; manual smoke covers the new-product-creates-with-released-identifier scenarios |
| SQLite `DROP INDEX sqlite_autoindex_*` cannot drop autoindexes that back table-level `UNIQUE` constraints | High (resolved) | The V19 rebuild pattern (CREATE …_v19 / INSERT / DROP / RENAME) recreates `products` and `product_barcodes` without inline `UNIQUE` clauses, dropping the autoindexes implicitly. This mirrors V17's `lot_movements` rebuild. |
| `products` rebuild breaks FK references from `expiry_lots`, `product_categories`, `product_barcodes`, `product_lifecycle_events` | High (mitigated) | `PRAGMA defer_foreign_keys = ON` at the top of the V19 transaction defers FK checks to COMMIT. The rebuild sequence preserves the table name and every copied row, so FK references resolve to the new `products` table at COMMIT time. V19 includes regression tests `v19_preserves_expiry_lots_fk_after_products_rebuild` and `v19_preserves_product_categories_fk_after_products_rebuild`. |
| `product_barcodes.lifecycle` drifts out of sync with `products.lifecycle` | High | The §2.1 invariant is enforced inside `apply_lifecycle_transition`, which performs `set_product_lifecycle` and `sync_product_barcodes_lifecycle` in the same database transaction. Tests `archive_mirrors_lifecycle_to_owned_barcodes`, `unarchive_mirrors_active_to_owned_barcodes`, `retire_mirrors_retired_to_owned_barcodes_and_releases_value_for_reuse` cover the three transitions; `lifecycle_event_atomicity` covers the failure-isolation contract. |
| Migration checksum drift if V19 is edited later | High | V19 lives at a fresh index; comment in `migrations.rs` mirrors the V15 precedent calling out that the SQL bytes are part of the on-disk identity |
| `is_active` / `lifecycle` projection drift | Medium | `set_lifecycle_and_legacy_is_active` keeps both columns in sync; dual-read window tests cover legacy filters |
| CSV import preview overcounts `valid_rows` | Medium | The summary counter shifts `ReleasedSku` from `invalid_rows` to `valid_rows`; explicit test |
| Retire IPC called from a path that does not supply a reason | Medium | The `RetireReasonRequired` validation is enforced inside the transactional helper, before any DB write |
| PR 1 / PR 2 individually exceed the 400-line budget | Medium | Pre-split escape hatches are defined in §10; the orchestrator triggers `deliveryStrategy: ask-on-risk` mid-implementation |
| `product_lifecycle_events` not in `REQUIRED_TABLES` | Low | Listed in §6.7 and added to the backup/restore service in PR 1 |
| Foreign desktop environment (tray fallback precedent) does not apply here | n/a | No tray or window-lifecycle changes in this change; tray work is in `scanner-quick-operations` |
| Frontend binary deployed against an older backend (or vice versa) during rolling upgrade | Low | `lifecycle` is an `Option` on the wire; the missing-field fallback in §5.3 keeps both binaries working |
| `Retired product` lot-scan badge is missing on the lot detail modal | Medium | Manual smoke test in §8.3 explicitly covers the scenario; the change ships the badge rendering in `LotDetailPage.svelte` as part of PR 2 |
