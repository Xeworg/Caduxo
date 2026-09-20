# Backend simple BusinessRule localization

## Goal
Localize the backend BusinessRule messages that are constant, user-visible, and safe to route through the existing IPC error shape.

## Scope
- Add `localize_business_rule` beside `localize_validation` for `DomainError::BusinessRule` only.
- Add catalog entries for simple constant BusinessRule messages:
  - archived product barcode rejection
  - inactive store location rejection
  - restore confirmation rejection
  - unit referenced archive rejection as catalog-ready service-only coverage
- Keep service/domain messages canonical English and localize only at command boundaries.
- Wire affected commands without changing IPC schema.

## Non-goals
- Do not localize expiry lot BusinessRule messages with dynamic status/quantity/unit in this slice.
- Do not change `lot_movements` command error shape or movement BusinessRule messages.
- Do not localize CSV DTO row reasons, backup validation reasons, NotFound, DuplicateField, or internal errors.
- Do not add frontend changes or a third locale.

## Tasks
- [x] Task 1: Extend `UserMessage` catalog/parser/tests and add `localize_business_rule`.
- [x] Task 2: Emit canonical catalog strings in simple BusinessRule services.
- [x] Task 3: Wire command-boundary BusinessRule localization for product/store/backup commands.
- [x] Task 4: Run focused backend checks and verification.
- [x] Task 5: Commit the verified slice after explicit user request.

## Evidence

### Task 1
- Added four `UserMessage` variants to `services/user_messages.rs`:
  - `ProductArchivedForBarcode` (EN: `Cannot add barcode to an archived product`, ES: `No se puede agregar un código de barras a un producto archivado`)
  - `InactiveStoreLocation` (EN: `Cannot add location to an inactive store`, ES: `No se puede agregar una ubicación a una tienda inactiva`)
  - `RestoreRequiresConfirmation` (EN: `Restore requires explicit user confirmation.`, ES: `La restauración requiere confirmación explícita del usuario.`)
  - `UnitStillReferenced` (EN: `Unit is still referenced by product(s) and cannot be archived`, ES: `La unidad todavía está referenciada por producto(s) y no se puede archivar`)
- Module docstring catalog table extended with the same four rows.
- `parse_user_message_kind` extended with exact-string checks for all four EN strings; the parser remains disambiguated from Validation variants because each new string is unique.
- Added `localize_business_rule(err: AppError, locale: Locale) -> AppError` mirroring `localize_validation` but only destructuring `DomainError::BusinessRule`. Non-BusinessRule errors pass through untouched.
- New focused tests appended under `services/user_messages::tests`:
  - EN/ES shape: `product_archived_for_barcode_{en,es}`, `inactive_store_location_{en,es}`, `restore_requires_confirmation_{en,es}`, `unit_still_referenced_{en,es}`.
  - Parser round-trips: `parse_{product_archived_for_barcode,inactive_store_location,restore_requires_confirmation,unit_still_referenced}_roundtrips`.
  - Helper integration: `localize_business_rule_translates_known_message`, `localize_business_rule_translates_restore_confirmation`, `localize_business_rule_preserves_unknown_business_rule_message` (dynamic strings stay English), `localize_business_rule_ignores_non_business_rule_error` (NotFound unchanged), `localize_business_rule_ignores_infrastructure_error` (NotFound + Infrastructure paths).
  - Companion guard: `localize_validation_still_ignores_non_validation_error` confirms `localize_validation` continues to ignore BusinessRule (only `localize_business_rule` translates it).

### Task 2
- `services/products.rs::add_barcode`: replaced raw string with `user_message(UserMessage::ProductArchivedForBarcode, Locale::En)`.
- `services/stores.rs::create_location`: added imports `Locale` and `user_message`/`UserMessage`; replaced raw string with `user_message(UserMessage::InactiveStoreLocation, Locale::En)`.
- `services/backup_restore.rs::restore_backup`: added imports `Locale` and `user_message`/`UserMessage`; replaced raw string with `user_message(UserMessage::RestoreRequiresConfirmation, Locale::En)`.
- `services/unit_definitions.rs::archive_unit`: added imports `Locale` and `user_message`/`UserMessage`; replaced raw string with `user_message(UserMessage::UnitStillReferenced, Locale::En)`. Note: no command currently calls `archive_unit`; coverage is catalog-ready service-only as planned.
- All four canonical English strings remain byte-identical to the previous literals — confirmed via the existing service-level tests that assert `DomainError::BusinessRule { message }` shape.

### Task 3
- `commands/products.rs`: import updated to `{localize_business_rule, localize_validation}`; `add_product_barcode` now chains `.map_err(|e| localize_validation(e, loc))` → `.map_err(|e| localize_business_rule(e, loc))` → `.map_err(AppError::into)`. Doc comment updated to describe the dual localization.
- `commands/stores.rs`: import updated to `{localize_business_rule, localize_validation, user_message, UserMessage}`; `create_store_location` now chains both helpers identically. Doc comment updated.
- `commands/backup_restore.rs`: added imports `Locale` and `localize_business_rule`; added a local `resolve_locale(Option<String>) -> Locale` helper mirroring the one in `commands::products` / `commands::stores`; `restore_backup` command signature gained an optional `locale: Option<String>` argument; the error chain now runs `.map_err(|e| localize_business_rule(e, loc))` → `.map_err(AppError::into)`. The optional argument is fully backwards compatible — callers that omit it keep their existing behavior, and `Locale::En` is the fallback. Doc comment updated to call out the optional locale, the fallback, and the backwards-compatibility guarantee.

### Task 4
- Writer checks:
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib services::user_messages::` → **78 passed; 0 failed** (includes 14 new tests covering EN/ES shape, parser round-trips, helper integration, and the validation-pass-through guard).
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib services::products::` → **39 passed; 0 failed** (existing `add_barcode_to_archived_product_is_rejected` still asserts the `DomainError::BusinessRule` shape).
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib services::stores::` → **12 passed; 0 failed** (existing `create_location_rejects_inactive_store` still asserts the rejection).
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib services::backup_restore::` → **14 passed; 0 failed** (existing `restore_requires_confirmation` still asserts the `DomainError::BusinessRule` shape and matches the new canonical EN string).
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib services::unit_definitions::` → **11 passed; 0 failed** (existing `archive_unit_referenced_returns_business_rule_error` still asserts the rejection).
  - `cargo build --manifest-path src-tauri/Cargo.toml --lib` → **Finished `dev` profile [unoptimized + debuginfo]** (clean).
- Independent verifier result: **PASS**.
  - Re-ran the same focused service suites successfully.
  - `cargo test --manifest-path src-tauri/Cargo.toml --lib --quiet` → **533 passed; 0 failed**.
  - `cargo build --manifest-path src-tauri/Cargo.toml --lib` → clean.
  - Confirmed scope boundaries: no diffs in `expiry_lots`, `lot_movements`, CSV DTO/service/commands, NotFound, DuplicateField, or internal-error localization.

### Task 5
- User explicitly requested the commit after the verified slice.
- `git diff --check` was clean before staging.
- Commit: `ccd3e07 feat: localize simple business rule messages`.
