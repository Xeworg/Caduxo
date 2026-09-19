# Verify Report — caduxo-i18n-support — Lot movements residue slice

## Status

**PASS** — independent verification confirms the slice meets its stated goals.

## Executive summary

The parent-applied inline correction is sound and complete. The two components
(`LotMovementsPanel.svelte`, `ArchiveLotDialog.svelte`) now route their movement
kind and archive reason labels through `$LL.lotMovements.movementKinds.*`,
`$LL.lotMovements.exitReasons.*`, and `$LL.lotMovements.archive.reasons.*` in both
locales. EN/ES key parity is enforced by the typesafe-i18n generator and confirmed
by `npm run i18n:generate`. The authoritative toolchain (`tsc`, `svelte-check`,
`vite build`) is green end-to-end with exit code 0 in every command. The pi-lens
stale-snapshot caveat noted by the parent is documented and not load-bearing for
this slice.

## Verification commands and results

| # | Command | Exit | Evidence |
|---|---------|------|----------|
| 1 | `npm run i18n:generate` | 0 | `[typesafe-i18n] ... all files are up to date` — generator confirmed no missing-key drift between EN/ES |
| 2 | `npx tsc --noEmit` | 0 | no output, exit 0 — no TypeScript errors |
| 3 | `npx svelte-check --workspace . --threshold error` | 0 | `svelte-check found 0 errors and 0 warnings` |
| 4 | `npm run build` | 0 | `✓ 200 modules transformed`, `✓ built in 1.34s`, `dist/index.html 0.39 kB`, `dist/assets/index-6tgen3dX.js 264.60 kB` |

## Spec coverage

The verification scope is **residual UI key parity** under the existing
`Internationalisation` capability in
`openspec/changes/caduxo-i18n-support/specs/caduxo-expiry-tracker/spec.md`.

| Spec requirement (delta) | Coverage |
|--------------------------|----------|
| `typesafe-i18n Svelte 5 integration` — "no component MUST embed a hard-coded user-visible English or Spanish string outside of the translation tree" | The `getKindLabel` Spanish-only function in `src/lib/lot_movements.ts:108` is no longer imported by `LotMovementsPanel.svelte`; the new `getMovementKindLabel` reads exclusively from `$LL.lotMovements.*`. |
| `translation key parity between locales` — every key in `en.json` has a non-empty counterpart in `es.json` | `npm run i18n:generate` exits 0 with "all files are up to date", which is the typesafe-i18n parity gate. |

## Component-by-component findings

### 1. `src/components/LotMovementsPanel.svelte`

**Before (residue):** imported `getKindLabel` from `../lib/lot_movements.js`. That
function returned hardcoded Spanish strings
(`"Entrada inicial"`, `"Venta"`, `"Merma"`, `"Vencido"`, etc.) regardless of the
active locale, so English users saw mixed Spanish labels in the movement history.

**After (this slice):**
- Removed `getKindLabel` from the import list (line 8 in diff).
- Added `getMovementKindLabel(kind, direction?)` that branches on
  `inventory_adjustment` to render the increase/decrease variants, otherwise
  indexes a `Record<string, () => string>` table whose every value is a
  `$LL.lotMovements.movementKinds.*` or `$LL.lotMovements.exitReasons.*` function
  reference.
- Replaced the `{#each}` body's `getKindLabel(...)` call site with
  `getMovementKindLabel(...)`.

All 12 known kinds covered:
- `"entry:initial"` → `movementKinds.initialEntry`
- `transfer` → `movementKinds.transfer`
- `inventory_adjustment` (no direction) → `movementKinds.inventoryAdjustment`
- `inventory_adjustment` + `direction === "increase"` → `movementKinds.inventoryAdjustmentIncrease`
- `inventory_adjustment` + `direction === "decrease"` → `movementKinds.inventoryAdjustmentDecrease`
- `"exit:sale"` / `waste` / `expired` / `damaged` / `internal_consumption` / `return_to_supplier` / `inventory_adjustment` / `other` → `exitReasons.*`

Fallback `?? kind` keeps the UI safe if a future backend movement kind is added
without a corresponding key (the raw kind string renders until the key is added).

**No Spanish-only residue remains in rendered output.** Confirmed by
`grep -RIn 'Venta\|Merma\|Vencido\|Caducado\|Devolución' src/components/ src/App.svelte`
returning no matches.

### 2. `src/components/ArchiveLotDialog.svelte`

**Before (residue):** rendered `<option value={option.value}>{option.label}</option>`
in the reason `<select>`. `option.label` is the hardcoded English string from
`ARCHIVE_REASONS` in `src/lib/expiry_lots.ts` (e.g. `"Expired (unsold)"`,
`"Manufacturer recall"`).

