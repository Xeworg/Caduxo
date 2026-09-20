<!--
  Listbox.svelte — closed-choice themed Svelte popover/listbox primitive
  (caduxo-daisyui-redesign follow-up to PR 8a.1; visual correction that
  replaces native `<select>`-based selects on ReportsPage.svelte and the
  sibling product-detail screen).

  Renders an Svelte-rendered popover for the option list instead of a
  native `<select>` element. Native selects on WebKit / Chromium leak
  OS-styled chrome into the visible options (system popup, OS font,
  unthemed), which cannot be themed via the DaisyUI token set. This
  primitive replaces that pattern for single-value closed-choice fields
  where the user can ONLY pick from the supplied options (no free
  typing).

  Keyboard ergonomics (mirrors Combobox / CategoryPicker):
    - click / focus / Enter / Space / ArrowDown / ArrowUp opens the popover
    - ArrowUp / ArrowDown move the active descendant
    - Enter / Space selects the active option
    - Escape closes
    - Tab closes and moves focus to the next tabbable element
    - Home / End jump to the first / last option
    - clicking outside closes
    - scrolling repositions the popover

  Accessibility (WAI-ARIA 1.2 select-only combobox / listbox pattern):
    - trigger button carries role="combobox" with aria-haspopup="listbox",
      aria-expanded, aria-controls, aria-activedescendant, aria-required,
      aria-invalid — the standard 1.2 contract for a closed-choice
      listbox invoked from a button trigger
    - popover carries role="listbox"
    - each option carries role="option" + aria-selected (and aria-disabled
      when the option is unselectable)
    - a hidden `<input type="hidden">` carries the form-submission
      value when `name` is supplied (the visible trigger is a button,
      not a form control)

  No `<select>` is rendered anywhere in the output.

  Tailwind / DaisyUI classes referenced here (for the JIT scanner):
    dropdown dropdown-content
    text-error
