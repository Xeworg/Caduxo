<!-- ProductCatalogPage.svelte: searchable product catalog. Tailwind scan hints: table table-zebra cell-mono. -->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    listCategories,
    searchProducts,
    lifecycleOf,
    type CategoryResponse,
    type ProductResponse,
    type ProductSearchResult,
  } from "../lib/products.js";
  import { UNCATEGORIZED_SENTINEL } from "../lib/categories.js";
  import CategoryPicker from "./inputs/CategoryPicker.svelte";
  import { exportProductsWithDialog } from "../lib/csv.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";
  import ProductForm from "./ProductForm.svelte";
  import ProductDetailPage from "./ProductDetailPage.svelte";
  import Table from "./ui/Table.svelte";
  import Badge from "./ui/Badge.svelte";
  import Alert from "./ui/Alert.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import Button from "./ui/Button.svelte";

  // ── View state ─────────────────────────────────────────────────────────────

  type View = "list" | "create" | "edit" | "detail";
  let view: View = "list";

  let searchQuery = "";
  let appliedQuery = "";
  let results: ProductSearchResult[] = [];
  let selectedProduct: ProductResponse | null = null;
  let selectedProductId: string | null = null;

  let categoryIds: string[] = [];
  let categories: CategoryResponse[] = [];

  let loading = true;
  let searching = false;
  let errorMsg = "";
  let successMsg = "";
  let exporting = false;
  let highlightedProductId: string | null = null;
  let highlightTimer: ReturnType<typeof setTimeout> | null = null;

  // Debounce timer for search input
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  // ── List preferences (PR `product-catalog-list-preferences`) ─────────────
  //
  // Preferences live in browser localStorage so they survive reloads and
  // app restarts. There is intentionally no backend settings scope for
  // these toggles in this PR — see the ODD task doc for scope notes.
  //
  // Description + SKU are always visible (mandatory identifiers for the
  // row). The other four columns are user-toggleable; the defaults match
  // the previous fixed-column layout (Barcode, Unit, Alert days, Status).
  type OptionalColumnKey =
    | "barcode"
    | "unit"
    | "alertDays"
    | "status";

  const STORAGE_KEY_COLUMNS = "caduxo.products.catalog.columns.v1";
  const STORAGE_KEY_HIDE_ARCHIVED = "caduxo.products.catalog.hideArchived.v1";
  const STORAGE_KEY_SHOW_RETIRED = "caduxo.products.catalog.showRetired.v1";

  const DEFAULT_VISIBLE_COLUMNS: OptionalColumnKey[] = [
    "barcode",
    "unit",
    "alertDays",
    "status",
  ];

  /** Optional columns offered in the Columns menu. */
  interface ColumnOption {
    key: OptionalColumnKey;
    label: string;
  }

  let visibleColumns: Set<OptionalColumnKey> = new Set(
    DEFAULT_VISIBLE_COLUMNS,
  );
  let hideArchived = false;
  let showRetired = false;

  // Column menu options are reactive so the localized labels update when the
  // active locale changes.
  $: columnOptions = [
    { key: "barcode" as const, label: $LL.products.productBarcode() },
    { key: "unit" as const, label: $LL.products.catalog.columnUnit() },
    { key: "alertDays" as const, label: $LL.products.catalog.columnAlertDays() },
    { key: "status" as const, label: $LL.dashboard.status() },
  ] satisfies ColumnOption[];

  // Popover open state for the Columns menu. Click-outside / Escape close it.
  let columnsMenuOpen = false;
  let columnsMenuRoot: HTMLDivElement | null = null;

  // Restore preserved window scroll position after returning to the list view.
  // We save the Y position right before any transition out of the list and
  // restore it (after the list re-mounts) when we come back.
  let savedListScrollY = 0;

  // ── Init ───────────────────────────────────────────────────────────────────

  onMount(async () => {
    try {
      loadPreferences();
      categories = await listCategories();
      await runSearch("");
    } catch (e: unknown) {
      errorMsg = humanizeError(e);
    } finally {
      loading = false;
    }
  });

  // Document-level click + Escape to close the columns menu.
  // Kept as its own onMount so the callback returns a synchronous cleanup
  // function (Svelte 5's onMount rejects Promise-returning callbacks).
  onMount(() => {
    function onDocClick(event: MouseEvent) {
      if (!columnsMenuOpen) return;
      const target = event.target as Node | null;
      if (
        columnsMenuRoot &&
        target &&
        !columnsMenuRoot.contains(target)
      ) {
        columnsMenuOpen = false;
      }
    }
    function onDocKey(event: KeyboardEvent) {
      if (columnsMenuOpen && event.key === "Escape") {
        columnsMenuOpen = false;
      }
    }
    document.addEventListener("mousedown", onDocClick, true);
    document.addEventListener("keydown", onDocKey, true);
    return () => {
      document.removeEventListener("mousedown", onDocClick, true);
      document.removeEventListener("keydown", onDocKey, true);
    };
  });

  // ── Preferences (localStorage) ─────────────────────────────────────────────

  function loadPreferences() {
    if (typeof localStorage === "undefined") return;
    try {
      const rawCols = localStorage.getItem(STORAGE_KEY_COLUMNS);
      if (rawCols) {
        const parsed = JSON.parse(rawCols);
        if (Array.isArray(parsed)) {
          const next = new Set<OptionalColumnKey>();
          for (const key of parsed) {
            if (
              key === "barcode" ||
              key === "unit" ||
              key === "alertDays" ||
              key === "status"
            ) {
              next.add(key);
            }
          }
          // Fall back to defaults when storage is empty / invalid.
          visibleColumns = next.size > 0 ? next : new Set(DEFAULT_VISIBLE_COLUMNS);
        }
      }
      const rawHide = localStorage.getItem(STORAGE_KEY_HIDE_ARCHIVED);
      if (rawHide === "true") hideArchived = true;
      const rawShowRetired = localStorage.getItem(STORAGE_KEY_SHOW_RETIRED);
      if (rawShowRetired === "true") showRetired = true;
    } catch {
      // Corrupt storage → fall back to defaults silently.
      visibleColumns = new Set(DEFAULT_VISIBLE_COLUMNS);
      hideArchived = false;
    }
  }

  function persistColumns() {
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.setItem(
        STORAGE_KEY_COLUMNS,
        JSON.stringify(Array.from(visibleColumns)),
      );
    } catch {
      // localStorage may be unavailable (private mode, quota) — fail silently.
    }
  }

  function persistHideArchived() {
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.setItem(
        STORAGE_KEY_HIDE_ARCHIVED,
        hideArchived ? "true" : "false",
      );
    } catch {
      // localStorage may be unavailable — fail silently.
    }
  }

  function persistShowRetired() {
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.setItem(
        STORAGE_KEY_SHOW_RETIRED,
        showRetired ? "true" : "false",
      );
    } catch {
      // localStorage may be unavailable — fail silently.
    }
  }

  function toggleColumn(key: OptionalColumnKey, checked: boolean) {
    const next = new Set(visibleColumns);
    if (checked) next.add(key);
    else next.delete(key);
    visibleColumns = next;
    persistColumns();
  }

  // ── Search ─────────────────────────────────────────────────────────────────

  function onSearchInput() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => runSearch(searchQuery), 200);
  }

  async function runSearch(query: string) {
    searching = true;
    try {
      appliedQuery = query;
      results = await searchProducts({ query });
    } catch (e: unknown) {
      errorMsg = humanizeError(e);
    } finally {
      searching = false;
    }
  }

  function clearSearch() {
    searchQuery = "";
    if (searchTimer) clearTimeout(searchTimer);
    runSearch("");
  }

  function flash(msg: string, kind: "success" | "error") {
    if (kind === "success") {
      successMsg = msg;
      setTimeout(() => (successMsg = ""), 3000);
    } else {
      errorMsg = msg;
      setTimeout(() => (errorMsg = ""), 5000);
    }
  }

  function highlightProduct(productId: string) {
    if (highlightTimer) clearTimeout(highlightTimer);
    highlightedProductId = productId;
    highlightTimer = setTimeout(() => {
      highlightedProductId = null;
      highlightTimer = null;
    }, 1200);
  }

  // ── View transitions (with scroll preservation) ───────────────────────────

  function captureListScroll() {
    if (view !== "list" || typeof window === "undefined") return;
    savedListScrollY = window.scrollY;
  }

  function restoreListScroll() {
    if (typeof window === "undefined") return;
    // Wait for the list view to actually paint before restoring the Y
    // position, otherwise the browser may clamp to the pre-layout height.
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        window.scrollTo({ top: savedListScrollY, left: 0, behavior: "auto" });
      });
    });
  }

  function startCreate() {
    captureListScroll();
    selectedProduct = null;
    view = "create";
  }

  function openDetail(product: ProductSearchResult) {
    captureListScroll();
    selectedProductId = product.id;
    view = "detail";
  }

  function cancelForm() {
    selectedProduct = null;
    view = "list";
    restoreListScroll();
  }

  async function handleSaved(product: ProductResponse) {
    const wasEditing = view === "edit";
    if (!wasEditing) {
      flash($LL.products.createProduct() + ` ${product.sku}`, "success");
    }
    selectedProduct = null;
    view = "list";
    await runSearch(appliedQuery);
    restoreListScroll();
    if (wasEditing) {
      highlightProduct(product.id);
    }
  }

  function handleCategoryCreated(category: CategoryResponse) {
    if (!categories.find((c) => c.id === category.id)) {
      categories = [...categories, category].sort((a, b) =>
        a.name.localeCompare(b.name),
      );
    }
  }

  function handleArchived() {
    flash($LL.products.archived(), "success");
    selectedProductId = null;
    view = "list";
    runSearch(appliedQuery).finally(restoreListScroll);
  }

      function handleEditedFromDetail(product: ProductResponse) {
        selectedProduct = product;
        view = "edit";
      }

      // ── CSV export (Slice 10a) ───────────────────────────────────────────────

      async function exportProducts() {
        exporting = true;
        try {
          const result = await exportProductsWithDialog();
          if (result) {
            const rows = result.rows_written;
            flash(
              `${$LL.products.catalog.exportCsv()} — ${rows} ${rows === 1 ? $LL.products.catalog.productCount({ n: rows }) : $LL.products.catalog.productCount_plural({ n: rows })}`,
              "success",
            );
          }
        } catch (e: unknown) {
          errorMsg = humanizeError(e);
          setTimeout(() => (errorMsg = ""), 5000);
        } finally {
              exporting = false;
            }
          }

      /**
       * Products matching the active search query, filtered client-side by:
       *   1. The active category picker selection (existing behaviour).
       *   2. The hide-archived toggle (new in PR
       *      `product-catalog-list-preferences`).
       */
      $: displayedProducts = (() => {
        let filtered = results;
        if (hideArchived) {
          filtered = filtered.filter((p: ProductSearchResult) => p.is_active);
        }
        if (!showRetired) {
          filtered = filtered.filter((p: ProductSearchResult) => lifecycleOf(p) !== "retired");
        }
        if (categoryIds.length === 0) return filtered;
        const hasUncat = categoryIds.includes(UNCATEGORIZED_SENTINEL);
        return filtered.filter((p: ProductSearchResult) => {
          if (p.category_ids.length === 0) return hasUncat;
          return p.category_ids.some((cid: string) => categoryIds.includes(cid));
        });
      })();
    </script>

    <div class="page">
      <header class="page-header">
        <h1>{$LL.products.catalog.pageTitle()}</h1>
        {#if view === "list"}
          <div class="page-header-actions">
            <button
              class="btn-secondary"
              onclick={exportProducts}
              disabled={exporting}
              title={$LL.products.catalog.exportCsvTitle()}
            >
              {exporting ? $LL.products.catalog.exporting() : $LL.products.catalog.exportCsv()}
            </button>
            <button class="btn-primary" onclick={startCreate}>
              + {$LL.products.createProduct()}
            </button>
          </div>
        {/if}
      </header>

  {#if errorMsg}
    <Alert variant="error">{errorMsg}</Alert>
  {/if}
  {#if successMsg}
    <Alert variant="success">{successMsg}</Alert>
  {/if}

      {#if view === "list"}
        <!-- ── Search bar ──────────────────────────────────────────────────── -->
        <div class="search-bar">
          <input
            class="search-input"
            type="text"
            bind:value={searchQuery}
            oninput={onSearchInput}
            placeholder={$LL.products.catalog.searchPlaceholder()}
            autocomplete="off"
          />
          {#if searchQuery}
            <button type="button" class="btn-secondary btn-small" onclick={clearSearch}>
              {$LL.products.catalog.clear()}
            </button>
          {/if}
          <div class="category-filter">
            <CategoryPicker
              bind:value={categoryIds}
              {categories}
              includeUncategorized={true}
            />
          </div>

          <!-- Hide-archived toggle (PR `product-catalog-list-preferences`).
               Persisted in localStorage. Default false. -->
          <div class="hide-archived-wrap">
            <Toggle
              checked={hideArchived}
              size="sm"
              label={hideArchived
                ? $LL.products.catalog.hideArchived()
                : $LL.products.catalog.showArchived()}
              onchange={(checked) => {
                hideArchived = checked;
                persistHideArchived();
              }}
            />
          </div>

          <!-- Show-retired toggle (PR `product-lifecycle-reusable-identifiers`).
               Persisted in localStorage. Default false. -->
          <div class="show-retired-wrap">
            <Toggle
              checked={showRetired}
              size="sm"
              label={showRetired
                ? "Hide retired products"
                : ($LL.products.catalog.showRetired?.() ?? "Show retired products")}
              onchange={(checked) => {
                showRetired = checked;
                persistShowRetired();
              }}
            />
          </div>

          <!-- Columns menu: toggles optional columns (Barcode / Unit /
               Alert days / Status). Description and SKU remain always
               visible. Preferences persist in localStorage. -->
          <div class="columns-menu-wrap" bind:this={columnsMenuRoot}>
            <Button
              variant="secondary"
              size="sm"
              aria-label={$LL.products.catalog.toggleColumnsAria()}
              onclick={() => (columnsMenuOpen = !columnsMenuOpen)}
            >
              {$LL.products.catalog.toggleColumns()}
            </Button>
            {#if columnsMenuOpen}
              <div class="columns-menu" role="group" aria-label={$LL.products.catalog.toggleColumnsAria()}>
                {#each columnOptions as opt (opt.key)}
                  <label class="columns-menu-item">
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm checkbox-primary"
                      checked={visibleColumns.has(opt.key)}
                      onchange={(e) => toggleColumn(opt.key, (e.currentTarget as HTMLInputElement).checked)}
                    />
                    <span>{opt.label}</span>
                  </label>
                {/each}
              </div>
            {/if}
          </div>
        </div>

    {#if loading}
      <p class="loading">{$LL.products.catalog.loading()}</p>
    {:else if searching && results.length === 0}
      <p class="loading">{$LL.products.catalog.searching()}</p>
    {:else if results.length === 0}
      <div class="empty-state">
        {#if appliedQuery}
          <p>{$LL.products.catalog.noProductsMatch({ query: appliedQuery })}</p>
        {:else}
          <p>{$LL.products.catalog.noProductsYet()}</p>
          <p class="hint">{$LL.products.catalog.noProductsYetHint()}</p>
        {/if}
      </div>
    {:else}
      <div class="results-summary">
        <span>{displayedProducts.length === 1
          ? $LL.products.catalog.productCount({ n: displayedProducts.length })
          : $LL.products.catalog.productCount_plural({ n: displayedProducts.length })}</span>
        {#if appliedQuery}
          <span class="results-filter">{$LL.products.catalog.forQuery({ query: appliedQuery })}</span>
        {/if}
      </div>
      <Table zebra aria-label={$LL.products.catalog.pageTitle()}>
        {#snippet head()}
          <tr>
            <th>{$LL.products.productDescription()}</th>
            <th>{$LL.products.productSku()}</th>
            {#if visibleColumns.has("barcode")}
              <th>{$LL.products.productBarcode()}</th>
            {/if}
            {#if visibleColumns.has("unit")}
              <th>{$LL.products.catalog.columnUnit()}</th>
            {/if}
            {#if visibleColumns.has("alertDays")}
              <th>{$LL.products.catalog.columnAlertDays()}</th>
            {/if}
            {#if visibleColumns.has("status")}
              <th>{$LL.dashboard.status()}</th>
            {/if}
          </tr>
        {/snippet}
        {#snippet body()}
          {#each displayedProducts as product (product.id)}
            <tr
              class="product-row"
              class:archived={!product.is_active}
              class:highlighted={highlightedProductId === product.id}
              tabindex="0"
              onclick={() => openDetail(product)}
              onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  openDetail(product);
                }
              }}
            >
              <td>
                <button
                  type="button"
                  class="product-link"
                  onclick={(e) => { e.stopPropagation(); openDetail(product); }}
                >
                  {product.description}
                </button>
              </td>
              <td class="cell-mono">{product.sku}</td>
              {#if visibleColumns.has("barcode")}
                <td class="cell-mono">{product.primary_barcode ?? "—"}</td>
              {/if}
              {#if visibleColumns.has("unit")}
                <td>{product.default_unit ?? "—"}</td>
              {/if}
              {#if visibleColumns.has("alertDays")}
                <td class="cell-mono">{product.default_alert_days_before}</td>
              {/if}
              {#if visibleColumns.has("status")}
                <td>
                  {#if lifecycleOf(product) === "retired"}
                    <Badge semantic="warning" size="sm">{$LL.products.lifecycleRetired()}</Badge>
                  {:else if product.is_active}
                    <Badge semantic="success" size="sm">{$LL.stores.active()}</Badge>
                  {:else}
                    <Badge semantic="neutral" size="sm">{$LL.products.archived()}</Badge>
                  {/if}
                </td>
              {/if}
            </tr>
          {/each}
        {/snippet}
      </Table>
    {/if}

  {:else if view === "create"}
    <section class="panel">
      <ProductForm
        mode="create"
        {categories}
        onSaved={handleSaved}
        onCancel={cancelForm}
        onCategoryCreated={handleCategoryCreated}
      />
    </section>

  {:else if view === "edit" && selectedProduct}
    <section class="panel">
      <ProductForm
        mode="edit"
        initial={selectedProduct}
        {categories}
        onSaved={handleSaved}
        onCancel={cancelForm}
        onCategoryCreated={handleCategoryCreated}
      />
    </section>

  {:else if view === "detail" && selectedProductId}
    <section class="panel">
      <ProductDetailPage
        productId={selectedProductId!}
        onBack={cancelForm}
        onEdit={handleEditedFromDetail}
        onArchived={handleArchived}
      />
    </section>
  {/if}
</div>

<style>
  .page {
    padding: 24px 32px;
    max-width: 1100px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }

  .page-header h1 {
    margin: 0;
    font-size: 1.5rem;
  }

  .loading {
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    font-style: italic;
  }

  .empty-state {
    background: var(--color-base-100);
    border: 1px dashed var(--color-base-300);
    border-radius: 10px;
    padding: 40px 20px;
    text-align: center;
  }

  .empty-state p {
    margin: 0 0 6px;
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
  }

  .empty-state .hint {
    font-size: 0.85rem;
    color: color-mix(in oklch, var(--color-base-content) 55%, transparent);
  }

  /* Search bar */
  .search-bar {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
    flex-wrap: wrap;
    align-items: center;
  }

  .search-input {
    flex: 1 1 320px;
    padding: 8px 12px;
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    background: var(--color-base-100);
    color: var(--color-base-content);
    min-width: 240px;
  }

  .search-input:focus {
    outline: 2px solid var(--color-primary);
    border-color: var(--color-primary);
  }

  .category-filter {
    flex: 0 1 auto;
  }

  .hide-archived-wrap {
    flex: 0 0 auto;
  }

  .columns-menu-wrap {
    position: relative;
    flex: 0 0 auto;
  }

  .columns-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 20;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-300);
    border-radius: 10px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 190px;
    box-shadow: 0 8px 24px color-mix(in oklch, black 16%, transparent);
  }

  .columns-menu-item {
    display: grid;
    grid-template-columns: 16px 1fr;
    align-items: center;
    gap: 10px;
    padding: 7px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.88rem;
    line-height: 1.2;
    color: var(--color-base-content);
  }

  .columns-menu-item input[type="checkbox"] {
    width: 16px;
    height: 16px;
    min-width: 16px;
    margin: 0;
    padding: 0;
    flex: 0 0 auto;
  }

  .columns-menu-item:hover {
    background: var(--color-base-200);
  }

  /* Results */
  .results-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    font-size: 0.82rem;
    margin-bottom: 8px;
  }

  .results-filter {
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
  }

  .product-row {
    cursor: pointer;
  }

  .product-row:hover {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
  }

  .product-row.archived {
    opacity: 0.65;
    background: color-mix(in oklch, var(--color-base-200) 90%, transparent);
  }

  .product-row.highlighted {
    animation: product-row-highlight 1.2s ease-out;
  }

  @keyframes product-row-highlight {
    0%,
    100% {
      background: inherit;
      box-shadow: none;
    }
    20%,
    65% {
      background: color-mix(in oklch, var(--color-primary) 18%, var(--color-base-100));
      box-shadow: inset 4px 0 0 var(--color-primary);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .product-row.highlighted {
      animation: none;
      background: color-mix(in oklch, var(--color-primary) 14%, var(--color-base-100));
      box-shadow: inset 4px 0 0 var(--color-primary);
    }
  }

  .product-row:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .product-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--color-primary);
    cursor: pointer;
    font: inherit;
    text-align: left;
  }

  .product-link:hover {
    text-decoration: underline;
  }

  .cell-mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.85rem;
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
  }

  /* Panel */
  .panel {
    background: var(--color-base-100);
    border: 1px solid var(--color-base-300);
    border-radius: 10px;
    padding: 24px;
  }

  /* Local buttons (kept for markup compatibility; tokens now drive colour) */
  .btn-primary {
    background: var(--color-primary);
    color: var(--color-primary-content);
    border: none;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 0.9rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-primary:hover {
    background: color-mix(in oklch, var(--color-primary) 88%, black);
  }

  .btn-secondary {
    background: var(--color-base-100);
    color: var(--color-base-content);
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 0.9rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-secondary:hover {
    background: var(--color-base-200);
  }

  .btn-small {
    padding: 5px 12px;
    font-size: 0.82rem;
  }
</style>
