<script lang="ts">
  import { onMount } from "svelte";
  import {
    listDashboardLots,
    type DashboardFilters,
    type DashboardLotRow,
    type DashboardPreset,
    type UrgencyCounts,
    type DashboardResponse,
  } from "../lib/dashboard.js";
  import { scannerNavigation } from "../lib/navigation.js";
  import type { UnitKind } from "../lib/products.js";
  import {
    listStores,
    listStoreLocations,
    type StoreResponse,
    type StoreLocationResponse,
  } from "../lib/stores.js";
  import {
    getProduct,
    listCategories,
    lifecycleOf,
    type ProductDetailResponse,
    type CategoryResponse,
    type ProductResponse,
  } from "../lib/products.js";
  import {
    getExpiryLot,
    listExpiryLotsByProduct,
    type ExpiryLotResponse,
  } from "../lib/expiry_lots.js";
  import { exportReportWithDialog } from "../lib/csv.js";
  import CategoryPicker from "./inputs/CategoryPicker.svelte";
  import {
    listUnitDefinitions,
    type UnitDefinitionResponse,
  } from "../lib/unit_definitions.js";
  import ScanSearchBox from "./ScanSearchBox.svelte";
  import ProductForm from "./ProductForm.svelte";
  import UnitReviewBanner from "./UnitReviewBanner.svelte";
  import UnitReviewPage from "./UnitReviewPage.svelte";
  import LotMovementsPanel from "./LotMovementsPanel.svelte";
  // PR 6 — shared UI primitives from caduxo-daisyui-redesign.

  import Badge, {
    type BadgeUrgency,
    type BadgeSemantic,
  } from "./ui/Badge.svelte";
  import Table from "./ui/Table.svelte";
  import Tabs from "./ui/Tabs.svelte";
  import Alert from "./ui/Alert.svelte";
  import Button from "./ui/Button.svelte";
  import Icon from "./ui/Icon.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Tooltip from "./ui/Tooltip.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import LoadingState from "./ui/LoadingState.svelte";
  // PR 7b — inline overlay migration to the shared Modal primitive.
  import Modal from "./ui/Modal.svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";

  // ─── State ──────────────────────────────────────────────────────────────────

  let loading = true;
  let errorMsg = "";

  // Urgency counts
  let counts: UrgencyCounts = {
    expired: 0,
    today: 0,
    alert_window: 0,
    next_30_days: 0,
  };

  // Lot rows
  let lots: DashboardLotRow[] = [];

  // Stores and locations
  let stores: StoreResponse[] = [];
  let locations: StoreLocationResponse[] = [];

  // Filters
  let selectedStoreId: string | null = null;
  let selectedLocationId: string | null = null;
  let activePreset: DashboardPreset = "all";
  let categoryIds: string[] = [];

  /** Unit catalog for display-name resolution. */
  let unitCatalog: UnitDefinitionResponse[] = [];

  // Quick filter presets mapping
  const PRESET_LABELS: Record<DashboardPreset, string> = {
    all: "all",
    expired: "expired",
    today: "today",
    alert_window: "alertWindow",
    next_7_days: "next7Days",
    next_30_days: "next30Days",
  };
  const PRESET_ORDER: DashboardPreset[] = [
    "all",
    "expired",
    "today",
    "alert_window",
    "next_7_days",
    "next_30_days",
  ];

  // ─── Modal states ───────────────────────────────────────────────────────────

  let showProductDetail = false;
  let showUnitReview = false;
  let detailProduct: ProductDetailResponse | null = null;
  let detailLoading = false;

  // Product detail: lots + per-lot movement history
  let detailLots: ExpiryLotResponse[] = [];
  let detailLotsLoading = false;
  let detailSelectedLotId: string | null = null;
  /** Locations across every store that holds at least one of this product's lots. */
  let detailAllLocations: StoreLocationResponse[] = [];

  // CSV export state (Slice 10a)
  let exporting = false;

  let showLotDetail = false;
  let detailLot: ExpiryLotResponse | null = null;
  let detailLotLoading = false;
  let lotDetailTab: "detail" | "history" = "detail";
  let lotDetailLocations: StoreLocationResponse[] = [];

  // ─── Quick-create modal ─────────────────────────────────────────────────────

  let showQuickCreate = false;
  let quickCreateScannedValue = "";   // pre-fill UPC field in quick-create
  let categories: CategoryResponse[] = [];

  // ─── Lifecycle ──────────────────────────────────────────────────────────────

  onMount(async () => {
    categories = await listCategories().catch(() => []);
    await Promise.all([loadStores(), loadDashboard(), loadUnitCatalog()]);
    loading = false;
  });

  // Reload dashboard when store, location, or quick-filter preset changes.
  $: if (!loading && (selectedStoreId || selectedLocationId || activePreset)) {
    loadDashboard();
  }

  // Reload locations when store changes
  $: if (!loading && selectedStoreId) {
    loadLocationsForStore(selectedStoreId);
  }

  // ─── Data loading ───────────────────────────────────────────────────────────

  async function loadStores() {
    try {
      stores = await listStores();
      // Auto-select the first store if only one exists
      if (stores.length === 1) {
        selectedStoreId = stores[0].id;
        storeFilterValue = stores[0].id;
      }
    } catch (e) {
      errorMsg = humanizeError(e);
    }
  }

  async function loadLocationsForStore(storeId: string) {
    try {
      locations = await listStoreLocations(storeId);
      selectedLocationId = null;
      locationFilterValue = "";
    } catch (e) {
      errorMsg = humanizeError(e);
    }
  }

  async function loadDashboard() {
    const filters: DashboardFilters = {
      store_id: selectedStoreId,
      location_id: selectedLocationId,
      preset: activePreset,
      category_ids: categoryIds.length > 0 ? categoryIds : null,
    };
    try {
      const data: DashboardResponse = await listDashboardLots(filters);
      counts = data.counts;
      lots = data.lots;
      errorMsg = "";
    } catch (e) {
      errorMsg = humanizeError(e);
    }
  }

  async function loadUnitCatalog() {
    try {
      unitCatalog = await listUnitDefinitions();
    } catch {
      // Non-fatal: unit catalog is a convenience display feature.
    }
  }

  /**
   * Returns the effective unit display name for a lot row.
   * When the product has a catalog link (default_unit_id), uses the catalog
   * display_name; otherwise falls back to the raw lot.unit text.
   */
  function getUnitDisplayName(lot: DashboardLotRow): string {
    if (lot.default_unit_id && lot.default_unit_id !== "") {
      const match = unitCatalog.find((u) => u.id === lot.default_unit_id);
      if (match) return match.display_name;
    }
    return lot.unit;
  }

  function clearStoreFilter() {
    selectedStoreId = null;
    selectedLocationId = null;
    locations = [];
    storeFilterValue = "";
    locationFilterValue = "";
  }

  /** Resets every dashboard filter back to the default view. */
  function clearAllFilters() {
    activePreset = "all";
    selectedStoreId = null;
    selectedLocationId = null;
    storeFilterValue = "";
    locationFilterValue = "";
  }

  // ─── CSV export (Slice 10a) ────────────────────────────────────────────────

  async function exportReport() {
    exporting = true;
    try {
      const result = await exportReportWithDialog({
        store_id: selectedStoreId,
        location_id: selectedLocationId,
        preset: activePreset,
        urgency: null,
      });
      if (result) {
        errorMsg = "";
        // eslint-disable-next-line no-console
        console.info(
          `Exported ${result.rows_written} lot row(s) to ${result.path}`,
        );
      }
    } catch (e) {
      errorMsg = humanizeError(e);
    } finally {
      exporting = false;
    }
  }

  // ─── Scan handler ──────────────────────────────────────────────────────────

  function handleScanFound(productId: string, hasLots: boolean) {
    if (hasLots) {
      const row = lots.find((l) => l.product_id === productId);
      if (row) {
        viewProduct(row, row.lot_id);
      }
    } else {
      viewProduct({ lot_id: "", product_id: productId } as DashboardLotRow);
    }
  }

  function handleScanNotFound(scannedValue: string) {
    quickCreateScannedValue = scannedValue;
    showQuickCreate = true;
  }

  async function onQuickCreateSaved(product: ProductResponse) {
    showQuickCreate = false;
    categories = await listCategories();
    viewProduct({ lot_id: "", product_id: product.id } as DashboardLotRow);
  }

  // ─── Row actions ────────────────────────────────────────────────────────────

  async function viewProduct(lot: DashboardLotRow, preselectLotId: string | null = null) {
    detailLoading = true;
    showProductDetail = true;
    detailProduct = null;
    detailLots = [];
    detailSelectedLotId = preselectLotId;
    detailAllLocations = [];
    try {
      detailProduct = await getProduct(lot.product_id);
      await loadProductDetailLots(lot.product_id);
    } catch (e) {
      errorMsg = humanizeError(e);
      showProductDetail = false;
    } finally {
      detailLoading = false;
    }
  }

  async function loadAllStoreLocations(): Promise<StoreLocationResponse[]> {
    const activeStores = stores.length > 0
      ? stores.filter((store) => store.is_active)
      : (await listStores()).filter((store) => store.is_active);
    const locationLists = await Promise.all(
      activeStores.map((store) =>
        listStoreLocations(store.id)
          .then((locations) => locations.map((location) => ({ ...location, store_name: store.name })))
          .catch(() => [] as StoreLocationResponse[]),
      ),
    );
    return locationLists.flat();
  }

  async function loadProductDetailLots(productId: string) {
    detailLotsLoading = true;
    try {
      detailLots = await listExpiryLotsByProduct(productId);
      const stillExists = detailSelectedLotId
        && detailLots.some((l) => l.id === detailSelectedLotId);
      if (!stillExists) {
        const firstActive = detailLots.find((l) => l.status === "active");
        detailSelectedLotId = firstActive?.id ?? detailLots[0]?.id ?? null;
      }
      detailAllLocations = await loadAllStoreLocations();
    } catch (e) {
      errorMsg = humanizeError(e);
    } finally {
      detailLotsLoading = false;
    }
  }

  function closeProductDetail() {
    showProductDetail = false;
    detailProduct = null;
    detailLots = [];
    detailSelectedLotId = null;
    detailAllLocations = [];
  }

  async function refreshSelectedLot() {
    if (!detailSelectedLotId) return;
    try {
      const fresh = await getExpiryLot(detailSelectedLotId);
      detailLots = detailLots.map((l) => (l.id === fresh.id ? fresh : l));
    } catch (e) {
      errorMsg = humanizeError(e);
    }
  }

  $: detailSelectedLot =
    detailLots.find((l) => l.id === detailSelectedLotId) ?? null;

  // ─── Filter control bridges (Listbox value contract is `string`) ─────────
  // The dashboard's source of truth remains `selectedStoreId` /
  // `selectedLocationId` (`string | null`). The Listbox primitives need a
  // concrete `string` to bind to, so we use `""` as the "all" sentinel.
  // We deliberately do NOT use `bind:value` here (it would create a
  // reactive cycle because every `selectedStoreId` write would re-derive
  // `storeFilterValue`, which the Listbox would then write back into
  // `selectedStoreId`). Instead, we drive the Listbox with `value` + an
  // `onchange` callback, and seed the bridge once from the current
  // source-of-truth.
  let storeFilterValue: string = selectedStoreId ?? "";
  let locationFilterValue: string = selectedLocationId ?? "";

  $: storeFilterOptions = [
    { value: "", label: $LL.dashboard.allStores() },
    ...stores.map((store) => ({ value: store.id, label: store.name })),
  ];

  $: locationFilterOptions = [
    { value: "", label: $LL.dashboard.allLocations() },
    ...locations.map((loc) => ({ value: loc.id, label: loc.name })),
  ];

  function onStoreFilterChange(value: string) {
    selectedStoreId = value === "" ? null : value;
  }

  function onLocationFilterChange(value: string) {
    selectedLocationId = value === "" ? null : value;
  }

  async function editLot(lot: DashboardLotRow) {
    detailLotLoading = true;
    showLotDetail = true;
    lotDetailTab = "detail";
    detailLot = null;
    lotDetailLocations = [];
    try {
      [detailLot, lotDetailLocations] = await Promise.all([
        getExpiryLot(lot.lot_id),
        loadAllStoreLocations(),
      ]);
    } catch (e) {
      errorMsg = humanizeError(e);
      showLotDetail = false;
    } finally {
      detailLotLoading = false;
    }
  }

  // ─── Urgency / status helpers ───────────────────────────────────────────────

  function urgencyLabel(urgency: string): string {
    switch (urgency) {
      case "expired": return $LL.dashboard.urgency.expired();
      case "today": return $LL.dashboard.urgency.today();
      case "alert_window": return $LL.dashboard.urgency.alertWindow();
      case "next_30_days": return $LL.dashboard.urgency.next30Days();
      default: return $LL.dashboard.urgency.future();
    }
  }

  /** Maps the backend urgency value to the closed BadgeUrgency union. */
  function toBadgeUrgency(urgency: string): BadgeUrgency {
    switch (urgency) {
      case "expired": return "expired";
      case "today": return "today";
      case "alert_window": return "alert";
      case "next_30_days": return "soon";
      default: return "normal";
    }
  }

  /** Maps the backend lot-status value to a Badge semantic colour. */
  function toLotStatusSemantic(status: string): BadgeSemantic {
    switch (status) {
      case "active": return "success";
      case "resolved": return "info";
      case "archived": return "neutral";
      default: return "neutral";
    }
  }

  // ── Dashboard → Scanner navigation (ODD task 7) ─────────────────────────
  // Writes an in-memory request to the shared navigation channel.
  // App.svelte reacts by switching the shell to Scanner; ScannerPage
  // consumes the request, resolves the lot and product, populates the
  // canonical LotContextPanel, then clears the request.
  function openInScanner(lot: DashboardLotRow): void {
    scannerNavigation.set({
      lotId: lot.lot_id,
      productId: lot.product_id,
      unitType: (lot.unit_type ?? null) as UnitKind | null,
    });
  }

  function lotStatusLabel(status: string): string {
    switch (status) {
      case "active": return $LL.dashboard.active();
      case "resolved": return $LL.dashboard.resolved();
      case "archived": return $LL.dashboard.archived();
      default: return status;
    }
  }

  function formatDate(dateStr: string): string {
    if (!dateStr || dateStr.length !== 10) return dateStr;
    const [y, m, d] = dateStr.split("-");
    return `${d}/${m}/${y}`;
  }
