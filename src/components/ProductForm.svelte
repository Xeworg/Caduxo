<script lang="ts">
      import {
        createProduct,
        updateProduct,
        suggestedProductAlertDays,
        addProductBarcodeOnCreate,
        type CategoryResponse,
        type ProductResponse,
        type UnitKind,
      } from "../lib/products.js";
      import {
        listUnitDefinitions,
        createUnitDefinition,
        type UnitDefinitionResponse,
      } from "../lib/unit_definitions.js";
      import CategoryPicker from "./inputs/CategoryPicker.svelte";
      import { onMount } from "svelte";
      import { LL } from "../i18n/i18n-svelte.js";

      // ── Props ──────────────────────────────────────────────────────────────────

      export let mode: "create" | "edit";
      /** Required for edit mode; ignored in create mode. */
      export let initial: ProductResponse | null = null;
      /** Master category list from the parent; used for chip display names. */
      export let categories: CategoryResponse[];
      /** Called after a successful save with the saved product. */
      export let onSaved: (product: ProductResponse) => void;
      /** Called when the user cancels the form. */
      export let onCancel: () => void;
      /** Called after a successful inline category create (from CategoryPicker). */
      export let onCategoryCreated: (category: CategoryResponse) => void;

      /**
       * Optional UPC/barcode pre-fill for the scan quick-create path.
       * Seeds the Barcode value input on first create-mount.
       */
      export let prefillUpc: string | undefined = undefined;

  // ── Local state ────────────────────────────────────────────────────────────

  let sku = "";
  let description = "";
  let categoryIds: string[] = [];
  let defaultUnit = "";
  let defaultUnitId = ""; // FK into unit_definitions; empty = no catalog link
  let defaultAlertDays = 30;
  let notes = "";
  let isActive = true;

  let submitting = false;
  let errorMsg = "";

  // Barcodes subsection state (create mode only).
  let upcValue = "";
  let upcType = "";
  let upcIsPrimary = false;
  let barcodeNotice = ""; // empty = no notice

  // ── Unit catalog ───────────────────────────────────────────────────────────
  /** Cached unit list for the datalist. */
  let unitList: UnitDefinitionResponse[] = [];
  /** Controls visibility of the inline unit creation sub-form. */
  let showInlineUnitForm = false;
  /** Key for the new unit (auto-derived from display name). */
  let newUnitKey = "";
  /** Display name for the new unit. */
  let newUnitDisplayName = "";
  /** Kind for the new unit (radio selection). */
  let newUnitKind: UnitKind = "integer";
  /** Error message for inline unit creation. */
  let unitError = "";
  /** In-progress flag for inline unit creation. */
  let creatingUnit = false;

