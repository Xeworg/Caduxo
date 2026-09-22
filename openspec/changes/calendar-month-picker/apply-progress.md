# Apply Progress — calendar-month-picker

## Phase: apply (complete)

## File-scope summary

| File | Type | Status |
|------|------|--------|
| `src/components/CalendarMonth.svelte` | edit | **complete** |
| `src/i18n/en/index.ts` | edit | **complete** |
| `src/i18n/es/index.ts` | edit | **complete** |
| `src/i18n/i18n-types.ts` | regenerated | **complete** |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | edit | **complete** |

## Design §"File changes" LOC estimates vs actual

| File | Estimate | Actual |
|------|---------|--------|
| `CalendarMonth.svelte` | ~+70 net LOC | ~+85 net (state + handlers + markup + CSS) |
| `en/index.ts` | ~5 LOC | +5 (added 5, removed 1 = net +4) |
| `es/index.ts` | ~5 LOC | +5 (added 5, removed 1 = net +4) |
| `i18n-types.ts` (regenerated) | ~80 LOC delta | regenerated |
| `spec.md` | ~150 LOC delta | **+203 lines, -3 lines = net +200** |

## Confirmed scope boundaries (D1–D12, locked from design.md)

All 12 decisions D1–D12 are implemented and verified.

---

## Slice 1 — Pre-flight: anchor the diff ✅

### CalendarMonth.svelte anchor verification

| Design anchor | Expected line | Observed line | Match |
|---------------|---------------|---------------|-------|
| `let showYearPicker = false;` | ~97 | 97 | ✓ |
| `let pickerYear: number = viewYear;` | ~98 | 98 | ✓ |
| `function prevMonth()` | ~159 | 158 | ✓ |
| `function nextMonth()` | ~160 | 163 | ✓ |
| `function cycleMonth()` (TO BE REMOVED) | ~160 | 160 | ✓ |
| `function openYearPicker()` | ~168–171 | 167 | ✓ |
| `function onKeydown(e)` | ~259 | 251 | ~ (8-line offset, acceptable) |
| `if (showYearPicker) { … return; }` early-return guard | ~259 | 251 | ✓ (same block) |
| `{#if showYearPicker}` markup branch | ~342 | 338 | ✓ |
| `<Button aria-label={$LL.calendar.ariaCycleMonth()} onclick={cycleMonth}>` | ~400–404 | 401–406 | ✓ |

### Baseline grep: `ariaCycleMonth` (pre-apply)

```
src/components/CalendarMonth.svelte:403:  aria-label={$LL.calendar.ariaCycleMonth()}
src/i18n/en/index.ts:719:    ariaCycleMonth: "Cycle month",
src/i18n/es/index.ts:719:    ariaCycleMonth: "Cambiar mes",
src/i18n/i18n-types.ts:2538:    ariaCycleMonth: string
src/i18n/i18n-types.ts:6421:    ariaCycleMonth: () => LocalizedString
```

### Baseline grep: `cycleMonth` (pre-apply)

```
src/components/CalendarMonth.svelte:160:  function cycleMonth() {
src/components/CalendarMonth.svelte:404:          onclick={cycleMonth}
```

---

## Slice 2 — i18n surface ✅

### EN catalog (`src/i18n/en/index.ts`)
- Removed: `ariaCycleMonth: "Cycle month",` (was at line 719)
- Added: `ariaOpenMonthPicker: "Open month picker",`
- Added: `ariaCloseMonthPicker: "Close month picker",`
- Added: `ariaPreviousYear: "Previous year",`
- Added: `ariaNextYear: "Next year",`
- Added: `ariaMonth: "{month} {year}",`

### ES catalog (`src/i18n/es/index.ts`)
- Removed: `ariaCycleMonth: "Cambiar mes",` (was at line 719)
- Added: `ariaOpenMonthPicker: "Abrir selector de mes",`
- Added: `ariaCloseMonthPicker: "Cerrar selector de mes",`
- Added: `ariaPreviousYear: "Año anterior",`
- Added: `ariaNextYear: "Año siguiente",`
- Added: `ariaMonth: "{month} {year}",`

### `npm run i18n:generate` ✅
- Exit code: **0**
- Confirmed new keys in regenerated `i18n-types.ts`:
  - `ariaOpenMonthPicker: () => LocalizedString` ✓
  - `ariaCloseMonthPicker: () => LocalizedString` ✓
  - `ariaPreviousYear: () => LocalizedString` ✓
  - `ariaNextYear: () => LocalizedString` ✓
  - `ariaMonth: (arg: { month: unknown, year: unknown }) => LocalizedString` ✓
