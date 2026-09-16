<script lang="ts">
  import { onMount } from "svelte";
  import {
    listLotMovements,
    getLotLocationBalances,
    formatMovementQuantity,
    getKindLabel,
    type LotMovementResponse,
    type LotLocationBalance,
  } from "../lib/lot_movements.js";
  import MoveStockModal from "./MoveStockModal.svelte";
  import RegisterExitModal from "./RegisterExitModal.svelte";
  import AdjustCountModal from "./AdjustCountModal.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let lotId: string;
  export let lotQuantity: number;
  export let lotUnit: string;
  export let lotStatus: string;
  export let locations: { id: string; name: string; store_id: string }[] = [];
  export let allLocations: { id: string; name: string; store_id: string; store_name?: string }[] = [];
  export let onMovementCreated: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  let movements: LotMovementResponse[] = [];
  let balances: LotLocationBalance[] = [];
  let loading = true;
  let errorMsg = "";

  // Modal states
  let showMoveStock = false;
  let showRegisterExit = false;
  let showAdjustCount = false;

  // ── Load ───────────────────────────────────────────────────────────────────

  async function load() {
    loading = true;
    errorMsg = "";
    try {
      [movements, balances] = await Promise.all([
        listLotMovements(lotId),
        getLotLocationBalances(lotId),
      ]);
    } catch (e) {
      errorMsg = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  // Reload when lotId changes
  $: if (lotId) {
    load();
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  function formatDate(dateStr: string): string {
    const d = new Date(dateStr);
    return d.toLocaleString("es-MX", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function getLocationName(id: string | null): string {
    if (!id) return "—";
    const loc = allLocations.find((l) => l.id === id);
    return loc?.name ?? id.slice(0, 8) + "…";
  }

  function getLocationBalance(id: string): number {
    const bal = balances.find((b) => b.location_id === id);
    return bal?.balance ?? 0;
  }

  function handleMovementCreated() {
    showMoveStock = false;
    showRegisterExit = false;
    showAdjustCount = false;
    onMovementCreated();
    load();
  }

  $: isActive = lotStatus === "active";
</script>

<div class="panel">
  <!-- ── Header: totals + actions ─────────────────────────────────────────── -->
  <div class="panel-header">
    <div class="totals">
      <div class="total-item">
        <span class="total-label">Total</span>
        <span class="total-value">{lotQuantity} {lotUnit}</span>
      </div>
      {#each balances as bal}
        {@const locName = getLocationName(bal.location_id)}
        {#if locName !== "—"}
          <div class="total-item">
            <span class="total-label">{locName}</span>
            <span class="total-value">{bal.balance} {lotUnit}</span>
          </div>
        {/if}
      {/each}
    </div>

    {#if isActive}
      <div class="panel-actions">
        <button
          type="button"
          class="btn-action btn-move"
          on:click={() => (showMoveStock = true)}
          title="Mover stock entre ubicaciones"
        >
          Mover stock
        </button>
        <button
          type="button"
          class="btn-action btn-exit"
          on:click={() => (showRegisterExit = true)}
          title="Registrar salida de stock"
        >
          Registrar salida
        </button>
        <button
          type="button"
          class="btn-action btn-adjust"
          on:click={() => (showAdjustCount = true)}
          title="Ajustar conteo de inventario"
        >
          Ajustar conteo
        </button>
      </div>
    {/if}
  </div>

  <!-- ── Movements list ────────────────────────────────────────────────────── -->
  {#if loading}
    <p class="loading">Cargando movimientos…</p>
  {:else if errorMsg}
    <div class="alert-error" role="alert">{errorMsg}</div>
  {:else if movements.length === 0}
    <p class="empty-hint">Sin movimientos registrados.</p>
  {:else}
    <ul class="movement-list">
      {#each movements as mov (mov.id)}
        {@const label = getKindLabel(mov.movement_kind, mov.direction)}
        {@const qty = formatMovementQuantity(mov.quantity, mov.movement_kind, mov.direction)}
        <li class="movement-item">
          <div class="movement-main">
            <span class="movement-kind">{label}</span>
            <span class="movement-qty">{qty}</span>
          </div>
          <div class="movement-meta">
            {#if mov.movement_kind === "transfer" || mov.movement_kind.startsWith("entry:") || mov.movement_kind === "inventory_adjustment"}
              <span class="movement-locations">
                {#if mov.source_location_id}
                  <span class="loc-badge">{getLocationName(mov.source_location_id)}</span>
                  →
                {/if}
                <span class="loc-badge">{getLocationName(mov.destination_location_id)}</span>
              </span>
            {:else}
              <span class="movement-locations">
                desde <span class="loc-badge">{getLocationName(mov.source_location_id)}</span>
              </span>
            {/if}
            <span class="movement-time">{formatDate(mov.created_at)}</span>
          </div>
          {#if mov.notes}
            <p class="movement-notes">{mov.notes}</p>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<!-- ── Modals ─────────────────────────────────────────────────────────────── -->
{#if showMoveStock}
  <MoveStockModal
    {lotId}
    {lotQuantity}
    {locations}
    {allLocations}
    currentBalances={balances}
    onClose={() => (showMoveStock = false)}
    onCreated={handleMovementCreated}
  />
{/if}

{#if showRegisterExit}
  <RegisterExitModal
    {lotId}
    currentBalances={balances}
    onClose={() => (showRegisterExit = false)}
    onCreated={handleMovementCreated}
  />
{/if}

{#if showAdjustCount}
  <AdjustCountModal
    {lotId}
    {lotQuantity}
    {lotUnit}
    currentBalances={balances}
    allLocations={allLocations}
    onClose={() => (showAdjustCount = false)}
    onCreated={handleMovementCreated}
  />
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .totals {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .total-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .total-label {
    font-size: 0.72rem;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .total-value {
    font-size: 0.95rem;
    font-weight: 600;
    color: #374151;
  }

  .panel-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .btn-action {
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.82rem;
    cursor: pointer;
    border: 1px solid;
    font-family: inherit;
    transition: background-color 0.15s;
  }

  .btn-move {
    background: #eff6ff;
    border-color: #bfdbfe;
    color: #1e40af;
  }

  .btn-move:hover {
    background: #dbeafe;
  }

  .btn-exit {
    background: #fef2f2;
    border-color: #fecaca;
    color: #991b1b;
  }

  .btn-exit:hover {
    background: #fee2e2;
  }

  .btn-adjust {
    background: #f0fdf4;
    border-color: #bbf7d0;
    color: #166534;
  }

  .btn-adjust:hover {
    background: #dcfce7;
  }

  .loading {
    color: #6b7280;
    font-style: italic;
    font-size: 0.85rem;
    margin: 0;
  }

  .empty-hint {
    color: #9ca3af;
    font-size: 0.85rem;
    font-style: italic;
    margin: 0;
  }

  .alert-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 0.85rem;
  }

  .movement-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .movement-item {
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 7px;
    padding: 10px 12px;
  }

  .movement-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .movement-kind {
    font-size: 0.88rem;
    font-weight: 500;
    color: #374151;
  }

  .movement-qty {
    font-size: 0.95rem;
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    color: #1f2937;
  }

  .movement-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    flex-wrap: wrap;
  }

  .movement-locations {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.78rem;
    color: #6b7280;
  }

  .loc-badge {
    background: #e5e7eb;
    color: #374151;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.74rem;
  }

  .movement-time {
    font-size: 0.74rem;
    color: #9ca3af;
  }

  .movement-notes {
    margin: 6px 0 0;
    font-size: 0.8rem;
    color: #4b5563;
    font-style: italic;
    white-space: pre-wrap;
  }
</style>
