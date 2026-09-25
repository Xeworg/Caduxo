<!--
  RegisterExitModal.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7b. Shell markup replaced with the
  shared primitive; the source-location picker and the motivo
  picker both render through the themed `Listbox` primitive (no
  native `<select>` so OS black dropdowns cannot leak through).
  Quantity input and notes textarea stay inline (textareas and
  number spinners are out of scope for the themed primitives).
  Business state, validation, submit handlers, and visible copy
  preserved verbatim.
-->
<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
import {
  EXIT_KINDS,
  exitRequiresNotes,
  isFractionalForIntegerUnit,
  qtyAttrs,
  type ExitKind,
} from "../lib/movementRules.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";
  import Modal from "./ui/Modal.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Button from "./ui/Button.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let currentBalances: LotLocationBalance[];
  /**
   * All active store locations (all stores) — resolved to human-readable names
   * in the source-location picker instead of raw location IDs.
   */
  export let allLocations: { id: string; name: string; store_id: string; store_name?: string }[] = [];
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

  $: requiresNotes = exitRequiresNotes(exitReason);
  $: availableQuantity = sourceLocationId
    ? currentBalances.find((b) => b.location_id === sourceLocationId)?.balance ?? 0
    : 0;

  $: isIntegerUnit = unitType === "integer";
  $: _qtyAttrs = qtyAttrs(isIntegerUnit);

  /**
   * Motivo options for the themed `<Listbox>` primitive. The value
   * is the exit-reason code; the label is the i18n string for that
   * reason (verbatim from the original shell). The empty
   * placeholder is folded in as a disabled `value: ""` row — the
   * themed Listbox has no `<option>` slot for an external
   * placeholder row, so the previous `<Select>` `leading` snippet
   * moved into the options array.
   */
  /**
   * Resolves an exit-kind to its i18n display label.
   * Exhaustive switch — mirrors `ScannerPage.svelte` and avoids the
   * runtime key-computation bug where
   * `"exit:inventory_adjustment"` would resolve to the nonexistent
   * `inventoryAdjustment` instead of `inventoryAdjustmentExit`.
   */
  function exitReasonLabel(kind: ExitKind): string {
    switch (kind) {
      case "exit:sale":                return $LL.lotMovements.exitReasons.sale();
      case "exit:waste":               return $LL.lotMovements.exitReasons.waste();
      case "exit:expired":             return $LL.lotMovements.exitReasons.expired();
      case "exit:damaged":             return $LL.lotMovements.exitReasons.damaged();
      case "exit:internal_consumption": return $LL.lotMovements.exitReasons.internalConsumption();
      case "exit:return_to_supplier":  return $LL.lotMovements.exitReasons.returnToSupplier();
      case "exit:inventory_adjustment": return $LL.lotMovements.exitReasons.inventoryAdjustmentExit();
      case "exit:other":              return $LL.lotMovements.exitReasons.other();
    }
  }

  $: exitReasonOptions = [
    { value: "", label: $LL.lotMovements.modal.selectExitReason(), disabled: true },
    ...EXIT_KINDS.map((kind) => ({ value: kind, label: exitReasonLabel(kind) })),
  ];

  /**
   * Source-location options for the `<Listbox>` primitive. Filters
   * balances to those with stock available and labels each entry
   * with the resolved location name (from `allLocations`) and the
   * available-balance badge. When a location id is not found in
   * `allLocations` the raw id is used as a fallback so the row
   * stays inspectable rather than disappearing silently.
   */
  $: sourceLocationOptions = [
    { value: "", label: $LL.lotMovements.modal.selectLocation(), disabled: true },
    ...currentBalances
      .filter((bal) => bal.balance > 0)
      .map((bal) => {
        const resolved = allLocations.find((l) => l.id === bal.location_id);
        const displayName = resolved?.name ?? bal.location_id;
        return {
          value: bal.location_id,
          label: `${displayName} (${$LL.lotMovements.modal.availableOption({ balance: bal.balance })})`,
        };
      }),
  ];



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
    if (isFractionalForIntegerUnit(quantity, isIntegerUnit)) {
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
        <Listbox
          id="exit-source"
          value={sourceLocationId}
          options={sourceLocationOptions}
          disabled={submitting}
          aria-label={$LL.lotMovements.modal.sourceLocation()}
          onchange={(v) => (sourceLocationId = v)}
        />
      </div>

      <div class="form-group">
        <label for="exit-reason">{$LL.lotMovements.modal.exitReason()}</label>
        <Listbox
          id="exit-reason"
          bind:value={exitReason}
          options={exitReasonOptions}
          disabled={submitting}
          aria-label={$LL.lotMovements.modal.exitReason()}
        />
      </div>

      <div class="form-group">
        <label for="exit-qty">{$LL.lotMovements.modal.quantity()}</label>
        <input
          id="exit-qty"
          type="number"
          class="input input-md w-full motion-reduce:transition-none"
          min={_qtyAttrs.min}
          step={_qtyAttrs.step}
          inputmode={_qtyAttrs.inputmode}
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
    color: var(--color-base-content);
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
    color: var(--color-base-content);
    font-weight: 500;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--color-base-content);
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
