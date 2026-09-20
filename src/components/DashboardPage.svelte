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
  import {
    listStores,
    listStoreLocations,
    type StoreResponse,
    type StoreLocationResponse,
  } from "../lib/stores.js";
  import {
    getProduct,
    listCategories,
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
      import { UNCATEGORIZED_SENTINEL } from "../lib/categories.js";
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
      }
    } catch (e) {
      errorMsg = humanizeError(e);
    }
  }

  async function loadLocationsForStore(storeId: string) {
    try {
      locations = await listStoreLocations(storeId);
      selectedLocationId = null;
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
            // Use a transient success indicator via errorMsg reset path:
            // a small ephemeral log line keeps the wiring minimal.
            // (A dedicated success banner can land in Slice 10b alongside the
            // import commit and mapping modal.)
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
        // Jump to product detail if the product already has lots, otherwise show detail.
        if (hasLots) {
          // Find the lot row from the dashboard to open product detail with that lot pre-selected.
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
        // Reload categories in case a new one was added inline.
        categories = await listCategories();
        // Navigate to product detail for lot entry.
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

      /**
       * Loads every active/resolved/archived expiry lot for the given product and
       * caches the per-store location catalog so the movements panel can resolve
       * transfer/exit/adjust location names. Reuses the same `listExpiryLotsByProduct`
       * + `listStoreLocations` calls used by the dedicated Product Detail page.
       */
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
          // Keep the current selection if it still exists; otherwise pick the
          // first active lot (and fall back to the first available lot).
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

      /** Closes the product-detail modal and clears its transient state. */
      function closeProductDetail() {
        showProductDetail = false;
        detailProduct = null;
        detailLots = [];
        detailSelectedLotId = null;
        detailAllLocations = [];
      }

      /** Refreshes the currently selected lot row after a movement is created. */
      async function refreshSelectedLot() {
        if (!detailSelectedLotId) return;
        try {
          const fresh = await getExpiryLot(detailSelectedLotId);
          detailLots = detailLots.map((l) => (l.id === fresh.id ? fresh : l));
        } catch (e) {
          errorMsg = humanizeError(e);
        }
      }

      /** The lot row currently highlighted in the picker (or null). */
      $: detailSelectedLot =
        detailLots.find((l) => l.id === detailSelectedLotId) ?? null;

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

  // ─── Urgency helpers ────────────────────────────────────────────────────────

  function urgencyLabel(urgency: string): string {
    switch (urgency) {
      case "expired": return $LL.dashboard.urgency.expired();
      case "today": return $LL.dashboard.urgency.today();
      case "alert_window": return $LL.dashboard.urgency.alertWindow();
      case "next_30_days": return $LL.dashboard.urgency.next30Days();
      default: return $LL.dashboard.urgency.future();
    }
  }

  function urgencyClass(urgency: string): string {
    switch (urgency) {
      case "expired": return "badge-expired";
      case "today": return "badge-today";
      case "alert_window": return "badge-alert";
      case "next_30_days": return "badge-soon";
      default: return "badge-normal";
    }
  }

  function formatDate(dateStr: string): string {
    if (!dateStr || dateStr.length !== 10) return dateStr;
    const [y, m, d] = dateStr.split("-");
    return `${d}/${m}/${y}`;
  }

  
</script>

<!-- Dashboard layout -->
<div class="dashboard">

      <!-- ── Header ─────────────────────────────────────────────────────────── -->
      <header class="dash-header">
        <div class="dash-title-row">
          <h2>{$LL.dashboard.pageTitle()}</h2>
          <div class="dash-title-actions">
            <button
              class="btn-secondary btn-small"
              on:click={exportReport}
              disabled={exporting}
              title={$LL.dashboard.actions.exportCsvTitle()}
            >
              {exporting ? $LL.dashboard.actions.exporting() : $LL.dashboard.actions.exportCsv()}
            </button>
            {#if selectedStoreId}
              <span class="store-chip">
                {stores.find((s) => s.id === selectedStoreId)?.name ?? $LL.dashboard.store()}
                <button class="chip-clear" on:click={clearStoreFilter} title={$LL.dashboard.clearStoreFilter()}>✕</button>
              </span>
            {:else}
              <span class="store-chip store-chip-all">{$LL.dashboard.allStores()}</span>
            {/if}
          </div>
        </div>

        <!-- Always-visible scan/search input -->
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
            // Refresh the dashboard to pick up any unit-link changes.
            loadDashboard();
          }} />
        {/if}

    <!-- Store / location filter -->
    <div class="filter-row">
      <label>
        {$LL.dashboard.store()}:
        <select bind:value={selectedStoreId}>
          <option value={null}>{$LL.dashboard.allStores()}</option>
          {#each stores as store}
            <option value={store.id}>{store.name}</option>
          {/each}
        </select>
      </label>

      {#if selectedStoreId && locations.length > 0}
        <label>
          {$LL.dashboard.location()}:
          <select bind:value={selectedLocationId}>
            <option value={null}>{$LL.dashboard.allLocations()}</option>
            {#each locations as loc}
              <option value={loc.id}>{loc.name}</option>
            {/each}
          </select>
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
  <section class="urgency-cards" aria-label={$LL.dashboard.aria.urgencySummary()}>
    <article class="urgency-card urgency-card-expired" class:has-count={counts.expired > 0}>
      <span class="urgency-label">{$LL.dashboard.urgencyCard.expired()}</span>
      <strong class="urgency-count">{counts.expired}</strong>
    </article>
    <article class="urgency-card urgency-card-today" class:has-count={counts.today > 0}>
      <span class="urgency-label">{$LL.dashboard.urgencyCard.today()}</span>
      <strong class="urgency-count">{counts.today}</strong>
    </article>
    <article class="urgency-card urgency-card-alert" class:has-count={counts.alert_window > 0}>
      <span class="urgency-label">{$LL.dashboard.urgencyCard.alertWindow()}</span>
      <strong class="urgency-count">{counts.alert_window}</strong>
    </article>
    <article class="urgency-card urgency-card-soon" class:has-count={counts.next_30_days > 0}>
      <span class="urgency-label">{$LL.dashboard.urgencyCard.next30Days()}</span>
      <strong class="urgency-count">{counts.next_30_days}</strong>
    </article>
  </section>

  <!-- ── Quick filters ───────────────────────────────────────────────────── -->
  <div class="quick-filters" role="group" aria-label={$LL.dashboard.aria.quickFilters()}>
    {#each PRESET_ORDER as preset}
      <button
        class="filter-btn"
        class:active={activePreset === preset}
        on:click={() => (activePreset = preset)}
      >
        {$LL.dashboard.presets[PRESET_LABELS[preset] as keyof typeof $LL.dashboard.presets]()}
      </button>
    {/each}
  </div>

  <!-- ── Error banner ────────────────────────────────────────────────────── -->
  {#if errorMsg}
    <div class="error-banner" role="alert">
      {errorMsg}
      <button on:click={() => (errorMsg = "")}>{$LL.common.dismiss()}</button>
    </div>
  {/if}

  <!-- ── Lot table ──────────────────────────────────────────────────────── -->
  {#if loading}
    <div class="loading-row">{$LL.common.loading()}</div>
  {:else if lots.length === 0}
    <div class="empty-state">
      <p>{$LL.dashboard.emptyState.title()}</p>
      <p>
        <button class="link-btn" on:click={() => { activePreset = "all"; selectedStoreId = null; selectedLocationId = null; }}>
          {$LL.dashboard.actions.clearFilters()}
        </button>
      </p>
    </div>
  {:else}
    <div class="table-wrapper" role="region" aria-label={$LL.dashboard.aria.lotTable()}>
      <table class="lot-table">
        <thead>
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
        </thead>
        <tbody>
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
              <td class="cell-qty">{lot.quantity} {getUnitDisplayName(lot)}</td>
              <td class="cell-date">{formatDate(lot.expiry_date)}</td>
              <td class="cell-days" class:days-negative={lot.days_remaining < 0}>
                {lot.days_remaining >= 0 ? lot.days_remaining : $LL.pdf.daysAgo({ n: Math.abs(lot.days_remaining) })}
              </td>
              <td>
                <span class="urgency-badge {urgencyClass(lot.urgency)}">
                  {urgencyLabel(lot.urgency)}
                </span>
              </td>
              <td class="cell-batch">{lot.batch_code ?? "—"}</td>
              <td class="cell-actions">
                <button
                  class="action-btn"
                  title={$LL.common.edit()}
                  on:click={() => editLot(lot)}
                >✏️</button>
                <button
                  class="action-btn action-btn-lot-actions"
                  title={$LL.dashboard.viewProductAndMovements()}
                  on:click={() => viewProduct(lot, lot.lot_id)}
                >↓</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

    <!-- ── Product detail modal ─────────────────────────────────────────────────── -->
    {#if showProductDetail}
      <div class="modal-overlay" role="dialog" aria-modal="true" aria-label={$LL.products.pageTitle()}>
        <div class="modal-box modal-box-wide">
          <div class="modal-header">
            <h3>{$LL.products.pageTitle()}</h3>
            <button class="modal-close" on:click={closeProductDetail}>✕</button>
          </div>
          {#if detailLoading}
            <p class="modal-loading">{$LL.common.loadingWithDots()}</p>
          {:else if detailProduct}
            <dl class="detail-grid">
              <dt>{$LL.dashboard.sku()}</dt><dd>{detailProduct.product.sku}</dd>
              <dt>{$LL.dashboard.description()}</dt><dd>{detailProduct.product.description}</dd>
              <dt>{$LL.dashboard.category()}</dt><dd>{detailProduct.categories.length > 0 ? detailProduct.categories.map(c => c.name).join(", ") : "—"}</dd>
              <dt>{$LL.dashboard.unit()}</dt><dd>{detailProduct.product.default_unit ?? "—"}</dd>
              <dt>{$LL.dashboard.alertDaysBefore()}</dt><dd>{detailProduct.product.default_alert_days_before}</dd>
              <dt>{$LL.dashboard.status()}</dt><dd>{detailProduct.product.is_active ? $LL.dashboard.active() : $LL.dashboard.archived()}</dd>
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
                        on:click={() => (detailSelectedLotId = lot.id)}
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
                        <span class="lot-picker-status status-{lot.status}">
                          {lot.status}
                        </span>
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
      </div>
    {/if}

<!-- ── Lot detail modal ─────────────────────────────────────────────────────── -->
{#if showLotDetail}
  <div class="modal-overlay" role="dialog" aria-modal="true" aria-label={$LL.dashboard.aria.lotDetail()}>
    <div class="modal-box modal-box-wide">
      <div class="modal-header">
        <h3>{$LL.dashboard.lotDetail()}</h3>
        <button class="modal-close" on:click={() => (showLotDetail = false)}>✕</button>
      </div>

      {#if detailLotLoading}
        <p class="modal-loading">{$LL.dashboard.loading()}</p>
      {:else if detailLot}
        <!-- Tabs -->
        <div class="detail-tabs">
          <button
            type="button"
            class="tab-btn"
            class:active={lotDetailTab === "detail"}
            on:click={() => (lotDetailTab = "detail")}
          >
            {$LL.dashboard.detail()}
          </button>
          <button
            type="button"
            class="tab-btn"
            class:active={lotDetailTab === "history"}
            on:click={() => (lotDetailTab = "history")}
          >
            {$LL.lotMovements.history()}
          </button>
        </div>

        {#if lotDetailTab === "detail"}
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
        {:else}
          <!-- Historial tab -->
          <div class="tab-content">
            <LotMovementsPanel
              lotId={detailLot.id}
              lotQuantity={detailLot.quantity}
              lotUnit={detailLot.unit}
              lotStatus={detailLot.status}
              locations={lotDetailLocations}
              allLocations={lotDetailLocations}
              unitType={detailLot.unit_type}
              onMovementCreated={async () => {
// Reload lot data after movement
detailLot = await getExpiryLot(detailLot!.id);
              }}
            />
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}

    <!-- ── Quick-create product modal ──────────────────────────────────────────── -->
    {#if showQuickCreate}
      <div class="modal-overlay" role="dialog" aria-modal="true" aria-label={$LL.dashboard.aria.quickProductCreate()}>
        <div class="modal-box modal-box-wide">
          <div class="modal-header">
            <h3>{$LL.products.createProduct()}</h3>
            <button class="modal-close" on:click={() => (showQuickCreate = false)}>✕</button>
          </div>
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
      </div>
    {/if}

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
    color: #0f172a;
  }

  /* ── Scan input row ───────────────────────────────────────────────────── */
  .scan-row {
    margin-top: 4px;
  }

  .store-chip {
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 12px;
    padding: 2px 10px;
    font-size: 0.78rem;
    color: #1d4ed8;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .store-chip-all {
    background: #f9fafb;
    border-color: #e5e7eb;
    color: #6b7280;
  }

  .chip-clear {
    background: none;
    border: none;
    cursor: pointer;
    color: #93c5fd;
    padding: 0;
    font-size: 0.7rem;
    line-height: 1;
  }

  .chip-clear:hover {
    color: #bfdbfe;
  }

  .filter-row {
    display: flex;
    gap: 16px;
    align-items: center;
    flex-wrap: wrap;
  }

  .filter-row label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
    color: #475569;
  }

  .filter-row select {
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 0.85rem;
    background: #fff;
    color: #1e293b;
  }

  /* ── Urgency cards ────────────────────────────────────────────────────── */
  .urgency-cards {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }

  .urgency-card {
    background: #fff;
    border: 2px solid #e5e7eb;
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    transition: box-shadow 0.15s;
  }

  .urgency-card.has-count {
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.06);
  }

  .urgency-card-expired {
    border-color: #dc2626;
    background: #fef2f2;
  }
  .urgency-card-expired .urgency-count { color: #dc2626; }

  .urgency-card-today {
    border-color: #d97706;
    background: #fffbeb;
  }
  .urgency-card-today .urgency-count { color: #d97706; }

  .urgency-card-alert {
    border-color: #2563eb;
    background: #eff6ff;
  }
  .urgency-card-alert .urgency-count { color: #2563eb; }

  .urgency-card-soon {
    border-color: #7c3aed;
    background: #f5f3ff;
  }
  .urgency-card-soon .urgency-count { color: #7c3aed; }

  .urgency-label {
    font-size: 0.75rem;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .urgency-count {
    font-size: 1.8rem;
    font-weight: 700;
    line-height: 1;
    color: #374151;
  }

  /* ── Quick filters ────────────────────────────────────────────────────── */
  .quick-filters {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .filter-btn {
    background: #fff;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 4px 12px;
    font-size: 0.82rem;
    color: #374151;
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
    font-family: inherit;
  }

  .filter-btn:hover {
    background: #f3f4f6;
  }

  .filter-btn.active {
    background: #1e40af;
    border-color: #1e40af;
    color: #fff;
  }

  /* ── Error banner ────────────────────────────────────────────────────── */
  .error-banner {
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    padding: 8px 12px;
    color: #dc2626;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .error-banner button {
    margin-left: auto;
    background: none;
    border: none;
    color: #dc2626;
    cursor: pointer;
    font-size: 0.8rem;
    text-decoration: underline;
  }

  /* ── Table ────────────────────────────────────────────────────────────── */
  .table-wrapper {
    overflow-x: auto;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
    background: #fff;
  }

  .lot-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .lot-table thead {
    background: #f8fafc;
    position: sticky;
    top: 0;
  }

  .lot-table th {
    text-align: left;
    padding: 8px 12px;
    font-weight: 600;
    color: #475569;
    border-bottom: 1px solid #e5e7eb;
    white-space: nowrap;
  }

  .lot-table td {
    padding: 8px 12px;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: middle;
    color: #1e293b;
  }

  .lot-table tr:last-child td {
    border-bottom: none;
  }

  .lot-table tr:hover {
    background: #f8fafc;
  }

  .row-expired td {
    background: #fff5f5;
  }

  .row-expired:hover td {
    background: #ffe4e4;
  }

  .row-today td {
    background: #fffbeb;
  }

  .row-today:hover td {
    background: #fef3c7;
  }

  /* ── Table cells ──────────────────────────────────────────────────────── */
  .cell-sku {
    font-family: monospace;
    font-size: 0.8rem;
    color: #475569;
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
    color: #94a3b8;
    font-size: 0.78rem;
  }

  .cell-qty {
    white-space: nowrap;
  }

  .cell-date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .cell-days {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  .days-negative {
    color: #dc2626;
  }

  .cell-batch {
    color: #94a3b8;
    font-size: 0.8rem;
  }

  .cell-actions {
    white-space: nowrap;
    display: flex;
    gap: 4px;
  }

  /* ── Urgency badges ──────────────────────────────────────────────────── */
  .urgency-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .badge-expired {
    background: #fee2e2;
    color: #991b1b;
  }

  .badge-today {
    background: #fef3c7;
    color: #92400e;
  }

  .badge-alert {
    background: #dbeafe;
    color: #1e40af;
  }

  .badge-soon {
    background: #ede9fe;
    color: #5b21b6;
  }

  .badge-normal {
    background: #f1f5f9;
    color: #475569;
  }

  /* ── Action buttons ──────────────────────────────────────────────────── */
  .action-btn {
    background: none;
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.1s;
    line-height: 1;
  }

  .action-btn:hover {
    background: #f3f4f6;
  }

  .action-btn-lot-actions {
    color: #2563eb;
    border-color: #bfdbfe;
  }

  .action-btn-lot-actions:hover {
    background: #eff6ff;
  }

  /* ── Loading / empty ─────────────────────────────────────────────────── */
  .loading-row {
    text-align: center;
    padding: 40px;
    color: #94a3b8;
    font-style: italic;
  }

  .empty-state {
    text-align: center;
    padding: 40px;
    color: #94a3b8;
  }

  .empty-state p {
    margin: 4px 0;
  }

  .link-btn {
    background: none;
    border: none;
    color: #2563eb;
    cursor: pointer;
    font-size: 0.85rem;
    text-decoration: underline;
  }

      /* ── Modals ──────────────────────────────────────────────────────────── */
      .modal-box-wide {
        width: 720px;
        max-width: 95vw;
      }

      .scan-hint {
        font-size: 0.85rem;
        color: #475569;
        margin: 0 0 12px;
        background: #f8fafc;
        border: 1px solid #e5e7eb;
        border-radius: 6px;
        padding: 8px 12px;
      }

      .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal-box {
    background: #fff;
    border-radius: 12px;
    padding: 24px;
    width: 480px;
    max-width: 95vw;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.2);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1rem;
    color: #0f172a;
  }

  .modal-close {
    background: none;
    border: none;
    font-size: 1rem;
    cursor: pointer;
    color: #94a3b8;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .modal-close:hover {
    background: #f1f5f9;
    color: #475569;
  }

  .modal-loading {
    color: #94a3b8;
    font-style: italic;
  }

  /* ── Detail grid ────────────────────────────────────────────────────── */
  .detail-grid {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 6px 12px;
    margin-bottom: 16px;
  }

  /* ── Detail tabs ────────────────────────────────────────────────────── */
  .detail-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
    border-bottom: 1px solid #e5e7eb;
  }

  .tab-btn {
    background: none;
    border: none;
    padding: 8px 16px;
    font-size: 0.88rem;
    cursor: pointer;
    color: #6b7280;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    font-family: inherit;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab-btn:hover {
    color: #374151;
  }

  .tab-btn.active {
    color: #2563eb;
    border-bottom-color: #2563eb;
    font-weight: 500;
  }

  .tab-content {
    min-height: 200px;
  }

  .detail-grid dt {
    font-size: 0.8rem;
    color: #6b7280;
    font-weight: 500;
  }

  .detail-grid dd {
    font-size: 0.85rem;
    color: #1e293b;
    margin: 0;
  }

  .barcode-chip {
    display: inline-block;
    background: #f1f5f9;
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 0.78rem;
    font-family: monospace;
    margin: 0 2px;
  }

  .barcode-chip.primary {
    background: #eff6ff;
    border-color: #bfdbfe;
    color: #1d4ed8;
  }

  /* ── Product detail: expiry lots picker ─────────────────────────────── */
  .lots-section {
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid #e5e7eb;
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
    color: #0f172a;
    font-weight: 600;
  }

  .lots-loading-hint,
  .lots-count {
    font-size: 0.78rem;
    color: #6b7280;
  }

  .empty-hint {
    color: #9ca3af;
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
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    padding: 7px 10px;
    font-family: inherit;
    font-size: 0.82rem;
    color: #1e293b;
    cursor: pointer;
    text-align: left;
    transition: background 0.12s, border-color 0.12s;
  }

  .lot-picker-item:hover {
    background: #f3f4f6;
  }

  .lot-picker-item.active {
    background: #eff6ff;
    border-color: #3b82f6;
  }

  .lot-picker-item.lot-status-inactive {
    opacity: 0.7;
  }

  .lot-picker-qty {
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }

  .lot-picker-date {
    color: #475569;
    font-variant-numeric: tabular-nums;
  }

  .lot-picker-batch {
    background: #e5e7eb;
    color: #374151;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.74rem;
  }

  .lot-picker-status {
    margin-left: auto;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 1px 7px;
    border-radius: 999px;
    background: #f1f5f9;
    color: #475569;
  }

  .lot-picker-status.status-active {
    background: #dcfce7;
    color: #166534;
  }

  .lot-picker-status.status-resolved {
    background: #dbeafe;
    color: #1e40af;
  }

  .lot-picker-status.status-archived {
    background: #f3f4f6;
    color: #9ca3af;
  }

  .lot-panel-wrap {
    margin-top: 6px;
    padding-top: 10px;
    border-top: 1px dashed #e5e7eb;
  }

  .modal-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }

  .btn-primary {
    background: #2563eb;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 7px 16px;
    font-size: 0.85rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-primary:hover {
    background: #1d4ed8;
  }

  .btn-primary:disabled {
    background: #93c5fd;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: #fff;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 7px 16px;
    font-size: 0.85rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-secondary:hover {
    background: #f9fafb;
  }

  .btn-secondary:disabled {
    color: #9ca3af;
    cursor: not-allowed;
  }
</style>