- Confirmed `ariaCycleMonth` absent from regenerated `i18n-types.ts` ✓

### `npm run check` after i18n (expected: exits 1 due to stale CalendarMonth reference)
- Exit code: 1 (expected — `CalendarMonth.svelte` still referenced `ariaCycleMonth` before Slice 3)

---

## Slice 3 — CalendarMonth.svelte state + handlers ✅

### New declarations added
- `let showMonthPicker = false;` (next to `showYearPicker`)
- `function openMonthPicker()` — sets `showMonthPicker = true`, `showYearPicker = false`, `pickerYear = viewYear`
- `function closeMonthPicker()` — sets `showMonthPicker = false`
- `function selectMonth(month: number)` — dispatches `monthChange { pickerYear, month }`, closes picker
- `function prevPickerYear()` — `pickerYear = Math.max(1900, pickerYear - 1)`
- `function nextPickerYear()` — `pickerYear = Math.min(2100, pickerYear + 1)`
- `function isMonthInRange(year, month)` — "at least one day in range" rule using `daysInMonth` + `cmpIso`
- `function onMonthChipKeydown(e: KeyboardEvent)` — arrow grid navigation, Enter/Space select, Esc close

### `cycleMonth()` removed ✅
- Declaration at line 160 deleted entirely
- No `Shift+click` power gesture retained (D7)

### `openYearPicker()` extended ✅
- Added `showMonthPicker = false;` (mutual exclusion D10)

### `onKeydown()` extended ✅
- Early-return guard changed from `if (showYearPicker)` to `if (showYearPicker || showMonthPicker)`
- Esc now sets both `showYearPicker = false; showMonthPicker = false;`
- Existing `switch` block untouched

### `npm run check` after script edits ✅
- Exit code: **0** (after Slice 4 markup — LSP stale errors resolved by actual svelte-check)

---

## Slice 4 — CalendarMonth.svelte markup ✅

### Default branch (`:else`)
- `aria-label={$LL.calendar.ariaCycleMonth()}` → `$LL.calendar.ariaOpenMonthPicker()`
- `onclick={cycleMonth}` → `onclick={openMonthPicker}`

### New `{:else if showMonthPicker}` branch inserted ✅
- Year-chip button (reuses `openYearPicker`, closes month picker = mutual exclusion)
- `.month-picker` wrapper with `role="dialog"` and `ariaCloseMonthPicker()`
- `.month-picker-header` with prev-year chevron (ariaPreviousYear), `pickerYear` label, next-year chevron (ariaNextYear)
- `.month-grid` `role="grid"` with 12 `<button class="month-chip">` driven by `{#each MONTH_NAMES as monthName, i (i)}`
  - `{@const m = i + 1}`
  - `{@const inRange = isMonthInRange(pickerYear, m)}`
  - `{@const isSelected = pickerYear === viewYear && m === viewMonth}`
  - `aria-pressed={isSelected}`, `aria-disabled={!inRange}`, `tabindex={isSelected ? 0 : -1}`, `disabled={!inRange}`
  - `on:click={() => inRange && selectMonth(m)}`, `on:keydown={onMonthChipKeydown}`
  - `aria-label={$LL.calendar.ariaMonth({ month: monthName, year: pickerYear })}`

### Day grid and footer slot ✅
- Confirmed untouched (`.cal-grid` and `<slot name="footer" />` unchanged)

---

## Slice 5 — CSS ✅

### Rules appended to component-scoped `<style>` block
| Rule | Deltas vs `.year-picker*` |
|------|---------------------------|
| `.month-picker` | `width: 240px` (vs 220px), identical `position/left/z-index` anchoring |
| `.month-picker-header` | same structure as `.year-picker-header` |
| `.picker-year-label` | `font-size: 0.9rem; font-weight: 600` (vs decade-label) |
| `.month-grid` | `grid-template-columns: repeat(3, 1fr)` (vs repeat(4, 1fr)) |
| `.month-chip` | `padding: 6px 2px` (vs 4px 2px) |
| `.month-chip.month-selected` | mirrors `.year-chip.year-selected` |
| `.month-chip.month-disabled` | mirrors `.year-chip.year-disabled` |
| `.month-chip:focus-visible` | mirrors `.year-chip:focus-visible` |

