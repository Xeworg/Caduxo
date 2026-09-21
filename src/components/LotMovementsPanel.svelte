<!--
  LotMovementsPanel.svelte — single-lot movement ledger
  (PR 9a of caduxo-daisyui-redesign).

  Migration to shared UI primitives:
    - Table.svelte (zebra, stickyHeader) hosts the per-lot movement
      ledger; numeric columns use the `num` utility from PR 1.
    - LoadingState.svelte (text variant) replaces the bespoke
      `.loading` placeholder.
    - EmptyState.svelte replaces the bespoke `.empty-hint` text.
    - Alert.svelte (variant=error) replaces the bespoke
      `.alert-error` div.
    - Button.svelte + Tooltip.svelte host the action buttons
      (moveStock / registerExit / adjustCount).

  New i18n keys (`lotMovements.table.*`) introduce the table-header
  copy that did not exist before the tabular migration.

  Tailwind classes referenced here (for the JIT scanner):
    table table-zebra table-pin-rows
    overflow-x-auto
    btn btn-secondary btn-error btn-success btn-sm
    tooltip tooltip-bottom
    alert alert-error alert-soft
-->
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
  import Table from "./ui/Table.svelte";
  import LoadingState from "./ui/LoadingState.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import Alert from "./ui/Alert.svelte";
  import Button from "./ui/Button.svelte";
  import Tooltip from "./ui/Tooltip.svelte";
  import Icon from "./ui/Icon.svelte";

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
        <Tooltip text={$LL.lotMovements.actionTitles.moveStock()} position="bottom">
          <Button variant="secondary" size="sm" onclick={() => openActionForm("move")}>
            {#snippet iconStart()}
              <Icon name="arrows-right-left" size="sm" />
            {/snippet}
            {$LL.lotMovements.moveStock()}
          </Button>
        </Tooltip>
        <Tooltip text={$LL.lotMovements.actionTitles.registerExit()} position="bottom">
          <Button variant="danger" size="sm" onclick={() => openActionForm("exit")}>
            {#snippet iconStart()}
              <Icon name="arrow-right-on-rectangle" size="sm" />
            {/snippet}
            {$LL.lotMovements.registerExit()}
          </Button>
        </Tooltip>
        <Tooltip text={$LL.lotMovements.actionTitles.adjustCount()} position="bottom">
          <Button variant="success" size="sm" onclick={() => openActionForm("adjust")}>
            {#snippet iconStart()}
              <Icon name="adjustments-horizontal" size="sm" />
            {/snippet}
            {$LL.lotMovements.adjustCount()}
          </Button>
        </Tooltip>
      </div>
    {/if}
  </div>

  <!-- ── Movements list ────────────────────────────────────────────────────── -->
  {#if loading}
    <LoadingState variant="text" label={$LL.lotMovements.loadingMovements()} />
  {:else if errorMsg}
    <Alert variant="error">{errorMsg}</Alert>
  {:else if movements.length === 0}
    <EmptyState
      title={$LL.lotMovements.noMovements()}
      icon="inbox"
    />
  {:else}
    <Table zebra stickyHeader scrollable aria-label={$LL.lotMovements.panelTitle()}>
      {#snippet head()}
        <tr>
          <th>{$LL.lotMovements.table.kind()}</th>
          <th class="num">{$LL.lotMovements.table.quantity()}</th>
          <th>{$LL.lotMovements.table.locations()}</th>
          <th>{$LL.lotMovements.table.time()}</th>
        </tr>
      {/snippet}
      {#snippet body()}
        {#each movements as mov (mov.id)}
          {@const label = getMovementKindLabel(mov.movement_kind, mov.direction)}
          {@const qty = formatMovementQuantity(mov.quantity, mov.movement_kind, mov.direction)}
          <tr>
            <td>{label}</td>
            <td class="num">{qty}</td>
            <td>
              {#if mov.movement_kind === "transfer" || mov.movement_kind.startsWith("entry:") || mov.movement_kind === "inventory_adjustment"}
                {#if mov.source_location_id}
                  <span class="loc-badge">{getLocationName(mov.source_location_id)}</span>
                  →
                {/if}
                <span class="loc-badge">{getLocationName(mov.destination_location_id)}</span>
              {:else}
                <span>{$LL.lotMovements.fromLocation()}</span>
                <span class="loc-badge">{getLocationName(mov.source_location_id)}</span>
              {/if}
            </td>
            <td>{formatDate(mov.created_at)}</td>
          </tr>
          {#if mov.notes}
            <tr class="notes-row">
              <td colspan="4" class="movement-notes">{mov.notes}</td>
            </tr>
          {/if}
        {/each}
      {/snippet}
    </Table>
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
    color: var(--color-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .total-value {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--color-base-content);
  }

  .panel-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .loc-badge {
    background: color-mix(in oklch, var(--color-base-200) 80%, transparent);
    color: var(--color-base-content);
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.74rem;
  }

  .notes-row td {
    background: color-mix(in oklch, var(--color-base-200) 50%, transparent);
    padding-top: 4px;
    padding-bottom: 8px;
  }

  .movement-notes {
    margin: 0;
    font-size: 0.8rem;
    color: var(--color-secondary);
    font-style: italic;
    white-space: pre-wrap;
  }
</style>