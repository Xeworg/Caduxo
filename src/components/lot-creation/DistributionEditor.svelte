<!--
  DistributionEditor.svelte — optional, opt-in multi-location initial
  distribution editor for the reusable `LotForm` create flow (ODD task 3
  of `scanner-first-inventory`).

  Contract:
    - The parent (LotForm) owns the `allocations` array via two-way
      binding. The editor mutates the array (add, remove, update) and
      reports row-level + aggregate validity through the
      `onValidityChange` callback so the parent can disable the submit
      button when the editor is not valid.
    - The first row is the anchor (receives the `entry:initial`
      movement). It cannot be removed; row reordering is intentionally
      NOT supported (the backend guarantees the first allocation becomes
      the anchor).
    - Every location appears in at most one row. Already-selected
      locations are surfaced as disabled options in every other row's
      location Listbox.
    - Active same-store locations are the only selectable options; the
      parent filters and the editor additionally surfaces a friendly
      empty state when no active location is available.
    - The total quantity is fixed by the parent (LotForm's `quantity`).
      The editor shows a live, polite-region feedback region with the
      current allocated total and a status sentence (match / short /
      over).
    - Integer-unit products reject fractional row quantities with an
      inline alert; the backend enforces the same rule independently.

  Accessibility:
    - Each row is a labelled region with an `aria-label` that names its
      role ("Anchor location", "Additional location N").
    - The location Listbox carries the row's accessible label so screen
      readers announce "Anchor location. Select location" instead of
      just "Select location".
    - The "Remove allocation" button carries
      `aria-label="Remove allocation N"` and is disabled on the anchor.
    - After add / remove, focus moves to the location Listbox of the
      newly-added row / the row above the removed one. The container
      uses `bind:this={rowContainerEls}` keyed by the stable row id.
    - The live total feedback uses `role="status"` + `aria-live="polite"`
      so the aggregate state is announced without interrupting the
      user.

  Tailwind classes referenced here (for the JIT scanner):
    card card-body card-title card-actions
    bg-base-100 bg-base-200
    border-base-300 border-warning border-error border-success
    flex flex-col flex-wrap gap-*
    btn btn-primary btn-ghost btn-square btn-xs btn-sm
    alert alert-info alert-success alert-warning alert-error alert-soft
-->
<script lang="ts">
    import { tick } from "svelte";
    import type { StoreLocationResponse } from "../../lib/stores.js";
    import Input from "../ui/Input.svelte";
    import Listbox from "../ui/Listbox.svelte";
    import Button from "../ui/Button.svelte";
    import Alert from "../ui/Alert.svelte";
    import { LL } from "../../i18n/i18n-svelte.js";

    /**
     * One editor row. The `id` is a stable client-side key used for
     * keyed each-blocks + focus management; it is never sent to the
     * backend.
     */
    export interface DistributionAllocationRow {
        id: string;
        locationId: string;
        /**
         * String-bridge for `<input type="number">` — matches the
         * existing Input.svelte contract. `quantity` is derived.
         */
        quantityStr: string;
    }

    interface Props {
        /**
         * Active, same-store locations offered to the editor. The
         * editor does not filter further — callers are responsible for
         * passing `is_active === true` rows only, so the editor's
         * "no active locations" empty state matches the parent's
         * intent.
         */
        locations: StoreLocationResponse[];
        /**
         * The lot's total quantity. Drives the aggregate validation
         * check (`sum(allocations) ≈ totalQuantity`) and the live
         * feedback region.
         */
        totalQuantity: number;
        /**
         * Drives the integer-unit fractional-quantity guard. `null`
         * matches the legacy / uncatalogued path (treated as decimal
         * by the backend too).
         */
        unitKind: "integer" | "decimal" | null;
        /** Disables interaction while the parent is submitting. */
        disabled?: boolean;
        /**
         * Two-way bindable allocation list. The editor mutates this
         * array (add / remove / update) and the parent reads it at
         * submit time to build the distributed-create payload. The
         * editor never re-initialises existing rows — the parent owns
         * the seed state so toggling distribution on / off round-trips
         * cleanly.
         */
        allocations: DistributionAllocationRow[];
        /**
         * Fired whenever the editor's aggregate validity changes.
         * `valid` is `true` only when every row is individually valid
         * AND the allocation sum matches `totalQuantity` within the
         * backend tolerance. The `message` is a human-readable summary
         * the parent can surface under the form (or `null` when valid).
         */
        onValidityChange?: (valid: boolean, message: string | null) => void;
    }

    let {
        locations,
        totalQuantity,
        unitKind,
        disabled = false,
        allocations = $bindable(),
        onValidityChange,
    }: Props = $props();

    // ── Tolerances / rules mirrored from the backend ────────────────────────
    //
    // The Rust service uses `DISTRIBUTED_TOTAL_EQ_TOLERANCE = 1e-9` for
    // the sum-vs-total check (`validate_distributed_total` in
    // `src-tauri/src/services/expiry_lots.rs`). We mirror the same
    // value here so the frontend rejects the same inputs the backend
    // would. Mirroring avoids a defensive round-trip when the totals
    // don't match (the backend would still reject with
    // `DistributionTotalMismatch` anyway).
    const TOTAL_EQ_TOLERANCE = 1e-9;
    const isIntegerUnit = $derived(unitKind === "integer");

    // ── ID factory (stable, sortable) ───────────────────────────────────────
    // We use a monotonically-increasing counter per editor instance so
    // generated ids stay small + readable in dev tools + DOM tests. The
    // parent never sees the id; it is purely a Svelte key + DOM
    // selector hook for focus management.
    let nextId = 0;
    function newRowId(): string {
        nextId += 1;
        return `alloc-${nextId}`;
    }

    // ── Refs for focus management ───────────────────────────────────────────
    // Each row container exposes its DOM node via `bind:this`. After
    // add / remove we look up the appropriate container and focus the
    // Listbox trigger inside it. Containers are stored in a Map keyed
    // by row id so we can find the right one after a list mutation.
    const rowContainers: Map<string, HTMLElement> = new Map();

    function focusRowLocation(rowId: string): void {
        const container = rowContainers.get(rowId);
        if (!container) return;
        const trigger = container.querySelector<HTMLButtonElement>(
            'button[role="combobox"]',
        );
        trigger?.focus();
    }

    // ── Derived state ───────────────────────────────────────────────────────
    /** Computed numeric quantity for each row. */
    const parsedQuantities = $derived(
        allocations.map((row) => Number.parseFloat(row.quantityStr) || 0),
    );

    /**
     * Sum of all allocation quantities. Compared against `totalQuantity`
     * to drive the live aggregate feedback + the parent-level
     * `validity` signal.
     */
    const allocatedSum = $derived(parsedQuantities.reduce((s, q) => s + q, 0));

    /** Difference `totalQuantity - allocatedSum`. Positive = under, negative = over. */
    const totalDelta = $derived(totalQuantity - allocatedSum);
    const totalMatches = $derived(Math.abs(totalDelta) <= TOTAL_EQ_TOLERANCE);
    const isShort = $derived(totalDelta > TOTAL_EQ_TOLERANCE);
    const isOver = $derived(totalDelta < -TOTAL_EQ_TOLERANCE);

    /**
     * Per-row validation errors. Each entry is either `null` (row
     * valid) or a stable key that the parent + tests can branch on.
     * Keys map to localised strings via `$LL.lotForm.distribution.*`
     * and to user-facing inline alerts in the row.
     */
    type RowErrorKey =
        | "rowLocationRequired"
        | "rowDuplicateLocation"
        | "rowLocationUnavailable"
        | "rowQuantityRequired"
        | "rowQuantityNonPositive"
        | "rowQuantityFractional";

    const locationIdSet = $derived(new Set(locations.map((l) => l.id)));
    const selectedLocationIds = $derived(allocations.map((a) => a.locationId));

    /**
     * For each row: the list of location ids already used by another
     * row. The editor does not allow the user to pick a location that
     * appears in another allocation (rows show already-selected
     * locations as disabled options).
     */
    function duplicateLocationIds(idx: number): Set<string> {
        const own = allocations[idx]?.locationId ?? "";
        const others = new Set<string>();
        for (let i = 0; i < allocations.length; i++) {
            if (i === idx) continue;
            const id = allocations[i]?.locationId ?? "";
            if (id && id !== own) others.add(id);
        }
        return others;
    }

    function rowError(idx: number): RowErrorKey | null {
        const row = allocations[idx];
        if (!row) return null;
        // Location checks first — they're the cheapest signal and most
        // common error in practice.
        if (!row.locationId) return "rowLocationRequired";
        const duplicates = duplicateLocationIds(idx);
        if (duplicates.has(row.locationId)) return "rowDuplicateLocation";
        if (!locationIdSet.has(row.locationId)) return "rowLocationUnavailable";
        // Quantity checks.
        const qty = parsedQuantities[idx] ?? 0;
        if (!row.quantityStr.trim()) return "rowQuantityRequired";
        if (qty <= 0) return "rowQuantityNonPositive";
        if (isIntegerUnit && qty > 0 && !Number.isInteger(qty)) {
            return "rowQuantityFractional";
        }
        return null;
    }

    const rowErrors = $derived(allocations.map((_, i) => rowError(i)));

    /** All rows valid + total matches + total > 0. */
    const allRowsValid = $derived(
        allocations.length > 0 && rowErrors.every((e) => e === null),
    );
    const totalPositive = $derived(totalQuantity > 0);

    /**
     * Aggregated validity. The `totalQuantity` field on the parent is
     * the source of truth — we do not allow a zero total even if the
     * rows happen to sum to zero. Mirrors the backend's
     * `total_quantity <= 0.0` guard.
     */
    const isValid = $derived(allRowsValid && totalMatches && totalPositive);

    /**
     * Human-readable summary of the editor's state. The parent can
     * surface this string under the form when `valid === false` (we
     * expose it through `onValidityChange`). We keep the most
     * actionable error first.
     */
    const summaryMessage = $derived.by((): string | null => {
        if (allocations.length === 0) {
            // Empty distribution is a structural error — the editor
            // cannot enforce a minimum row count via the disabled
            // "Add row" button alone, so we report it here.
            return null;
        }
        if (!totalPositive) {
            return "lotForm.quantityGreaterThanZero";
        }
        const firstRowError = rowErrors.find((e) => e !== null);
        if (firstRowError) {
            return firstRowError;
        }
        if (!totalMatches) {
            if (isShort) {
                return "lotForm.distribution.totalShort";
            }
            if (isOver) {
                return "lotForm.distribution.totalOver";
            }
        }
        return null;
    });

    /**
     * Re-emit the validity signal whenever any input that affects it
     * changes. The parent uses this to disable the submit button +
     * surface the summary message; we never own submit-block logic
     * (the parent re-checks at submit time).
     */
    $effect(() => {
        onValidityChange?.(isValid, summaryMessage);
    });

    // ── Mutators ────────────────────────────────────────────────────────────

    /**
     * Returns the set of location ids NOT yet used by any row, sorted
     * by location name. Used by the "Add another location" affordance
     * to pick the next picker value and to disable the button when no
     * unused location remains.
     */
    const remainingLocationIds = $derived.by(() => {
        const used = new Set(selectedLocationIds.filter((id) => id !== ""));
        return locations
            .filter((l) => !used.has(l.id))
            .sort((a, b) => a.name.localeCompare(b.name));
    });

    function addRow(): void {
        if (disabled) return;
        // Pick the first remaining location (alphabetical). When no
        // location is available the button is disabled — this guard
        // is the last line of defence.
        const next = remainingLocationIds[0];
        if (!next) return;
        const newRow: DistributionAllocationRow = {
            id: newRowId(),
            locationId: next.id,
            quantityStr: "",
        };
        allocations = [...allocations, newRow];
        // Focus the new row's location picker after Svelte commits the
        // DOM so the screen reader announces the freshly-added row.
        const newId = newRow.id;
        tick().then(() => focusRowLocation(newId));
    }

    function removeRow(idx: number): void {
        if (disabled) return;
        if (idx <= 0) return; // Anchor row is immutable.
        const removedId = allocations[idx]?.id;
        const prevId = allocations[idx - 1]?.id;
        allocations = allocations.filter((_, i) => i !== idx);
        // Move focus to the previous row's location picker so keyboard
        // users don't lose their place. `removeRow` is only ever called
        // for `idx >= 1` (the anchor is immutable), so the previous
        // row always exists and the fallback below never fires — but
        // we keep a defensive guard so a future regression cannot
        // strand focus on a removed node.
        tick().then(() => {
            if (prevId) {
                focusRowLocation(prevId);
            }
        });
        // The removed row's container is unmounted — keep the ref map
        // tidy so the GC can reclaim the DOM node.
        if (removedId) rowContainers.delete(removedId);
    }

    function updateRowLocation(idx: number, locationId: string): void {
        if (disabled) return;
        allocations = allocations.map((row, i) =>
            i === idx ? { ...row, locationId } : row,
        );
    }

    function updateRowQuantity(idx: number, value: string): void {
        if (disabled) return;
        allocations = allocations.map((row, i) =>
            i === idx ? { ...row, quantityStr: value } : row,
        );
    }

    /**
     * Build the listbox option list for the row at `idx`. Already-used
     * locations (including the row's own pick) are surfaced as
     * disabled rows so the picker remains consistent regardless of
     * which row the user opens first. The empty placeholder is folded
     * in as the first row (`value: ""`, disabled) so the field never
     * looks selected-by-default.
     */
    function optionsForRow(idx: number) {
        const duplicates = duplicateLocationIds(idx);
        const sorted = [...locations].sort((a, b) =>
            a.name.localeCompare(b.name),
        );
        return [
            {
                value: "",
                label: $LL.lotForm.selectLocationPrompt(),
                disabled: true,
            },
            ...sorted.map((loc) => ({
                value: loc.id,
                label: loc.name,
                disabled: duplicates.has(loc.id),
            })),
        ];
    }

    /** Accessible label for the location Listbox inside a row. */
    function locationAriaLabel(idx: number): string {
        if (idx === 0) return $LL.lotForm.distribution.anchorLabel();
        return $LL.lotForm.distribution.additionalLocationLabel({
            index: idx + 1,
        });
    }

    function quantityAriaLabel(idx: number): string {
        if (idx === 0) return $LL.lotForm.distribution.anchorQuantityLabel();
        return $LL.lotForm.distribution.quantityLabel();
    }

    /**
     * Round helper for the live total display. The UI never shows more
     * than two decimals for the aggregate (per-row inputs keep their
     * raw string). Keeps the display stable when the user is mid-edit
     * and a `quantityStr` evaluates to a slightly different float
     * representation.
     */
    function formatQuantity(value: number): string {
        if (Number.isInteger(value)) return value.toString();
        // Two decimals, trimmed, matches the backend's `{:.2}` format.
        return value.toFixed(2).replace(/\.?0+$/, "");
    }

    /**
     * Svelte action that registers / updates / removes the row
     * container element in the `rowContainers` map keyed by the row
     * id. We use the action pattern instead of `bind:this` so we can
     * capture multiple bindings (one per row) — Svelte 5 binds to a
     * single variable, not a keyed map. The action captures both the
     * initial mount + every subsequent `update` call.
     */
    function setRowContainer(node: HTMLElement, rowId: string) {
        rowContainers.set(rowId, node);
        return {
            update(nextRowId: string) {
                if (nextRowId !== rowId) {
                    rowContainers.delete(rowId);
                    rowContainers.set(nextRowId, node);
                }
            },
            destroy() {
                rowContainers.delete(rowId);
            },
        };
    }
</script>

{#if locations.length === 0}
    <!--
      No active locations available for distribution. The editor
      surfaces this as an inline Alert so the parent can keep the
      toggle on without the user seeing an empty form section.
    -->
    <Alert variant="info">{$LL.lotForm.distribution.noLocationsAvailable()}</Alert>
{:else}
    <div
        class="distribution-editor"
        aria-labelledby="distribution-editor-heading"
    >
        <h4 class="distribution-heading" id="distribution-editor-heading">
            {$LL.lotForm.distribution.sectionHeading()}
        </h4>
        <p class="distribution-anchor-hint">
            {$LL.lotForm.distribution.anchorHint()}
        </p>

        <!--
          Live aggregate feedback region. `role="status"` +
          `aria-live="polite"` mirrors the scanner's existing
          feedback pattern (see ScannerPage's loading-diag region) so
          screen readers announce "Allocated total matches…" / "…
          still needs to be allocated" without interrupting the user.
        -->
        <div
            class="distribution-total"
            role="status"
            aria-live="polite"
        >
            <span class="distribution-total-label">
                {$LL.lotForm.distribution.totalLabel()}:
            </span>
            <span class="distribution-total-value">
                {formatQuantity(allocatedSum)} / {formatQuantity(totalQuantity)}
            </span>
            <span
                class="distribution-total-message"
                class:distribution-total-message--ok={totalMatches && totalPositive}
                class:distribution-total-message--warn={!totalMatches && totalPositive}
                class:distribution-total-message--error={!totalPositive}
            >
                {#if !totalPositive}
                    {$LL.lotForm.quantityGreaterThanZero()}
                {:else if totalMatches}
                    {$LL.lotForm.distribution.totalMatch({
                        total: formatQuantity(totalQuantity),
                    })}
                {:else if isShort}
                    {$LL.lotForm.distribution.totalShort({
                        remaining: formatQuantity(totalDelta),
                    })}
                {:else if isOver}
                    {$LL.lotForm.distribution.totalOver({
                        overflow: formatQuantity(-totalDelta),
                    })}
                {:else}
                    {$LL.lotForm.distribution.totalMismatch({
                        allocated: formatQuantity(allocatedSum),
                        total: formatQuantity(totalQuantity),
                    })}
                {/if}
            </span>
        </div>

        <ul class="distribution-rows" role="list">
            {#each allocations as row, idx (row.id)}
                {@const rowKey = row.id}
                {@const errorKey = rowErrors[idx] ?? null}
                <li
                    class="distribution-row"
                    class:distribution-row--invalid={errorKey !== null}
                    class:distribution-row--anchor={idx === 0}
                    use:setRowContainer={rowKey}
                    aria-label={idx === 0
                        ? $LL.lotForm.distribution.anchorLabel()
                        : $LL.lotForm.distribution.additionalLocationLabel({
                              index: idx + 1,
                          })}
                >
                    <div class="distribution-row-fields">
                        <Listbox
                            value={row.locationId}
                            options={optionsForRow(idx)}
                            size="md"
                            aria-label={locationAriaLabel(idx)}
                            disabled={disabled}
                            invalid={errorKey === "rowLocationRequired" ||
                                errorKey === "rowDuplicateLocation" ||
                                errorKey === "rowLocationUnavailable"}
                            required
                            onchange={(v) => updateRowLocation(idx, v)}
                        />
                        <Input
                            value={row.quantityStr}
                            type="number"
                            label={quantityAriaLabel(idx)}
                            required
                            invalid={errorKey === "rowQuantityRequired" ||
                                errorKey === "rowQuantityNonPositive" ||
                                errorKey === "rowQuantityFractional"}
                            disabled={disabled}
                            aria-label={quantityAriaLabel(idx)}
                            aria-describedby={`distribution-row-${rowKey}-hint`}
                            oninput={(v) => updateRowQuantity(idx, v)}
                            placeholder={isIntegerUnit ? "0" : "0.00"}
                        />
                        <Button
                            type="button"
                            variant="icon"
                            size="sm"
                            aria-label={idx === 0
                                ? $LL.lotForm.distribution.removeRowDisabledAnchor()
                                : $LL.lotForm.distribution.removeRow({
                                      index: idx + 1,
                                  })}
                            disabled={disabled || idx === 0}
                            onclick={() => removeRow(idx)}
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                aria-hidden="true"
                                focusable="false"
                                class="h-4 w-4"
                            >
                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                <line x1="6" y1="6" x2="18" y2="18"></line>
                            </svg>
                        </Button>
                    </div>
                    <p
                        class="distribution-row-hint"
                        class:distribution-row-hint--error={errorKey !== null}
                        id={`distribution-row-${rowKey}-hint`}
                        aria-live="polite"
                    >
                        {#if errorKey === "rowLocationRequired"}
                            {$LL.lotForm.distribution.rowLocationRequired()}
                        {:else if errorKey === "rowDuplicateLocation"}
                            {$LL.lotForm.distribution.rowDuplicateLocation()}
                        {:else if errorKey === "rowLocationUnavailable"}
                            {$LL.lotForm.distribution.rowLocationUnavailable()}
                        {:else if errorKey === "rowQuantityRequired"}
                            {$LL.lotForm.distribution.rowQuantityRequired()}
                        {:else if errorKey === "rowQuantityNonPositive"}
                            {$LL.lotForm.distribution.rowQuantityNonPositive()}
                        {:else if errorKey === "rowQuantityFractional"}
                            {$LL.lotForm.distribution.rowQuantityFractional()}
                        {:else if idx === 0}
                            {$LL.lotForm.distribution.anchorLabel()}
                        {:else}
                            {$LL.lotForm.distribution.additionalLocationLabel({
                                index: idx + 1,
                            })}
                        {/if}
                    </p>
                </li>
            {/each}
        </ul>

        <div class="distribution-actions">
            <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={disabled || remainingLocationIds.length === 0}
                aria-label={remainingLocationIds.length === 0
                    ? $LL.lotForm.distribution.addRowDisabledNoMoreLocations()
                    : $LL.lotForm.distribution.addRow()}
                onclick={addRow}
            >
                + {$LL.lotForm.distribution.addRow()}
            </Button>
        </div>
    </div>
{/if}

<!--

  (The `setRowContainer` action lives in the main `<script>` block
  above so this component keeps a single top-level script.)

-->
<style>
    .distribution-editor {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 12px;
        border: 1px solid var(--color-base-300);
        border-radius: 8px;
        background: var(--color-base-100);
    }

    .distribution-heading {
        margin: 0;
        font-size: 0.92rem;
        font-weight: 600;
        color: var(--color-base-content);
    }

    .distribution-anchor-hint {
        margin: 0;
        font-size: 0.78rem;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        line-height: 1.4;
    }

    .distribution-total {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 6px;
        padding: 6px 10px;
        border-radius: 6px;
        background: color-mix(in oklch, var(--color-base-200) 60%, transparent);
        font-size: 0.82rem;
        color: var(--color-base-content);
    }

    .distribution-total-label {
        font-weight: 600;
        color: var(--color-base-content);
    }

    .distribution-total-value {
        font-variant-numeric: tabular-nums;
        font-weight: 500;
    }

    .distribution-total-message {
        font-size: 0.78rem;
        color: var(--color-base-content);
        margin-left: auto;
    }

    .distribution-total-message--ok {
        color: var(--color-success);
    }

    .distribution-total-message--warn {
        color: var(--color-warning);
    }

    .distribution-total-message--error {
        color: var(--color-error);
    }

    .distribution-rows {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .distribution-row {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 8px;
        border-radius: 6px;
        border: 1px solid var(--color-base-300);
        background: color-mix(in oklch, var(--color-base-100) 92%, transparent);
    }

    .distribution-row--anchor {
        border-color: color-mix(in oklch, var(--color-primary) 35%, transparent);
        background: color-mix(in oklch, var(--color-primary) 4%, transparent);
    }

    .distribution-row--invalid {
        border-color: color-mix(in oklch, var(--color-error) 55%, transparent);
        background: color-mix(in oklch, var(--color-error) 4%, transparent);
    }

    .distribution-row-fields {
        display: grid;
        grid-template-columns: 1fr 130px auto;
        gap: 8px;
        align-items: end;
    }

    .distribution-row-hint {
        margin: 0;
        padding: 0 4px;
        font-size: 0.74rem;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        line-height: 1.35;
    }

    .distribution-row-hint--error {
        color: var(--color-error);
    }

    .distribution-actions {
        display: flex;
        justify-content: flex-start;
    }
</style>
