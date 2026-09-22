<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { LL } from "../i18n/i18n-svelte.js";

  // ── Props ───────────────────────────────────────────────────────────────────

  export let viewYear: number;
  export let todayDate: string = "";
  export let selectedDate: string | null = null;
  export let dayBadges: Record<string, number> | undefined = undefined;
  export let ariaLabel: string = "Calendar year";
  export let minDate: string = "1900-01-01";
  export let maxDate: string = "2100-12-31";

  const dispatch = createEventDispatcher<{
    selectDate: string; // ISO YYYY-MM-DD
  }>();

  // ── Locale-backed arrays ────────────────────────────────────────────────────

  $: MONTH_NAMES = $LL.calendar.monthNames
    ? Array.from({ length: 12 }, (_, i) => ($LL.calendar.monthNames as Record<string, () => string>)[String(i)]())
    : [];
  $: WEEKDAY_SHORT = $LL.calendar.weekdayShort
    ? Array.from({ length: 7 }, (_, i) => ($LL.calendar.weekdayShort as Record<string, () => string>)[String(i)]())
    : [];

  // ── Helpers ────────────────────────────────────────────────────────────────

  function pad(n: number): string {
    return n < 10 ? "0" + n : String(n);
  }

  function isoDate(year: number, month: number, day: number): string {
    return `${year}-${pad(month)}-${pad(day)}`;
  }

  function cmpIso(a: string, b: string): number {
    return a < b ? -1 : a > b ? 1 : 0;
  }

  function inRange(iso: string): boolean {
    if (!iso) return false;
    return cmpIso(iso, minDate) >= 0 && cmpIso(iso, maxDate) <= 0;
  }

  function daysInMonth(year: number, month: number): number {
    return new Date(year, month, 0).getDate();
  }

  function firstDowOfMonth(year: number, month: number): number {
    return new Date(year, month - 1, 1).getDay(); // 0=Sun
  }

  function buildMonthGrid(
    year: number,
    month: number,
  ): { idx: number; iso: string; day: number; isOutside: boolean }[] {
    const cells: { idx: number; iso: string; day: number; isOutside: boolean }[] = [];
    const firstDow = firstDowOfMonth(year, month);
    const totalDays = daysInMonth(year, month);
    let idx = 0;
    function pushBlank() {
      cells.push({ idx: idx++, iso: "", day: 0, isOutside: true });
    }
    for (let i = 0; i < firstDow; i++) pushBlank();
    for (let d = 1; d <= totalDays; d++) {
      cells.push({ idx: idx++, iso: isoDate(year, month, d), day: d, isOutside: false });
    }
    while (cells.length < 42) pushBlank();
    return cells;
  }

  function isWeekend(cell: { iso: string; day: number }): boolean {
    if (!cell.iso) return false;
    const dow = new Date(
      parseInt(cell.iso.slice(0, 4)),
      parseInt(cell.iso.slice(5, 7)) - 1,
      cell.day,
    ).getDay();
    return dow === 0 || dow === 6;
  }

  function cellAriaLabel(cell: { iso: string; day: number }, monthName: string): string {
    if (!cell.iso) return "";
    const d = new Date(
      parseInt(cell.iso.slice(0, 4)),
      parseInt(cell.iso.slice(5, 7)) - 1,
      cell.day,
    );
    return $LL.calendar.ariaDayCell({
      month: monthName,
      day: d.getDate(),
      year: d.getFullYear(),
    });
  }

  // ── Roving tabindex state ──────────────────────────────────────────────────

  // tileFocus[i] = cell index (0..41) of focused cell inside tile i, or -1 if never focused.
  let tileFocus: number[] = Array.from({ length: 12 }, () => -1);
  // The tile that currently owns tabindex=0 (-1 = none focused yet).
  let focusedTileIdx: number = -1;

  // Initialise focusedTileIdx and tileFocus whenever viewYear or selectedDate changes.
  $: {
    if (viewYear) {
      // Try to land on the tile containing selectedDate.
      if (
        selectedDate &&
        selectedDate.startsWith(`${viewYear}-`)
      ) {
        const m = parseInt(selectedDate.slice(5, 7));
        const d = parseInt(selectedDate.slice(8, 10));
        const tile = m - 1;
        const firstDow = firstDowOfMonth(viewYear, m);
        tileFocus = tileFocus.map((v, i) => (i === tile ? firstDow + (d - 1) : v));
        focusedTileIdx = tile;
      } else if (
        todayDate &&
        todayDate.startsWith(`${viewYear}-`)
      ) {
        const m = parseInt(todayDate.slice(5, 7));
        const d = parseInt(todayDate.slice(8, 10));
        const tile = m - 1;
        const firstDow = firstDowOfMonth(viewYear, m);
        tileFocus = tileFocus.map((v, i) => (i === tile ? firstDow + (d - 1) : v));
        focusedTileIdx = tile;
      } else if (focusedTileIdx === -1) {
        // Default: January first day (blank cell may be first, that's fine).
        const firstDow = firstDowOfMonth(viewYear, 1);
        tileFocus = tileFocus.map((v, i) => (i === 0 ? firstDow : v));
        focusedTileIdx = 0;
      }
    }
  }

  // ── Focus helper ──────────────────────────────────────────────────────────

  function focusCell(tileIdx: number, cellIdx: number) {
    tileFocus = tileFocus.map((v, i) => (i === tileIdx ? cellIdx : v));
    // Synchronous DOM focus: the keyboard handler is already inside the event
    // loop so a reactive update via Svelte is not needed.
    requestAnimationFrame(() => {
      const btn = document.querySelector<HTMLButtonElement>(
        `[data-tile-idx="${tileIdx}"][data-cell-idx="${cellIdx}"]`,
      );
      btn?.focus();
    });
  }

  // ── Day selection ─────────────────────────────────────────────────────────

  function selectDay(iso: string) {
    if (!inRange(iso)) return;
    dispatch("selectDate", iso);
  }

  // ── Keyboard navigation ───────────────────────────────────────────────────

  function onTileDayKeydown(
    e: KeyboardEvent,
    tileIdx: number,
    cells: { idx: number; iso: string; day: number; isOutside: boolean }[],
  ) {
    const idx = tileFocus[tileIdx];
    if (idx < 0) return;
    const cols = 7;

    function clampCell(nextIdx: number): number {
      if (nextIdx < 0) return 0;
      if (nextIdx > 41) return 41;
      return nextIdx;
    }

    function moveWithinTile(delta: number) {
      tileFocus = tileFocus.map((v, i) => (i === tileIdx ? clampCell(idx + delta) : v));
      focusCell(tileIdx, clampCell(idx + delta));
    }

    function moveAcrossTile(targetTile: number, targetCell: number) {
      // Leave current tile without a focused cell so it resets to tabindex=-1.
      tileFocus = tileFocus.map((v, i) => (i === tileIdx ? -1 : v));
      focusedTileIdx = targetTile;
      tileFocus = tileFocus.map((v, i) => (i === targetTile ? targetCell : v));
      focusCell(targetTile, targetCell);
    }

    switch (e.key) {
      case "ArrowLeft": {
        e.preventDefault();
        const col = idx % cols;
        if (col === 0 && tileIdx > 0) {
          // Wrap to column 6 (Saturday) of the previous month tile.
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
          // Wrap to column 0 (Sunday) of the next month tile.
          moveAcrossTile(tileIdx + 1, idx + 1);
        } else {
          moveWithinTile(+1);
        }
        return;
      }
      case "ArrowUp": {
        e.preventDefault();
        if (idx < cols) {
          // First row of the tile — wrap to previous month tile.
          if (tileIdx > 0) {
            const targetTile = tileIdx - 1;
            const targetCol = idx % cols;
            const prevMonth = targetTile + 1;
            const prevFirstDow = firstDowOfMonth(viewYear, prevMonth);
            const prevTotalDays = daysInMonth(viewYear, prevMonth);
            // Cell index of the last day in the previous month.
            const prevLastCellIdx = prevFirstDow + (prevTotalDays - 1);
            // Wrap to the cell at the same column in the last row of the
            // previous month (clamp to prevLastCellIdx if that column is blank).
            const rowBase = prevLastCellIdx - (prevLastCellIdx % cols);
            const targetIdx = Math.min(prevLastCellIdx, rowBase + targetCol);
            moveAcrossTile(targetTile, targetIdx);
          } else {
            moveWithinTile(0); // January first row — no wrap.
          }
        } else {
          moveWithinTile(-cols);
        }
        return;
      }
      case "ArrowDown": {
        e.preventDefault();
        const nextIdx = idx + cols;
        const nextCell = cells[nextIdx];
        if (nextIdx >= 42 || !nextCell?.iso) {
          // Last row of the tile or cell below is blank — wrap to next month.
          if (tileIdx < 11) {
            const targetTile = tileIdx + 1;
            const targetCol = idx % cols;
            const nextMonth = targetTile + 1;
            const nextFirstDow = firstDowOfMonth(viewYear, nextMonth);
            // Walk down from the first row of the next month to find the
            // first in-range cell at the target column.
            let targetIdx = nextFirstDow + targetCol;
            // Clamp to 41 if the target column is past the last cell.
            if (targetIdx > 41) targetIdx = 41;
            moveAcrossTile(targetTile, targetIdx);
          } else {
            moveWithinTile(0); // December last row — no wrap.
          }
        } else {
          moveWithinTile(+cols);
        }
        return;
      }
      case "Enter":
      case " ": {
        e.preventDefault();
        const cell = cells[idx];
        if (cell?.iso && inRange(cell.iso)) {
          dispatch("selectDate", cell.iso);
        }
        return;
      }
      case "Escape": {
        e.preventDefault();
        // Return focus to the Today button in the page-level year-nav.
        if (typeof document !== "undefined") {
          const prev = document.querySelector<HTMLElement>("[data-cal-nav-today]");
          prev?.focus();
        }
        return;
      }
      case "PageUp":
      case "PageDown": {
        // Year-boundary navigation is owned by the page-level chevrons.
        e.preventDefault();
        return;
      }
      default:
        return;
    }
  }
