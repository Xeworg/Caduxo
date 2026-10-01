# Integer Quantity Input Bug Fix

## Goal
Ensure integer-unit products cannot accept fractional quantities in user-facing numeric fields, while decimal-unit products continue to accept fractions. Preserve backend validation as the authoritative boundary.

## Scope
- Fix Scanner quantity inputs across sale/stock-out and lot/product match variants.
- Fix quantity-entry flows identified by the audit (`LotForm`, `ResolveQuantityDialog`) so input constraints and submit validation follow unit kind.
- Review existing integer/decimal numeric field patterns and add focused regression coverage where the repository's test setup supports it.
- Do not alter persistence schema or unrelated numeric fields without evidence they represent unit quantities.

## Acceptance
- Integer-unit quantity fields expose integer-appropriate input attributes and reject fractional values before IPC submission.
- Decimal-unit quantity fields still accept fractional values.
- Existing backend validation remains in place.
- Focused verification passes; report any unavailable or failing checks.

## Route and evidence
- Route: delegated direct implementation via `gentle-ai-worker`; multi-file write trigger, based on read-only audit findings.
- Branch: `fix/integer-quantity-inputs` from `main` at `f1e5a3c`.
- TDD configuration: not yet resolved; worker must inspect and report the effective mode and exact runner before tests.

## Tasks

### Task 1 — Implement unit-kind constraints and pre-submit fractional validation

**Status: DONE**

**Changes applied:**

#### `src/components/ScannerPage.svelte`
- Added `isFractionalForIntegerUnit` and `qtyAttrs` imports from `../lib/movementRules.js`.
- Added `activeProductUnitKind`, `isIntegerUnit`, and `_qtyAttrs` derived state derived from `activeProduct?.unit_type`.
- Fixed **four quantity `<input type="number">` elements** (Sale lot_match, Sale product_match, Stock-out lot_match, Stock-out product_match): replaced hardcoded `min="0" step="0.01" inputmode="decimal"` with `min={_qtyAttrs.min} step={_qtyAttrs.step} inputmode={_qtyAttrs.inputmode}`. When the product uses integer units, the browser now enforces `min=1 step=1 inputmode=numeric`; for decimal units, `min=0 step=0.01 inputmode=decimal` is preserved.
- Added `isFractionalForIntegerUnit` guard in `confirmSale()` before IPC — returns `integerQuantityError` locally.
- Added `isFractionalForIntegerUnit` guard in `confirmStockOut()` before IPC — same.
- Added fractional cases to `invalidSaleMessage()` and `invalidStockOutMessage()` — return `lotMovements.modal.integerQuantityError` for integer-unit products.

#### `src/components/LotForm.svelte`
- Added `isFractionalForIntegerUnit` import from `../lib/movementRules.js`.
- Added fractional guard in `submit()` after the `quantity > 0` check and before the alert-days validation. Catches programmatic form submissions that bypass the HTML `min`/`step` attributes. Reuses the existing `lotMovements.modal.integerQuantityError` i18n key.
- HTML `min`/`step` attributes were already adapted (`min={productUnitKind === "decimal" ? 0.01 : 1}` / `step={productUnitKind === "decimal" ? 0.01 : 1}`) — no change needed.

#### `src/components/ResolveQuantityDialog.svelte`
- Added `isFractionalForIntegerUnit` and `qtyAttrs` imports from `../lib/movementRules.js`; added `UnitKind` type import from `../lib/products.js`.
- Added optional `unitType: UnitKind | null = null` prop with safe default (decimal treatment for legacy/uncatalogued products).
- Replaced hardcoded `min={0} step={1}` on the quantity `<Input>` with `min={_qtyAttrs.min} step={_qtyAttrs.step}` — now adapts to integer vs. decimal unit kind.
- Added `$: isIntegerUnit` and `$: _qtyAttrs` reactive state (Svelte 4 `$:` syntax to avoid activating runes mode in a file that already uses `$:` for `quantity`).
- Added `isFractionalForIntegerUnit` guard in `submit()` before IPC — returns `lotMovements.modal.integerQuantityError`.

