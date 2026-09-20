# Backend validation catalog

## Goal
Extend backend validation localization coverage for products and stores while keeping the slice backend-only and IPC-compatible.

## Scope
- Add catalog entries for shared domain validation messages: SKU, description, barcode, name, scan value, and settings language validation.
- Keep canonical service/domain validation output in English so `localize_validation` can translate at command boundaries.
- Wire locale-aware validation localization through product and store command paths using optional locale input where needed to preserve compatibility with existing frontend calls.

## Non-goals
- Do not localize `BusinessRule`, `NotFound`, `DuplicateField`, or infrastructure errors in this slice.
- Do not change frontend wrappers/call sites in this slice.
- Do not change `lot_movements`, `expiry_lots` beyond already-catalogued create validation, `unit_definitions`, `notifications`, CSV, or reports.
- Do not redesign `CommandError` or IPC error shape.
- Do not add a third locale.

## Tasks
- [x] Task 1: Extend `UserMessage` catalog/parser/tests for product/store validation strings.
- [x] Task 2: Align product/domain/store validation emitters with canonical English messages.
- [x] Task 3: Wire optional locale localization through product/store command boundaries.
- [x] Task 4: Run focused backend checks and independent verification.
- [x] Task 5: Commit the verified slice after explicit user request.

## Evidence

### Task 1
- Added 11 `UserMessage` variants in `services/user_messages.rs`: `SkuEmpty`,
  `SkuTooLong { max: usize }`, `DescriptionEmpty`,
  `DescriptionTooLong { max: usize }`, `BarcodeEmpty`,
  `BarcodeTooLong { max: usize }`, `BarcodeInvalidChars`, `NameEmpty`,
  `NameTooLong { max: usize }`, `ScanValueEmpty`, and
  `LanguageNotAllowed { value: String }`.
- Each variant has English and Spanish arms in `user_message` plus an inverse
  branch in `parse_user_message_kind`. English canonicals preserve the
  previous string shape (`SKU cannot be empty`, `SKU exceeds maximum length
  of 64 characters`, `Scan value cannot be empty`, etc.) so the existing
  service/domain assertions remain meaningful.
- Parser helpers `parse_too_long_suffix` and `parse_language_not_allowed`
  extract the captured `max: usize` and `value: String` payload and guard
  against prefix collisions between `SKU` / `Description` / `Barcode` /
  `Name` TooLong variants.
- Module doc table updated with the 11 new catalog rows.

### Task 2
- `domain/validation.rs`: `validate_sku`, `validate_description`,
  `validate_barcode`, and `validate_name` now emit canonical English via
  `user_message(UserMessage::*, Locale::En)`. Magic numbers (`64`, `128`,
  `512`) replaced with named `*_MAX_LEN` consts that feed both the runtime
  cap and the catalog payload, so a future cap change stays in one place.
  Signatures stay `Result<(), String>`.
- `services/products.rs::find_product_by_scan`: empty scan value now emits
  `user_message(UserMessage::ScanValueEmpty, Locale::En)` via the canonical
  English path.
- Service-level validation strings still flow through `parse_user_message_kind`
  at the command boundary, so existing `DomainError::Validation { .. }`
  pattern tests stay opaque to the message shape.

### Task 3
- `commands/products.rs`: added optional `locale: Option<String>` to
  `create_category`, `update_category`, `create_product`, `update_product`,
  `find_product_by_scan`, and `add_product_barcode`. Each command resolves
  the optional tag via a small `resolve_locale` helper
  (`Locale::parse` → `Locale::En` fallback) and wraps the service error with
  `localize_validation(e, loc)` so `DomainError::Validation { .. }` reaches
  the UI in the requested locale. Other product commands
  (`list_categories`, `list_categories_search`, `archive_product`,
  `get_product`, `search_products`, `suggested_product_alert_days`,
  `list_product_barcodes`, `remove_product_barcode`) stay untouched because
  they only surface `NotFound` / `DuplicateField` / `BusinessRule` errors.
- `commands/stores.rs`: added optional `locale: Option<String>` to
  `create_store`, `update_store`, `create_store_location`,
  `update_store_location`, and `update_settings`. The store / location
  commands wire `localize_validation` exactly like products.
- `update_settings` previously hardcoded the English
  `language must be one of {en, es}, got \`{value}\`` rejection. It now
  constructs the localized message directly via
  `user_message(UserMessage::LanguageNotAllowed { value }, loc)` because the
  rejection is inlined at the command boundary (no service round-trip). This
  keeps the canonical English shape used by the parser while honouring the
  requested locale.
- Frontend compatibility: every new `locale` argument is `Option<String>`,
  so existing frontend calls that don't pass it still work — Tauri surfaces
  `None` and `resolve_locale` falls back to English.

### Task 4
- `cargo build --manifest-path src-tauri/Cargo.toml --lib` → finished cleanly.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` →
  `515 passed; 0 failed; 0 ignored`.
- Focused suites (all green):
  - `services::user_messages::` → `60 passed` (was 21, +39 new tests:
    en/es message shape for each new variant, parser roundtrip for each
    variant, prefix-collision guards for `TooLong` and
    `LanguageNotAllowed`, plus two `localize_validation` integration tests
    for `SkuTooLong` and `LanguageNotAllowed`).
  - `domain::validation::` → `4 passed` (opaque is_ok/is_err assertions).
  - `services::products::` → `39 passed` (existing `Validation { .. }`
    pattern matches still hold).
  - `services::stores::` → `12 passed` (untouched but exercised by the
    full run).
  - `services::settings::` → `3 passed` (untouched but exercised by the
    full run).
- `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --no-deps`:
  pre-existing warnings only, in files outside the allowed edit surfaces
  (`db/repositories/products.rs`, `services/csv_io.rs`,
  `services/reports.rs`). No new warnings introduced by the slice.

### Task 5
- User explicitly requested the commit after the verified slice.
- Commit: `55d4e8d feat: localize product and store validation`.
