<!--
  ResolveQuantityDialog.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7b. Shell markup replaced with the
  shared primitive; the quantity input now uses Input.svelte per
  the spec. Resolution select and notes textarea stay inline (the
  resolution select is a five-value domain tuple not in scope;
  textareas are out of scope for Input.svelte). Business state,
  validation, submit handlers, history panel, and visible copy
  preserved verbatim.
-->
<script lang="ts">
    import { LL } from "../i18n/i18n-svelte.js";
    import { humanizeError } from "../lib/errors.js";
    import {
        resolveExpiryLot,
        listLotResolutionEvents,
        type ExpiryLotResponse,
        type ExpiryLotResolveResult,
        type LotResolutionEventResponse,
    } from "../lib/expiry_lots.js";
    import Modal from "./ui/Modal.svelte";
    import Input from "./ui/Input.svelte";
    import Button from "./ui/Button.svelte";
    import Listbox from "./ui/Listbox.svelte";

    // ── Props ──────────────────────────────────────────────────────────────────

    /** The lot to resolve quantity from. */
    export let lot: ExpiryLotResponse;
    /** Called after a successful resolve. */
    export let onResolved: (result: ExpiryLotResolveResult) => void;
    /** Called when the user dismisses the dialog. */
    export let onClose: () => void;

    // ── Local state ────────────────────────────────────────────────────────────

    /** Backs the `<Modal>` primitive via two-way binding. */
    let visible = true;
    /** Element to receive focus when the modal closes (the trigger row). */
    let returnFocusTo: HTMLElement | null = null;

    /**
     * Two-way bridge between `quantityAsString` (bound to the
     * `<Input>` primitive's `value: string` contract) and
     * `quantity: number` (consumed by the submit handler + the
     * validation guards). The HTML `<input type="number">` round-
     * trips through a string, so we keep both representations in
     * sync.
     */
    let quantityAsString = "1";
    $: quantity =
        quantityAsString === "" || quantityAsString === "-"
            ? 0
            : Number(quantityAsString);

    let resolution: "consumed" | "sold" | "discarded" | "donated" | "other" = "consumed";
    let notes = "";

    let submitting = false;
    let errorMsg = "";

    // History panel
    let history: LotResolutionEventResponse[] = [];
    let loadingHistory = false;

    // ── Load history ───────────────────────────────────────────────────────────

    async function loadHistory() {
        loadingHistory = true;
        try {
            history = await listLotResolutionEvents(lot.id);
        } catch {
            history = [];
        } finally {
            loadingHistory = false;
        }
    }

    loadHistory();

    // ── Submit ─────────────────────────────────────────────────────────────────

    function resolutionLabel(value: LotResolutionEventResponse["resolution"]): string {
        const labels: Record<string, () => string> = {
            consumed: $LL.lotMovements.resolution.consumed,
            sold: $LL.lotMovements.resolution.sold,
            discarded: $LL.lotMovements.resolution.discarded,
            donated: $LL.lotMovements.resolution.donated,
            other: $LL.lotMovements.resolution.other,
        };

        return labels[value]?.() ?? value;
    }

    /**
     * Listbox option shape for the resolution-type picker. Built once
     * per render from the i18n catalogue so the visible labels stay
     * verbatim from the original shell.
     */
    $: resolutionOptions = [
        { value: "consumed", label: $LL.lotMovements.resolution.consumed() },
        { value: "sold", label: $LL.lotMovements.resolution.sold() },
        { value: "discarded", label: $LL.lotMovements.resolution.discarded() },
        { value: "donated", label: $LL.lotMovements.resolution.donated() },
        { value: "other", label: $LL.lotMovements.resolution.other() },
    ];

    function formatDateTime(value: string): string {
        // Delegate to the locale-aware typesafe-i18n `dateTime` formatter
        // exposed via `$LL.lotMovements.resolution.eventDateTime`. Avoids
        // branching on `en`/`es` at the call site.
        if (!value) return "";
        return $LL.lotMovements.resolution.eventDateTime({ value });
    }

    async function submit() {
        errorMsg = "";
        if (quantity <= 0) {
            errorMsg = $LL.lotMovements.resolution.quantityPositiveError();
            return;
        }
        if (quantity > lot.quantity) {
            errorMsg = $LL.lotMovements.resolution.quantityExceedsRemainingError({ quantity: lot.quantity });
            return;
        }
        submitting = true;
        try {
            const result = await resolveExpiryLot({
                lot_id: lot.id,
                quantity,
                resolution,
                notes: notes.trim() || null,
            });
            onResolved(result);
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        } finally {
            submitting = false;
        }
    }

    function handleCancel() {
        if (submitting) return;
        onClose();
    }

    function handleClose() {
        if (submitting) return;
        onClose();
    }
</script>

<Modal
    bind:open={visible}
    size="md"
    showClose
    closeLabel={$LL.lotMovements.modal.close()}
    {returnFocusTo}
    oncancel={handleCancel}
    onclose={handleClose}
    titleId="resolve-title"
