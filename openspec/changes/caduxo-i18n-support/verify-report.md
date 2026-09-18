# Verify Report — caduxo-i18n-support (residual-fix re-verify)

Change: `caduxo-i18n-support`
Scope: independent re-verification after the writer addressed the CSV info card
blocker from the previous verify pass. Read-only; no edits and no commits.
Phase: verify (optional; native recommends `apply` then `archive`).

---

## Overall verdict: **PASS**

All five previously flagged residuals are now correctly fixed and the focus
blocker (sibling info action card in `CsvImportPage.svelte`) is resolved with
the established call shape. All four mandated commands succeed.

---

## Focus blocker — CsvImportPage info action card

The previous verify pass flagged that the sibling info/expected-columns action
card misused `$LL.csvImport.selectFileDesc()` as the optional-columns label.
The writer replaced that with the existing `$LL.csvImport.optional()` call
shape.

- File: `src/components/CsvImportPage.svelte`
- Current rendering of the info card body (line 201):

  ```svelte
  <span class="card-desc">
    {$LL.csvImport.required()}: <code>sku</code>, <code>description</code><br />
    {$LL.csvImport.optional()}: <code>barcode</code>, <code>category</code>,
    <code>unit</code>, <code>alert_days_before</code>, <code>notes</code>
  </span>
  ```

- `grep -n 'csvImport\.selectFileDesc\|csvImport\.selectFileAction\|csvImport\.optional'`
  on the file confirms three independent keys are used in their proper
  slots:
  - Line 181 (`<p class="page-desc">`): `{$LL.csvImport.selectFileDesc()}` —
    page-level description key, appropriate for that slot.
  - Line 193 (primary action card): `{$LL.csvImport.selectFileAction()}` —
    action-keyed brief click hint.
  - Line 201 (info action card): `{$LL.csvImport.optional()}` — established
    optional-marker key, previously used in other components.
- Locale coverage for the three keys:
  - `en/index.ts`: `selectFileDesc` (line 660), `selectFileAction` (line 702),
    `optional` (line 669).
  - `es/index.ts`: `selectFileDesc` (line 657), `selectFileAction` (line 658),
    `optional` (line 667).
- The card now renders meaningfully in both locales:
  > Required: `sku`, `description`
  > Optional: `barcode`, `category`, `unit`, `alert_days_before`, `notes`
  > (English); Obligatorio / Opcional in Spanish.

---

## Spot-check of the prior five residuals

### Residual 1 — CSV strategy descriptions → locale keys (PASS)

- File: `src/components/CsvImportPage.svelte` (strategies array).
- Verified three descriptions use locale keys:
  - `$LL.csvImport.conflictOptions.skipDuplicatesDesc()`
  - `$LL.csvImport.conflictOptions.updateExistingDesc()`
  - `$LL.csvImport.conflictOptions.reviewConflictsDesc()`
- No hardcoded English strategy descriptions remain in the file.
- Locale coverage:
  - `en/index.ts` lines 706 / 710 / 712 ✓
  - `es/index.ts` lines 703 / 707 / 709 ✓

### Residual 2 — Primary action card select-file description (PASS)

- File: `src/components/CsvImportPage.svelte` (line 193).
- `<span class="card-desc">{$LL.csvImport.selectFileAction()}</span>` uses the
  dedicated action key (`Click to select a CSV file from your device` /
  `Haz clic para seleccionar un archivo CSV desde tu dispositivo`).
- No `selectFileDesc()` call on the card body.

### Residual 3 — CalendarPage date formatting (PASS)

- File: `src/components/CalendarPage.svelte` (function `formatDate`).
- Uses `new Date(y, m - 1, day).toLocaleDateString(undefined, { month: "long",
  day: "numeric", year: "numeric" })`.
- No hardcoded English month-name array in `CalendarPage.svelte` —
  `grep -n 'months\s*=\s*\[' src/components/CalendarPage.svelte` returns no
  matches.
- Honors the runtime locale rune via the `undefined` first argument.

> **Note (out of slice).** `src/components/CalendarMonth.svelte` (child
> component) still has a hardcoded `LABELS.months` and `LABELS.weekdays`
> array (lines 28–32) plus several `aria-label="..."` literals
> (`"Previous month"`, `"Next month"`, `"Cycle month"`, `"Open year
> picker"`, `"Previous decade"`, `"Next decade"`, `"Year {yr}"`). These
> were NOT part of the five residuals the previous verifier flagged, and
> they predate this change. Surfacing here as an observation for a possible
> follow-up slice; not a regression introduced by this apply.

### Residual 4 — CalendarPage expiry calendar aria label (PASS)

- File: `src/components/CalendarPage.svelte`.
- `grep -n 'Expiry calendar' src/components/CalendarPage.svelte` returns no
  matches.
- The `CalendarMonth` child now receives
  `ariaLabel={$LL.calendar.pageTitle()}` from its parent.
- Other aria-labels in `CalendarPage.svelte` use `$LL.calendar.refreshAria()`
  and `$LL.dashboard.lotDetail()`.

### Residual 5 — UnitReviewPage product count duplication (PASS)

- File: `src/components/UnitReviewPage.svelte` (`group-header` span).
- Current rendering (lines 149–153) is the plural-aware conditional only —
  no raw `{group.product_count}` prefix precedes the translation:

  ```svelte
  <span class="product-count">
    {group.product_count === 1
      ? $LL.unitReview.unitCount_singular({ n: group.product_count })
      : $LL.unitReview.unitCount_plural({ n: group.product_count })}
  </span>
  ```

