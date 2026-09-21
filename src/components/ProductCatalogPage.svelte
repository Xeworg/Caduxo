<!--
  ProductCatalogPage.svelte — searchable product catalog (PR 9b of
  caduxo-daisyui-redesign).

  Migration to shared UI primitives:
    - Table.svelte (zebra) hosts the per-product list. Numeric /
      monospaced cells (SKU + barcode) use the local `.cell-mono`
      utility; the description cell renders a `<button
      class="product-link">` so screen readers can announce the
      row action without depending on the row-level click handler.
    - Badge.svelte renders the active / archived status cell. Active
      uses `semantic="success"`; archived uses `semantic="neutral"`
      and reuses the existing `$LL.products.archived()` label.
    - The "Status" column header reuses `$LL.dashboard.status()`
      because no `products.table.status` key exists (no new i18n
      keys per PR 9b's no-new-keys constraint). The same key is
      used by the dashboard table; the cross-namespace reuse is
      intentional because "Status" is a generic English / Spanish
      word.

  Tailwind classes referenced here (for the JIT scanner):
    table table-zebra
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    listCategories,
    searchProducts,
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

  // Debounce timer for search input
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Init ───────────────────────────────────────────────────────────────────

  onMount(async () => {
    try {
      categories = await listCategories();
      await runSearch("");
    } catch (e: unknown) {
      errorMsg = humanizeError(e);
    } finally {
      loading = false;
    }
  });

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

  // ── View transitions ──────────────────────────────────────────────────────

  function startCreate() {
    selectedProduct = null;
    view = "create";
  }

  function startEdit(product: ProductResponse) {
    selectedProduct = product;
    view = "edit";
  }

  function openDetail(product: ProductSearchResult) {
    selectedProductId = product.id;
    view = "detail";
  }

  function cancelForm() {
    selectedProduct = null;
    view = "list";
  }

  async function handleSaved(product: ProductResponse) {
    const wasEditing = view === "edit";
    flash(
      wasEditing ? $LL.products.detail.edit() + ` ${product.sku}` : $LL.products.createProduct() + ` ${product.sku}`,
      "success",
    );
    selectedProduct = null;
    view = "list";
    await runSearch(appliedQuery);
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
    runSearch(appliedQuery);
  }

      function handleEditedFromDetail(product: ProductResponse) {
        startEdit(product);
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
       * Products matching the active search query.
       * When `categoryIds` is non-empty, further filtered client-side by category.
       */
      $: displayedProducts = (() => {
        if (categoryIds.length === 0) return results;
        const hasUncat = categoryIds.includes(UNCATEGORIZED_SENTINEL);
        return results.filter((p: ProductSearchResult) => {
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
              on:click={exportProducts}
              disabled={exporting}
              title={$LL.products.catalog.exportCsvTitle()}
            >
              {exporting ? $LL.products.catalog.exporting() : $LL.products.catalog.exportCsv()}
            </button>
            <button class="btn-primary" on:click={startCreate}>
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
            type="text"
            bind:value={searchQuery}
            on:input={onSearchInput}
            placeholder={$LL.products.catalog.searchPlaceholder()}
            autocomplete="off"
          />
          {#if searchQuery}
            <button type="button" class="btn-secondary btn-small" on:click={clearSearch}>
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
            <th>{$LL.products.productBarcode()}</th>
            <th>{$LL.dashboard.status()}</th>
          </tr>
        {/snippet}
        {#snippet body()}
          {#each displayedProducts as product (product.id)}
            <tr
              class="product-row"
              class:archived={!product.is_active}
              tabindex="0"
              on:click={() => openDetail(product)}
              on:keydown={(e) => {
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
                  on:click|stopPropagation={() => openDetail(product)}
                >
                  {product.description}
                </button>
              </td>
              <td class="cell-mono">{product.sku}</td>
              <td class="cell-mono">{product.primary_barcode ?? "—"}</td>
              <td>
                {#if product.is_active}
                  <Badge semantic="success" size="sm">{$LL.stores.active()}</Badge>
                {:else}
                  <Badge semantic="neutral" size="sm">{$LL.products.archived()}</Badge>
                {/if}
              </td>
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
  }

  .search-bar input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    background: var(--color-base-100);
    color: var(--color-base-content);
  }

  .search-bar input:focus {
    outline: 2px solid var(--color-primary);
    border-color: var(--color-primary);
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
