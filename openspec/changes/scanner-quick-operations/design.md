# Design — scanner-quick-operations

## 1. Overview

`scanner-quick-operations` adds a top-level Scanner workflow for repeated handheld-scanner use. The design keeps the existing dashboard scan/search path intact and introduces a separate scanner operation path because the new workflow resolves both products and lots and can mutate stock after explicit confirmation.

The implementation is split into reviewable slices:

1. Backend foundation: scanner lookup DTO/IPC plus settings persistence.
2. Frontend Scanner tab and operation modes.
3. Configuration selectors plus Tauri system-tray lifecycle.

The project review budget is 800 changed lines. The full feature is forecast above that budget, so implementation must use chained PR slices rather than one accumulated branch.

## 2. Existing model decisions

### 2.1 Lot code

Use the existing `expiry_lots.batch_code` column as the v1 lot code.

Rationale:

- `expiry_lots.batch_code` already exists in the database schema.
- The existing lot creation flow already accepts and preserves a non-empty `batch_code`.
- The backend already auto-generates batch codes when the user leaves the field empty.
- No schema migration is needed for v1.

Matching is exact after scanner-input trimming. The implementation must not uppercase, lowercase, or otherwise normalize the scanned value before lookup.

### 2.2 Existing scan/search remains separate

The current `find_product_by_scan` command remains product-only and is used by the existing dashboard scan/search surface. Scanner operations introduce a new command:

```text
resolve_scanner_code(input) -> ScannerResolveResult
```

This avoids changing the existing `ScanSearchResult` wire contract and avoids making dashboard search aware of stock-mutation flows.

### 2.3 Archived products

The new Scanner operation path ignores archived/inactive products for barcode and SKU matches. This is intentionally stricter than historical product search: Scanner is an operational stock mutation path and must not offer archived products for new sale/registration/stock-out operations.

## 3. Backend design

### 3.1 DTOs

Add scanner DTOs in `src-tauri/src/dto/scanner.rs` while preserving the existing scan-search DTOs.

Recommended shape:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "match_type", rename_all = "snake_case")]
pub enum ScannerResolveResult {
    LotMatch { lot: ExpiryLotResponse, product: ProductResponse },
    ProductMatch { product: ProductResponse, lots: Vec<ExpiryLotResponse> },
    Unknown { scanned_value: String },
}
```

If existing product/lot response types differ, use the already exported UI-facing response DTOs rather than introducing partial duplicate structs.

### 3.2 Lookup priority

Implement scanner lookup in a new service module, preferably `src-tauri/src/services/scanner.rs`, with this strict priority:

1. Active lot in active store by exact `expiry_lots.batch_code`.
2. Active product by exact product barcode.
3. Active product by exact product SKU.
4. Unknown.

For lot matches, return the lot and its parent product in one command result.

For product matches, return active lots for the active store ordered by stable FEFO order:

```text
expiry_date ASC, created_at ASC, id ASC
```

The active store comes from the persisted settings snapshot. If there is no active store, the frontend should block Scanner input before invoking lookup; the backend should still fail safely with a validation error if invoked without store context.

### 3.3 Repository helpers

Add narrowly scoped helpers rather than changing broad product search behavior:

- `find_active_lot_by_batch_code(store_id, batch_code)` in the lot repository.
- Scanner-specific product lookup helpers that filter inactive products.
- `list_active_lots_for_product_in_store(product_id, store_id)` ordered by FEFO.

Do not retrofit `find_product_by_scan` unless a later change deliberately decides dashboard scan/search should also ignore archived products.

### 3.4 Settings persistence

Extend settings with two persisted keys in `app_settings`:

- `scanner_fefo_policy`
- `close_behavior`

Use Rust enums serialized as snake_case:

```rust
FefoPolicy:
- suggest_fefo
- require_fefo
- manual_lot_choice

