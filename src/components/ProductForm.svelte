<!--
  ProductForm.svelte — product create/edit form (PR 8a of
  caduxo-daisyui-redesign).

  Full migration to the shared UI primitives:
    - Input.svelte for text/number fields.
    - Combobox.svelte for the free-text-with-suggestions fields
      (barcode type + product unit definitions). The Combobox
      primitive renders an Svelte-themed popover in lieu of the
      native `<datalist>` so the visible suggestions carry theme
      tokens (no OS-styled WebKit popup).
    - Button.svelte for primary / ghost / link actions.
    - Alert.svelte for error + barcode-notice surfaces.
    - DaisyUI `checkbox checkbox-primary checkbox-sm` wrapper for
      the "set as primary" + "active" checkboxes (native <input>
      stays in the DOM for form semantics).
    - DaisyUI `radio radio-primary radio-sm` wrapper for the
      integer/decimal kind radios.
  The CategoryPicker + DatePicker primitives are owned by their
  own components and are NOT migrated in this PR.

  Tailwind classes referenced here (for the JIT scanner):
    input input-md input-error
    checkbox checkbox-primary checkbox-sm
    radio radio-primary radio-sm
    textarea
    btn btn-primary btn-ghost btn-link btn-square
    alert alert-error alert-info alert-soft
    flex items-center justify-between gap-2
-->
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
  import Input from "./ui/Input.svelte";
  import Combobox from "./ui/Combobox.svelte";
  import Button from "./ui/Button.svelte";
  import Alert from "./ui/Alert.svelte";
  import { onMount } from "svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";

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
  // Alert days is bound via a string bridge because Input.svelte's `value`
  // contract is `string` (HTML <input type="number"> round-trips through a
  // string). The derived `defaultAlertDays` is the numeric value used for
  // submit-time validation and the backend payload.
  let defaultAlertDaysStr = "30";
  let defaultAlertDays = 30;
  $: defaultAlertDays =
    Number.parseInt(defaultAlertDaysStr, 10) || 0;
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
  /** Cached unit list surfaced as Combobox suggestions. */
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
    defaultAlertDaysStr = String(initial.default_alert_days_before);
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
        defaultAlertDaysStr = String(d);
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
      const msg = humanizeError(e);
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
      errorMsg = humanizeError(e);
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

  // Called from the unit input on:blur. Auto-creates an inline unit create
  // form when the typed value matches no known unit (mirrors the original
  // on:blur handler; the Input.svelte's onblur prop forwards the native
  // focus event).
  function handleUnitBlur() {
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
  }

  // Called from the unit input on:input. When the user edits the text,
  // clear the FK so the backend resolves the text fresh on save.
  function handleUnitInput() {
    defaultUnitId = "";
    showInlineUnitForm = false;
  }

  function toggleInlineUnitForm() {
    showInlineUnitForm = !showInlineUnitForm;
    if (showInlineUnitForm && defaultUnit.trim()) {
      newUnitDisplayName = defaultUnit.trim();
      newUnitKey = slugify(defaultUnit.trim());
    }
    unitError = "";
  }
</script>

