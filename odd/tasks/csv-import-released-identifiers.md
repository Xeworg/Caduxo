# CSV import released identifiers

Branch: `feat/product-lifecycle-reusable-identifiers`

## Goal

Make CSV import commit behavior match preview behavior for released/retired SKU and barcode values.

## Findings

Read-only exploration surfaced two backend contract gaps against the lifecycle-aware preview path:

1. **`csv_io::import_row` did not filter retired products** from the SKU/barcode conflict checks before applying the conflict strategy. A retired-only match fell through to `Skipped` (Skip strategy), `Updated` (Update strategy), or `Skipped` with `sku_conflict_manual` (Review strategy) instead of creating the new product. Active/archived duplicates were correctly distinguished by `lifecycle`, but retired-only matches were treated as if they were active duplicates.
2. **`db::repositories::products::find_by_barcode_exact` does not project the `lifecycle` column** on its joined SELECT (the helper is shared with scanner / search read paths that only need the metadata). `ProductSearchResult.lifecycle` is therefore always `None` for barcode matches, which silently broke the three-state barcode classifier in `csv_io::classify_row` — `ReleasedBarcode` was unreachable from the preview path even before any commit path bug existed. The SKU side worked because `find_by_sku_exact` already projects `lifecycle`. The same gap would have hit the commit path even after fix #1 was in place, so the helper was the necessary second-layer fix.

The original PR 1b implementation referenced test names (`classify_row_released_sku`, `classify_row_released_barcode`, `classify_row_active_duplicate_sku_wins_over_released`, `preview_summary_counts_released_rows_as_valid`) in `tasks.md` Slice 6 but the corresponding tests never landed in `csv_io::tests`, and the verify-report's spec-scenario coverage matrix inherited those references without verifying them. The PR 3 follow-up closes that gap.

## Tasks

- [x] Add regression tests for released SKU/barcode preview and import commit behavior (10 tests in `csv_io::tests`).
- [x] Fix import commit classification to ignore retired product identifier collisions (`csv_io::import_row` SKU/barcode filters + `lookup_barcode_owner_lifecycle` helper for the barcode gap).
- [x] Update SDD evidence (`verify-report.md` M13/M14 rows + spec-scenario coverage matrix + PR boundary review letter f + G2/G8 gate counts; `tasks.md` M13/M14 entries; `apply-progress.md` PR 3 follow-up section).
- [x] Run checks (all green; no commits, push, or archive per ODD scope).

## Implementation

### Helper — `csv_io::lookup_barcode_owner_lifecycle`

```rust
async fn lookup_barcode_owner_lifecycle(
    pool: &DbPool,
    barcode: &str,
) -> Result<Option<ProductLifecycle>, AppError>
```

Focused SQL lookup against the same `products INNER JOIN product_barcodes ON product_id` join that `find_by_barcode_exact` uses, but pulls only the `lifecycle` column. Lets the three-state classifier distinguish `ReleasedBarcode` from `DuplicateBarcode` without changing the shared `find_by_barcode_exact` contract (which is shared with scanner / search read paths). Lives inside `csv_io.rs` per the ODD task's allowed edit surfaces.

### Preview (`classify_row`)

- SKU branch: unchanged — `find_by_sku_exact_including_retired` returns `lifecycle`; the existing `if existing.lifecycle == Some(ProductLifecycle::Retired)` check emits `ReleasedSku`, otherwise `DuplicateSku`.
- Barcode branch: now calls `lookup_barcode_owner_lifecycle` after the metadata-bearing `find_by_barcode_exact_including_retired` lookup; the helper drives the retired-vs-active/archived split while the metadata still comes from `find_by_barcode_exact` so `existing_product_id` / `existing_barcode` keep pointing at the (possibly retired) row.

### Commit (`import_row`)