// ── Init ───────────────────────────────────────────────────────────────────

      /**
       * Tracks which `initial.id` we've already seeded from. Reseeding only fires
       * when the product identity changes (initial mount or switching to a
       * different product); re-renders that re-supply the same `initial` object
       * leave the in-flight `categoryIds` and other field edits alone.
       * Without this guard, the CategoryPicker's local selection was being
       * clobbered by reactive re-evaluation when the parent updated `initial`.
       */
      let lastSeededProductId: string | null = null;

      $: if (mode === "edit" && initial && lastSeededProductId !== initial.id) {
        sku = initial.sku;
        description = initial.description;
        categoryIds = initial.category_ids ?? [];
        defaultUnit = initial.default_unit ?? "";
        defaultUnitId = initial.default_unit_id ?? "";
        defaultAlertDays = initial.default_alert_days_before;
        notes = initial.notes ?? "";
        isActive = initial.is_active;
        lastSeededProductId = initial.id;
      }

  // Pre-fill alert days and optional UPC from the backend on first create mount.
  let suggestedFetched = false;
  $: if (mode === "create" && !suggestedFetched) {
    suggestedFetched = true;
    // Seed UPC/barcode field from the optional prefillUpc prop.
    if (prefillUpc !== undefined) {
      upcValue = prefillUpc;
    }
    suggestedProductAlertDays()
      .then((d) => {
        defaultAlertDays = d;
      })
      .catch(() => {
        // Keep the local default of 30.
      });
      }

      // ── Unit catalog lifecycle ────────────────────────────────────────────────
      onMount(async () => {
        try {
          unitList = await listUnitDefinitions();
        } catch {
          // Non-fatal: unit list is a convenience feature.
        }
      });

      // ── Helpers ────────────────────────────────────────────────────────────────



      // ── Unit helpers ───────────────────────────────────────────────────────────

      /**
       * Singular Spanish unit-text forms that resolve to a canonical catalog
       * key on the backend. Mirrors the alias map in
       * `services::products::unit_text_aliases` so the UI guard here does not
       * surface the inline unit-create form for values the backend already
       * resolves to an existing preset (e.g. `Unidad` → `ud-units`).
       * Keep the entries aligned with the Rust alias map; expand in tandem.
       */
      const UNIT_TEXT_ALIASES: Record<string, string> = {
        unidad: "units",
        caja: "cajas",
        botella: "bottles",
        bolsa: "bags",
        paquete: "packs",
        pieza: "pcs",
      };

      /** Slugifies display_name into a valid unit key (lowercase, alphanumeric, hyphen/underscore). */
  function slugify(name: string): string {
    return name
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "")
      .slice(0, 16);
  }

  function resetInlineUnit() {
    showInlineUnitForm = false;
    newUnitKey = "";
    newUnitDisplayName = "";
    unitError = "";
  }

  /** Handles inline unit creation from the ProductForm sub-form. */
  async function submitInlineUnit() {
    const key = newUnitKey.trim();
    const displayName = newUnitDisplayName.trim();

    if (!key) {
      unitError = $LL.lotForm.keyRequired();
      return;
    }
    if (!displayName) {
      unitError = $LL.lotForm.displayNameRequired();
      return;
    }
    if (!/^[a-z0-9_-]{1,16}$/.test(key)) {
      unitError = $LL.lotForm.keyPattern();
      return;
    }
    // Check for duplicate (case-insensitive).
    const dupe = unitList.find(
      (u) => u.key === key || u.key === key.toLowerCase(),
    );
    if (dupe) {
      unitError = $LL.lotForm.keyAlreadyExists({ key, existing: dupe.display_name });
      return;
    }

    creatingUnit = true;
    unitError = "";
    try {
      const created = await createUnitDefinition({
    key,
    display_name: displayName,
    kind: newUnitKind,
      });
      // Add to local list and select it.
      unitList = [...unitList, created].sort((a, b) =>
    a.display_name.localeCompare(b.display_name),
      );
      defaultUnitId = created.id;
      defaultUnit = created.display_name;
      resetInlineUnit();
    } catch (e: unknown) {
      const msg = String(e);
      // Recoverable DuplicateField error.
        if (/duplicate|already exists|unique/i.test(msg)) {
        unitError = $LL.lotForm.keyAlreadyExistsGeneric({ key });
        } else {
        unitError = msg;
        }
    } finally {
      creatingUnit = false;
    }
  }

      async function submit() {
        errorMsg = "";
        if (!sku.trim()) {
          errorMsg = $LL.products.productSku() + " " + $LL.common.required();
          return;
        }
        if (!description.trim()) {
          errorMsg = $LL.products.productDescription() + " " + $LL.common.required();
          return;
        }
        if (defaultAlertDays < 0) {
          errorMsg = $LL.products.productAlertDays() + " " + $LL.errors.generic();
          return;
        }
        submitting = true;
        try {
          const payload = {
            sku: sku.trim(),
            description: description.trim(),
            category_ids: categoryIds.length > 0 ? categoryIds : null,
            default_unit: defaultUnit.trim() || null,
            default_unit_id: defaultUnitId || null,
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
            // Attempt to attach the UPC/barcode if the field is non-empty.
            const trimmed = upcValue.trim();
            if (trimmed !== "") {
              const result = await addProductBarcodeOnCreate({
                product_id: saved.id,
                barcode: trimmed,
                barcode_type: upcType.trim() || null,
                is_primary: upcIsPrimary,
              });
              if (!result.ok) {
                switch (result.kind) {
                  case "duplicate_other":
                    barcodeNotice = $LL.products.detail.barcode.valueRequired() + ` (${trimmed})`;
                    break;
                  case "duplicate_same":
                    barcodeNotice = result.message || $LL.products.detail.barcode.valueRequired() + ` (${trimmed})`;
                    break;
                  case "other":
                    barcodeNotice = result.message;
                    break;
                }
              }
            }
          }
          onSaved(saved);
        } catch (e: unknown) {
          errorMsg = String(e);
        } finally {
          submitting = false;
        }
      }

      /** Handler for CategoryPicker's on:create event. Propagates new categories
       *  to the parent so the parent's category list stays in sync. */
      function handleCategoryCreated(e: CustomEvent<CategoryResponse>) {
        const created = e.detail;
        if (!categories.find((c) => c.id === created.id)) {
          onCategoryCreated(created);
        }
      }
    </script>

    <form class="product-form" on:submit|preventDefault={submit}>
  <h3>{mode === "edit" ? $LL.products.editProduct() : $LL.products.createProduct()}</h3>

  {#if errorMsg}
    <div class="alert alert-error" role="alert">{errorMsg}</div>
  {/if}

  <label>
    {$LL.products.productSku()} *
    <input
      type="text"
      bind:value={sku}
      placeholder="e.g. MILK-1L"
      required
      autocomplete="off"
    />
  </label>

  <label>
    {$LL.products.productDescription()} *
    <input
      type="text"
      bind:value={description}
      placeholder="e.g. Whole Milk 1L"
      required
    />
  </label>

  <div class="category-field">
    <span class="field-label">{$LL.products.productCategory()}</span>
    <CategoryPicker
      bind:value={categoryIds}
      {categories}
      placeholder="{$LL.categoryPicker.searchPlaceholder()}"
      on:create={handleCategoryCreated}
    />
  </div>

      {#if mode === "create"}
        <section class="barcode-subsection">
          <div class="subsection-header"><h4>{$LL.products.detail.barcode.title()}</h4></div>
          <div class="grid-2">
            <label>
              {$LL.products.detail.barcode.valueLabel()}
              <input
                type="text"
                bind:value={upcValue}
                placeholder="e.g. 7501234567890"
                autocomplete="off"
              />
            </label>
            <label>
              {$LL.products.detail.barcode.typeLabel()}
              <input
                type="text"
                bind:value={upcType}
                placeholder={$LL.products.detail.barcode.typePlaceholder()}
                list="barcode-types-create"
              />
              <datalist id="barcode-types-create">
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
            <input type="checkbox" bind:checked={upcIsPrimary} />
            {$LL.products.detail.barcode.setAsPrimary()}
          </label>
          {#if barcodeNotice}
            <div class="alert alert-info inline-error" role="status">{barcodeNotice}</div>
          {/if}
        </section>
      {/if}

          <div class="grid-2">
          <label>
          {$LL.products.productUnit()}
          <div class="unit-input-row">
            <input
              type="text"
              bind:value={defaultUnit}
              list="unit-definitions-list"
              placeholder="e.g. kg, L, pcs"
              on:input={() => {
                // When the user edits the text, clear the FK so the backend
                // resolves the text fresh on save.
                defaultUnitId = "";
                showInlineUnitForm = false;
              }}
              on:blur={() => {
                // Auto-create: if the typed value matches no known unit, show the inline form.
                // The match must also recognise the singular→plural aliases the
                // backend resolver applies (e.g. "Unidad" → "units"), otherwise
                // we would surface an inline unit-create form for a value that
                // already resolves to an integer preset on the backend.
                const v = defaultUnit.trim().toLowerCase();
                const canonical = v ? UNIT_TEXT_ALIASES[v] ?? v : v;
                if (
                  v &&
                  !unitList.some(
                    (u) =>
                      u.key === v ||
                      u.key === canonical ||
                      u.display_name.toLowerCase() === v,
                  )
                ) {
                  showInlineUnitForm = true;
                  newUnitDisplayName = defaultUnit.trim();
                  newUnitKey = slugify(defaultUnit.trim());
                  newUnitKind = "integer";
                  unitError = "";
                }
              }}
            />
            <datalist id="unit-definitions-list">
              {#each unitList as unit (unit.id)}
                <option value={unit.display_name} data-id={unit.id}></option>
              {/each}
            </datalist>
            <button
              type="button"
              class="btn-link unit-add-btn"
              title={$LL.lotForm.createCustomUnit()}
              on:click={() => {
                showInlineUnitForm = !showInlineUnitForm;
                if (showInlineUnitForm && defaultUnit.trim()) {
                  newUnitDisplayName = defaultUnit.trim();
                  newUnitKey = slugify(defaultUnit.trim());
                }
                unitError = "";
              }}
            >
              + {$LL.lotForm.createCustomUnit()}
            </button>
          </div>
          {#if unitError}
            <span class="field-error">{unitError}</span>
          {/if}
        </label>

        {#if showInlineUnitForm}
          <div class="inline-unit-form">
            <div class="inline-unit-header">
              <span class="inline-unit-hint">{$LL.lotForm.createCustomUnit()}</span>
              <button
                type="button"
                class="inline-unit-close"
                aria-label={$LL.lotForm.close()}
                title={$LL.lotForm.close()}
                disabled={creatingUnit}
                on:click={resetInlineUnit}
              >
                ×
              </button>
            </div>
            <div class="inline-unit-fields">
              <label class="small-label">
                {$LL.lotForm.key()}
                <input
                  type="text"
                  bind:value={newUnitKey}
                  placeholder={$LL.lotForm.keyPlaceholder()}
                  maxlength="16"
                />
              </label>
              <label class="small-label">
                {$LL.lotForm.displayName()}
                <input
                  type="text"
                  bind:value={newUnitDisplayName}
                  placeholder={$LL.lotForm.displayNamePlaceholder()}
                />
              </label>
              <div class="kind-radios">
                <label class="radio-label">
                  <input
                    type="radio"
                    bind:group={newUnitKind}
                    value={"integer"}
                  />
                  {$LL.lotForm.integer()}
                </label>
                <label class="radio-label">
                  <input
                    type="radio"
                    bind:group={newUnitKind}
                    value={"decimal"}
                  />
                  {$LL.lotForm.decimal()}
                </label>
              </div>
              <button
                type="button"
                class="btn-primary btn-sm"
                disabled={creatingUnit}
                on:click={submitInlineUnit}
              >
                {creatingUnit ? $LL.lotForm.creating() : $LL.lotForm.addUnit()}
              </button>
            </div>
          </div>
        {/if}

        <label>
      {$LL.products.productAlertDays()}
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
    {$LL.products.productNotes()}
    <textarea
      bind:value={notes}
      placeholder={$LL.common.optional()}
      rows="3"
    ></textarea>
  </label>

  {#if mode === "edit"}
    <label class="checkbox-label">
      <input type="checkbox" bind:checked={isActive} />
      {$LL.dashboard.active()}
    </label>
  {/if}

  <div class="form-actions">
    <button type="submit" class="btn-primary" disabled={submitting}>
      {submitting
        ? $LL.common.saving()
        : mode === "edit"
          ? $LL.lotForm.saveChanges()
          : $LL.products.createProduct()}
    </button>
    <button
      type="button"
      class="btn-secondary"
      on:click={onCancel}
      disabled={submitting}
    >
      {$LL.common.cancel()}
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

  .alert-info {
    background: #fefce8;
    color: #854d0e;
    border: 1px solid #fde047;
  }

  .inline-error {
    margin: 4px 0 0;
    padding: 6px 10px;
    font-size: 0.8rem;
  }

  label,
  .category-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    color: #374151;
  }

  .field-label {
    font-weight: 500;
  }

  label input[type="text"],
  label input[type="number"],
  label textarea {
    padding: 7px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    background: #fff;
  }

  label input:focus,
  label textarea:focus {
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

  /* Barcodes subsection (create mode only) */
  .barcode-subsection {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: #f8fafc;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
  }

  .subsection-header {
    margin: 0;
  }

  .subsection-header h4 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #374151;
  }

  .unit-input-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .unit-input-row input[type="text"] {
    flex: 1;
  }

  .unit-add-btn {
    font-size: 0.8rem;
    white-space: nowrap;
    color: #3b82f6;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .unit-add-btn:hover {
    text-decoration: underline;
  }

  .inline-unit-form {
    background: #f0f9ff;
    border: 1px solid #bae6fd;
    border-radius: 8px;
    padding: 10px;
    margin-top: 6px;
  }

  .inline-unit-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .inline-unit-hint {
    font-size: 0.8rem;
    font-weight: 600;
    color: #0369a1;
  }

  .inline-unit-close {
    border: none;
    background: transparent;
    color: #0369a1;
    cursor: pointer;
    font-size: 1.2rem;
    line-height: 1;
    padding: 0 4px;
  }

  .inline-unit-close:hover:not(:disabled) {
    color: #0f172a;
  }

  .inline-unit-close:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .inline-unit-fields {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .small-label {
    font-size: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .kind-radios {
    display: flex;
    gap: 12px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.85rem;
  }

  .btn-sm {
    padding: 4px 10px;
    font-size: 0.8rem;
  }
</style>
