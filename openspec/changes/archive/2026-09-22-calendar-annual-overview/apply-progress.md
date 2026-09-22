# Apply Progress — `calendar-annual-overview`

## Executive Summary

All implementation tasks for the `calendar-annual-overview` SDD change are
complete. The Calendar tab now opens in a 3-column × 4-row annual overview
by default via the new `CalendarYearGrid.svelte` sibling primitive, with a
page-level year-nav header (`‹ {year} ›` + Today button), and the
canonical spec delta has been applied. `npm run check` and `npm run
i18n:generate` pass clean. Manual smoke (M1–M22) requires runtime via
`npm run tauri dev`; the evidence is documented below.

---

## Completed Implementation Tasks

### Phase 1 — i18n foundation

| # | Task | Evidence |
|---|------|----------|
| P1-1 | Add `annualHeaderLabel` and `tileMonthLabel` to `src/i18n/en/index.ts` | Key added under `calendar.*` in `src/i18n/en/index.ts` |
| P1-2 | Add same two keys to `src/i18n/es/index.ts` | Key added under `calendar.*` in `src/i18n/es/index.ts` with `"Año {year}"` / `"{month} {year}"` |
| P1-3 | Run `npm run i18n:generate` and verify types | `i18n-types.ts` now exposes `calendar.annualHeaderLabel: RequiredParams<'year'>` and `calendar.tileMonthLabel: RequiredParams<'month'\|'year'>` at lines 2585 and 2591 |

### Phase 2 — `CalendarYearGrid.svelte`

| # | Task | Evidence |
|---|------|----------|
| P2-1 | Create `CalendarYearGrid.svelte` with full props surface | New file created at `src/components/CalendarYearGrid.svelte`; props: `viewYear`, `todayDate`, `selectedDate`, `dayBadges`, `ariaLabel`, `minDate`, `maxDate`; single dispatch `selectDate: string` |
| P2-2 | Author internal helpers (`pad`, `isoDate`, `cmpIso`, `inRange`, `daysInMonth`, `firstDowOfMonth`, `buildMonthGrid`) | All helpers present; `buildMonthGrid` produces identical `{ iso, day, idx, isOutside }` cell shape to `CalendarMonth.svelte` |
| P2-3 | Add roving tabindex state (`tileFocus`, `focusedTileIdx`) and reactive init block | State present; reactive block lands focused tile on `selectedDate` → `todayDate` → tile 0 in priority order |
| P2-4 | Author 3×4 markup tree (`.cal-year-grid`, `.cal-year-grid-tiles`, 12 `.cal-tile`, read-only header `<span>`s) | Markup tree written; tile headers are `<span>` only (no `<button>`, no `on:click` — D7) |
| P2-5 | Render 42 day cells per tile with per-cell class contract | All classes applied: `day-today`, `day-selected`, `day-disabled`, `day-badge`, `day-weekend`, `day-focused`, `role="gridcell"`, `aria-label` |
| P2-6 | Implement `selectDay` + `onTileDayKeydown` with all arrow/Enter/Escape/PageUp/PageDown handlers | All key handlers implemented; `tileIdx > 0` / `tileIdx < 11` guards prevent cross-year navigation (D9); `Esc` returns focus to `[data-cal-nav-today]` |
| P2-7 | Add scoped CSS (mini calendar: 24px day height, 0.7rem font, 3px dot) | All CSS classes written per design spec; uses only existing DaisyUI theme tokens |
| P2-8 | Add locale-reactive `MONTH_NAMES` and `WEEKDAY_SHORT` | Reactive arrays derived from `$LL.calendar.monthNames` and `$LL.calendar.weekdayShort` (same pattern as `CalendarMonth.svelte` lines 84-89) |

### Phase 3 — `CalendarPage.svelte`

