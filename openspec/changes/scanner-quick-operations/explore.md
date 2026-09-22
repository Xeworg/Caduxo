# Explore — scanner-quick-operations

Read-only exploration note for the `scanner-quick-operations` SDD change. No source code was edited in this phase.

## 1. Lot code data model

`expiry_lots.batch_code` already exists in the schema and is surfaced by the current lot creation/editing flow. The backend already auto-generates batch codes when the field is blank and preserves a non-empty user-provided value.

**Decision:** v1 reuses `expiry_lots.batch_code` as the scanner lot code. No schema migration is required.

Scanner lookup must compare the trimmed scanned value exactly against `batch_code`. It must not uppercase, lowercase, or otherwise normalize the code.

## 2. Existing scan/search path

The current dashboard scan/search path uses `find_product_by_scan` and `ScanSearchResult` with a product-oriented shape:

- found product plus `has_lots`
- not found plus scanned value

That path is intentionally product-only and should remain unchanged.

**Decision:** add a new scanner operation command, `resolve_scanner_code`, instead of reusing or extending `find_product_by_scan`.

Reasons:

1. Scanner operations need lot-code matching before product matching.
2. Scanner operations need active lots for FEFO selection.
3. Scanner operations are stock-mutation flows and must ignore archived/inactive products.
4. The dashboard search path should not inherit those operational constraints.

## 3. Scanner lookup contract

The new command should return a discriminated result:

```rust
ScannerResolveResult::LotMatch { lot, product }
ScannerResolveResult::ProductMatch { product, lots }
ScannerResolveResult::Unknown { scanned_value }
```

Lookup priority:

1. active lot in active store by exact `expiry_lots.batch_code`,
2. active product by exact barcode,
3. active product by exact SKU,
4. unknown.

For product matches, lots should be ordered by stable FEFO order:

```text
expiry_date ASC, created_at ASC, id ASC
```

Archived/inactive products must not match in the Scanner operation path.

## 4. Movement/reason reuse

The existing movement ledger already supports the needed operation kinds.

Sale mode uses:

```text
exit:sale
```

Stock-out mode lists the existing non-sale exit reasons only:

```text
exit:waste
exit:expired
exit:damaged
exit:internal_consumption
exit:return_to_supplier
exit:inventory_adjustment
exit:other
```

`exit:sale` must not appear in Stock-out mode because Sale has its own mode.

Existing notes-required behavior should be reused. In particular, the current app requires notes for:

```text
exit:inventory_adjustment
exit:other
```

ScannerPage should mirror these rules for immediate UX feedback, but backend movement validation remains the source of truth.

## 5. Settings persistence pattern

Settings use the existing `app_settings` key/value table and the existing pipeline:

```text
commands::stores::update_settings
services::settings::update_settings
db::repositories::settings::upsert_setting
```

The new keys are:

```text
scanner_fefo_policy
close_behavior
```

Values:

```text
scanner_fefo_policy:
- suggest_fefo
- require_fefo
- manual_lot_choice

close_behavior:
- minimize_to_tray
- exit_application
```

Defaults:

```text
scanner_fefo_policy = suggest_fefo
close_behavior = minimize_to_tray
```

Implementation should mirror the existing language/theme settings pattern:

- Rust enum or validated string set at IPC boundary.
- Repository get/set helpers with fallback defaults.
- Partial update branches that do not touch unrelated keys.
- TypeScript mirror in `src/lib/stores.ts`.
- Configuration selectors using optimistic update and rollback.

Invalid values must be rejected at the IPC boundary and must not rewrite persisted rows.

## 6. App navigation and Configuration patterns

`src/App.svelte` has a typed tab union, a `tabs` array, desktop navbar, and mobile overflow dropdown. Scanner should be added as a new tab between Reports and Import and should reuse the existing `aria-current="page"` pattern.

`src/components/ConfigurationPage.svelte` already uses the correct pattern for selectors:

