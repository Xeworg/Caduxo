<!--
  AdjustCountModal.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7a. Shell markup replaced with the
  shared primitive; the "real physical quantity" input uses
  `Input.svelte` and the location picker uses the themed
  `Listbox.svelte` primitive (no native `<select>` so OS black
  dropdowns cannot leak through). Business state, validation,
  submit handlers, and visible copy preserved verbatim.
-->
<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";
  import Modal from "./ui/Modal.svelte";
  import Input from "./ui/Input.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Button from "./ui/Button.svelte";

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

  /** Backs the `<Modal>` primitive via two-way binding. */
  let visible = true;
  let returnFocusTo: HTMLElement | null = null;

  let locationId = "";
  /**
   * Two-way bridge between `realQuantityAsString` (bound to the
   * `<Input>` primitive's `value: string` contract) and
   * `realQuantity: number` (consumed by the submit handler + the
   * delta preview). The HTML `<input type="number">` round-trips
   * through a string, so we keep both representations in sync.
   */
  let realQuantityAsString = "0";
  $: realQuantity =
    realQuantityAsString === "" || realQuantityAsString === "-"
      ? 0
      : Number(realQuantityAsString);

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

  // ── Unit-aware quantity validation ───────────────────────────────────────
  $: isIntegerUnit = unitType === "integer";

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

  /**
   * Location options for the themed `<Listbox>` primitive. The
   * value is the location id; the label is `id — current balance`
   * (verbatim from the original shell). The empty placeholder is
   * folded in as a disabled `value: ""` row — the themed Listbox
   * has no `<option>` slot for an external placeholder row, so the
   * previous `<Select>` `leading` snippet moved into the options
   * array.
   */
  $: locationOptions = [
    { value: "", label: $LL.lotMovements.modal.selectLocation(), disabled: true },
    ...currentBalances.map((bal) => ({
      value: bal.location_id,
      label: `${bal.location_id} — ${$LL.lotMovements.modal.currentBalanceOption({ balance: bal.balance })}`,
    })),
  ];

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
  aria-label={$LL.lotMovements.adjustCount()}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.lotMovements.adjustCount()}</h3>
    </header>

    <div class="dialog-body">
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
        <Listbox
          id="adjust-location"
          bind:value={locationId}
          options={locationOptions}
          disabled={submitting}
          aria-label={$LL.lotsDetail.location()}
        />
      </div>

      <div class="form-group">
        <Input
          id="adjust-real"
          type="number"
          label={$LL.lotMovements.quantityToSet()}
          bind:value={realQuantityAsString}
          invalid={false}
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
          class="textarea w-full motion-reduce:transition-none"
          rows="3"
          bind:value={notes}
          disabled={submitting}
          placeholder={$LL.lotMovements.modal.adjustNotesPlaceholder()}
          aria-label={$LL.lotMovements.modal.notesRequiredReason()}
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
      disabled={submitting || isNoOp}
      loading={submitting}
    >
      {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.adjustSubmit()}
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

  .info-box {
    background: color-mix(in oklch, var(--color-info) 8%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-info) 30%, transparent);
    border-radius: 6px;
    padding: 10px 12px;
    margin-bottom: 16px;
  }

  .info-box p {
    margin: 0 0 4px;
    font-size: 0.88rem;
  }

  .info-hint {
    color: color-mix(in oklch, var(--color-info) 90%, black);
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
    color: var(--color-base-content);
    font-weight: 500;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--color-base-content);
    opacity: 0.7;
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
    background: color-mix(in oklch, var(--color-success) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-success) 30%, transparent);
    color: var(--color-success);
  }

  .delta-preview.negative {
    background: color-mix(in oklch, var(--color-error) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-error) 30%, transparent);
    color: var(--color-error);
  }

  .delta-preview.no-change {
    background: var(--color-base-200);
    border: 1px solid var(--color-base-300);
    color: var(--color-base-content);
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
    background: color-mix(in oklch, var(--color-error) 12%, transparent);
    color: var(--color-error);
    border: 1px solid color-mix(in oklch, var(--color-error) 30%, transparent);
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.85rem;
    margin-bottom: 12px;
  }
</style>