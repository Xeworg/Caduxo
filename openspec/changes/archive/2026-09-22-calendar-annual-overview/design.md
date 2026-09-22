# Design — calendar-annual-overview

## Executive summary

This design locks the implementation shape for the Calendar tab's
default landing primitive, switching it from a single-month
`CalendarMonth.svelte` to a new sibling `CalendarYearGrid.svelte` that
renders a 3-column × 4-row CSS grid of 12 mini-month tiles. The slice
is **frontend-only and additive inside the Calendar tab**, mirroring
the layering of the most recent predecessor (`2026-09-21-caduxo-calendar-month-picker`)
and the daisyui-redesign cleanup. The architectural commitment is a
**sibling primitive** that reuses `CalendarMonth.svelte`'s visual
contracts (badge dots, today ring, selection ring, weekend muted,
6-week day grid) so the two primitives look like siblings rather than
nested components. `DatePicker.svelte`, the year picker inside
`CalendarMonth.svelte`, the day-detail panel, the backend, and the
`list_dashboard_lots` IPC are all untouched.

Year navigation lives at the page level: a new
`.cal-year-nav` row inside `CalendarPage.svelte` above the year grid
contains a previous-year chevron, a clickable `viewYear` year-picker trigger, a
next-year chevron, and a `Today` button. The header owns all year
navigation; the new primitive knows nothing about it. The
`CalendarMonth.svelte` primitive remains the source of truth for the
keyboard surface and visual contracts inside each tile, but is **not
imported as a Svelte component** — the year-grid primitive renders its
12 mini-month bodies inline, reusing the same class contracts
(`cal-month`, `cal-grid`, `cal-day`, `day-today`, `day-selected`,
`day-disabled`, `day-badge`, `day-weekend`, `badge-dot`) so the two
primitives share a CSS vocabulary.

