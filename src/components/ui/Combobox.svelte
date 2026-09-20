<!--
  Combobox.svelte — single-value combobox primitive (caduxo-daisyui-redesign
  follow-up to PR 8a; visual correction that replaces <datalist>-based
  autocomplete on ProductForm.svelte).

  Renders an Svelte-rendered popover for suggestions instead of a native
  `<datalist>`. The native <datalist> listbox on WebKit / Chromium leaks
  OS-styled chrome into the visible suggestions (black popup, OS font,
  unthemed), which cannot be themed via the DaisyUI token set. This
  primitive replaces that pattern for single-value free-text-allowed
  fields where the user can type any text but is offered a list of
  suggestions.

  Two option shapes are supported:
    - string[]                                              → simple label
    - { value: string; id?: string; label?: string }[]      → typed; `id`
      is optional metadata the consumer receives via `onselect`

  Keyboard ergonomics (mirrors CategoryPicker):
    - focus / click opens the popover
    - typing filters the option list (case-insensitive substring)
    - ArrowUp / ArrowDown move the active descendant
    - Enter selects the active suggestion
    - Escape closes
    - clicking outside closes
    - scrolling repositions the popover

  Accessibility (WAI-ARIA combobox 1.2 pattern):
    - trigger div carries role="combobox", aria-expanded, aria-controls
    - input carries aria-autocomplete="list", aria-controls,
      aria-activedescendant, role="searchbox"
    - popover carries role="listbox"
    - each option carries role="option" + aria-selected
    - the fieldset / legend wraps the whole control so screen readers
      announce the visible label

  No <datalist> is rendered anywhere in the output.

  Tailwind / DaisyUI classes referenced here (for the JIT scanner):
    dropdown dropdown-content
    fieldset fieldset-legend
    label
    text-error
