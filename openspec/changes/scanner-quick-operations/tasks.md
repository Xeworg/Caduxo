# Tasks — scanner-quick-operations

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1020 net (PR 1 backend ≈ 360 across settings + scanner lookup + tests, PR 2 scanner UI ≈ 500 across `ScannerPage.svelte` + App.svelte navbar + `lib/scanner.ts` + i18n keys, PR 3 config + tray ≈ 160 across ConfigurationPage selectors + tray + lifecycle + i18n) |
| Project review budget | 800 changed lines (`openspec/config.yaml` → `sdd.reviewBudgetChangedLines`) |
| 400-line budget risk | **High** — the full feature is ~128% of the 800-line project budget. Per-PR risk is Low for PR 1 (~360 net), **Medium/High for PR 2** (~500 net, may need a 2a/2b split), Low for PR 3 (~160 net). |
| Chained PRs recommended | **Yes** — the 800-line budget is the project's hard ceiling per `openspec/config.yaml`, and the spec/design already exceed it as a single PR. |
| Suggested split | **PR 1 — Backend foundation**: `resolve_scanner_code` IPC + scanner service + repo helpers + tests + settings persistence (`scanner_fefo_policy`, `close_behavior` enums + repo + IPC + tests). **PR 2 — Frontend scanner surface**: `ScannerPage.svelte` with three modes, App.svelte navbar entry, `src/lib/scanner.ts`, lib/stores.ts type updates, and the full i18n key tree under `$LL.scanner.*`. **PR 3 — Configuration + tray lifecycle**: ConfigurationPage selectors (FEFO + close-behavior) + tray icon + `WindowEvent::CloseRequested` handler + i18n keys under `$LL.configuration.*`. If PR 2's diff exceeds 400 net lines during implementation, escalate and split into PR 2a (`ScannerPage.svelte` + modes + navbar + `lib/scanner.ts`) and PR 2b (i18n keys + remaining wiring). |
| Delivery strategy | `ask-on-risk` (per `proposal.md`) — escalate to the user before merging any PR whose net diff exceeds the 400-line review-budget threshold, and before any `size:exception` decision. |
| Chain strategy | `stacked-to-main` (proposed): PR 1 lands to `main`, PR 2 stacks onto PR 1's branch and lands once PR 1 is in, PR 3 stacks onto PR 2's branch and lands once PR 2 is in. The orchestrator confirms with the user before PR 1 apply. |
| Strict TDD | `false` (project default). RED/GREEN evidence below is reported as `not active` because strict TDD was not activated for this change. |
| Native review candidate | The first work-unit commit per slice, not the accumulated feature branch |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

The 800-line review budget is the project's hard ceiling per change (`openspec/config.yaml`). The full scanner workflow (3 modes + IPC + settings + tray + lifecycle) is structurally above that ceiling when delivered as a single PR, so the chain recommendation is non-optional. The orchestrator must confirm the slice boundaries, the chain strategy, and any PR 2 sub-split before PR 1 apply.

---

## Slice 1 — Explore current model and IPC skeleton

Goal: ground every later slice in the existing data model, command surface, and lifecycle code. No production code changes here beyond a thin exploration note inside this change folder.