Keyboard navigation is **roving tabindex per tile** (only the focused
tile's grid owns `tabindex="0"`); arrow keys traverse within a tile
and wrap to the previous / next month tile at column boundaries;
cross-year navigation never happens inside the primitive (the page-
level chevrons are the only year-boundary path). Today's anchor ring
renders inside today's tile in any year the grid renders, so users
browsing a non-current year can still find the anchor.

The user-approved review budget for this change is **800 changed
lines** (project config; session cap 3000). Forecasted delta is
comfortably under that budget (see §10).

## Confirmed scope boundaries (locked from proposal)

| #   | Decision                                                                                                                                                 | Source                                                                  | Status  |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------- |
| D1  | Calendar tab opens in annual view by default                                                                                                             | proposal §Outcomes 1, decision 1                                         | locked  |
| D2  | Year grid layout is **3 columns × 4 rows** of mini-month tiles                                                                                            | proposal §Outcomes, decision 2                                          | locked  |
| D3  | Clicking a day **stays in annual view** and updates the day-detail panel; the panel shows expirations for the selected date                              | proposal §Outcomes 2, decision 3                                         | locked  |
| D4  | Year navigation uses page-level chevrons rendered as `‹ {viewYear} ›` above the year grid                                                                  | proposal §Outcomes 3, decision 4                                         | locked  |
| D5  | Today button in the page-level header returns to the current year and selects today                                                                       | proposal §Outcomes 4, decision 5                                         | locked  |
| D6  | Day-detail panel stays side-by-side with the year grid inside `.cal-layout`; collapses below the grid at the existing app breakpoint (`max-width: 1024px`) | proposal question 1 (assumed)                                            | locked  |
| D7  | Tile header is read-only in this slice; no drill-down to a single-month view                                                                              | proposal question 2 (assumed)                                            | locked  |
| D8  | Today's ring always renders inside today's tile in any year                                                                                              | proposal question 3 (assumed)                                            | locked  |
| D9  | Cross-tile arrow navigation wraps across month boundaries inside the year grid; year boundaries are NOT crossed inside the primitive                     | proposal question 4 (assumed)                                            | locked  |
| D10 | Today button updates `viewYear` and `selectedDate` together inside `CalendarPage.svelte`; no new event for the primitive                                   | proposal question 5 (assumed)                                            | locked  |
| D11 | Single event `selectDate: string` (ISO `YYYY-MM-DD`); `CalendarMonth.svelte`'s `daySelect` event is NOT re-used (separate dispatch surface for clarity)  | proposal §"In scope", spec delta                                         | locked  |
| D12 | Existing day-detail panel is unchanged in content; only its placement inside `.cal-layout` adapts via the page-level flex                                    | proposal §"In scope", spec delta                                         | locked  |
| D13 | `DatePicker.svelte` continues to render a single-month `<CalendarMonth />`; the year-grid primitive is consumed ONLY by `CalendarPage.svelte`             | proposal §Non-goals, spec delta                                          | locked  |
| D14 | No new backend Tauri command; `list_dashboard_lots` is the only data source                                                                                | proposal §Non-goals, spec delta                                          | locked  |
| D15 | No Month / Year view toggle; Year is the only landing mode in this slice                                                                                  | proposal §Non-goals, spec delta                                          | locked  |
| D16 | No unit tests for the new primitive; manual smoke matrix mirrors `calendar-month-picker`                                                                  | proposal §Non-goals, spec delta                                          | locked  |
| D17 | Both occurrences of `### Requirement: Calendar tab` in the canonical spec are updated (the second occurrence at line 2739 is `#### Requirement`, not `###`) | proposal §"Risks", daisyui-redesign precedent                            | locked  |
| D18 | `CalendarMonth.svelte` is **not** imported as a Svelte component by `CalendarYearGrid.svelte`; the year grid renders its 12 mini-month bodies inline, reusing the same class contracts | spec delta §"Calendar annual view" Scenario: DatePicker is not affected | locked  |
| D19 | No new i18n locale; `en` and `es` only                                                                                                                    | proposal §Non-goals, spec delta                                          | locked  |
| D20 | i18n keys are added under `calendar.*`: `annualHeaderLabel`, `tileMonthLabel`; existing `ariaPreviousYear`, `ariaNextYear`, `ariaMonth`, `today` are reused | proposal §"In scope", spec delta                                         | locked  |

These 20 decisions are the contract for `tasks.md` and `apply`. No new
product question round is required before `tasks.md`; the proposal's
product question round already settled every open decision surfaced by
exploration.

## Architecture decision

The slice is **frontend-only**, scoped to the new
`src/components/CalendarYearGrid.svelte`, the modified
`src/components/CalendarPage.svelte`, and the two i18n dictionaries
plus the regenerated type file. The Rust backend is untouched; no IPC
change; no host-page edit required for `DatePicker.svelte`.

The architectural commitment is **sibling-primitive reuse of visual
contracts**. `CalendarYearGrid.svelte` is a new file that owns the
3×4 tile grid, the per-tile 7-column day grid, the roving tabindex
algorithm, and the cross-tile arrow wrapping rule. It is **not** a
Svelte wrapper that imports `<CalendarMonth />` — it renders its 12
mini-month bodies inline and reuses the same CSS class vocabulary
(`cal-month`, `cal-header`, `cal-weekdays`, `cal-grid`, `cal-day`,
`day-today`, `day-selected`, `day-disabled`, `day-badge`, `day-weekend`,
`day-focused`, `badge-dot`) so the two primitives look like siblings.
The reasons for inline rendering rather than `<CalendarMonth />`
nesting:

1. **`CalendarMonth.svelte` carries its own header** (prev/next chevrons
   + month label + year chip + picker overlays). A 12-tile grid that
   imports `<CalendarMonth />` would render 12 redundant headers and 12
   picker overlays — exactly what the spec forbids.
2. **`CalendarMonth.svelte`'s keyboard surface owns `focusedIso` and
   `resetFocused()`** with assumptions about a single visible month.
   Nested instances would each carry their own `focusedIso` and the
   parent would lose the roving-tabindex invariant the spec requires.
3. **CSS class collisions** between 12 nested `.cal-month` blocks would
   make the styles hard to scope (Svelte's scoped CSS is per-component,
   but the day-cell sizing and the picker overlays would need to be
   re-scaled anyway inside a smaller tile).

So the year-grid primitive is a self-contained mini-month renderer that
shares the visual vocabulary but does not depend on `<CalendarMonth />`
as a Svelte component. The class contract is **the** contract; the
runtime contract between the two primitives is not.

```text
                       ┌──────────────────────────────────┐
                       │   src-tauri/list_dashboard_lots  │
                       │   (unchanged)                    │
                       └──────────────┬───────────────────┘
                                      │  active lots
                                      ▼
   ┌─────────────────────┐   composes   ┌───────────────────────────────┐
   │ DatePicker.svelte   │────────────▶│ CalendarMonth.svelte          │
   │ (typed text input + │              │  (single-month primitive,     │
   │  popover wrapper)   │              │   unchanged in this slice)    │
   │ UNCHANGED           │              │                               │
   └─────────────────────┘              └───────────────┬───────────────┘
              ▲                                         ▲
              │                                         │ shared visual
              │                                         │ contracts
              │                                         │ (CSS classes only)
              │                                         │
   ┌──────────┴──────────┐   ┌───────────────────────────┴───────────┐
   │ LotForm.svelte      │   │ CalendarPage.svelte                     │
   │ ReportsPage.svelte  │   │  • page-level year-nav row (NEW)       │
   │  (host sites)       │   │  • page-level header (title+refresh)   │
   │ UNCHANGED           │   │  • .cal-layout flex (NEW breakpoint)   │
   └─────────────────────┘   │  • new <CalendarYearGrid /> (NEW)      │
                            │    └─ 12 inline mini-month tiles        │
                            │  • day-detail panel (UNCHANGED)         │
                            └───────────────────────────────────────┘
```

Cross-cutting rules (locked):

- **No new Tauri command.** `list_dashboard_lots` is the only data
  source; the existing in-memory `lots: DashboardLotRow[]` array drives
  the `dayBadges` map and the day-detail `dayRows`. Year navigation does
  NOT trigger a refetch.
- **No portal, no popover anchor rewrite.** The year grid renders in
  normal document flow inside `.cal-grid-wrapper`. The day-detail panel
  remains the second column of `.cal-layout` at the default breakpoint
  and flips to a second row at the existing app breakpoint
  (`max-width: 1024px`).
- **No CSS framework change.** Component-scoped `<style>` block only;
  the new `.cal-year-grid`, `.cal-year-tile`, `.cal-tile-header`,
  `.cal-tile-body`, `.cal-tile-grid` classes mirror the existing
  `.cal-month` / `.cal-grid` / `.cal-day` vocabulary. Theme tokens
  (`--color-base-100`, `--color-base-content`, `--color-primary`,
  `--color-warning`, `--color-error`) are reused.
- **No new frontend test harness.** `strictTdd: false` in
  `openspec/config.yaml`; the proposal explicitly does not introduce
  unit tests for the new primitive. Manual smoke on the Calendar tab
  is the verify gate, mirroring `calendar-month-picker`.
- **No new i18n locale.** `en` and `es` only.
- **No Month / Year view toggle.** Year is the only landing mode; a
  Month toggle is a future slice.
- **No drill-down from a tile header.** Clicking the tile header does
  nothing in this slice; clicking the tile body emits `selectDate`.
- **`CalendarMonth.svelte` is unchanged.** No edits to the existing
  primitive in this slice.

## Component design

### `CalendarYearGrid.svelte` (new)

A new self-contained primitive that renders a 3-column × 4-row CSS
grid of 12 mini-month tiles for a single `viewYear`. The file is
~250 LOC including the inline mini-month body markup, the keyboard
handler, and the scoped CSS.

#### Props

```ts
export let viewYear: number;
export let todayDate: string = "";                          // ISO YYYY-MM-DD; "" disables today ring
export let selectedDate: string | null = null;              // ISO YYYY-MM-DD; null renders no selection ring
export let dayBadges: Record<string, number> | undefined = undefined;
export let ariaLabel: string = "Calendar year";             // a11y name for the outer region
export let minDate: string = "1900-01-01";
export let maxDate: string = "2100-12-31";
```

`viewMonth`, `focusedIso`, picker state, and the dispatch surface are
NOT props on this primitive — they are page-level concerns owned by
`CalendarPage.svelte`. The primitive is intentionally minimal: it owns
the `viewYear` it renders, the `selectedDate` it highlights, and the
`dayBadges` it draws. Everything else is the page's responsibility.

#### Single dispatch event

```ts
const dispatch = createEventDispatcher<{
  selectDate: string;            // ISO YYYY-MM-DD of the clicked day cell
}>();
```

Only one event is emitted by the primitive: `selectDate: string`. This
is a deliberate divergence from `CalendarMonth.svelte`'s `daySelect`
event name (D11) — the year view never has the granularity question
that month view has (always day-level selection), and the spec delta
codifies the new event name explicitly. The parent
`CalendarPage.svelte` adds a parallel `onSelectDate` handler that
mirrors `onDaySelect`.

#### Internal helpers (mirror of `CalendarMonth.svelte`)

```ts
function pad(n: number): string { return n < 10 ? "0" + n : String(n); }
function isoDate(year: number, month: number, day: number): string {
  return `${year}-${pad(month)}-${pad(day)}`;
}
function cmpIso(a: string, b: string): number { return a < b ? -1 : a > b ? 1 : 0; }
function inRange(iso: string): boolean {
  if (!iso) return false;
  return cmpIso(iso, minDate) >= 0 && cmpIso(iso, maxDate) <= 0;
}
function daysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}
function buildMonthGrid(year: number, month: number) {
  // 42-cell grid, leading/trailing blanks as { iso:"", day:0, isOutside:true }
  // identical to CalendarMonth.svelte's buildMonthGrid.
}
function firstDowOfMonth(year: number, month: number): number {
  return new Date(year, month - 1, 1).getDay(); // 0=Sun
}
```

These helpers are duplicates of the same-named helpers in
`CalendarMonth.svelte` (so `CalendarYearGrid.svelte` is
self-contained). The decision to duplicate rather than extract to a
shared `lib/calendar-grid.ts` module is deliberate for this slice: the
predecessor `calendar-month-picker` change did not extract the helpers
either, and the proposal locks "no refactor of `CalendarMonth.svelte`
into a generic multi-mode component" as a non-goal. A future
`calendar-shared-grid-helpers` follow-on can extract the helpers; this
slice does not.

#### Roving tabindex state

```ts
// Per-tile focused cell index. 12 entries, one per tile in render order
// (January → December). Each entry is the cell index 0..41 inside the
// tile's 42-cell grid, OR -1 if the tile has never received focus.
let tileFocus: number[] = Array.from({ length: 12 }, () => -1);

// The tile that owns tabindex="0" today (0..11) OR -1 if no tile is
// focused. Initial value: the tile containing selectedDate if it is
// inside viewYear, else the tile containing todayDate if it is inside
// viewYear, else tile 0 (January).
let focusedTileIdx: number = -1;
```

Initialization happens in a reactive block that re-runs whenever
`viewYear` or `selectedDate` changes:

```ts
$: {
  if (viewYear) {
    if (selectedDate) {
      const [y, m] = [parseInt(selectedDate.slice(0, 4)), parseInt(selectedDate.slice(5, 7))];
      if (y === viewYear && m >= 1 && m <= 12) {
        const d = parseInt(selectedDate.slice(8, 10));
        const tile = m - 1;
        const firstDow = firstDowOfMonth(viewYear, m);
        tileFocus[tile] = firstDow + (d - 1);   // 0-indexed cell inside tile
        focusedTileIdx = tile;
      }
    }
    if (focusedTileIdx === -1 && todayDate) {
      const [y, m] = [parseInt(todayDate.slice(0, 4)), parseInt(todayDate.slice(5, 7))];
      if (y === viewYear && m >= 1 && m <= 12) {
        const d = parseInt(todayDate.slice(8, 10));
        const tile = m - 1;
        const firstDow = firstDowOfMonth(viewYear, m);
        tileFocus[tile] = firstDow + (d - 1);
        focusedTileIdx = tile;
      }
    }
    if (focusedTileIdx === -1) {
      // Default to the first in-range day of the year (typically Jan 1)
      const tile = 0;
      const firstDow = firstDowOfMonth(viewYear, 1);
      tileFocus[tile] = firstDow;
      focusedTileIdx = tile;
    }
  }
}
```

The reactive block is the year-grid analogue of
`CalendarMonth.svelte`'s `resetFocused()`. It runs whenever `viewYear`,
`selectedDate`, or `todayDate` changes (any one of which can move the
anchor across tiles). When `selectedDate` is set from the parent via
`selectDate`, the reactive cascade re-runs and the focused tile moves
to the new tile.

#### Per-tile tabindex

```svelte
<div
  class="cal-tile-grid"
  role="grid"
  aria-label={$LL.calendar.tileMonthLabel({ month: MONTH_NAMES[m - 1], year: viewYear })}
  tabindex={focusedTileIdx === tileIdx ? 0 : -1}
>
```

Every tile's `cal-tile-grid` carries the same `role="grid"` pattern as
`CalendarMonth.svelte`'s `cal-grid`. The `tabindex` is `0` on exactly
one tile at any moment — the focused tile. The other 11 tiles carry
`tabindex="-1"` so `Tab` from the page-level header enters the year
grid at the focused tile and `Shift+Tab` returns to the Today button.

#### Per-day tabindex (within a tile)

Inside each tile, the 42 day cells (or 6-week grid of `cal-day`
buttons plus blank `<span>` placeholders for `isOutside: true`) carry
`tabindex="-1"` by default and `tabindex="0"` only on the focused cell
within the focused tile:

```svelte
<button
  type="button"
  role="gridcell"
  class="cal-day"
  class:day-today={cell.iso === todayDate}
  class:day-selected={cell.iso === selectedDate}
  class:day-disabled={!inRange(cell.iso)}
  class:day-badge={!!(dayBadges && cell.iso && dayBadges[cell.iso] > 0)}
  class:day-weekend={isWeekend(cell)}
  class:day-focused={focusedTileIdx === tileIdx && tileFocus[tileIdx] === cell.idx}
  aria-label={cellAriaLabel(cell, MONTH_NAMES[m - 1])}
  aria-selected={cell.iso === selectedDate}
  aria-disabled={!inRange(cell.iso)}
  tabindex={focusedTileIdx === tileIdx && tileFocus[tileIdx] === cell.idx ? 0 : -1}
  disabled={!inRange(cell.iso)}
  on:click={() => !isDisabled(cell) && selectDay(cell.iso)}
  on:keydown={onTileDayKeydown}
>
  {cell.day}
  {#if hasBadge}<span class="badge-dot" aria-hidden="true"></span>{/if}
</button>
```

`cell.idx` is the 0-indexed cell inside the tile (0..41), as produced
by `buildMonthGrid`. `tileFocus[tileIdx]` is the focused cell inside
the focused tile; all other cells (including all cells in the other 11
tiles) carry `tabindex="-1"`.

#### Keyboard handler (one per tile, shared)

```ts
function onTileDayKeydown(e: KeyboardEvent, tileIdx: number, cells: Cell[]) {
  const idx = tileFocus[tileIdx];
  if (idx < 0) return;
  const cols = 7;
  const cell = cells[idx];

  function clampToTile(nextIdx: number) {
    if (nextIdx < 0) nextIdx = 0;
    if (nextIdx > 41) nextIdx = 41;
    return nextIdx;
  }

  function moveWithinTile(delta: number) {
    const next = clampToTile(idx + delta);
    tileFocus[tileIdx] = next;
    // Update DOM tabindex synchronously and refocus
    focusCell(tileIdx, next);
  }

  function moveAcrossTile(targetTile: number, targetCell: number) {
    tileFocus[tileIdx] = -1; // leave the current tile's grid with no focus
    focusedTileIdx = targetTile;
    tileFocus[targetTile] = targetCell;
    // The target tile's grid now owns tabindex=0; focus its day cell.
    focusCell(targetTile, targetCell);
  }

  switch (e.key) {
    case "ArrowLeft": {
      e.preventDefault();
      const col = idx % cols;
      if (col === 0 && tileIdx > 0) {
        // Wrap to column 6 of previous month tile at the SAME row
        // (idx - 1 falls into the previous tile's last column when
        // idx % cols === 0, so the previous tile's last column is at
        // the same row). Use the cell at idx - 1 inside the previous tile.
        moveAcrossTile(tileIdx - 1, idx - 1);
      } else {
        moveWithinTile(-1);
      }
      return;
    }
    case "ArrowRight": {
      e.preventDefault();
      const col = idx % cols;
      if (col === cols - 1 && tileIdx < 11) {
        moveAcrossTile(tileIdx + 1, idx + 1);
      } else {
        moveWithinTile(+1);
      }
      return;
    }
    case "ArrowUp": {
      e.preventDefault();
      if (idx < cols) {
        // First row of the tile — wrap to last row of previous month tile
        if (tileIdx > 0) {
          // Target cell in previous tile: the same column, last row that
          // contains a day in that month. If the previous month's last
          // week has a day at this column, the wrapped cell is in the
          // previous tile's last row. If the previous month doesn't have
          // a day at this column (e.g., its last week has blanks at this
          // column), clamp to the previous tile's last in-range cell.
          const targetTile = tileIdx - 1;
          const targetCol = idx % cols;
          // The previous tile's grid is 42 cells; the last in-range cell
          // is at `daysInMonth(viewYear, targetTile + 1) - 1` plus the
          // leading-blank offset.
          const prevMonth = targetTile + 1;
          const prevFirstDow = firstDowOfMonth(viewYear, prevMonth);
          const prevTotalDays = daysInMonth(viewYear, prevMonth);
          const prevLastCellIdx = prevFirstDow + (prevTotalDays - 1);
          // Clamp targetIdx to the cell at the same column in the last
          // week of the previous month, or to prevLastCellIdx if that
          // column is blank.
          const targetIdx = Math.min(prevLastCellIdx, prevLastCellIdx - (prevLastCellIdx % cols) + targetCol);
          moveAcrossTile(targetTile, targetIdx);
        } else {
          moveWithinTile(0); // January first row, no wrap
        }
      } else {
        moveWithinTile(-cols);
      }
      return;
    }
    case "ArrowDown": {
      e.preventDefault();
      const tileLen = cells.length; // 42
      if (idx + cols >= tileLen || !cells[idx + cols].iso) {
        // Last row of the tile or the cell below is blank — wrap to
        // next month tile's second week row (or its last in-range row
        // if the next month doesn't have a day at this column).
        if (tileIdx < 11) {
          const targetTile = tileIdx + 1;
          const targetCol = idx % cols;
          const nextMonth = targetTile + 1;
          const nextFirstDow = firstDowOfMonth(viewYear, nextMonth);
          // Walk down from the first row until we find a cell at the
          // target column that is in-range.
          let targetIdx = nextFirstDow + targetCol;
          if (targetIdx >= 42 || !cells_at_tile_first_row_plus[targetCol]?.iso) {
            // No day at target column in next month — find the closest
            // in-range cell at or below targetCol in the next month.
            targetIdx = nextFirstDow; // fallback
          }
          moveAcrossTile(targetTile, targetIdx);
        } else {
          moveWithinTile(0); // December last row, no wrap
        }
      } else {
        moveWithinTile(+cols);
      }
      return;
    }
    case "Enter":
    case " ":
      e.preventDefault();
      if (cell.iso && inRange(cell.iso)) {
        dispatch("selectDate", cell.iso);
      }
      return;
    case "Escape":
      e.preventDefault();
      // Blur the focused cell and return focus to the page-level header
      if (typeof document !== "undefined") {
        const prev = document.querySelector<HTMLElement>("[data-cal-year-nav] [data-cal-nav-today]");
        prev?.focus();
      }
      return;
    case "PageUp":
    case "PageDown":
      // Not exposed by the year-grid primitive (D9). Swallow to prevent
      // the page from scrolling.
      e.preventDefault();
      return;
    default:
      return;
  }
}
```

Notes on the algorithm:

- **Cross-tile wrap is only across month boundaries inside the same
  `viewYear`.** `tileIdx > 0` / `tileIdx < 11` bounds keep the wrap
  inside the 12-tile range; the year-grid primitive never touches
  `viewYear` from the keyboard (D9).
- **Up/Down wrap targets the column-aligned row** in the adjacent
  tile. When the adjacent month has no in-range day at the target
  column (e.g., February's first week has 0-2 blank cells at column
  0-1), the algorithm falls back to the closest in-range cell at or
  below the target column in the adjacent month, mirroring the spec's
  "wrap to previous month tile's last week row at column 3" rule.
- **Esc returns focus to the page-level header** (specifically, the
  Today button, marked with `data-cal-nav-today`). This is the
  spec's "Esc returns focus to the page-level header" rule.
- **PageUp / PageDown are explicitly swallowed** to prevent the page
  from scrolling and to honor D9.
- **`focusCell(tileIdx, cellIdx)`** is a small helper that (1) sets
  `tileFocus[tileIdx] = cellIdx` (2) calls `.focus()` on the
  corresponding `<button>` element via a `data-cell-tile` /
  `data-cell-idx` attribute lookup. It does not run a Svelte reactive
  cycle (the keyboard handler runs synchronously inside the event
  loop, so direct DOM focus is correct here).

#### Tile header (read-only)

```svelte
<div class="cal-tile">
  <div class="cal-tile-header" aria-hidden="false">
    <span class="cal-tile-month">{MONTH_NAMES[m - 1]}</span>
    <span class="cal-tile-year">{viewYear}</span>
  </div>
  <div class="cal-tile-body">
    <!-- weekday row (7 cells, aria-hidden) -->
    <div class="cal-tile-weekdays" aria-hidden="true">
      {#each WEEKDAY_SHORT as wd}<span class="cal-tile-weekday">{wd}</span>{/each}
    </div>
    <!-- day grid (42 cells, 7 cols) -->
    <div class="cal-tile-grid" role="grid" tabindex={focusedTileIdx === tileIdx ? 0 : -1}
         aria-label={$LL.calendar.tileMonthLabel({ month: MONTH_NAMES[m - 1], year: viewYear })}>
      {#each tileGrid as cell (cell.idx)}
        …{#if cell.iso && !cell.isOutside}…{:else}<span class="cal-day cal-day-blank" aria-hidden="true"></span>{/if}
      {/each}
    </div>
  </div>
</div>
```

The tile header is **read-only** in this slice (D7). It is rendered as
two `<span>` elements (no `<button>`, no `on:click`, no `tabindex`).
The `aria-label` on the tile's `cal-tile-grid` provides the accessible
name so a screen reader announces "Mayo 2027" or "May 2027" when focus
enters the tile's grid.

`MONTH_NAMES` and `WEEKDAY_SHORT` are computed from
`$LL.calendar.monthNames` and `$LL.calendar.weekdayShort` — the exact
same reactive pattern `CalendarMonth.svelte` uses today (lines 84-89).

#### Markup tree

```text
.cal-year-grid              ← root, role="application", aria-label={ariaLabel}
└── .cal-year-grid-tiles    ← CSS Grid (repeat(3, 1fr), 4 rows)
    ├── .cal-tile           ← tile 0 (January)
    │   ├── .cal-tile-header
    │   │   ├── .cal-tile-month  "January"
    │   │   └── .cal-tile-year   "2027"
    │   └── .cal-tile-body
    │       ├── .cal-tile-weekdays (7 spans, aria-hidden)
    │       └── .cal-tile-grid    role="grid" tabindex={0|-1}
    │           └── (42 .cal-day or .cal-day-blank cells)
    ├── .cal-tile           ← tile 1 (February) … tile 11 (December)
    └── …
```

#### Sizing rules (locked)

| Element                       | Width                              | Notes |
|-------------------------------|------------------------------------|-------|
| `.cal-year-grid` root         | `flex-shrink: 0` (parent supplies) | Page-level wrapper sizes this; the root itself is `display: inline-flex` only for the tile grid container. |
| `.cal-year-grid-tiles`        | parent width                       | `display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px;` |
| `.cal-tile`                   | `~220px`                           | Three columns × ~220px + 2 × 8px gutters = ~680px inside `.cal-grid-wrapper`. Page wrapper `.cal-layout` gap is 24px; remaining ~292px is the day-detail panel at the default breakpoint. |
| `.cal-tile-header`            | full tile width                    | `padding: 4px 6px; font-size: 0.75rem; font-weight: 600;` |
| `.cal-tile-grid`              | full tile width minus body padding | `display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px;` |
| `.cal-day` (mini)             | full column width, height `24px`   | `font-size: 0.7rem; height: 24px;` — ~30% smaller than `CalendarMonth.svelte`'s `0.8rem` / `32px`. |
| `.badge-dot` (mini)           | 3px diameter                       | Smaller than the existing 4px dot so it doesn't dominate the mini cell. |
| Tile gaps                     | `gap: 8px` on `.cal-year-grid-tiles` | Same visual weight as `.cal-layout`'s 24px gap, scaled down for tile-grid scale. |

**Fit math at the default 1100-px container:**

- Page wrapper `.cal-page` has `padding: 20px 24px` → inner width
  ~1052px.
- `.cal-layout` flex with `gap: 24px` and `align-items: flex-start`.
- `.cal-grid-wrapper` is `flex-shrink: 0` (existing CSS, line 663 in
  the current file).
- Year-grid wrapper width: 3 × 220px + 2 × 8px = **~672px**.
- Day-detail panel `flex: 1` → ~1052 - 672 - 24 = **~356px**.

356px is enough for the day-detail `Table` (the existing table renders
inside `flex: 1` with `min-width: 0`; horizontal overflow scrolls
within the table cell). This is the same contract that exists today
when the page renders `<CalendarMonth />` (which is 280px) — the
day-detail panel currently gets ~770px, so dropping to 356px is a
significant reduction. The day-detail table adapts because the
existing `Table.svelte` primitive handles `min-width: 0` and the row
cells wrap. Spec scenario "year grid fits the 1100-px container
without horizontal scroll" validates this.

**Breakpoint behavior at `max-width: 1024px`:**

```css
@media (max-width: 1024px) {
  .cal-layout {
    flex-direction: column;
  }
  .day-panel {
    width: 100%;
  }
}
```

The existing app-wide breakpoint (App.svelte line 555) is at
`max-width: 1024px` and the page container shrinks accordingly. The
new local rule flips `.cal-layout` from `flex-direction: row` to
`flex-direction: column` so the day-detail panel moves below the year
grid at the same threshold. This is a **new** CSS rule in this slice;
no new breakpoint is introduced (D6).

#### Today's anchor in any year (D8)

The primitive does not branch on `viewYear` vs. the current year.
`todayDate` is passed in by the parent; the primitive compares each
cell's ISO with `todayDate` to apply the `day-today` class. Today's
ring renders identically regardless of which year the grid renders:

```svelte
class:day-today={cell.iso === todayDate}
```

Same logic as `CalendarMonth.svelte`'s today ring (line 487 in the
current file). The Today button in the page-level header is the
explicit reset path that flips `viewYear` back to the current year
(D5).

