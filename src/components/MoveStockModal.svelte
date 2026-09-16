<script lang="ts">
  import { createLotMovement, type LotLocationBalance } from "../lib/lot_movements.js";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let lotQuantity: number;
  export let locations: { id: string; name: string; store_id: string }[];
  export let allLocations: { id: string; name: string; store_id: string; store_name?: string }[];
  export let currentBalances: LotLocationBalance[];
  export let onClose: () => void;
  export let onCreated: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  let sourceLocationId = "";
  let destinationLocationId = "";
  let quantity = 0;
  let submitting = false;
  let errorMsg = "";

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

  $: filteredDestinations = locations.filter((l) => l.id !== sourceLocationId);

  // ── Submit ────────────────────────────────────────────────────────────────

  async function submit() {
    errorMsg = "";

    if (!sourceLocationId) {
      errorMsg = "Selecciona una ubicación de origen";
      return;
    }
    if (!destinationLocationId) {
      errorMsg = "Selecciona una ubicación de destino";
      return;
    }
    if (quantity <= 0) {
      errorMsg = "La cantidad debe ser mayor a 0";
      return;
    }
    if (quantity > availableQuantity) {
      errorMsg = `Solo hay ${availableQuantity} unidades disponibles en esta ubicación`;
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

<div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Mover stock">
  <div class="modal-box">
    <div class="modal-header">
      <h3>Mover stock</h3>
      <button class="modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="modal-body">
      {#if locations.length < 2}
        <p class="info-text">Se necesita al menos dos ubicaciones para realizar una transferencia.</p>
      {:else}
        <div class="form-group">
          <label for="move-source">Ubicación de origen *</label>
          <select id="move-source" bind:value={sourceLocationId} disabled={submitting}>
            <option value="">Seleccionar ubicación…</option>
            {#each locations as loc}
              {@const bal = currentBalances.find((b) => b.location_id === loc.id)?.balance ?? 0}
              <option value={loc.id}>{loc.name} ({bal} disponibles)</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label for="move-dest">Ubicación de destino *</label>
          <select id="move-dest" bind:value={destinationLocationId} disabled={submitting}>
            <option value="">Seleccionar ubicación…</option>
            {#each filteredDestinations as loc}
              <option value={loc.id}>{loc.name}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label for="move-qty">Cantidad *</label>
          <input
            id="move-qty"
            type="number"
            min="0.01"
            step="0.01"
            max={availableQuantity}
            bind:value={quantity}
            disabled={submitting}
          />
          <span class="hint">Disponibles: {availableQuantity}</span>
        </div>

        {#if errorMsg}
          <div class="alert-error" role="alert">{errorMsg}</div>
        {/if}
      {/if}
    </div>

    <div class="modal-footer">
      <button type="button" class="btn-secondary" on:click={onClose} disabled={submitting}>
        Cancelar
      </button>
      <button
        type="button"
        class="btn-primary"
        on:click={submit}
        disabled={submitting || locations.length < 2}
      >
        {submitting ? "Guardando…" : "Mover"}
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