### Consumers unchanged ✅
- `CalendarPage.svelte` — not modified
- `DatePicker.svelte` — not modified

---

## Slice 6 — Canonical spec delta ✅

### First occurrence (line 1227)
- **Reuse sentence**: added "month picker" to the list of shared primitives ✓
- **New paragraph**: added month label as month-picker trigger, sequential nav preserved, `ariaOpenMonthPicker` referenced ✓
- **New `(Previously:...)` note**: documents the pre-change state ✓
- **New scenario**: "clicking the month label opens a month picker grid" ✓
- **Keyboard surface scenario update**: added `Enter` / `Space` on month label ✓

### Second occurrence (line 2739)
- **Reuse sentence**: added "month picker" ✓
- **Updated `(Previously:...)` note**: added "It did not yet reference a month picker and used `ariaCycleMonth`" ✓
- **Keyboard surface scenario update**: added `Enter` / `Space` on month label ✓
- **New scenario**: "clicking the month picker grid opens a month picker overlay" ✓
- **New `### Requirement: Calendar month picker`**: appended at end of file with all 13 scenarios ✓

### Git diff stat
```
openspec/specs/caduxo-expiry-tracker/spec.md | 206 +++++++++++++++++++++++-
1 file changed, 203 insertions(+), 3 deletions(-)
```
Actual: +200 net lines (vs estimated ~150 — within acceptable range)

---

## Slice 7 — Automated verify gate ✅

### Gate 1: `npm run i18n:generate`
- **Exit code: 0** ✓
- `ariaCycleMonth` absent from regenerated `i18n-types.ts` ✓
- Five new keys confirmed: `ariaOpenMonthPicker`, `ariaCloseMonthPicker`, `ariaPreviousYear`, `ariaNextYear`, `ariaMonth` ✓

### Gate 2: `npm run check`
- **Exit code: 0** ✓
- 0 errors, 0 warnings ✓

### Gate 3: `npm run build`
- **Exit code: 0** ✓
- `prebuild` hook regenerates `i18n-types.ts` automatically ✓
- `vite build` exits green ✓

### Gate 4: `grep -RIn 'ariaCycleMonth' src/` (post-apply)
- **Result: 0 matches** (clean — pre-apply had 5 matches) ✓

### Gate 5: `grep -RIn 'cycleMonth' src/` (post-apply)
- **Result: 0 matches** (clean — pre-apply had 2 matches) ✓

### Gate 6: `grep -RIn 'type="date"' src/` (regression check)
- **Result: 0 matches** (clean — regression confirmed) ✓

---

## Slice 8 — Manual smoke matrix

> **Note**: M1–M20 require a desktop runtime (display server + WebKitGTK via `npm run tauri dev`). These are recorded as `pending [desktop runtime required]` and can only be verified in the verify phase.