>
    {#snippet children()}
        <header class="dialog-header">
            <h3 id="resolve-title">{$LL.lotMovements.resolution.resolveQuantity()}</h3>
        </header>

        {#if errorMsg}
            <div class="alert alert-error" role="alert">{errorMsg}</div>
        {/if}

        <!-- Lot summary -->
        <div class="lot-summary">
            <span class="lot-unit">
                {#if lot.batch_code}
                    <span class="batch">{lot.batch_code}</span>
                {/if}
                {$LL.lotMovements.resolution.remainingSummary({ quantity: lot.quantity, unit: lot.unit || $LL.lotMovements.unitsFallback() })}
                &nbsp;·&nbsp; {$LL.lotMovements.resolution.expirySummary({ date: lot.expiry_date })}
            </span>
        </div>

        <form
            onsubmit={(e) => {
                e.preventDefault();
                submit();
            }}
            class="resolve-form"
        >
            <div class="grid-2">
                <Input
                    id="resolve-qty"
                    type="number"
                    label={$LL.lotMovements.resolution.quantityToResolve()}
                    bind:value={quantityAsString}
                    disabled={submitting}
                />

                <label>
                    <span>{$LL.lotMovements.resolution.resolutionType()}</span>
                    <Listbox
                        bind:value={resolution}
                        options={resolutionOptions}
                        disabled={submitting}
                        aria-label={$LL.lotMovements.resolution.resolutionType()}
                    />
                </label>
            </div>

            <label>
                {$LL.lotMovements.resolution.notesOptional()}
                <textarea
                    class="textarea w-full motion-reduce:transition-none"
                    bind:value={notes}
                    placeholder={$LL.lotMovements.resolution.notesPlaceholder()}
                    rows="2"
                    disabled={submitting}
                ></textarea>
            </label>
        </form>

        <!-- Resolution history -->
        {#if loadingHistory}
            <p class="loading">{$LL.lotMovements.resolution.loadingHistory()}</p>
        {:else if history.length > 0}
            <div class="history">
                <h4>{$LL.lotMovements.resolution.resolutionHistory()}</h4>
                <ul class="history-list">
                    {#each history as event (event.id)}
                        <li class="history-item">
                            <span class="event-qty">
                                {event.quantity} {lot.unit || ""}
                            </span>
                            <span class="event-type badge-{event.resolution}">
                                {resolutionLabel(event.resolution)}
                            </span>
                            {#if event.notes}
                                <span class="event-notes">{event.notes}</span>
                            {/if}
                            <span class="event-date">
                                {formatDateTime(event.created_at)}
                            </span>
                        </li>
                    {/each}
                </ul>
            </div>
        {:else}
            <p class="no-history">{$LL.lotMovements.resolution.noHistory()}</p>
        {/if}
    {/snippet}

    {#snippet footer()}
        <Button
            variant="primary"
            onclick={submit}
            disabled={submitting}
            loading={submitting}
        >
            {submitting ? $LL.lotMovements.resolution.resolving() : $LL.lotMovements.resolution.resolve()}
        </Button>
        <Button
            variant="ghost"
            onclick={handleClose}
            disabled={submitting}
        >
            {$LL.lotMovements.modal.cancel()}
        </Button>
    {/snippet}
</Modal>

<style>
    /* ── Header ────────────────────────────────────────────────────────────── */
    .dialog-header {
        display: flex;
        align-items: center;
        margin-bottom: 12px;
    }

    .dialog-header h3 {
        margin: 0;
        font-size: 1.05rem;
        color: var(--color-base-content);
    }

    /* ── Alert ─────────────────────────────────────────────────────────────── */
    .alert {
        padding: 10px 14px;
        border-radius: 6px;
        font-size: 0.9rem;
        margin-bottom: 12px;
    }

    .alert-error {
        background: color-mix(in oklch, var(--color-error) 12%, transparent);
        color: var(--color-error);
        border: 1px solid color-mix(in oklch, var(--color-error) 30%, transparent);
    }

    /* ── Lot summary ───────────────────────────────────────────────────────── */
    .lot-summary {
        background: var(--color-base-200);
        border: 1px solid var(--color-base-300);
        border-radius: 7px;
        padding: 10px 14px;
        font-size: 0.85rem;
        color: var(--color-base-content);
        margin-bottom: 14px;
    }

    .batch {
        background: var(--color-base-300);
        border-radius: 4px;
        padding: 1px 5px;
        font-size: 0.75rem;
        margin-right: 4px;
    }

    /* ── Form ──────────────────────────────────────────────────────────────── */
    .resolve-form {
        display: flex;
        flex-direction: column;
        gap: 14px;
        margin-bottom: 4px;
    }

    label {
        display: flex;
        flex-direction: column;
        gap: 4px;
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .grid-2 {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
    }

    /* History */
    .history {
        border-top: 1px solid var(--color-base-200);
        padding-top: 12px;
        margin-top: 14px;
    }

    .history h4 {
        margin: 0 0 8px;
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .history-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .history-item {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 0.82rem;
        padding: 6px 8px;
        background: var(--color-base-200);
        border-radius: 5px;
        flex-wrap: wrap;
    }

    .event-qty {
        font-weight: 500;
    }

    .event-type {
        font-size: 0.7rem;
        padding: 1px 6px;
        border-radius: 4px;
    }

    .badge-consumed {
        background: color-mix(in oklch, var(--color-success) 18%, transparent);
        color: var(--color-success);
    }

    .badge-discarded {
        background: color-mix(in oklch, var(--color-error) 15%, transparent);
        color: var(--color-error);
    }

    .badge-sold,
    .badge-donated,
    .badge-other {
        background: color-mix(in oklch, var(--color-info) 15%, transparent);
        color: var(--color-info);
    }

    .event-notes {
        color: var(--color-base-content);
        flex: 1;
        font-style: italic;
    }

    .event-date {
        color: var(--color-base-content);
        font-size: 0.74rem;
        margin-left: auto;
    }

    .loading,
    .no-history {
        font-size: 0.82rem;
        color: var(--color-base-content);
        font-style: italic;
        margin: 12px 0 0;
    }
</style>