CloseBehavior:
- minimize_to_tray
- exit_application
```

Defaults:

- `scanner_fefo_policy = suggest_fefo`
- `close_behavior = minimize_to_tray`

Extend:

- `SettingsResponse`
- `SettingsUpdate`
- settings repository helpers
- settings service partial-update branches
- IPC validation in the stores command boundary
- TS types in `src/lib/stores.ts`

Invalid enum values must be rejected at the IPC boundary and must not rewrite existing settings.

## 4. Stock movement design

Sale and Stock-out both reuse the existing lot movement ledger.

### 4.1 Sale

Sale mode creates a movement with:

```text
kind = exit:sale
```

The Scanner UI defaults quantity to `1`, lets the user edit it, requires a selected source lot/location, and only invokes the backend after explicit confirmation.

### 4.2 Stock-out

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

It must not list `exit:sale`; sale has its own mode.

Reuse existing notes rules. In particular, `exit:inventory_adjustment` and `exit:other` require notes if that is how the existing movement domain currently validates them.

### 4.3 Source location

If the selected lot has exactly one source location balance, the UI may preselect that location. If the lot has multiple location balances, the UI must ask the user to choose before enabling confirmation.

## 5. FEFO runtime behavior

FEFO applies when a scan resolves to a product with multiple lots.

- `suggest_fefo`: preselect FEFO lot, show it as recommendation, allow override.
- `require_fefo`: preselect FEFO lot, disable alternate lot choice while that lot has stock.
- `manual_lot_choice`: show lots without preselecting by expiration; user must choose.

A direct lot-code match bypasses FEFO selection because the user scanned a specific lot identity. The UI may still show a notice if configured policy is `require_fefo`, but it must not silently replace the scanned lot with another lot.

## 6. Frontend design

### 6.1 Navigation

Add `scanner` to the `App.svelte` tab union and tabs array. Place it between Reports and Import to match the spec. Reuse the existing `aria-current="page"` and mobile overflow patterns.

### 6.2 ScannerPage

Add `src/components/ScannerPage.svelte`.

Responsibilities:

- Render the three modes: Sale, Registration, Stock-out.
- Keep one scanner input focused when practical.
- Submit on Enter.
- Trim only leading/trailing whitespace before invoking lookup.
- Debounce repeated identical scans within 400 ms.
- Keep resolved scan state when switching modes.
- Disable mutation buttons while an IPC call is in flight.
- Keep form state on failure and clear it only on success.

Add `src/lib/scanner.ts` as the typed wrapper around `resolve_scanner_code`.

### 6.3 Registration mode

For an existing product match, Registration opens a new-lot creation path for that product. It must not add quantity to an existing lot in v1.

For an unknown scan, Registration starts quick product creation and preserves the scanned value verbatim. The UI should let the user choose whether to use that scanned value as SKU or barcode.

If a product match came from barcode/SKU, do not automatically use that scanned value as a lot `batch_code`. If the user is creating a new lot and wants a specific batch code, they can edit the lot form.

### 6.4 i18n

Add translation keys in both locale trees for:

- `nav.scanner`
- scanner mode labels
- scanner input placeholder/help
- Sale labels and success/error copy
- Registration labels and success/error copy
- Stock-out labels and success/error copy
- FEFO notices
- Configuration FEFO selector
- Configuration close-behavior selector

Reuse existing lot movement reason labels where practical instead of duplicating reason vocabulary.

## 7. Configuration design

Add two Configuration sections using the existing selector pattern:

1. Scanner FEFO policy
2. Close behavior

Both selectors must:

- optimistically update local state,
- call `updateSettings(...)`,
- roll back on failure,
- show translated inline error copy,
- preserve unrelated settings on partial update.

## 8. Desktop lifecycle and tray design

Caduxo uses Tauri 2. The tray feature should use the built-in Tauri tray API if available in the current crate version. Do not add a tray plugin unless the build proves the built-in API is unavailable.

Implementation strategy:

1. During setup, create a tray icon using the existing app icon.
2. Add tray menu entries:
   - Restore
   - Quit
3. Restore calls `window.show()` and then attempts `window.set_focus()`.
4. Quit calls `app.exit(0)`.
5. Install a `WindowEvent::CloseRequested` handler.
6. If `close_behavior == minimize_to_tray` and tray setup succeeded, call `api.prevent_close()` and `window.hide()`.
7. If `close_behavior == exit_application`, do not prevent close.
8. If tray setup fails on a platform, log a warning and fall back to normal exit for that runtime session. Do not persistently rewrite the user's setting.

The app currently has one main window. If needed, give it an explicit `label = "main"` in Tauri config so lifecycle code can address it reliably.

## 9. Test and verification strategy

### Backend automated tests

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Cover:

- lot-code match wins over barcode and SKU,
- barcode match wins over SKU,
- archived products are ignored by scanner lookup,
- lot-code lookup is scoped to the active store,
- unknown scan preserves trimmed scanned value,
- FEFO ordering is stable,
- settings defaults are returned on missing rows,
- partial settings updates preserve unrelated fields,
- invalid FEFO/close-behavior values are rejected.

### Frontend checks

Run:

```bash
npx svelte-check --workspace . --threshold error
npm run build
```

Cover:

- i18n key parity,
- ScannerPage type safety,
- settings selector type safety,
- App tab union exhaustiveness.

### Manual smoke

Required manual checks:

- Sale: scan lot code, quantity defaults to `1`, confirm, lot quantity decreases and movement is `exit:sale`.
- Sale: scan product barcode with multiple lots under each FEFO policy.
- Registration: scan existing product and create a new lot.
- Registration: scan unknown code and use it as SKU or barcode in quick creation.
- Stock-out: scan product, choose existing non-sale reason, confirm, movement records with selected reason.
- Stock-out: confirm `exit:sale` is not listed.
- Configuration: FEFO and close-behavior selectors persist and roll back on failure.
- Tray: close hides to tray by default; Restore shows the window; Quit exits.
- Exit behavior: when configured to exit, closing the window exits normally.

## 10. PR / slice plan

### PR 1 — Backend foundation

Files likely touched:

- `src-tauri/src/dto/scanner.rs`
- `src-tauri/src/dto/stores.rs`
- `src-tauri/src/db/repositories/settings.rs`
- `src-tauri/src/db/repositories/expiry_lots.rs`
- `src-tauri/src/services/scanner.rs`
- `src-tauri/src/services/settings.rs`
- `src-tauri/src/commands/scanner.rs`
- `src-tauri/src/commands/stores.rs`
- `src-tauri/src/lib.rs`

Exit criteria:

- scanner lookup command registered,
- settings fields persisted,
- backend tests pass.

### PR 2 — Scanner frontend

Files likely touched:

- `src/App.svelte`
- `src/components/ScannerPage.svelte`
- `src/lib/scanner.ts`
- `src/lib/stores.ts`
- `src/i18n/en/index.ts`
- `src/i18n/es/index.ts`

Exit criteria:

- Scanner tab usable for Sale, Registration, Stock-out,
- no stock mutation before confirmation,
- `svelte-check` and build pass.

### PR 3 — Configuration and tray lifecycle

Files likely touched:

- `src/components/ConfigurationPage.svelte`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json` if a stable main-window label is needed
- locale files for settings/tray copy

Exit criteria:

- FEFO and close behavior configurable,
- close-to-tray default works,
- restore and quit work from tray,
- normal exit setting works.

## 11. Risks

- Tauri tray behavior varies by Linux desktop environment; manual verification is required.
- Scanner lookup depends on active store semantics. The UI must block Scanner operations when no store exists.
- Direct lot-code scans bypass FEFO. This is deliberate, but should be visible in review because it is a business rule.
- Full feature exceeds the review budget as a single PR; implementation must stay sliced.
