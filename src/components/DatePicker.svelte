<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import CalendarMonth from "./CalendarMonth.svelte";

  // ── Props ───────────────────────────────────────────────────────────────────

  export let value: string = "";
  export let clearable: boolean = true;
  export let minDate: string = "1900-01-01";
  export let maxDate: string = "2100-12-31";
  export let ariaLabel: string = "Date";
  export let id: string | undefined = undefined;
  export let placeholder: string = "YYYY-MM-DD";
  export let name: string | undefined = undefined;
  export let todayDate: string = todayIso();

  // ── Validation helper ────────────────────────────────────────────────────────

  /** Parse a YYYY-MM-DD string; return { ok, reason } */
  export function parseIsoDate(
    input: string,
    minD: string = minDate,
    maxD: string = maxDate,
  ): { ok: true; iso: string } | { ok: false; reason: "shape" | "calendar" | "range" } {
    const shape = /^\d{4}-\d{2}-\d{2}$/;
    if (!shape.test(input)) return { ok: false, reason: "shape" };

    const [y, m, d] = input.split("-").map(Number);

    // Out-of-range year
    if (y < 1900 || y > 2100) return { ok: false, reason: "range" };

    // Calendar validity (e.g. Feb 30)
    const maxDay = new Date(y, m, 0).getDate();
    if (d < 1 || d > maxDay) return { ok: false, reason: "calendar" };

    const iso = `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;

    if (iso < minD || iso > maxD) return { ok: false, reason: "range" };

    return { ok: true, iso };
  }

  function todayIso(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  // ── Local state ─────────────────────────────────────────────────────────────

  // What the <input> currently renders — decoupled from `value`
  let displayText: string = value ?? "";

  // Sync displayText when value changes externally (e.g. form reset)
  $: if (value === "" && displayText !== "") {
    // intentional: let user clear
  } else if (value !== displayText) {
    displayText = value;
  }

  let isOpen = false;
  let isInvalid = false;
  let invalidReason = "";

  let triggerEl: HTMLDivElement;
  let popoverEl: HTMLDivElement;
  let inputEl: HTMLInputElement;

  // Popover view state — may differ from committed value
  let popoverYear: number;
  let popoverMonth: number;

  function initPopoverState() {
    const iso = value || todayDate;
    const d = parseFromIso(iso);
    popoverYear = d.getFullYear();
    popoverMonth = d.getMonth() + 1;
  }

  function parseFromIso(iso: string): Date {
    if (!iso) return new Date();
    const [y, m, day] = iso.split("-").map(Number);
    return new Date(y, m - 1, day);
  }

  // ── Popover positioning ─────────────────────────────────────────────────────



  function positionPopover() {
    if (!isOpen || !triggerEl || !popoverEl) return;
    const rect = triggerEl.getBoundingClientRect();
    const popHeight = popoverEl.offsetHeight || 320;
    const popWidth = popoverEl.offsetWidth || 284;
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    const spaceBelow = vh - rect.bottom;
    const spaceAbove = rect.top;
    const placeBelow = spaceBelow >= popHeight + 8 || spaceBelow >= spaceAbove;

    popoverEl.style.top = placeBelow
      ? `${rect.bottom + 4}px`
      : `${rect.top - popHeight - 4}px`;
    const left = Math.max(8, Math.min(rect.left, vw - popWidth - 8));
    popoverEl.style.left = `${left}px`;
    popoverEl.style.position = "fixed";
  }

  function openPopover() {
    initPopoverState();
    isOpen = true;
    // Position after DOM renders
    requestAnimationFrame(() => positionPopover());
  }

  function closePopover() {
    isOpen = false;
  }

  function togglePopover() {
    if (isOpen) closePopover();
    else openPopover();
  }

  // ── Outside-click & scroll listeners ────────────────────────────────────────

  function onDocMousedown(e: MouseEvent) {
    if (
      !isOpen ||
      !popoverEl ||
      !triggerEl
    )
      return;
    const target = e.target as Node;
    if (
      popoverEl.contains(target) ||
      triggerEl.contains(target)
    )
      return;
    closePopover();
  }

  let scrollTimer: ReturnType<typeof setTimeout> | undefined;
  function onScroll() {
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(positionPopover, 100);
  }

  let resizeTimer: ReturnType<typeof setTimeout> | undefined;
  function onResize() {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(positionPopover, 100);
  }

  onMount(() => {
    document.addEventListener("mousedown", onDocMousedown, true);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onResize);
  });

  onDestroy(() => {
    document.removeEventListener("mousedown", onDocMousedown, true);
    window.removeEventListener("scroll", onScroll, true);
    window.removeEventListener("resize", onResize);
    clearTimeout(scrollTimer);
    clearTimeout(resizeTimer);
  });

  // ── Day selection ───────────────────────────────────────────────────────────

  function onDaySelect(e: CustomEvent<string>) {
    value = e.detail;
    displayText = e.detail;
    isInvalid = false;
    invalidReason = "";
    isOpen = false;
  }

  function onMonthChange(e: CustomEvent<{ year: number; month: number }>) {
    popoverYear = e.detail.year;
    popoverMonth = e.detail.month;
    requestAnimationFrame(() => positionPopover());
  }

  function onViewYearChange(e: CustomEvent<number>) {
    popoverYear = e.detail;
    requestAnimationFrame(() => positionPopover());
  }

  function onEscape() {
    // Close without committing a calendar selection; typed text stays
    closePopover();
  }

  // ── Today shortcut ───────────────────────────────────────────────────────────

  function onToday() {
    const today = todayDate;
    if (today < minDate || today > maxDate) return;
    value = today;
    displayText = today;
    isInvalid = false;
    invalidReason = "";
    isOpen = false;
  }

  function isTodayDisabled(): boolean {
    return todayDate < minDate || todayDate > maxDate;
  }

  // ── Clear button ────────────────────────────────────────────────────────────

  function onClear() {
    value = "";
    displayText = "";
    isInvalid = false;
    invalidReason = "";
    isOpen = false;
  }

  // ── Input handling ───────────────────────────────────────────────────────────

  function onInput() {
    // Re-validate on every keystroke; only update `isInvalid` (not `value`)
    const result = parseIsoDate(displayText, minDate, maxDate);
    if (displayText === "") {
      isInvalid = false;
      invalidReason = "";
      return;
    }
    if (result.ok) {
      isInvalid = false;
      invalidReason = "";
      // value stays as last good value while typing
    } else {
      // Show invalid state immediately during typing
      isInvalid = true;
      invalidReason = result.reason;
    }
  }

  function onBlur() {
    if (displayText === "") {
      if (clearable) {
        value = "";
        isInvalid = false;
        invalidReason = "";
      } else {
        // Required: show invalid, revert to last good value
        displayText = value;
        isInvalid = true;
        invalidReason = "required";
      }
      return;
    }

    const result = parseIsoDate(displayText, minDate, maxDate);
    if (result.ok) {
      value = result.iso;
      displayText = result.iso;
      isInvalid = false;
      invalidReason = "";
    } else {
      // Commit nothing; show error
      isInvalid = true;
      invalidReason = result.reason;
      // displayText stays as typed (no silent rewrite)
      // value stays at last good value
    }
  }

  // ── Input keydown ────────────────────────────────────────────────────────────

  function onInputKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      closePopover();
      // Typed text stays; blur will validate
    }
  }

  // ── Validation helper text ────────────────────────────────────────────────────

  $: helperText = (() => {
    if (!isInvalid) return "";
    if (invalidReason === "shape" || invalidReason === "calendar")
      return "Use YYYY-MM-DD";
    if (invalidReason === "range") return "Year must be 1900–2100";
    if (invalidReason === "required") return "This field is required";
    return "Invalid date";
  })();
</script>

<div class="dp-root" class:dp-open={isOpen}>
  <!-- Trigger -->
  <div class="dp-trigger" bind:this={triggerEl}>
    <input
      bind:this={inputEl}
      type="text"
      inputmode="numeric"
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      {id}
      {name}
      {placeholder}
      aria-label={ariaLabel}
      aria-invalid={isInvalid}
      aria-haspopup="dialog"
      class:dp-invalid={isInvalid}
      value={displayText}
      on:focus={openPopover}
      on:input={onInput}
      on:blur={onBlur}
      on:keydown={onInputKeydown}
    />
    <button
      type="button"
      class="dp-icon"
      aria-label="Open calendar"
      tabindex="0"
      on:click={togglePopover}
    >
      📅
    </button>
    {#if clearable && value !== ""}
      <button
        type="button"
        class="dp-clear"
        aria-label="Clear date"
        on:click={onClear}
      >
        ✕
      </button>
    {/if}
  </div>

  <!-- Helper text -->
  {#if isInvalid && helperText}
    <span class="dp-helper" role="alert">{helperText}</span>
  {/if}

  <!-- Popover -->
  {#if isOpen}
    <div
      bind:this={popoverEl}
      class="dp-popover"
      role="dialog"
      aria-label="{ariaLabel} calendar"
    >
      <CalendarMonth
        viewYear={popoverYear}
        viewMonth={popoverMonth}
        selectedDate={value || null}
        {todayDate}
        {minDate}
        {maxDate}
        ariaLabel="{ariaLabel} calendar"
        on:daySelect={onDaySelect}
        on:monthChange={onMonthChange}
        on:viewYearChange={onViewYearChange}
        on:escape={onEscape}
      >
        <svelte:fragment slot="footer">
          <button
            type="button"
            class="dp-today"
            disabled={isTodayDisabled()}
            on:click={onToday}
          >
            Today
          </button>
        </svelte:fragment>
      </CalendarMonth>
    </div>
  {/if}
</div>

<style>
  .dp-root {
    position: relative;
    display: inline-flex;
    flex-direction: column;
    gap: 2px;
  }

  /* ── Trigger ─────────────────────────────────────────────────────────────── */

  .dp-trigger {
    position: relative;
    display: flex;
    align-items: center;
  }

  .dp-trigger input[type="text"] {
    padding: 7px 10px;
    padding-right: 36px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    color: #1e293b;
    background: #fff;
    width: 100%;
    min-width: 140px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .dp-trigger input[type="text"]:focus {
    outline: none;
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.15);
  }

  .dp-trigger input[type="text"].dp-invalid {
    border-color: #ef4444;
    box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.12);
  }

  .dp-icon {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    padding: 2px 4px;
    color: #64748b;
    line-height: 1;
    border-radius: 4px;
    transition: color 0.15s;
  }

  .dp-icon:hover {
    color: #1e293b;
  }

  .dp-icon:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  .dp-clear {
    position: absolute;
    right: 28px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.75rem;
    color: #94a3b8;
    padding: 2px 4px;
    border-radius: 3px;
    line-height: 1;
    transition: color 0.15s;
  }

  .dp-clear:hover {
    color: #ef4444;
  }

  .dp-clear:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  /* ── Helper text ─────────────────────────────────────────────────────────── */

  .dp-helper {
    font-size: 0.75rem;
    color: #ef4444;
    margin-top: 2px;
    display: block;
  }

  /* ── Popover ─────────────────────────────────────────────────────────────── */

  .dp-popover {
    position: fixed;
    z-index: 300;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 12px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
    padding: 8px;
    display: inline-block;
  }

  /* ── Today button ────────────────────────────────────────────────────────── */

  .dp-today {
    display: block;
    width: 100%;
    margin-top: 4px;
    padding: 5px 8px;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 0.8rem;
    font-family: inherit;
    color: #2563eb;
    cursor: pointer;
    text-align: center;
    transition: background 0.15s;
  }

  .dp-today:hover:not(:disabled) {
    background: #e2e8f0;
  }

  .dp-today:disabled {
    color: #cbd5e1;
    cursor: not-allowed;
  }

  .dp-today:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }
</style>
