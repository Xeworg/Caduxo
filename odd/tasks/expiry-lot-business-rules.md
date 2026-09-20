# Expiry lot BusinessRule localization

## Goal
Localize expiry-lot BusinessRule messages with dynamic status/quantity/unit data while preserving canonical English service output and command compatibility.

## Scope
- Add catalog/parser support for expiry-lot update/archive/resolve BusinessRule messages.
- Add catalog/parser support for the adjacent resolve quantity Validation message if it shares the same command boundary.
- Keep services emitting canonical English through `user_message(..., Locale::En)`.
- Add optional locale command arguments where needed to preserve existing frontend calls.
- Localize at `commands/expiry_lots.rs` boundaries using existing `localize_validation` and `localize_business_rule`.

## Non-goals
- Do not touch `lot_movements` or change its `Result<_, String>` command contract.
- Do not localize archive reason/notes validation, resolution type validation, NotFound, DuplicateField, CSV DTOs, backup validation, or internal errors.
- Do not change frontend wrappers/call sites.
- Do not add a third locale.

## Tasks
- [x] Task 1: Extend `UserMessage` catalog/parser/tests for expiry-lot dynamic messages.
- [x] Task 2: Emit canonical catalog strings in expiry-lot service guards.
- [x] Task 3: Wire optional locale localization through expiry-lot command boundaries.
- [x] Task 4: Run focused backend checks and independent verification.
- [x] Task 5: Commit the verified slice after explicit user request.

## Evidence

### Task 1 — `UserMessage` catalog/parser/tests
Added 6 variants in `src-tauri/src/services/user_messages.rs` with byte-for-byte
EN/ES translations, parser entries, and per-variant tests:

| Variant | EN shape | Status |
| --- | --- | --- |
| `CannotUpdateLotStatus { status }` | `Cannot update lot: status is \`{status}\`` | done |
| `CannotChangeQuantityDirect { quantity, unit }` | `Cannot change quantity of expiry lot directly: quantity must remain {quantity:.2} {unit}. Use movement / adjustment / resolve actions to change it.` | done |
| `CannotArchiveLotStatus { status }` | `Cannot archive lot: status is already \`{status}\`` | done |
| `LotNoLongerActive` | `Lot is no longer active and cannot be archived` | done |
| `CannotResolveLotStatus { status }` | `Cannot resolve lot: lot is already \`{status}\`` | done |
| `ResolveQuantityExceedsRemaining { requested, unit, available }` | `Cannot resolve {requested:.2} {unit}: only {available:.2} {unit} remain` | done |

Parser helpers added: `parse_backticked_suffix` (shared by the three status
messages, with guards against empty status / inner backticks),
`parse_change_quantity_direct`, `parse_resolve_quantity_exceeds_remaining`
(rejects mismatched units between the two sides).

Test coverage added (35 new tests):
- EN/ES literal assertions per variant (13 tests).
- Parser roundtrips (format → parse → format) per variant (6 tests).
- Parser prefix disambiguation guards (8 negative tests).
- `localize_business_rule` integration per BusinessRule variant (5 tests).
- `localize_validation` integration for the Validation variant (1 test).
- Cross-helper guards (Validation must not be touched by
  `localize_business_rule` and vice-versa) (2 tests).

### Task 2 — Service emits canonical catalog strings
`src-tauri/src/services/expiry_lots.rs` now emits each of the 5 BusinessRule
guards and the resolve-quantity Validation through
`user_message(UserMessage::..., Locale::En)`, preserving the exact English
byte sequence the parser recognises. Replaced:
- `Cannot update lot: status is \`{existing.status}\`` →
  `CannotUpdateLotStatus { status }`.
- `Cannot change quantity of expiry lot directly: quantity must remain {quantity:.2} {unit}. Use movement / adjustment / resolve actions to change it.` →
  `CannotChangeQuantityDirect { quantity, unit }`.
- `Cannot archive lot: status is already \`{lot.status}\`` →
  `CannotArchiveLotStatus { status }`.
- `Lot is no longer active and cannot be archived` →
  `LotNoLongerActive`.
- `Cannot resolve lot: lot is already \`{lot.status}\`` →
  `CannotResolveLotStatus { status }`.
- `Cannot resolve {requested:.2} {unit}: only {available:.2} {unit} remain` →
  `ResolveQuantityExceedsRemaining { requested, unit, available }`.

The pre-existing `update_lot_rejects_quantity_change` test continues to
match the message prefix (`Cannot change quantity of expiry lot directly`)
because the EN shape is byte-for-byte identical.

### Task 3 — Command boundary localization
`src-tauri/src/commands/expiry_lots.rs`:
- Added `use crate::pdf::locale::Locale;` and
  `use crate::services::user_messages::{localize_business_rule, localize_validation};`.
- Added local helper `fn resolve_locale(locale: Option<String>) -> Locale`
  matching the pattern in `commands/products.rs` and `commands/stores.rs`.
- `create_expiry_lot`: kept `locale: String` (required) to preserve the
  existing frontend contract — the `createExpiryLot` wrapper already passes
  the active locale. Chained `localize_validation` then `localize_business_rule`
  before `AppError::into`, so both the `LocationRequired` Validation and the
  constant `at least one store must exist first` BusinessRule reach the UI in
  the active locale (the store-required string is not in the catalog and
  passes through `localize_business_rule` unchanged, but the chain is
  consistent with the rest of the commands and ready for any future catalog
  addition).
- `update_expiry_lot`, `archive_expiry_lot`, `resolve_expiry_lot`: changed
  the `locale` arg to `Option<String>` and threaded it through
  `resolve_locale`. Tauri's IPC serialises missing optional arguments as
  `null`, so existing frontend callers (`updateExpiryLot(input)`,
  `archiveExpiryLot(input)`, `resolveExpiryLot(input)`) keep working with
  English as the safe default.

### Task 4 — Focused backend checks
Writer-observed checks:
- `cargo test --manifest-path src-tauri/Cargo.toml --lib services::user_messages::`:
  `113 passed; 0 failed` (78 pre-existing + 35 new).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib services::expiry_lots::`:
  `33 passed; 0 failed` (no change in count; assertions that match by
  message prefix continue to hold).
- `cargo build --manifest-path src-tauri/Cargo.toml --lib`: clean build.
- Cross-suite sanity: `cargo test --manifest-path src-tauri/Cargo.toml --lib`:
  `568 passed; 0 failed`.

Independent verifier result: **PASS**.
- Re-ran `cargo test --manifest-path src-tauri/Cargo.toml --lib services::user_messages::`:
  `113 passed; 0 failed`.
- Re-ran `cargo test --manifest-path src-tauri/Cargo.toml --lib services::expiry_lots::`:
  `33 passed; 0 failed`.
- Confirmed parser round-trips and malformed/colliding-shape guards.
- Confirmed services emit canonical English through `user_message(..., Locale::En)`.
- Confirmed command localization and optional locale compatibility by inspection.
- Confirmed scope boundaries: no diffs in `lot_movements`, frontend `src/`, DTO/CSV paths, NotFound, DuplicateField, or internal-error localization.

No frontend, lot_movements, archive reason/notes validation, resolution-type
validation, NotFound, DuplicateField, CSV DTOs, backup validation, or
internal-error paths were touched. Service output is unchanged for English
callers; Spanish callers now see the localized text via `localize_*` at the
command boundary.

### Task 5
- User explicitly requested the commit after the verified slice.
- `git diff --check` was clean before staging.
- Commit: `6aa1a86 feat: localize expiry lot business rules`.

