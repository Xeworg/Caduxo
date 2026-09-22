<script lang="ts">
    import { UNCATEGORIZED_SENTINEL } from "../../lib/categories.js";
    import { createCategory, type CategoryResponse } from "../../lib/products.js";
    import { tick } from "svelte";
    import { LL } from "../../i18n/i18n-svelte.js";
    import { humanizeError } from "../../lib/errors.js";
    import { placePopover } from "../../lib/popoverPlacement.js";
    import Button from "../ui/Button.svelte";
    import Icon from "../ui/Icon.svelte";

    // ─── Props ──────────────────────────────────────────────────────────────────

    /**
     * Bound value: array of selected category ids.
     * Empty array = no categories selected.
     * May contain `UNCATEGORIZED_SENTINEL` when `includeUncategorized` is true.
     */
    export let value: string[] = [];

    /**
     * Initial category list. Used for client-side filtering.
     * When `onSearch` is provided, this is the "initial" pool shown on open.
     */
    export let categories: CategoryResponse[] = [];

    /** Placeholder text for the internal search input. */
    export let placeholder = "";

    /**
     * When true, an "Uncategorized" pseudo-row appears in the result list.
     * Selecting it pushes/pops `UNCATEGORIZED_SENTINEL` into `value`.
     */
    export let includeUncategorized: boolean = false;

    /**
     * Optional async search function. When provided, the picker uses it instead of
     * client-side filtering on `categories`. Receives the current query string and
     * returns a promise resolving to a `CategoryResponse[]`.
     *
     * Usage:
     * ```
     * <CategoryPicker bind:value onSearch={listCategoriesSearch} />
     * ```
     */
    export let onSearch: ((query: string) => Promise<CategoryResponse[]>) | undefined = undefined;

    /**
     * Dispatched when the user clicks "Create" on the inline-create row.
     * The event detail is the newly created `CategoryResponse`.
     */
    import { createEventDispatcher } from "svelte";
    const dispatch = createEventDispatcher<{ create: CategoryResponse }>();

    // ─── Local state ─────────────────────────────────────────────────────────────

    /** Whether the popover is open. */
    let isOpen = false;

    /** Current search/filter query string. */
    let query = "";

    /** Active descendant index within the result list (for WAI-ARIA). */
    let activeIndex = -1;

    /** Whether an inline create is in progress. */
    let creating = false;

    /** Error message from inline create. */
    let createError = "";

    /** Whether the async search is in flight. */
    let searching = false;

    /** Results to display in the popover (may come from onSearch or local filter). */
    let results: CategoryResponse[] = [];

    /**
     * Race-suppression flag: set by `openPopover` when invoked via the input's
     * focus event. The next click on the trigger (which fires synchronously
     * after the focus event) would otherwise toggle the freshly opened popover
     * shut. `togglePopover` consumes the flag and bails out.
     */
    let justOpenedByFocus = false;

    /** DOM refs */
    let triggerEl: HTMLDivElement;
    let popoverEl: HTMLDivElement;
    let inputEl: HTMLInputElement;

    // ─── Derived state ───────────────────────────────────────────────────────────

    /** True when `UNCATEGORIZED_SENTINEL` is in the current selection. */
    $: hasUncategorized = value.includes(UNCATEGORIZED_SENTINEL);

    /** Category names for the selected chips (look up from `categories` prop). */
    $: selectedChips = value
        .filter((id) => id !== UNCATEGORIZED_SENTINEL)
        .map((id) => {
            const cat = categories.find((c) => c.id === id);
            return cat ? { id, name: cat.name } : { id, name: `?${id.slice(0, 4)}` };
        });

    /**
     * Whether the query exactly matches an existing category name.
     * Partial matches should still offer inline create: typing "Bate" while
     * "Bateria" exists must allow creating the distinct "Bate" category.
     */
    $: hasExactMatch = results.some(
        (c) => c.name.trim().toLowerCase() === query.trim().toLowerCase(),
    );

    // ─── Popover open / close ───────────────────────────────────────────────────

    async function openPopover() {
        if (isOpen) {
            // Idempotent re-open keeps focus state consistent without re-running
            // the initial fetch (which would clobber an in-flight typing state).
            await tick();
            inputEl?.focus();
            return;
        }
        isOpen = true;
        // Mark the open as focus-driven so the click that bubbles from the
        // input on the same gesture doesn't immediately toggle it shut.
        justOpenedByFocus = true;
        activeIndex = -1;
        // Reset results: if onSearch, fetch all; otherwise use local categories.
        if (onSearch) {
            searching = true;
            try {
                results = await onSearch("");
            } finally {
                searching = false;
            }
        } else {
            results = categories.filter((c) => c.is_active);
        }
        await tick();
        inputEl?.focus();
        requestAnimationFrame(() => positionPopover());
    }

    function closePopover() {
        isOpen = false;
        query = "";
        createError = "";
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

    // ─── Search ─────────────────────────────────────────────────────────────────

    let searchTimer: ReturnType<typeof setTimeout> | null = null;

    async function onQueryInput() {
        activeIndex = -1;
        if (searchTimer) clearTimeout(searchTimer);

        const q = query.trim();

        if (!q) {
            // Reset to full list
            if (onSearch) {
                searching = true;
                try {
                    results = await onSearch("");
                } finally {
                    searching = false;
                }
            } else {
                results = categories.filter((c) => c.is_active);
            }
            return;
        }

        searchTimer = setTimeout(async () => {
            if (onSearch) {
                searching = true;
                try {
                    results = await onSearch(q);
                } finally {
                    searching = false;
                }
            } else {
                const lower = q.toLowerCase();
                // Prefix-first, then substring fallback
                const prefix = results.filter((c) =>
                    c.name.toLowerCase().startsWith(lower),
                );
                const substring = results.filter(
                    (c) =>
                        !prefix.includes(c) &&
                        c.name.toLowerCase().includes(lower),
                );
                results = [...prefix, ...substring];
            }
        }, 150);
    }

    // ─── Selection ───────────────────────────────────────────────────────────────

    function isSelected(id: string): boolean {
        return value.includes(id);
    }

    function toggle(id: string) {
        if (value.includes(id)) {
            value = value.filter((v) => v !== id);
        } else {
            value = [...value, id];
        }
    }

    function removeChip(id: string) {
        value = value.filter((v) => v !== id);
    }

    function clearAll() {
        value = [];
    }

    // ─── Inline create ───────────────────────────────────────────────────────────

    async function submitInlineCreate() {
        const name = query.trim();
        if (!name) return;
        creating = true;
        createError = "";
        try {
            const created = await createCategory({ name });
            // Auto-select the newly created category
            if (!value.includes(created.id)) {
                value = [...value, created.id];
            }
            dispatch("create", created);
            // Close popover and reset
            closePopover();
        } catch (e: unknown) {
            const msg = humanizeError(e);
            if (/duplicate|case|already|exists/i.test(msg)) {
                createError = $LL.categoryPicker.alreadyExists({ name });
            } else {
                createError = msg;
            }
        } finally {
            creating = false;
        }
    }

    // ─── Positioning ─────────────────────────────────────────────────────────────

    /**
     * Apply the shared popover placement helper. The CategoryPicker uses
     * a fixed 300px width (its themed chip-row design) and the CSS
     * `max-height: 320px` value, so the helper is called with explicit
     * `maxHeight` / `width` numbers. See `src/lib/popoverPlacement.ts`
     * for the placement rules (prefer below when the measured content
     * fits, clamp to viewport, measure every call so the position
     * tracks live rendered content such as the inline create row).
     */
    function positionPopover() {
        if (!triggerEl || !popoverEl) return;
        placePopover({
            trigger: triggerEl,
            popover: popoverEl,
            maxHeight: 320,
            width: 300,
        });
    }

    // ─── Keyboard ergonomics ────────────────────────────────────────────────────

    function onTriggerKeydown(e: KeyboardEvent) {
        if (e.key === "ArrowDown" && !isOpen) {
            e.preventDefault();
            openPopover();
        } else if (e.key === "Backspace" && !query && value.length > 0) {
            // Remove last chip when backspacing in the empty input
            e.preventDefault();
            value = value.slice(0, -1);
        }
    }

    function onInputKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.stopPropagation();
            closePopover();
        } else if (e.key === "ArrowDown") {
            e.preventDefault();
            e.stopPropagation();
            moveActive(1);
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            e.stopPropagation();
            moveActive(-1);
        } else if (e.key === "Enter") {
            e.preventDefault();
            e.stopPropagation();
            commitActive();
        }
    }

    /** Total count of navigable items in the list (uncategorized + results + create). */
    function totalItems(): number {
        let n = includeUncategorized ? 1 : 0;
        n += results.length;
        if (!hasExactMatch && query.trim()) n += 1; // create row
        return n;
    }

    /** Convert an active index to a "logical" item: 0=uncategorized, 1..results.length=results[i-1], last=create */
    function moveActive(delta: number) {
        const total = totalItems();
        if (total === 0) return;
        activeIndex = ((activeIndex + delta) + total) % total;
    }

    function commitActive() {
        if (activeIndex < 0) return;
        const uncOffset = includeUncategorized ? 1 : 0;
        if (includeUncategorized && activeIndex === 0) {
            toggle(UNCATEGORIZED_SENTINEL);
            closePopover();
            return;
        }
        const resultIdx = activeIndex - uncOffset;
        if (resultIdx >= 0 && resultIdx < results.length) {
            toggle(results[resultIdx].id);
            closePopover();
        } else if (resultIdx === results.length && !hasExactMatch && query.trim()) {
            submitInlineCreate();
        }
    }

    // ─── Outside-click ───────────────────────────────────────────────────────────

    function onDocMousedown(e: MouseEvent) {
        if (
            !isOpen ||
            !popoverEl ||
            !triggerEl
        )
            return;
        const target = e.target as Node;
        if (popoverEl.contains(target) || triggerEl.contains(target)) return;
        closePopover();
    }

    function onScroll() {
        if (isOpen) requestAnimationFrame(() => positionPopover());
    }

    // Throttle resize/scroll repositioning so a single resize gesture
    // does not flood the helper with measurements. The previous
    // implementation repositioned on every scroll event with no
    // throttling, which is fine for `scroll` (the browser coalesces)
    // but `resize` fires once per layout change in rapid succession.
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    function onResize() {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
            if (isOpen) requestAnimationFrame(() => positionPopover());
        }, 80);
    }

    import { onMount, onDestroy } from "svelte";
    onMount(() => {
        document.addEventListener("mousedown", onDocMousedown, true);
        window.addEventListener("scroll", onScroll, true);
        window.addEventListener("resize", onResize);
    });
    onDestroy(() => {
        document.removeEventListener("mousedown", onDocMousedown, true);
        window.removeEventListener("scroll", onScroll, true);
        window.removeEventListener("resize", onResize);
        clearTimeout(resizeTimer);
    });

    // ─── Reactive reposition on content changes ──────────────────────────────────

    /**
     * The popover must refresh its placement whenever its content
     * changes (search results updated, inline create row appeared /
     * disappeared, loading state toggled). The previous fix only
     * repositioned on open + scroll, which left the popover anchored
     * to a stale height after the user typed and the create row
     * appeared — the popover then overflowed the viewport edge or
     * covered the trigger input.
     *
     * The reactive block reads the content inputs Svelte tracks so it
     * re-runs when any of them change, and it short-circuits when the
     * popover is closed. `tick()` waits for Svelte to apply the DOM
     * update; `requestAnimationFrame` waits for the next paint so the
     * helper measures the live rendered size (not a 0px placeholder
     * right after `isOpen` flips).
     */
    $: if (isOpen) {
        // Touch the inputs we want to react to.
        void results;
        void searching;
        void creating;
        void createError;
        void query;
        tick().then(() => {
            if (isOpen) requestAnimationFrame(() => positionPopover());
        });
    }

    // ─── Popover item click handlers (index-based) ─────────────────────────────────

    function onItemClick(index: number) {
        activeIndex = index;
        commitActive();
    }
