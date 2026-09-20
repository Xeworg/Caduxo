# Lot movement backend localization

## Goal
Localize lot movement backend validation and business-rule messages while preserving the current string-based IPC contract.

## Scope
- Add `UserMessage` catalog/parser coverage for lot movement validation and business-rule messages.
- Convert unit-kind quantity validation from hardcoded Spanish to canonical English plus locale-aware translation at command boundary.
- Keep `commands/lot_movements.rs` returning `Result<_, String>` to avoid changing frontend error handling in this slice.
- Thread the active frontend locale through create-lot-movement call sites.

## Non-goals
- Do not migrate lot movement commands to `CommandError` in this slice.
- Do not localize repository/database/internal errors such as product unit lookup failures.
- Do not redesign frontend error object handling or the broader Tauri error contract.
- Do not add a third locale.

## Tasks
- [x] Task 1: Extend `UserMessage` catalog/parser/tests for lot movement messages.
- [x] Task 2: Emit canonical English for unit-kind quantity validation and command-boundary localization while keeping string IPC.
- [x] Task 3: Thread locale through frontend lot movement wrappers and modal call sites.
- [x] Task 4: Run focused backend/frontend checks and independent verification.
- [x] Task 5: Commit the verified slice after explicit user request.

## Evidence

### Exploration
- `commands/lot_movements.rs` currently returns `Result<_, String>` and flattens `AppError` via `to_string()`.
- `services/lot_movements.rs` already returns `AppError` and emits several user-facing `Validation`/`BusinessRule` messages.
- `domain/lot_movements.rs::validate_quantity_for_unit_kind` emits Spanish strings directly; this must become English canonical text so `localize_validation` can translate by locale.
- Recommended bounded implementation keeps the IPC string contract and adds a local command flattener after `localize_validation`/`localize_business_rule`.

### Task 1 — `UserMessage` catalog/parser/tests
Added 18 variants in `src-tauri/src/services/user_messages.rs` with byte-for-byte
EN/ES translations, parser entries, and per-variant tests:

| Variant | EN shape | Status |
| --- | --- | --- |
| `UnknownMovementKind { kind }` | `` Unknown movement kind: `{kind}` `` | done |
| `DirectionRequiredForInventoryAdjustment` | `direction is required for inventory_adjustment` | done |
| `DirectionOnlyForInventoryAdjustment { kind }` | `` direction is only valid for inventory_adjustment, not `{kind}` `` | done |
| `EntryInitialNoSource` | `entry:initial must not have a source location` | done |
| `EntryInitialRequiresDestination` | `entry:initial requires a destination location` | done |
| `TransferRequiresSource` | `transfer requires a source location` | done |
| `TransferRequiresDestination` | `transfer requires a destination location` | done |
| `TransferSourceAndDestinationDiffer` | `transfer source and destination must differ` | done |
| `InventoryAdjustmentIncreaseNoSource` | `inventory_adjustment with direction=increase must not have a source location` | done |
| `InventoryAdjustmentIncreaseRequiresDestination` | `inventory_adjustment with direction=increase requires a destination location` | done |
| `InventoryAdjustmentDecreaseNoDestination` | `inventory_adjustment with direction=decrease must not have a destination location` | done |
| `InventoryAdjustmentDecreaseRequiresSource` | `inventory_adjustment with direction=decrease requires a source location` | done |
| `ExitRequiresSource { kind }` | `` `{kind}` requires a source location `` | done |
| `ExitNoDestination { kind }` | `` `{kind}` must not have a destination location `` | done |
| `NotesRequiredForMovement { kind }` | `` Notes are required for movement kind `{kind}` `` | done |
| `QuantityIntegerFractional { value }` | `Quantity must be a whole number for integer-unit products, got {value}` | done |
| `InsufficientBalance { available, requested }` | `Insufficient balance at source location: available={available}, requested={requested}` | done |
| `LocationInactive` | `Location is inactive` | done |

Parser helpers added: `parse_exit_kind_message` (shared by the two exit-kind
dynamic shapes; guards against missing backticks, inner backticks, empty kind),
`parse_integer_fractional` (rejects non-numeric values), and
`parse_insufficient_balance` (rejects missing middle / non-numeric pair).

Test coverage added (67 new tests, bringing `services::user_messages` from
113 to 180):
- EN/ES literal assertions per variant (34 tests).
- Parser roundtrips (format → parse → format) per variant (16 tests).
- Parser prefix disambiguation guards (8 negative tests covering
  `parse_exit_kind_message`, `parse_integer_fractional`, and
  `parse_insufficient_balance`).
- `localize_validation` integration for the new Validation variants
  (`UnknownMovementKind`, `EntryInitialNoSource`,
  `QuantityIntegerFractional`) (3 tests).
- `localize_business_rule` integration for the new BusinessRule variants
  (`InsufficientBalance`, `LocationInactive`) (2 tests).
- Cross-helper guards (e.g. `Failed to resolve product unit kind: ...` is
  preserved untouched by `localize_validation`) (1 test).
- Three additional guards covering `QuantityNonNegative` (regression) and
  constant-text collisions.

