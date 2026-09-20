# Backend error boundary localization

## Goal

Localize remaining backend error-boundary messages for `NotFound`, `DuplicateField`, and infrastructure/internal errors so user-facing IPC errors respect the active locale.

## Scope

- Add user-message parsing/localization support for `NotFound`, `DuplicateField`, and `Internal` errors.
- Ensure `DuplicateField` IPC payloads include a localized `message` while preserving `field` and `value`.
- Wire localization helpers through command boundaries that already receive locale.
- Keep frontend code unchanged unless a type or compatibility issue requires a narrow fix.
- Add focused backend tests for parser/localizer behavior and command/error conversion.

## Non-goals

- Do not rename backend resource identifiers into friendly translated labels in this slice.
- Do not change database schema or migrations.
- Do not change CSV/backup reason DTO localization.
- Do not touch `.codegraph/`.
- Do not push.

## Tasks

- [x] Task 1: Add backend user-message support for NotFound, DuplicateField, and Internal errors.
- [x] Task 2: Wire command boundaries to localize these variants.
- [x] Task 3: Run focused backend/frontend checks and fix regressions within scope.
- [x] Task 4: Commit the verified slice.

## Evidence

### Task 1 — backend user-message support (NotFound / DuplicateField / Internal)

- `src-tauri/src/services/user_messages.rs`
  - Added three new `UserMessage` variants: `ResourceNotFound { resource, id }`,
    `DuplicateField { field, value }`, `InternalError`.
  - Added English + Spanish strings in `user_message()` for each new variant
    and updated the doc-comment table at the top of the file.
  - Added parser arms in `parse_user_message_kind` (`parse_resource_not_found`
    and `parse_duplicate_field` helpers + a constant-match for
    `InternalError`) so the canonical English strings round-trip and a future
    frontend i18n layer can identify the underlying kind.
  - Added three new boundary helpers in parallel with the existing
    `localize_validation` / `localize_business_rule`:
    - `localize_not_found(err, locale) -> AppError` — rewrites
      `DomainError::NotFound` into `DomainError::LocalizedNotFound { resource, id, message }`.
    - `localize_duplicate_field(err, locale) -> AppError` — rewrites
      `DomainError::DuplicateField` into `DomainError::LocalizedDuplicateField { field, value, message }`.
    - `localize_internal(err, locale) -> AppError` — rewrites
      `AppError::Infrastructure(_)` into `DomainError::LocalizedInternal { message }`.
  - Derived `PartialEq` on `UserMessage` so the round-trip tests can use
    `assert_eq!` directly.
  - Added 19 focused tests (round-trip + helper pass-through + malformed
    rejection + wire-collapse).

- `src-tauri/src/error.rs`
  - Added three new `DomainError` variants (`LocalizedNotFound`,
    `LocalizedDuplicateField`, `LocalizedInternal`) with `#[error("...")]`
    Display impls that deliberately preserve the canonical English prefix
    (`uniqueness violation: …`) so the frontend defensive regex
    `/^uniqueness violation: barcode/` in `src/lib/products.ts` keeps
    matching when the helper chain is bypassed.
  - Changed `CommandError::DuplicateField` from
    `{ field: String, value: String }` to
    `{ field: String, value: String, message: String }`. The structured
    `field` and `value` fields are preserved verbatim so the frontend
    `isDuplicateFieldError` check (`detail.field === "barcode"`) keeps
    working.
  - Updated `From<AppError> for CommandError>` to handle the three new
    variants and to populate `CommandError::DuplicateField.message` on
    the canonical (un-helped) path too, so the un-helped conversion
    never leaves the frontend `humanizeError` empty for `DuplicateField`.
  - Added 4 focused tests (Display regression guards + wire-shape
    serialization + un-helped conversion).

### Task 2 — wire command boundaries