-->
<script lang="ts">
  import { tick } from "svelte";
  import { onMount, onDestroy } from "svelte";

  /** Normalised option shape used internally. */
  type Normalised = { value: string; id?: string; label?: string };

  /** Public option shape — string or object. */
  export type ComboboxOption = string | { value: string; id?: string; label?: string };

  function normalize(o: ComboboxOption): Normalised {
    return typeof o === "string" ? { value: o, label: o } : o;
  }

  interface Props {
    /** Two-way bindable text value. */
    value: string;
    /** Suggestions offered when the popover is open. */
    options: ComboboxOption[];
    /** Visible label rendered above the input via fieldset / legend. */
    label: string;
    /** When true, surfaces a `*` after the legend (visible marker) plus an
     *  sr-only `(required)` annotation so screen readers announce the
     *  required state. */
    required?: boolean;
    /** Placeholder text for the input. */
    placeholder?: string;
    /** Optional id passthrough for the underlying input element (form semantics). */
    id?: string;
    /** Optional name passthrough for form submission. */
    name?: string;
    /** Optional helper text rendered below the input. */
    helper?: string;
    /**
     * Optional text shown when no options match the current value. Defaults
     * to no message (the popover simply renders empty when there are no
     * matches).
     */
    emptyText?: string;
    /** Fired on every keystroke (native input event). */
    oninput?: (value: string) => void;
    /** Fired when the input loses focus (native blur event). */
    onblur?: (event: FocusEvent) => void;
    /**
     * Fired when the user explicitly picks a known option via mouse click
     * or Enter on the active suggestion. The payload is the normalised
     * option object — `id` is present only when the source option supplied
     * one. NOT fired when the user simply types a value without selecting
     * a suggestion.
     */
    onselect?: (option: Normalised) => void;
  }

  let {
    value = $bindable(),
    options,
    label,
    required = false,
    placeholder = "",
    id,
    name,
    helper,
    emptyText = "",
    oninput,
    onblur,
    onselect,
  }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────────
  const instanceId = `cb-${Math.random().toString(36).slice(2, 10)}`;
  const listboxId = `${instanceId}-listbox`;
  let isOpen = $state(false);
  let activeIndex = $state(-1);
  /**
   * Race-suppression flag: set by `openPopover` when invoked via the
   * input's focus event. The click that bubbles from the input on the
   * same gesture would otherwise toggle the freshly opened popover shut.
   * `togglePopover` consumes the flag and bails out.
   */
  let justOpenedByFocus = $state(false);

  /** DOM refs */
  let triggerEl: HTMLDivElement | undefined = $state();
  let popoverEl: HTMLDivElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();

  // ── Derived ────────────────────────────────────────────────────────────────

  /**
   * Filtered results — case-insensitive substring of `value`. An empty
   * `value` shows the full list so users can pick without typing.
   */
  const results: Normalised[] = $derived.by(() => {
    const q = (value ?? "").trim().toLowerCase();
    const all = options.map(normalize);
    if (!q) return all;
    return all.filter((o) => o.value.toLowerCase().includes(q));
  });

  /** Helper text id used for `aria-describedby` on the input. */
  const helperId = $derived(
    id ? `${id}-helper` : `cb-helper-${Math.random().toString(36).slice(2, 10)}`,
  );

  // ── Popover open / close ──────────────────────────────────────────────────

  async function openPopover() {
    if (isOpen) {
      // Idempotent re-open keeps focus state consistent.
      await tick();
      inputEl?.focus();
      return;
    }
    isOpen = true;
    // Mark the open as focus-driven so the click that bubbles from the
    // input on the same gesture doesn't immediately toggle it shut.
    justOpenedByFocus = true;
    activeIndex = -1;
    await tick();
    inputEl?.focus();
    requestAnimationFrame(() => positionPopover());
  }

  function closePopover() {
    if (!isOpen) return;
    isOpen = false;
    activeIndex = -1;
  }

  function togglePopover() {
    if (justOpenedByFocus) {
      // The previous focus event opened the popover; the click that just
      // fired on the trigger is the same gesture and must not toggle it.
      justOpenedByFocus = false;
      return;
    }
    if (isOpen) closePopover();
    else openPopover();
  }

  // ── Selection ──────────────────────────────────────────────────────────────

  function selectOption(option: Normalised) {
    value = option.value;
    onselect?.(option);
    closePopover();
  }

  // ── Input handlers ────────────────────────────────────────────────────────

  function onInput(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    value = target.value;
    activeIndex = -1;
    oninput?.(target.value);
    if (!isOpen) openPopover();
  }

  function onInputBlur(e: FocusEvent) {
    // Defer the close so a click on an option is processed first. The
    // option's `onmousedown` calls `preventDefault()` to suppress focus
    // shifting to the option (which is not focusable), so blur does not
    // fire on option click — the click handler resolves the selection
    // before this setTimeout has a chance to run.
    setTimeout(() => {
      if (isOpen) closePopover();
    }, 0);
    onblur?.(e);
  }

  function onInputKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      closePopover();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      e.stopPropagation();
      if (!isOpen) openPopover();
      else moveActive(1);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      e.stopPropagation();
      if (!isOpen) openPopover();
      else moveActive(-1);
      return;
    }
    if (e.key === "Enter") {
      // Only intercept Enter while the popover is open. When closed, let
      // the native form-submit semantics propagate.
      if (isOpen) {
        e.preventDefault();
        if (activeIndex >= 0 && activeIndex < results.length) {
          selectOption(results[activeIndex]);
        }
      }
      return;
    }
  }

  function moveActive(delta: number) {
    const total = results.length;
    if (total === 0) return;
    activeIndex = ((activeIndex + delta) % total + total) % total;
  }

  function onItemClick(index: number) {
    activeIndex = index;
    selectOption(results[index]);
  }

  // ── Positioning ───────────────────────────────────────────────────────────

  function positionPopover() {
    if (!triggerEl || !popoverEl) return;
    const rect = triggerEl.getBoundingClientRect();
    const POPOVER_HEIGHT = 260;
    const POPOVER_WIDTH = Math.max(rect.width, 200);
    const spaceBelow = window.innerHeight - rect.bottom;
    const spaceAbove = rect.top;
    const placeBelow = spaceBelow >= POPOVER_HEIGHT + 8 || spaceBelow >= spaceAbove;
    popoverEl.style.top = placeBelow
      ? `${rect.bottom + 4}px`
      : `${rect.top - POPOVER_HEIGHT - 4}px`;
    const left = Math.max(8, Math.min(rect.left, window.innerWidth - POPOVER_WIDTH - 8));
    popoverEl.style.left = `${left}px`;
    popoverEl.style.position = "fixed";
    popoverEl.style.width = `${POPOVER_WIDTH}px`;
  }

  // ── Outside-click / scroll ────────────────────────────────────────────────

  function onDocMousedown(e: MouseEvent) {
    if (!isOpen || !popoverEl || !triggerEl) return;
    const target = e.target as Node;
    if (popoverEl.contains(target) || triggerEl.contains(target)) return;
    closePopover();
  }

  function onScroll() {
    if (isOpen) requestAnimationFrame(() => positionPopover());
  }

  onMount(() => {
    document.addEventListener("mousedown", onDocMousedown, true);
    window.addEventListener("scroll", onScroll, true);
  });

  onDestroy(() => {
    document.removeEventListener("mousedown", onDocMousedown, true);
    window.removeEventListener("scroll", onScroll, true);
  });
</script>

