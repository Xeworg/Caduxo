<script lang="ts">
      import {
        createProduct,
        updateProduct,
        createCategory,
        suggestedProductAlertDays,
        addProductBarcodeIfNew,
        type CategoryResponse,
        type ProductResponse,
      } from "../lib/products.js";

      // ── Props ──────────────────────────────────────────────────────────────────

      export let mode: "create" | "edit";
      /** Required for edit mode; ignored in create mode. */
      export let initial: ProductResponse | null = null;
      /** Master category list from the parent; used to populate the dropdown. */
      export let categories: CategoryResponse[];
      /** Called after a successful save with the saved product. */
      export let onSaved: (product: ProductResponse) => void;
      /** Called when the user cancels the form. */
      export let onCancel: () => void;
      /** Called after a successful inline category create. */
      export let onCategoryCreated: (category: CategoryResponse) => void;

      /**
       * Optional SKU pre-fill for scan/keyboard quick-create.
       * Applied only in create mode on first mount.
       */
      export let prefillSku: string | undefined = undefined;
      /**
       * Optional barcode to attach to the product after successful creation.
       * Applied only in create mode; silently ignored if the barcode is already attached.
       */
      export let prefillBarcode: string | undefined = undefined;

  // ── Local state ────────────────────────────────────────────────────────────

  let sku = "";
  let description = "";
  let categoryId: string = "";
  let defaultUnit = "";
  let defaultAlertDays = 30;
  let notes = "";
  let isActive = true;

  let submitting = false;
  let errorMsg = "";

  // Local category mirror so inline creates show up without parent round-trip.
  let localCategories: CategoryResponse[] = [];
  $: localCategories = categories;

  // Inline category create
  let addingCategory = false;
  let newCategoryName = "";
  let creatingCategory = false;
  let categoryError = "";

  // ── Init ───────────────────────────────────────────────────────────────────

  $: if (mode === "edit" && initial) {
    sku = initial.sku;
    description = initial.description;
    categoryId = initial.category_id ?? "";
    defaultUnit = initial.default_unit ?? "";
    defaultAlertDays = initial.default_alert_days_before;
    notes = initial.notes ?? "";
    isActive = initial.is_active;
  }

  // Pre-fill alert days from the backend suggestion on first create mount.
  // Pre-fill SKU from the optional prefillSku prop.
  let suggestedFetched = false;
  $: if (mode === "create" && !suggestedFetched) {
    suggestedFetched = true;
    // Apply SKU pre-fill if provided.
    if (prefillSku !== undefined) {
      sku = prefillSku;
    }
    suggestedProductAlertDays()
      .then((d) => {
        defaultAlertDays = d;
      })
      .catch(() => {
        // Keep the local default of 30.
      });
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  function resetInlineCategory() {
    addingCategory = false;
    newCategoryName = "";
    categoryError = "";
  }

  async function submitInlineCategory() {
    const name = newCategoryName.trim();
    if (!name) {
      categoryError = "Category name is required";
      return;
    }
    creatingCategory = true;
    categoryError = "";
    try {
      const created = await createCategory({ name });
      localCategories = [...localCategories, created].sort((a, b) =>
        a.name.localeCompare(b.name),
      );
      categoryId = created.id;
      onCategoryCreated(created);
      resetInlineCategory();
    } catch (e: unknown) {
      categoryError = String(e);
    } finally {
      creatingCategory = false;
    }
  }

  async function submit() {
    errorMsg = "";
    if (!sku.trim()) {
      errorMsg = "SKU is required";
      return;
    }
    if (!description.trim()) {
      errorMsg = "Description is required";
      return;
    }
    if (defaultAlertDays < 0) {
      errorMsg = "Alert days cannot be negative";
      return;
    }
    submitting = true;
    try {
      const payload = {
        sku: sku.trim(),
        description: description.trim(),
        category_id: categoryId || null,
        default_unit: defaultUnit.trim() || null,
        default_alert_days_before: defaultAlertDays,
        notes: notes.trim() || null,
      };
          let saved: ProductResponse;
          if (mode === "edit" && initial) {
            saved = await updateProduct({
              ...payload,
              id: initial.id,
              is_active: isActive,
            });
          } else {
            saved = await createProduct(payload);
            // Attach the scanned barcode to the newly created product if provided.
            if (prefillBarcode !== undefined) {
              await addProductBarcodeIfNew({
                product_id: saved.id,
                barcode: prefillBarcode,
                barcode_type: null,
                is_primary: true,
              });
            }
          }
          onSaved(saved);
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<form class="product-form" on:submit|preventDefault={submit}>
  <h3>{mode === "edit" ? "Edit product" : "Create product"}</h3>

  {#if errorMsg}
    <div class="alert alert-error" role="alert">{errorMsg}</div>
  {/if}

  <label>
    SKU *
    <input
      type="text"
      bind:value={sku}
      placeholder="e.g. MILK-1L"
      required
      autocomplete="off"
    />
  </label>

  <label>
    Description *
    <input
      type="text"
      bind:value={description}
      placeholder="e.g. Whole Milk 1L"
      required
    />
  </label>

  <div class="category-row">
    <label class="category-field">
      Category
      <select bind:value={categoryId}>
        <option value="">— None —</option>
      </select>
    </label>
    {#if localCategories.length > 0}
      <ul class="category-pills">
        {#each localCategories as cat (cat.id)}
          <li>
            <button
              type="button"
              class="pill"
              class:selected={categoryId === cat.id}
              on:click={() => (categoryId = cat.id)}
            >
              {cat.name}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if addingCategory}
    <div class="inline-category">
      <label>
        New category name
        <input
          type="text"
          bind:value={newCategoryName}
          placeholder="e.g. Dairy"
          disabled={creatingCategory}
        />
      </label>
      {#if categoryError}
        <div class="alert alert-error inline-error" role="alert">
          {categoryError}
        </div>
      {/if}
      <div class="form-actions">
        <button
          type="button"
          class="btn-primary btn-small"
          disabled={creatingCategory}
          on:click={submitInlineCategory}
        >
          {creatingCategory ? "Creating…" : "Add category"}
        </button>
        <button
          type="button"
          class="btn-secondary btn-small"
          disabled={creatingCategory}
          on:click={resetInlineCategory}
        >
          Cancel
        </button>
      </div>
    </div>
  {:else}
    <button
      type="button"
      class="btn-link"
      on:click={() => (addingCategory = true)}
    >
      + New category
    </button>
  {/if}

  <div class="grid-2">
    <label>
      Default unit
      <input
        type="text"
        bind:value={defaultUnit}
        placeholder="e.g. kg, L, unit"
      />
    </label>

    <label>
      Alert days before *
      <input
        type="number"
        bind:value={defaultAlertDays}
        min="0"
        max="3650"
        step="1"
        required
      />
    </label>
  </div>

  <label>
    Notes
    <textarea
      bind:value={notes}
      placeholder="Optional notes…"
      rows="3"
    ></textarea>
  </label>

  {#if mode === "edit"}
    <label class="checkbox-label">
      <input type="checkbox" bind:checked={isActive} />
      Active
    </label>
  {/if}

  <div class="form-actions">
    <button type="submit" class="btn-primary" disabled={submitting}>
      {submitting
        ? "Saving…"
        : mode === "edit"
          ? "Save changes"
          : "Create product"}
    </button>
    <button
      type="button"
      class="btn-secondary"
      on:click={onCancel}
      disabled={submitting}
    >
      Cancel
    </button>
  </div>
</form>

<style>
  .product-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 560px;
  }

  .product-form h3 {
    margin: 0 0 4px;
    font-size: 1rem;
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

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    color: #374151;
  }

  label input[type="text"],
  label input[type="number"],
  label textarea,
  label select {
    padding: 7px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    background: #fff;
  }

  label input:focus,
  label textarea:focus,
  label select:focus {
    outline: 2px solid #3b82f6;
    border-color: #3b82f6;
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
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

  .btn-secondary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-small {
    padding: 5px 12px;
    font-size: 0.82rem;
  }

  .btn-link {
    align-self: flex-start;
    background: none;
    border: none;
    color: #2563eb;
    cursor: pointer;
    font-size: 0.82rem;
    padding: 0;
    font-family: inherit;
  }

  .btn-link:hover {
    text-decoration: underline;
  }

  /* Category selector */
  .category-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .category-field {
    flex: none;
  }

  .category-pills {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill {
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #e5e7eb;
    border-radius: 999px;
    padding: 3px 12px;
    font-size: 0.78rem;
    cursor: pointer;
    font-family: inherit;
  }

  .pill:hover {
    background: #e5e7eb;
  }

  .pill.selected {
    background: #dbeafe;
    border-color: #3b82f6;
    color: #1e40af;
  }

  .inline-category {
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