</script>

<!--
  Reusable snippet: the canonical `<tr>` row that becomes the table header
  for the dashboard lot table. Hoisted to the component scope so every
  conditional `<Table>` render can reference the same row without
  duplication.
-->
{#snippet lotTableHead()}
  <tr>
    <th>{$LL.dashboard.sku()}</th>
    <th>{$LL.dashboard.description()}</th>
    <th>{$LL.dashboard.store()} / {$LL.dashboard.location()}</th>
    <th>{$LL.dashboard.qty()}</th>
    <th>{$LL.dashboard.expiryDate()}</th>
    <th>{$LL.dashboard.daysLeft()}</th>
    <th>{$LL.dashboard.urgencyLabel()}</th>
    <th>{$LL.dashboard.batch()}</th>
    <th>{$LL.common.actions()}</th>
  </tr>
{/snippet}

<!--
  Tabs panel snippets for the lot-detail modal. Hoisted to the component
  scope so the `items={[…]}` array can reference them by name; Svelte 5
  hoists `{#snippet}` declarations to the top of the component, which
  makes this forward-reference safe.
-->
{#snippet lotDetailPanel()}
  {#if detailLot}
    <dl class="detail-grid">
      <dt>{$LL.dashboard.lotId()}</dt><dd class="cell-sku">{detailLot.id.slice(0, 8)}…</dd>
      <dt>{$LL.dashboard.qty()}</dt><dd>{detailLot.quantity} {detailLot.unit}</dd>
      <dt>{$LL.dashboard.expiryDate()}</dt><dd>{formatDate(detailLot.expiry_date)}</dd>
      <dt>{$LL.dashboard.alertDaysBefore()}</dt><dd>{detailLot.alert_days_before}</dd>
      <dt>{$LL.dashboard.batch()}</dt><dd>{detailLot.batch_code ?? "—"}</dd>
      <dt>{$LL.dashboard.status()}</dt><dd>{detailLot.status}</dd>
      {#if detailLot.resolution}
        <dt>{$LL.lotMovements.resolution.resolveQuantity()}</dt><dd>{detailLot.resolution}</dd>
      {/if}
      {#if detailLot.notes}
        <dt>{$LL.lotForm.notes()}</dt><dd>{detailLot.notes}</dd>
      {/if}
    </dl>
  {/if}
{/snippet}

{#snippet lotHistoryPanel()}
  {#if detailLot}
    <LotMovementsPanel
      lotId={detailLot.id}
      lotQuantity={detailLot.quantity}
      lotUnit={detailLot.unit}
      lotStatus={detailLot.status}
      locations={lotDetailLocations}
      allLocations={lotDetailLocations}
      unitType={detailLot.unit_type}
      onMovementCreated={async () => {
        detailLot = await getExpiryLot(detailLot!.id);
      }}
    />
  {/if}
{/snippet}

<!-- Dashboard layout -->
<div class="dashboard">

  <!-- ── Header ─────────────────────────────────────────────────────────── -->
  <header class="dash-header">
    <div class="dash-title-row">
      <h2>{$LL.dashboard.pageTitle()}</h2>
      <div class="dash-title-actions">
        <Tooltip text={$LL.dashboard.actions.exportCsvTitle()} position="bottom">
          <Button
            variant="secondary"
            size="sm"
            onclick={exportReport}
            disabled={exporting}
            loading={exporting}
          >
            {exporting ? $LL.dashboard.actions.exporting() : $LL.dashboard.actions.exportCsv()}
          </Button>
        </Tooltip>

        {#if selectedStoreId}
          <span class="store-chip">
            {stores.find((s) => s.id === selectedStoreId)?.name ?? $LL.dashboard.store()}
            <Tooltip text={$LL.dashboard.clearStoreFilter()} position="left">
              <Button
                variant="ghost"
                size="xs"
                onclick={clearStoreFilter}
                aria-label={$LL.dashboard.clearStoreFilter()}
              >
                {#snippet iconStart()}
                  <Icon name="x-mark" size="xs" />
                {/snippet}
              </Button>
            </Tooltip>
          </span>
        {:else}
          <span class="store-chip store-chip-all">{$LL.dashboard.allStores()}</span>
        {/if}
      </div>
    </div>

    <!-- Always-visible scan/search input. The inline spinner inside
         ScanSearchBox still uses the legacy spinner class — that
         component is out of PR 6 scope (see apply-progress). -->
    <div class="scan-row">
      <ScanSearchBox
        placeholder={$LL.scan.placeholder()}
        onFound={handleScanFound}
        onNotFound={handleScanNotFound}
      />
    </div>

    <!-- Unit audit banner (appears when unrecognized units exist) -->
    {#if !showUnitReview}
      <UnitReviewBanner
        onReview={() => {
          showUnitReview = true;
        }}
      />
    {:else}
      <UnitReviewPage onDone={() => {
        showUnitReview = false;
        loadDashboard();
      }} />
    {/if}

    <!-- Store / location filter -->
    <div class="filter-row">
      <label class="filter-row-label">
        {$LL.dashboard.store()}:
        <Listbox
          value={storeFilterValue}
          options={storeFilterOptions}
          size="sm"
          aria-label={$LL.dashboard.store()}
          onchange={onStoreFilterChange}
        />
      </label>

      {#if selectedStoreId && locations.length > 0}
        <label class="filter-row-label">
          {$LL.dashboard.location()}:
          <Listbox
            value={locationFilterValue}
            options={locationFilterOptions}
            size="sm"
            aria-label={$LL.dashboard.location()}
            onchange={onLocationFilterChange}
          />
        </label>
      {/if}

      <CategoryPicker
        bind:value={categoryIds}
        {categories}
        includeUncategorized={true}
      />
    </div>
  </header>

  <!-- ── Urgency cards ────────────────────────────────────────────────────── -->
  <section
    class="stats stats-vertical lg:stats-horizontal shadow w-full overflow-hidden border-base-300"
    aria-label={$LL.dashboard.aria.urgencySummary()}
  >
    <div class="stat border-t-4 border-error">
      <div class="stat-figure">
        <Badge semantic="error" dot size="md" />
      </div>
      <div class="stat-title">{$LL.dashboard.urgencyCard.expired()}</div>
      <div class="stat-value text-error">{counts.expired}</div>
    </div>
    <div class="stat border-t-4 border-warning">
      <div class="stat-figure">
        <Badge semantic="warning" dot size="md" />
      </div>
      <div class="stat-title">{$LL.dashboard.urgencyCard.today()}</div>
      <div class="stat-value text-warning">{counts.today}</div>
    </div>
    <div class="stat border-t-4 border-info">
      <div class="stat-figure">
        <Badge semantic="info" dot size="md" />
      </div>
      <div class="stat-title">{$LL.dashboard.urgencyCard.alertWindow()}</div>
      <div class="stat-value text-info">{counts.alert_window}</div>
    </div>
    <div class="stat border-t-4 border-success">
      <div class="stat-figure">
        <Badge semantic="success" dot size="md" />
      </div>
      <div class="stat-title">{$LL.dashboard.urgencyCard.next30Days()}</div>
      <div class="stat-value text-success">{counts.next_30_days}</div>
    </div>
  </section>

  <!-- ── Quick filters ───────────────────────────────────────────────────── -->
  <div class="quick-filters" role="group" aria-label={$LL.dashboard.aria.quickFilters()}>
    {#each PRESET_ORDER as preset}
      {@const isActive = activePreset === preset}
      <button
        type="button"
        class="btn btn-ghost btn-sm motion-reduce:transition-none"
        class:btn-active={isActive}
        aria-pressed={isActive}
        onclick={() => (activePreset = preset)}
      >
        {$LL.dashboard.presets[PRESET_LABELS[preset] as keyof typeof $LL.dashboard.presets]()}
      </button>
    {/each}
  </div>

  <!-- ── Error alert ────────────────────────────────────────────────────── -->
  {#if errorMsg}
    <Alert
      variant="error"
      dismissible
      dismissLabel={$LL.common.dismiss()}
      ondismiss={() => (errorMsg = "")}
    >
      {errorMsg}
    </Alert>
  {/if}

  <!-- ── Lot table ──────────────────────────────────────────────────────── -->
  {#if loading}
    <div role="region" aria-label={$LL.dashboard.aria.lotTable()} aria-busy="true">
      <Table zebra stickyHeader scrollable aria-label={$LL.dashboard.aria.lotTable()}>
        {#snippet head()}
          {@render lotTableHead()}
        {/snippet}
        {#snippet loading()}
          <tr>
            <td colspan="9" class="p-4 text-center">
              <LoadingState variant="text" label={$LL.common.loading()} />
            </td>
          </tr>
        {/snippet}
      </Table>
    </div>
  {:else if lots.length === 0}
    <EmptyState
      title={$LL.dashboard.emptyState.title()}
      body={$LL.dashboard.emptyState.subtitle()}
      icon="inbox"
    >
      {#snippet actions()}
        <Button variant="link" onclick={clearAllFilters}>
          {$LL.dashboard.actions.clearFilters()}
        </Button>
      {/snippet}
    </EmptyState>
  {:else}
    <Table zebra stickyHeader scrollable aria-label={$LL.dashboard.aria.lotTable()}>
      {#snippet head()}
        {@render lotTableHead()}
      {/snippet}
      {#snippet body()}
        {#each lots as lot (lot.lot_id)}
          <tr class:row-expired={lot.urgency === "expired"} class:row-today={lot.urgency === "today"}>
            <td class="cell-sku">{lot.sku}</td>
            <td class="cell-desc">{lot.description}</td>
            <td class="cell-store">
              {lot.store_name}
              {#if lot.location_name}
                <span class="loc-name">/ {lot.location_name}</span>
              {/if}
            </td>
            <td class="cell-qty num">{lot.quantity} {getUnitDisplayName(lot)}</td>
            <td class="cell-date num">{formatDate(lot.expiry_date)}</td>
            <td class="cell-days num" class:days-negative={lot.days_remaining < 0}>
              {lot.days_remaining >= 0 ? lot.days_remaining : $LL.pdf.daysAgo({ n: Math.abs(lot.days_remaining) })}
            </td>
            <td>
              <Badge urgency={toBadgeUrgency(lot.urgency)} size="sm">
                {urgencyLabel(lot.urgency)}
              </Badge>
            </td>
            <td class="cell-batch">{lot.batch_code ?? "—"}</td>
            <td class="cell-actions">
              <Tooltip text={$LL.common.edit()}>
                <Button
                  variant="icon"
                  size="sm"
                  onclick={() => editLot(lot)}
                  aria-label={$LL.common.edit()}
                >
                  {#snippet iconStart()}
                    <Icon name="pencil" size="sm" />
                  {/snippet}
                </Button>
              </Tooltip>
              <Tooltip text={$LL.dashboard.viewProductAndMovements()}>
                <Button
                  variant="primary"
                  size="sm"
                  onclick={() => viewProduct(lot, lot.lot_id)}
                  aria-label={$LL.dashboard.viewProductAndMovements()}
                >
                  {#snippet iconStart()}
                    <Icon name="arrow-down-tray" size="sm" />
                  {/snippet}
                </Button>
              </Tooltip>
              <Tooltip text={$LL.dashboard.openInScanner()}>
                <Button
                  variant="secondary"
                  size="sm"
                  onclick={() => openInScanner(lot)}
                  aria-label={$LL.dashboard.openInScanner()}
                >
                  {#snippet iconStart()}
                    <Icon name="arrow-right-on-rectangle" size="sm" />
                  {/snippet}
                </Button>
              </Tooltip>
            </td>
          </tr>
        {/each}
      {/snippet}
    </Table>
  {/if}
</div>

<!-- ── Product detail modal ──────────────────────────────────────────────── -->
<Modal
  bind:open={showProductDetail}
  size="wide"
  showClose
  closeLabel={$LL.lotMovements.modal.close()}
  aria-label={$LL.dashboard.aria.productDetail()}
  oncancel={closeProductDetail}
  onclose={closeProductDetail}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.products.pageTitle()}</h3>
    </header>

    <div class="dialog-body">
      {#if detailLoading}
        <p class="dialog-loading">{$LL.common.loadingWithDots()}</p>
      {:else if detailProduct}
        <dl class="detail-grid">
          <dt>{$LL.dashboard.sku()}</dt><dd>{detailProduct.product.sku}</dd>
          <dt>{$LL.dashboard.description()}</dt><dd>{detailProduct.product.description}</dd>
          <dt>{$LL.dashboard.category()}</dt><dd>{detailProduct.categories.length > 0 ? detailProduct.categories.map(c => c.name).join(", ") : "—"}</dd>
          <dt>{$LL.dashboard.unit()}</dt><dd>{detailProduct.product.default_unit ?? "—"}</dd>
          <dt>{$LL.dashboard.alertDaysBefore()}</dt><dd>{detailProduct.product.default_alert_days_before}</dd>
          <dt>{$LL.dashboard.status()}</dt>
          <dd>
            {#if lifecycleOf(detailProduct.product) === "retired"}
              <span class="lifecycle-badge-retired">{$LL.products.lifecycleRetired()}</span>
            {:else if lifecycleOf(detailProduct.product) === "archived"}
              <span class="lifecycle-badge-archived">{$LL.products.lifecycleArchived()}</span>
            {:else}
              <span class="lifecycle-badge-active">{$LL.products.lifecycleActive()}</span>
            {/if}
          </dd>
          {#if detailProduct.barcodes.length > 0}
            <dt>{$LL.dashboard.barcode}s</dt>
            <dd>
              {#each detailProduct.barcodes as bc}
                <span class="barcode-chip" class:primary={bc.is_primary}>
                  {bc.barcode}{bc.is_primary ? " ★" : ""}
                </span>
              {/each}
            </dd>
          {/if}
        </dl>

        <!-- ── Expiry lots + per-lot movement history ───────────────────── -->
        <section class="lots-section" aria-label={$LL.dashboard.aria.expiryLots()}>
          <div class="lots-section-header">
            <h4>{$LL.dashboard.expiryLots()}</h4>
            {#if detailLotsLoading}
              <span class="lots-loading-hint">{$LL.dashboard.loading()}</span>
            {:else}
              <span class="lots-count">
                {$LL.dashboard.emptyState.lots({ n: detailLots.length })}
              </span>
            {/if}
          </div>

          {#if !detailLotsLoading && detailLots.length === 0}
            <p class="empty-hint">
              {$LL.dashboard.emptyState.noExpiryLots()}
            </p>
          {:else if detailLots.length > 0}
            <ul class="lot-picker" role="listbox" aria-label={$LL.dashboard.aria.productExpiryLots()}>
              {#each detailLots as lot (lot.id)}
                <li>
                  <button
                    type="button"
                    class="lot-picker-item"
                    class:active={detailSelectedLotId === lot.id}
                    class:lot-status-inactive={lot.status !== "active"}
                    onclick={() => (detailSelectedLotId = lot.id)}
                    aria-pressed={detailSelectedLotId === lot.id}
                  >
                    <span class="lot-picker-qty">
                      {lot.quantity} {lot.unit}
                    </span>
                    <span class="lot-picker-date">
                      Exp {formatDate(lot.expiry_date)}
                    </span>
                    {#if lot.batch_code}
                      <span class="lot-picker-batch">{lot.batch_code}</span>
                    {/if}
                    <Badge
                      semantic={toLotStatusSemantic(lot.status)}
                      size="sm"
                      dot
                    >
                      {lotStatusLabel(lot.status)}
                    </Badge>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}

          {#if detailSelectedLot}
            <div class="lot-panel-wrap">
              <LotMovementsPanel
                lotId={detailSelectedLot.id}
                lotQuantity={detailSelectedLot.quantity}
                lotUnit={detailSelectedLot.unit}
                lotStatus={detailSelectedLot.status}
                locations={detailAllLocations.filter(
                  (l) => l.store_id === detailSelectedLot!.store_id,
                )}
                allLocations={detailAllLocations}
                unitType={detailSelectedLot.unit_type}
                onMovementCreated={refreshSelectedLot}
              />
            </div>
          {/if}
        </section>
      {/if}
    </div>
  {/snippet}
</Modal>

<!-- ── Lot detail modal ─────────────────────────────────────────────────── -->
<Modal
  bind:open={showLotDetail}
  size="wide"
  showClose
  closeLabel={$LL.lotMovements.modal.close()}
  aria-label={$LL.dashboard.aria.lotDetail()}
  oncancel={() => (showLotDetail = false)}
  onclose={() => (showLotDetail = false)}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.dashboard.lotDetail()}</h3>
    </header>

    <div class="dialog-body">
      {#if detailLotLoading}
        <LoadingState variant="text" label={$LL.dashboard.loading()} />
      {:else if detailLot}
        <Tabs
          items={[
            { id: "detail", label: $LL.dashboard.detail(), panel: lotDetailPanel },
            { id: "history", label: $LL.lotMovements.history(), panel: lotHistoryPanel },
          ]}
          bind:activeId={lotDetailTab}
          style="bordered"
          aria-label={$LL.dashboard.lotDetail()}
        />
      {/if}
    </div>
  {/snippet}
</Modal>

<!-- ── Quick-create product modal ────────────────────────────────────────── -->
<Modal
  bind:open={showQuickCreate}
  size="wide"
  showClose
  closeLabel={$LL.lotMovements.modal.close()}
  aria-label={$LL.dashboard.aria.quickProductCreate()}
  oncancel={() => (showQuickCreate = false)}
  onclose={() => (showQuickCreate = false)}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.products.createProduct()}</h3>
    </header>

    <div class="dialog-body">
      <p class="scan-hint">
        {$LL.scan.noMatch()}
      </p>
      {#await listCategories() then cats}
        <ProductForm
          mode="create"
          initial={null}
          categories={cats}
          prefillUpc={quickCreateScannedValue}
          onSaved={onQuickCreateSaved}
          onCancel={() => (showQuickCreate = false)}
          onCategoryCreated={(c) => { categories = [...categories, c]; }}
        />
      {/await}
    </div>
  {/snippet}
</Modal>

<style>
  /* ── Layout ────────────────────────────────────────────────────────────── */
  .dashboard {
    padding: 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: calc(100vh - 48px);
  }

  /* ── Header ────────────────────────────────────────────────────────────── */
  .dash-header {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dash-title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .dash-title-row h2 {
    margin: 0;
    font-size: 1.2rem;
    color: var(--color-base-content);
  }

  .dash-title-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }

  /* ── Scan input row ───────────────────────────────────────────────────── */
  .scan-row {
    margin-top: 4px;
  }

  .store-chip {
    background: color-mix(in oklch, var(--color-primary) 6%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-primary) 25%, transparent);
    border-radius: 12px;
    padding: 2px 10px;
    font-size: 0.78rem;
    color: var(--color-primary);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .store-chip-all {
    background: var(--color-base-200);
    border-color: var(--color-base-300);
    color: color-mix(in oklch, var(--color-base-content) 65%, transparent);
  }

  .filter-row {
    display: flex;
    gap: 16px;
    align-items: center;
    flex-wrap: wrap;
  }

  .filter-row-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
  }

  /* ── Quick filters ────────────────────────────────────────────────────── */
  .quick-filters {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  /* ── Table ────────────────────────────────────────────────────────────── */
  .cell-sku {
    font-family: monospace;
    font-size: 0.8rem;
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
  }

  .cell-desc {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cell-store {
    white-space: nowrap;
  }

  .loc-name {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.78rem;
  }

  .cell-qty {
    white-space: nowrap;
  }

  .cell-date {
    white-space: nowrap;
  }

  .cell-days {
    white-space: nowrap;
    font-weight: 600;
  }

  .days-negative {
    color: var(--color-error);
  }

  .cell-batch {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.8rem;
  }

  .cell-actions {
    white-space: nowrap;
    display: flex;
    gap: 4px;
  }

  /* Force the lot table to keep its readable columns on narrow widths.
     The shared Table primitive already wraps the table in
     overflow-x-auto when scrollable, but DaisyUI .table defaults to
     width 100%, which lets the columns compress inside the scroll
     container. A min-width on the table makes the wrapper scroll
     horizontally instead of squeezing urgency/status text. */
  .dashboard :global(table) {
    min-width: 900px;
  }

  .row-expired td {
    background: color-mix(in oklch, var(--color-error) 6%, transparent);
  }

  .row-expired:hover td {
    background: color-mix(in oklch, var(--color-error) 12%, transparent);
  }

  .row-today td {
    background: color-mix(in oklch, var(--color-warning) 8%, transparent);
  }

  .row-today:hover td {
    background: color-mix(in oklch, var(--color-warning) 15%, transparent);
  }

  /* ── Modal overlays (PR 7b) ───────────────────────────────────────────── */
  /* Inner-section classes — the modal shell itself is now provided by
     `Modal.svelte` so the legacy shell selectors have been removed.
     The wide box width is now handled by `size="wide"` on `<Modal>`. */

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

  .dialog-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .scan-hint {
    font-size: 0.85rem;
    color: var(--color-base-content);
    margin: 0 0 12px;
    background: var(--color-base-200);
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    padding: 8px 12px;
  }

  .dialog-loading {
    color: var(--color-base-content);
    font-style: italic;
  }

  /* ── Detail grid ────────────────────────────────────────────────────── */
  .detail-grid {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 6px 12px;
    margin-bottom: 16px;
  }

  .detail-grid dt {
    font-size: 0.8rem;
    color: color-mix(in oklch, var(--color-base-content) 65%, transparent);
    font-weight: 500;
  }

  .detail-grid dd {
    font-size: 0.85rem;
    color: var(--color-base-content);
    margin: 0;
  }

  .barcode-chip {
    display: inline-block;
    background: var(--color-base-200);
    border: 1px solid var(--color-base-300);
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 0.78rem;
    font-family: monospace;
    margin: 0 2px;
  }

  .barcode-chip.primary {
    background: color-mix(in oklch, var(--color-primary) 6%, transparent);
    border-color: color-mix(in oklch, var(--color-primary) 25%, transparent);
    color: var(--color-primary);
  }

  /* ── Product detail: expiry lots picker ─────────────────────────────── */
  .lots-section {
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--color-base-300);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .lots-section-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .lots-section-header h4 {
    margin: 0;
    font-size: 0.92rem;
    color: var(--color-base-content);
    font-weight: 600;
  }

  .lots-loading-hint,
  .lots-count {
    font-size: 0.78rem;
    color: color-mix(in oklch, var(--color-base-content) 65%, transparent);
  }

  .empty-hint {
    color: color-mix(in oklch, var(--color-base-content) 55%, transparent);
    font-size: 0.85rem;
    font-style: italic;
    margin: 0;
  }

  .lot-picker {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 180px;
    overflow-y: auto;
  }

  .lot-picker-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    background: var(--color-base-200);
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    padding: 7px 10px;
    font-family: inherit;
    font-size: 0.82rem;
    color: var(--color-base-content);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s, border-color 0.12s;
  }

  .lot-picker-item:hover {
    background: var(--color-base-200);
  }

  .lot-picker-item.active {
    background: color-mix(in oklch, var(--color-primary) 6%, transparent);
    border-color: var(--color-primary);
  }

  .lot-picker-item.lot-status-inactive {
    opacity: 0.7;
  }

  .lot-picker-qty {
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }

  .lot-picker-date {
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
    font-variant-numeric: tabular-nums;
  }

  .lot-picker-batch {
    background: var(--color-base-300);
    color: var(--color-base-content);
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.74rem;
  }

  .lot-panel-wrap {
    margin-top: 6px;
    padding-top: 10px;
    border-top: 1px dashed var(--color-base-300);
  }

  /* ── Lifecycle badges in product modal (PR `product-lifecycle-reusable-identifiers`) ── */
  .lifecycle-badge-retired {
    display: inline-block;
    font-size: 0.75rem;
    background: color-mix(in oklch, var(--color-warning) 18%, transparent);
    color: color-mix(in oklch, var(--color-warning) 80%, var(--color-base-content));
    padding: 2px 7px;
    border-radius: 4px;
    font-weight: 600;
  }

  .lifecycle-badge-archived {
    display: inline-block;
    font-size: 0.75rem;
    background: var(--color-base-200);
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    padding: 2px 7px;
    border-radius: 4px;
    font-weight: 600;
  }

  .lifecycle-badge-active {
    display: inline-block;
    font-size: 0.75rem;
    background: color-mix(in oklch, var(--color-success) 15%, transparent);
    color: var(--color-success);
    padding: 2px 7px;
    border-radius: 4px;
    font-weight: 600;
  }
</style>