- [x] Read `src-tauri/src/db/migrations.rs` and confirm `expiry_lots.batch_code` is the existing column that will act as the scanned "lot code"; confirm that v1 needs **no schema migration** (the existing lot-creation flow already accepts and preserves a non-empty `batch_code`, and the backend auto-generates one when the field is blank). <!-- sdd-owner: implementation -->
- [x] Read `src-tauri/src/services/products.rs::find_product_by_scan` and `src-tauri/src/dto/scanner.rs` to capture the existing `ScanSearchResult` discriminator and confirm the new `resolve_scanner_code` IPC lives alongside `find_product_by_scan` without colliding. The dashboard scan/search surface MUST keep using the existing `find_product_by_scan`; the new Scanner tab MUST NOT call it. <!-- sdd-owner: implementation -->
- [x] Read `src-tauri/src/services/lot_movements.rs` (around the existing `MovementKind::Exit*` validation) and `src-tauri/src/domain/lot_movements.rs` to confirm Sale (`exit:sale`) and the seven Stock-out reasons (`exit:waste`, `exit:expired`, `exit:damaged`, `exit:internal_consumption`, `exit:return_to_supplier`, `exit:inventory_adjustment`, `exit:other`) reuse the existing ledger unchanged. <!-- sdd-owner: implementation -->
- [x] Read `src/components/RegisterExitModal.svelte`, `src/components/LotForm.svelte`, and `src/lib/lot_movements.ts` to capture the existing exit-reason picker pattern, integer-unit quantity rules, and the `REQUIRES_NOTES` set (`exit:inventory_adjustment`, `exit:other`). <!-- sdd-owner: implementation -->
- [x] Read `src/lib/stores.ts` (`SettingsResponse` / `SettingsUpdate`) and `src-tauri/src/dto/stores.rs` plus the settings IPC validator to confirm the new `scanner_fefo_policy` and `close_behavior` fields follow the existing snake_case wire + key-value `app_settings` row pattern (the same pattern used for `language` and `theme`). <!-- sdd-owner: implementation -->
- [x] Read `src/App.svelte` (tab union, `tabs` array, mobile overflow) to confirm the Scanner tab entry point and the `aria-current="page"` pattern to reuse. <!-- sdd-owner: implementation -->
- [x] Read `src-tauri/Cargo.toml` and `src-tauri/src/lib.rs` to determine whether the **built-in Tauri tray API** (`tauri::tray::TrayIconBuilder` or the v2 runtime equivalent) is available without adding `tauri-plugin-tray`. The implementation MUST prefer the built-in API; `tauri-plugin-tray` is only added as a dependency if the build proves the built-in API is unavailable. <!-- sdd-owner: implementation -->
- [x] Read `src-tauri/capabilities/default.json` to confirm the minimum-permission list that tray + window-lifecycle additions will need. <!-- sdd-owner: implementation -->
- [x] Read `src/components/ConfigurationPage.svelte` (language selector block) to capture the optimistic-update + rollback-on-failure pattern the new FEFO and close-behavior selectors MUST mirror. <!-- sdd-owner: implementation -->
- [x] Produce a short exploration note inside this change folder (e.g. `explore.md` under `openspec/changes/scanner-quick-operations/`) recording findings and listing the exact files each subsequent slice will touch. <!-- sdd-owner: implementation -->

## Slice 2 — Backend foundation (PR 1): `resolve_scanner_code` + settings persistence

Goal: ship the backend command that resolves a scanned value into `LotMatch`, `ProductMatch`, or `Unknown`, and persist the FEFO policy and close-behavior settings in the same slice so PR 1 carries a coherent backend contract that PR 2 and PR 3 can build against.