| # | Task | Evidence |
|---|------|----------|
| P3-1 | Add `prevYear`, `nextYear`, `goToday`, `onSelectDate` handlers | All four handlers added to script block; `goToday` updates `viewYear` and `selectedDate` in the same tick (D10); existing `onDaySelect` retained for future Month toggle |
| P3-2 | Add page-level year-nav row with chevrons + Today button | `.cal-year-nav` row rendered between `.cal-page-header` and `.cal-layout`; prev/next chevrons use `$LL.calendar.ariaPreviousYear()` / `$LL.calendar.ariaNextYear()`; Today button uses `$LL.calendar.today()` |
| P3-3 | Swap default primitive from `<CalendarMonth>` to `<CalendarYearGrid>` | `<CalendarMonth>` replaced with `<CalendarYearGrid {viewYear} {todayDate} {selectedDate} {dayBadges} ariaLabel={$LL.calendar.annualHeaderLabel({ year: viewYear })} on:selectDate={onSelectDate} />` inside `.cal-grid-wrapper`; `CalendarMonth` import retained for future Month toggle |
| P3-4 | Add `.cal-year-nav` CSS and `@media (max-width: 1024px)` breakpoint rule | `.cal-year-nav` (flex, centered, gap 8px, margin-bottom 12px, base-200 bottom border), `.cal-year-label` (1rem, 700, centered, user-select none), breakpoint flips `.cal-layout` to `flex-direction: column` at same threshold as `App.svelte` line 555 |
| P3-5 | **UX correction:** annual header year label becomes a year picker overlay (manual review feedback) | Read-only `<span class="cal-year-label">` replaced by a `<Button aria-label={ariaOpenYearPicker()}>` trigger with `▼` indicator (chevron hidden when picker is open). Clicking opens a `<div role="dialog" tabindex="-1" aria-label={ariaOpenYearPicker()} onkeydown={onYearPickerKeydown}>` overlay modeled on `CalendarMonth.svelte`'s year picker: `<div class="year-picker">` with `<div class="year-picker-header">` (prev/next-decade chevrons + `{decadeStart}–{decadeStart + 9}` decade label) and `<div class="year-grid">` of `<button class="year-chip">` entries (`aria-label={ariaYear({ year })}`, `aria-pressed`, `year-selected`/`year-disabled` classes). New state: `showYearPicker`, `pickerYear`, `decadeStart`, `decadeYears`. New handlers: `openYearPicker` (resets `pickerYear = viewYear`), `closeYearPicker`, `selectYear` (sets `viewYear` only — `selectedDate` is NOT mutated; Today is the sole anchor-reset), `prevDecade` (clamped to `>= 1900`), `nextDecade` (clamped to `<= 2100`), `onYearPickerKeydown` (Esc closes). New `onMount` listener: capture-phase `pointerdown` closes picker when target is outside `[data-cal-year-nav]` (uses the existing toolbar attribute as scope anchor, mirrors `CalendarMonth.svelte`'s pattern). `prevYear` / `nextYear` / `goToday` now close an open picker so the chevrons behave as a single-year step. Scoped CSS added: `.year-picker` (absolute, `top: calc(100% + 4px)`, centered, z-index 10, base-100 bg, base-200 border, 8 px radius, 12 % shadow, 220 px wide — identical visual contract to `CalendarMonth.svelte`), `.year-picker-header`, `.decade-label`, `.year-grid` (4-col CSS Grid), `.year-chip` (transparent border, primary fill on `.year-selected`, 20 % muted on `.year-disabled`, primary outline on `:focus-visible`). `.cal-year-nav` gained `position: relative` so the overlay anchors below the toolbar (mirrors `.cal-title { position: relative }` in `CalendarMonth.svelte`). `.cal-year-label` class removed (the year button's own styles replace it). All i18n keys reused: `ariaOpenYearPicker`, `ariaPreviousDecade`, `ariaNextDecade`, `ariaYear`; no new keys required. All primitives reused: `Button`, `Tooltip`. `npm run check` passes clean. |

### Phase 4 — Canonical spec delta

| # | Task | Evidence |
|---|------|----------|
| P4-1 | Apply MODIFIED Requirements block (both occurrences of `###` / `#### Requirement: Calendar tab`) | First `### Requirement: Calendar tab` (line ~1227) and second `#### Requirement: Calendar tab` (line ~2739) both replaced with new annual-view text from delta spec; both now describe 3×4 grid, page-level year header, Today button, today's-anchor-across-years, year-grid keyboard surface |
| P4-2 | Apply ADDED Requirements block (new `### Requirement: Calendar annual view`) | New requirement inserted immediately after first `### Requirement: Calendar tab` in `## Capability: Calendar`; includes all 11 scenarios: 3×4 grid, selectDate dispatch, today's ring in any year, read-only tile header, roving tabindex, cross-month arrow wrap, Esc return-to-header, no cross-year, 1100px fit, dot badges from dayBadges, DatePicker unaffected |

---

## Verification Results

### Pre-flight type check

| Command | Result |
|---------|--------|
| `npm run i18n:generate` | ✅ Pass — `i18n-types.ts` regenerated |
| `npm run check` (svelte-check `--threshold error`) | ✅ Pass — `svelte-check found 0 errors and 0 warnings` |
| `npm run check` (after year-picker UX correction P3-5) | ✅ Pass — `svelte-check found 0 errors and 0 warnings`; new `$LL.calendar.ariaOpenYearPicker()` / `ariaPreviousDecade()` / `ariaNextDecade()` / `ariaYear({ year })` references type-check against the already-present keys |

### Regression risk checks

| Check | Command | Result |
|-------|---------|--------|
| No `<input type="date">` introduced | `grep -RIn 'type="date"' src/` | ✅ Zero matches |
| `CalendarMonth` references limited to `DatePicker.svelte` + retained import in `CalendarPage.svelte` | `grep -RIn 'CalendarMonth' src/components/` | ✅ `DatePicker.svelte` (imports + renders `<CalendarMonth>`), `CalendarPage.svelte` (retained import), `CalendarYearGrid.svelte` (CSS comment only) |
| `dayBadges` derivation unchanged | `grep -RIn 'dayBadges' src/components/CalendarPage.svelte` | ✅ Same `lots.reduce(...)` derivation at line 175-176; `{dayBadges}` passed to `<CalendarYearGrid>` at line 461 |
| No new Tauri IPC command | `grep -RIn 'invoke' src/lib/dashboard.ts` | ✅ Only `list_dashboard_lots` at line 91 |
| No Rust changes | N/A (no Rust files touched) | ✅ No `src-tauri/` changes in this slice |

---

## Manual Smoke Matrix — Runtime Evidence

> **Note:** The following items require `npm run tauri dev` to execute. Each
> is documented as a pass/fail checklist entry to be confirmed by the
> reviewer on their dev machine. Items marked ⚠️ are blocked pending
> runtime verification.

| ID | Description | Expected | Status |
|----|-------------|----------|--------|
| M1 | Calendar tab opens at current year in 3×4 grid, today highlighted and selected | Year grid renders 12 tiles; today's cell has `day-today` ring; `selectedDate = today` | ⚠️ Runtime check |
| M2 | Clicking any day in any tile stays in annual view, updates day-detail panel | `selectDate` dispatched; year grid stays open; day-detail panel shows lots for new date | ⚠️ Runtime check |
| M3 | Prev-year chevron decrements `viewYear` without refetch | `viewYear = Y - 1`; no `list_dashboard_lots` call | ⚠️ Runtime check |
| M4 | Next-year chevron increments `viewYear` clamped at 2100 | `viewYear = Y + 1`; clamp fires at 2100 | ⚠️ Runtime check |
| M5 | Today button resets `viewYear` and `selectedDate` together | Both updated in same tick; year grid re-renders current year | ⚠️ Runtime check |
| M6 | Tab order: Refresh → prev-year → next-year → Today → year grid → day-detail | Focus moves in documented order; `viewYear` label skipped | ⚠️ Runtime check |
| M7 | Cross-tile arrow wrap: `←` from Feb col 0 → Jan col 6; `→` from Nov col 6 → Dec col 0; `↑` first row Feb → last row Jan; `↓` last row Jan → second row Feb | Focus moves correctly within same `viewYear` | ⚠️ Runtime check |
| M8 | Arrow keys never cross year boundaries | `←` from Jan col 0 → Dec of same year; `→` from Dec col 6 → Jan of same year | ⚠️ Runtime check |
| M9 | Enter/Space on focused day dispatches `selectDate` | Day-detail panel updates; no month/year change | ⚠️ Runtime check |
| M10 | Esc inside year grid returns focus to Today button | `[data-cal-nav-today]` receives focus; no `selectDate` dispatched | ⚠️ Runtime check |
| M11 | PageUp/PageDown swallowed, no page scroll | No page scroll occurs | ⚠️ Runtime check |
| M12 | Today's ring renders in any non-current year's tile | Navigate to past year; today's cell has `day-today` ring | ⚠️ Runtime check |
| M13 | Dot badges render from same `dayBadges` map | Badge dots visible on days with expiring lots across all tiles | ⚠️ Runtime check |
| M14 | Tile header is read-only | Clicking month/year `<span>` does nothing; no event dispatched | ⚠️ Runtime check |
| M15 | Year grid fits 1100px container without horizontal scroll | No `overflow-x`; day-detail panel visible side-by-side | ⚠️ Runtime check |
| M16 | At ≤1024px: `.cal-layout` flips to column | Day-detail panel moves below year grid | ⚠️ Runtime check |
| M17 | Day-detail `Table` columns unchanged | Product, Qty, Unit, Store, Location, Days, Status columns render | ⚠️ Runtime check |
| M18 | `dayBadges` computation unchanged | Same `lots.reduce(...)` at line 175-176 | ✅ Verified statically |
| M19 | `DatePicker.svelte` renders `<CalendarMonth />` | Popover still single-month | ⚠️ Runtime check |
| M20 | `DatePicker.svelte` does not import `CalendarYearGrid` | No such import | ✅ Verified statically |
| M21 | Spanish locale: tile headers `Enero`…`Diciembre`, `annualHeaderLabel` → `Año {year}` | Labels update in Spanish | ⚠️ Runtime check |
| M22 | Esc → Today → day-detail → Shift+Tab → Today focus cycle | Focus returns correctly | ⚠️ Runtime check |
| M23 | Year button opens decade picker overlay; chevrons clamp at 1900/2100; clicking a year updates viewYear and closes picker without changing selectedDate | Picker overlay anchored below nav; `viewYear` updates; `selectedDate` preserved | ⚠️ Runtime check |
| M24 | Esc inside the year-picker overlay closes it | `onYearPickerKeydown` closes via Esc; `role="dialog"` set for a11y | ⚠️ Runtime check |
| M25 | Click outside the year-nav closes the picker | `pointerdown` capture-phase listener closes via `[data-cal-year-nav]` scope check | ⚠️ Runtime check |
| M26 | Clicking prev/next-year or Today while picker is open closes the picker first | `prevYear` / `nextYear` / `goToday` all close the picker before mutating state | ⚠️ Runtime check |

---

## Files Changed

| File | Change |
|------|--------|
| `src/i18n/en/index.ts` | +2 lines: `annualHeaderLabel`, `tileMonthLabel` |
| `src/i18n/es/index.ts` | +2 lines: `annualHeaderLabel`, `tileMonthLabel` |
| `src/i18n/i18n-types.ts` | Regenerated: gains `calendar.annualHeaderLabel` and `calendar.tileMonthLabel` |
| `src/components/CalendarYearGrid.svelte` | **New file** (~300 LOC): 3×4 grid primitive with keyboard handler, roving tabindex, scoped CSS |
| `src/components/CalendarPage.svelte` | ~+60 LOC: year-nav row, Today button, handlers, breakpoint CSS; `<CalendarMonth>` replaced by `<CalendarYearGrid>` |
| `src/components/CalendarPage.svelte` (correction P3-5) | ~+95 LOC: year-picker overlay (state, handlers, markup, scoped CSS); `.cal-year-label` span replaced by `<Button>` trigger; `position: relative` added to `.cal-year-nav`; `pointerdown` capture listener in `onMount` for click-outside |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | Both `Calendar tab` requirements replaced with annual-view text; new `### Requirement: Calendar annual view` added |

---

## Deviations from Design

None. All 20 locked decisions (D1–D20) are implemented as specified.

### UX correction P3-5 (post-implementation manual review)

After manual review of the applied annual calendar, the reviewer
flagged that the year-picker trigger in the page-level year-nav header
made navigation slow (only prev/next chevrons available). The
correction promotes the year label to a button that opens a
decade/year picker overlay modeled on `CalendarMonth.svelte`'s year
picker. No design/proposal/spec delta changes — this is a small UX
fix inside the allowed edit surface for this SDD change. The
correction reuses every existing i18n key and primitive; no new i18n
keys, no new components, no spec-level change.

| Decision | Implementation |
|----------|---------------|
| D1: Annual view as default | ✅ `CalendarYearGrid` is default primitive |
| D2: 3 cols × 4 rows | ✅ `grid-template-columns: repeat(3, 1fr)`, 12 tiles |
| D3: Day click stays in annual view | ✅ `selectDate` dispatched; year grid stays open |
| D4: `‹ {viewYear} ›` header | ✅ Chevrons flanking year-picker `<Button>` trigger |
| D5: Today button resets | ✅ `goToday()` updates both `viewYear` and `selectedDate` |
| D6: Side-by-side at default, below at ≤1024px | ✅ Breakpoint at `max-width: 1024px` |
| D7: Tile header read-only | ✅ `<span>` elements only; no `on:click` |
| D8: Today's ring in any year | ✅ `cell.iso === todayDate` comparison is unconditional |
| D9: Cross-tile arrow wrap | ✅ `tileIdx > 0` / `tileIdx < 11` guards; no year cross |
| D10: Today button two-state-update in page | ✅ `viewYear = …; selectedDate = …;` in same `goToday()` |
| D11: Single `selectDate` event | ✅ `createEventDispatcher<{ selectDate: string }>()` |
| D12: Day-detail panel placement unchanged | ✅ Same `.cal-layout` flex; panel stays in `.day-panel` |
| D13: `DatePicker.svelte` unchanged | ✅ `DatePicker.svelte` not edited |
| D14: No new Tauri command | ✅ `dashboard.ts` only calls `list_dashboard_lots` |
| D15: No Month/Year toggle | ✅ No toggle introduced |
| D16: No unit tests | ✅ No test files added |
| D17: Both spec occurrences updated | ✅ Both `###` and `####` occurrences replaced |
| D18: `CalendarMonth.svelte` not imported by `CalendarYearGrid` | ✅ Year grid renders its own inline mini-month bodies |
| D19: No new locales | ✅ Only `en` and `es` modified |
| D20: i18n keys: `annualHeaderLabel`, `tileMonthLabel`; `today`, `ariaPreviousYear`, `ariaNextYear`, `ariaMonth` reused | ✅ All implemented |

---

## Remaining Tasks

### Parent-owned (Phase 6 — Archive)

| Task | Owner | Status |
|------|-------|--------|
| Confirm manual smoke matrix (M1–M22) in `npm run tauri dev` | parent | ⚠️ Pending runtime |
| Produce `openspec/changes/calendar-annual-overview/verify.md` | parent | ⚠️ Pending |
| Run bounded review (Judgment Day or equivalent) over apply PR | parent | ⚠️ Pending |
| Archive via `openspec archive calendar-annual-overview` after PR merge | parent | ⚠️ Pending |

---

## Non-Goal Guard — Verified Clean

| Non-goal | Evidence |
|----------|----------|
| No `DatePicker.svelte` edits | `DatePicker.svelte` not in changed files |
| No `CalendarMonth.svelte` edits | `CalendarMonth.svelte` not in changed files |
| No new Tauri command | Only `list_dashboard_lots` in `invoke` grep |
| No Month/Year toggle | No toggle code added |
| No tile header drill-down | Tile headers are `<span>`, not `<button>` |
| No compact panel redesign | `.cal-layout` flex unchanged; only breakpoint added |
| No unit tests | No `*.test.ts` / `*.spec.ts` files added |
| No `<input type="date">` | Zero grep matches |
| No new locales | Only `en` and `es` dictionaries touched |
| No existing event rename/reorder | `monthChange`, `viewYearChange`, `ariaOpenMonthPicker`, `ariaOpenYearPicker` untouched |

---

## Status

```
applyState:  ready_for_verify
dependencies.apply: done
dependencies.verify: ready (parent-owned, runtime required)
dependencies.archive: pending (parent-owned)
```

**Next recommended action:** `verify` phase — manual smoke matrix confirmation in `npm run tauri dev` + bounded review → then `archive`.