#### Visual classes (CSS)

```css
.cal-year-grid {
  display: inline-flex;
  flex-direction: column;
  user-select: none;
  font-family: inherit;
}

.cal-year-grid-tiles {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  width: 100%;
}

.cal-tile {
  display: flex;
  flex-direction: column;
  background: var(--color-base-100);
  border: 1px solid var(--color-base-200);
  border-radius: 8px;
  overflow: hidden;
}

.cal-tile-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 4px 6px;
  border-bottom: 1px solid var(--color-base-200);
  background: color-mix(in oklch, var(--color-base-200) 30%, transparent);
}

.cal-tile-month {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--color-base-content);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.cal-tile-year {
  font-size: 0.7rem;
  color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
}

.cal-tile-body {
  padding: 4px;
}

.cal-tile-weekdays {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  padding: 2px 0;
  border-bottom: 1px solid color-mix(in oklch, var(--color-base-200) 60%, transparent);
}

.cal-tile-weekday {
  text-align: center;
  font-size: 0.6rem;
  font-weight: 700;
  color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
  text-transform: uppercase;
}

.cal-tile-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px;
  padding: 2px 0;
}

.cal-day {
  position: relative;
  background: none;
  border: none;
  cursor: pointer;
  font-size: 0.7rem;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  color: var(--color-base-content);
  padding: 0;
}

.cal-day:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 1px;
  z-index: 1;
}

.cal-day:hover:not(.day-disabled):not(.day-selected) {
  background: color-mix(in oklch, var(--color-base-300) 50%, transparent);
}

/* Today: distinct highlight (mirrors CalendarMonth.svelte) */
.cal-day.day-today {
  color: var(--color-primary);
  font-weight: 700;
}

/* Selected: filled primary */
.cal-day.day-selected {
  background: var(--color-primary);
  color: var(--color-base-100);
  font-weight: 600;
}

.cal-day.day-selected.day-today {
  color: var(--color-base-100);
}

/* Disabled */
.cal-day.day-disabled {
  color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
  cursor: not-allowed;
}

/* Weekend muted */
.cal-day.day-weekend:not(.day-selected):not(.day-disabled) {
  color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
}

/* Focused ring (only on the focused cell inside the focused tile) */
.cal-day.day-focused:not(.day-selected) {
  outline: 2px solid var(--color-primary);
  outline-offset: 1px;
}

.cal-day-blank {
  cursor: default;
}

/* Badge dot (smaller than the monthly grid's 4px dot) */
.badge-dot {
  position: absolute;
  bottom: 1px;
  left: 50%;
  transform: translateX(-50%);
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: var(--color-warning);
}

.cal-day.day-selected .badge-dot {
  background: color-mix(in oklch, var(--color-base-100) 80%, transparent);
}
```

