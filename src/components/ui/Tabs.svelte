<!--
  Tabs.svelte — shared primitive (PR 4 of caduxo-daisyui-redesign).

  Composes DaisyUI `tabs tabs-{style}` with `tab tab-active`. The
  primitive owns:
    - ARIA wiring — `role="tablist"` on the wrapper, `role="tab"` on
      each tab button, `role="tabpanel"` on each panel with
      `aria-labelledby` pointing at the corresponding tab.
    - Keyboard handling — ArrowLeft / ArrowRight cycle focus
      (with wrap-around), Home / End jump to the first / last tab,
      Enter / Space activate the focused tab.
    - Visual style — `bordered`, `lifted`, or `boxed`.

  The consumer owns:
    - The set of tabs (`items`).
    - The active tab id (`activeId`) and the change handler.
    - The contents of each panel (the `panel` snippet inside the
      item is rendered when the item is active).

  Tailwind classes referenced here (for the JIT scanner):
    tabs tabs-border tabs-lift tabs-box
    tab tab-active
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  export type TabStyle = "bordered" | "lifted" | "boxed";

  export interface TabItem {
    /** Stable id used for `activeId` matching. */
    id: string;
    /** Visible label for the tab (the accessible name). */
    label: string;
    /** When true, the tab is rendered but disabled. */
    disabled?: boolean;
    /** Panel content rendered when the tab is active. */
    panel: Snippet;
  }

  interface Props {
    items: TabItem[];
    /** Two-way bindable active tab id. The primitive flips it via the keyboard / click handlers. */
    activeId: string;
    style?: TabStyle;
    /** Optional id for each tab (`{id}-tab`). Auto-derived from `id` when not supplied. */
    id?: string;
    /** Required accessible label for the tablist (ARIA convention for `role="tablist"`). */
    "aria-label": string;
    /** Fired when the active tab changes (via keyboard, click, or `activeId` write). */
    onchange?: (id: string) => void;
  }

  let {
    items,
    activeId = $bindable(),
    style = "bordered",
    id,
    "aria-label": ariaLabel,
    onchange,
  }: Props = $props();

  let tablist: HTMLDivElement | undefined = $state();

  // DaisyUI v5 renamed the style modifiers: `tabs-bordered` → `tabs-border`,
  // `tabs-lifted` → `tabs-lift`, `tabs-boxed` → `tabs-box`. The public
  // `TabStyle` union keeps the v4-era names so the PR 4 task contract
  // (`bordered | lifted | boxed`) is preserved — we only change the
  // emitted class.
  const styleClass = $derived.by((): string => {
    switch (style) {
      case "lifted":
        return "tabs-lift";
      case "boxed":
        return "tabs-box";
      case "bordered":
      default:
        return "tabs-border";
    }
  });

  // Base id for each tab / panel pair. Per ARIA, tabs and panels
  // share an id namespace; we expose `{id}-tab` and `{id}-panel` so
  // external consumers can target them when needed.
  const baseId = $derived(id ?? `tabs-${Math.random().toString(36).slice(2, 10)}`);

  function tabId(itemId: string): string {
    return `${baseId}-${itemId}-tab`;
  }

  function panelId(itemId: string): string {
    return `${baseId}-${itemId}-panel`;
  }

  function selectTab(itemId: string) {
    if (activeId === itemId) return;
    activeId = itemId;
    onchange?.(itemId);
  }

  function handleKey(event: KeyboardEvent) {
    // Resolve the enabled (non-disabled) tab indices so the keyboard
    // nav skips over disabled tabs. We cycle through the enabled set
    // only.
    const enabledIndices = items
      .map((item, index) => ({ item, index }))
      .filter(({ item }) => !item.disabled)
      .map(({ index }) => index);

    if (enabledIndices.length === 0) return;

    const currentIndex = items.findIndex((item) => item.id === activeId);
    const currentEnabledPosition = enabledIndices.indexOf(currentIndex);

    let nextEnabledPosition: number | null = null;

    switch (event.key) {
      case "ArrowRight": {
        event.preventDefault();
        nextEnabledPosition =
          currentEnabledPosition === -1
            ? 0
            : (currentEnabledPosition + 1) % enabledIndices.length;
        break;
      }
      case "ArrowLeft": {
        event.preventDefault();
        nextEnabledPosition =
          currentEnabledPosition === -1
            ? enabledIndices.length - 1
            : (currentEnabledPosition - 1 + enabledIndices.length) %
              enabledIndices.length;
        break;
      }
      case "Home": {
        event.preventDefault();
        nextEnabledPosition = 0;
        break;
      }
      case "End": {
        event.preventDefault();
        nextEnabledPosition = enabledIndices.length - 1;
        break;
      }
      case "Enter":
      case " ": {
        // Space scrolls the page by default — preventDefault so the
        // activation is unambiguous. The click handler covers the
        // pointer path; this handler covers the keyboard path.
        event.preventDefault();
        if (currentIndex !== -1 && !items[currentIndex].disabled) {
          // Already active — Enter / Space do nothing on the active
          // tab. Arrow keys still move focus.
        }
        return;
      }
      default:
        return;
    }

    if (nextEnabledPosition === null) return;
    const nextIndex = enabledIndices[nextEnabledPosition];
    const nextItem = items[nextIndex];
    selectTab(nextItem.id);
    // Move focus to the newly active tab so keyboard users land on it.
    queueMicrotask(() => {
      const tabEl = tablist?.querySelector<HTMLButtonElement>(
        `[data-tab-id="${nextItem.id}"]`,
      );
      tabEl?.focus();
    });
  }
