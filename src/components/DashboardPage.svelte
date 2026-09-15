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
    all: "All",
    expired: "Expired",
    today: "Today",
    alert_window: "Alert window",
    next_7_days: "Next 7 days",
    next_30_days: "Next 30 days",
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

  // CSV export state (Slice 10a)
  let exporting = false;

  let showLotDetail = false;
  let detailLot: ExpiryLotResponse | null = null;
  let detailLotLoading = false;

  let showResolveDialog = false;
  let resolveLot: DashboardLotRow | null = null;
  let resolveQty = 0;
  let resolveType = "consumed";
  let resolveNotes = "";
  let resolveLoading = false;
  let resolveError = "";

  // ─── Quick-create modal ─────────────────────────────────────────────────────

  let showQuickCreate = false;
  let quickCreateScannedValue = "";   // pre-fill UPC field in quick-create
  let categories: CategoryResponse[] = [];

  // ─── Resolve types ──────────────────────────────────────────────────────────

  const RESOLVE_TYPES = [
    { value: "consumed", label: "Consumed / Used" },
    { value: "sold", label: "Sold" },
    { value: "discarded", label: "Discarded" },
    { value: "donated", label: "Donated" },
    { value: "transferred", label: "Transferred" },
    { value: "other", label: "Other" },
  ];

  // ─── Lifecycle ──────────────────────────────────────────────────────────────

  onMount(async () => {
    categories = await listCategories().catch(() => []);
    await Promise.all([loadStores(), loadDashboard(), loadUnitCatalog()]);
    loading = false;
  });

  // Reload dashboard when store, location, or quick-filter preset changes.
  $: if (!loading) {
    selectedStoreId;
    selectedLocationId;
    activePreset;
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
      errorMsg = String(e);
    }
  }

  async function loadLocationsForStore(storeId: string) {
    try {
      locations = await listStoreLocations(storeId);
      selectedLocationId = null;
    } catch (e) {
      errorMsg = String(e);
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
          errorMsg = String(e);
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
          errorMsg = String(e);
        } finally {
          exporting = false;
        }
      }

      // ─── Scan handler ──────────────────────────────────────────────────────────

      function handleScanFound(productId: string, hasLots: boolean) {
        // Jump to lot entry if the product already has lots, otherwise show detail.
        if (hasLots) {
          // Find the lot row from the dashboard to open resolve dialog directly.
          const row = lots.find((l) => l.product_id === productId);
          if (row) {
            openResolve(row);
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

  async function viewProduct(lot: DashboardLotRow) {
    detailLoading = true;
    showProductDetail = true;
    detailProduct = null;
    try {
      detailProduct = await getProduct(lot.product_id);
    } catch (e) {
      errorMsg = String(e);
      showProductDetail = false;
    } finally {
      detailLoading = false;
    }
  }

  async function editLot(lot: DashboardLotRow) {
    detailLotLoading = true;
    showLotDetail = true;
    detailLot = null;
    try {
      detailLot = await getExpiryLot(lot.lot_id);
    } catch (e) {
      errorMsg = String(e);
      showLotDetail = false;
    } finally {
      detailLotLoading = false;
    }
  }

  function openResolve(lot: DashboardLotRow) {
    resolveLot = lot;
    resolveQty = 0;
    resolveType = "consumed";
    resolveNotes = "";
    resolveError = "";
    resolveLoading = false;
    showResolveDialog = true;
  }

  async function openResolveFromDetail(detail: ExpiryLotResponse) {
    // Build a minimal DashboardLotRow from the ExpiryLotResponse for the resolve dialog.
    const row: DashboardLotRow = {
      lot_id: detail.id,
      product_id: detail.product_id,
      sku: "",       // resolved from product in viewProduct; not needed for resolve
      description: "",
      store_id: detail.store_id,
      store_name: "",
      location_id: detail.location_id,
      location_name: null,
      quantity: detail.quantity,
      unit: detail.unit,
      expiry_date: detail.expiry_date,
      alert_days_before: detail.alert_days_before,
      batch_code: detail.batch_code,
      status: detail.status,
      urgency: "",
      days_remaining: 0,
      default_unit_id: null,
      unit_type: null,
    };
    resolveLot = row;
    resolveQty = 0;
    resolveType = "consumed";
    resolveNotes = "";
    resolveError = "";
    resolveLoading = false;
    showResolveDialog = true;
  }

  async function submitResolve() {
    if (!resolveLot) return;
    resolveError = "";
    if (resolveQty <= 0) {
      resolveError = "Quantity must be greater than zero.";
      return;
    }
    if (resolveQty > resolveLot.quantity) {
      resolveError = `Cannot resolve more than the remaining quantity (${resolveLot.quantity} ${resolveLot.unit}).`;
      return;
    }
    resolveLoading = true;
    try {
      const { resolveExpiryLot } = await import("../lib/expiry_lots.js");
      await resolveExpiryLot({
        lot_id: resolveLot.lot_id,
        quantity: resolveQty,
        resolution: resolveType,
        notes: resolveNotes || null,
      });
      showResolveDialog = false;
      resolveLot = null;
      await loadDashboard();
    } catch (e) {
      resolveError = String(e);
    } finally {
      resolveLoading = false;
    }
  }

  // ─── Urgency helpers ────────────────────────────────────────────────────────

  function urgencyLabel(urgency: string): string {
    switch (urgency) {
      case "expired": return "Expired";
      case "today": return "Today";
      case "alert_window": return "Alert window";
      case "next_30_days": return "Next 30 days";
      default: return "Future";
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
          <h2>Dashboard</h2>
          <div class="dash-title-actions">
            <button
              class="btn-secondary btn-small"
              on:click={exportReport}
              disabled={exporting}
              title="Export the current dashboard report to a CSV file"
            >
              {exporting ? "Exporting…" : "Export CSV"}
            </button>
            {#if selectedStoreId}
              <span class="store-chip">
                {stores.find((s) => s.id === selectedStoreId)?.name ?? "Store"}
                <button class="chip-clear" on:click={clearStoreFilter} title="Clear store filter">✕</button>
              </span>
            {:else}
              <span class="store-chip store-chip-all">All stores</span>
            {/if}
          </div>
        </div>

        <!-- Always-visible scan/search input -->
        <div class="scan-row">
          <ScanSearchBox
            placeholder="Scan barcode or type SKU, then press Enter…"
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
        Store:
        <select bind:value={selectedStoreId}>
          <option value={null}>All stores</option>
          {#each stores as store}
            <option value={store.id}>{store.name}</option>
          {/each}
        </select>
      </label>

      {#if selectedStoreId && locations.length > 0}
        <label>
          Location:
          <select bind:value={selectedLocationId}>
            <option value={null}>All locations</option>
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
        placeholder="Filter by category…"
      />
    </div>
  </header>

  <!-- ── Urgency cards ────────────────────────────────────────────────────── -->
  <section class="urgency-cards" aria-label="Urgency summary">
    <article class="urgency-card urgency-card-expired" class:has-count={counts.expired > 0}>
      <span class="urgency-label">Expired</span>
      <strong class="urgency-count">{counts.expired}</strong>
    </article>
    <article class="urgency-card urgency-card-today" class:has-count={counts.today > 0}>
      <span class="urgency-label">Today</span>
      <strong class="urgency-count">{counts.today}</strong>
    </article>
    <article class="urgency-card urgency-card-alert" class:has-count={counts.alert_window > 0}>
      <span class="urgency-label">Alert window</span>
      <strong class="urgency-count">{counts.alert_window}</strong>
    </article>
    <article class="urgency-card urgency-card-soon" class:has-count={counts.next_30_days > 0}>
      <span class="urgency-label">Next 30 days</span>
      <strong class="urgency-count">{counts.next_30_days}</strong>
    </article>
  </section>

  <!-- ── Quick filters ───────────────────────────────────────────────────── -->
  <div class="quick-filters" role="group" aria-label="Quick filters">
    {#each PRESET_ORDER as preset}
      <button
        class="filter-btn"
        class:active={activePreset === preset}
        on:click={() => (activePreset = preset)}
      >
        {PRESET_LABELS[preset]}
      </button>
    {/each}
  </div>

  <!-- ── Error banner ────────────────────────────────────────────────────── -->
  {#if errorMsg}
    <div class="error-banner" role="alert">
      {errorMsg}
      <button on:click={() => (errorMsg = "")}>Dismiss</button>
    </div>
  {/if}

  <!-- ── Lot table ──────────────────────────────────────────────────────── -->
  {#if loading}
    <div class="loading-row">Loading dashboard…</div>
  {:else if lots.length === 0}
    <div class="empty-state">
      <p>No lots match the current filters.</p>
      <p>
        <button class="link-btn" on:click={() => { activePreset = "all"; selectedStoreId = null; selectedLocationId = null; }}>
          Clear all filters
        </button>
      </p>
    </div>
  {:else}
    <div class="table-wrapper" role="region" aria-label="Lot table">
      <table class="lot-table">
        <thead>
          <tr>
            <th>SKU</th>
            <th>Description</th>
            <th>Store / Location</th>
            <th>Qty</th>
            <th>Expiry</th>
            <th>Days left</th>
            <th>Urgency</th>
            <th>Batch</th>
            <th>Actions</th>
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
                {lot.days_remaining >= 0 ? lot.days_remaining : `+${Math.abs(lot.days_remaining)} ago`}
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
                  title="View product"
                  on:click={() => viewProduct(lot)}
                >👁</button>
                <button
                  class="action-btn"
                  title="Edit lot"
                  on:click={() => editLot(lot)}
                >✏️</button>
                <button
                  class="action-btn action-btn-resolve"
                  title="Resolve quantity"
                  on:click={() => openResolve(lot)}
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
  <div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Product detail">
    <div class="modal-box">
      <div class="modal-header">
        <h3>Product Detail</h3>
        <button class="modal-close" on:click={() => (showProductDetail = false)}>✕</button>
      </div>
      {#if detailLoading}
        <p class="modal-loading">Loading…</p>
      {:else if detailProduct}
        <dl class="detail-grid">
          <dt>SKU</dt><dd>{detailProduct.product.sku}</dd>
          <dt>Description</dt><dd>{detailProduct.product.description}</dd>
          <dt>Category</dt><dd>{detailProduct.categories.length > 0 ? detailProduct.categories.map(c => c.name).join(", ") : "—"}</dd>
          <dt>Default unit</dt><dd>{detailProduct.product.default_unit ?? "—"}</dd>
          <dt>Alert days</dt><dd>{detailProduct.product.default_alert_days_before}</dd>
          <dt>Status</dt><dd>{detailProduct.product.is_active ? "Active" : "Archived"}</dd>
          {#if detailProduct.barcodes.length > 0}
            <dt>Barcodes</dt>
            <dd>
              {#each detailProduct.barcodes as bc}
                <span class="barcode-chip" class:primary={bc.is_primary}>
                  {bc.barcode}{bc.is_primary ? " ★" : ""}
                </span>
              {/each}
            </dd>
          {/if}
        </dl>
      {/if}
    </div>
  </div>
{/if}

<!-- ── Lot detail modal ─────────────────────────────────────────────────────── -->
{#if showLotDetail}
  <div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Lot detail">
    <div class="modal-box">
      <div class="modal-header">
        <h3>Lot Detail</h3>
        <button class="modal-close" on:click={() => (showLotDetail = false)}>✕</button>
      </div>
      {#if detailLotLoading}
        <p class="modal-loading">Loading…</p>
      {:else if detailLot}
        <dl class="detail-grid">
          <dt>Lot ID</dt><dd class="cell-sku">{detailLot.id.slice(0, 8)}…</dd>
          <dt>Quantity</dt><dd>{detailLot.quantity} {detailLot.unit}</dd>
          <dt>Expiry</dt><dd>{formatDate(detailLot.expiry_date)}</dd>
          <dt>Alert days</dt><dd>{detailLot.alert_days_before}</dd>
          <dt>Batch</dt><dd>{detailLot.batch_code ?? "—"}</dd>
          <dt>Status</dt><dd>{detailLot.status}</dd>
          {#if detailLot.resolution}
            <dt>Resolution</dt><dd>{detailLot.resolution}</dd>
          {/if}
          {#if detailLot.notes}
            <dt>Notes</dt><dd>{detailLot.notes}</dd>
          {/if}
        </dl>
        <div class="modal-actions">
          <button
            class="btn-primary"
            on:click={() => {
              showLotDetail = false;
              openResolveFromDetail(detailLot!);
            }}
          >Resolve quantity</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- ── Resolve dialog ──────────────────────────────────────────────────────── -->
{#if showResolveDialog}
  <div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Resolve quantity">
    <div class="modal-box">
      <div class="modal-header">
        <h3>Resolve Quantity</h3>
        <button class="modal-close" on:click={() => (showResolveDialog = false)}>✕</button>
      </div>
      {#if resolveLot}
        <div class="resolve-info">
          <strong>{resolveLot.description}</strong>
          <br>
          Remaining: <strong>{resolveLot.quantity} {resolveLot.unit}</strong>
          — Expires: <strong>{formatDate(resolveLot.expiry_date)}</strong>
        </div>

        <div class="form-group">
          <label for="resolve-qty">Quantity to resolve</label>
          <input
            id="resolve-qty"
            type="number"
            min="0.01"
            step="0.01"
            bind:value={resolveQty}
            disabled={resolveLoading}
          />
        </div>

        <div class="form-group">
          <label for="resolve-type">Resolution type</label>
          <select id="resolve-type" bind:value={resolveType} disabled={resolveLoading}>
            {#each RESOLVE_TYPES as rt}
              <option value={rt.value}>{rt.label}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label for="resolve-notes">Notes (optional)</label>
          <textarea
            id="resolve-notes"
            rows="2"
            bind:value={resolveNotes}
            disabled={resolveLoading}
          ></textarea>
        </div>

        {#if resolveError}
          <div class="error-inline" role="alert">{resolveError}</div>
        {/if}

        <div class="modal-actions">
          <button class="btn-secondary" on:click={() => (showResolveDialog = false)} disabled={resolveLoading}>
            Cancel
          </button>
          <button class="btn-primary" on:click={submitResolve} disabled={resolveLoading}>
            {resolveLoading ? "Saving…" : "Save"}
          </button>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- ── Quick-create product modal ──────────────────────────────────────────── -->
    {#if showQuickCreate}
      <div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Quick product create">
        <div class="modal-box modal-box-wide">
          <div class="modal-header">
            <h3>Quick product create</h3>
            <button class="modal-close" on:click={() => (showQuickCreate = false)}>✕</button>
          </div>
          <p class="scan-hint">
            No product matched <strong>{quickCreateScannedValue}</strong>.
            Create it now — the scanned value has been pre-filled into the Barcode field. Type a SKU and submit.
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

  .action-btn-resolve {
    color: #2563eb;
    border-color: #bfdbfe;
  }

  .action-btn-resolve:hover {
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
        width: 600px;
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

  /* ── Resolve form ────────────────────────────────────────────────────── */
  .resolve-info {
    background: #f8fafc;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    padding: 10px 12px;
    font-size: 0.85rem;
    color: #475569;
    margin-bottom: 16px;
    line-height: 1.6;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }

  .form-group label {
    font-size: 0.82rem;
    color: #374151;
    font-weight: 500;
  }

  .form-group input,
  .form-group select,
  .form-group textarea {
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 0.85rem;
    font-family: inherit;
    background: #fff;
    color: #1e293b;
  }

  .form-group input:focus,
  .form-group select:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #2563eb;
    box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15);
  }

  .error-inline {
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 4px;
    padding: 6px 10px;
    color: #dc2626;
    font-size: 0.82rem;
    margin-bottom: 12px;
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
