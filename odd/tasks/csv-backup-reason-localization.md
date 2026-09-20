# CSV and backup reason localization

## Goal

Localize CSV row-level reasons and backup validation checks so user-facing DTO text is not hardcoded English.

## Scope

- Add stable reason/check codes to CSV preview/import row DTOs while keeping existing reason strings as fallback.
- Add typed backup validation checks with stable codes and fallback English messages.
- Emit codes from CSV and backup services.
- Resolve reason/check codes in frontend with EN/ES i18n dictionary keys and fallback to existing strings.
- Add/update focused backend tests and run frontend checks.

## Non-goals

- Do not change database schema or migrations.
- Do not rewrite generic `NotFound`, `DuplicateField`, or infra-error localization in this slice.
- Do not change CSV import conflict resolution UX beyond displayed reason localization.
- Do not touch `.codegraph/`.
- Do not push.

## Tasks

- [x] Task 1: Extend DTOs and backend services to emit stable CSV/backup reason codes.
- [x] Task 2: Localize frontend rendering for CSV row reasons and backup validation checks.
- [x] Task 3: Run focused backend/frontend checks and fix regressions within scope.
- [x] Task 4: Commit the verified slice.

## Evidence

### Task 1 — Backend DTOs + service emission

- `src-tauri/src/dto/csv_io.rs`:
  - Added optional `reason_code: Option<String>` to
    `CsvPreviewRowStatus::Invalid`, `CsvImportRowOutcome::Skipped`, and
    `CsvImportRowOutcome::Invalid`. Existing `reason: String` fields are
    retained as the English fallback so IPC stays backwards compatible.
- `src-tauri/src/dto/backup_restore.rs`:
  - Added `check_codes: Vec<Option<String>>` to `RestoreValidation`, parallel
    to the existing `checks: Vec<String>`. Older frontends keep reading
    `checks`; the new field is additive.
- `src-tauri/src/services/csv_io.rs`:
  - Declared `REASON_CODE_*` constants at module top (must mirror the
    frontend dispatch table in `CsvImportPage.svelte`).
  - `classify_row` (preview) emits codes on every `CsvPreviewRowStatus::Invalid`
    path: `generic_validation` for `validate_sku` / `validate_description` /
    `validate_barcode` domain errors, `alert_days_negative`,
    `alert_days_too_large`, and `db_error` for the SKU/barcode uniqueness
    database-error branches.
  - `import_row` (commit) emits codes on every `CsvImportRowOutcome::Skipped`
    and `CsvImportRowOutcome::Invalid` path:
    `required_field_sku` / `required_field_description`,
    `generic_validation` (domain rules), `alert_days_range_invalid` for
    out-of-range alert days, `sku_already_exists` (skip strategy),
    `sku_conflict_manual` (review strategy),
    `barcode_belongs_to_other_product` (skip/update), and
    `barcode_conflict_manual` (review).
- `src-tauri/src/services/backup_restore.rs`:
  - Declared `CHECK_CODE_*` constants and a private `push_check` helper that
    keeps `checks` and `check_codes` parallel.
  - `validate_backup` now threads both arrays together for every emission
    path (file not found early return, SQLite header, schema/integrity
    block, and version compatibility).

### Task 2 — Frontend localization

- `src/lib/csv.ts`:
  - Added optional `reason_code?: string | null` to the `invalid` variant of
    `CsvPreviewRowStatus` and to the `skipped` / `invalid` variants of
    `CsvImportRowOutcome`. Older IPC payloads without the field fall back
    to the `reason` string.
- `src/lib/backup_restore.ts`:
  - Added `checkCodes: (string | null)[]` to `RestoreValidation`, parallel
    to the existing `checks: string[]`.
