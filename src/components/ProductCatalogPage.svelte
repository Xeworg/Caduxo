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
  import ProductForm from "./ProductForm.svelte";
  import ProductDetailPage from "./ProductDetailPage.svelte";

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
      errorMsg = String(e);
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
      errorMsg = String(e);
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
      wasEditing ? `Updated ${product.sku}` : `Created ${product.sku}`,
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
    flash("Product archived", "success");
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
            flash(
              `Exported ${result.rows_written} ${result.rows_written === 1 ? "product" : "products"}`,
              "success",
            );
          }
        } catch (e: unknown) {
          errorMsg = String(e);
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
        <h1>Products</h1>
        {#if view === "list"}
          <div class="page-header-actions">
            <button
              class="btn-secondary"
              on:click={exportProducts}
              disabled={exporting}
              title="Export all products to a CSV file"
            >
              {exporting ? "Exporting…" : "Export CSV"}
            </button>
            <button class="btn-primary" on:click={startCreate}>
              + New Product
            </button>
          </div>
        {/if}
      </header>

  {#if errorMsg}
    <div class="alert alert-error" role="alert">{errorMsg}</div>
  {/if}
  {#if successMsg}
    <div class="alert alert-success" role="status">{successMsg}</div>
  {/if}

      {#if view === "list"}
        <!-- ── Search bar ──────────────────────────────────────────────────── -->
        <div class="search-bar">
          <input
            type="text"
            bind:value={searchQuery}
            on:input={onSearchInput}
            placeholder="Search by description, SKU, or barcode…"
            autocomplete="off"
          />
          {#if searchQuery}
            <button type="button" class="btn-secondary btn-small" on:click={clearSearch}>
              Clear
            </button>
          {/if}
          <div class="category-filter">
            <CategoryPicker
              bind:value={categoryIds}
              {categories}
              includeUncategorized={true}
              placeholder="Filter by category…"
            />
          </div>
        </div>

    {#if loading}
      <p class="loading">Loading products…</p>
    {:else if searching && results.length === 0}
      <p class="loading">Searching…</p>
    {:else if results.length === 0}
      <div class="empty-state">
        {#if appliedQuery}
          <p>No products match "<strong>{appliedQuery}</strong>".</p>
        {:else}
          <p>No products yet.</p>
          <p class="hint">Click <strong>+ New Product</strong> to add the first one.</p>
        {/if}
      </div>
    {:else}
      <div class="results-summary">
        <span>{displayedProducts.length} {displayedProducts.length === 1 ? "product" : "products"}</span>
        {#if appliedQuery}
          <span class="results-filter">for "<strong>{appliedQuery}</strong>"</span>
        {/if}
      </div>
      <ul class="product-list">
        {#each displayedProducts as product (product.id)}
          <li>
            <button
              type="button"
              class="product-item"
              class:archived={!product.is_active}
              on:click={() => openDetail(product)}
            >
              <div class="product-main">
                <span class="product-description">{product.description}</span>
                <span class="product-sku">{product.sku}</span>
              </div>
              <div class="product-meta">
                {#if product.primary_barcode}
                  <span class="product-barcode">{product.primary_barcode}</span>
                {/if}
                {#if !product.is_active}
                  <span class="badge-inactive">Archived</span>
                {/if}
              </div>
            </button>
          </li>
        {/each}
      </ul>
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

  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 0.9rem;
  }

  .alert-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
  }

  .alert-success {
    background: #dcfce7;
    color: #166534;
    border: 1px solid #86efac;
  }

  .loading {
    color: #6b7280;
    font-style: italic;
  }

  .empty-state {
    background: #fff;
    border: 1px dashed #d1d5db;
    border-radius: 10px;
    padding: 40px 20px;
    text-align: center;
  }

  .empty-state p {
    margin: 0 0 6px;
    color: #4b5563;
  }

  .empty-state .hint {
    font-size: 0.85rem;
    color: #9ca3af;
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
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
  }

  .search-bar input:focus {
    outline: 2px solid #3b82f6;
    border-color: #3b82f6;
  }

  /* Results */
  .results-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #6b7280;
    font-size: 0.82rem;
    margin-bottom: 8px;
  }

  .results-filter {
    color: #4b5563;
  }

  .product-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .product-item {
    width: 100%;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 12px 14px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    transition: background 0.15s, border-color 0.15s;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .product-item:hover {
    background: #f9fafb;
    border-color: #cbd5e1;
  }

  .product-item.archived {
    background: #f9fafb;
    opacity: 0.75;
  }

  .product-main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .product-description {
    font-weight: 500;
    font-size: 0.95rem;
    color: #1f2937;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .product-sku {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.78rem;
    color: #6b7280;
  }

  .product-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .product-barcode {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.78rem;
    color: #4b5563;
    background: #f3f4f6;
    padding: 2px 7px;
    border-radius: 4px;
  }

  .badge-inactive {
    font-size: 0.7rem;
    background: #f3f4f6;
    color: #9ca3af;
    border-radius: 4px;
    padding: 2px 7px;
  }

  /* Panel */
  .panel {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 24px;
  }

  /* Buttons */
  .btn-primary {
    background: #2563eb;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 0.9rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-primary:hover {
    background: #1d4ed8;
  }

  .btn-secondary {
    background: #fff;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 0.9rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-secondary:hover {
    background: #f9fafb;
  }

  .btn-small {
    padding: 5px 12px;
    font-size: 0.82rem;
  }
</style>