**After (this slice):**
- Added `archiveReasonLabel(value: string): string` that returns the
  `$LL.lotMovements.archive.reasons.<Key>()` for every `ARCHIVE_REASONS` value
  (`expired_unsold`, `damaged`, `returned_to_supplier`, `recall`, `lost`,
  `internal_use`, `administrative`, `other`).
- Replaced the `<option>` body with `{archiveReasonLabel(option.value)}`.
- The remaining `label` field is still referenced in the `some()` predicate's
  type assertion `r: { value: string; label: string }` (line 34), which is a
  TypeScript shape guard — not UI rendering. This is acceptable; the `label`
  string itself never appears in rendered output.

**No `option.label` rendering remains.** Confirmed by
`grep -n 'option\.label' src/components/ArchiveLotDialog.svelte` returning no UI-rendering matches.

### 3. Locale tree parity (`src/i18n/en/index.ts` ↔ `src/i18n/es/index.ts`)

| Key | EN value | ES value |
|-----|----------|----------|
| `lotMovements.movementKinds.initialEntry` | "Initial entry" | "Entrada inicial" |
| `lotMovements.movementKinds.transfer` | "Transfer" | "Transferencia" |
| `lotMovements.movementKinds.inventoryAdjustment` | "Inventory adjustment" | "Ajuste de inventario" |
| `lotMovements.movementKinds.inventoryAdjustmentIncrease` | "Inventory adjustment (+)" | "Ajuste de inventario (+)" |
| `lotMovements.movementKinds.inventoryAdjustmentDecrease` | "Inventory adjustment (−)" | "Ajuste de inventario (−)" |
| `lotMovements.archive.reasons.expiredUnsold` | "Expired (unsold)" | "Caducado sin vender" |
| `lotMovements.archive.reasons.damaged` | "Damaged" | "Dañado" |
| `lotMovements.archive.reasons.returnedToSupplier` | "Returned to supplier" | "Devuelto al proveedor" |
| `lotMovements.archive.reasons.recall` | "Manufacturer recall" | "Retiro del fabricante" |
| `lotMovements.archive.reasons.lost` | "Lost / unaccounted" | "Perdido / no contabilizado" |
| `lotMovements.archive.reasons.internalUse` | "Internal use" | "Uso interno" |
| `lotMovements.archive.reasons.administrative` | "Administrative cleanup" | "Limpieza administrativa" |
| `lotMovements.archive.reasons.other` | "Other" | "Otro" |

`lotMovements.exitReasons.*` (sale / waste / expired / damaged /
internalConsumption / returnToSupplier / inventoryAdjustmentExit / other) were
already present in both locales from a prior pass — no diff needed; the
components only newly consume them via `getMovementKindLabel`. Confirmed by
reading the EN tree at lines 569-577 and ES tree at lines 567-575.

Parity is enforced by the generator (verified above) and reflected in the
hand-written `BaseTranslation` shape: every new key in EN has a matching key in
ES and vice versa.

### 4. Generated types (`src/i18n/i18n-types.ts`)

`i18n-types.ts` was regenerated as part of `npm run i18n:generate`. New types
appear at:
- `RootTranslation.lotMovements.movementKinds` — lines 1858-1878 (5 keys)
- `RootTranslation.lotMovements.archive.reasons` — lines 2142-2176 (8 keys)
- `TranslationFunctions.lotMovements.movementKinds` — lines 5011-5031 (5 keys)
- `TranslationFunctions.lotMovements.archive.reasons` — lines 5282-5316 (8 keys)

`lotMovements.exitReasons` types already existed (lines 2044, 5191) from the
prior pass. The 13 newly generated key types match the 13 new keys added to
`en/index.ts` and `es/index.ts`.

## LSP stale-snapshot handling

The pi-lens embedded LSP diagnostic server holds a snapshot of `i18n-types.ts`
that is not invalidated when the generator rewrites the file. This is a
session-persistent caveat documented across all prior apply-progress slices.
For this slice:

- After `npm run i18n:generate` reports "all files are up to date", the
  authoritative pipeline (`tsc --noEmit`, `svelte-check`, `vite build`) all exit
  0. The actual on-disk `i18n-types.ts` carries the new keys, and the
  TypeScript compiler re-reads it on each invocation — confirming there is no
  genuine type drift.
- No cache-clearing action is needed; the authoritative toolchain output is the
  verification contract for this slice.

## Task checkbox verification

Scanning `openspec/changes/caduxo-i18n-support/tasks.md` against
`taskProgress` (22/58 complete per native status):

**The relevant unchecked implementation task marker for this slice is:**

