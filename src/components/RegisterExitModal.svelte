<!--
  RegisterExitModal.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7b. Shell markup replaced with the
  shared primitive; the motivo select now uses Select.svelte per
  the spec. Source-location select, quantity input, and notes
  textarea stay inline (the source select is a domain list not in
  scope; textareas and number spinners are out of scope for
  Input.svelte / Select.svelte). Business state, validation,
  submit handlers, and visible copy preserved verbatim.
-->
<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";
  import Modal from "./ui/Modal.svelte";
  import Select from "./ui/Select.svelte";
  import Button from "./ui/Button.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let currentBalances: LotLocationBalance[];
  /**
   * Unit kind resolved from the lot's product catalog link.
   * `null` for legacy/uncatalogued products — treated as decimal.
   */
  export let unitType: UnitKind | null = null;
  export let onClose: () => void;
  export let onCreated: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  /** Backs the `<Modal>` primitive via two-way binding. */
  let visible = true;
  /** Element to receive focus when the modal closes (the trigger row). */
  let returnFocusTo: HTMLElement | null = null;

  let sourceLocationId = "";
  let exitReason = "";
  let quantity = 0;
  let notes = "";
  let submitting = false;
  let errorMsg = "";

  // Exit reasons (the eight kinds from the spec)
  const EXIT_REASONS = [
    { value: "exit:sale", label: () => $LL.lotMovements.exitReasons.sale() },
    { value: "exit:waste", label: () => $LL.lotMovements.exitReasons.waste() },
    { value: "exit:expired", label: () => $LL.lotMovements.exitReasons.expired() },
    { value: "exit:damaged", label: () => $LL.lotMovements.exitReasons.damaged() },
    { value: "exit:internal_consumption", label: () => $LL.lotMovements.exitReasons.internalConsumption() },
    { value: "exit:return_to_supplier", label: () => $LL.lotMovements.exitReasons.returnToSupplier() },
    { value: "exit:inventory_adjustment", label: () => $LL.lotMovements.exitReasons.inventoryAdjustmentExit() },
    { value: "exit:other", label: () => $LL.lotMovements.exitReasons.other() },
  ];

  // Reasons that require notes
  const REQUIRES_NOTES = ["exit:inventory_adjustment", "exit:other"];

  $: requiresNotes = REQUIRES_NOTES.includes(exitReason);
  $: availableQuantity = sourceLocationId
    ? currentBalances.find((b) => b.location_id === sourceLocationId)?.balance ?? 0
    : 0;

  // ── Unit-aware quantity input rules ───────────────────────────────────────
  $: isIntegerUnit = unitType === "integer";
  $: qtyMin = isIntegerUnit ? 1 : 0.01;
  $: qtyStep = isIntegerUnit ? 1 : 0.01;
  $: qtyInputMode = (isIntegerUnit ? "numeric" : "decimal") as
    | "numeric"
    | "decimal";

  /**
   * Motivo options for the `<Select>` primitive. The value is the
   * exit-reason code; the label is the i18n string for that reason
   * (verbatim from the original shell).
   */
  $: exitReasonOptions = EXIT_REASONS.map((reason) => ({
    value: reason.value,
    label: reason.label(),
  }));

  /**
   * Local validation for integer-unit products: catches fractional input
   * before submit so the user gets immediate feedback. Backend enforces the
   * same invariant, but this avoids a round-trip for the common case.
   */
  function isFractionalForIntegerUnit(qty: number): boolean {
    if (!isIntegerUnit) return false;
    if (qty <= 0) return false;
    return !Number.isInteger(qty);
  }

  // ── Submit ────────────────────────────────────────────────────────────────

  async function submit() {
    errorMsg = "";

    if (!sourceLocationId) {
      errorMsg = $LL.lotMovements.modal.selectSourceLocationError();
      return;
    }
    if (!exitReason) {
      errorMsg = $LL.lotMovements.modal.selectExitReasonError();
      return;
    }
    if (quantity <= 0) {
      errorMsg = $LL.lotMovements.modal.quantityPositiveError();
      return;
    }
    if (isFractionalForIntegerUnit(quantity)) {
      errorMsg = $LL.lotMovements.modal.integerQuantityError({ quantity });
      return;
    }
    if (quantity > availableQuantity) {
      errorMsg = $LL.lotMovements.modal.availableQuantityError({ available: availableQuantity });
      return;
    }
    if (requiresNotes && !notes.trim()) {
      errorMsg = $LL.lotMovements.modal.exitNotesRequiredError();
      return;
    }

    submitting = true;
    try {
      await createLotMovement({
        lot_id: lotId,
        kind: exitReason,
        direction: null,
        quantity,
        source_location_id: sourceLocationId,
        destination_location_id: null,
        notes: notes.trim() || null,
      }, locale.current);
      onCreated();
    } catch (e) {
      errorMsg = humanizeError(e);
      submitting = false;
    }
  }

  /**
   * The modal's `oncancel` callback wires Escape to the consumer's
   * close handler so the "discard in-progress edits on Escape"
   * semantic is preserved verbatim.
   */
  function handleCancel() {
    if (submitting) return;
    onClose();
  }

  function handleClose() {
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
  aria-label={$LL.lotMovements.registerExit()}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.lotMovements.registerExit()}</h3>
    </header>

    <div class="dialog-body">
      <div class="form-group">
        <label for="exit-source">{$LL.lotMovements.modal.sourceLocation()}</label>
        <select
          id="exit-source"
          class="select select-md w-full motion-reduce:transition-none"
          bind:value={sourceLocationId}
          disabled={submitting}
          aria-label={$LL.lotMovements.modal.sourceLocation()}
        >
          <option value="">{$LL.lotMovements.modal.selectLocation()}</option>
          {#each currentBalances as bal}
            {#if bal.balance > 0}
              <option value={bal.location_id}>{bal.location_id} ({$LL.lotMovements.modal.availableOption({ balance: bal.balance })})</option>
            {/if}
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label for="exit-reason">{$LL.lotMovements.modal.exitReason()}</label>
        <Select
          id="exit-reason"
          bind:value={exitReason}
          options={exitReasonOptions}
          disabled={submitting}
          aria-label={$LL.lotMovements.modal.exitReason()}
        >
          {#snippet leading()}
            <option value="" disabled>
              {$LL.lotMovements.modal.selectExitReason()}
            </option>
          {/snippet}
        </Select>
      </div>

      <div class="form-group">
        <label for="exit-qty">{$LL.lotMovements.modal.quantity()}</label>
        <input
          id="exit-qty"
          type="number"
          class="input input-md w-full motion-reduce:transition-none"
          min={qtyMin}
          step={qtyStep}
          inputmode={qtyInputMode}
          max={availableQuantity}
          bind:value={quantity}
          disabled={submitting}
          aria-label={$LL.lotMovements.modal.quantity()}
        />
        <span class="hint">{$LL.lotMovements.available({ available: availableQuantity })}{isIntegerUnit ? $LL.lotMovements.integerNote() : ""}</span>
      </div>

      <div class="form-group">
        <label for="exit-notes">
          {requiresNotes ? $LL.lotMovements.modal.notesRequired() : $LL.lotMovements.modal.notesOptional()}
        </label>
        <textarea
          id="exit-notes"
          class="textarea w-full motion-reduce:transition-none"
          rows="3"
          bind:value={notes}
          disabled={submitting}
          placeholder={requiresNotes ? $LL.lotMovements.modal.notesRequiredPlaceholder() : $LL.lotMovements.modal.notesOptionalPlaceholder()}
        ></textarea>
      </div>

      {#if errorMsg}
        <div class="alert-error" role="alert">{errorMsg}</div>
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <Button
      variant="ghost"
      onclick={handleClose}
      disabled={submitting}
    >
      {$LL.lotMovements.modal.cancel()}
    </Button>
    <Button
      variant="primary"
      onclick={submit}
      disabled={submitting}
      loading={submitting}
    >
      {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.registerExitSubmit()}
    </Button>
  {/snippet}
</Modal>

<style>
  /* ── Header ────────────────────────────────────────────────────────────── */
  .dialog-header {
    display: flex;
    align-items: center;
    margin-bottom: 16px;
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--color-base-content, #0f172a);
  }

  /* ── Body ──────────────────────────────────────────────────────────────── */
  .dialog-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
  }

  .form-group label {
    font-size: 0.85rem;
    color: var(--color-base-content, #374151);
    font-weight: 500;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--color-base-content, #6b7280);
    opacity: 0.7;
  }

  .alert-error {
    background: color-mix(in oklch, var(--color-error) 12%, transparent);
    color: var(--color-error);
    border: 1px solid color-mix(in oklch, var(--color-error) 30%, transparent);
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.85rem;
    margin-bottom: 12px;
  }
</style>