-->
<script lang="ts">
  import { tick } from "svelte";
  import { onMount, onDestroy } from "svelte";

  /** Normalised option shape used internally. */
  type Normalised = { value: string; label: string; disabled: boolean };

  /** Public option shape consumed by callers. */
  export interface ListboxOption {
    value: string;
    label: string;
    /** When true, the option is rendered but unselectable. */
    disabled?: boolean;
  }

  interface Props {
    /** Two-way bindable selected value. */
    value: string;
    /** Options offered in the popover. */
    options: ListboxOption[];
    /** Visual size. `sm` shrinks the trigger height; `md` is the default. */
    size?: "sm" | "md";
    /** Disables interaction; the trigger renders greyed-out. */
    disabled?: boolean;
    /** When true, the trigger renders with `select-error` chrome (red border). */
    invalid?: boolean;
    /** Plain accessible label (use when no visible `<label>` is rendered). */
    "aria-label"?: string;
    /** id of a visible `<label>` that labels the trigger. */
    "aria-labelledby"?: string;
    /** id of an element that describes the trigger (helper text, error message). */
    "aria-describedby"?: string;
    /** Native `required` marker — surfaces in form validation. */
    required?: boolean;
    /** Native `name` attribute; emitted as a hidden input for form submission. */
    name?: string;
    /** Optional id passthrough for the trigger button. */
    id?: string;
    /** Fired when the selection changes. */
    onchange?: (value: string) => void;
  }

  let {
    value = $bindable(),
    options,
    size = "md",
    disabled = false,
    invalid = false,
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
    required = false,
    name,
    id,
    onchange,
  }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────────
  const instanceId = `lb-${Math.random().toString(36).slice(2, 10)}`;
  const listboxId = `${instanceId}-listbox`;
  let isOpen = $state(false);
  let activeIndex = $state(-1);

  /** DOM refs */
  let triggerEl: HTMLButtonElement | undefined = $state();
  let popoverEl: HTMLDivElement | undefined = $state();

  // ── Derived ────────────────────────────────────────────────────────────────

  /** Normalised option list — flat shape, `disabled` defaulted to false. */
  const normalised: Normalised[] = $derived(
    options.map((o) => ({
      value: o.value,
      label: o.label,
      disabled: o.disabled ?? false,
    })),
  );

  /**
   * Index of the currently-selected option. When `value` is not in the
   * option list (e.g. an upstream filter reset the bound value), no
   * option is marked selected — the trigger renders the first option's
   * label as a fallback display so the field never looks empty.
   */
  const selectedIndex = $derived(
    normalised.findIndex((o) => o.value === (value ?? "")),
  );

  /** Label shown in the trigger. Falls back to the first option so the
   *  trigger always has visible copy. */
  const displayLabel = $derived.by(() => {
    if (selectedIndex >= 0) return normalised[selectedIndex].label;
    if (normalised.length > 0) return normalised[0].label;
    return "";
  });

  const sizeClass = $derived(size === "sm" ? "lb-sm" : "lb-md");
  const invalidClass = $derived(invalid ? "lb-invalid" : "");

  // ── Popover open / close ──────────────────────────────────────────────────

  async function openPopover() {
    if (isOpen || disabled) {
      await tick();
      triggerEl?.focus();
      return;
    }
    isOpen = true;
    // Open on the currently-selected option so ArrowDown moves to the
    // next entry, matching native <select> behavior.
    activeIndex = selectedIndex >= 0 ? selectedIndex : 0;
    await tick();
    requestAnimationFrame(() => positionPopover());
  }

  function closePopover() {
    if (!isOpen) return;
    isOpen = false;
    activeIndex = -1;
  }

  function togglePopover() {
    if (disabled) return;
    if (isOpen) closePopover();
    else openPopover();
  }

  // ── Selection ──────────────────────────────────────────────────────────────

  function selectOption(option: Normalised) {
    if (option.disabled) return;
    value = option.value;
    onchange?.(option.value);
    closePopover();
    // Return focus to the trigger so keyboard users land back where
    // they came from after a selection.
    triggerEl?.focus();
  }

  // ── Trigger handlers ──────────────────────────────────────────────────────

  function onTriggerClick() {
    togglePopover();
  }

  function onTriggerKeydown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === "Enter" || e.key === " " || e.key === "Spacebar") {
      e.preventDefault();
      togglePopover();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (!isOpen) {
        openPopover();
      } else {
        moveActive(1);
      }
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (!isOpen) {
        openPopover();
      } else {
        moveActive(-1);
      }
      return;
    }
    if (e.key === "Home") {
      if (!isOpen) return;
      e.preventDefault();
      activeIndex = 0;
      return;
    }
    if (e.key === "End") {
      if (!isOpen) return;
      e.preventDefault();
      activeIndex = normalised.length - 1;
      return;
    }
    if (e.key === "Escape") {
      if (isOpen) {
        e.preventDefault();
        closePopover();
      }
      return;
    }
    if (e.key === "Tab") {
      // Tab should close the popover and let focus move on naturally.
      if (isOpen) closePopover();
    }
  }

  function moveActive(delta: number) {
    const total = normalised.length;
    if (total === 0) return;
    // Find the next non-disabled option in the requested direction.
    let next = activeIndex;
    for (let i = 0; i < total; i++) {
      next = ((next + delta) % total + total) % total;
      if (!normalised[next].disabled) {
        activeIndex = next;
        return;
      }
    }
    // All options are disabled — leave activeIndex unchanged.
  }

  function onItemClick(index: number) {
    selectOption(normalised[index]);
  }

  function onPopoverKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      moveActive(1);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      moveActive(-1);
      return;
    }
    if (e.key === "Home") {
      e.preventDefault();
      activeIndex = 0;
      return;
    }
    if (e.key === "End") {
      e.preventDefault();
      activeIndex = normalised.length - 1;
      return;
    }
    if (e.key === "Enter" || e.key === " " || e.key === "Spacebar") {
      if (activeIndex >= 0 && activeIndex < normalised.length) {
        e.preventDefault();
        selectOption(normalised[activeIndex]);
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      closePopover();
      triggerEl?.focus();
      return;
    }
    if (e.key === "Tab") {
      closePopover();
    }
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

<!-- Hidden form input — surfaced when `name` is supplied so the
     primitive can stand in for a native <select> inside a <form>. -->
{#if name}
  <input type="hidden" {name} {value} />
{/if}

<!--
  WAI-ARIA 1.2 select-only combobox pattern: the trigger carries
  `role="combobox"` with `aria-haspopup="listbox"` so the standard
  ARIA combobox attribute contract (aria-expanded, aria-controls,
  aria-activedescendant, aria-required, aria-invalid) is valid.
  The popover is `role="listbox"`, options are `role="option"`.
-->
<button
  {id}
  type="button"
  bind:this={triggerEl}
  class="lb-trigger {sizeClass} {invalidClass}"
  role="combobox"
  aria-haspopup="listbox"
  aria-expanded={isOpen}
  aria-controls={isOpen ? listboxId : undefined}
  aria-label={ariaLabel}
  aria-labelledby={ariaLabelledBy}
  aria-describedby={ariaDescribedBy}
  aria-required={required ? "true" : undefined}
  aria-invalid={invalid ? "true" : undefined}
  aria-activedescendant={
    isOpen && activeIndex >= 0 ? `${instanceId}-option-${activeIndex}` : undefined
  }
  {disabled}
  onclick={onTriggerClick}
  onkeydown={onTriggerKeydown}
>
  <span class="lb-label">{displayLabel}</span>
  <svg
    class="lb-caret"
    class:lb-caret--open={isOpen}
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
</button>

{#if isOpen}
  <div
    bind:this={popoverEl}
    id={listboxId}
    class="lb-popover dropdown dropdown-content"
    role="listbox"
    aria-label={ariaLabel}
    aria-labelledby={ariaLabelledBy}
    tabindex="-1"
    onkeydown={onPopoverKeydown}
  >
    {#each normalised as opt, i (`${opt.value}`)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="lb-option"
        class:lb-option--active={activeIndex === i}
        class:lb-option--selected={selectedIndex === i}
        class:lb-option--disabled={opt.disabled}
        role="option"
        aria-selected={selectedIndex === i}
        aria-disabled={opt.disabled ? "true" : undefined}
        id={`${instanceId}-option-${i}`}
        tabindex="-1"
        onmousedown={(e) => e.preventDefault()}
        onclick={() => onItemClick(i)}
        onmouseenter={() => {
          if (!opt.disabled) activeIndex = i;
        }}
      >
        <span class="lb-option-label">{opt.label}</span>
        {#if selectedIndex === i}
          <svg
            class="lb-option-check"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            aria-hidden="true"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  /* Trigger ───────────────────────────────────────────────────────────────── */

  .lb-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 7px 10px;
    border: 1px solid var(--color-base-300);
    border-radius: 8px;
    background: var(--color-base-100);
    transition: border-color 0.15s, box-shadow 0.15s;
    width: 100%;
    box-sizing: border-box;
    font-family: inherit;
    font-size: 0.9rem;
    color: var(--color-base-content);
    cursor: pointer;
    text-align: left;
  }

  .lb-trigger:hover:not(:disabled) {
    border-color: color-mix(in oklch, var(--color-base-300) 80%, var(--color-primary) 20%);
  }

  .lb-trigger:focus,
  .lb-trigger:focus-visible {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-primary) 15%, transparent);
    outline: none;
  }

  .lb-trigger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: color-mix(in oklch, var(--color-base-200) 60%, transparent);
  }

  .lb-trigger.lb-invalid {
    border-color: var(--color-error);
  }

  .lb-trigger.lb-invalid:focus {
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-error) 20%, transparent);
  }

  /* Sizes ─────────────────────────────────────────────────────────────────── */

  .lb-trigger.lb-sm {
    padding: 4px 8px;
    font-size: 0.82rem;
    min-height: 30px;
  }

  .lb-trigger.lb-md {
    padding: 7px 10px;
    font-size: 0.9rem;
    min-height: 38px;
  }

  /* Label + caret ────────────────────────────────────────────────────────── */

  .lb-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lb-caret {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    flex-shrink: 0;
    transition: transform 0.15s;
  }

  .lb-caret--open {
    transform: rotate(180deg);
  }

  /* Popover ───────────────────────────────────────────────────────────────── */

  .lb-popover {
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

  /* Options ───────────────────────────────────────────────────────────────── */

  .lb-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    cursor: pointer;
    border-radius: 6px;
    margin: 0 4px;
    font-size: 0.88rem;
    color: var(--color-base-content);
    transition: background 0.1s;
    user-select: none;
  }

  .lb-option:hover,
  .lb-option--active {
    background: color-mix(in oklch, var(--color-primary) 8%, transparent);
  }

  .lb-option--selected {
    font-weight: 600;
  }

  .lb-option--disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .lb-option--disabled:hover,
  .lb-option--disabled.lb-option--active {
    background: transparent;
  }

  .lb-option-label {
    flex: 1;
  }

  .lb-option-check {
    color: var(--color-primary);
    flex-shrink: 0;
  }
</style>
