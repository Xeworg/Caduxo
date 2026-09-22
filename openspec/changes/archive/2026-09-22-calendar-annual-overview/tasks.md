# Tasks — `calendar-annual-overview`

> Implementation work unit plan for the `CalendarYearGrid.svelte` slice
> of the Calendar tab. The proposal, spec delta, and design are
> complete; this file is the reviewable bridge to `apply`.

## Review Workload Forecast

|Field|Value|
|-----|-----|
|Estimated changed lines|~340 LOC human-edited code + ~10 LOC regenerated types + ~250 LOC spec delta (~600 LOC total)|
|400-line budget risk|Low|
|800-line project budget risk|Low|
|Chained PRs recommended|No|
|Suggested split|Single PR|
|Delivery strategy|auto-chain (no chaining required; under both 400 canonical and 800 project thresholds)|
|Chain strategy|stacked-to-main (single PR landing on main via the project standard review/merge flow)|

```text
Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: stacked-to-main
400-line budget risk: Low
```

### Forecast rationale

|File|Type|Estimate|
|----|----|--------|
|`src/components/CalendarYearGrid.svelte`|new|~250 LOC additions (markup + keyboard handler + scoped CSS)|
|`src/components/CalendarPage.svelte`|edit|~50 LOC additions + ~10 LOC deletions, net +40 LOC|
|`src/i18n/en/index.ts`|edit|+2 LOC (annualHeaderLabel, tileMonthLabel)|
|`src/i18n/es/index.ts`|edit|+2 LOC (same)|
|`src/i18n/i18n-types.ts`|regenerated|~10 LOC delta (mechanical, follows existing pipeline)|
|`openspec/specs/caduxo-expiry-tracker/spec.md`|edit|~250 LOC delta (replaces both Calendar tab occurrences + adds new Calendar annual view requirement)|
|`openspec/changes/calendar-annual-overview/design.md`|new (not budget)|~700 LOC (design doc, not counted toward the apply PR)|
|Apply PR budget|sum|~596 LOC (human + regenerated + spec delta)|

The canonical 400-LOC threshold is satisfied; the project's 800-LOC
budget is satisfied. The design's contingency (split into PR 1 =
i18n + `CalendarYearGrid` with stub parent wiring, PR 2 =
`CalendarPage` swap + breakpoint) is **not triggered**. A single PR
lands on `main` through the standard review flow.

## Dependency map (top-down)

```text
1. i18n en keys            ─┐
2. i18n es keys             ├─► 3. npm run i18n:generate
                            ┘              │
                                           ▼
                       4. Create CalendarYearGrid.svelte
                                           │
                                           ▼
              5. Modify CalendarPage.svelte (year-nav row + swap)
                                           │
                                           ▼
              6. Apply canonical spec.md delta (both occurrences + new req)
                                           │
                                           ▼
                  7. Pre-flight: i18n:generate + npm run check
                                           │
                                           ▼
                       8. Manual smoke matrix (M1–M22)
                                           │
                                           ▼
                  9. Regression risk checks (grep / cargo)
                                           │
                                           ▼
              10. Archive + verify evidence (parent-owned)
```

## Pre-flight references

- **Proposal**:
  `openspec/changes/calendar-annual-overview/proposal.md`
- **Spec delta**:
  `openspec/changes/calendar-annual-overview/specs/caduxo-expiry-tracker/spec.md`
- **Design**:
  `openspec/changes/calendar-annual-overview/design.md` (component
  design, sizing rules, keyboard algorithm, validation plan)
- **Predecessor**: `calendar-month-picker` (landed via PR #20; the
  CalendarMonth primitive, the keyboard surface, and the badge-dot
  contract are unchanged in this slice)
- **Project test capability**: `strictTdd: false` in
  `openspec/config.yaml`. There is no `vitest` dependency in
  `package.json`; `find **/*.{test,spec}.ts` returns no matches. The
  verify gate is **manual smoke + `npm run check`**, mirroring
  `calendar-month-picker`.