- [x] Add `pub enum FefoPolicy { SuggestFefo, RequireFefo, ManualLotChoice }` and `pub enum CloseBehavior { MinimizeToTray, ExitApplication }` in `src-tauri/src/dto/stores.rs` (or a sibling `settings_enums.rs`) with `serde(rename_all = "snake_case")` matching the existing language / theme wire shape. <!-- sdd-owner: implementation -->
- [x] Extend `SettingsResponse` with `pub scanner_fefo_policy: FefoPolicy` and `pub close_behavior: CloseBehavior` (both non-optional); extend `SettingsUpdate` with the matching `Option<...>` fields in `src-tauri/src/dto/stores.rs`. <!-- sdd-owner: implementation -->
- [x] Add `get_scanner_fefo_policy(pool) -> FefoPolicy` and `set_scanner_fefo_policy(pool, value)` helpers in `src-tauri/src/db/repositories/settings.rs`, defaulting to `SuggestFefo` when the row is absent or unrecognised (mirror the `get_language_setting` fallback pattern). <!-- sdd-owner: implementation -->
- [x] Add `get_close_behavior(pool) -> CloseBehavior` and `set_close_behavior(pool, value)` helpers in the same file, defaulting to `MinimizeToTray`. <!-- sdd-owner: implementation -->
- [x] Update `get_settings` so the response carries the two new fields with their fallbacks applied. <!-- sdd-owner: implementation -->
- [x] Branch on `input.scanner_fefo_policy` and `input.close_behavior` in `services/settings.rs::update_settings` so partial updates do not touch the other keys; reject values outside the enum sets at the IPC boundary in `commands/stores.rs::update_settings` with a `CommandError::Validation` translated through the existing `user_message` table. The persisted row MUST stay untouched on rejection. <!-- sdd-owner: implementation -->
- [x] Add `dto/scanner.rs` types for `ScannerResolveResult { LotMatch { lot, product }, ProductMatch { product, lots }, Unknown { scanned_value } }`, using the already exported UI-facing response DTOs for `lot` and `product` rather than introducing partial duplicate structs. <!-- sdd-owner: implementation -->
- [x] Implement `commands::scanner::resolve_scanner_code(pool, input)` in a new `src-tauri/src/commands/scanner.rs` module. The command MUST (a) trim the input value and reject empty with `CommandError::Validation`, (b) try exact `expiry_lots.batch_code` match scoped to the active store and `status = 'active'`, (c) fall back to exact `product_barcodes.barcode` match for **active products only** (`is_active = true`), (d) fall back to exact `products.sku` match for **active products only**, (e) return `Unknown { scanned_value }` with the trimmed value preserved verbatim when none match. <!-- sdd-owner: implementation -->
- [x] When the lot-code match returns the lot, look up the parent product in the same call and return `LotMatch { lot, product }`. <!-- sdd-owner: implementation -->
- [x] When the product match returns the product, query the active lots for that product in the same store ordered by `(expiry_date ASC, created_at ASC, id ASC)` and return `ProductMatch { product, lots }` in FEFO order. <!-- sdd-owner: implementation -->
- [x] Register `commands::scanner::resolve_scanner_code` in the `invoke_handler` chain in `src-tauri/src/lib.rs`. <!-- sdd-owner: implementation -->
- [x] Behaviour tests in `db/repositories/settings.rs`: missing rows return the defaults, partial update of only one of the new keys preserves the others, invalid values are rejected, and the response shape carries the fields under both `en` and `es` locale tags. <!-- sdd-owner: implementation -->
- [x] Behaviour tests in `services/products.rs` or a new `services/scanner.rs`: lot-code wins over barcode, barcode wins over SKU, lot-code is scoped to the active store, archived-product barcode does not match, unknown returns `Unknown` with the trimmed value preserved. <!-- sdd-owner: implementation -->
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` and confirm the scanner lookup and settings tests are green. <!-- sdd-owner: implementation -->

## Slice 3 — Scanner tab UI (PR 2): modes, lookup, Sale / Registration / Stock-out flows

Goal: ship `ScannerPage.svelte`, integrate it into the navbar, and wire the three mode flows against the new `resolve_scanner_code` IPC and the existing `create_lot_movement` / `create_expiry_lot` / `create_product` commands.

- [x] Add `ScannerPage.svelte` under `src/components/` with three mode tabs (`Sale`, `Registration`, `Stock-out`) and a single primary input that submits on `Enter` and clears after a successful resolution. <!-- sdd-owner: implementation -->
- [x] Implement a `400 ms` debounce inside the ScannerPage so a single physical scan never produces two downstream side effects, matching the spec's `rapid double-scan is debounced` scenario. <!-- sdd-owner: implementation -->
- [x] Persist the active mode in component-local state and keep the resolved scan, lot selection, and quantity across mode switches (per the `switching modes preserves the in-progress scan` scenario). <!-- sdd-owner: implementation -->
- [x] Add `src/lib/scanner.ts` as the typed wrapper around `resolve_scanner_code`, then call it on every committed scan; map the discriminated result to the mode-specific UI states. <!-- sdd-owner: implementation -->
- [x] **FEFO direct lot-code bypass rule**: when the scanner resolves to `LotMatch`, do NOT apply the FEFO policy to select a different lot. The scanned lot is the selected lot, no lot picker is rendered, and the FEFO policy (even `require_fefo`) MUST NOT override the user's scanned lot identity. The translated `require_fefo` notice may still render, but it MUST NOT replace the scanned lot. <!-- sdd-owner: implementation -->
- [x] Sale mode: default quantity to `1`, render an editable quantity input with integer-unit-aware `step` / `min`, render the FEFO-driven lot picker (or the read-only chip when `require_fefo`), render a `Confirm` button disabled until lot + quantity are valid, call `create_lot_movement({ kind: "exit:sale", lot_id, quantity, source_location_id })`, clear the form on success, and render a translated success notice that includes the SKU and the sold quantity. <!-- sdd-owner: implementation -->
- [x] Registration mode, `LotMatch` / `ProductMatch`: open the existing lot creation flow with `product_id`, active store, and the scanned value as a candidate `batch_code` (preserved verbatim — no uppercase, lowercase, or whitespace normalization); on submit call `create_expiry_lot` and confirm a brand-new `expiry_lots` row plus its own `entry:initial` movement is created. The Registration flow MUST NOT auto-add the registered quantity to any existing lot in v1. <!-- sdd-owner: implementation -->
- [x] Registration mode, `Unknown`: open the quick product creation flow with `SKU` and `Barcode` fields empty and a small `Use as SKU` / `Use as barcode` affordance that routes the trimmed scanned value into exactly one slot; on submit call `create_product` then `add_product_barcode_on_create` (the typed wrapper introduced for the dashboard scan-search slice). <!-- sdd-owner: implementation -->
- [x] Stock-out mode: render a reason picker that lists the **seven non-sale** v1 exit reasons (`exit:waste`, `exit:expired`, `exit:damaged`, `exit:internal_consumption`, `exit:return_to_supplier`, `exit:inventory_adjustment`, `exit:other`); `exit:sale` MUST NOT appear in the picker; require notes when the reason is `exit:inventory_adjustment` or `exit:other`; call `create_lot_movement` with the selected kind, lot, source location, and quantity; render a translated success notice that includes the SKU, the reason label, and the removed quantity. <!-- sdd-owner: implementation -->
- [x] Gate every confirm-driven IPC call: re-disable the `Confirm` button while the call is in flight, surface the translated backend error in an inline error slot on failure, keep the form state on failure, clear the form only on success (per the `IPC failure keeps the form state` scenario). Stock MUST NOT mutate before the user activates `Confirm`. <!-- sdd-owner: implementation -->
- [x] Render the unavailable state ("create the first store first") when `hasStore()` returns `false`, and disable the scanner input. <!-- sdd-owner: implementation -->
- [x] Add `Scanner` to the `tabs` array and the `Tab` union in `src/App.svelte`, render the `<ScannerPage />` block in the tab switch, and keep the existing mobile overflow dropdown in sync. <!-- sdd-owner: implementation -->
- [x] Add i18n keys under `$LL.scanner.*` (and the sub-namespaces `sale`, `registration`, `stockOut`, `modes`, `reasons`, `confirm`, `errors`) in both `src/i18n/en/index.ts` and `src/i18n/es/index.ts`, preserving the parity invariant enforced by the existing `typesafe-i18n` build step. <!-- sdd-owner: implementation -->
- [x] Run `npx svelte-check --workspace . --threshold error` and `npm run build`; resolve any missing-key diagnostics emitted by `typesafe-i18n`. <!-- sdd-owner: implementation -->

## Slice 4 — Configuration surface (PR 3a): FEFO + close-behavior selectors

Goal: expose the two new settings in Configuration with the existing selector pattern (optimistic update + rollback-on-failure).

- [x] Add the `FefoPolicy` and `CloseBehavior` types (and the snake_case wire aliases) to `src/lib/stores.ts` and re-export them from there. <!-- sdd-owner: implementation -->
- [x] Add the i18n keys under `$LL.configuration.scannerFefoPolicy.*` and `$LL.configuration.closeBehavior.*` in both locales. <!-- sdd-owner: implementation -->
- [x] Add two new sections in `src/components/ConfigurationPage.svelte`: one for the FEFO policy (three-option selector matching the language selector pattern) and one for the close behavior (two-option selector). <!-- sdd-owner: implementation -->
- [x] Each selector MUST optimistically update on change, call `updateSettings({ scanner_fefo_policy } | { close_behavior })`, roll back on IPC failure, and surface a translated "could not save" inline error in the same pattern used by the existing language selector. Partial updates MUST NOT touch unrelated settings (`last_selected_store_id`, `require_initial_location_on_lot_create`, `language`, `theme`). <!-- sdd-owner: implementation -->
- [x] Run `npx svelte-check --workspace . --threshold error` and confirm no missing-key diagnostics. <!-- sdd-owner: implementation -->

## Slice 5 — System tray and close-window lifecycle (PR 3b)

Goal: install the tray icon and the `WindowEvent::CloseRequested` handler so the persisted close-behavior setting takes effect from the very first close attempt.

- [x] In `src-tauri/src/lib.rs` (or a new `lifecycle.rs` module), use the **built-in Tauri tray API** (e.g. `tauri::tray::TrayIconBuilder` or the v2 runtime equivalent) to build the tray icon on `setup` with two menu entries — `Restore` and `Quit` — bound to handlers that call `window.show()` + `window.set_focus()` and `app.exit(0)` respectively. Only add `tauri-plugin-tray` as a dependency in `src-tauri/Cargo.toml` if the build proves the built-in API is unavailable. <!-- sdd-owner: implementation -->
- [x] Update `src-tauri/capabilities/default.json` to grant the minimum permissions required by the tray and the new window-lifecycle commands. <!-- sdd-owner: implementation -->
- [x] Install a `WindowEvent::CloseRequested` handler that reads the persisted `close_behavior` value (defaulting to `minimize_to_tray`) BEFORE installing the handler so the configured behaviour is in effect from the very first close attempt. When the value is `minimize_to_tray`, call `api.prevent_close()` and `window.hide()`; when the value is `exit_application`, the handler MUST NOT call `prevent_close()` so the existing Tauri shutdown path runs. Switching the value MUST take effect on the next close-window event; in-flight windows MUST NOT have their behaviour retroactively changed mid-session. <!-- sdd-owner: implementation -->
- [x] Add a defensive fallback for platforms without tray support per the `close on a platform without tray support fails safely` scenario: detect the capability failure, log at `warn` level, and fall back to a normal exit so the user is not stranded with a hidden process they cannot restore. The fallback MUST NOT persistently rewrite the user's `close_behavior` setting. <!-- sdd-owner: implementation -->
- [ ] Manual smoke — close-window on Windows + Linux: with `minimize_to_tray` the window hides, the tray icon remains, and `Restore` brings the window back; with `exit_application` the process exits cleanly and no tray icon remains; on the unsupported-platform fallback the process exits and a `warn` log is recorded. <!-- sdd-owner: implementation -->

## Slice 6 — Verification

Goal: exercise the spec scenarios end-to-end so the change can be archived with a receipt.

- [ ] Cross-reference every `#### Scenario` block in `specs/caduxo-expiry-tracker/spec.md` against a manual smoke record (or an automated test where one exists) inside this change folder (e.g. `verify-report.md` under `openspec/changes/scanner-quick-operations/`); mark the scenario outcome as `pass`, `fail`, or `partial`. <!-- sdd-owner: implementation -->
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` and confirm the scanner lookup and settings tests pass. <!-- sdd-owner: implementation -->
- [ ] Run `npx svelte-check --workspace . --threshold error` and `npm run build` and confirm both are green with no missing-key diagnostics emitted by `typesafe-i18n`. <!-- sdd-owner: implementation -->
- [ ] Manual end-to-end smoke — Sale: scan a known lot code, confirm quantity `1`, confirm the `exit:sale` movement lands in `lot_movements`, confirm the lot total decremented. <!-- sdd-owner: implementation -->
- [ ] Manual end-to-end smoke — Registration: scan an unknown value, route it into the SKU slot, create the product, then create the lot with the scanned value as `batch_code`; confirm a brand-new lot row with its own `entry:initial` movement was written. <!-- sdd-owner: implementation -->
- [ ] Manual end-to-end smoke — Stock-out: scan a product barcode, choose `Damaged`, enter quantity `2`, confirm the `exit:damaged` movement lands, confirm `exit:sale` is not selectable in Stock-out mode. <!-- sdd-owner: implementation -->
- [ ] Manual end-to-end smoke — Settings: persist `require_fefo`, restart, confirm the Scanner tab picks up the policy on mount; persist `exit_application`, restart, close the window, confirm the process exits. <!-- sdd-owner: implementation -->
- [ ] Manual end-to-end smoke — Tray: with `minimize_to_tray`, close the window, restore from the tray, quit from the tray, confirm the process exits with status `0` after the database pool close. <!-- sdd-owner: implementation -->

---

## Parent actions (lifecycle gates and review)

- [x] Before PR 1 apply: confirm the chain strategy (`stacked-to-main` vs `feature-branch-chain`) and the slice boundaries listed above. The 800-line review budget is the hard ceiling per `openspec/config.yaml`; do not lower it without explicit user acceptance. <!-- sdd-owner: parent -->
- [ ] Open PR 1 (backend scanner lookup + settings persistence) against `main` once `cargo test --lib` is green; merge PR 1 only after the slice-2 tests pass. <!-- sdd-owner: parent -->
- [ ] Open PR 2 (frontend ScannerPage + i18n + navbar) stacked onto PR 1 once `npm run build` is green; if PR 2 alone exceeds 400 net changed lines, escalate via `deliveryStrategy: ask-on-risk` and split into PR 2a + PR 2b before merging. <!-- sdd-owner: parent -->
- [ ] Open PR 3 (ConfigurationPage selectors + tray + close-window handler + tray i18n keys) stacked onto PR 2 once the manual smoke records are filed; if PR 3 alone exceeds 400 net changed lines, escalate via `deliveryStrategy: ask-on-risk` before merging. <!-- sdd-owner: parent -->
- [ ] Start or reuse a bounded review for the change covering at minimum: (a) the lot-code → barcode → SKU priority in `resolve_scanner_code`, (b) the FEFO policy matrix (`suggest_fefo` / `require_fefo` / `manual_lot_choice`) across `LotMatch` vs `ProductMatch`, including the direct-lot-code bypass rule, (c) the Sale / Stock-out confirmation gate and the `confirm fires only one movement` invariant, (d) the Registration path that creates a new lot for an existing product without touching other lots, (e) the tray fallback on platforms without tray support, (f) the close-behavior persistence across restart, (g) the archived-product filter in scanner lookup, (h) the `exit:sale` rejection in Stock-out mode. <!-- sdd-owner: parent -->
- [ ] After all PRs land and the bounded review passes, archive the SDD artifacts (`proposal.md`, `specs/...`, this `tasks.md`, `design.md`, `explore.md`, `verify-report.md`) per the project's OpenSpec lifecycle and mark the change `applied`. <!-- sdd-owner: parent -->