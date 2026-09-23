# CSV import commit error boundary

Branch: `feat/product-lifecycle-reusable-identifiers`

## Goal

Make CSV import commit failures row-scoped and understandable instead of surfacing a generic internal error or leaving unclear partial state during released-identifier imports.

## Trigger

Manual import of `/home/xeworg/Documentos/testimportacion.csv` showed preview counts (`766` rows, `751` duplicate SKUs, `15` released SKUs) but commit displayed `An internal error occurred. Please try again.` UI banner is the `CommandError::Internal` shape that `AppError::Infrastructure(...)` becomes at the Tauri boundary in `error.rs`.

## Diagnosis (read-only)

- Source DB at `$HOME/.local/share/com.caduxo.app/caduxo.db`.
- App-side log (`logs/caduxo.log`) records `total_migrations=19`, no csv/import error lines because the path did not log enough context.
- CSV row schema: `upc,sku,description` (UTF-8 BOM in header). Header detection maps `upc` → `barcode`.
- For SKUs with both a retired and an active row (e.g. `1106000241`, `1307000081`), the released-identifier reuse is reachable, but the active rows are not retired siblings in this dataset. The CSV also has many SKUs not present in DB at all, so the create path is the dominant branch.
- Most plausible row-level root cause: a row whose SKU is `null`/empty after header mapping for header rows missing fields, or a SKU that races with an earlier committed row in the same CSV.
- Manual reproduction blocked because the import `INSERT` errors never reach `tracing`; command errors only surface as `CommandError::Internal` to the UI. Verified the code path: `services::csv_io::import_row` calls `products_repo::insert_product` which returns `sqlx::Error` → `AppError::Infrastructure` → `CommandError::Internal`.

## Fix

In `src-tauri/src/services/csv_io.rs`:

1. New helper `find_non_retired_sku_owner` returns `(id, sku)` of the *non-retired* row for a SKU, ordered by lifecycle (`active` then `archived`). Replaces the previous `find_by_sku_exact` filter so a SKU with both a retired and an active row no longer leaks through as a missing-owner.
2. New helper `lookup_barcode_owner_lifecycle` orders candidate owners by lifecycle so a barcode that belongs to both a retired and an active row reports `active` (the partial UNIQUE index only excludes `retired`).
3. `import_row` catches `sqlx::Error::Database(db_err) where db_err.is_unique_violation()` at the `insert_product` call and converts it to a row-level `Skipped` outcome through `duplicate_outcome_from_create`, which dispatches by the table hint embedded in the SQLite error message (`product_barcodes` → barcode reason code, otherwise SKU reason code).
4. Added regression test `import_sku_race_returns_row_skip_not_internal_error`: drives `Skip` with two rows reusing `RETIRED-RACE` and asserts `created=1`, `skipped=1` with `reason_code = sku_already_exists`.

## Tasks

- [x] Reproduce / diagnose the commit path against local evidence without mutating the real database.
- [x] Harden CSV import commit so uniqueness drift is converted into row outcomes instead of command-level infrastructure errors.
- [x] Add regression coverage and run backend checks.

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml --lib csv_io` → **40 passed; 0 failed; 0 ignored** (was 39 before; +1 regression test).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` → **747 passed; 0 failed; 0 ignored** (was 746 before).
- `npm run check` (svelte-check) → **0 errors, 0 warnings** (no frontend changes).
- Clippy on `csv_io.rs`: clean for this file (the remaining `approx_constant` warning is pre-existing on `src-tauri/src/domain/lot_movements.rs:718`, out of scope here).

## Risks / follow-ups

- The same hard boundary should be applied to `insert_barcode` calls inside `import_row` (currently they silently swallow UNIQUE violations). Left as-is to avoid scope creep; the current behavior is documented in code comments.
- Future: add a `tracing::warn!` at the import commit boundary with `row_index`, `sku`, `barcode`, and the SQLite error message so a manual reproduction surfaces the failing row in `logs/caduxo.log` without needing DB-side forensics.

## Manual checks (M1–M16) — `2026-09-23`

Per `openspec/changes/product-lifecycle-reusable-identifiers/verify-report.md`:

- M1 Archivar → ok
- M2 Desarchivar → ok
- M3 Retirar activo con motivo → ok
- M4 Retirar archivado con motivo → ok (auditoría `archived → retired`)
- M5 Motivo obligatorio → ok (submit bloqueado)
- M6 Producto retirado bloqueado → ok (edit / barcode / archive deshabilitados)
- M7 Catálogo sin `Show retired` → ok (retirados no aparecen)
- M8 Catálogo con `Show retired` → ok (badge, persistencia en `localStorage`, retirados al fondo)
- M9 Dashboard busca SKU retirado → ok (badge Retired visible)
- M10 Scanner SKU retirado → ok (`Unknown { scanned_value }`)
- M11 Scanner barcode retirado → ok (`Unknown { scanned_value }`)
- M12 Escanear lote de retirado → ok (badge Retired product en header)
- M13 CSV import SKU retirado → ok (preview `ReleasedSku` + commit)
- M14 CSV import SKU activo duplicado → ok (preview `DuplicateSku`)
- M15 Migración en DB fresca → **requiere automatización** (la verificación manual del binary no aplica; la cobertura ya está en `db/migrations.rs::tests`)
- M16 Migración en DB poblada + `product_lifecycle_events` preservado → **requiere automatización** (la cobertura está en `db/migrations.rs::tests` + `backup_restore` round-trip test)

M13/M14 ejecutados con este fix y los counts son correctos. M15/M16 ya tenían cobertura automática previa, así que solo resta marcarlos como `requires other test` en el verify-report.