Notes:

- **Class vocabulary mirrors `CalendarMonth.svelte`**: `.cal-day`,
  `.day-today`, `.day-selected`, `.day-disabled`, `.day-weekend`,
  `.badge-dot`. New classes for the tile-level structure:
  `.cal-year-grid`, `.cal-year-grid-tiles`, `.cal-tile`,
  `.cal-tile-header`, `.cal-tile-body`, `.cal-tile-weekdays`,
  `.cal-tile-weekday`, `.cal-tile-grid`. The day-cell rules are a
  scaled-down copy of `CalendarMonth.svelte`'s `.cal-day` rules (24px
  height vs 32px, 0.7rem vs 0.8rem, 3px dot vs 4px dot).
- **The empty tile cell placeholder** (`<span class="cal-day cal-day-blank" aria-hidden="true">`)
  carries the `cal-day` class so the CSS Grid layout stays
  consistent across in-range and out-of-month cells. This is the same
  pattern `CalendarMonth.svelte` uses today (line 510).
- **No new theme tokens.** All colors are existing DaisyUI theme
  variables (`--color-base-100`, `--color-base-200`, `--color-base-300`,
  `--color-base-content`, `--color-primary`, `--color-warning`).
  Light/dark themes inherit the same way the rest of the app does.

#### Locale (inherited from `$LL.calendar.*`)

