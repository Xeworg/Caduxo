<!--
  MoveStockModal.svelte — Migrated to Modal.svelte primitive in
  caduxo-daisyui-redesign PR 7a. Shell markup replaced with the
  shared primitive; business state, validation, submit handlers,
  and visible copy preserved verbatim.

  Source / destination pickers now use the themed `Listbox`
  primitive instead of `<Select>`. The empty placeholder is folded
  into the option array as a disabled `value: ""` row (the themed
  Listbox has no `<option>` slot for an external placeholder).
-->
<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import type { UnitKind } from "../lib/products.js";
  import { humanizeError } from "../lib/errors.js";
  import Modal from "./ui/Modal.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Button from "./ui/Button.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let locations: { id: string; name: string; store_id: string }[];
  export let allLocations: { id: string; name: string; store_id: string; store_name?: string }[];
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
  let destinationLocationId = "";
  let quantity = 0;
  let transferAcrossStores = false;
  let submitting = false;
  let errorMsg = "";

  // ── Unit-aware quantity input rules ───────────────────────────────────────
  // Integer products (e.g. Unidad) reject fractional quantities at the backend;
  // we apply matching min/step/inputmode here so the browser input UX matches
  // the server invariant. Decimal and legacy (null) units accept any positive.
  $: isIntegerUnit = unitType === "integer";
  $: qtyMin = isIntegerUnit ? 1 : 0.01;
  $: qtyStep = isIntegerUnit ? 1 : 0.01;
  $: qtyInputMode = (isIntegerUnit ? "numeric" : "decimal") as
    | "numeric"
    | "decimal";

  // Initialize source to highest-balance location
  $: if (!sourceLocationId && currentBalances.length > 0) {
    const highest = [...currentBalances].sort((a, b) => b.balance - a.balance)[0];
    if (highest && highest.balance > 0) {
      sourceLocationId = highest.location_id;
    }
  }

  $: availableQuantity = sourceLocationId
    ? currentBalances.find((b) => b.location_id === sourceLocationId)?.balance ?? 0
    : 0;

  $: locationCatalog = allLocations.length > 0 ? allLocations : locations;
  $: sourceLocations = locationCatalog.filter((loc) =>
    currentBalances.some((bal) => bal.location_id === loc.id && bal.balance > 0),
  );
  $: sourceStoreId = locationCatalog.find((loc) => loc.id === sourceLocationId)?.store_id ?? null;
  $: filteredDestinations = locationCatalog.filter((loc) => {
    if (loc.id === sourceLocationId) return false;
    if (!sourceStoreId) return true;
    return transferAcrossStores ? loc.store_id !== sourceStoreId : loc.store_id === sourceStoreId;
  });
  $: if (destinationLocationId && !filteredDestinations.some((loc) => loc.id === destinationLocationId)) {
    destinationLocationId = "";
  }

  function locationLabel(loc: { name: string; store_name?: string }): string {
    return loc.store_name ? `${loc.store_name} / ${loc.name}` : loc.name;
  }

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
    if (!destinationLocationId) {
      errorMsg = $LL.lotMovements.modal.selectDestinationLocationError();
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

    submitting = true;
    try {
      await createLotMovement({
        lot_id: lotId,
        kind: "transfer",
        direction: null,
        quantity,
        source_location_id: sourceLocationId,
        destination_location_id: destinationLocationId,
        notes: null,
      }, locale.current);
      onCreated();
    } catch (e) {
      errorMsg = humanizeError(e);
      submitting = false;
    }
  }

  /**
   * Source location options for the themed `<Listbox>` primitive.
   * The empty placeholder is folded in as the first row
   * (`value: ""`, `disabled: true`) so the user can clear the
   * selection without losing the bound rune state — the previous
   * `<Select>` implementation surfaced the same placeholder via a
   * `leading` snippet, but the Listbox primitive has no `<option>`
   * slot for an external placeholder row.
   */
  $: sourceLocationOptions = [
    { value: "", label: $LL.lotMovements.modal.selectLocation(), disabled: true },
    ...sourceLocations.map((loc) => {
      const bal = currentBalances.find((b) => b.location_id === loc.id)?.balance ?? 0;
      return {
        value: loc.id,
        label: `${locationLabel(loc)} (${$LL.lotMovements.modal.availableOption({ balance: bal })})`,
      };
    }),
  ];

  /**
   * Destination location options for the themed `<Listbox>`
   * primitive. Same shape as the source list minus the balance
   * annotation, per the original shell; the empty placeholder is
   * folded in identically to the source list.
   */
  $: destinationLocationOptions = [
    { value: "", label: $LL.lotMovements.modal.selectLocation(), disabled: true },
    ...filteredDestinations.map((loc) => ({
      value: loc.id,
      label: locationLabel(loc),
    })),
  ];

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
  aria-label={$LL.lotMovements.moveStock()}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.lotMovements.moveStock()}</h3>
    </header>

    <div class="dialog-body">
      <label class="checkbox-row">
        <input
          type="checkbox"
          class="checkbox checkbox-primary checkbox-sm"
          bind:checked={transferAcrossStores}
          disabled={submitting || sourceLocations.length === 0}
        />
        {$LL.lotMovements.modal.moveAcrossStores()}
      </label>

      {#if sourceLocations.length === 0 || filteredDestinations.length === 0}
        <p class="info-text">
          {sourceLocations.length === 0
            ? $LL.lotMovements.modal.noSourceStock()
            : transferAcrossStores
              ? $LL.lotMovements.modal.noOtherStoreLocations()
              : $LL.lotMovements.modal.noSameStoreLocations()}
        </p>
      {:else}
        <div class="form-group">
          <label for="move-source">{$LL.lotMovements.modal.sourceLocation()}</label>
          <Listbox
            id="move-source"
            bind:value={sourceLocationId}
            options={sourceLocationOptions}
            disabled={submitting}
            aria-label={$LL.lotMovements.modal.sourceLocation()}
          />
        </div>

        <div class="form-group">
          <label for="move-dest">{$LL.lotMovements.modal.destinationLocation()}</label>
          <Listbox
            id="move-dest"
            bind:value={destinationLocationId}
            options={destinationLocationOptions}
            disabled={submitting}
            aria-label={$LL.lotMovements.modal.destinationLocation()}
          />
        </div>

        <div class="form-group">
          <label for="move-qty">{$LL.lotMovements.modal.quantity()}</label>
          <input
            id="move-qty"
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

        {#if errorMsg}
          <div class="alert-error" role="alert">{errorMsg}</div>
        {/if}
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
      disabled={submitting || sourceLocations.length === 0 || filteredDestinations.length === 0}
      loading={submitting}
    >
      {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.moveSubmit()}
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

  .info-text {
    color: var(--color-base-content);
    opacity: 0.7;
    font-size: 0.88rem;
    margin: 0;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 14px;
    font-size: 0.86rem;
    color: var(--color-base-content);
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