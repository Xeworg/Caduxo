<!--
  ArchiveLotDialog.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7a. Shell markup replaced with the
  shared primitive; the confirmation button now uses
  Button.svelte variant="danger" per the spec. Business state,
  validation, submit handlers, and visible copy preserved verbatim.
-->
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
    import Modal from "./ui/Modal.svelte";
    import Button from "./ui/Button.svelte";
    import Listbox from "./ui/Listbox.svelte";

    // ── Props ──────────────────────────────────────────────────────────────────

    /** The lot to archive. */
    export let lot: ExpiryLotResponse;
    /** Called after a successful archive. */
    export let onArchived: () => void;
    /** Called when the user dismisses the dialog without archiving. */
    export let onClose: () => void;

    // ── Local state ────────────────────────────────────────────────────────────

    /** Backs the `<Modal>` primitive via two-way binding. */
    let visible = true;
    let returnFocusTo: HTMLElement | null = null;

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

    /**
     * Archive reason options for the native `<select>`. The value
     * is the reason code; the label is the i18n string for that
     * reason (verbatim from the original shell).
     */
    $: archiveReasonOptions = ARCHIVE_REASONS.map((option) => ({
        value: option.value,
        label: archiveReasonLabel(option.value),
    }));

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

    function handleCancel() {
        if (submitting) return;
        onClose();
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

<Modal
    bind:open={visible}
    size="md"
    showClose
    closeLabel={$LL.lotMovements.modal.close()}
    {returnFocusTo}
    oncancel={handleCancel}
    onclose={handleClose}
    titleId="archive-title"
>
    {#snippet children()}
        <header class="dialog-header">
            <h3 id="archive-title">{$LL.lotMovements.archive.title()}</h3>
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

        <form
            onsubmit={(e) => {
                e.preventDefault();
                submit();
            }}
            class="archive-form"
        >
            <label>
                {$LL.lotMovements.archive.reason()}
                <Listbox
                    bind:value={reason}
                    options={archiveReasonOptions}
                    required
                    disabled={submitting}
                    aria-label={$LL.lotMovements.archive.reason()}
                />
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
                    class="textarea w-full motion-reduce:transition-none"
                    bind:value={notes}
                    placeholder={$LL.lotMovements.archive.notesPlaceholder()}
                    rows="3"
                    minlength={ARCHIVE_NOTES_MIN_CHARS}
                    maxlength={ARCHIVE_NOTES_MAX_CHARS + 200}
                    required
                    disabled={submitting}
                    aria-label={$LL.lotMovements.archive.notes()}
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
        </form>
    {/snippet}

    {#snippet footer()}
        <Button
            variant="danger"
            onclick={submit}
            disabled={!canSubmit}
            loading={submitting}
        >
            {submitting ? $LL.lotMovements.archive.archiving() : $LL.lotMovements.archive.archiveLot()}
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
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin-bottom: 14px;
    }

    .batch {
        background: var(--color-base-300);
        border-radius: 4px;
        padding: 1px 5px;
        font-size: 0.75rem;
        margin-right: 4px;
    }

    .hint {
        margin: 0;
        color: var(--color-base-content);
        opacity: 0.7;
        font-size: 0.82rem;
        font-style: italic;
    }

    /* ── Form ──────────────────────────────────────────────────────────────── */
    .archive-form {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    label {
        display: flex;
        flex-direction: column;
        gap: 4px;
        font-size: 0.85rem;
        color: var(--color-base-content);
        position: relative;
    }

    .char-counter {
        position: absolute;
        right: 4px;
        top: -2px;
        font-size: 0.74rem;
        color: var(--color-base-content);
        opacity: 0.7;
        background: transparent;
        pointer-events: none;
    }

    .char-counter .invalid {
        color: var(--color-error);
        opacity: 1;
    }

    .field-hint {
        font-size: 0.78rem;
        color: var(--color-base-content);
        opacity: 0.7;
    }

    .field-hint.invalid {
        color: var(--color-error);
        opacity: 1;
    }
</style>