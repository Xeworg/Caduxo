<script lang="ts">
  import { onMount } from "svelte";
  import {
    getProduct,
    addProductBarcode,
    removeProductBarcode,
    archiveProduct,
    type ProductDetailResponse,
    type ProductResponse,
    type ProductBarcodeResponse,
  } from "../lib/products.js";

  // ── Props ──────────────────────────────────────────────────────────────────

  export let productId: string;
  export let onBack: () => void;
  export let onEdit: (product: ProductResponse) => void;
  export let onArchived: () => void;

  // ── State ──────────────────────────────────────────────────────────────────

  let detail: ProductDetailResponse | null = null;
  let loading = true;
  let errorMsg = "";

  // Barcode add form
  let barcodeValue = "";
  let barcodeType = "";
  let isPrimary = false;
  let addingBarcode = false;
  let barcodeError = "";

  // Archive confirmation
  let confirmingArchive = false;

  // ── Load ───────────────────────────────────────────────────────────────────

  async function load() {
    loading = true;
    errorMsg = "";
    try {
      detail = await getProduct(productId);
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  // Reload when productId changes (e.g. parent switches products).
  $: if (productId) {
    load();
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  function resetBarcodeForm() {
    barcodeValue = "";
    barcodeType = "";
    isPrimary = false;
    barcodeError = "";
  }

  async function submitBarcode() {
    if (!detail) return;
    const value = barcodeValue.trim();
    if (!value) {
      barcodeError = "Barcode value is required";
      return;
    }
    addingBarcode = true;
    barcodeError = "";
    try {
      await addProductBarcode({
        product_id: detail.product.id,
        barcode: value,
        barcode_type: barcodeType.trim() || null,
        is_primary: isPrimary,
      });
      // Re-fetch detail so the list reorders with primary first.
      await load();
      resetBarcodeForm();
    } catch (e: unknown) {
      barcodeError = String(e);
    } finally {
      addingBarcode = false;
    }
  }

  async function removeBarcode(b: ProductBarcodeResponse) {
    try {
      await removeProductBarcode({ id: b.id });
      await load();
    } catch (e: unknown) {
      errorMsg = String(e);
    }
  }

  async function confirmArchive() {
    if (!detail) return;
    try {
      await archiveProduct(detail.product.id);
      confirmingArchive = false;
      onArchived();
    } catch (e: unknown) {
      errorMsg = String(e);
    }
  }
</script>

<div class="detail-page">
  <div class="detail-toolbar">
    <button type="button" class="btn-secondary btn-small" on:click={onBack}>
      ← Back to list
    </button>
  </div>

  {#if loading && !detail}
    <p class="loading">Loading product…</p>
  {:else if errorMsg && !detail}
    <div class="alert alert-error" role="alert">{errorMsg}</div>
    <button type="button" class="btn-secondary" on:click={load}>Retry</button>
  {:else if detail}
    {@const product = detail.product}
    <header class="detail-header">
      <div>
        <div class="title-row">
          <h2>{product.description}</h2>
          {#if !product.is_active}
            <span class="badge-inactive">Archived</span>
          {/if}
        </div>
        <div class="meta-row">
          <span class="sku">SKU: {product.sku}</span>
          {#if detail.category}
            <span class="category-badge">{detail.category.name}</span>
          {/if}
          {#if product.default_unit}
            <span class="meta">Unit: {product.default_unit}</span>
          {/if}
          <span class="meta">Alert: {product.default_alert_days_before} days</span>
        </div>
        {#if product.notes}
          <p class="notes">{product.notes}</p>
        {/if}
      </div>

      <div class="detail-actions">
        <button
          type="button"
          class="btn-secondary"
          on:click={() => onEdit(product)}
        >
          Edit
        </button>
        {#if product.is_active}
          {#if !confirmingArchive}
            <button
              type="button"
              class="btn-danger"
              on:click={() => (confirmingArchive = true)}
            >
              Archive
            </button>
          {:else}
            <span class="archive-confirm">
              Archive this product?
              <button
                type="button"
                class="btn-danger btn-small"
                on:click={confirmArchive}
              >
                Yes, archive
              </button>
              <button
                type="button"
                class="btn-secondary btn-small"
                on:click={() => (confirmingArchive = false)}
              >
                Cancel
              </button>
            </span>
          {/if}
        {/if}
      </div>
    </header>

    {#if errorMsg}
      <div class="alert alert-error" role="alert">{errorMsg}</div>
    {/if}

    <!-- ── Barcodes ──────────────────────────────────────────────────────── -->
    <section class="section">
      <div class="section-header">
        <h3>Barcodes</h3>
      </div>

      {#if detail.barcodes.length === 0}
        <p class="empty-hint">No barcodes yet.</p>
      {:else}
        <ul class="barcode-list">
          {#each detail.barcodes as b (b.id)}
            <li class="barcode-item">
              <div class="barcode-info">
                <span class="barcode-value">{b.barcode}</span>
                {#if b.barcode_type}
                  <span class="barcode-type">{b.barcode_type}</span>
                {/if}
                {#if b.is_primary}
                  <span class="badge-primary">Primary</span>
                {/if}
              </div>
              <button
                type="button"
                class="btn-icon"
                title="Remove barcode"
                on:click={() => removeBarcode(b)}
              >
                ✕
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if product.is_active}
        <form class="barcode-form" on:submit|preventDefault={submitBarcode}>
          <div class="grid-2">
            <label>
              Barcode value *
              <input
                type="text"
                bind:value={barcodeValue}
                placeholder="e.g. 7501234567890"
                required
                autocomplete="off"
              />
            </label>
            <label>
              Type (optional)
              <input
                type="text"
                bind:value={barcodeType}
                placeholder="e.g. EAN13, UPC"
                list="barcode-types"
              />
              <datalist id="barcode-types">
                <option value="EAN13"></option>
                <option value="EAN8"></option>
                <option value="UPC"></option>
                <option value="CODE128"></option>
                <option value="CODE39"></option>
                <option value="QR"></option>
              </datalist>
            </label>
          </div>
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={isPrimary} />
            Set as primary
          </label>
          {#if barcodeError}
            <div class="alert alert-error inline-error" role="alert">
              {barcodeError}
            </div>
          {/if}
          <div class="form-actions">
            <button
              type="submit"
              class="btn-primary btn-small"
              disabled={addingBarcode}
            >
              {addingBarcode ? "Adding…" : "+ Add barcode"}
            </button>
          </div>
        </form>
      {:else}
        <p class="hint-muted">Archived products cannot receive new barcodes.</p>
      {/if}
    </section>

    <!-- ── Expiry lots placeholder ─────────────────────────────────────── -->
    <section class="section">
      <div class="section-header">
        <h3>Expiry lots</h3>
      </div>
      <div class="placeholder">
        <p>
          Expiry lot tracking for this product will appear here. Lot CRUD is
          scheduled for the next slice.
        </p>
      </div>
    </section>
  {/if}
</div>

<style>
  .detail-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .detail-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .loading {
    color: #6b7280;
    font-style: italic;
  }

  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.9rem;
  }

  .alert-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
  }

  .inline-error {
    margin: 4px 0 0;
    padding: 6px 10px;
    font-size: 0.8rem;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .detail-header h2 {
    margin: 0;
    font-size: 1.3rem;
  }

  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 6px;
    align-items: center;
  }

  .sku {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.85rem;
    background: #f3f4f6;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .category-badge {
    font-size: 0.78rem;
    background: #dbeafe;
    color: #1e40af;
    padding: 2px 8px;
    border-radius: 999px;
  }

  .meta {
    font-size: 0.82rem;
    color: #4b5563;
  }

  .notes {
    margin: 8px 0 0;
    font-size: 0.88rem;
    color: #4b5563;
    white-space: pre-wrap;
  }

  .detail-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .archive-confirm {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
    color: #991b1b;
  }

  /* Sections */
  .section {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 18px 20px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .section-header h3 {
    margin: 0;
    font-size: 1rem;
  }

  /* Barcodes */
  .barcode-list {
    list-style: none;
    margin: 0 0 14px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .barcode-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    background: #f9fafb;
    border-radius: 6px;
  }

  .barcode-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .barcode-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.85rem;
  }

  .barcode-type {
    font-size: 0.74rem;
    color: #6b7280;
    background: #f3f4f6;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .badge-primary {
    font-size: 0.7rem;
    background: #dcfce7;
    color: #166534;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .badge-inactive {
    font-size: 0.7rem;
    background: #f3f4f6;
    color: #9ca3af;
    border-radius: 4px;
    padding: 2px 7px;
  }

  .btn-icon {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.95rem;
    padding: 2px 6px;
    color: #6b7280;
  }

  .btn-icon:hover {
    color: #991b1b;
  }

  /* Barcode add form */
  .barcode-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-top: 1px dashed #e5e7eb;
    padding-top: 14px;
    margin-top: 4px;
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    color: #374151;
  }

  label input[type="text"] {
    padding: 7px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
  }

  label input:focus {
    outline: 2px solid #3b82f6;
    border-color: #3b82f6;
  }

  .checkbox-label {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    width: auto;
  }

  .form-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
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

  .btn-primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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

  .btn-secondary:hover:not(:disabled) {
    background: #f9fafb;
  }

  .btn-danger {
    background: #fff;
    color: #991b1b;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 0.9rem;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-danger:hover {
    background: #fee2e2;
  }

  .btn-small {
    padding: 5px 12px;
    font-size: 0.82rem;
  }

  .empty-hint {
    color: #9ca3af;
    font-size: 0.85rem;
    font-style: italic;
    margin: 0 0 10px;
  }

  .hint-muted {
    color: #9ca3af;
    font-size: 0.82rem;
    font-style: italic;
    margin: 8px 0 0;
  }

  /* Placeholder */
  .placeholder {
    background: #f9fafb;
    border: 1px dashed #d1d5db;
    border-radius: 8px;
    padding: 18px;
  }

  .placeholder p {
    margin: 0;
    color: #6b7280;
    font-size: 0.88rem;
    line-height: 1.5;
  }
</style>