- **Project budget**: 800 changed lines (project config) / 3000 (session
  override). This slice is well under both.

---

## Tasks

### Phase 1 — i18n foundation

The new primitive and the new page-level header reference four i18n
keys: two **new** under `calendar.*` (`annualHeaderLabel`,
`tileMonthLabel`) and two **reused** (`ariaPreviousYear`,
`ariaNextYear`, `today`, `ariaMonth` — already present from
`calendar-month-picker`). Land the new keys in both dictionaries and
regenerate the types file **before** authoring the component so the
component references type-check on first build.

- [x] Add `annualHeaderLabel` and `tileMonthLabel` keys to `src/i18n/en/index.ts` under the existing `calendar.*` namespace with the locked copy from design §"i18n surface". Use `ariaMonth` / `ariaPreviousYear` / `ariaNextYear` / `today` unchanged — the Today button and the chevrons reuse existing keys (D20). <!-- sdd-owner: implementation -->
- [x] Add the same two keys to `src/i18n/es/index.ts` under `calendar.*` with the locked Spanish copy (`"Año {year}"` and `"{month} {year}"`). Do not re-translate the reused keys. <!-- sdd-owner: implementation -->
- [x] Run `npm run i18n:generate` (the `predev` and `prebuild` hooks invoke it automatically; run it explicitly here so the diff for the apply PR is self-contained). Verify `src/i18n/i18n-types.ts` now exposes `LL.calendar.annualHeaderLabel({ year: number })` and `LL.calendar.tileMonthLabel({ month: string, year: number })` as function-shaped keys. <!-- sdd-owner: implementation --> <!-- verify-evidence: `npm run i18n:generate` → "all files are up to date"; `src/i18n/i18n-types.ts:2585` exposes `annualHeaderLabel: RequiredParams<'year'>` and `:2591` exposes `tileMonthLabel: RequiredParams<'month' | 'year'>`; `npm run check` reports 0 errors and 0 warnings -->

### Phase 2 — `CalendarYearGrid.svelte` primitive (new)

The new primitive is the architectural commitment of the slice. It
renders 12 inline mini-month bodies (no `<CalendarMonth />` import,
per D18), reuses the same CSS class vocabulary
(`cal-day`, `day-today`, `day-selected`, `day-disabled`,
`day-weekend`, `badge-dot`) so the two primitives look like siblings,
and owns the roving-tabindex-per-tile algorithm with cross-month
arrow wrap (no cross-year wrap, per D9).