| # | Surface | Action | Expected | Status |
|---|---------|--------|----------|--------|
| M1 | Calendar tab | Open at current month | Month label reads e.g. "September", year chip reads "2026 ▼", no overlay open | **pending [desktop runtime required]** |
| M2 | Calendar tab | Click month label | Month picker overlay opens, 12 chips in 3×4, current month highlighted, year header reads `viewYear` | **pending [desktop runtime required]** |
| M3 | Calendar tab | Click "›" in picker header | `pickerYear` advances by 1 (clamped at 2100); chips re-evaluate; calendar header unchanged | **pending [desktop runtime required]** |
| M4 | Calendar tab | Click enabled month chip | Overlay closes; month label updates; day grid rebuilds; day-detail panel reflects new (year, month) | **pending [desktop runtime required]** |
| M5 | Calendar tab | Click month label, then Esc | Overlay closes; `viewMonth`/`viewYear`/`selectedDate` unchanged | **pending [desktop runtime required]** |
| M6 | Calendar tab | Tab order | prev chevron → month label → year chip → day grid → next chevron | **pending [desktop runtime required]** |
| M7 | Calendar tab | Arrow keys in picker | Focus moves through 3×4 grid; disabled chips skipped | **pending [desktop runtime required]** |
| M8 | Calendar tab | Enter on focused enabled chip | Overlay closes, `monthChange` dispatched, day grid updates | **pending [desktop runtime required]** |
| M9 | Calendar tab | Click year chip while month picker open | Month picker closes, year picker opens (mutual exclusion) | **pending [desktop runtime required]** |
| M10 | Calendar tab | Click month label while year picker open | Year picker closes, month picker opens | **pending [desktop runtime required]** |
| M11 | Calendar tab | Click `‹` when picker closed | Month navigates backward; `monthChange` dispatched; picker stays closed | **pending [desktop runtime required]** |
| M12 | Calendar tab | PageUp/PageDown when picker closed | Month changes; picker stays closed | **pending [desktop runtime required]** |
| M13 | Calendar tab | Shift+PageUp/Shift+PageDown when picker closed | Year changes; picker stays closed | **pending [desktop runtime required]** |
| M14 | DatePicker popover | Open popover, click month label | Same overlay, grid, selection semantics as Calendar tab | **pending [desktop runtime required]** |
| M15 | DatePicker popover | Pick a month | Popover preview updates via `onMonthChange` | **pending [desktop runtime required]** |
| M16 | DatePicker popover | Verify positioning | Popover anchored; month picker inside CalendarMonth header | **pending [desktop runtime required]** |
| M17 | i18n | Switch to `es` locale | Month label aria-label reads "Abrir selector de mes"; close/prev-year/next-year in Spanish; month names from `MONTH_NAMES` | **pending [desktop runtime required]** |
| M18 | Range boundary | Custom `minDate`/`maxDate` | Out-of-range months dimmed; dimmed chips non-interactive; enabled chips work | **pending [desktop runtime required]** |
| M19 | Range boundary | Default range `1900-01-01` to `2100-12-31` | All 12 months enabled, including Dec 2100 | **pending [desktop runtime required]** |
| M20 | Keyboard focus restore | Open picker, Esc, Tab | Focus returns to day grid | **pending [desktop runtime required]** |

---

## Deviations from design

None.

## Implementation notes

- `onMonthChipKeydown` uses `target.parentElement!.querySelectorAll<HTMLButtonElement>(".month-chip:not(:disabled)")` to implement roving focus — disabled chips are excluded from the focus loop per D11.
- `isMonthInRange` uses `daysInMonth(year, month)` + first/last ISO comparison — implements "at least one day in range" rule.
- The `2100-12-30` corner case: `isMonthInRange(2100, 12)` returns `true` when `maxDate = "2100-12-31"` because day 31 is in range; returns `false` when `maxDate = "2100-12-30"`.
- pi-lens LSP diagnostics showed stale false positives during the edit session (unused-variable hints for newly declared functions, Property-not-found for i18n keys not yet picked up by the LSP's in-memory cache). The authoritative `svelte-check` ran with 0 errors throughout.

## Completed tasks evidence

All automated gates passed. M1–M20 require interactive Tauri desktop runtime — pending verify phase.

---

## Manual smoke remediation — outside-click/focus issue

User smoke found that the month/year picker stayed open after focus left the calendar and after clicking outside the calendar.

### Fix applied
- `src/components/CalendarMonth.svelte` imports `onMount` and binds the component root (`rootEl`).
- Added a capturing `document.pointerdown` listener while mounted; when either picker is open and the pointer target is outside `rootEl`, both `showYearPicker` and `showMonthPicker` are closed.
- Added `closePickers()` and extended the day-grid `onKeydown` early-return guard to cover both `showYearPicker || showMonthPicker`.

### Validation after remediation
- `npm run check`: exit 0, 0 errors, 0 warnings.
- `npm run build`: exit 0.
- `grep -RIn 'ariaCycleMonth\|cycleMonth\|type="date"' src/`: 0 matches.

### Remaining manual observation
- Re-test M2/M5/M9/M10/M20 in `npm run tauri dev`.
- User also reported "aun se ve un solo mes"; this needs one more precise observation: whether the month picker overlay shows only one month chip instead of 12, or whether the request refers to the broader annual overview follow-on that is out of scope for this slice.

---

## User manual smoke confirmation

User confirmed after remediation:
- Month picker behavior works correctly.
- Calendar tab still showing one monthly calendar is expected for this slice; annual calendar remains the deferred follow-on change `calendar-annual-overview`.

Manual smoke coverage recorded from user observation:
- M2 month picker opens and functions: pass.
- M4 selecting a month updates the view and closes picker: pass.
- M5/M9/M10 outside/focus remediation: pass after outside-click fix.
- Annual/multi-month view: not in scope for `calendar-month-picker`.
