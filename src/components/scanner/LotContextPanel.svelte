<!--
  LotContextPanel.svelte — Scanner-native lot-context view (ODD task 5 of
  `feat/scanner-first-inventory`).

  A thin page-inline wrapper that surfaces lot identity, per-location
  balances, movement history, and canonical actions for the Scanner tab.
  This is NOT a modal and NOT a refactor of `LotMovementsPanel` — it
  owns its own loading/state and reuses the existing `MoveStockModal`,
  `RegisterExitModal`, `AdjustCountModal`, and movement helpers.

  Wired into `ScannerPage` from the resolved sale/stock-out lot state via
  an explicit "View lot context" affordance. The pinned context survives
  successful actions for immediate readback and is cleared on active-store
  change or when the user resolves a different/unknown item.

  Props:
    lot          — the pinned ExpiryLotResponse
    product      — the resolved product
    unitType     — "integer" | "decimal" | null, from the product catalog
    allLocations — all active store locations (all stores), for cross-store
                  transfer destination hydration in MoveStockModal
    onClose      — clears the pinned context in ScannerPage
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { LL } from "../../i18n/i18n-svelte.js";
  import { locale } from "../../i18n/locale.svelte.js";
  import { humanizeError } from "../../lib/errors.js";
  import {
    listLotMovements,
    getLotLocationBalances,
    formatMovementQuantity,
    type LotLocationBalance,
    type LotMovementResponse,
  } from "../../lib/lot_movements.js";
  import type { ExpiryLotResponse } from "../../lib/expiry_lots.js";
  import type { UnitKind } from "../../lib/products.js";
  import type { LocationRef } from "../../lib/lotDisplay.js";
  import { resolveLocationDisplay } from "../../lib/lotDisplay.js";
  import MoveStockModal from "../MoveStockModal.svelte";
  import RegisterExitModal from "../RegisterExitModal.svelte";
  import AdjustCountModal from "../AdjustCountModal.svelte";
  import LoadingState from "../ui/LoadingState.svelte";
  import EmptyState from "../ui/EmptyState.svelte";
  import Alert from "../ui/Alert.svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../ui/Icon.svelte";

  // ── Props ──────────────────────────────────────────────────────────────────

  interface Props {
    lot: ExpiryLotResponse;
    unitType?: UnitKind | null;
    /** All active store locations (all stores) — feeds MoveStockModal cross-store destinations. */
    allLocations?: { id: string; name: string; store_id: string; store_name?: string }[];
    onClose: () => void;
  }

  let {
    lot,
    unitType = null,
    allLocations = [],
    onClose,
  }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────────

  let movements: LotMovementResponse[] = $state([]);
  let balances: LotLocationBalance[] = $state([]);
  let loading = $state(true);
  let loadError = $state("");

  // Modal visibility
  let showMoveStock = $state(false);
  let showRegisterExit = $state(false);
  let showAdjustCount = $state(false);

  // Location name lookup built from `allLocations`.
  // The ledger renders location names in the Locations column; the
  // balance rows render the same names with the available count.
  let locationNameById = $state<Record<string, string>>({});

  // ── Derived ────────────────────────────────────────────────────────────────

  let locationLookup: LocationRef[] = $derived.by((): LocationRef[] => {
    return Object.entries(locationNameById).map(([id, name]) => ({ id, name }));
  });

  // ── Load ───────────────────────────────────────────────────────────────────

  async function load(): Promise<void> {
    loading = true;
    loadError = "";
    try {
      [movements, balances] = await Promise.all([
        listLotMovements(lot.id),
        getLotLocationBalances(lot.id),
      ]);
    } catch (e) {
      loadError = humanizeError(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  // Build the location name lookup whenever `allLocations` arrives.
  $effect(() => {
    const next: Record<string, string> = {};
    for (const loc of allLocations) {
      next[loc.id] = loc.name;
    }
    locationNameById = next;
  });

  // ── Helpers ────────────────────────────────────────────────────────────────

  function formatDate(dateStr: string): string {
    return $LL.lotMovements.resolution.eventDateTime({ value: dateStr });
  }

  function getMovementKindLabel(kind: string, direction?: string | null): string {
    if (kind === "inventory_adjustment" && direction) {
      return direction === "increase"
        ? $LL.lotMovements.movementKinds.inventoryAdjustmentIncrease()
        : $LL.lotMovements.movementKinds.inventoryAdjustmentDecrease();
    }
    const labels: Record<string, () => string> = {
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
    return labels[kind]?.() ?? kind;
  }

  function resolveLocationName(id: string | null): string {
    if (!id) return "—";
    return resolveLocationDisplay(id, locationLookup, $LL.common.noLocation());
  }

  // Total across all locations (ledger-derived total, not the lot snapshot).
  let totalBalance = $derived(
    balances.reduce((sum, b) => sum + b.balance, 0),
  );

  // ── Action handlers ────────────────────────────────────────────────────────

  function handleMovementCreated(): void {
    showMoveStock = false;
    showRegisterExit = false;
    showAdjustCount = false;
    void load();
  }

  function close(): void {
    showMoveStock = false;
    showRegisterExit = false;
    showAdjustCount = false;
    onClose();
  }
</script>

<aside class="lot-context-panel" aria-label={$LL.scanner.lotContext.ariaLabel()}>
  <!-- Header -->
  <div class="panel-header">
    <div class="panel-header-left">
      <h3 class="panel-title">{$LL.scanner.lotContext.title()}</h3>
      <button
        type="button"
        class="close-btn"
        aria-label={$LL.common.close()}
        onclick={close}
      >
        <Icon name="x-mark" />
      </button>
    </div>
  </div>

  <!-- Lot identity -->
  <div class="lot-identity">
    <p class="lot-line">
      <span class="lot-label">{$LL.dashboard.batch()}:</span>
      <span class="lot-value">{lot.batch_code ?? "—"}</span>
    </p>
    <p class="lot-line">
      <span class="lot-label">{$LL.dashboard.expiryDate()}:</span>
      <span class="lot-value">{lot.expiry_date}</span>
    </p>
    {#if lot.status && lot.status !== "active"}
      <p class="lot-line">
        <span class="lot-label">{$LL.dashboard.status()}:</span>
        <span class="lot-value status-badge">{lot.status}</span>
      </p>
    {/if}
    <p class="lot-line">
      <span class="lot-label">{$LL.lotMovements.total()}:</span>
      <span class="lot-value">
        {totalBalance}
        {#if lot.unit}
          <span class="lot-unit">{lot.unit}</span>
        {/if}
      </span>
    </p>
  </div>

  <!-- Balance table -->
  <section aria-labelledby="lot-context-balances-heading">
    <h4 class="section-heading" id="lot-context-balances-heading">
      {$LL.scanner.lotContext.balancesHeading()}
    </h4>

    {#if loading}
      <LoadingState variant="text" label={$LL.common.loading()} />
    {:else if balances.length === 0}
      <p class="empty-text">{$LL.scanner.lotContext.noBalances()}</p>
    {:else}
      <div class="balance-list" role="list">
        {#each balances as balance}
          {@const name = resolveLocationName(balance.location_id)}
          <div class="balance-row" role="listitem">
            <span class="balance-location">{name}</span>
            <span class="balance-qty" class:balance-zero={balance.balance === 0}>
              {balance.balance}
              {#if lot.unit}
                <span class="lot-unit">{lot.unit}</span>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Action buttons -->
  <section aria-label={$LL.scanner.lotContext.actionsLabel()}>
    <h4 class="section-heading">{$LL.common.actions()}</h4>
    <div class="action-row">
      <Button
        variant="secondary"
        size="sm"
        onclick={() => (showMoveStock = true)}
        disabled={loading}
      >
        <Icon name="arrows-right-left" />
        {$LL.lotMovements.moveStock()}
      </Button>
      <Button
        variant="secondary"
        size="sm"
        onclick={() => (showRegisterExit = true)}
        disabled={loading}
      >
        <Icon name="arrow-right-on-rectangle" />
        {$LL.lotMovements.registerExit()}
      </Button>
      <Button
        variant="secondary"
        size="sm"
        onclick={() => (showAdjustCount = true)}
        disabled={loading}
      >
        <Icon name="adjustments-horizontal" />
        {$LL.lotMovements.adjustCount()}
      </Button>
    </div>
  </section>

  <!-- Movement ledger -->
  <section aria-labelledby="lot-context-history-heading">
    <h4 class="section-heading" id="lot-context-history-heading">
      {$LL.lotMovements.history()}
    </h4>

    {#if loadError}
      <Alert variant="error">{loadError}</Alert>
    {:else if loading}
      <LoadingState variant="text" label={$LL.common.loading()} />
    {:else if movements.length === 0}
      <EmptyState
        icon="info"
        title={$LL.lotMovements.noMovements()}
        body=""
      />
    {:else}
      <div class="movement-log" role="list">
        {#each movements as movement}
          {@const qtyStr = formatMovementQuantity(
            movement.quantity,
            movement.movement_kind,
            movement.direction,
          )}
          <div class="movement-row" role="listitem">
            <div class="movement-kind">
              {getMovementKindLabel(movement.movement_kind, movement.direction)}
            </div>
            <div class="movement-locations">
              <span class="movement-loc">{resolveLocationName(movement.source_location_id)}</span>
              {#if movement.destination_location_id}
                <span class="movement-arrow">→</span>
                <span class="movement-loc">{resolveLocationName(movement.destination_location_id)}</span>
              {/if}
            </div>
            <div class="movement-qty" class:qty-positive={movement.direction === "increase"} class:qty-negative={movement.direction === "decrease"}>
              {qtyStr}
            </div>
            <div class="movement-time">{formatDate(movement.created_at)}</div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</aside>

<!-- Reused modals — receive all locations so cross-store destinations hydrate correctly -->
{#if showMoveStock}
  <MoveStockModal
    lotId={lot.id}
    locations={allLocations}
    allLocations={allLocations}
    currentBalances={balances}
    {unitType}
    onClose={() => (showMoveStock = false)}
    onCreated={handleMovementCreated}
  />
{/if}

{#if showRegisterExit}
  <RegisterExitModal
    lotId={lot.id}
    currentBalances={balances}
    {unitType}
    onClose={() => (showRegisterExit = false)}
    onCreated={handleMovementCreated}
  />
{/if}

{#if showAdjustCount}
  <AdjustCountModal
    lotId={lot.id}
    lotQuantity={lot.quantity}
    lotUnit={lot.unit ?? ""}
    currentBalances={balances}
    {unitType}
    onClose={() => (showAdjustCount = false)}
    onCreated={handleMovementCreated}
  />
{/if}

<style>
  .lot-context-panel {
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 10px;
    background: color-mix(in oklch, var(--color-base-100) 80%, transparent);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* ── Header ──────────────────────────────────────────────────────────────── */
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-header-left {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
  }

  .panel-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--color-base-content);
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--color-base-content);
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    opacity: 0.65;
    transition: opacity 0.15s;
  }

  .close-btn:hover {
    opacity: 1;
    background: color-mix(in oklch, var(--color-base-content) 10%, transparent);
  }

  /* ── Lot identity ───────────────────────────────────────────────────────── */
  .lot-identity {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    background: color-mix(in oklch, var(--color-base-200) 60%, transparent);
    border-radius: 6px;
  }

  .lot-line {
    margin: 0;
    font-size: 0.86rem;
    display: flex;
    gap: 6px;
    align-items: baseline;
  }

  .lot-label {
    font-weight: 500;
    color: var(--color-base-content);
    opacity: 0.7;
    min-width: 70px;
    flex-shrink: 0;
  }

  .lot-value {
    color: var(--color-base-content);
    font-weight: 500;
  }

  .lot-unit {
    font-size: 0.8em;
    opacity: 0.75;
    margin-left: 2px;
  }

  .status-badge {
    font-size: 0.8rem;
    text-transform: capitalize;
    background: color-mix(in oklch, var(--color-base-300) 60%, transparent);
    padding: 1px 6px;
    border-radius: 4px;
  }

  /* ── Section headings ───────────────────────────────────────────────────── */
  .section-heading {
    margin: 0 0 8px;
    font-size: 0.82rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-base-content);
    opacity: 0.6;
  }

  /* ── Balance list ──────────────────────────────────────────────────────── */
  .balance-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .balance-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 5px 10px;
    border-radius: 5px;
    background: color-mix(in oklch, var(--color-base-200) 50%, transparent);
    font-size: 0.86rem;
  }

  .balance-location {
    color: var(--color-base-content);
  }

  .balance-qty {
    font-weight: 600;
    color: var(--color-base-content);
    font-variant-numeric: tabular-nums;
  }

  .balance-zero {
    opacity: 0.45;
  }

  /* ── Action row ────────────────────────────────────────────────────────── */
  .action-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* ── Movement log ──────────────────────────────────────────────────────── */
  .movement-log {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 280px;
    overflow-y: auto;
  }

  .movement-row {
    display: grid;
    grid-template-columns: 1fr 1.4fr auto auto;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    font-size: 0.82rem;
    background: color-mix(in oklch, var(--color-base-200) 35%, transparent);
  }

  .movement-kind {
    color: var(--color-base-content);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .movement-locations {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--color-base-content);
    opacity: 0.75;
    font-size: 0.8rem;
    overflow: hidden;
    white-space: nowrap;
  }

  .movement-loc {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .movement-arrow {
    flex-shrink: 0;
    opacity: 0.6;
  }

  .movement-qty {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: right;
    color: var(--color-base-content);
  }

  .qty-positive {
    color: var(--color-success);
  }

  .qty-negative {
    color: var(--color-error);
  }

  .movement-time {
    font-size: 0.78rem;
    color: var(--color-base-content);
    opacity: 0.55;
    white-space: nowrap;
    text-align: right;
  }

  .empty-text {
    margin: 0;
    font-size: 0.86rem;
    color: var(--color-base-content);
    opacity: 0.55;
  }
</style>
