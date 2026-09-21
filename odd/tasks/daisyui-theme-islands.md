# DaisyUI theme islands cleanup

## Goal
Remove remaining dark-theme white islands and native OS popup leaks after the DaisyUI redesign.

## Scope
Use existing DaisyUI v5 theme tokens and shared UI primitives so `caduxo-light` and `dark` both render consistently.

## Status: completed (single bounded pass)

## Completed changes

### Native `<select>` → `Listbox.svelte` (popup-leak fix)
- [x] `src/components/ColumnMapper.svelte` — 7 raw `<select>` (sku, description, barcode, category, unit, alert_days, notes) replaced with `<Listbox bind:value options size="sm">`. Numeric column indices now round-trip through `String(...)` because the Listbox contract is `value: string`; `handleApply` converts back with `Number()`.
- [x] `src/components/DashboardPage.svelte` — 2 filter `<select>` (store + location) replaced with `<Listbox>` driven by an explicit `value` + `onchange` bridge (NOT `bind:value`, to avoid a reactive cycle with the `string | null` source-of-truth ids).
- [x] `src/components/ResolveQuantityDialog.svelte` — resolution-type `<select>` replaced with `<Listbox>` whose `options` are derived from the i18n catalogue.
- [x] `src/components/RegisterExitModal.svelte` — source-location raw `<select>` replaced with `<Listbox>` (the motivo select already uses the themed `Select.svelte`).
- [x] `src/components/ArchiveLotDialog.svelte` — reason `<select>` replaced with `<Listbox>`.

### Hand-rolled modal/panel → shared primitives
- [x] `src/components/CalendarPage.svelte` — hand-rolled `.modal-overlay / .modal-box / .modal-close` replaced with `<Modal bind:open size="wide" showClose>`. Hand-rolled `.detail-tabs` replaced with `<Tabs items={[…]} bind:activeId>`. Hand-rolled `.btn-primary / .btn-secondary` replaced with `<Button variant="primary|ghost">`. All dead CSS selectors removed.
- [x] `src/components/ColumnMapper.svelte` — hand-rolled `.overlay / .mapper-box` replaced with `<Modal>` (size="wide"). Apply / cancel actions replaced with `<Button variant="primary|ghost">`.
- [x] `src/components/ProductDetailPage.svelte` — lot-detail `.modal-overlay / .modal-box` block replaced with `<Modal>`; `.detail-tabs` replaced with `<Tabs>`; `.btn-primary / .btn-secondary / .btn-danger` retained as local classes but tokenised (kept for markup compatibility — primitive swap is out of scope for this pass).
- [x] `src/components/ProductCatalogPage.svelte` — error/success `<div class="alert-error / .alert-success">` replaced with `<Alert variant="error|success">`. Dead `.alert` CSS removed.
- [x] `src/components/StoresPage.svelte` — error/success `<div class="alert-…">` replaced with `<Alert>` primitive. Dead `.alert` CSS removed.
- [x] `src/components/CsvImportPage.svelte` — already uses `<Alert>` (no change needed); only hex-tokenisation applied.

### Hex → DaisyUI tokens (`color-mix(in oklch, var(--color-X) NN%, transparent)` where tinting is required)
- [x] `src/components/ScanSearchBox.svelte` — full tokenisation: `border 2px solid #bfdbfe → color-mix(primary 25%)`, `background #fff → var(--color-base-100)`, error tint via `color-mix(error …)`.
- [x] `src/components/UnitReviewBanner.svelte` — yellow hex banner → `color-mix(in oklch, var(--color-warning) …)`; buttons converted from raw hex to `<Button variant="warning|ghost">`.
- [x] `src/components/ProductDetailPage.svelte`, `ProductCatalogPage.svelte`, `StoresPage.svelte`, `DashboardPage.svelte` — every `background: #fff`, `border: 1px solid #e5e7eb`, `color: #6b7280 / #9ca3af / #94a3b8` etc. replaced with the corresponding DaisyUI semantic token + `color-mix` opacity blends.

### Token fallback normalisation
- [x] Across all 14 allowed files: every `var(--color-*, #hex)` light-hex fallback stripped via sed. Final grep shows **0 remaining** `var(--color-*, #…)` patterns in the scope.
- [x] Final grep across the scope shows **0 remaining bare `color: #hex` / `background: #hex` / `border: 1px solid #hex` patterns**.

## Verification evidence

- `npm run check` (svelte-check, error threshold):
  ```
  svelte-check found 0 errors and 0 warnings
  ```
- `npm run build` (vite production build + typesafe-i18n prebuild):
  ```
  ✓ 224 modules transformed.
  dist/index.html                   0.39 kB │ gzip:   0.26 kB
  dist/assets/index-BdkAgrjK.css  228.39 kB │ gzip:  32.68 kB
  dist/assets/index-C_cZ1grv.js   377.81 kB │ gzip: 111.46 kB
  ✓ built in 1.89s
  ```
