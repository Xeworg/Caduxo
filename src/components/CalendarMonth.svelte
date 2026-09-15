<script lang="ts">
  import { createEventDispatcher } from "svelte";

  // ── Props ───────────────────────────────────────────────────────────────────

  export let viewYear: number;
  export let viewMonth: number;
  export let selectedDate: string | null = null;
  export let todayDate: string = "";
  export let dayBadges: Record<string, number> | undefined = undefined;
  export let minDate: string = "1900-01-01";
  export let maxDate: string = "2100-12-31";
  export let ariaLabel: string = "Calendar";

  const dispatch = createEventDispatcher<{
    daySelect: string;
    monthChange: { year: number; month: number };
    viewYearChange: number;
    viewMonthChange: number;
    escape: void;
  }>();

  // ── Labels ─────────────────────────────────────────────────────────────────

  const LABELS = {
    weekdays: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const,
    months: [
      "January", "February", "March", "April", "May", "June",
      "July", "August", "September", "October", "November", "December",
    ] as const,
  };

  // ── Helpers ────────────────────────────────────────────────────────────────

  /** Pad a number to two digits. */
  function pad(n: number): string {
    return n < 10 ? "0" + n : String(n);
  }

  /** Build a local-date ISO string from year/month/day parts. */
  function isoDate(year: number, month: number, day: number): string {
    return `${year}-${pad(month)}-${pad(day)}`;
  }

  /** Parse an ISO date string (YYYY-MM-DD) to a local Date. */
  function parseIso(s: string): Date {
    const [y, m, d] = s.split("-").map(Number);
    return new Date(y, m - 1, d);
  }

  /** Compare two ISO dates lexicographically. */
  function cmpIso(a: string, b: string): number {
    return a < b ? -1 : a > b ? 1 : 0;
  }

  /** Return the last day of a given month. */
  function daysInMonth(year: number, month: number): number {
    return new Date(year, month, 0).getDate();
  }

  /** Build 42-cell month grid. Leading/trailing cells encode { iso:"", day:0 }. */
  function buildMonthGrid(
    year: number,
    month: number,
  ): { iso: string; day: number; isOutside: boolean }[] {
    const cells: { iso: string; day: number; isOutside: boolean }[] = [];
    const firstDow = new Date(year, month - 1, 1).getDay(); // 0=Sun
    const totalDays = daysInMonth(year, month);

    // Leading blanks
    for (let i = 0; i < firstDow; i++) {
      cells.push({ iso: "", day: 0, isOutside: true });
    }
    // Actual days
    for (let d = 1; d <= totalDays; d++) {
      cells.push({ iso: isoDate(year, month, d), day: d, isOutside: false });
    }
    // Trailing blanks to reach 42 cells
    while (cells.length < 42) {
      cells.push({ iso: "", day: 0, isOutside: true });
    }
    return cells;
  }

  /** Is an ISO date within [min, max] (inclusive)? */
  function inRange(iso: string): boolean {
    if (!iso) return false;
    return cmpIso(iso, minDate) >= 0 && cmpIso(iso, maxDate) <= 0;
  }

  // ── State ───────────────────────────────────────────────────────────────────

  let showYearPicker = false;
  let pickerYear: number = viewYear;

  // Decades centered around pickerYear (e.g. pickerYear=2026 → 2020–2029)
  $: decadeStart = Math.floor(pickerYear / 10) * 10;
  $: decadeYears = Array.from({ length: 10 }, (_, i) => decadeStart + i);

  // Current display year (may differ from pickerYear when picker is open)
  $: displayYear = showYearPicker ? pickerYear : viewYear;

  // Build the month grid for the current view
  $: grid = buildMonthGrid(viewYear, viewMonth);

  // Focused day state
  let focusedIso: string = "";

  function resetFocused() {
    // Prefer selectedDate, fall back to todayDate if visible in current month
    const todayIso = todayDate
      ? isoDate(
          new Date().getFullYear(),
          new Date().getMonth() + 1,
          new Date().getDate(),
        )
      : "";
    if (
      selectedDate &&
      selectedDate.startsWith(`${viewYear}-${pad(viewMonth)}`)
    ) {
      focusedIso = selectedDate;
    } else if (
      todayDate &&
      todayDate.startsWith(`${viewYear}-${pad(viewMonth)}`)
    ) {
      focusedIso = todayDate;
    } else {
      // First visible day in the grid
      const firstDay = grid.find((c) => !c.isOutside);
      focusedIso = firstDay?.iso ?? "";
    }
  }

  // Re-evaluate focused cell whenever the view changes
  $: if (viewYear || viewMonth) {
    resetFocused();
  }

  // ── Navigation ───────────────────────────────────────────────────────────────

  function prevMonth() {
    let m = viewMonth - 1;
    let y = viewYear;
    if (m < 1) { m = 12; y -= 1; }
    dispatch("monthChange", { year: y, month: m });
  }

  function nextMonth() {
    let m = viewMonth + 1;
    let y = viewYear;
    if (m > 12) { m = 1; y += 1; }
    dispatch("monthChange", { year: y, month: m });
  }

  function cycleMonth() {
    // Clicking the month label cycles: Jan→Feb→…→Dec→Jan
    let m = viewMonth + 1;
    let y = viewYear;
    if (m > 12) { m = 1; y += 1; }
    dispatch("monthChange", { year: y, month: m });
  }

  function openYearPicker() {
    pickerYear = viewYear;
    showYearPicker = true;
  }

  function selectYear(year: number) {
    dispatch("viewYearChange", year);
    showYearPicker = false;
  }

  function prevDecade() {
    pickerYear = Math.max(1900, pickerYear - 10);
  }

  function nextDecade() {
    pickerYear = Math.min(2100, pickerYear + 10);
  }

  // ── Day selection ─────────────────────────────────────────────────────────────

  function selectDay(iso: string) {
    if (!iso || !inRange(iso)) return;
    focusedIso = iso;
    dispatch("daySelect", iso);
  }

  // ── Keyboard navigation ──────────────────────────────────────────────────────

  function moveFocusedDays(delta: number) {
    if (!focusedIso) return;
    const d = parseIso(focusedIso);
    d.setDate(d.getDate() + delta);
    let iso = isoDate(d.getFullYear(), d.getMonth() + 1, d.getDate());
    // Clamp to [minDate, maxDate]
    if (cmpIso(iso, minDate) < 0) iso = minDate;
    if (cmpIso(iso, maxDate) > 0) iso = maxDate;
    focusedIso = iso;

    // If the new focused date moved outside the visible month, navigate
    const [fy, fm] = [
      parseInt(focusedIso.slice(0, 4)),
      parseInt(focusedIso.slice(5, 7)),
    ];
    if (fy !== viewYear || fm !== viewMonth) {
      dispatch("monthChange", { year: fy, month: fm });
    }
  }

  function moveFocusedMonths(delta: number) {
    if (!focusedIso) return;
    const d = parseIso(focusedIso);
    d.setMonth(d.getMonth() + delta);
    // Clamp day to the target month length
    const targetDay = Math.min(
      d.getDate(),
      daysInMonth(d.getFullYear(), d.getMonth() + 1),
    );
    d.setDate(targetDay);
    let iso = isoDate(d.getFullYear(), d.getMonth() + 1, d.getDate());
    if (cmpIso(iso, minDate) < 0) iso = minDate;
    if (cmpIso(iso, maxDate) > 0) iso = maxDate;
    focusedIso = iso;

    const [fy, fm] = [
      parseInt(focusedIso.slice(0, 4)),
      parseInt(focusedIso.slice(5, 7)),
    ];
    if (fy !== viewYear || fm !== viewMonth) {
      dispatch("monthChange", { year: fy, month: fm });
    }
  }

  function moveFocusedYears(delta: number) {
    if (!focusedIso) return;
    const d = parseIso(focusedIso);
    d.setFullYear(d.getFullYear() + delta);
    let iso = isoDate(d.getFullYear(), d.getMonth() + 1, d.getDate());
    if (cmpIso(iso, minDate) < 0) iso = minDate;
    if (cmpIso(iso, maxDate) > 0) iso = maxDate;
    focusedIso = iso;

    const [fy, fm] = [
      parseInt(focusedIso.slice(0, 4)),
      parseInt(focusedIso.slice(5, 7)),
    ];
    if (fy !== viewYear || fm !== viewMonth) {
      dispatch("monthChange", { year: fy, month: fm });
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (showYearPicker) {
      // In year picker, only close on Escape
      if (e.key === "Escape") {
        showYearPicker = false;
        e.preventDefault();
      }
      return;
    }

    switch (e.key) {
      case "ArrowLeft":
        e.preventDefault();
        moveFocusedDays(-1);
        break;
      case "ArrowRight":
        e.preventDefault();
        moveFocusedDays(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveFocusedDays(-7);
        break;
      case "ArrowDown":
        e.preventDefault();
        moveFocusedDays(7);
        break;
      case "PageUp":
        e.preventDefault();
        if (e.shiftKey) moveFocusedYears(-1);
        else moveFocusedMonths(-1);
        break;
      case "PageDown":
        e.preventDefault();
        if (e.shiftKey) moveFocusedYears(1);
        else moveFocusedMonths(1);
        break;
      case "Enter":
        if (focusedIso && inRange(focusedIso)) {
          e.preventDefault();
          selectDay(focusedIso);
        }
        break;
      case "Escape":
        e.preventDefault();
        dispatch("escape");
        break;
    }
  }

  // ── Helpers ─────────────────────────────────────────────────────────────────

  function isWeekend(cell: { iso: string; day: number }): boolean {
    if (!cell.iso) return false;
    const dow = new Date(
      parseInt(cell.iso.slice(0, 4)),
      parseInt(cell.iso.slice(5, 7)) - 1,
      cell.day,
    ).getDay();
    return dow === 0 || dow === 6;
  }

  function cellAriaLabel(cell: { iso: string; day: number }): string {
    if (!cell.iso) return "";
    const d = parseIso(cell.iso);
    return `${LABELS.months[d.getMonth()]} ${d.getDate()}, ${d.getFullYear()}`;
  }
</script>

<div class="cal-month" aria-label={ariaLabel} role="application">
  <!-- Header -->
  <div class="cal-header">
    <button
      type="button"
      class="cal-nav"
      aria-label="Previous month"
      on:click={prevMonth}
    >
      ‹
    </button>

    <div class="cal-title">
      {#if showYearPicker}
        <button type="button" class="cal-year-btn" on:click={openYearPicker}>
          {displayYear}
        </button>
        <div class="year-picker">
          <div class="year-picker-header">
            <button
              type="button"
              class="cal-nav cal-nav-sm"
              aria-label="Previous decade"
              on:click={prevDecade}
            >
              ‹
            </button>
            <span class="decade-label">
              {decadeStart}–{decadeStart + 9}
            </span>
            <button
              type="button"
              class="cal-nav cal-nav-sm"
              aria-label="Next decade"
              on:click={nextDecade}
            >
              ›
            </button>
          </div>
          <div class="year-grid">
            {#each decadeYears as yr}
              {@const disabled = yr < 1900 || yr > 2100}
              <button
                type="button"
                class="year-chip"
                class:year-selected={yr === viewYear}
                class:year-disabled={disabled}
                aria-label="Year {yr}"
                aria-pressed={yr === viewYear}
                {disabled}
                on:click={() => !disabled && selectYear(yr)}
              >
                {yr}
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <button
          type="button"
          class="cal-month-btn"
          aria-label="Cycle month"
          on:click={cycleMonth}
        >
          {LABELS.months[viewMonth - 1]}
        </button>
        <button
          type="button"
          class="cal-year-btn"
          aria-label="Open year picker"
          on:click={openYearPicker}
        >
          {displayYear} ▼
        </button>
      {/if}
    </div>

    <button
      type="button"
      class="cal-nav"
      aria-label="Next month"
      on:click={nextMonth}
    >
      ›
    </button>
  </div>

  <!-- Weekday row -->
  <div class="cal-weekdays" aria-hidden="true">
    {#each LABELS.weekdays as wd}
      <span class="cal-weekday">{wd}</span>
    {/each}
  </div>

  <!-- Day grid -->
  <!-- svelte-ignore a11y-no-noninteractive-element-to-interactive-role -->
  <div
    class="cal-grid"
    role="grid"
    aria-label="{LABELS.months[viewMonth - 1]} {viewYear}"
    tabindex="0"
    on:keydown={onKeydown}
  >
    {#each grid as cell (cell.iso + cell.day)}
      {#if cell.iso && !cell.isOutside}
        {@const isToday = cell.iso === todayDate}
        {@const isSelected = cell.iso === selectedDate}
        {@const isFocused = cell.iso === focusedIso}
        {@const isDisabled = !inRange(cell.iso)}
        {@const hasBadge = !!(dayBadges && dayBadges[cell.iso] > 0)}
        {@const weekend = isWeekend(cell)}
        <button
          type="button"
          role="gridcell"
          class="cal-day"
          class:day-today={isToday}
          class:day-selected={isSelected}
          class:day-disabled={isDisabled}
          class:day-badge={hasBadge}
          class:day-outside={false}
          class:day-weekend={weekend}
          class:day-focused={isFocused}
          aria-label={cellAriaLabel(cell)}
          aria-selected={isSelected}
          aria-disabled={isDisabled}
          tabindex={isFocused ? 0 : -1}
          disabled={isDisabled}
          on:click={() => !isDisabled && selectDay(cell.iso)}
        >
          {cell.day}
          {#if hasBadge}
            <span class="badge-dot" aria-hidden="true"></span>
          {/if}
        </button>
      {:else}
        <!-- Blank outside cell -->
        <span
          role="gridcell"
          class="cal-day cal-day-blank"
          aria-hidden="true"
        ></span>
      {/if}
    {/each}
  </div>

  <!-- Footer slot for Today shortcut (used by DatePicker) -->
  <slot name="footer" />
</div>

<style>
  .cal-month {
    display: inline-flex;
    flex-direction: column;
    gap: 0;
    user-select: none;
    width: 280px;
    font-family: inherit;
  }

  /* ── Header ─────────────────────────────────────────────────────────────── */

  .cal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 4px;
    gap: 4px;
  }

  .cal-nav {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    color: #1e293b;
    padding: 4px 8px;
    border-radius: 4px;
    line-height: 1;
    transition: background 0.15s;
  }

  .cal-nav:hover {
    background: #e2e8f0;
  }

  .cal-nav:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  .cal-nav-sm {
    font-size: 0.9rem;
    padding: 2px 6px;
  }

  .cal-title {
    display: flex;
    align-items: center;
    gap: 4px;
    position: relative;
  }

  .cal-month-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    color: #1e293b;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .cal-month-btn:hover {
    background: #e2e8f0;
  }

  .cal-month-btn:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  .cal-year-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    color: #1e293b;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .cal-year-btn:hover {
    background: #e2e8f0;
  }

  .cal-year-btn:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  /* ── Year picker ─────────────────────────────────────────────────────────── */

  .year-picker {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
    padding: 8px;
    width: 220px;
  }

  .year-picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .decade-label {
    font-size: 0.8rem;
    color: #64748b;
    font-weight: 600;
  }

  .year-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 2px;
  }

  .year-chip {
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 4px 2px;
    border-radius: 4px;
    color: #1e293b;
    transition: background 0.1s;
    text-align: center;
  }

  .year-chip:hover:not(.year-disabled) {
    background: #e2e8f0;
  }

  .year-chip.year-selected {
    background: #2563eb;
    color: #fff;
    border-color: #2563eb;
  }

  .year-chip.year-disabled {
    color: #cbd5e1;
    cursor: not-allowed;
  }

  .year-chip:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  /* ── Weekdays ───────────────────────────────────────────────────────────── */

  .cal-weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    padding: 4px 0;
    border-bottom: 1px solid #e2e8f0;
  }

  .cal-weekday {
    text-align: center;
    font-size: 0.7rem;
    font-weight: 700;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 2px 0;
  }

  /* ── Day grid ───────────────────────────────────────────────────────────── */

  .cal-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 1px;
    padding: 4px 0;
  }

  .cal-day {
    position: relative;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.8rem;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: #1e293b;
    transition: background 0.1s, color 0.1s;
    padding: 0;
  }

  .cal-day:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
    z-index: 1;
  }

  .cal-day:hover:not(.day-disabled):not(.day-selected) {
    background: #e2e8f0;
  }

  /* Today: distinct highlight */
  .cal-day.day-today {
    color: #2563eb;
    font-weight: 700;
  }

  /* Selected: filled blue */
  .cal-day.day-selected {
    background: #2563eb;
    color: #fff;
    font-weight: 600;
  }

  .cal-day.day-selected.day-today {
    /* both today and selected: blue wins */
    color: #fff;
  }

  /* Disabled */
  .cal-day.day-disabled {
    color: #cbd5e1;
    cursor: not-allowed;
  }

  /* Weekend: muted */
  .cal-day.day-weekend:not(.day-selected):not(.day-disabled) {
    color: #94a3b8;
  }

  /* Outside month: blank placeholder */
  .cal-day-blank {
    cursor: default;
  }

  /* Focused ring */
  .cal-day.day-focused:not(.day-selected) {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  /* Badge dot */
  .badge-dot {
    position: absolute;
    bottom: 2px;
    left: 50%;
    transform: translateX(-50%);
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: #f59e0b;
  }

  .cal-day.day-selected .badge-dot {
    background: rgba(255, 255, 255, 0.8);
  }
</style>