- `src/components/CsvImportPage.svelte`:
  - Added `REASON_CODES` table mirroring the Rust constants and a
    `localizeReason(code, fallback, ctx)` dispatcher that maps each code to
    the matching `$LL.csvImport.reasonCodes.*` key and falls back to the
    raw `reason` string when the code is missing or unknown.
  - `outcomeReason(row)` now passes the row (so `sku` / `barcode` are
    available for parameterized keys) and calls `localizeReason` for
    `skipped` and `invalid` outcomes.
  - `rowDetailMessage(row)` takes the full `CsvPreviewRow` so `sku` /
    `barcode` can populate `skuAlreadyExists({sku})` and
    `barcodeBelongsToOtherProduct({barcode})` for `invalid` rows.
- `src/components/BackupRestorePage.svelte`:
  - Added `CHECK_CODES` table mirroring the Rust constants, a
    `REQUIRED_TABLES_COUNT` constant synced with `REQUIRED_TABLES.len()`,
    and a `localizeCheck(code, fallback)` dispatcher. Schema version
    placeholders are extracted from the fallback text via regex so the
    localized `schemaVersionDetected({v})` /
    `schemaVersionInvalid({v})` keys carry the same `v` the backend
    reports.
  - The two `{#each validation.checks as check}` loops now render through
    `localizeCheck(validation.checkCodes[i], check)`.
- `src/i18n/en/index.ts` and `src/i18n/es/index.ts`:
  - Added `csvImport.reasonCodes` (11 keys: `requiredFieldSku`,
    `requiredFieldDescription`, `skuAlreadyExists`, `skuConflictManual`,
    `barcodeBelongsToOtherProduct`, `barcodeConflictManual`,
    `alertDaysNegative`, `alertDaysTooLarge`, `alertDaysRangeInvalid`,
    `dbError`, `genericValidation`).
  - Added `backupRestore.checks` (11 keys: `fileNotFound`,
    `sqliteHeaderValid`, `sqliteHeaderInvalid`, `requiredTablesPresent`,
    `requiredTablesMissing`, `schemaVersionDetected`, `integrityCheckOk`,
    `integrityCheckFailed`, `schemaCheckFailed`, `schemaVersionInvalid`,
    `schemaVersionCompatible`).
- `src/i18n/i18n-types.ts`:
  - Regenerated by `npm run i18n:generate`. The new `csvImport.reasonCodes`
    and `backupRestore.checks` namespaces are present in both `Translations`
    (line 2606 / 2954) and `TranslationFunctions` (line 5931 / 6269) blocks.

### Task 3 — Verification
- Writer observed:
  - `cargo test --lib` from `src-tauri` — 635 passed, 0 failed.
  - `cargo test --lib services::csv_io` — 29 passed, 0 failed.
  - `cargo test --lib services::backup_restore` — 14 passed, 0 failed.
  - `cargo test --lib services::backup_restore::tests::validate_backup` — 4 passed, 0 failed.
  - `cargo check --lib` from `src-tauri` — clean.
  - `cargo clippy --lib` from `src-tauri` — no new lints; four pre-existing warnings unrelated to this slice.
  - `npm run i18n:generate` — idempotent.
  - `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — 0 errors, 0 warnings.
- Independent verifier PASS:
  - Confirmed all 11 backend CSV reason codes match frontend dispatch values and EN/ES dictionary keys.
  - Confirmed all 11 backup check codes match frontend dispatch values and EN/ES dictionary keys.
  - Confirmed backup `checks` / `check_codes` alignment on all return paths, including file-not-found early return.
  - Confirmed missing/unknown codes fall back to raw `reason` / `check` strings for older payloads.
  - Re-ran `cd src-tauri && cargo test --lib services::csv_io` — 29 passed, 0 failed.
  - Re-ran `cd src-tauri && cargo test --lib services::backup_restore` — 14 passed, 0 failed.
  - Re-ran `npm run i18n:generate` — idempotent.
  - Re-ran `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — 0 errors, 0 warnings.

### Task 4
- Commit: `f5ecadd feat: localize csv and backup reasons`.