The primitive uses two reactive arrays exactly like
`CalendarMonth.svelte`:

```ts
$: MONTH_NAMES = $LL.calendar.monthNames
  ? Array.from({ length: 12 }, (_, i) => ($LL.calendar.monthNames as Record<string, () => string>)[String(i)]())
  : [];
$: WEEKDAY_SHORT = $LL.calendar.weekdayShort
  ? Array.from({ length: 7 }, (_, i) => ($LL.calendar.weekdayShort as Record<string, () => string>)[String(i)]())
  : [];
```

Locale switching (e.g., Configuration → Language: `es`) re-renders the
tile headers and weekday rows in Spanish because both arrays are
derived from `$LL`. No additional locale wiring is required in the
year-grid primitive.

#### Type contract

`createEventDispatcher<{ selectDate: string }>()` — a single event with
a string payload (ISO `YYYY-MM-DD`). The parent `CalendarPage.svelte`
handles this via `on:selectDate={onSelectDate}` and updates
`selectedDate`. No `viewYearChange`, no `monthChange`, no
`viewMonthChange`, no `escape`, no picker-state events are emitted by
the year-grid primitive (the page-level header owns year navigation).

### `CalendarPage.svelte` (modified)

Three surgical changes:

1. **Add page-level year-nav row** (NEW, between `.cal-page-header`
   and `.cal-layout`).
2. **Swap `<CalendarMonth />` for `<CalendarYearGrid />`** in the
   default render branch (the calendar grid inside `.cal-grid-wrapper`).