<fieldset class="fieldset cb-fieldset">
  <legend class="fieldset-legend">
    {label}
    {#if required}
      <span class="text-error" aria-hidden="true">*</span>
      <span class="sr-only">(required)</span>
    {/if}
  </legend>
  <div class="cb-root" class:cb-open={isOpen}>
    <!-- Trigger -->
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="cb-trigger"
      role="combobox"
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-controls={isOpen ? listboxId : undefined}
      bind:this={triggerEl}
      onclick={() => {
        // Focus the input preemptively when the user clicks anywhere on
        // the trigger (including the padding around the input), then
        // toggle the popover. The input's own `onfocus` also calls
        // `openPopover`, but that path is idempotent.
        inputEl?.focus();
        togglePopover();
      }}
    >
      <input
        bind:this={inputEl}
        {id}
        {name}
        type="text"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
        class="cb-input"
        {placeholder}
        value={value ?? ""}
        role="searchbox"
        aria-autocomplete="list"
        aria-controls={isOpen ? listboxId : undefined}
        aria-activedescendant={
          isOpen && activeIndex >= 0 ? `${instanceId}-option-${activeIndex}` : undefined
        }
        aria-required={required ? "true" : undefined}
        aria-describedby={helper ? helperId : undefined}
        onfocus={openPopover}
        oninput={onInput}
        onblur={onInputBlur}
        onkeydown={onInputKeydown}
      />
      <svg
        class="cb-caret"
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        aria-hidden="true"
      >
        <polyline points="6 9 12 15 18 9" />
      </svg>
    </div>

    <!-- Popover -->
    {#if isOpen}
      <div
        bind:this={popoverEl}
        id={listboxId}
        class="cb-popover dropdown dropdown-content"
        role="listbox"
      >
        {#if results.length === 0}
          {#if emptyText}
            <div class="cb-status">{emptyText}</div>
          {/if}
        {:else}
          {#each results as opt, i (`${opt.value}|${opt.id ?? ""}`)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="cb-option"
              class:cb-option--active={activeIndex === i}
              role="option"
              aria-selected={activeIndex === i}
              id={`${instanceId}-option-${i}`}
              tabindex="-1"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => onItemClick(i)}
              onmouseenter={() => (activeIndex = i)}
            >
              <span class="cb-option-label">{opt.label ?? opt.value}</span>
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
  {#if helper}
    <p class="label" id={helperId}>{helper}</p>
  {/if}
</fieldset>

<style>
  .cb-fieldset {
    width: 100%;
    min-width: 0;
  }

  .cb-root {
    position: relative;
    font-family: inherit;
    font-size: 0.9rem;
    width: 100%;
  }

  /* Trigger ───────────────────────────────────────────────────────────────── */

  .cb-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    border: 1px solid var(--color-base-300);
    border-radius: 8px;
    background: var(--color-base-100);
    transition: border-color 0.15s, box-shadow 0.15s;
    width: 100%;
    box-sizing: border-box;
  }

  .cb-trigger:focus-within,
  .cb-root.cb-open .cb-trigger {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-primary) 15%, transparent);
    outline: none;
  }

  .cb-input {
    flex: 1;
    border: none;
    outline: none;
    font-size: 0.9rem;
    font-family: inherit;
    color: var(--color-base-content);
    background: transparent;
    min-width: 0;
    padding: 0;
  }

  .cb-input::placeholder {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
  }

  .cb-caret {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    flex-shrink: 0;
  }

  /* Popover ──────────────────────────────────────────────────────────────── */

  /* DaisyUI `dropdown dropdown-content` classes are also applied in markup
     so the popover inherits DaisyUI's `border-radius` / shadow contract,
     but the hand-rolled `position: fixed` + manual top/left computed in
     `positionPopover()` stays unchanged. The visual contract is the only
     thing borrowed (mirrors the CategoryPicker approach). */
  .cb-popover {
    position: fixed;
    z-index: 500;
    max-height: 260px;
    overflow-y: auto;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-200);
    border-radius: 10px;
    box-shadow: 0 8px 24px color-mix(in oklch, var(--color-base-content) 13%, transparent);
    padding: 4px 0;
  }

  /* Options ──────────────────────────────────────────────────────────────── */

  .cb-option {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    cursor: pointer;
    border-radius: 6px;
    margin: 0 4px;
    font-size: 0.88rem;
    color: var(--color-base-content);
    transition: background 0.1s;
    user-select: none;
  }

  .cb-option:hover,
  .cb-option--active {
    background: color-mix(in oklch, var(--color-primary) 8%, transparent);
  }

  .cb-option-label {
    flex: 1;
  }

  /* Status (no matches) ─────────────────────────────────────────────────── */

  .cb-status {
    padding: 12px;
    text-align: center;
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.85rem;
  }

  /* Screen-reader-only utility — mirrors the .sr-only block in ProductForm.svelte */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
