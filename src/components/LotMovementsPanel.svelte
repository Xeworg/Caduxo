<script lang="ts">
  import { onMount } from "svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";
  import {
    listLotMovements,
    getLotLocationBalances,
    formatMovementQuantity,
    type LotMovementResponse,
    type LotLocationBalance,
  } from "../lib/lot_movements.js";
  import type { UnitKind } from "../lib/products.js";
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
  /**
   * Unit kind resolved from the lot's product catalog link. Propagated to
   * every modal so quantity inputs apply unit-aware min/step/validation.
   * `null` for legacy/uncatalogued products — treated as decimal.
   */
  export let unitType: UnitKind | null = null;
  export let onMovementCreated: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  let movements: LotMovementResponse[] = [];
  let balances: LotLocationBalance[] = [];
  let loading = true;
  let errorMsg = "";

  // Modal states. Only one action form should be open at a time.
  let showMoveStock = false;
  let showRegisterExit = false;
  let showAdjustCount = false;

  type ActionForm = "move" | "exit" | "adjust";

  function openActionForm(form: ActionForm) {
    showMoveStock = form === "move";
    showRegisterExit = form === "exit";
    showAdjustCount = form === "adjust";
  }

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
      errorMsg = humanizeError(e);
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
    return $LL.lotMovements.resolution.eventDateTime({ value: dateStr });
  }

  function getLocationName(id: string | null): string {
    if (!id) return "—";
    const loc = allLocations.find((l) => l.id === id);
    return loc?.name ?? id.slice(0, 8) + "…";
  }

  function getMovementKindLabel(kind: string, direction?: string | null): string {
    if (kind === "inventory_adjustment" && direction) {
      return direction === "increase"
        ? $LL.lotMovements.movementKinds.inventoryAdjustmentIncrease()
        : $LL.lotMovements.movementKinds.inventoryAdjustmentDecrease();
    }

    const movementKindLabels: Record<string, () => string> = {
      "entry:initial": $LL.lotMovements.movementKinds.initialEntry,
      transfer: $LL.lotMovements.movementKinds.transfer,
      "exit:sale": $LL.lotMovements.exitReasons.sale,
      "exit:waste": $LL.lotMovements.exitReasons.waste,
      "exit:expired": $LL.lotMovements.exitReasons.expired,
      "exit:damaged": $LL.lotMovements.exitReasons.damaged,
      "exit:internal_consumption": $LL.lotMovements.exitReasons.internalConsumption,
      "exit:return_to_supplier": $LL.lotMovements.exitReasons.returnToSupplier,
      "exit:inventory_adjustment": $LL.lotMovements.exitReasons.inventoryAdjustmentExit,
      "exit:other": $LL.lotMovements.exitReasons.other,
      inventory_adjustment: $LL.lotMovements.movementKinds.inventoryAdjustment,
    };

    return movementKindLabels[kind]?.() ?? kind;
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
        <span class="total-label">{$LL.lotMovements.total()}</span>
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
          on:click={() => openActionForm("move")}
          title={$LL.lotMovements.actionTitles.moveStock()}
        >
          {$LL.lotMovements.moveStock()}
        </button>
        <button
          type="button"
          class="btn-action btn-exit"
          on:click={() => openActionForm("exit")}
          title={$LL.lotMovements.actionTitles.registerExit()}
        >
          {$LL.lotMovements.registerExit()}
        </button>
        <button
          type="button"
          class="btn-action btn-adjust"
          on:click={() => openActionForm("adjust")}
          title={$LL.lotMovements.actionTitles.adjustCount()}
        >
          {$LL.lotMovements.adjustCount()}
        </button>
      </div>
    {/if}
  </div>

  <!-- ── Movements list ────────────────────────────────────────────────────── -->
  {#if loading}
    <p class="loading">{$LL.lotMovements.loadingMovements()}</p>
  {:else if errorMsg}
    <div class="alert-error" role="alert">{errorMsg}</div>
  {:else if movements.length === 0}
    <p class="empty-hint">{$LL.lotMovements.noMovements()}</p>
  {:else}
    <ul class="movement-list">
      {#each movements as mov (mov.id)}
        {@const label = getMovementKindLabel(mov.movement_kind, mov.direction)}
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
                {$LL.lotMovements.fromLocation()} <span class="loc-badge">{getLocationName(mov.source_location_id)}</span>
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
    {locations}
    {allLocations}
    currentBalances={balances}
    {unitType}
    onClose={() => (showMoveStock = false)}
    onCreated={handleMovementCreated}
  />
{/if}

{#if showRegisterExit}
  <RegisterExitModal
    {lotId}
    currentBalances={balances}
    {unitType}
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
    {unitType}
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