- `## 2.5 Re-key remaining components` — top-level task item: "Re-key titles,
  buttons, placeholders, modals through `$LL.*` in `src/components/ProductCatalogPage.svelte`,
  `src/components/ProductForm.svelte`, `src/components/ProductDetailPage.svelte`,
  `src/components/LotForm.svelte`, `src/components/CalendarPage.svelte`,
  `src/components/BackupRestorePage.svelte`, `src/components/CsvImportPage.svelte`,
  `src/components/UnitReviewPage.svelte`, `src/components/LotMovementsPanel.svelte`."
  (sub-bullet, unchecked). The `LotMovementsPanel.svelte` portion is now done
  after this slice.
- Sub-task: "Re-key modal helpers (`MoveStockModal`, `RegisterExitModal`,
  `AdjustCountModal`, `ResolveQuantityDialog`, `ArchiveLotDialog`,
  `LotMovementsPanel` dialogs) ...". The `ArchiveLotDialog` portion is now done.

**Other unchecked implementation tasks remain as known carry-over** (per native
status `taskProgress.completed: 22` and `pending: 36`); these are outside this
slice's scope and not regressions introduced by this correction. Most
prominently: PR 1 backend tasks 1.1–1.6 still show unchecked, but `apply-progress.md`
states they were completed in the parent session and `cargo test --lib` is green
(431 passed) — this slice does not address PR 1 and the verifier does not
double-check completed tasks.

## Review workload / PR boundary findings

The parent prompt notes two prior `sdd-apply` failures (no detail) and a
follow-up failure-cause request that also failed. No information was preserved
about those failures. The parent applied this focused correction inline to
unblock the residue work; the slice scope is consistent with task 2.5 sub-bullet
"modal helpers (ArchiveLotDialog, LotMovementsPanel dialogs)".

The diff is bounded to **214 insertions / 3 deletions across 6 files** per
`git diff --stat`, well under the 3000-line session review budget and the
canonical 400-line per-PR boundary. No `size:exception` flag was raised.
No chained-PR concerns are triggered by this slice.

## Risks and follow-ups

1. **Dead-code export — `src/lib/lot_movements.ts::getKindLabel`** still exists
   as a Spanish-only label function but is no longer imported by any component.
   Recommend a follow-up cleanup to either delete it or replace its body with
   `$LL` calls (parity with `getMovementKindLabel`). Not a blocker for this
   slice because it does not render anywhere.

2. **Backend movement-kind coverage** — the `getMovementKindLabel` table
   covers the 12 known kinds. If the backend adds a new `movement_kind`, the
   UI falls back to the raw kind string (e.g. `"entry:supplier_return"`) until
   a key is added. This is the same fail-soft pattern used elsewhere in the
   codebase.

3. **No manual smoke performed in this verification** — the spec requires
   manual smoke on Linux + Windows for the full UI translation pass (task 2.6).
   This slice is a focused key-parity correction; manual smoke remains an
   outstanding PR 2 task and is not regressed by this work.

## Spec coverage matrix

- `#### Scenario: no hard-coded user-visible strings outside the translation tree`
  — satisfied for the two components covered by this slice.
- `#### Scenario: missing Spanish counterpart fails the frontend build` — the
  generator and build pass; parity is enforced.

## Verification evidence summary

```text
npm run i18n:generate                        # EXIT 0  — all files up to date
npx tsc --noEmit                            # EXIT 0  — no errors
npx svelte-check --workspace . --threshold error   # EXIT 0  — 0 errors / 0 warnings
npm run build                               # EXIT 0  — built in 1.34s, 200 modules transformed
```

**Final verdict: PASS.**

## Key Learnings

1. The typesafe-i18n `npm run i18n:generate` exit code plus "all files are up to date" message is the authoritative parity gate between en/index.ts and es/index.ts; this slice adds 13 keys (5 movementKinds + 8 archive.reasons) and all 13 appear in both trees with non-empty strings.
2. The Spanish-only `getKindLabel` export in src/lib/lot_movements.ts was a residue hazard because components could import it and bypass the `$LL` layer; future corrections should audit lib/ helpers for hardcoded locale strings before relying on consumer components to re-key.
3. The pi-lens embedded LSP diagnostic server holds a stale i18n-types.ts snapshot independent of the typesafe-i18n generator output; tsc / svelte-check / vite build always supersede the snapshot for verification purposes.
4. LotMovementsPanel.svelte now uses a `Record<string, () => string>` table pattern for locale-backed label lookup with a `?? kind` fallback, which mirrors the same fail-soft strategy used elsewhere when a backend kind has no translation key yet.
5. The archive reason `<select>` cleanly separates the canonical enum (`ARCHIVE_REASONS[].value`) from its UI label by routing through `archiveReasonLabel(value)`, leaving the legacy `label` field on the data shape unused for rendering and available only as a TypeScript shape guard in the predicate.