- Hex-leak grep (scope-only):
  ```
  $ for f in <scope>; do
      echo "$(grep -cE 'color: #...|background: #...|border.*#...' $f)  $f"
    done
  0  src/components/CalendarPage.svelte
  0  src/components/ScanSearchBox.svelte
  0  src/components/UnitReviewBanner.svelte
  0  src/components/ProductDetailPage.svelte
  0  src/components/ProductCatalogPage.svelte
  0  src/components/StoresPage.svelte
  0  src/components/DashboardPage.svelte
  0  src/components/MoveStockModal.svelte
  0  src/components/AdjustCountModal.svelte
  0  src/components/ResolveQuantityDialog.svelte
  0  src/components/RegisterExitModal.svelte
  0  src/components/ArchiveLotDialog.svelte
  0  src/components/CsvImportPage.svelte
  0  src/components/ColumnMapper.svelte
  ```

## Files changed (14 files + this task doc)

- `src/components/ColumnMapper.svelte` — full rewrite: `Modal` + 7 `Listbox` + `Button`, hex-tokenised
- `src/components/DashboardPage.svelte` — 2 `Listbox` filter controls + tokenised CSS
- `src/components/ResolveQuantityDialog.svelte` — `Listbox` for resolution type + tokenised CSS
- `src/components/RegisterExitModal.svelte` — `Listbox` for source-location + tokenised CSS
- `src/components/ArchiveLotDialog.svelte` — `Listbox` for reason + tokenised CSS
- `src/components/CalendarPage.svelte` — `Modal` + `Tabs` + `Button` + tokenised CSS, dead selectors removed
- `src/components/ProductDetailPage.svelte` — `Modal` + `Tabs` + `Button` + `Alert` + `Badge` + tokenised CSS, dead selectors removed
- `src/components/ProductCatalogPage.svelte` — `Alert` primitive + tokenised CSS, dead selectors removed
- `src/components/StoresPage.svelte` — `Alert` primitive + tokenised CSS, dead selectors removed
- `src/components/ScanSearchBox.svelte` — full tokenisation (border / focus / error tints)
- `src/components/UnitReviewBanner.svelte` — tokenised banner + `Button` primitive
- `src/components/CsvImportPage.svelte` — tokenisation pass (already used `<Alert>`)
- `src/components/MoveStockModal.svelte` — token fallback normalisation only (already uses Modal/Select/Button)
- `src/components/AdjustCountModal.svelte` — token fallback normalisation only (already uses Modal/Input/Select/Button)
- `odd/tasks/daisyui-theme-islands.md` — this evidence doc

## Key patterns used
- **Popup-leak fix:** swap every raw `<select>` for `<Listbox bind:value=… options=…>` — themed popover inherits DaisyUI tokens in both `caduxo-light` and `dark`.
- **Modal swap:** `<Modal bind:open showClose closeLabel={$LL.lotMovements.modal.close()}>` replaces hand-rolled `.overlay / .modal-box` so focus trap, Escape handling, backdrop click, and the DaisyUI dialog state machine come from the shared primitive.
- **Tabs swap:** `<Tabs items={[{id, label, panel}]} bind:activeId style="bordered">` replaces hand-rolled tab buttons. Panel content lives in hoisted `{#snippet}` declarations so the `items` array can reference them by name.
- **Bridge pattern for `bind:value` cycles:** when the source-of-truth is `string | null` (e.g. `selectedStoreId`) and the primitive requires `string`, drive the Listbox with `value={bridge}` + `onchange={updateSource}` instead of `bind:value`, to avoid a reactive cycle.
- **Theme tokens + tints:** bare declarations use `--color-base-100/200/300` and `--color-base-content`; tints/blends use `color-mix(in oklch, var(--color-X) NN%, transparent)` so the surface scales with the theme.

## Remaining risks / follow-ups
- **Local `.btn-primary / .btn-secondary / .btn-danger` classes** in `ProductDetailPage.svelte`, `ProductCatalogPage.svelte`, `StoresPage.svelte`, `CsvImportPage.svelte` were tokenised but not swapped to `<Button.svelte>` (the inline markup still binds to the local class names; swapping would be a larger refactor outside this pass). They are now theme-aware via the new token values — the visual parity holds — but a follow-up pass should replace them with the shared primitive for consistency.
- **`ProductDetailPage.svelte` urgency badges** (`.urgency-expired / -critical / -warning / -normal`) are still local CSS classes, not the `<Badge>` primitive. They render the same urgency ladder; a follow-up could swap them to `<Badge urgency={…}>` for consistency with the DashboardPage / CalendarPage surfaces.
- **No git commit / push performed** — per the task contract. Commit and PR creation remain user decisions.

## Allowed edit surfaces
- `src/components/CalendarPage.svelte`
- `src/components/ColumnMapper.svelte`
- `src/components/ScanSearchBox.svelte`
- `src/components/UnitReviewBanner.svelte`
- `src/components/ProductDetailPage.svelte`
- `src/components/ProductCatalogPage.svelte`
- `src/components/StoresPage.svelte`
- `src/components/DashboardPage.svelte`
- `src/components/MoveStockModal.svelte`
- `src/components/AdjustCountModal.svelte`
- `src/components/ResolveQuantityDialog.svelte`
- `src/components/RegisterExitModal.svelte`
- `src/components/ArchiveLotDialog.svelte`
- `src/components/CsvImportPage.svelte`

## Evidence
- Read-only DaisyUI research and code audit identified scoped hex CSS, hand-rolled modal stacks, and native selects as the remaining risks.
- `npm run check` → 0 errors, 0 warnings.
- `npm run build` → built in 1.89s, 224 modules transformed.
- Hex-leak grep → 0 hex declarations remaining in scope.