**Test runner: none**
- `package.json` has no test runner configured (no vitest, jest, playwright, or mocha scripts).
- Existing `src/lib/lotDisplay.test.ts` documents expected behaviour as commented-out cases with a note explaining the runner is not wired.
- Structural regression review (below) is the substitute.

**Structural regression review** — existing patterns verified unchanged:
- `MoveStockModal`, `AdjustCountModal`, `RegisterExitModal`: already used `qtyAttrs` + `isFractionalForIntegerUnit` correctly; no changes made.
- `DistributionEditor`: already received `unitKind={productUnitKind}` and enforced fractional rows internally; no changes made.
- `LotForm.quantity` Input: `min`/`step` already adapted; guard added for programmatic submission path.
- Backend IPC functions (`createExpiryLot`, `createLotMovement`, etc.): unchanged — backend remains authoritative boundary.

**Verification:**
```
npm run check 2>&1 | tail -5
→ svelte-check found 0 errors and 0 warnings
```

### Task 2 — Verify remaining quantity fields

**Status: DONE**

All quantity-entry surfaces checked:
- Scanner Sale (lot_match + product_match): ✅ now uses `_qtyAttrs`
- Scanner Stock-out (lot_match + product_match): ✅ now uses `_qtyAttrs`
- LotForm quantity (create mode): ✅ HTML attrs correct + submit guard added
- ResolveQuantityDialog: ✅ now uses `_qtyAttrs` + submit guard
- MoveStockModal: ✅ was already correct
- AdjustCountModal: ✅ was already correct
- RegisterExitModal: ✅ was already correct
- DistributionEditor rows: ✅ already enforced fractional validation per row
- ProductForm quantity (product creation): ✅ not a lot quantity — out of scope
- AdjustCountModal `realQuantity`: ✅ was already correct (`step={isIntegerUnit ? 1 : 0.01}`)

**Additional coverage pass — ProductDetailPage wiring:**
- `src/components/ProductDetailPage.svelte` wires `unitType={resolvingLot.unit_type ?? null}`
  to `ResolveQuantityDialog` so the dialog receives the actual lot unit kind when opened
  from the product detail view. `ExpiryLotResponse.unit_type` carries the kind from the
  product catalog link; the null-coalesce handles legacy lots that predate the field.

## Follow-up: verification findings (parent-authorised)

**Changes applied:**

1. **ScannerPage: `canConfirmSale` / `canConfirmStockOut` now gate fractional integer quantities.**
   Added `isFractionalForIntegerUnit(quantity, isIntegerUnit)` to both derived gates so the
   confirm buttons are disabled (not just guarded on click) when the user has typed a
   fractional value for an integer-unit product. The guards in `confirmSale`/`confirmStockOut`
   remain as belt-and-suspenders for any code path that bypasses the button state.

2. **LotForm: HTML `min`/`step` now treats `null` (legacy products) as decimal — consistent
   with the submit guard.**
   Previous condition: `productUnitKind === "decimal" ? 0.01 : 1` — `null` fell to the
   `else` branch (integer, min=1), contradicting the submit guard which treats `null` as decimal.
   Fixed to: `productUnitKind !== "integer" ? 0.01 : 1` — `null` and `"decimal"` both
   resolve to min=0.01/step=0.01; `"integer"` resolves to min=1/step=1. Equivalent to
   inlining `qtyAttrs(isIntegerUnit)` without requiring a new import.

## Version metadata
- Approved issue: #40
- Planned version: 0.2.2 (SemVer bug-fix patch)

## Work-unit evidence
- Commit: `8ee0e78` (`fix(scanner): enforce integer quantity inputs`)
- Version bump: 0.2.1 → 0.2.2 (package.json, package-lock.json, src-tauri/Cargo.toml, src-tauri/Cargo.lock [caduxo entry], src-tauri/tauri.conf.json)
- Verification: `npm run check` → 0 errors, 0 warnings
- Status: task 1 complete; task 2 complete; ProductDetailPage wiring done; follow-up done
