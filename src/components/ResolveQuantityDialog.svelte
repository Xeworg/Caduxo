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

    // ── Props ──────────────────────────────────────────────────────────────────

    /** The lot to resolve quantity from. */
    export let lot: ExpiryLotResponse;
    /** Called after a successful resolve. */
    export let onResolved: (result: ExpiryLotResolveResult) => void;
    /** Called when the user dismisses the dialog. */
    export let onClose: () => void;

    // ── Local state ────────────────────────────────────────────────────────────

    let quantity = 1;
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
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-labelledby="resolve-title">
    <div class="dialog">
        <header class="dialog-header">
            <h3 id="resolve-title">{$LL.lotMovements.resolution.resolveQuantity()}</h3>
            <button
                type="button"
                class="btn-close"
                title={$LL.lotMovements.modal.close()}
                on:click={onClose}
            >
                ✕
            </button>
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

        <form on:submit|preventDefault={submit}>
            <div class="grid-2">
                <label>
                    {$LL.lotMovements.resolution.quantityToResolve()}
                    <input
                        type="number"
                        bind:value={quantity}
                        min="0.01"
                        max={lot.quantity}
                        step="0.01"
                        required
                    />
                </label>

                <label>
                    {$LL.lotMovements.resolution.resolutionType()}
                    <select bind:value={resolution}>
                        <option value="consumed">{$LL.lotMovements.resolution.consumed()}</option>
                        <option value="sold">{$LL.lotMovements.resolution.sold()}</option>
                        <option value="discarded">{$LL.lotMovements.resolution.discarded()}</option>
                        <option value="donated">{$LL.lotMovements.resolution.donated()}</option>
                        <option value="other">{$LL.lotMovements.resolution.other()}</option>
                    </select>
                </label>
            </div>

            <label>
                {$LL.lotMovements.resolution.notesOptional()}
                <textarea
                    bind:value={notes}
                    placeholder={$LL.lotMovements.resolution.notesPlaceholder()}
                    rows="2"
                ></textarea>
            </label>

            <div class="form-actions">
                <button
                    type="submit"
                    class="btn-primary"
                    disabled={submitting}
                >
                    {submitting ? $LL.lotMovements.resolution.resolving() : $LL.lotMovements.resolution.resolve()}
                </button>
                <button
                    type="button"
                    class="btn-secondary"
                    on:click={onClose}
                    disabled={submitting}
                >
                    {$LL.lotMovements.modal.cancel()}
                </button>
            </div>
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
    </div>
</div>

<style>
    .overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.45);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 50;
    }

    .dialog {
        background: #fff;
        border-radius: 12px;
        padding: 24px;
        width: min(520px, calc(100vw - 32px));
        max-height: calc(100vh - 64px);
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 16px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
    }

    .dialog-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .dialog-header h3 {
        margin: 0;
        font-size: 1.05rem;
    }

    .btn-close {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 1rem;
        padding: 2px 8px;
        color: #6b7280;
        border-radius: 4px;
    }

    .btn-close:hover {
        background: #f3f4f6;
        color: #374151;
    }

    .alert {
        padding: 10px 14px;
        border-radius: 6px;
        font-size: 0.9rem;
    }

    .alert-error {
        background: #fee2e2;
        color: #991b1b;
        border: 1px solid #fca5a5;
    }

    .lot-summary {
        background: #f9fafb;
        border: 1px solid #e5e7eb;
        border-radius: 7px;
        padding: 10px 14px;
        font-size: 0.85rem;
        color: #374151;
    }

    .batch {
        background: #e5e7eb;
        border-radius: 4px;
        padding: 1px 5px;
        font-size: 0.75rem;
        margin-right: 4px;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    label {
        display: flex;
        flex-direction: column;
        gap: 4px;
        font-size: 0.85rem;
        color: #374151;
    }

    label input[type="number"],
    label textarea,
    label select {
        padding: 7px 10px;
        border: 1px solid #d1d5db;
        border-radius: 6px;
        font-size: 0.9rem;
        font-family: inherit;
    }

    label input:focus,
    label textarea:focus,
    label select:focus {
        outline: 2px solid #3b82f6;
        border-color: #3b82f6;
    }

    .grid-2 {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
    }

    .form-actions {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }

    .btn-primary {
        background: #2563eb;
        color: #fff;
        border: none;
        border-radius: 6px;
        padding: 8px 16px;
        font-size: 0.9rem;
        cursor: pointer;
        font-family: inherit;
    }

    .btn-primary:hover:not(:disabled) {
        background: #1d4ed8;
    }

    .btn-primary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .btn-secondary {
        background: #fff;
        color: #374151;
        border: 1px solid #d1d5db;
        border-radius: 6px;
        padding: 8px 16px;
        font-size: 0.9rem;
        cursor: pointer;
        font-family: inherit;
    }

    .btn-secondary:hover:not(:disabled) {
        background: #f9fafb;
    }

    .btn-secondary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    /* History */
    .history {
        border-top: 1px solid #f3f4f6;
        padding-top: 12px;
    }

    .history h4 {
        margin: 0 0 8px;
        font-size: 0.85rem;
        color: #6b7280;
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
        background: #f9fafb;
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
        background: #dcfce7;
        color: #166534;
    }

    .badge-discarded {
        background: #fee2e2;
        color: #991b1b;
    }

    .badge-sold,
    .badge-donated,
    .badge-other {
        background: #dbeafe;
        color: #1e40af;
    }

    .event-notes {
        color: #6b7280;
        flex: 1;
        font-style: italic;
    }

    .event-date {
        color: #9ca3af;
        font-size: 0.74rem;
        margin-left: auto;
    }

    .loading,
    .no-history {
        font-size: 0.82rem;
        color: #9ca3af;
        font-style: italic;
        margin: 0;
    }
</style>