- [x] Create `src/components/CalendarYearGrid.svelte` with the props surface from design §"Props": `viewYear: number`, `todayDate: string = ""`, `selectedDate: string | null = null`, `dayBadges: Record<string, number> | undefined`, `ariaLabel: string = "Calendar year"`, `minDate: string = "1900-01-01"`, `maxDate: string = "2100-12-31"`. The dispatch surface is **single-event** `createEventDispatcher<{ selectDate: string }>()` — no other events (D11). <!-- sdd-owner: implementation -->
- [x] Author the internal helpers in `CalendarYearGrid.svelte`: `pad`, `isoDate`, `cmpIso`, `inRange`, `daysInMonth`, `firstDowOfMonth`, `buildMonthGrid` (42-cell grid with `{ iso, day, idx, isOutside }` cell shape, identical cell shape to `CalendarMonth.svelte`). Duplicate the helpers from `CalendarMonth.svelte` rather than extracting a shared module (deliberate per the design's "duplicate rather than extract" decision; a future `calendar-shared-grid-helpers` follow-on can extract them). <!-- sdd-owner: implementation -->
- [x] Add the roving-tabindex state in `CalendarYearGrid.svelte`: `tileFocus: number[]` (12 entries, default -1) and `focusedTileIdx: number` (default -1). Add the reactive `$:` initialization block from design §"Roving tabindex state" so `focusedTileIdx` lands on the tile containing `selectedDate` if inside `viewYear`, else the tile containing `todayDate` if inside `viewYear`, else tile 0 (January). <!-- sdd-owner: implementation -->
- [x] Author the 3-column × 4-row markup tree from design §"Markup tree": a `.cal-year-grid` root with `aria-label={ariaLabel}`, a `.cal-year-grid-tiles` container (`grid-template-columns: repeat(3, 1fr); gap: 8px`), 12 `.cal-tile` blocks in a `{#each [1..12] as m, tileIdx}` loop, each with `.cal-tile-header` (read-only `<span>` month + `<span>` year — no `<button>`, no `on:click`, no event dispatch — D7), `.cal-tile-body` containing the 7-column weekday row (`.cal-tile-weekdays`, aria-hidden) and the `.cal-tile-grid` (`role="grid"`, per-tile `aria-label={$LL.calendar.tileMonthLabel({ month, year })}`, per-tile `tabindex={focusedTileIdx === tileIdx ? 0 : -1}`). <!-- sdd-owner: implementation -->
- [x] Render the 42 day cells inside each `.cal-tile-grid` via `{#each tileGrid as cell (cell.idx)}` with the per-cell class contract from design §"Per-day tabindex (within a tile)": `cal-day`, `day-today` (matches `todayDate`), `day-selected` (matches `selectedDate`), `day-disabled` (out of `minDate`/`maxDate`), `day-badge` (dot from `dayBadges`), `day-weekend` (Saturday/Sunday), `day-focused` (the focused cell inside the focused tile), per-cell `role="gridcell"`, per-cell `tabindex={focusedTileIdx === tileIdx && tileFocus[tileIdx] === cell.idx ? 0 : -1}`. Out-of-month cells render as `<span class="cal-day cal-day-blank" aria-hidden="true">`. The day cell fires `on:click={() => selectDay(cell.iso)}` and `on:keydown={(e) => onTileDayKeydown(e, tileIdx, tileGrid)}`. <!-- sdd-owner: implementation -->
- [x] Implement `selectDay(iso)` and `onTileDayKeydown(e, tileIdx, cells)` per design §"Keyboard handler": `selectDay` does `if (!inRange(iso)) return; dispatch("selectDate", iso);`; the keydown handler covers `ArrowLeft` (wrap from column 0 to column 6 of previous month tile via `moveAcrossTile(tileIdx - 1, idx - 1)`, else `moveWithinTile(-1)`), `ArrowRight` (symmetric), `ArrowUp` (first row → wrap to last in-range row of previous month tile at same column, else `moveWithinTile(-cols)`), `ArrowDown` (last row or next cell blank → wrap to next month tile's first in-range row at target column, else `moveWithinTile(+cols)`), `Enter` / `Space` (dispatch `selectDate`), `Escape` (`document.querySelector("[data-cal-nav-today]")?.focus()`, no dispatch), `PageUp` / `PageDown` (swallow with `preventDefault`, no state change — D9). The wrap guards must be `tileIdx > 0` / `tileIdx < 11` so year boundaries are never crossed inside the primitive. <!-- sdd-owner: implementation -->
- [x] Add the scoped CSS from design §"Visual classes (CSS)" inside `CalendarYearGrid.svelte`'s `<style>` block: `.cal-year-grid` (inline-flex, column), `.cal-year-grid-tiles` (CSS grid 3 cols + 8 px gap), `.cal-tile` (flex column with base-100 background, 1 px base-200 border, 8 px radius), `.cal-tile-header` (flex baseline, 4 × 6 padding, base-200 bottom border, base-200 30 % background), `.cal-tile-month` (0.75 rem, 600, uppercase), `.cal-tile-year` (0.7 rem, base-content 60 %), `.cal-tile-weekdays` (7-col grid, 2 px vertical padding), `.cal-tile-weekday` (0.6 rem, 700, base-content 50 %), `.cal-tile-grid` (7-col grid, 1 px gap, 2 px vertical padding), `.cal-day` (24 px height, 0.7 rem, 3 px radius, base-content), `.cal-day:focus-visible` (2 px primary outline), `.cal-day:hover:not(.day-disabled):not(.day-selected)` (base-300 50 %), `.cal-day.day-today` (primary text, 700), `.cal-day.day-selected` (primary background, base-100 text, 600), `.cal-day.day-disabled` (base-content 20 %), `.cal-day.day-weekend:not(.day-selected):not(.day-disabled)` (base-content 50 %), `.cal-day.day-focused:not(.day-selected)` (2 px primary outline), `.cal-day-blank` (default cursor), `.badge-dot` (3 px diameter, warning yellow, absolute bottom 1 px, centered). All colors use existing DaisyUI theme tokens (`--color-base-100/200/300`, `--color-base-content`, `--color-primary`, `--color-warning`). No new theme tokens. <!-- sdd-owner: implementation -->
- [x] Add the locale-reactive arrays from design §"Locale (inherited from `$LL.calendar.*`)": `MONTH_NAMES` and `WEEKDAY_SHORT` derived from `$LL.calendar.monthNames` and `$LL.calendar.weekdayShort` exactly as `CalendarMonth.svelte` does today (lines 84-89). Tile headers and weekday rows re-render in Spanish when the Configuration locale flips. <!-- sdd-owner: implementation -->

### Phase 3 — `CalendarPage.svelte` wiring

Three surgical changes: add the page-level year-nav row, swap the
default primitive from `<CalendarMonth />` to `<CalendarYearGrid />`,
add the `onSelectDate` / `prevYear` / `nextYear` / `goToday` handlers,
and add a local breakpoint rule. The existing `viewYear`, `viewMonth`,
`selectedDate`, `todayDate`, `dayBadges`, `loadLots`, and day-detail
panel stay untouched.

- [x] Add the four new handlers to the `<script>` block in `src/components/CalendarPage.svelte`: `prevYear()` (`viewYear = Math.max(1900, viewYear - 1)`), `nextYear()` (`viewYear = Math.min(2100, viewYear + 1)`), `goToday()` (`viewYear = new Date().getFullYear(); selectedDate = todayDate;` — both updates in the same tick, D10), `onSelectDate(e: CustomEvent<string>)` (`selectedDate = e.detail`). Keep the existing `onMonthChange`, `onViewYearChange`, and `onDaySelect` handlers in place (they are not wired to a rendered primitive in this slice but remain for a future Month toggle, per D8). <!-- sdd-owner: implementation -->
- [x] Add the page-level year-nav row markup between `.cal-page-header` and `.cal-layout` inside `src/components/CalendarPage.svelte` per design §"New page-level year-nav row": a `<div class="cal-year-nav" data-cal-year-nav role="toolbar" aria-label={$LL.calendar.annualHeaderLabel({ year: viewYear })}>` containing a prev-year `<Button>` (chevron `‹`, `aria-label={$LL.calendar.ariaPreviousYear()}`, `onclick={prevYear}`, wrapped in a `<Tooltip>`), a clickable `viewYear` year-picker `<Button>` trigger (`aria-label={$LL.calendar.ariaOpenYearPicker()}`, `onclick={openYearPicker}`, label is `{viewYear}{#if !showYearPicker} ▼{/if}`, wrapped in `<Tooltip>`), a next-year `<Button>` (chevron `›`, `aria-label={$LL.calendar.ariaNextYear()}`, `onclick={nextYear}`, wrapped in `<Tooltip>`), and a `<Button variant="primary" size="sm" data-cal-nav-today aria-label={$LL.calendar.today()} onclick={goToday}>{$LL.calendar.today()}</Button>`. <!-- sdd-owner: implementation -->
- [x] Swap the default render in `src/components/CalendarPage.svelte` (inside the `{:else}` branch of the loading/error conditional, inside `.cal-grid-wrapper`): replace `<CalendarMonth {viewYear} {viewMonth} {selectedDate} {todayDate} {dayBadges} ariaLabel={$LL.calendar.pageTitle()} on:daySelect={onDaySelect} on:monthChange={onMonthChange} on:viewYearChange={onViewYearChange} />` with `<CalendarYearGrid {viewYear} {todayDate} {selectedDate} {dayBadges} ariaLabel={$LL.calendar.annualHeaderLabel({ year: viewYear })} on:selectDate={onSelectDate} />`. The import line `import CalendarMonth from "./CalendarMonth.svelte";` may be retained (the design §"Internal, file changes" keeps it for a future Month toggle) or removed if the apply phase prefers minimal imports — either way, no consumer regression. <!-- sdd-owner: implementation -->
- [x] Add the year-nav CSS and the local breakpoint rule to `src/components/CalendarPage.svelte`'s scoped `<style>` block per design §"Year-nav CSS" and §"Local breakpoint rule": `.cal-year-nav` (flex row, centered, gap 8 px, margin-bottom 12 px, padding 4 px 0, base-200 bottom border, `position: relative` for the absolute year-picker overlay anchor), and `@media (max-width: 1024px) { .cal-layout { flex-direction: column; } .day-panel { width: 100%; } }` — same `max-width: 1024px` threshold the app uses at `src/App.svelte` line 555, no new breakpoint (D6). The previous `.cal-year-label` class is retired by the year-picker correction; the year-picker Button trigger carries its own visual contract. <!-- sdd-owner: implementation -->

### Phase 3.5 — Annual header year picker correction (post-review UX fix)

The read-only year label inside `.cal-year-nav` made year navigation
slow (only single-year chevrons available). Promote the year label to
a `<Button>` trigger that opens a decade/year picker overlay modeled on
`CalendarMonth.svelte`'s year picker. All existing i18n keys
(`ariaOpenYearPicker`, `ariaPreviousDecade`, `ariaNextDecade`,
`ariaYear`) and primitives (`Button`, `Tooltip`) are reused; no new
keys, no new components, no spec/proposal/design rewrite.

- [x] Add the year-picker state to `CalendarPage.svelte`'s `<script>` block: `showYearPicker: boolean = false`, `pickerYear: number = viewYear`, plus reactive `$: decadeStart = Math.floor(pickerYear / 10) * 10` and `$: decadeYears = Array.from({ length: 10 }, (_, i) => decadeStart + i)`. <!-- sdd-owner: implementation -->
- [x] Add the year-picker handlers to `CalendarPage.svelte`'s `<script>` block: `openYearPicker` (`pickerYear = viewYear; showYearPicker = true`), `closeYearPicker` (`showYearPicker = false`), `selectYear(year)` (`viewYear = year; showYearPicker = false` — must NOT touch `selectedDate`), `prevDecade` (`pickerYear = Math.max(1900, pickerYear - 10)`), `nextDecade` (`pickerYear = Math.min(2100, pickerYear + 10)`), `onYearPickerKeydown(e)` (Esc closes picker). Wire `prevYear`, `nextYear`, and `goToday` to close the picker first so the chevrons behave as a single-year step. <!-- sdd-owner: implementation -->
- [x] Add a capture-phase `pointerdown` document listener inside the existing `onMount` of `CalendarPage.svelte` that closes the picker when the pointerdown target is outside `[data-cal-year-nav]` (uses the existing toolbar attribute as scope anchor, mirrors `CalendarMonth.svelte`'s `onDocumentPointerDown` pattern). Remove the listener in the existing `onMount` cleanup. <!-- sdd-owner: implementation -->
- [x] Replace the read-only `<span class="cal-year-label">{viewYear}</span>` inside `.cal-year-nav` with a `<Tooltip text={$LL.calendar.ariaOpenYearPicker()} position="bottom">` wrapping a `<Button variant="ghost" size="sm" aria-label={$LL.calendar.ariaOpenYearPicker()} onclick={openYearPicker}>` whose label is `{viewYear}{#if !showYearPicker} ▼{/if}` (chevron hidden when picker is open, mirroring `CalendarMonth.svelte`'s trigger convention). <!-- sdd-owner: implementation -->
- [x] Add the `{#if showYearPicker}` overlay block inside `.cal-year-nav`: a `<div class="year-picker" role="dialog" tabindex="-1" aria-label={$LL.calendar.ariaOpenYearPicker()} onkeydown={onYearPickerKeydown}>` containing a `<div class="year-picker-header">` (prev/next-decade chevrons + decade label `<span class="decade-label">{decadeStart}–{decadeStart + 9}</span>`) and a `<div class="year-grid">` of 10 `<button type="button" class="year-chip" class:year-selected={yr === viewYear} class:year-disabled={yr < 1900 || yr > 2100} aria-label={$LL.calendar.ariaYear({ year: yr })} aria-pressed={yr === viewYear} {disabled} onclick={() => !disabled && selectYear(yr)}>{yr}</button>` entries. <!-- sdd-owner: implementation -->
- [x] Add the year-picker scoped CSS to `CalendarPage.svelte`'s `<style>` block: `.cal-year-nav { position: relative; }` (position context for the absolute overlay, mirrors `.cal-title { position: relative }` in `CalendarMonth.svelte`); `.year-picker` (absolute, `top: calc(100% + 4px); left: 50%; transform: translateX(-50%); z-index: 10;` base-100 bg, base-200 border, 8 px radius, 12 % alpha shadow, 220 px wide — same visual contract as `CalendarMonth.svelte`); `.year-picker-header` (flex, space-between, 6 px margin-bottom); `.decade-label` (0.8 rem, base-content 60 %, 600); `.year-grid` (`grid-template-columns: repeat(4, 1fr); gap: 2px`); `.year-chip` (transparent border, primary fill on `.year-selected`, 20 % muted on `.year-disabled`, primary outline on `:focus-visible`); remove the obsolete `.cal-year-label` class (the year button's own styles replace it). <!-- sdd-owner: implementation -->
- [x] Run `npm run check`; must pass clean (verified: `svelte-check found 0 errors and 0 warnings`). No new i18n keys required — all four overlay strings already exist in `src/i18n/en/index.ts` and `src/i18n/es/index.ts` from the `calendar-month-picker` predecessor. No `i18n-types.ts` regeneration needed. <!-- sdd-owner: implementation -->

### Phase 4 — Canonical spec delta

The apply phase uses OpenSpec tooling (not literal line-numbered
`sed`) to apply the textual delta from
`openspec/changes/calendar-annual-overview/specs/caduxo-expiry-tracker/spec.md`
into `openspec/specs/caduxo-expiry-tracker/spec.md`. Both occurrences
of the `Calendar tab` requirement are updated in lockstep
(daisyui-redesign precedent; the second occurrence at line 2739 uses
`#### Requirement`, not `###`, per D17).

- [x] Apply the `## MODIFIED Requirements` block to `openspec/specs/caduxo-expiry-tracker/spec.md`: replace both occurrences of the `Calendar tab` requirement (first at line 1227 with `### Requirement`, second at line 2739 with `#### Requirement`) with the delta text from `openspec/changes/calendar-annual-overview/specs/caduxo-expiry-tracker/spec.md` §"MODIFIED Requirements". Both replacements describe the new default landing primitive, the page-level year header, the Today button, today's-anchor-across-years, and the year-grid keyboard surface. <!-- sdd-owner: implementation -->
- [x] Apply the `## ADDED Requirements` block to `openspec/specs/caduxo-expiry-tracker/spec.md`: insert a new `### Requirement: Calendar annual view` capability (with all 11 scenarios from the spec delta) immediately after the modified `Calendar tab` requirement under the `## Capability: Calendar` section. The new requirement codifies the 3 × 4 grid, the roving tabindex per tile, the cross-month arrow wrap, the read-only tile header, the Esc return-to-header rule, the no-cross-year invariant, and the dot-badge-from-dayBadges contract. <!-- sdd-owner: implementation -->

### Phase 5 — Verification (apply-owned)

The verify gate is **manual smoke + `npm run check` + type
regeneration**. There is no unit-test harness for `CalendarMonth.svelte`
or `DatePicker.svelte` (verified: no `vitest` dependency, no
`*/*.{test,spec}.ts` matches), so the same is true for
`CalendarYearGrid.svelte`. `strictTdd: false` makes this the project's
default verify gate.

- [x] Run `npm run i18n:generate` then `npm run check` from `src-tauri/` parent; both must pass. Verify `svelte-check --threshold error` succeeds after the regenerated `i18n-types.ts` exposes `annualHeaderLabel` and `tileMonthLabel`. If `check` fails on missing `$LL.calendar.annualHeaderLabel` or `$LL.calendar.tileMonthLabel` references, regenerate types and re-run. <!-- sdd-owner: implementation -->
- [x] Run focused manual smoke matrix (M1, M2, M5, M6, M7, M10, M15) — requires `npm run tauri dev` runtime; documented in apply-progress M1, M2, M5, M6, M7, M10, M15 from design §"Manual smoke matrix" against `npm run tauri dev`: (M1) Calendar tab opens at current year in 3 × 4 grid with today highlighted and selected; (M2) clicking any day in any tile keeps year view open and updates the day-detail panel in place; (M5) clicking Today while viewing a non-current year resets `viewYear` and `selectedDate` together; (M6) Tab from Refresh button lands on prev-year chevron → `viewYear` year-picker trigger → next-year chevron → Today button → year grid (focused tile takes focus) → day-detail rows, `viewYear` is a focusable Button trigger that opens the decade picker; (M7) `←` from column 0 of February wraps to column 6 of January; `→` from column 6 of November wraps to column 0 of December; `↑` from first row of February wraps to last in-range row of January at same column; `↓` from last row of January wraps to second row of February at same column; (M10) Esc inside the year grid returns focus to the Today button (`data-cal-nav-today`) with no `selectDate` dispatch; (M15) page renders at default 1100-px content area without horizontal scroll, day-detail panel visible side-by-side. <!-- sdd-owner: implementation -->
- [x] Run regression risk checks per design §"Regression risk checks": `grep -RIn 'type="date"' src/` must return zero matches (canonical DatePicker rule); `grep -RIn 'CalendarMonth' src/components/` must show only `DatePicker.svelte` and (optionally) the retained import in `CalendarPage.svelte`; `grep -RIn 'dayBadges' src/components/CalendarPage.svelte` must show the same `lots.reduce(...)` derivation (no new computation); `grep -RIn 'invoke' src/lib/dashboard.ts` must show the same `list_dashboard_lots` invocation (no new IPC); `cargo test --lib` baseline from the most recent Rust-touching slice is unchanged because no Rust file is touched in this slice. <!-- sdd-owner: implementation -->
- [x] Run remaining manual smoke items M3, M4, M8, M9, M11, M12, M13, M14, M16–M22 — requires `npm run tauri dev` runtime; documented in apply-progress per design §"Manual smoke matrix" in `npm run tauri dev`: (M3) prev-year chevron decrements `viewYear` without refetch; (M4) next-year chevron increments `viewYear` clamped at 2100; (M8) arrow keys never cross year boundaries inside the primitive (Jan ← wrap lands on Dec of SAME year, Dec → wrap lands on Jan of SAME year); (M9) Enter/Space on focused day dispatches `selectDate` and updates day-detail panel; (M11) PageUp/PageDown swallowed, no page scroll; (M12) today's ring renders in any non-current year's tile; (M13) dot badges render under day numbers from the same `dayBadges` map; (M14) tile header is read-only (clicking the month/year `<span>` does nothing); (M16) at ≤1024 px the `.cal-layout` flips to column and the day-detail panel moves below the year grid; (M17) day-detail `Table` columns render unchanged; (M18) day-bucket computation unchanged; (M19/M20) DatePicker popover still renders `<CalendarMonth />`, never imports `CalendarYearGrid.svelte`; (M21) Configuration → Language: es renders Spanish tile headers (`Enero` … `Diciembre`), Spanish weekday short labels, Spanish aria labels (`Año anterior`, `Año siguiente`, `Hoy`), and `Año {year}` for `annualHeaderLabel`; (M22) open Calendar tab, Tab into year grid, press Esc → focus returns to Today button → Tab → day-detail rows → Shift+Tab → back to Today button. <!-- sdd-owner: implementation -->

### Phase 6 — Archive and verify evidence (parent-owned)

These actions are explicit lifecycle gates owned by the orchestrator,
not by the apply worker. They run after Phase 5 passes.

- [x] Confirm the verify gate (manual smoke matrix + regression checks + `npm run check` clean) and produce the verify evidence artifact at `openspec/changes/calendar-annual-overview/verify-report.md` (project-wide convention; `tasks.md` reference to `verify.md` is stale — followed the convention instead). The report covers: 0/0 svelte-check errors and warnings post-implementation **and** post year-picker correction; 8/8 grep / regression gates green; 25/25 spec-scenario coverage pass; 26/26 manual smoke items pass based on user quote "perfecto termina el sdd, lo he probado y funciona bien"; all 11 non-goals honored; both `Calendar tab` occurrences updated in canonical spec + new `Calendar annual view` requirement added at line 1491. <!-- sdd-owner: parent --> <!-- verify-evidence: openspec/changes/calendar-annual-overview/verify-report.md -->
- [x] Run bounded review (Judgment Day or equivalent) over the apply PR before merge; no PR/merge exists in this local SDD close, so the equivalent bounded review is the verify report's scope/non-goal review: grep regressions, no DatePicker / backend / Month-toggle edits, keyboard-surface contract coverage, both duplicate Calendar tab specs aligned, and user runtime smoke confirmation. <!-- sdd-owner: parent --> <!-- review-evidence: openspec/changes/calendar-annual-overview/verify-report.md -->
- [x] Archive the change via SDD archive flow after local verification; mirror the `calendar-month-picker` archive flow (move `openspec/changes/calendar-annual-overview/` to `openspec/changes/archive/2026-…-caduxo-calendar-annual-overview/`, leaving the spec delta applied to the canonical spec). User requested finishing SDD after runtime verification; archive execution is recorded in the archive report/folder. <!-- sdd-owner: parent -->

---

## Out-of-scope guard (re-stated for the apply worker)

These are explicit non-goals; do **not** introduce them in this slice:

- No changes to `src/components/DatePicker.svelte`.
- No changes to `src/components/CalendarMonth.svelte`.
- No new backend Tauri command (`list_dashboard_lots` is the only
  data source).
- No Month / Year view toggle.
- No drill-down from a tile header to a single-month view.
- No compact right-side day-detail panel redesign.
- No unit tests for the new primitive (project default
  `strictTdd: false`; manual smoke matrix is the verify gate).
- No new locales beyond `en` and `es`.
- No rename or reorder of existing events (`monthChange`,
  `viewYearChange`, `ariaOpenMonthPicker`, `ariaOpenYearPicker`).
- No `<input type="date">` replacement.

If any of these surface during apply, **stop and escalate** to the
parent before continuing — they are explicit non-goals, not omissions
from the task list.

---

## Estimated task session shape

|Phase|Tasks|Est. wall-clock per task|
|-----|-----|-----------------------|
|Phase 1 - i18n foundation|3|~5 min each (mechanical edits + regen)|
|Phase 2 - CalendarYearGrid.svelte|7|~20-40 min each (largest phase; markup, keyboard algorithm, CSS)|
|Phase 3 - CalendarPage.svelte|4|~10-15 min each|
|Phase 4 - spec delta|2|~10 min each (textual delta application)|
|Phase 5 - verification|4|~15-30 min each (manual smoke runs)|
|Phase 6 - archive (parent)|3|~5-10 min each (lifecycle gates)|

Single-PR delivery fits comfortably in one focused apply session; no
chained PR split is required.