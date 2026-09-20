<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import CalendarMonth from "./CalendarMonth.svelte";
  import Button from "./ui/Button.svelte";
  import { LL } from "../i18n/i18n-svelte.js";

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
      return $LL.errors.dateUseIsoFormat();
    if (invalidReason === "range") return $LL.errors.dateYearRange();
    if (invalidReason === "required") return $LL.errors.dateFieldRequired();
    return $LL.errors.dateInvalid();
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
    <div class="dp-icon-slot">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.datePicker.ariaOpenCalendar()}
        onclick={togglePopover}
      >
        📅
      </Button>
    </div>
    {#if clearable && value !== ""}
      <div class="dp-clear-slot">
        <Button
          variant="ghost"
          size="sm"
          aria-label={$LL.datePicker.ariaClearDate()}
          onclick={onClear}
        >
          ✕
        </Button>
      </div>
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
      class="dp-popover dropdown dropdown-content"
      role="dialog"
      aria-label={$LL.datePicker.calendarDialog({ label: ariaLabel })}
    >
      <CalendarMonth
        viewYear={popoverYear}
        viewMonth={popoverMonth}
        selectedDate={value || null}
        {todayDate}
        {minDate}
        {maxDate}
        ariaLabel={$LL.datePicker.calendarDialog({ label: ariaLabel })}
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
            {$LL.datePicker.today()}
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
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    color: var(--color-base-content);
    background: var(--color-base-100);
    width: 100%;
    min-width: 140px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .dp-trigger input[type="text"]:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-primary) 15%, transparent);
  }

  .dp-trigger input[type="text"].dp-invalid {
    border-color: var(--color-error);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-error) 12%, transparent);
  }

  .dp-icon-slot {
    position: absolute;
    right: 8px;
    display: inline-flex;
    align-items: center;
    line-height: 1;
  }

  .dp-icon-slot :global(.btn) {
    padding: 2px 6px;
    min-height: auto;
    height: auto;
    font-size: 1rem;
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
  }

  .dp-clear-slot {
    position: absolute;
    right: 32px;
    display: inline-flex;
    align-items: center;
    line-height: 1;
  }

  .dp-clear-slot :global(.btn) {
    padding: 2px 4px;
    min-height: auto;
    height: auto;
    font-size: 0.75rem;
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
  }

  .dp-clear-slot :global(.btn:hover) {
    color: var(--color-error);
  }

  /* ── Helper text ─────────────────────────────────────────────────────────── */

  .dp-helper {
    font-size: 0.75rem;
    color: var(--color-error);
    margin-top: 2px;
    display: block;
  }

  /* ── Popover ─────────────────────────────────────────────────────────────── */

  /* DaisyUI `dropdown dropdown-content` classes are also applied in markup
     so the popover inherits DaisyUI's `border-radius` / shadow contract,
     but the hand-rolled `position: fixed` + manual top/left computed in
     `positionPopover()` stays unchanged. The DaisyUI anchor pattern
     (`dropdown-end`, etc.) is NOT used here because the popover is anchored
     to the trigger's bounding rect via JS rather than the DaisyUI anchor
     pattern — the visual contract is the only thing borrowed. */
  .dp-popover {
    position: fixed;
    z-index: 300;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-200);
    border-radius: 12px;
    box-shadow: 0 8px 24px color-mix(in oklch, var(--color-base-content) 14%, transparent);
    padding: 8px;
    display: inline-block;
  }

  /* ── Today button ────────────────────────────────────────────────────────── */

  .dp-today {
    display: block;
    width: 100%;
    margin-top: 4px;
    padding: 5px 8px;
    background: color-mix(in oklch, var(--color-base-200) 60%, transparent);
    border: 1px solid var(--color-base-200);
    border-radius: 6px;
    font-size: 0.8rem;
    font-family: inherit;
    color: var(--color-primary);
    cursor: pointer;
    text-align: center;
    transition: background 0.15s;
  }

  .dp-today:hover:not(:disabled) {
    background: var(--color-base-200);
  }

  .dp-today:disabled {
    color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
    cursor: not-allowed;
  }

  .dp-today:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
</style>