- SKU conflict check: `find_by_sku_exact` results are filtered with `.filter(|row| row.lifecycle != Some(ProductLifecycle::Retired))`. A `None` lifecycle (pre-V19 backend during rolling deploy) defaults to active so the legacy duplicate semantics are preserved.
- Barcode conflict check: `lookup_barcode_owner_lifecycle` decides whether the match is retired; only an active or archived match returns `Skipped` per the strategy-specific `reason_code`. Retired-only matches fall through to the create path under every strategy.

The partial unique index `uq_product_barcodes_barcode_active WHERE lifecycle != 'retired'` already excludes the retired barcode row from uniqueness enforcement, so the new attach succeeds when the helper releases it.

## Regression tests (10 new)

| Test | Pins |
|------|------|
| `preview_emits_released_sku_for_retired_only_match` | `ReleasedSku` preview + `released_sku_count` increments + counts as `valid_rows` |
| `preview_emits_released_barcode_for_retired_only_match` | `ReleasedBarcode` preview + `released_barcode_count` increments |
| `preview_active_duplicate_sku_still_emits_duplicate_sku` | regression guard: active duplicate still emits `DuplicateSku`, not `ReleasedSku` |
| `preview_active_duplicate_barcode_still_emits_duplicate_barcode` | regression guard: active duplicate still emits `DuplicateBarcode` |
| `preview_summary_partitions_released_and_active_duplicates` | mixed CSV: `valid_rows`, `released_*_count`, `duplicate_*_count`, `invalid_rows` partition correctly |
| `import_skip_creates_when_sku_matches_only_retired` | Skip strategy commits `Created` for retired SKU |
| `import_update_creates_when_sku_matches_only_retired` | Update strategy commits `Created` for retired SKU (no mutation of retired row) |
| `import_review_creates_when_sku_matches_only_retired` | Review strategy commits `Created` for retired SKU (no manual-resolution `Skipped`) |
| `import_skip_creates_and_attaches_when_barcode_matches_only_retired` | Skip strategy attaches released barcode to the new product |
| `import_skip_still_skips_active_duplicate_sku` | regression guard: active duplicate keeps the `sku_already_exists` skip semantics |

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml --lib csv_io` → **39 passed; 0 failed; 0 ignored** (was 29 before PR 3 follow-up; +10 new regression tests).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` → **746 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out** (was 734 before PR 3 follow-up).
- `npx svelte-check` — not run; no frontend artifacts changed in this task.

## Evidence

- `openspec/changes/product-lifecycle-reusable-identifiers/verify-report.md` — M13/M14 rows now reference the actual regression tests; spec-scenario coverage matrix adds three new rows for the released SKU / barcode commit contracts and the preview summary partition; PR boundary review letter f documents the `lookup_barcode_owner_lifecycle` helper rationale; G2 / G8 gate counts updated to `746`.
- `openspec/changes/product-lifecycle-reusable-identifiers/tasks.md` — M13 / M14 entries now cite the regression tests as evidence for the backend portion.
- `openspec/changes/product-lifecycle-reusable-identifiers/apply-progress.md` — new `## PR 3 follow-up — CSV import released identifiers (post-verify)` section documents the trigger, the two-layer fix, the regression coverage, and the validation results.

## Risks / follow-ups

- The 16 manual smoke steps (M1–M16) still require a Tauri desktop runtime to confirm visually. M13/M14 specifically are now backed by automated regression tests for the backend classification + commit paths; the UI render is verified through the PR 2 implementation per `apply-progress.md` PR 2 § Slice 13.
- `find_by_barcode_exact` not projecting `lifecycle` is a latent contract gap that other downstream consumers may also rely on. Consider extending the helper to project `lifecycle` in a future maintenance PR (outside this ODD task's allowed edit surfaces — file an issue / new ODD task to track).
- The clippy advisory about `import_row`'s parameter count (9 parameters) is pre-existing and not introduced by this task; the function already had 9 arguments before the PR 3 follow-up.

## Status

Completed within the ODD `csv-import-released-identifiers` scope. No commit, push, or archive performed per the task scope.