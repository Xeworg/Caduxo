<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";

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
</script>

<div class="modal-overlay" role="dialog" aria-modal="true" aria-label={$LL.lotMovements.registerExit()}>
  <div class="modal-box">
    <div class="modal-header">
      <h3>{$LL.lotMovements.registerExit()}</h3>
      <button class="modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="modal-body">
      <div class="form-group">
        <label for="exit-source">{$LL.lotMovements.modal.sourceLocation()}</label>
        <select id="exit-source" bind:value={sourceLocationId} disabled={submitting}>
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
        <select id="exit-reason" bind:value={exitReason} disabled={submitting}>
          <option value="">{$LL.lotMovements.modal.selectExitReason()}</option>
          {#each EXIT_REASONS as reason}
            <option value={reason.value}>{reason.label()}</option>
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label for="exit-qty">{$LL.lotMovements.modal.quantity()}</label>
        <input
          id="exit-qty"
          type="number"
          min={qtyMin}
          step={qtyStep}
          inputmode={qtyInputMode}
          max={availableQuantity}
          bind:value={quantity}
          disabled={submitting}
        />
        <span class="hint">{$LL.lotMovements.available({ available: availableQuantity })}{isIntegerUnit ? $LL.lotMovements.integerNote() : ""}</span>
      </div>

      <div class="form-group">
        <label for="exit-notes">
          {requiresNotes ? $LL.lotMovements.modal.notesRequired() : $LL.lotMovements.modal.notesOptional()}
        </label>
        <textarea
          id="exit-notes"
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

    <div class="modal-footer">
      <button type="button" class="btn-secondary" on:click={onClose} disabled={submitting}>
        {$LL.lotMovements.modal.cancel()}
      </button>
      <button
        type="button"
        class="btn-primary"
        on:click={submit}
        disabled={submitting}
      >
        {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.registerExitSubmit()}
      </button>
    </div>
  </div>
</div>

<style>
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
  }

  .form-group label {
    font-size: 0.85rem;
    color: #374151;
    font-weight: 500;
  }

  .form-group select,
  .form-group input,
  .form-group textarea {
    padding: 8px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
  }

  .form-group select:focus,
  .form-group input:focus,
  .form-group textarea:focus {
    outline: 2px solid #3b82f6;
    border-color: #3b82f6;
  }

  .hint {
    font-size: 0.78rem;
    color: #6b7280;
  }

  .alert-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.85rem;
    margin-bottom: 12px;
  }
</style>
