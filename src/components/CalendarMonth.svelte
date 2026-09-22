<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import Button from "./ui/Button.svelte";
  import Tooltip from "./ui/Tooltip.svelte";

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

  // ── Locale-backed arrays ──
  $: MONTH_NAMES = $LL.calendar.monthNames
    ? Array.from({ length: 12 }, (_, i) => ($LL.calendar.monthNames as Record<string, () => string>)[String(i)]())
    : [];
  $: WEEKDAY_NAMES = $LL.calendar.weekdayShort
    ? Array.from({ length: 7 }, (_, i) => ($LL.calendar.weekdayShort as Record<string, () => string>)[String(i)]())
    : [];
  $: monthGridLabel = `${MONTH_NAMES[viewMonth - 1] ?? ""} ${viewYear}`;

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
  ): { idx: number; iso: string; day: number; isOutside: boolean }[] {
    const cells: { idx: number; iso: string; day: number; isOutside: boolean }[] = [];
    const firstDow = new Date(year, month - 1, 1).getDay(); // 0=Sun
    const totalDays = daysInMonth(year, month);

    // Each cell carries a stable, position-unique `idx` so the keyed-each has
    // a distinct key even for the blank cells (which all share iso=""+day=0).
    let idx = 0;
    function pushBlank() {
      cells.push({ idx: idx++, iso: "", day: 0, isOutside: true });
    }
    // Leading blanks
    for (let i = 0; i < firstDow; i++) pushBlank();
    // Actual days
    for (let d = 1; d <= totalDays; d++) {
      cells.push({ idx: idx++, iso: isoDate(year, month, d), day: d, isOutside: false });
    }
    // Trailing blanks to reach 42 cells
    while (cells.length < 42) pushBlank();
    return cells;
  }

  /** Is an ISO date within [min, max] (inclusive)? */
  function inRange(iso: string): boolean {
    if (!iso) return false;
    return cmpIso(iso, minDate) >= 0 && cmpIso(iso, maxDate) <= 0;
  }

  // ── State ───────────────────────────────────────────────────────────────────

  let showYearPicker = false;
  let showMonthPicker = false;
  let pickerYear: number = viewYear;
  let rootEl: HTMLDivElement;

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

  function openMonthPicker() {
    pickerYear = viewYear;
    showMonthPicker = true;
    showYearPicker = false; // mutual exclusion
  }

  function closePickers() {
    showYearPicker = false;
    showMonthPicker = false;
  }

  function closeMonthPicker() {
    showMonthPicker = false;
  }

  function selectMonth(month: number) {
    dispatch("monthChange", { year: pickerYear, month });
    showMonthPicker = false;
  }

  function prevPickerYear() {
    pickerYear = Math.max(1900, pickerYear - 1);
  }

  function nextPickerYear() {
    pickerYear = Math.min(2100, pickerYear + 1);
  }

  function isMonthInRange(year: number, month: number): boolean {
    // A month is in range iff at least one day in (year, month) lies
    // within [minDate, maxDate].
    const lastDay = daysInMonth(year, month);
    const firstIso = isoDate(year, month, 1);
    const lastIso  = isoDate(year, month, lastDay);
    return cmpIso(lastIso, minDate) >= 0 && cmpIso(firstIso, maxDate) <= 0;
  }

  function onMonthChipKeydown(e: KeyboardEvent) {
    const target = e.currentTarget as HTMLButtonElement;
    const idx = MONTH_NAMES.indexOf(target.textContent ?? "");
    if (idx < 0) return;
    const cols = 3;
    let next = idx;
    switch (e.key) {
      case "ArrowLeft":  next = idx - 1; break;
      case "ArrowRight": next = idx + 1; break;
      case "ArrowUp":    next = idx - cols; break;
      case "ArrowDown":  next = idx + cols; break;
      case "Enter":
      case " ":
        if (!target.disabled) {
          e.preventDefault();
          selectMonth(idx + 1);
        }
        return;
      case "Escape":
        e.preventDefault();
        showMonthPicker = false;
        return;
      default:
        return;
    }
    e.preventDefault();
    // Clamp inside the 3-column grid
    if (next < 0) next = 0;
    if (next > 11) next = 11;
    const grid = target.parentElement!.querySelectorAll<HTMLButtonElement>(".month-chip:not(:disabled)");
    grid.forEach((b) => b.tabIndex = -1);
    grid[next]?.focus();
  }

  function openYearPicker() {
    pickerYear = viewYear;
    showYearPicker = true;
    showMonthPicker = false; // mutual exclusion
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
    if (showYearPicker || showMonthPicker) {
      // In any picker, only close on Escape from the day-grid handler.
      // Picker-internal arrows / Enter / Space are handled by the overlay.
      if (e.key === "Escape") {
        closePickers();
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
    return $LL.calendar.ariaDayCell({
      month: MONTH_NAMES[d.getMonth()] ?? "",
      day: d.getDate(),
      year: d.getFullYear(),
    });
  }

  onMount(() => {
    function onDocumentPointerDown(event: PointerEvent) {
      if (!showYearPicker && !showMonthPicker) return;
      const target = event.target as Node | null;
      if (target && rootEl?.contains(target)) return;
      closePickers();
    }

    document.addEventListener("pointerdown", onDocumentPointerDown, true);
    return () => document.removeEventListener("pointerdown", onDocumentPointerDown, true);
  });
</script>

<div class="cal-month" aria-label={ariaLabel} role="application" bind:this={rootEl}>
  <!-- Header -->
  <div class="cal-header">
    <Tooltip text={$LL.calendar.ariaPreviousMonth()} position="bottom">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.calendar.ariaPreviousMonth()}
        onclick={prevMonth}
      >
        ‹
      </Button>
    </Tooltip>

    <div class="cal-title">
      {#if showYearPicker}
        <Button
          variant="ghost"
          size="sm"
          aria-label={$LL.calendar.ariaOpenYearPicker()}
          onclick={openYearPicker}
        >
          {displayYear}
        </Button>
        <div class="year-picker">
          <div class="year-picker-header">
            <Tooltip text={$LL.calendar.ariaPreviousDecade()} position="bottom">
              <Button
                variant="ghost"
                size="sm"
                aria-label={$LL.calendar.ariaPreviousDecade()}
                onclick={prevDecade}
              >
                ‹
              </Button>
            </Tooltip>
            <span class="decade-label">
              {decadeStart}–{decadeStart + 9}
            </span>
            <Tooltip text={$LL.calendar.ariaNextDecade()} position="bottom">
              <Button
                variant="ghost"
                size="sm"
                aria-label={$LL.calendar.ariaNextDecade()}
                onclick={nextDecade}
              >
                ›
              </Button>
            </Tooltip>
          </div>
          <div class="year-grid">
            {#each decadeYears as yr}
              {@const disabled = yr < 1900 || yr > 2100}
              <button
                type="button"
                class="year-chip"
                class:year-selected={yr === viewYear}
                class:year-disabled={disabled}
                aria-label={$LL.calendar.ariaYear({ year: yr })}
                aria-pressed={yr === viewYear}
                {disabled}
                on:click={() => !disabled && selectYear(yr)}
              >
                {yr}
              </button>
            {/each}
          </div>
        </div>
      {:else if showMonthPicker}
        <Button
          variant="ghost"
          size="sm"
          aria-label={$LL.calendar.ariaOpenYearPicker()}
          onclick={openYearPicker}
        >
          {pickerYear}
        </Button>
        <div class="month-picker" role="dialog" aria-label={$LL.calendar.ariaCloseMonthPicker()}>
          <div class="month-picker-header">
            <Tooltip text={$LL.calendar.ariaPreviousYear()} position="bottom">
              <Button
                variant="ghost"
                size="sm"
                aria-label={$LL.calendar.ariaPreviousYear()}
                onclick={prevPickerYear}
              >
                ‹
              </Button>
            </Tooltip>
            <span class="picker-year-label">{pickerYear}</span>
            <Tooltip text={$LL.calendar.ariaNextYear()} position="bottom">
              <Button
                variant="ghost"
                size="sm"
                aria-label={$LL.calendar.ariaNextYear()}
                onclick={nextPickerYear}
              >
                ›
              </Button>
            </Tooltip>
          </div>
          <div class="month-grid" role="grid" aria-label={$LL.calendar.ariaCloseMonthPicker()}>
            {#each MONTH_NAMES as monthName, i (i)}
              {@const m = i + 1}
              {@const inRange = isMonthInRange(pickerYear, m)}
              {@const isSelected = pickerYear === viewYear && m === viewMonth}
              <button
                type="button"
                class="month-chip"
                class:month-selected={isSelected}
                class:month-disabled={!inRange}
                aria-label={$LL.calendar.ariaMonth({ month: monthName, year: pickerYear })}
                aria-pressed={isSelected}
                aria-disabled={!inRange}
                tabindex={isSelected ? 0 : -1}
                disabled={!inRange}
                on:click={() => inRange && selectMonth(m)}
                on:keydown={onMonthChipKeydown}
              >
                {monthName}
              </button>
            {/each}
          </div>
        </div>
      {:else}
        <Button
          variant="ghost"
          size="sm"
          aria-label={$LL.calendar.ariaOpenMonthPicker()}
          onclick={openMonthPicker}
        >
          {MONTH_NAMES[viewMonth - 1] ?? ""}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          aria-label={$LL.calendar.ariaOpenYearPicker()}
          onclick={openYearPicker}
        >
          {displayYear} ▼
        </Button>
      {/if}
    </div>

    <Tooltip text={$LL.calendar.ariaNextMonth()} position="bottom">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.calendar.ariaNextMonth()}
        onclick={nextMonth}
      >
        ›
      </Button>
    </Tooltip>
  </div>

  <!-- Weekday row -->
  <div class="cal-weekdays" aria-hidden="true">
    {#each WEEKDAY_NAMES as wd}
      <span class="cal-weekday">{wd}</span>
    {/each}
  </div>

  <!-- Day grid -->
  <!-- svelte-ignore a11y-no-noninteractive-element-to-interactive-role -->
  <div
    class="cal-grid"
    role="grid"
    aria-label={monthGridLabel}
    tabindex="0"
    on:keydown={onKeydown}
  >
    {#each grid as cell (cell.idx)}
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

  .cal-title {
    display: flex;
    align-items: center;
    gap: 4px;
    position: relative;
  }

  /* ── Year picker ─────────────────────────────────────────────────────────── */

  .year-picker {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-200);
    border-radius: 8px;
    /* Theme-derived shadow: a generic literal alpha shadow is acceptable
       but a theme-derived one tracks light/dark. Keep the alpha very low
       so neither theme bleeds through. */
    box-shadow: 0 4px 12px color-mix(in oklch, var(--color-base-content) 12%, transparent);
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
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
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
    color: var(--color-base-content);
    transition: background 0.1s;
    text-align: center;
  }

  .year-chip:hover:not(.year-disabled) {
    background: color-mix(in oklch, var(--color-base-300) 50%, transparent);
  }

  .year-chip.year-selected {
    background: var(--color-primary);
    color: var(--color-base-100);
    border-color: var(--color-primary);
  }

  .year-chip.year-disabled {
    color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
    cursor: not-allowed;
  }

  .year-chip:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  /* ── Weekdays ───────────────────────────────────────────────────────────── */

  .cal-weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    padding: 4px 0;
    border-bottom: 1px solid var(--color-base-200);
  }

  .cal-weekday {
    text-align: center;
    font-size: 0.7rem;
    font-weight: 700;
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
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
    color: var(--color-base-content);
    transition: background 0.1s, color 0.1s;
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

  /* Selected: filled blue */
  .cal-day.day-selected {
    background: var(--color-primary);
    color: var(--color-base-100);
    font-weight: 600;
  }

  .cal-day.day-selected.day-today {
    /* both today and selected: blue wins */
    color: var(--color-base-100);
  }

  /* Disabled */
  .cal-day.day-disabled {
    color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
    cursor: not-allowed;
  }

  /* Weekend: muted */
  .cal-day.day-weekend:not(.day-selected):not(.day-disabled) {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
  }

  /* Outside month: blank placeholder */
  .cal-day-blank {
    cursor: default;
  }

  /* Focused ring */
  .cal-day.day-focused:not(.day-selected) {
    outline: 2px solid var(--color-primary);
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
    background: var(--color-warning);
  }

  .cal-day.day-selected .badge-dot {
    background: color-mix(in oklch, var(--color-base-100) 80%, transparent);
  }

  /* ── Month picker ─────────────────────────────────────────────────────────── */

  .month-picker {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-200);
    border-radius: 8px;
    box-shadow: 0 4px 12px color-mix(in oklch, var(--color-base-content) 12%, transparent);
    padding: 8px;
    width: 240px;
  }

  .month-picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .picker-year-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-base-content);
  }

  .month-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 2px;
  }

  .month-chip {
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 6px 2px;
    border-radius: 4px;
    color: var(--color-base-content);
    transition: background 0.1s;
    text-align: center;
  }

  .month-chip:hover:not(.month-disabled) {
    background: color-mix(in oklch, var(--color-base-300) 50%, transparent);
  }

  .month-chip.month-selected {
    background: var(--color-primary);
    color: var(--color-base-100);
    border-color: var(--color-primary);
  }

  .month-chip.month-disabled {
    color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
    cursor: not-allowed;
  }

  .month-chip:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
</style>