- Locale coverage: `unitCount_singular` / `unitCount_plural` exist in both
  `en/index.ts` (lines 746 / 747) and `es/index.ts` (lines 743 / 744).

---

## Commands (all PASS)

| Command | Result |
|---------|--------|
| `npm run i18n:generate` | `[typesafe-i18n] ... all files are up to date` — exit 0 |
| `npx svelte-check --workspace . --threshold error` | `svelte-check found 0 errors and 0 warnings` |
| `npx tsc --noEmit` | `EXIT: 0` (no output, clean) |
| `npm run build` | `✓ built in 1.27s` (200 modules transformed; `prebuild` regenerated i18n) |

The build pipeline is green and the regenerated `i18n-types.ts` matches the
locale trees.

---

## Diff summary (read-only inspection)

```
openspec/changes/caduxo-i18n-support/apply-progress.md  |  351 +++++
openspec/changes/caduxo-i18n-support/tasks.md          |    6 +-
src/components/AdjustCountModal.svelte                 |   44 +-
src/components/ArchiveLotDialog.svelte                 |   32 +-
src/components/BackupRestorePage.svelte                |   58 +-
src/components/CalendarPage.svelte                     |   81 +-
src/components/ColumnMapper.svelte                     |   33 +-
src/components/CsvImportPage.svelte                    |  144 +-
src/components/LotMovementsPanel.svelte                |   21 +-
src/components/MoveStockModal.svelte                   |   41 +-
src/components/RegisterExitModal.svelte               |   57 +-
src/components/ResolveQuantityDialog.svelte            |   41 +-
src/components/UnitReviewBanner.svelte                 |   17 +-
src/components/UnitReviewPage.svelte                   |   68 +-
src/i18n/en/index.ts                                   |  218 ++-
src/i18n/es/index.ts                                   |  216 ++-
src/i18n/i18n-types.ts                                 | 1644 ++++++++++++++++++--
17 files changed, 2645 insertions(+), 427 deletions(-)
```

Side inspections of the modal helpers in the diff (all carry `$LL.*`
bindings for their aria-labels; none regressed by this slice):

| File | aria-label binding | Notes |
|------|--------------------|-------|
| `AdjustCountModal.svelte` | `$LL.lotMovements.adjustCount()` | line 103 |
| `ArchiveLotDialog.svelte` | `aria-labelledby="archive-title"` | references title element id; no string-literal aria-label |
| `ColumnMapper.svelte` | n/a | no modal aria-label; component-localized body |
| `MoveStockModal.svelte` | `$LL.lotMovements.moveStock()` | line 126 |
| `RegisterExitModal.svelte` | `$LL.lotMovements.registerExit()` | line 115 |
| `ResolveQuantityDialog.svelte` | `aria-labelledby="resolve-title"` | references title element id; no string-literal aria-label |
| `LotMovementsPanel.svelte` | n/a | component-localized body |

---

## Findings

### Blockers

- None. The focus blocker is resolved and all five previously flagged
  residuals stay fixed.

### Risks / observations (non-blocking)

1. **`CalendarMonth.svelte` (child of `CalendarPage.svelte`) still has
   hardcoded English UI strings.** Pre-existing and outside the five
   residuals the previous verifier enumerated. The component still ships
   the `LABELS.months` / `LABELS.weekdays` arrays plus several
   `aria-label="..."` literals. The parent (`CalendarPage.svelte`) only
   passes `ariaLabel` for the calendar grid; navigation and picker
   controls are children-local. Surface here so the parent session can
   decide whether to absorb the re-keying into this slice or punt to a
   follow-up. Not a regression introduced by this apply.

2. **`tasks.md` checkbox state is stale relative to the current diff.**
   Multiple `- [ ]` boxes in PR 2.5 / PR 2.6 sections describe work that
   has landed in the working tree (ProductCatalogPage, ProductForm,
   ProductDetailPage, LotForm, CalendarPage, BackupRestorePage,
   CsvImportPage, UnitReviewPage, UnitReviewBanner, modal helpers).
   The parent apply-progress.md narrates the work; the tasks.md boxes
   have not been updated in lockstep. This is a hygiene gap that does
   not block the validation pipeline. Recommend a follow-up commit
   that ticks the relevant `- [ ]` boxes once the parent confirms the
   chain strategy and slices.

3. **No new files introduced.** All edits live in the existing source
   tree. No commits made (read-only verify).

---

## Verdict

| Item | Status |
|------|--------|
| Focus blocker (CsvImportPage info card uses `csvImport.optional`) | PASS |
| Residual 1 — CSV strategy descriptions | PASS |
| Residual 2 — Primary action card uses `selectFileAction` | PASS |
| Residual 3 — CalendarPage `formatDate` has no hardcoded English months | PASS |
| Residual 4 — CalendarPage expiry-calendar aria label is localized | PASS |
| Residual 5 — UnitReviewPage product count not duplicated | PASS |
| `npm run i18n:generate` | PASS |
| `npx svelte-check --workspace . --threshold error` | PASS |
| `npx tsc --noEmit` | PASS |
| `npm run build` | PASS |
| **Overall** | **PASS** |