3. **Add `onSelectDate` handler** parallel to the existing
   `onDaySelect` (the existing handler is retained because the day-
   detail panel's existing `selectedDate` reactivity is unchanged).
4. **Add a Today-button click handler** that updates both `viewYear`
   and `selectedDate` in one tick (D10).
5. **Add a `flex-direction: column` rule** to `.cal-layout` at the
   existing app breakpoint (`max-width: 1024px`).

#### New page-level year-nav row

```svelte
<div class="cal-page">
  <div class="cal-page-header">
    <h2 class="page-title">{$LL.calendar.pageTitle()}</h2>
    <Tooltip text={$LL.calendar.refreshAria()} position="bottom">
      <Button variant="ghost" size="sm"
              aria-label={$LL.calendar.refreshAria()}
              disabled={loading}
              onclick={loadLots}>
        {#snippet iconStart()}<Icon name="arrow-path" size="sm" />{/snippet}
        {$LL.calendar.refresh()}
      </Button>
    </Tooltip>
  </div>

  <!-- NEW: page-level year header -->
  <div class="cal-year-nav" data-cal-year-nav role="toolbar"
       aria-label={$LL.calendar.annualHeaderLabel({ year: viewYear })}>
    <Tooltip text={$LL.calendar.ariaPreviousYear()} position="bottom">
      <Button variant="ghost" size="sm"
              aria-label={$LL.calendar.ariaPreviousYear()}
              onclick={prevYear}>
        ‹
      </Button>
    </Tooltip>
    <Tooltip text={$LL.calendar.ariaOpenYearPicker()} position="bottom">
      <Button variant="ghost" size="sm"
              aria-label={$LL.calendar.ariaOpenYearPicker()}
              onclick={openYearPicker}>
        {viewYear}{#if !showYearPicker} ▼{/if}
      </Button>
    </Tooltip>
    <Tooltip text={$LL.calendar.ariaNextYear()} position="bottom">
      <Button variant="ghost" size="sm"
              aria-label={$LL.calendar.ariaNextYear()}
              onclick={nextYear}>
        ›
      </Button>
    </Tooltip>
    <Button variant="primary" size="sm"
            data-cal-nav-today
            aria-label={$LL.calendar.today()}
            onclick={goToday}>
      {$LL.calendar.today()}
    </Button>
  </div>

  {#if loading && lots.length === 0}
    …existing loading branch…
  {:else if errorMsg && lots.length === 0}
    …existing error branch…
  {:else}
    <div class="cal-layout">
      <div class="cal-grid-wrapper">
        <CalendarYearGrid
          {viewYear}
          {todayDate}
          {selectedDate}
          {dayBadges}
          ariaLabel={$LL.calendar.annualHeaderLabel({ year: viewYear })}
          on:selectDate={onSelectDate}
        />
      </div>
      <div class="day-panel">
        …existing day-detail panel (UNCHANGED)…
      </div>
    </div>
  {/if}
</div>
```

Notes:

- **`<CalendarMonth />` is removed from the default render.** The
  `viewMonth` state and `onMonthChange` / `onViewYearChange` /
  `onDaySelect` handlers remain in the page (they are not deleted
  because they could be re-used by a future Month toggle), but they
  are no longer wired to a rendered primitive in this slice.
- **`viewMonth` is still tracked** in page state (its initialization
  from `new Date()` is unchanged) so a future Month toggle can read
  it without re-deriving from `selectedDate`.
- **`selectedDate` initial value** is still `todayIso()` so the year
  grid's `focusedTileIdx` initialization falls into today's tile on
  first render.

#### New handlers

```ts
function prevYear() {
  viewYear = Math.max(1900, viewYear - 1);
}

function nextYear() {
  viewYear = Math.min(2100, viewYear + 1);
}

function goToday() {
  viewYear = new Date().getFullYear();
  selectedDate = todayDate;
}

function onSelectDate(e: CustomEvent<string>) {
  selectedDate = e.detail;
}
```

`prevYear` / `nextYear` are clamped to the same `[1900, 2100]` range
that `CalendarMonth.svelte` uses for its picker chevrons (lines 199,
204 of the current file). `goToday` updates `viewYear` and
`selectedDate` in the same tick; both state updates ride the same
Svelte reactive cycle, so the year grid receives the new `viewYear`
prop and the new `selectedDate` prop in a single re-render. The
existing `dayBadges` and `dayRows` derived stores recompute from
`selectedDate` exactly as before (D10).

#### Year-nav CSS

```css
.cal-year-nav {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-bottom: 12px;
  padding: 4px 0;
  border-bottom: 1px solid var(--color-base-200);
}

/* The .cal-year-label class is retired by the year-picker correction;
   the year-picker Button trigger carries its own visual contract. */
```

The year-nav row is rendered **between** `.cal-page-header` and
`.cal-layout` so the layout reads top-to-bottom: page title →
year navigation → year grid (left) + day detail (right).

#### Local breakpoint rule

```css
@media (max-width: 1024px) {
  .cal-layout {
    flex-direction: column;
  }
  .day-panel {
    width: 100%;
  }
}
```

The rule flips `.cal-layout` from `flex-direction: row` (default) to
`flex-direction: column` at the same `max-width: 1024px` threshold
the app uses elsewhere (App.svelte line 555). This is a **local** rule
in `CalendarPage.svelte`'s scoped `<style>` block; no global CSS is
added. The day-detail panel becomes a full-width row below the year
grid at narrow widths.

#### Tab order at the page level (locked)

| # | Element | Tabindex |
|---|---------|----------|
| 1 | Refresh button (`.cal-page-header`) | 0 |
| 2 | Prev-year chevron (`.cal-year-nav`) | 0 |
| 3 | `viewYear` year-picker trigger (focusable Button) | 0 |
| 4 | Next-year chevron | 0 |
| 5 | Today button (`data-cal-nav-today`) | 0 |
| 6 | Year grid — focused tile's grid | 0 (the other 11 tiles: -1) |
| 7 | Day-detail panel rows | 0 (each `.lot-link`) |

The page-level tab order is:

```text
prev-year chevron → viewYear year-picker trigger
                   → next-year chevron → Today button
                   → year grid (focused tile's grid owns tabindex=0)
                   → day-detail rows
```

This is the order codified in the spec delta's "keyboard surface
inherited through the year-grid primitive" scenario.

### `DatePicker.svelte` (unchanged)

Verified to need **no edits** for this slice. The popover host
continues to render `<CalendarMonth>` (single-month) for the date
input, exactly as today. `CalendarYearGrid.svelte` is consumed **only**
by `CalendarPage.svelte`; the `DatePicker.svelte` import list does not
gain it. The spec delta's "DatePicker is not affected by
CalendarYearGrid" scenario codifies this.

### i18n surface

#### New keys (added under `calendar.*`)

```ts
// en
calendar: {
  …
  annualHeaderLabel:  "Calendar year {year}",   // NEW
  tileMonthLabel:     "{month} {year}",          // NEW
  …
}

// es
calendar: {
  …
  annualHeaderLabel:  "Año {year}",              // NEW
  tileMonthLabel:     "{month} {year}",          // NEW
  …
}
```

#### Reused keys (already present)

| New affordance                | Key reused              | Current value (en / es) |
|-------------------------------|-------------------------|-------------------------|
| Prev-year chevron             | `ariaPreviousYear`      | "Previous year" / "Año anterior" |
| Next-year chevron             | `ariaNextYear`          | "Next year" / "Año siguiente" |
| Today button                  | `today`                 | "Today" / "Hoy" |
| Tile month label              | `ariaMonth`             | "{month} {year}" |

The `tileMonthLabel` key is added even though `ariaMonth` has the same
"shape" — having a dedicated key lets future slices diverge
(translate "{month} {year}" differently for a tile header vs. a
picker chip). `ariaPreviousYear` / `ariaNextYear` / `today` /
`ariaMonth` are NOT added; they are reused. This keeps the i18n
surface minimal (D20).

#### Regenerated types

`src/i18n/i18n-types.ts` is regenerated through the existing pipeline:

```bash
npm run i18n:generate     # direct invocation
# OR
npm run build             # prebuild hook invokes i18n:generate
npm run dev               # predev hook invokes i18n:generate
```

The generated `calendar` namespace gains two new function-shaped
keys (`annualHeaderLabel({ year: number })` and `tileMonthLabel({
month: string, year: number })`). If `npm run i18n:generate` is not
run, `svelte-check` (`npm run check`) fails on the new
`$LL.calendar.annualHeaderLabel({ year: viewYear })` reference inside
`CalendarPage.svelte` and `$LL.calendar.tileMonthLabel({ month, year })`
references inside `CalendarYearGrid.svelte`.

### Canonical spec delta (locked from `spec.md`)

The `spec.md` delta (already authored in
`openspec/changes/calendar-annual-overview/specs/caduxo-expiry-tracker/spec.md`)
modifies both occurrences of the "Calendar tab" requirement in the
canonical spec:

- **First occurrence** at line 1227 (`### Requirement: Calendar tab`,
  under `## Capability: Calendar`).
- **Second occurrence** at line 2739 (`#### Requirement: Calendar tab`,
  under `## Capability: Calendar` — a sub-requirement with four hashes,
  not three, per the daisyui-redesign precedent).

The delta also adds a new `### Requirement: Calendar annual view`
capability. The apply phase executes the textual `## MODIFIED
Requirements` and `## ADDED Requirements` blocks. The orchestrator
replaces the requirement blocks verbatim in the canonical spec (matching
the `calendar-month-picker` apply pattern, which handled a similar
two-occurrence case via the OpenSpec tooling rather than literal line-
numbered replacement).

## Data flow

End-to-end flow when the user opens the Calendar tab:

```text
[1] User clicks the Calendar tab in main navigation
      ↓
[2] CalendarPage mounts → onMount → loadLots() → list_dashboard_lots
      ↓
[3] lots: DashboardLotRow[] is populated; dayBadges = lots.reduce(...)
      ↓
[4] Page renders the default branch (year grid):
      <CalendarYearGrid viewYear={viewYear}
                       todayDate={todayDate}
                       selectedDate={todayDate}
                       dayBadges={dayBadges}
                       …>
      ↓
[5] Year grid mounts:
      • tileFocus initialized to today (or selectedDate if inside viewYear)
      • focusedTileIdx = today's tile (e.g., February if today is Feb 14)
      • The focused tile's grid owns tabindex="0"; the other 11 own tabindex="-1"
      ↓
[6] User tabs from Refresh button → prev-year chevron → next-year
   chevron → Today button → year grid (focused tile's grid takes
   focus) → day-detail rows
```

End-to-end flow when the user clicks a day cell inside a tile:

```text
[1] User clicks day D inside a tile
      ↓
[2] selectDay(iso) → dispatch("selectDate", iso)
      ↓
[3] CalendarPage.onSelectDate handler sets selectedDate = iso
      ↓
[4] Reactive cascade:
      • dayBadges is unchanged (the map doesn't change with selection)
      • dayRows = lots.filter(l => l.expiry_date === selectedDate) recomputes
      • Day-detail panel re-renders with the new rows
      • <CalendarYearGrid selectedDate={selectedDate} …> receives the new prop
      • Year grid's focusedTileIdx reactive block re-evaluates:
        if (selectedDate.startsWith(`${viewYear}-`)) {
          tileFocus[tile] = firstDow + (day - 1);
          focusedTileIdx = tile;
        }
      • The selected day's tile now owns tabindex="0" (if it changed)
      • Day-detail table renders the rows for D
      ↓
[5] Year view stays open; the day-detail panel updates in place; no
   <CalendarMonth /> swap; no chevron click; no refetch
```

End-to-end flow when the user clicks the Today button:

```text
[1] User clicks "Today" in .cal-year-nav
      ↓
[2] goToday() handler:
      viewYear = new Date().getFullYear();
      selectedDate = todayDate;
      ↓
[3] Reactive cascade:
      • <CalendarYearGrid viewYear={viewYear} selectedDate={selectedDate} …>
        receives both new props in the same Svelte reactive cycle
      • Year grid rebuilds 12 tiles for the new viewYear
      • focusedTileIdx reactive block re-evaluates → today's tile
      • Day-detail panel's dayRows recomputes for todayDate
      ↓
[4] No backend command; no refetch; no event dispatched by the primitive
```

End-to-end flow when the user clicks the prev-year chevron:

```text
[1] User clicks "‹" in .cal-year-nav
      ↓
[2] prevYear() handler:
      viewYear = Math.max(1900, viewYear - 1);
      ↓
[3] Reactive cascade:
      • <CalendarYearGrid viewYear={viewYear} …> receives the new prop
      • Year grid rebuilds 12 tiles for viewYear - 1
      • If selectedDate is still inside viewYear - 1 (e.g., user
        selected a date that exists in both years), the selection
        ring renders; otherwise no selection ring
      • focusedTileIdx reactive block re-evaluates the tile that
        contains selectedDate (or todayDate, if today is inside
        viewYear - 1, e.g., when browsing a past year)
      ↓
[4] No backend command; no refetch
```

End-to-end flow when the user presses `←` from column 0 of a tile:

```text
[1] onTileDayKeydown(e, tileIdx, cells) fires with key="ArrowLeft"
      ↓
[2] col = idx % 7 === 0 → cross-tile wrap
      ↓
[3] moveAcrossTile(tileIdx - 1, idx - 1):
      • tileFocus[tileIdx] = -1   (current tile no longer focused)
      • focusedTileIdx = tileIdx - 1
      • tileFocus[tileIdx - 1] = idx - 1
      • focusCell(tileIdx - 1, idx - 1) → DOM focus moves to the
        previous month's tile at column 6, same row
      ↓
[4] Previous month's tile's grid now owns tabindex="0"; current
   month's grid is back to tabindex="-1"
      ↓
[5] No selectDate dispatch; no viewYear change; no event
```

End-to-end flow when the user presses `Esc` inside the year grid:

```text
[1] onTileDayKeydown(e, tileIdx, cells) fires with key="Escape"
      ↓
[2] document.querySelector("[data-cal-nav-today]")?.focus()
      ↓
[3] Focus moves to the Today button in .cal-year-nav
      ↓
[4] No selectDate dispatch; no viewYear change; no state update
```

## File changes

| File | Type | Estimate |
|------|------|----------|
| `src/components/CalendarYearGrid.svelte` | **new** | ~250 LOC (mini-month markup × 12, keyboard handler, scoped CSS) |
| `src/components/CalendarPage.svelte` | edit (add `.cal-year-nav`, swap `<CalendarMonth />` → `<CalendarYearGrid />`, add `onSelectDate`/`prevYear`/`nextYear`/`goToday`, add `@media` rule) | ~50 LOC added, ~10 LOC removed → net **+40 LOC** |
| `src/i18n/en/index.ts` | edit (add `annualHeaderLabel`, `tileMonthLabel`) | ~2 LOC |
| `src/i18n/es/index.ts` | edit (same) | ~2 LOC |
| `src/i18n/i18n-types.ts` | regenerated by `npm run i18n:generate` (do not hand-edit) | ~10 LOC delta |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | edit (apply the textual delta from `specs/caduxo-expiry-tracker/spec.md`: both `Calendar tab` occurrences updated + new `Calendar annual view` requirement added) | ~250 LOC delta |
| `openspec/changes/calendar-annual-overview/design.md` | new (this file) | ~700 LOC |

**Net total: ~1254 LOC including regenerated types**, of which the
**human-edited** delta is ~334 LOC (the rest is regenerated code,
generated CSS, and this design doc). Comfortably under the 800-line
project review budget when measured as the *human* delta; the
regenerated `i18n-types.ts` is mechanical and follows the daisyui-
redesign precedent of being out-of-budget for review purposes.

If the budget becomes tight during `tasks.md`, the contingency is
**split the slice into PR 1 (i18n keys + CalendarYearGrid.svelte with
placeholder parent wiring) and PR 2 (CalendarPage.svelte swap + CSS
breakpoint)** — both are independently reviewable.

## Validation plan

The verify gate is **manual smoke + `svelte-check` + type regeneration**.
There is no unit-test harness for `CalendarMonth.svelte` or
`DatePicker.svelte` (verified: `find **/*.{test,spec}.ts` returns no
results in the project, and there is no `vitest` dependency in
`package.json`). The same is true for the new
`CalendarYearGrid.svelte`.

### Pre-flight checks (in CI / pre-merge)

```bash
npm run i18n:generate     # regenerates i18n-types.ts so the new keys type-check
npm run check             # svelte-check --threshold error; must pass
```

`npm run check` will fail if:

- `i18n-types.ts` is not regenerated after the dictionary edits —
  `annualHeaderLabel` and `tileMonthLabel` will be missing.
- `CalendarPage.svelte` references the new keys before types are
  regenerated.
- `CalendarYearGrid.svelte` references `$LL.calendar.tileMonthLabel`
  or `$LL.calendar.monthNames` / `weekdayShort` against a stale
  types file.

### Manual smoke (matrix)

Run inside `npm run tauri dev` on the same shell as the
`2026-09-21-caduxo-calendar-month-picker` and
`2026-09-15-caduxo-custom-date-picker` slices:

| #  | Surface                        | Action                                                                        | Expected |
|----|--------------------------------|-------------------------------------------------------------------------------|----------|
| M1 | Calendar tab                   | Open at current year                                                          | 3 × 4 grid of 12 tiles renders for the current year; today's tile shows today's cell with the today ring; today's cell is selected (filled blue); day-detail panel lists today's expirations |
| M2 | Calendar tab                   | Click any day in any tile                                                     | Year view stays open; clicked day is selected (filled blue); day-detail panel updates to list the clicked day's expirations; if no expirations, panel shows the existing "No lots expiring on this date." message |
| M3 | Calendar tab                   | Click prev-year chevron (`‹`)                                                 | Year header label updates to `viewYear - 1`; 12 tiles re-render for the new year; `selectedDate` is preserved if it exists in the new year, otherwise no selection ring renders; no `list_dashboard_lots` refetch |
| M4 | Calendar tab                   | Click next-year chevron (`›`)                                                 | Symmetric to M3 for `viewYear + 1`; clamped at `2100` |
| M5 | Calendar tab                   | Click Today button while viewing a non-current year                            | `viewYear` jumps to current year; `selectedDate` becomes today; year grid re-renders; day-detail panel updates to today's expirations; no refetch |
| M6 | Calendar tab                   | Tab order from page header → year grid → day-detail                           | Tab from Refresh button lands on prev-year chevron → year-picker trigger → next-year chevron → Today button → year grid (focused tile's grid takes focus) → day-detail rows. `viewYear` is a `<Button>` trigger that opens the decade picker |
| M7 | Calendar tab                   | Arrow keys traverse the year grid                                             | `←` from column 0 of February wraps to column 6 of January; `→` from column 6 of November wraps to column 0 of December; `↑` from the first row of February wraps to the last in-range row of January at the same column; `↓` from the last row of January wraps to the second row of February at the same column (first row of February if the second row is blank, etc.) |
| M8 | Calendar tab                   | Arrow keys never cross year boundaries inside the primitive                    | `←` from column 0 of January tile wraps to column 6 of December of the SAME year (not December of `viewYear - 1`); `→` from column 6 of December tile wraps to column 0 of January of the SAME year (not January of `viewYear + 1`); page-level chevrons remain the only year-boundary path |
| M9 | Calendar tab                   | Press `Enter` / `Space` on a focused day                                       | Dispatches `selectDate`; parent `selectedDate` updates; day-detail panel updates; year view stays open |
| M10 | Calendar tab                  | Press `Esc` inside the year grid                                               | Focus returns to the Today button (`data-cal-nav-today`); no `selectDate`; `viewYear` and `selectedDate` are unchanged |
| M11 | Calendar tab                  | Press `PageUp` / `PageDown` inside the year grid                              | Event is swallowed (page does NOT scroll); no state change |
| M12 | Calendar tab                  | Today's anchor in non-current years                                            | Navigate to a non-current year (e.g., 2024); today's tile inside 2024 (or the month that matches today's `MM-DD` if it exists in 2024) still renders today's cell with the today ring, visually identical to the current year's tile |
| M13 | Calendar tab                  | Dot badges on every tile                                                       | Open a tile that has at least one expiry in `dayBadges`; the day cell renders the 3-px dot under the day number; the dot count visual treatment matches `CalendarMonth.svelte`'s (warning yellow); selection ring on a cell with a dot uses the on-selection dot color |
| M14 | Calendar tab                  | Tile header is read-only                                                       | Click a tile header (the `<span>` with the month name and year); no event dispatched; no collapse to single-month; year view stays open; click on the tile body still selects a day normally |
| M15 | Calendar tab                  | Year grid fits the 1100-px container at the default breakpoint                  | Page renders at the default 1100-px content area; 3 × 4 grid plus gutters fits without horizontal scroll; no `overflow-x` on the year-grid root; day-detail panel is visible side-by-side |
| M16 | Calendar tab                  | Breakpoint collapse at 1024 px                                                 | Resize the window to ≤1024 px (or set Tauri's default window width); `.cal-layout` flips to `flex-direction: column`; day-detail panel moves below the year grid; no horizontal scroll |
| M17 | Calendar tab (existing)        | Day-detail panel still works                                                   | Click a day; day-detail panel renders the `Table` with `product / qty / unit / store / location / days / status` columns; click a lot row opens the existing `Modal` with `LotMovementsPanel`; the modal close flow is unchanged |
| M18 | Calendar tab (existing)        | Day-bucket computation unchanged                                               | The `dayBadges` derived map continues to be computed client-side from `lots`; year-grid dot badges come from the same map; no new IPC; no backend change |
| M19 | DatePicker popover (regression) | Open a form-field DatePicker; pick a date; close                              | `<CalendarMonth />` continues to render single-month popover; `monthChange` / `viewYearChange` flows unchanged; year picker overlay and month picker overlay continue to work; popover positioning math in `DatePicker.svelte` is untouched |
| M20 | DatePicker popover (regression) | DatePicker.svelte import list                                                  | `CalendarYearGrid.svelte` is NOT imported by `DatePicker.svelte`; only `<CalendarMonth />` is rendered |
| M21 | i18n                          | Switch locale to `es` in Configuration                                         | Tile headers render Spanish month names (`Enero`, `Febrero`, …); weekday short labels render in Spanish (`Dom`, `Lun`, …); year-nav aria labels read in Spanish (`Año anterior`, `Año siguiente`, `Hoy`); `annualHeaderLabel` renders as `Año {year}`; `tileMonthLabel` renders as `{month} {year}` (with Spanish month name) |
| M22 | Keyboard focus restore        | Open Calendar tab, Tab into year grid, press Esc                                | Focus returns to Today button; pressing Tab moves to day-detail rows; pressing Shift+Tab returns to Today button |

### Regression risk checks

- **No native date input regression**: `grep -RIn 'type="date"' src/`
  must still return zero matches.
- **No CalendarMonth regression**: `grep -RIn 'CalendarMonth'
  src/components/` must continue to show only the
  `DatePicker.svelte` and the unused import in `CalendarPage.svelte`
  (the latter is intentionally retained for a future Month toggle —
  the apply phase may remove it; this design keeps it).
- **No dayBadges regression**: `grep -RIn 'dayBadges'
  src/components/CalendarPage.svelte` must continue to show the
  same `lots.reduce(...)` derivation; no new computation.
- **No backend regression**: `cargo test --lib` baseline from the
  most recent Rust-touching slice is unchanged because no Rust file
  is touched.
- **No new IPC**: `grep -RIn 'invoke' src/lib/dashboard.ts` must
  continue to show the same `list_dashboard_lots` invocation; no new
  `invoke()` calls.

## Risks and mitigations

| #   | Risk                                                                                                                                                | Mitigation                                                                                                                                                                                       |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | DOM weight: 12 tiles × 42 cells = 504 day buttons + 12 weekday rows + 12 headers. Page might feel heavy.                                            | Roving tabindex per tile (only one tile's grid owns `tabindex="0"`); per-tile `role="grid"`; cells outside the focused tile are not in the natural Tab order; `Tab` leaves the year grid and continues to day-detail rows. |
| R2  | Cross-tile keyboard wrap algorithm has edge cases at month boundaries (e.g., February first row has 0-2 blanks at columns 0-1; December last row may have 5-6 blanks at columns 5-6). | The wrap algorithm targets the column-aligned row in the adjacent tile and falls back to the closest in-range cell at or below the target column when the adjacent month has no day at that column; manual smoke M7/M8 cover the wrap path; unit tests are not required (project default `strictTDD: false`). |
| R3  | Tile sizing could overshoot the 1100-px container if tile width is not constrained.                                                                  | Tile width is fixed by the `repeat(3, 1fr)` grid + `gap: 8px` layout; the year-grid wrapper is `flex-shrink: 0` (existing CSS) so it claims its natural width; manual smoke M15 validates the fit at the default breakpoint. |
| R4  | Today invisible across years: user navigating to a past year might lose the anchor.                                                                 | Today's ring renders identically inside any year's tile (D8); the Today button is the explicit reset path (D5); manual smoke M12 validates the cross-year anchor rule. |
| R5  | Spec.md has two occurrences of `Calendar tab` requirement (line 1227 with `###`, line 2739 with `####`). Apply phase might miss the second.            | The OpenSpec tooling replaces the requirement blocks verbatim from the textual delta; the precedent (calendar-month-picker) handled a similar two-occurrence case; the apply phase uses the OpenSpec replace-text flow, not line-numbered `sed`. |
| R6  | i18n drift if `npm run i18n:generate` is not run after adding `annualHeaderLabel` / `tileMonthLabel`.                                              | `prebuild` and `predev` hooks invoke `i18n:generate` automatically; `npm run check` fails on missing type entries; CI runs `npm run check` before merging. |
| R7  | Roving tabindex per tile could leak: two tiles might temporarily own `tabindex="0"` during the `moveAcrossTile` transition.                          | The handler sets `tileFocus[tileIdx] = -1` BEFORE updating `focusedTileIdx`; only one `tileIdx` matches `focusedTileIdx === tile` after the synchronous update; the DOM focus is moved last, after the reactive `tabindex` recompute settles in the same microtask. |
| R8  | CalendarMonth.svelte regression: removing `<CalendarMonth />` from the default render could break a future Month toggle that re-uses the page state. | `viewYear` / `viewMonth` / `onMonthChange` / `onViewYearChange` / `onDaySelect` handlers are RETAINED in `CalendarPage.svelte` (they just aren't wired to a rendered primitive in this slice); a future Month toggle can re-add `<CalendarMonth />` with those handlers in one swap. |
| R9  | Page layout overflow at narrow widths: at <1024 px the day-detail panel might not fit side-by-side with the year grid.                              | Local `@media (max-width: 1024px)` rule flips `.cal-layout` to `flex-direction: column` so the day-detail panel moves below the grid; manual smoke M16 validates the breakpoint behavior. |
| R10 | Esc returning focus to the Today button assumes the Today button has been rendered. If a future slice conditionally hides the Today button, focus would have nowhere to land. | The Today button is unconditionally rendered in this slice (no `{#if}`); a future conditional rendering must update the `Esc` focus target (documented in the spec delta's "Esc returns focus to the page-level header" scenario as "the `viewYear` label or the prev-year chevron, whichever was last focused before the year grid took focus" — the design locks the Today button because it's always rendered in this slice). |
| R11 | Review budget risk: ~334 LOC human-edited delta + 10 LOC regenerated types = ~344 LOC, well under 800. The contingency (split into two PRs) is reserved for the unlikely case that `npm run check` discovers cross-file type churn that bumps the regenerated types by more than ~50 LOC. | Contingency: split into PR 1 (i18n keys + CalendarYearGrid.svelte with stub parent wiring) and PR 2 (CalendarPage.svelte swap + CSS breakpoint). |

## Out-of-scope confirmations

The following are explicitly NOT part of this slice and remain separate
follow-on changes:

- **Compact right-side day-detail panel** → follow-on
  `calendar-compact-day-panel`. The day-detail panel stays as the
  existing side-by-side panel in this slice.
- **Month / Year view toggle** → follow-on `calendar-month-year-toggle`.
  Year is the only landing mode in this slice.
- **Drill-down from a tile header to a single-month view** → future
  slice. Tile headers are read-only in this slice.
- **Cross-tab badge counts** (e.g., "expiring this year" pill on the
  Calendar nav tab) → separate concern.
- **`minDate` / `maxDate` semantic change** → not touched; the default
  range `1900-01-01` → `2100-12-31` already covers the user's
  plausible navigation horizon.
- **`DatePicker.svelte` changes** → none. The popover stays
  single-month and continues to consume `<CalendarMonth />`.
- **New backend Tauri command** → none. `list_dashboard_lots` is the
  only data source.
- **Refactor `CalendarMonth.svelte` into a generic multi-mode
  component** → not in this slice. The annual-view primitive is a
  sibling, not a refactor.
- **Unit tests for the new primitive** → not added (project default
  `strictTdd: false`; manual smoke matrix is the verify gate).
- **New locales beyond `en` / `es`** → not added.
- **Rename or reorder existing events** (`monthChange`,
  `viewYearChange`, `ariaOpenMonthPicker`, `ariaOpenYearPicker`) →
  none. Only the **consumer** (`CalendarPage.svelte`) switches its
  default child primitive.

## Open questions deferred to `tasks.md`

The proposal's product question round is closed (all five questions
answered with the assumed defaults baked into D6–D10). No new product
question round is required before `tasks.md`. The orchestrator can
proceed to tasks authoring against the locked decisions above.

## Cross-references

- Proposal: `openspec/changes/calendar-annual-overview/proposal.md`
- Spec delta:
  `openspec/changes/calendar-annual-overview/specs/caduxo-expiry-tracker/spec.md`
- Canonical spec to be updated by apply:
  `openspec/specs/caduxo-expiry-tracker/spec.md` (lines 1227 and 2739
  for the `Calendar tab` requirement; new `Calendar annual view`
  requirement added)
- Predecessor design (month picker overlay, 3-column 12-chip grid
  pattern):
  `openspec/changes/calendar-month-picker/design.md`
- Consumer 1: `src/components/CalendarPage.svelte` (Calendar tab) —
  page-level header + default primitive swap
- Consumer 2: `src/components/DatePicker.svelte` (form-field popover) —
  unchanged
- Source primitive: `src/components/CalendarMonth.svelte` —
  unchanged; CSS class vocabulary mirrored by
  `CalendarYearGrid.svelte`
- i18n pipeline: `src/i18n/en/index.ts`, `src/i18n/es/index.ts`,
  `src/i18n/i18n-types.ts` (generated), `npm run i18n:generate`
  (`prebuild` + `predev` hooks)
- App breakpoint: `src/App.svelte` line 555 (`@media (max-width: 1024px)`)
