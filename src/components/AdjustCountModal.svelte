<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let lotQuantity: number;
  export let lotUnit: string;
  export let currentBalances: LotLocationBalance[];
  /**
   * Unit kind resolved from the lot's product catalog link.
   * `null` for legacy/uncatalogued products — treated as decimal.
   */
  export let unitType: UnitKind | null = null;
  export let onClose: () => void;
  export let onCreated: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  let locationId = "";
  let realQuantity = 0;
  let notes = "";
  let submitting = false;
  let errorMsg = "";

  $: currentBalance = locationId
    ? currentBalances.find((b) => b.location_id === locationId)?.balance ?? 0
    : 0;

  $: delta = realQuantity - currentBalance;
  $: isIncrease = delta > 0;
  $: isDecrease = delta < 0;
  $: isNoOp = delta === 0;

  // ── Unit-aware quantity input rules ───────────────────────────────────────
  $: isIntegerUnit = unitType === "integer";
  $: qtyMin = isIntegerUnit ? 0 : 0.01;
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
    if (qty < 0) return false;
    return !Number.isInteger(qty);
  }

  // ── Submit ────────────────────────────────────────────────────────────────

  async function submit() {
    errorMsg = "";

    if (!locationId) {
      errorMsg = $LL.lotMovements.modal.selectLocationError();
      return;
    }
    if (realQuantity < 0) {
      errorMsg = $LL.lotMovements.modal.quantityNonNegativeError();
      return;
    }
    if (isFractionalForIntegerUnit(realQuantity)) {
      errorMsg = $LL.lotMovements.modal.integerQuantityError({ quantity: realQuantity });
      return;
    }
    if (!notes.trim()) {
      errorMsg = $LL.lotMovements.modal.adjustNotesRequiredError();
      return;
    }

    // No-op: nothing to adjust
    if (isNoOp) {
      errorMsg = $LL.lotMovements.modal.noAdjustmentError();
      return;
    }

    submitting = true;
    try {
      await createLotMovement({
        lot_id: lotId,
        kind: "inventory_adjustment",
        direction: isIncrease ? "increase" : "decrease",
        quantity: Math.abs(delta),
        source_location_id: isDecrease ? locationId : null,
        destination_location_id: isIncrease ? locationId : null,
        notes: notes.trim(),
      }, locale.current);
      onCreated();
    } catch (e) {
      errorMsg = humanizeError(e);
      submitting = false;
    }
  }
</script>

<div class="modal-overlay" role="dialog" aria-modal="true" aria-label={$LL.lotMovements.adjustCount()}>
  <div class="modal-box">
    <div class="modal-header">
      <h3>{$LL.lotMovements.adjustCount()}</h3>
      <button class="modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="modal-body">
      <div class="info-box">
        <p>
          {$LL.lotMovements.modal.adjustCurrentLotInventory({ quantity: lotQuantity, unit: lotUnit })}
        </p>
        <p class="info-hint">
          {$LL.lotMovements.modal.adjustInstruction()}
        </p>
      </div>

      <div class="form-group">
        <label for="adjust-location">{$LL.lotsDetail.location()}</label>
        <select id="adjust-location" bind:value={locationId} disabled={submitting}>
          <option value="">{$LL.lotMovements.modal.selectLocation()}</option>
          {#each currentBalances as bal}
            <option value={bal.location_id}>
              {bal.location_id} — {$LL.lotMovements.modal.currentBalanceOption({ balance: bal.balance })}
            </option>
          {/each}
        </select>
      </div>

      <div class="form-group">
        <label for="adjust-real">{$LL.lotMovements.quantityToSet()}</label>
        <input
          id="adjust-real"
          type="number"
          min={qtyMin}
          step={qtyStep}
          inputmode={qtyInputMode}
          bind:value={realQuantity}
          disabled={submitting}
        />
        <span class="hint">{$LL.lotMovements.currentInventory({ current: currentBalance })}{isIntegerUnit ? $LL.lotMovements.integerNote() : ""}</span>
      </div>

      {#if locationId && realQuantity >= 0}
        <div class="delta-preview" class:positive={isIncrease} class:negative={isDecrease} class:no-change={isNoOp}>
          {#if isIncrease}
            <span class="delta-sign">+</span>
            <span class="delta-value">{delta} {lotUnit}</span>
            <span class="delta-label">{$LL.lotMovements.willIncrease()}</span>
          {:else if isDecrease}
            <span class="delta-sign">−</span>
            <span class="delta-value">{Math.abs(delta)} {lotUnit}</span>
            <span class="delta-label">{$LL.lotMovements.willDecrease()}</span>
          {:else}
            <span class="delta-label no-change">{$LL.lotMovements.modal.noAdjustment()}</span>
          {/if}
        </div>
      {/if}

      <div class="form-group">
        <label for="adjust-notes">{$LL.lotMovements.modal.notesRequiredReason()}</label>
        <textarea
          id="adjust-notes"
          rows="3"
          bind:value={notes}
          disabled={submitting}
          placeholder={$LL.lotMovements.modal.adjustNotesPlaceholder()}
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
        disabled={submitting || isNoOp}
      >
        {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.adjustSubmit()}
      </button>
    </div>
  </div>
</div>

<style>
  .info-box {
    background: #f0f9ff;
    border: 1px solid #bae6fd;
    border-radius: 6px;
    padding: 10px 12px;
    margin-bottom: 16px;
  }

  .info-box p {
    margin: 0 0 4px;
    font-size: 0.88rem;
  }

  .info-hint {
    color: #0369a1;
    font-size: 0.82rem !important;
  }

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

  .delta-preview {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 6px;
    margin-bottom: 14px;
    font-size: 0.9rem;
  }

  .delta-preview.positive {
    background: #dcfce7;
    border: 1px solid #86efac;
    color: #166534;
  }

  .delta-preview.negative {
    background: #fee2e2;
    border: 1px solid #fca5a5;
    color: #991b1b;
  }

  .delta-preview.no-change {
    background: #f3f4f6;
    border: 1px solid #e5e7eb;
    color: #6b7280;
  }

  .delta-sign {
    font-size: 1.2rem;
    font-weight: 700;
  }

  .delta-value {
    font-weight: 600;
  }

  .delta-label {
    color: inherit;
    opacity: 0.8;
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