- `src-tauri/src/commands/products.rs`
  - `create_category`, `update_category`, `create_product`,
    `update_product`, `add_product_barcode`: chained
    `localize_not_found`, `localize_duplicate_field`, and
    `localize_internal` after the existing validation/business_rule
    helpers at every locale-aware boundary.
  - `find_product_by_scan`: chained `localize_internal` (the scanner
    service does not raise `NotFound` or `DuplicateField`).
- `src-tauri/src/commands/stores.rs`
  - `create_store`, `update_store`, `create_store_location`: chained
    the three new helpers at locale-aware boundaries.
  - `update_store_location`: chained `localize_not_found` and
    `localize_internal`; `localize_duplicate_field` is intentionally
    omitted because this service path does not raise duplicate-field errors.
- `src-tauri/src/commands/expiry_lots.rs`
  - `create_expiry_lot`, `update_expiry_lot`, `archive_expiry_lot`:
    chained the three new helpers at locale-aware boundaries.
  - `resolve_expiry_lot`: chained `localize_not_found` and
    `localize_internal`; `localize_duplicate_field` is intentionally
    omitted because this service path does not raise duplicate-field errors.
- `src-tauri/src/commands/lot_movements.rs`
  - `create_lot_movement`: chained the three new helpers before the
    special `Result<_, String>` `.map_err(|e| e.to_string())` flatten,
    so localization is applied without changing the IPC shape.
- `src-tauri/src/commands/unit_definitions.rs`
  - **Left out by design**: every command in this module has no `locale`
    argument today. Adding `localize_*` here would require an IPC
    signature change for every command. A module-level doc-comment
    documents the rationale and the path forward (introduce an
    optional `locale: Option<String>` per command in a follow-up slice).
- `src-tauri/src/commands/notifications.rs`
  - **Left out by design**: same rationale as `unit_definitions.rs`.
    Module-level doc-comment added.
- `src-tauri/src/commands/health.rs`
  - **Left out by design**: the `Database unreachable` text is a
    developer-facing debug string, not a localized user message. The
    IPC payload stays on the constant English path to avoid a silent
    wire-shape change.
- `src-tauri/src/commands/categories.rs`
  - **Does not exist as a separate file** — category commands live
    inside `commands::products.rs` and were wired there.

## Verification (preliminary, before Task 3)

- `cargo test --lib services::user_messages`: 199 passed, 0 failed.
- `cargo test --lib error::`: 4 passed, 0 failed.
- `cargo test --lib` (full lib suite): 658 passed, 0 failed.
- `cargo build --lib`: clean, no dead-code warnings.
- `npx svelte-check --tsconfig ./tsconfig.json --threshold error`:
  0 errors, 0 warnings.
- Independent verifier PASS:
  - Re-ran `cargo test --lib services::user_messages --manifest-path src-tauri/Cargo.toml` — 199 passed, 0 failed.
  - Re-ran `cargo test --lib error:: --manifest-path src-tauri/Cargo.toml` — 4 passed, 0 failed.
  - Re-ran `cargo test --lib --manifest-path src-tauri/Cargo.toml` — 658 passed, 0 failed.
  - Re-ran `cargo build --lib --manifest-path src-tauri/Cargo.toml` — clean.
  - Re-ran `npx svelte-check --tsconfig ./tsconfig.json --threshold error` — 0 errors, 0 warnings.
  - Confirmed `DuplicateField` preserves `field` / `value` and adds `message` for `humanizeError`.
  - Confirmed intentionally omitted command modules are documented and not accidental misses.

## Notes

- Strict TDD was **not** activated for this slice, so RED/GREEN lifecycle
  evidence is reported as `not active` in the handoff.
- `DomainError::DuplicateField`'s `#[error("uniqueness violation: ...")]
  Display` impl is preserved so the frontend defensive regex in
  `src/lib/products.ts` (line 236, `/^uniqueness violation: barcode/`)
  keeps matching when the helper chain is bypassed.
- Commit: `f950e8f feat: localize backend error boundaries`.
- No push was performed. `.codegraph/` was left untouched.

