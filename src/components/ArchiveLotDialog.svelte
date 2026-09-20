<script lang="ts">
    import { LL } from "../i18n/i18n-svelte.js";
    import { humanizeError } from "../lib/errors.js";
    import {
        archiveExpiryLot,
        ARCHIVE_REASONS,
        ARCHIVE_NOTES_MAX_CHARS,
        ARCHIVE_NOTES_MIN_CHARS,
        type ExpiryLotResponse,
    } from "../lib/expiry_lots.js";

    // ── Props ──────────────────────────────────────────────────────────────────

    /** The lot to archive. */
    export let lot: ExpiryLotResponse;
    /** Called after a successful archive. */
    export let onArchived: () => void;
    /** Called when the user dismisses the dialog without archiving. */
    export let onClose: () => void;

    // ── Local state ────────────────────────────────────────────────────────────

    let reason = ARCHIVE_REASONS[0]?.value ?? "";
    let notes = "";
    let submitting = false;
    let errorMsg = "";

    // ── Derived validation ─────────────────────────────────────────────────────

    $: trimmedNotes = notes.trim();
    $: notesChars = trimmedNotes.length;
    $: notesTooShort = notesChars < ARCHIVE_NOTES_MIN_CHARS;
    $: notesTooLong = notesChars > ARCHIVE_NOTES_MAX_CHARS;
    $: notesValid = !notesTooShort && !notesTooLong;
    $: reasonValid = ARCHIVE_REASONS.some((r: { value: string; label: string }) => r.value === reason);
    $: canSubmit = reasonValid && notesValid && !submitting;

    // ── Submit ─────────────────────────────────────────────────────────────────

    async function submit() {
        errorMsg = "";
        if (!reasonValid) {
            errorMsg = $LL.lotMovements.archive.selectReasonError();
            return;
        }
        if (notesTooShort) {
            errorMsg = $LL.lotMovements.archive.notesMinError({ min: ARCHIVE_NOTES_MIN_CHARS });
            return;
        }
        if (notesTooLong) {
            errorMsg = $LL.lotMovements.archive.notesMaxError({ max: ARCHIVE_NOTES_MAX_CHARS });
            return;
        }
        submitting = true;
        try {
            await archiveExpiryLot({
                id: lot.id,
                reason,
                notes: trimmedNotes,
            });
            onArchived();
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        } finally {
            submitting = false;
        }
    }

    function handleClose() {
        if (submitting) return;
        onClose();
    }

    function archiveReasonLabel(value: string): string {
        const labels: Record<string, () => string> = {
            expired_unsold: $LL.lotMovements.archive.reasons.expiredUnsold,
            damaged: $LL.lotMovements.archive.reasons.damaged,
            returned_to_supplier: $LL.lotMovements.archive.reasons.returnedToSupplier,
            recall: $LL.lotMovements.archive.reasons.recall,
            lost: $LL.lotMovements.archive.reasons.lost,
            internal_use: $LL.lotMovements.archive.reasons.internalUse,
            administrative: $LL.lotMovements.archive.reasons.administrative,
            other: $LL.lotMovements.archive.reasons.other,
        };

        return labels[value]?.() ?? value;
    }
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-labelledby="archive-title">
    <div class="dialog">
        <header class="dialog-header">
            <h3 id="archive-title">{$LL.lotMovements.archive.title()}</h3>
            <button
                type="button"
                class="btn-close"
                title={$LL.lotMovements.modal.close()}
                on:click={handleClose}
                disabled={submitting}
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
                <strong>{$LL.lotMovements.archive.quantitySummary({ quantity: lot.quantity, unit: lot.unit || $LL.lotMovements.unitsFallback() })}</strong>
                &nbsp;·&nbsp; {$LL.lotMovements.archive.expirySummary({ date: lot.expiry_date })}
            </span>
            <p class="hint">
                {$LL.lotMovements.archive.hint()}
            </p>
        </div>

        <form on:submit|preventDefault={submit}>
            <label>
                {$LL.lotMovements.archive.reason()}
                <select bind:value={reason} required disabled={submitting}>
                    {#each ARCHIVE_REASONS as option (option.value)}
                        <option value={option.value}>{archiveReasonLabel(option.value)}</option>
                    {/each}
                </select>
            </label>

            <label>
                {$LL.lotMovements.archive.notes()}
                <span class="char-counter">
                    <span
                        class:invalid={notesTooShort || notesTooLong}
                    >{notesChars}</span>
                    / {ARCHIVE_NOTES_MIN_CHARS}–{ARCHIVE_NOTES_MAX_CHARS}
                </span>
                <textarea
                    bind:value={notes}
                    placeholder={$LL.lotMovements.archive.notesPlaceholder()}
                    rows="3"
                    minlength={ARCHIVE_NOTES_MIN_CHARS}
                    maxlength={ARCHIVE_NOTES_MAX_CHARS + 200}
                    required
                    disabled={submitting}
                ></textarea>
                {#if notesTooShort}
                    <span class="field-hint invalid">
                        {$LL.lotMovements.archive.notesMinHint({ min: ARCHIVE_NOTES_MIN_CHARS })}
                    </span>
                {:else if notesTooLong}
                    <span class="field-hint invalid">
                        {$LL.lotMovements.archive.notesMaxHint({ max: ARCHIVE_NOTES_MAX_CHARS })}
                    </span>
                {/if}
            </label>

            <div class="form-actions">
                <button
                    type="submit"
                    class="btn-primary"
                    disabled={!canSubmit}
                >
                    {submitting ? $LL.lotMovements.archive.archiving() : $LL.lotMovements.archive.archiveLot()}
                </button>
                <button
                    type="button"
                    class="btn-secondary"
                    on:click={handleClose}
                    disabled={submitting}
                >
                    {$LL.lotMovements.modal.cancel()}
                </button>
            </div>
        </form>
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

    .btn-close:hover:not(:disabled) {
        background: #f3f4f6;
        color: #374151;
    }

    .btn-close:disabled {
        opacity: 0.5;
        cursor: not-allowed;
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
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .batch {
        background: #e5e7eb;
        border-radius: 4px;
        padding: 1px 5px;
        font-size: 0.75rem;
        margin-right: 4px;
    }

    .hint {
        margin: 0;
        color: #6b7280;
        font-size: 0.82rem;
        font-style: italic;
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
        position: relative;
    }

    label textarea,
    label select {
        padding: 7px 10px;
        border: 1px solid #d1d5db;
        border-radius: 6px;
        font-size: 0.9rem;
        font-family: inherit;
    }

    label textarea:focus,
    label select:focus {
        outline: 2px solid #3b82f6;
        border-color: #3b82f6;
    }

    .char-counter {
        position: absolute;
        right: 4px;
        top: -2px;
        font-size: 0.74rem;
        color: #9ca3af;
        background: transparent;
        pointer-events: none;
    }

    .char-counter .invalid {
        color: #991b1b;
    }

    .field-hint {
        font-size: 0.78rem;
        color: #6b7280;
    }

    .field-hint.invalid {
        color: #991b1b;
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
</style>