1. snapshot previous value,
2. optimistic local update,
3. call `updateSettings`,
4. on success update local settings snapshot,
5. on failure roll back and show a translated inline error.

FEFO policy and close behavior selectors should mirror that pattern.

## 7. Tauri tray and close behavior

The project uses Tauri 2. The design should prefer the built-in Tauri tray API first. A tray plugin should be added only if the build proves the built-in API is unavailable.

Tray behavior:

- Create tray icon during setup.
- Add Restore and Quit menu actions.
- Restore calls `window.show()` and attempts `window.set_focus()`.
- Quit calls `app.exit(0)`.
- Close handler intercepts `WindowEvent::CloseRequested`.

Close behavior:

- `minimize_to_tray`: prevent close and hide the window.
- `exit_application`: do not prevent close; let Tauri exit normally.

If tray creation fails on a platform, log a warning and fall back to normal exit for that runtime session. Do not rewrite the persisted user setting.

`src-tauri/capabilities/default.json` already includes `core:default`. Add tray-specific capability only if the build reports it is required.

## 8. Candidate files by PR slice

### PR 1 — Backend foundation

Likely files:

- `src-tauri/src/dto/stores.rs`
- `src-tauri/src/dto/scanner.rs`
- `src-tauri/src/db/repositories/settings.rs`
- `src-tauri/src/db/repositories/expiry_lots.rs`
- `src-tauri/src/db/repositories/products.rs`
- `src-tauri/src/services/settings.rs`
- `src-tauri/src/services/scanner.rs`
- `src-tauri/src/services/user_messages.rs`
- `src-tauri/src/commands/scanner.rs`
- `src-tauri/src/commands/stores.rs`
- `src-tauri/src/lib.rs`

Exit criteria:

- `resolve_scanner_code` registered and tested.
- Settings fields persisted and tested.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` green.

### PR 2 — Scanner frontend

Likely files:

- `src/App.svelte`
- `src/components/ScannerPage.svelte`
- `src/lib/scanner.ts`
- `src/lib/stores.ts`
- `src/i18n/en/index.ts`
- `src/i18n/es/index.ts`

Exit criteria:

- Scanner tab renders and supports Sale, Registration, and Stock-out flows.
- No stock mutation before explicit confirmation.
- `npx svelte-check --workspace . --threshold error` green.
- `npm run build` green.

### PR 3 — Configuration + tray lifecycle

Likely files:

- `src/components/ConfigurationPage.svelte`
- `src/i18n/en/index.ts`
- `src/i18n/es/index.ts`
- `src-tauri/src/lib.rs`
- optionally `src-tauri/tauri.conf.json` if an explicit `main` window label is required
- optionally `src-tauri/capabilities/default.json` if tray capability is required by build

Exit criteria:

- FEFO and close behavior selectors persist and roll back on failure.
- Close hides to tray by default.
- Restore brings the window back.
- Quit exits cleanly.
- Exit behavior setting exits normally.

## 9. Risks and implementation notes

- Full feature exceeds the 800-line budget as one PR; keep the chained PR plan.
- PR 2 is the likely oversize slice and may need a PR 2a / PR 2b split if it exceeds 400 net lines.
- Direct lot-code matches bypass FEFO by design; do not replace a scanned lot with another lot.
- Scanner lookup ignores archived products, while dashboard search remains unchanged.
- Tauri tray behavior varies by Linux desktop environment; Windows and Linux manual smoke are required.
- Add tray plugin or tray capability only if the build proves it is needed.
- i18n parity must be preserved in both locale trees whenever visible strings are added.

## 10. Recommended next step

Proceed to PR 1 apply only after the parent confirms the chain strategy and slice boundaries. Recommended chain strategy remains `stacked-to-main`:

1. PR 1 backend foundation to `main`.
2. PR 2 scanner frontend stacked on PR 1.
3. PR 3 configuration + tray stacked on PR 2.