</script>

<div class="cp-root" class:cp-open={isOpen}>
    <!-- Chip row + trigger -->
    <div class="cp-field" bind:this={triggerEl}>

<!-- Selected chips -->
            {#if selectedChips.length > 0 || hasUncategorized}
                <div class="cp-chips" role="list" aria-label={$LL.categoryPicker.selectedCategories()}>
                    {#if hasUncategorized}
                        <span class="cp-chip cp-chip--uncat" role="listitem">
                        {$LL.categoryPicker.uncategorized()}
                        <button
                            type="button"
                            class="cp-chip-remove"
                            aria-label={$LL.categoryPicker.removeCategory({ name: $LL.categoryPicker.uncategorized() })}
                            on:click|stopPropagation={() => removeChip(UNCATEGORIZED_SENTINEL)}
                        >
                            <Icon name="x-mark" size="xs" />
                        </button>
                    </span>
                {/if}
                {#each selectedChips as chip (chip.id)}
                    <span class="cp-chip" role="listitem">
                        {chip.name}
                        <button
                            type="button"
                            class="cp-chip-remove"
                            aria-label={$LL.categoryPicker.removeCategory({ name: chip.name })}
                            on:click|stopPropagation={() => removeChip(chip.id)}
                        >
                            <Icon name="x-mark" size="xs" />
                        </button>
                    </span>
                {/each}
            </div>
            <button
                type="button"
                class="cp-clear-all"
                aria-label={$LL.categoryPicker.clearAllAria()}
                on:click|stopPropagation={clearAll}
            >
                {$LL.categoryPicker.clearAll()}
            </button>
        {/if}

        <!-- Search trigger input -->
        <!-- svelte-ignore a11y-role-supports-aria-props -->
        <div
            class="cp-trigger"
            role="combobox"
            aria-haspopup="listbox"
            aria-expanded={isOpen}
            aria-controls={isOpen ? "cp-listbox" : undefined}
            tabindex="0"
            on:click={togglePopover}
            on:keydown={onTriggerKeydown}
        >
            <svg
                class="cp-search-icon"
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                aria-hidden="true"
            >
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
                bind:this={inputEl}
                type="text"
                bind:value={query}
                placeholder={placeholder || $LL.categoryPicker.placeholder()}
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                spellcheck="false"
                aria-label={$LL.categoryPicker.searchAria()}
                aria-autocomplete="list"
                aria-controls={isOpen ? "cp-listbox" : undefined}
                aria-activedescendant={
                    activeIndex >= 0
                        ? `cp-option-${activeIndex}`
                        : undefined
                }
                role="searchbox"
                on:focus={openPopover}
                on:input={onQueryInput}
                on:keydown={onInputKeydown}
            />
            {#if isOpen}
                <Button
                    variant="ghost"
                    size="sm"
                    aria-label={$LL.common.close()}
                    onclick={(e) => {
                        // Stop the click from bubbling up to the
                        // `.cp-trigger` div, which carries
                        // `on:click={togglePopover}`. Without this guard
                        // the close handler runs first (closing the
                        // popover) and then the bubble reaches the
                        // trigger, which sees the closed state and
                        // re-opens it — the user clicks X and the
                        // popover stays open.
                        e.stopPropagation();
                        closePopover();
                    }}
                    id="cp-close-icon"
                >
                    {#snippet iconStart()}
                        <Icon name="x-mark" size="sm" />
                    {/snippet}
                </Button>
            {/if}
        </div>
    </div>

    <!-- Popover -->
    {#if isOpen}
        <div
            bind:this={popoverEl}
            id="cp-listbox"
            class="cp-popover dropdown dropdown-content"
            role="listbox"
            aria-label={$LL.categoryPicker.results()}
            aria-multiselectable="true"
        >
            <!-- Loading state -->
            {#if searching}
                <div class="cp-status">{$LL.categoryPicker.searching()}</div>
            {:else if results.length === 0 && !query.trim() && !includeUncategorized}
                <div class="cp-status">{$LL.categoryPicker.noCategories()}</div>
            {:else}
                <!-- Uncategorized pseudo-row -->
                {#if includeUncategorized}
                    <!-- svelte-ignore a11y-click-events-have-key-events -->
                    <div
                        class="cp-option cp-option--uncat"
                        class:cp-option--active={activeIndex === 0}
                        role="option"
                        aria-selected={hasUncategorized}
                        id="cp-option-0"
                        tabindex="0"
                        on:mousedown={(e) => e.preventDefault()}
                        on:click={() => onItemClick(0)}
                        on:mouseenter={() => (activeIndex = 0)}
                    >
                        <span class="cp-option-name">{$LL.categoryPicker.uncategorized()}</span>
                        {#if hasUncategorized}
                            <span class="cp-check" aria-hidden="true">
                                <Icon name="check" size="sm" />
                            </span>
                        {/if}
                    </div>
                {/if}

                <!-- Category results -->
                {#each results as cat, i (cat.id)}
                    {@const idx = (includeUncategorized ? 1 : 0) + i}
                    <!-- svelte-ignore a11y-click-events-have-key-events -->
                    <div
                        class="cp-option"
                        class:cp-option--active={activeIndex === idx}
                        role="option"
                        aria-selected={isSelected(cat.id)}
                        id="cp-option-{idx}"
                        tabindex="0"
                        on:mousedown={(e) => e.preventDefault()}
                        on:click={() => onItemClick(idx)}
                        on:mouseenter={() => (activeIndex = idx)}
                    >
                        <span class="cp-option-name">{cat.name}</span>
                        {#if isSelected(cat.id)}
                            <span class="cp-check" aria-hidden="true">
                                <Icon name="check" size="sm" />
                            </span>
                        {/if}
                    </div>
                {/each}

                <!-- Inline create row -->
                {#if query.trim() && !hasExactMatch}
                    {@const createIdx = (includeUncategorized ? 1 : 0) + results.length}
                    <!-- svelte-ignore a11y-click-events-have-key-events -->
                    <div
                        class="cp-option cp-option--create"
                        class:cp-option--active={activeIndex === createIdx}
                        role="option"
                        aria-selected="false"
                        id="cp-option-{createIdx}"
                        tabindex="0"
                        on:mousedown={(e) => e.preventDefault()}
                        on:click={submitInlineCreate}
                        on:mouseenter={() => (activeIndex = createIdx)}
                    >
                        {#if creating}
                            <span class="cp-creating">{$LL.categoryPicker.creating()}</span>
                        {:else}
                            <span>{$LL.categoryPicker.createOption({ name: query.trim() })}</span>
                        {/if}
                    </div>
                {/if}

                {#if createError}
                    <div class="cp-create-error" role="alert">{createError}</div>
                {/if}
            {/if}
        </div>
    {/if}
</div>

<style>
    /* ── Root ────────────────────────────────────────────────────────────────── */

    .cp-root {
        position: relative;
        font-family: inherit;
        font-size: 0.9rem;
    }

    /* ── Chip row ──────────────────────────────────────────────────────────────── */

    .cp-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
        margin-bottom: 6px;
    }

    .cp-chip {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        padding: 2px 8px;
        background: color-mix(in oklch, var(--color-primary) 12%, transparent);
        color: var(--color-primary);
        border-radius: 9999px;
        font-size: 0.8rem;
        font-weight: 500;
        border: 1px solid color-mix(in oklch, var(--color-primary) 20%, transparent);
        line-height: 1.4;
    }

    .cp-chip--uncat {
        background: var(--color-base-200);
        color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
        border-color: var(--color-base-300);
    }

    .cp-chip-remove {
        background: none;
        border: none;
        cursor: pointer;
        padding: 0 0 0 2px;
        color: inherit;
        opacity: 0.7;
        font-size: 0.7rem;
        line-height: 1;
        border-radius: 3px;
        transition: opacity 0.15s;
        display: inline-flex;
        align-items: center;
    }

    .cp-chip-remove:hover {
        opacity: 1;
    }

    .cp-clear-all {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 0.75rem;
        color: var(--color-error);
        padding: 0;
        margin-bottom: 4px;
        text-decoration: underline;
        font-family: inherit;
    }

    .cp-clear-all:focus-visible {
        outline: 2px solid var(--color-primary);
        outline-offset: 1px;
        border-radius: 2px;
    }

    /* ── Trigger ───────────────────────────────────────────────────────────────── */

    .cp-trigger {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 7px 10px;
        border: 1px solid var(--color-base-300);
        border-radius: 8px;
        background: var(--color-base-100);
        cursor: text;
        transition: border-color 0.15s, box-shadow 0.15s;
    }

    .cp-trigger:focus-within,
    .cp-root.cp-open .cp-trigger {
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-primary) 15%, transparent);
        outline: none;
    }

    .cp-search-icon {
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
        flex-shrink: 0;
    }

    .cp-trigger input[type="text"] {
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

    .cp-trigger input[type="text"]::placeholder {
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    }

    /* The close icon is now a Button primitive (variant="ghost" size="sm");
       the muted gray hover behaviour it used to provide is inherited from
       DaisyUI's btn-ghost + the surrounding text colour. No local CSS rule
       is needed. */

    /* ── Popover ──────────────────────────────────────────────────────────────── */

    /* DaisyUI `dropdown dropdown-content` classes are also applied in markup
       so the popover inherits DaisyUI's `border-radius` / shadow contract,
       but the hand-rolled `position: fixed` + manual top/left computed by
       the shared `placePopover()` helper in `src/lib/popoverPlacement.ts`
       is what actually positions the element. The DaisyUI anchor pattern
       is NOT used here because the popover is anchored to the trigger's
       bounding rect via JS — the visual contract is the only thing
       borrowed. */
    .cp-popover {
        position: fixed;
        z-index: 500;
        width: 300px;
        max-height: 320px;
        overflow-y: auto;
        background: var(--color-base-100);
        border: 1px solid var(--color-base-200);
        border-radius: 10px;
        box-shadow: 0 8px 24px color-mix(in oklch, var(--color-base-content) 13%, transparent);
        padding: 4px 0;
    }

    /* ── Options ──────────────────────────────────────────────────────────────── */

    .cp-option {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 12px;
        cursor: pointer;
        border-radius: 6px;
        margin: 0 4px;
        font-size: 0.88rem;
        color: var(--color-base-content);
        transition: background 0.1s;
        user-select: none;
    }

    .cp-option:hover,
    .cp-option--active {
        background: color-mix(in oklch, var(--color-primary) 8%, transparent);
    }

    .cp-option--uncat {
        color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
        font-style: italic;
    }

    .cp-option--create {
        color: var(--color-primary);
        border-top: 1px solid var(--color-base-200);
        margin-top: 4px;
        padding-top: 10px;
    }

    .cp-option--create:hover,
    .cp-option--create.cp-option--active {
        background: color-mix(in oklch, var(--color-primary) 8%, transparent);
        color: color-mix(in oklch, var(--color-primary) 80%, black);
    }

    .cp-option-name {
        flex: 1;
    }

    .cp-check {
        color: var(--color-primary);
        font-weight: 600;
        font-size: 0.85rem;
        display: inline-flex;
        align-items: center;
    }

    .cp-creating {
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
        font-style: italic;
    }

    /* ── Status / error ─────────────────────────────────────────────────────── */

    .cp-status {
        padding: 12px;
        text-align: center;
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
        font-size: 0.85rem;
    }

    .cp-create-error {
        padding: 6px 12px;
        color: var(--color-error);
        font-size: 0.78rem;
        text-align: center;
    }
</style>
