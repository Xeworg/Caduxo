<script lang="ts">
    import { UNCATEGORIZED_SENTINEL } from "../../lib/categories.js";
    import { createCategory, type CategoryResponse } from "../../lib/products.js";
    import { tick } from "svelte";
    import { LL } from "../../i18n/i18n-svelte.js";

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
            const msg = String(e);
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

    function positionPopover() {
        if (!triggerEl || !popoverEl) return;
        const rect = triggerEl.getBoundingClientRect();
        const POPOVER_HEIGHT = 320;
        const POPOVER_WIDTH = 300;
        const spaceBelow = window.innerHeight - rect.bottom;
        const spaceAbove = rect.top;

        const placeBelow = spaceBelow >= POPOVER_HEIGHT + 8 || spaceBelow >= spaceAbove;

        popoverEl.style.top = placeBelow
            ? `${rect.bottom + 4}px`
            : `${rect.top - POPOVER_HEIGHT - 4}px`;
        const left = Math.max(
            8,
            Math.min(rect.left, window.innerWidth - POPOVER_WIDTH - 8),
        );
        popoverEl.style.left = `${left}px`;
        popoverEl.style.position = "fixed";
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

    import { onMount, onDestroy } from "svelte";
    onMount(() => {
        document.addEventListener("mousedown", onDocMousedown, true);
        window.addEventListener("scroll", onScroll, true);
    });
    onDestroy(() => {
        document.removeEventListener("mousedown", onDocMousedown, true);
        window.removeEventListener("scroll", onScroll, true);
    });

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
                        >✕</button>
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
                        >✕</button>
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
                <button
                    type="button"
                    class="cp-close-icon"
                    aria-label={$LL.common.close()}
                    tabindex="-1"
                    on:click|stopPropagation={closePopover}
                >✕</button>
            {/if}
        </div>
    </div>

    <!-- Popover -->
    {#if isOpen}
        <div
            bind:this={popoverEl}
            id="cp-listbox"
            class="cp-popover"
            role="listbox"
            aria-label="Category results"
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
                        on:click={() => onItemClick(0)}
                        on:mouseenter={() => (activeIndex = 0)}
                    >
                        <span class="cp-option-name">{$LL.categoryPicker.uncategorized()}</span>
                        {#if hasUncategorized}
                            <span class="cp-check" aria-hidden="true">✓</span>
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
                        on:click={() => onItemClick(idx)}
                        on:mouseenter={() => (activeIndex = idx)}
                    >
                        <span class="cp-option-name">{cat.name}</span>
                        {#if isSelected(cat.id)}
                            <span class="cp-check" aria-hidden="true">✓</span>
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
        background: #dbeafe;
        color: #1e40af;
        border-radius: 9999px;
        font-size: 0.8rem;
        font-weight: 500;
        border: 1px solid #bfdbfe;
        line-height: 1.4;
    }

    .cp-chip--uncat {
        background: #f3f4f6;
        color: #6b7280;
        border-color: #d1d5db;
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
    }

    .cp-chip-remove:hover {
        opacity: 1;
    }

    .cp-clear-all {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 0.75rem;
        color: #ef4444;
        padding: 0;
        margin-bottom: 4px;
        text-decoration: underline;
        font-family: inherit;
    }

    .cp-clear-all:focus-visible {
        outline: 2px solid #2563eb;
        outline-offset: 1px;
        border-radius: 2px;
    }

    /* ── Trigger ───────────────────────────────────────────────────────────────── */

    .cp-trigger {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 7px 10px;
        border: 1px solid #d1d5db;
        border-radius: 8px;
        background: #fff;
        cursor: text;
        transition: border-color 0.15s, box-shadow 0.15s;
    }

    .cp-trigger:focus-within,
    .cp-root.cp-open .cp-trigger {
        border-color: #2563eb;
        box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.15);
        outline: none;
    }

    .cp-search-icon {
        color: #9ca3af;
        flex-shrink: 0;
    }

    .cp-trigger input[type="text"] {
        flex: 1;
        border: none;
        outline: none;
        font-size: 0.9rem;
        font-family: inherit;
        color: #1e293b;
        background: transparent;
        min-width: 0;
        padding: 0;
    }

    .cp-trigger input[type="text"]::placeholder {
        color: #9ca3af;
    }

    .cp-close-icon {
        background: none;
        border: none;
        cursor: pointer;
        color: #9ca3af;
        font-size: 0.8rem;
        padding: 2px 4px;
        border-radius: 4px;
        line-height: 1;
        transition: color 0.15s;
    }

    .cp-close-icon:hover {
        color: #1e293b;
    }

    /* ── Popover ──────────────────────────────────────────────────────────────── */

    .cp-popover {
        position: fixed;
        z-index: 500;
        width: 300px;
        max-height: 320px;
        overflow-y: auto;
        background: #fff;
        border: 1px solid #e2e8f0;
        border-radius: 10px;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.13);
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
        color: #1e293b;
        transition: background 0.1s;
        user-select: none;
    }

    .cp-option:hover,
    .cp-option--active {
        background: #eff6ff;
    }

    .cp-option--uncat {
        color: #6b7280;
        font-style: italic;
    }

    .cp-option--create {
        color: #2563eb;
        border-top: 1px solid #e2e8f0;
        margin-top: 4px;
        padding-top: 10px;
    }

    .cp-option--create:hover,
    .cp-option--create.cp-option--active {
        background: #eff6ff;
        color: #1d4ed8;
    }

    .cp-option-name {
        flex: 1;
    }

    .cp-check {
        color: #2563eb;
        font-weight: 600;
        font-size: 0.85rem;
    }

    .cp-creating {
        color: #9ca3af;
        font-style: italic;
    }

    /* ── Status / error ─────────────────────────────────────────────────────── */

    .cp-status {
        padding: 12px;
        text-align: center;
        color: #9ca3af;
        font-size: 0.85rem;
    }

    .cp-create-error {
        padding: 6px 12px;
        color: #ef4444;
        font-size: 0.78rem;
        text-align: center;
    }
</style>