### Task 2 — Service emits canonical English through the catalog
- `domain/lot_movements.rs::validate_quantity_for_unit_kind` now returns
  `Result<(), UserMessage>` (domain-pure). The non-positive branch reuses
  the existing `QuantityPositive { value }` variant; the new integer-unit
  fractional branch surfaces the new `QuantityIntegerFractional { value }`
  variant. Tests updated to assert on the variant shape instead of the old
  Spanish substring matches. `validate_qty_error_messages_use_catalog_variants`
  replaces the previous Spanish-substring test and confirms every rejection
  branch surfaces a catalog variant.
- `services/lot_movements.rs` now emits every user-facing Validation and
  BusinessRule message through the `user_message(UserMessage::..., Locale::En)`
  catalog. A local `en_message(kind)` helper keeps the call sites readable.
  Replaced hardcoded `format!("...")` strings for:
  - `Unknown movement kind: \`{kind}\``
  - `direction is required for inventory_adjustment`
  - `direction is only valid for inventory_adjustment, not \`{kind}\``
  - `entry:initial must not have a source location`
  - `entry:initial requires a destination location`
  - `transfer requires a source location`
  - `transfer requires a destination location`
  - `transfer source and destination must differ`
  - `inventory_adjustment with direction=increase must not have a source location`
  - `inventory_adjustment with direction=increase requires a destination location`
  - `inventory_adjustment with direction=decrease must not have a destination location`
  - `inventory_adjustment with direction=decrease requires a source location`
  - `\`{kind}\` requires a source location`
  - `\`{kind}\` must not have a destination location`
  - `Quantity must be non-negative, got {value}` (existing catalog variant)
  - `Quantity must be positive, got {value}` (existing catalog variant)
  - `Notes are required for movement kind \`{kind}\``
  - `Insufficient balance at source location: available={available}, requested={requested}`
  - `Location is inactive`

  `Failed to resolve product unit kind: {err}` is kept as a non-catalog
  `format!("...")` because it's an internal repository error (the spec
  explicitly excludes it from the localization scope).

  The pre-existing
  `create_lot_movement_integer_unit_rejects_fractional_quantity` integration
  test was updated to match the new canonical English text; the byte-level
  assertion is now `"whole number" && "integer-unit products"` instead of
  the old Spanish substring.

### Task 3 — Command boundary localization + frontend locale threading
- `src-tauri/src/commands/lot_movements.rs`:
  - Added `use crate::pdf::locale::Locale;` and
    `use crate::services::user_messages::{localize_business_rule, localize_validation};`.
  - Added local helper `fn resolve_locale(locale: Option<String>) -> Locale`
    matching the pattern in `commands/products.rs` and `commands/expiry_lots.rs`.
  - `create_lot_movement` now accepts `locale: Option<String>` and chains
    `localize_validation` then `localize_business_rule` before flattening
    to `String`. Non-Validation / non-BusinessRule errors (NotFound,
    Infrastructure, the non-catalog `Failed to resolve product unit kind`
    Validation) pass through `localize_*` untouched and reach the UI in
    canonical English — the same safe behaviour as before.
  - The other two commands (`list_lot_movements`, `get_lot_location_balances`)
    keep their existing shape; they only emit the existing
    `resource not found` / database error surfaces that are out of scope.
- `src/lib/lot_movements.ts`:
  - `createLotMovement` now takes an optional second argument
    `locale?: Locales` (typed against the generated `i18n-types.ts`
    `Locales` union). The argument is forwarded as `{ input, locale }`
    in the `invoke` call so Tauri's IPC serialises it as the command's
    third parameter. Omitting the argument preserves the previous
    English-only behaviour.
- `src/components/MoveStockModal.svelte`,
  `src/components/RegisterExitModal.svelte`,
  `src/components/AdjustCountModal.svelte`:
  - Each modal now `import { locale } from "../i18n/locale.svelte.js";` and
    forwards `locale.current` to `createLotMovement` at the call site.

### Task 4 — Focused checks
Writer-observed checks:
- `cargo test --manifest-path src-tauri/Cargo.toml --lib services::user_messages::`:
  `180 passed; 0 failed` (113 pre-existing + 67 new).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib domain::lot_movements::`:
  `46 passed; 0 failed` (no count change; the previous Spanish-substring
  test was rewritten to a catalog-variant match).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib services::lot_movements::`:
  `33 passed; 0 failed` (the integer-unit-fractional rejection assertion was
  updated to the new canonical English text).
- `cargo build --manifest-path src-tauri/Cargo.toml --lib`: clean build.
- `npx svelte-check --tsconfig ./tsconfig.json --threshold error`:
  `svelte-check found 0 errors and 0 warnings`.
- Cross-suite sanity: `cargo test --manifest-path src-tauri/Cargo.toml --lib`:
  `635 passed; 0 failed` (568 baseline + 67 new).

No `Failed to resolve product unit kind` message, NotFound, database error,
third locale, or `CommandError` migration was touched. The string IPC
contract is preserved at the command boundary.

### Task 5 — Commit
- User explicitly requested the commit after the verified slice.
- `git diff --check` was clean before staging.
- Commit: `d38a5d6 feat: localize lot movement errors`.
