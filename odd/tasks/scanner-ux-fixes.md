# Scanner UX fixes

## Goal
Fix scanner and lot-detail UX issues found during manual testing.

## Tasks

- [x] Map the reported Scanner UX gaps and the lot detail modal display bug.
- [x] Fix Scanner behavior when a scanned product has no available lots: show a clear translated message and product context instead of a silent/blank state.
- [x] Fix Scanner result context so the user can see what product was found, not just a SKU/id.
- [x] Change the Scanner primary lookup button label from generic "Save"/"Guardar" to a search/scan-specific label.
- [x] Fix lot detail modal/calendar detail display so product/store/location descriptions or names are shown instead of raw ids.
- [x] Run focused validation and record evidence.

## Approach
- Preserve uncommitted scanner-store-selection work in `ScannerPage.svelte`,
  `StoresPage.svelte`, i18n files, and `Cargo.lock`.
- Scope fix to the four reported issues only.
- Lot detail modal: reuse the dashboard row context (`DashboardLotRow`)
  already in scope when the modal opens, instead of adding backend
  schema changes or a fresh IPC round-trip.
- Scanner empty-lots branch: add a third resolved-state branch in both
  Sale and Stock-out panels that renders an `Alert` with the product
  description + SKU plus a translated "no active lots" message.
- Scanner product context: render `activeProduct.description` next to
  the SKU for Sale, Stock-out, and direct lot-match flows.
- Scanner lookup button: introduce `scanner.input.lookupLabel` i18n key
  in both locales; remove the use of `common.save`.

## Evidence

### Issue 1 — Scanner no-lots message
- Before: when `ScannerProductMatch.lots.length === 0`, Sale and
  Stock-out panels rendered nothing (the `resolvedLot` derived is
  `null` and the only other branches were `unknown` or
  `activeProduct && resolvedLot`). User saw a blank panel with no
  explanation.
- After: a new `{:else if resolved?.match_type === "product_match" &&
  resolved.lots.length === 0 && activeProduct}` branch renders an
  `Alert variant="info"` that names the product (description + SKU)
  and shows `$LL.scanner.noLotsAvailable()`.
- Files: `src/components/ScannerPage.svelte`,
  `src/i18n/en/index.ts`, `src/i18n/es/index.ts`,
  `src/i18n/i18n-types.ts` (regenerated).

### Issue 2 — Scanner result context
- Before: Sale/Stock-out `selectedProduct` line rendered only
  `activeProduct.sku`. Direct `lot_match` resolved state rendered only
  an info alert, with no product/lot context visible.
- After: Sale/Stock-out `selectedProduct` line renders
  `{activeProduct.description} ({activeProduct.sku})`. `lot_match`
  renders a new resolved-summary block that names the product
  (description + SKU) and the lot (expiry + batch code) before the
  existing FEFO-bypass hint.
- Files: `src/components/ScannerPage.svelte`.

### Issue 3 — Scanner lookup button label
- Before: scanner lookup button used `{$LL.common.save()}` ("Save" /
  "Guardar"), which is a mutation verb that does not match the
  lookup intent.
- After: scanner lookup button uses `{$LL.scanner.input.lookupLabel()}`
  ("Look up" / "Buscar"). New key added in both locales; type
  regenerated via `npm run i18n:generate`.
- Files: `src/i18n/en/index.ts`, `src/i18n/es/index.ts`,
  `src/i18n/i18n-types.ts`, `src/components/ScannerPage.svelte`.

### Issue 4 — Calendar lot detail raw ids
- Before: lot detail modal snippet rendered
  `detailLot.product_id`, `detailLot.store_id`, and
  `detailLot.location_id` verbatim — opaque ids like `prod-...` and
  `loc-...`.
- After: the modal now reuses the `DashboardLotRow` already in scope
  when `openLot(row)` is called (`description`, `sku`, `store_name`,
  `location_name`) and falls back to the raw id only when the row
  context is missing. Implementation: a `detailRow` `$state` slot
  holds the originating row; `onLotCancel` clears it alongside
  `detailLot`.
- Files: `src/components/CalendarPage.svelte`.

## Validation

- `npm run check` — svelte-check passes with zero errors on the
  touched files. New i18n keys type-check.
- `npm run i18n:generate` — regenerated `i18n-types.ts` to mirror the
  new `en`/`es` keys.
- Manual code review confirmed no regression in mutation /
  confirmation paths; the new branches sit alongside the existing
  `else if resolved && activeProduct && resolvedLot` branch, which
  still gates confirms.
- No backend / Rust files were touched, so no extra focused Rust
  command is required.

## Risks / Follow-ups

- The lot-detail modal still shows raw id fallback only when the row
  context is missing. With the current flow `detailRow` is always set
  alongside `detailLot`, but the fallback is kept so a future
  refactor that opens the modal without a row context cannot
  regress silently.
- The Scanner empty-lots branch does not offer a next-step CTA yet
  (e.g. "Go to Registration to create a lot"). A follow-up PR could
  add a cross-mode shortcut if product asks for it.