<form class="product-form" on:submit|preventDefault={submit}>
  <h3 class="form-title">
    {mode === "edit" ? $LL.products.editProduct() : $LL.products.createProduct()}
  </h3>

  {#if errorMsg}
    <Alert variant="error">{errorMsg}</Alert>
  {/if}

  <Input
    bind:value={sku}
    label={$LL.products.productSku()}
    required
    placeholder={$LL.products.placeholders.sku()}
  />

  <Input
    bind:value={description}
    label={$LL.products.productDescription()}
    required
    placeholder={$LL.products.placeholders.description()}
  />

  <fieldset class="fieldset">
    <legend class="fieldset-legend">{$LL.products.productCategory()}</legend>
    <CategoryPicker
      bind:value={categoryIds}
      {categories}
      placeholder={$LL.categoryPicker.searchPlaceholder()}
      on:create={handleCategoryCreated}
    />
  </fieldset>

  {#if mode === "create"}
    <section class="barcode-subsection">
      <h4 class="subsection-title">{$LL.products.detail.barcode.title()}</h4>
      <div class="grid-2">
        <Input
          bind:value={upcValue}
          label={$LL.products.detail.barcode.valueLabel()}
          placeholder={$LL.products.placeholders.barcode()}
        />
        <Combobox
          bind:value={upcType}
          label={$LL.products.detail.barcode.typeLabel()}
          placeholder={$LL.products.detail.barcode.typePlaceholder()}
          options={["EAN13", "EAN8", "UPC", "CODE128", "CODE39", "QR"]}
        />
      </div>
      <label class="checkbox-wrapper">
        <input
          type="checkbox"
          class="checkbox checkbox-primary checkbox-sm"
          bind:checked={upcIsPrimary}
        />
        <span>{$LL.products.detail.barcode.setAsPrimary()}</span>
      </label>
      {#if barcodeNotice}
        <Alert variant="info">{barcodeNotice}</Alert>
      {/if}
    </section>
  {/if}

  <div class="grid-2">
    <div class="unit-field-col">
      <Combobox
        bind:value={defaultUnit}
        label={$LL.products.productUnit()}
        placeholder={$LL.products.placeholders.unit()}
        options={unitList.map((u) => ({ value: u.display_name, id: u.id }))}
        oninput={handleUnitInput}
        onblur={handleUnitBlur}
        onselect={(opt) => {
          if (opt.id) defaultUnitId = opt.id;
        }}
      />
      {#if unitError}
        <Alert variant="error">{unitError}</Alert>
      {/if}
      <Button
        type="button"
        variant="link"
        size="sm"
        onclick={toggleInlineUnitForm}
      >
        + {$LL.lotForm.createCustomUnit()}
      </Button>
    </div>

    <Input
      bind:value={defaultAlertDaysStr}
      label={$LL.products.productAlertDays()}
      type="number"
      required
    />

    {#if showInlineUnitForm}
      <div class="inline-unit-form">
        <div class="inline-unit-header">
          <span class="inline-unit-hint">{$LL.lotForm.createCustomUnit()}</span>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            aria-label={$LL.lotForm.close()}
            disabled={creatingUnit}
            onclick={resetInlineUnit}
          >
            ×
          </Button>
        </div>
        <Input
          bind:value={newUnitKey}
          label={$LL.lotForm.key()}
          placeholder={$LL.lotForm.keyPlaceholder()}
          maxlength={16}
        />
        <Input
          bind:value={newUnitDisplayName}
          label={$LL.lotForm.displayName()}
          placeholder={$LL.lotForm.displayNamePlaceholder()}
        />
        <fieldset class="fieldset">
          <legend class="fieldset-legend sr-only">
            {$LL.lotForm.integer()} / {$LL.lotForm.decimal()}
          </legend>
          <div class="kind-radios">
            <label class="radio-wrapper">
              <input
                type="radio"
                class="radio radio-primary radio-sm"
                bind:group={newUnitKind}
                value={"integer"}
              />
              <span>{$LL.lotForm.integer()}</span>
            </label>
            <label class="radio-wrapper">
              <input
                type="radio"
                class="radio radio-primary radio-sm"
                bind:group={newUnitKind}
                value={"decimal"}
              />
              <span>{$LL.lotForm.decimal()}</span>
            </label>
          </div>
        </fieldset>
        <Button
          type="button"
          variant="primary"
          size="sm"
          disabled={creatingUnit}
          loading={creatingUnit}
          onclick={submitInlineUnit}
        >
          {creatingUnit ? $LL.lotForm.creating() : $LL.lotForm.addUnit()}
        </Button>
      </div>
    {/if}
  </div>

  <fieldset class="fieldset">
    <legend class="fieldset-legend">{$LL.products.productNotes()}</legend>
    <textarea
      bind:value={notes}
      placeholder={$LL.common.optional()}
      rows="3"
      class="textarea textarea-md w-full motion-reduce:transition-none"
    ></textarea>
  </fieldset>

  {#if mode === "edit"}
    <label class="checkbox-wrapper">
      <input
        type="checkbox"
        class="checkbox checkbox-primary checkbox-sm"
        bind:checked={isActive}
      />
      <span>{$LL.dashboard.active()}</span>
    </label>
  {/if}

  <div class="form-actions">
    <Button
      type="submit"
      variant="primary"
      loading={submitting}
    >
      {submitting
        ? $LL.common.saving()
        : mode === "edit"
          ? $LL.lotForm.saveChanges()
          : $LL.products.createProduct()}
    </Button>
    <Button
      type="button"
      variant="ghost"
      onclick={onCancel}
      disabled={submitting}
    >
      {$LL.common.cancel()}
    </Button>
  </div>
</form>

<style>
  .product-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 560px;
  }

  .form-title {
    margin: 0 0 4px;
    font-size: 1rem;
  }

  /* Barcodes subsection (create mode only) */
  .barcode-subsection {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: color-mix(in oklch, var(--color-base-200) 80%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-base-300) 60%, transparent);
    border-radius: 8px;
  }

  .subsection-title {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-base-content);
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .checkbox-wrapper,
  .radio-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--color-base-content);
  }

  .checkbox-wrapper input[type="checkbox"],
  .radio-wrapper input[type="radio"] {
    flex-shrink: 0;
  }

  .unit-field-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .form-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* Inline unit creation sub-form (create mode only) */
  .inline-unit-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: color-mix(in oklch, var(--color-info) 8%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-info) 35%, transparent);
    border-radius: 8px;
    grid-column: 1 / -1;
  }

  .inline-unit-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .inline-unit-hint {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--color-info);
  }

  .kind-radios {
    display: flex;
    gap: 12px;
    padding: 6px 0;
  }

  /* Screen-reader only utility (Bootstrap-style sr-only). The fieldset-legend
     is kept visually hidden so screen readers can still announce the radio
     group label, but the form layout flows without a visible legend. */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>