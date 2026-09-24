# Lot location name projection

Branch: `fix/product-detail-lot-location-name`

## Goal

Show each lot's human-readable store-location name in product-detail and Scanner location surfaces, while retaining UUIDs only as internal relationship keys and preserving the sentinel `No location` behavior.

## Decisions

- Project `store_locations.name` as nullable `location_name` in lot responses with a `LEFT JOIN`.
- Preserve `location_id` for all writes and selection values.
- Include inactive locations in the projection so historic lots remain interpretable.
- Do not add a database migration or i18n keys.

## Tasks

- [x] Extend lot DTO/query projection with nullable `location_name` and backend regression coverage.
- [x] Consume `location_name` in Product Detail and Scanner while preserving sentinel fallback behavior.
- [x] Run focused and frontend checks; commit the verified work unit.

## Acceptance criteria

- A lot assigned to a real location renders that location's name, never its UUID.
- A lot with a sentinel or no location still renders localized `No location` where appropriate.
- Scanner Sale/Stock-out location labels render real names while retaining the original `location_id` option value.
- No schema migration is introduced.

## Evidence

- `src-tauri/src/dto/expiry_lots.rs` and `src-tauri/src/db/repositories/expiry_lots.rs` project nullable `location_name` through a `LEFT JOIN store_locations` in all lot read queries; four focused regressions cover real, null, inactive, and mixed-list locations.
- `src/lib/expiry_lots.ts`, `src/lib/lotDisplay.ts`, `src/components/ProductDetailPage.svelte`, and `src/components/ScannerPage.svelte` render human-readable names while preserving sentinel/null labels and raw `location_id` action values.
- Verification passed: `cd src-tauri && cargo test --lib` (751 passed), `npm run check` (0 errors/warnings), and `git diff --check`.
- Commit: recorded with this work unit.