</script>

<div
  class="cal-year-grid"
  role="application"
  aria-label={ariaLabel}
>
  <div class="cal-year-grid-tiles">
    {#each Array.from({ length: 12 }, (_, i) => i) as mIdx}
      {@const month = mIdx + 1}
      {@const monthName = MONTH_NAMES[mIdx] ?? ""}
      {@const tileGrid = buildMonthGrid(viewYear, month)}
      {@const tileTabindex = focusedTileIdx === mIdx ? 0 : -1}

      <div class="cal-tile">
        <!-- Tile header: read-only (D7 — no drill-down) -->
        <div class="cal-tile-header">
          <span class="cal-tile-month">{monthName}</span>
          <span class="cal-tile-year">{viewYear}</span>
        </div>

        <!-- Tile body -->
        <div class="cal-tile-body">
          <!-- Weekday row -->
          <div class="cal-tile-weekdays" aria-hidden="true">
            {#each WEEKDAY_SHORT as wd}
              <span class="cal-tile-weekday">{wd}</span>
            {/each}
          </div>

          <!-- Day grid -->
          <!-- svelte-ignore a11y-no-noninteractive-element-to-interactive-role -->
          <div
            class="cal-tile-grid"
            role="grid"
            tabindex={tileTabindex}
            aria-label={$LL.calendar.tileMonthLabel({ month: monthName, year: viewYear })}
          >
            {#each tileGrid as cell (cell.idx)}
              {#if cell.iso && !cell.isOutside}
                {@const hasBadge = !!(dayBadges && dayBadges[cell.iso] > 0)}
                {@const isFocused = focusedTileIdx === mIdx && tileFocus[mIdx] === cell.idx}
                <button
                  type="button"
                  role="gridcell"
                  class="cal-day"
                  class:day-today={cell.iso === todayDate}
                  class:day-selected={cell.iso === selectedDate}
                  class:day-disabled={!inRange(cell.iso)}
                  class:day-badge={hasBadge}
                  class:day-weekend={isWeekend(cell)}
                  class:day-focused={isFocused}
                  aria-label={cellAriaLabel(cell, monthName)}
                  aria-selected={cell.iso === selectedDate}
                  aria-disabled={!inRange(cell.iso)}
                  tabindex={isFocused ? 0 : -1}
                  disabled={!inRange(cell.iso)}
                  data-tile-idx={mIdx}
                  data-cell-idx={cell.idx}
                  on:click={() => selectDay(cell.iso)}
                  on:keydown={(e) => onTileDayKeydown(e, mIdx, tileGrid)}
                >
                  {cell.day}
                  {#if hasBadge}
                    <span class="badge-dot" aria-hidden="true"></span>
                  {/if}
                </button>
              {:else}
                <span
                  role="gridcell"
                  class="cal-day cal-day-blank"
                  aria-hidden="true"
                ></span>
              {/if}
            {/each}
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
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

  /* Today: distinct highlight */
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

  /* Badge dot (3px — smaller than CalendarMonth's 4px dot) */
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
</style>
