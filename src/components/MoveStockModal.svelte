<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import type { UnitKind } from "../lib/products.js";

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
      });
      onCreated();
    } catch (e) {
      errorMsg = String(e);
      submitting = false;
    }
  }
</script>

<div class="modal-overlay" role="dialog" aria-modal="true" aria-label={$LL.lotMovements.moveStock()}>
  <div class="modal-box">
    <div class="modal-header">
      <h3>{$LL.lotMovements.moveStock()}</h3>
      <button class="modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="modal-body">
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={transferAcrossStores} disabled={submitting || sourceLocations.length === 0} />
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
          <select id="move-source" bind:value={sourceLocationId} disabled={submitting}>
            <option value="">{$LL.lotMovements.modal.selectLocation()}</option>
            {#each sourceLocations as loc}
              {@const bal = currentBalances.find((b) => b.location_id === loc.id)?.balance ?? 0}
              <option value={loc.id}>{locationLabel(loc)} ({$LL.lotMovements.modal.availableOption({ balance: bal })})</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label for="move-dest">{$LL.lotMovements.modal.destinationLocation()}</label>
          <select id="move-dest" bind:value={destinationLocationId} disabled={submitting}>
            <option value="">{$LL.lotMovements.modal.selectLocation()}</option>
            {#each filteredDestinations as loc}
              <option value={loc.id}>{locationLabel(loc)}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label for="move-qty">{$LL.lotMovements.modal.quantity()}</label>
          <input
            id="move-qty"
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

        {#if errorMsg}
          <div class="alert-error" role="alert">{errorMsg}</div>
        {/if}
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
        disabled={submitting || sourceLocations.length === 0 || filteredDestinations.length === 0}
      >
        {submitting ? $LL.lotMovements.modal.saving() : $LL.lotMovements.modal.moveSubmit()}
      </button>
    </div>
  </div>
</div>

<style>
  .info-text {
    color: #6b7280;
    font-size: 0.88rem;
    margin: 0;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 14px;
    font-size: 0.86rem;
    color: #374151;
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
  .form-group input {
    padding: 8px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
  }

  .form-group select:focus,
  .form-group input:focus {
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
