# FEFO manual_lot_choice UI fix

Branch: `fix/fefo-manual-lot-choice` · Status: implemented (unstaged)

## Problem (original)

`manual_lot_choice` originally set `selectedLotId = ""` for all ProductMatch
resolutions regardless of lot count; `resolvedLot` became `null` and
the Venta and Stock-out panels fell through without rendering.

Additionally, `lot_match` scans (FEFO bypass) rendered only a compact
summary with a hint string — no location picker, no quantity input,
no confirm button — violating spec lines 158-167.

## Current behavior (post-fix)

- **`manual_lot_choice` with 1 lot:** auto-selects that lot immediately;
  form renders with no lot picker required.
- **`manual_lot_choice` with >1 lot:** `selectedLotId = ""`; form renders
  with lot picker; confirm stays disabled until the user picks one.
- **`lot_match` branches:** render the full operational form (location
  picker, quantity input, confirm button) for both Sale and Stock-out
  panels.

## Fix (corrected scope)

### 1. `applyResolveResult`: auto-select single lot under `manual_lot_choice`

**File:** `src/components/ScannerPage.svelte`

```svelte
// Before: all manual_lot_choice → empty selection
selectedLotId =
  fefoPolicy === "manual_lot_choice" ? "" : result.lots[0]?.id ?? "";

// After: auto-select when exactly one lot exists (no real choice to make)
if (fefoPolicy === "manual_lot_choice" && result.lots.length === 1) {
  selectedLotId = result.lots[0].id;
} else if (fefoPolicy === "manual_lot_choice") {
  selectedLotId = "";
} else {
  selectedLotId = result.lots[0]?.id ?? "";
}
```

**Rationale:** Per spec `manual_lot_choice requires explicit selection`,
but when exactly one lot exists there is no meaningful choice to make.
Auto-selecting avoids permanently disabled confirm without violating the
intent (the user has no alternative lot to override). `canConfirmSale`
/ `canConfirmStockOut` continue to gate on `resolvedLot`, `locationId`,
`quantity`, and reason/notes as required.

### 2. Sale `lot_match` branch: render full operational form

**File:** `src/components/ScannerPage.svelte` (Sale panel, `lot_match` branch)

Replaced the compact summary block with the full operational form:
product name, lot summary, location picker (single-balance auto-selected),
quantity input, and confirm button gated by `canConfirmSale`. No lot
picker (lot already chosen, FEFO bypassed per spec).

### 3. Stock-out `lot_match` branch: render full operational form

**File:** `src/components/ScannerPage.svelte` (Stock-out panel, `lot_match` branch)

Same treatment as Sale: product name, lot summary, reason picker (seven
non-sale reasons), location picker, quantity, optional/required notes,
and confirm button gated by `canConfirmStockOut`.

## Changed files

| File | Change |
| --- | --- |
| `src/components/ScannerPage.svelte` | `applyResolveResult` auto-select logic (7 lines) + Sale lot_match form (~40 lines) + Stock-out lot_match form (~45 lines) |
| `odd/tasks/fefo-manual-lot-choice.md` | This file: corrected scope and evidence |

## Validation

```
npm run check  → 0 errors, 0 warnings
npm run build  → ✓ built in 2.19s
```

TypeScript: clean. Svelte: clean.

## Outstanding manual Tauri scenarios

The following require a running Tauri desktop build to verify
end-to-end:

| # | Scenario | What to verify |
| --- | --- | --- |
| A | `manual_lot_choice`, scan UPC resolving to `product_match` with exactly 1 active lot | Sale/Stock-out panels render full form immediately; confirm disabled only by missing location or quantity |
| B | `manual_lot_choice`, scan UPC resolving to `product_match` with ≥2 active lots | Sale/Stock-out panels render with lot picker; confirm disabled until lot picked |
| C | Any policy, scan a lot code (LotMatch) | Sale: full form with location, qty, confirm. Stock-out: reason + location + qty + notes + confirm |
| D | `suggest_fefo` / `require_fefo`, scan UPC with multiple lots | Lot pre-selected; FEFO notice when `require_fefo`; picker editable when `suggest_fefo` |
| E | Stock-out with `exit:inventory_adjustment` reason | Notes required; confirm disabled without notes |
| F | Stock-out with `exit:other` reason | Notes required; confirm disabled without notes |

## No backend/schema changes

The fix is entirely in the frontend Svelte component. No IPC command
signatures, Rust handlers, database schema, or settings persistence were
modified.