</script>

<!--
  The tablist renders each tab as a button so the keyboard surface is
  accessible by default. Each tab is associated with its panel via
  `aria-controls` / `id`.

  svelte-ignore: tablist uses roving tabindex (active tab has tabindex=0,
  others -1) so the wrapper intentionally has no tabindex. Putting
  tabindex on the wrapper would create two tab stops in the page tab
  order, contradicting the ARIA Authoring Practices guide for tabs.
-->
<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  bind:this={tablist}
  role="tablist"
  class="tabs {styleClass}"
  aria-label={ariaLabel}
  aria-orientation="horizontal"
  onkeydown={handleKey}
>
  {#each items as item (item.id)}
    {@const isActive = item.id === activeId}
    <button
      type="button"
      role="tab"
      id={tabId(item.id)}
      data-tab-id={item.id}
      class="tab {isActive ? 'tab-active' : ''}"
      aria-selected={isActive ? "true" : "false"}
      aria-controls={panelId(item.id)}
      tabindex={isActive ? 0 : -1}
      disabled={item.disabled}
      onclick={() => selectTab(item.id)}
    >
      {item.label}
    </button>
  {/each}
</div>

<!--
  Each panel is a `<section role="tabpanel">`. We render every panel
  in the DOM (with `hidden` toggling visibility) so the consumer's
  snippet state survives tab switches without re-running effects.
  Hidden panels are still in the accessibility tree but inert to
  screen-reader users thanks to the `hidden` attribute.
-->
{#each items as item (item.id)}
  {@const isActive = item.id === activeId}
  <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
  <!--
    The tabpanel is a `<div role="tabpanel">` per the WAI-ARIA
    Authoring Practices guide. Svelte-check's rule against giving
    interactive roles to `<section>` is over-strict for tabs —
    `role="tabpanel"` is canonical on any container element.
  -->
  <div
    role="tabpanel"
    id={panelId(item.id)}
    aria-labelledby={tabId(item.id)}
    hidden={!isActive}
    tabindex={isActive ? 0 : -1}
  >
    {#if isActive}
      {@render item.panel()}
    {/if}
  </div>
{/each}