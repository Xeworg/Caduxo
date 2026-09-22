<!--
  LotForm.svelte — expiry-lot create/edit form (PR 8a of
  caduxo-daisyui-redesign).

  Full migration to the shared UI primitives:
    - Input.svelte for text/number fields.
    - Listbox.svelte for store + location closed-choice fields
      (themed popover; no native `<select>` so OS black dropdowns
      cannot leak through).
    - Button.svelte for primary / ghost actions.
    - Alert.svelte for error + no-stores-available surfaces.
  The DatePicker primitive (its own component) is preserved verbatim;
  its popover + keyboard contract is owned by PR 10 per the design.

  Tailwind classes referenced here (for the JIT scanner):
    input input-md input-error
    textarea
    btn btn-primary btn-ghost
    alert alert-error alert-soft
-->
<script lang="ts">
    import DatePicker from "./DatePicker.svelte";
    import Input from "./ui/Input.svelte";
    import Listbox from "./ui/Listbox.svelte";
    import Button from "./ui/Button.svelte";
    import Alert from "./ui/Alert.svelte";
    import {
        createExpiryLot,
        updateExpiryLot,
        type ExpiryLotCreate,
        type ExpiryLotUpdate,
        type ExpiryLotResponse,
    } from "../lib/expiry_lots.js";
    import {
        listStores,
        listStoreLocations,
        getSettings,
        type StoreResponse,
        type StoreLocationResponse,
    } from "../lib/stores.js";
    import { LL } from "../i18n/i18n-svelte.js";
    import { humanizeError } from "../lib/errors.js";

    // ── Props ──────────────────────────────────────────────────────────────────

    export let mode: "create" | "edit";
    /**
     * Required for edit mode; ignored in create mode.
     * In create mode, `initialProductId` must be set.
     */
    export let lot: ExpiryLotResponse | null = null;
    /** Required when mode === "create". */
    export let productId: string = "";
    /** Pre-filled defaults from the product (may be blank). */
    export let defaultUnit: string = "";
    export let defaultAlertDays: number = 30;
    /**
     * Unit kind from the product's catalog link. Drives quantity input rules:
     * - "integer": quantity must be whole numbers (min=1, step=1)
     * - "decimal": fractional quantities allowed (min=0.01, step=0.01)
     * When null (legacy/uncatalogued), defaults to decimal rules.
     */
    export let productUnitKind: "integer" | "decimal" | null = null;
    /**
     * When set (Scanner Registration flow), the store is locked for
     * create mode: `selectedStoreId` is pre-filled with this id, the
     * store select is replaced by a read-only label, and the user
     * cannot change the store from inside this form. Ignored in edit
     * mode (the existing lot already owns its store id). PR
     * `scanner-default-store-lock`.
     */
    export let lockedStoreId: string | null = null;
    /**
     * Display name of the locked store. Falls back to `lockedStoreId`
     * when the parent cannot resolve a name (e.g. stale persisted id).
     */
    export let lockedStoreName: string = "";
    /** Called after a successful save. */
    export let onSaved: (lot: ExpiryLotResponse) => void;
    /** Called when the user cancels. */
    export let onCancel: () => void;

    function todayIso(): string {
        const d = new Date();
        return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    }

    // ── Local state ────────────────────────────────────────────────────────────

    // Determines whether lot creation requires a location to be chosen.
    let requireInitialLocation = true;

    let stores: StoreResponse[] = [];
    let locations: StoreLocationResponse[] = [];

    // Form fields. Quantity + alert days use string bridges because
    // `Input.svelte`'s `value` contract is `string` (HTML <input
    // type="number"> round-trips through a string). The derived
    // numeric variants drive submit-time validation + the backend
    // payload.
    let selectedStoreId = "";
    let selectedLocationId = "";
    let quantityStr = "1";
    let quantity = 1;
    $: quantity = Number.parseFloat(quantityStr) || 0;
    let unit = "";
    let expiryDate = "";
    let alertDaysStr = "30";
    let alertDaysBefore = 30;
    $: alertDaysBefore = Number.parseInt(alertDaysStr, 10) || 0;
    let batchCode = "";
    let notes = "";

    let submitting = false;
    let errorMsg = "";

    let loadingStores = true;

    /** Echoes the generated batch code after a successful create when manual input was blank. */
    let batchEcho: string | null = null;

    // ── Init ───────────────────────────────────────────────────────────────────

    async function init() {
        loadingStores = true;
        // Fetch the require_initial_location_on_lot_create setting.
        try {
            const settings = await getSettings();
            requireInitialLocation = settings.require_initial_location_on_lot_create;
        } catch {
            // Keep default (true) if settings are unavailable.
        }

        try {
            stores = await listStores();
            if (mode === "edit" && lot) {
                selectedStoreId = lot.store_id;
                selectedLocationId = lot.location_id ?? "";
                quantityStr = String(lot.quantity);
                quantity = lot.quantity;
                unit = lot.unit;
                expiryDate = lot.expiry_date;
                alertDaysStr = String(lot.alert_days_before);
                alertDaysBefore = lot.alert_days_before;
                batchCode = lot.batch_code ?? "";
                notes = lot.notes ?? "";
            } else {
                // Create mode: pre-fill from product defaults.
                unit = defaultUnit;
                alertDaysBefore = defaultAlertDays;
                alertDaysStr = String(defaultAlertDays);
                // Store pre-fill: prefer the Scanner-locked store when
                // the parent passed one, otherwise the first available
                // store (preserves the Product Catalog default).
                if (lockedStoreId) {
                    selectedStoreId = lockedStoreId;
                } else if (stores.length > 0) {
                    selectedStoreId = stores[0].id;
                }
                // Default expiry date: today + alertDaysBefore.
                const d = new Date();
                d.setDate(d.getDate() + (defaultAlertDays > 0 ? defaultAlertDays : 30));
                expiryDate = d.toISOString().slice(0, 10);
            }
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        } finally {
            loadingStores = false;
        }
    }

    init();

    // Reload locations and clear selection when the selected store changes.
    $: if (selectedStoreId) {
        selectedLocationId = "";
        loadLocations(selectedStoreId);
    }

    async function loadLocations(storeId: string) {
        try {
            locations = await listStoreLocations(storeId);
        } catch {
            locations = [];
        }
    }

    // ── Listbox option derivations ──────────────────────────────────────────────

    // Store options always start with the disabled placeholder row so the
    // empty-selection state stays visually distinct from a real pick. The
    // placeholder is folded into `options` (rather than supplied via a
    // `leading` snippet, which was the `<select>`-era pattern) because
    // the themed Listbox has no `<option>` slot.
    $: storeOptions = [
        { value: "", label: $LL.lotForm.selectStorePlaceholder(), disabled: true },
        ...stores.map((s) => ({ value: s.id, label: s.name })),
    ];

    // Location options omit the "no location" entry when the setting
    // requires a location — keeps the validation contract intact. When
    // the location is optional the `value: ""` row is unselectable
    // history-free (it's a real choice, not a placeholder).
    $: locationOptions = [
        ...(!requireInitialLocation
            ? [{ value: "", label: $LL.lotForm.noLocation() }]
            : []),
        ...locations.map((loc) => ({ value: loc.id, label: loc.name })),
    ];

    // ── Submit ─────────────────────────────────────────────────────────────────

    async function submit() {
        errorMsg = "";
        if (!selectedStoreId) {
            errorMsg = $LL.lotForm.storeRequired();
            return;
        }
        if (requireInitialLocation && !selectedLocationId) {
            errorMsg = $LL.lotForm.selectLocationRequired();
            return;
        }
        if (!expiryDate) {
            errorMsg = $LL.lotForm.expiryDateRequired();
            return;
        }
        if (mode === "create" && quantity <= 0) {
            errorMsg = $LL.lotForm.quantityGreaterThanZero();
            return;
        }
        submitting = true;
        try {
            const userProvidedBatch = batchCode.trim();
            if (mode === "edit" && lot) {
                // Edit is metadata-only: quantity is sent verbatim from the
                // loaded lot so the server guard rejects any change. The
                // server enforces the same rule independently.
                const payload: ExpiryLotUpdate = {
                    id: lot.id,
                    location_id: selectedLocationId || null,
                    quantity: lot.quantity,
                    unit: unit.trim(),
                    expiry_date: expiryDate,
                    alert_days_before: alertDaysBefore,
                    batch_code: userProvidedBatch || null,
                    notes: notes.trim() || null,
                };
                const saved = await updateExpiryLot(payload);
                onSaved(saved);
            } else {
                const payload: ExpiryLotCreate = {
                    product_id: productId,
                    store_id: selectedStoreId,
                    location_id: selectedLocationId || null,
                    quantity,
                    unit: unit.trim() || null,
                    expiry_date: expiryDate,
                    alert_days_before: alertDaysBefore,
                    batch_code: userProvidedBatch || null,
                    notes: notes.trim() || null,
                };
                const saved = await createExpiryLot(payload);
                // Batch echo chip: show the generated batch code when the user left
                // the batch input blank and the server auto-generated one.
                if (!userProvidedBatch && saved.batch_code) {
                    batchEcho = saved.batch_code;
                }
                onSaved(saved);
            }
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        } finally {
            submitting = false;
        }
    }
</script>

<form class="lot-form" on:submit|preventDefault={submit}>
    <h3 class="form-title">
        {mode === "edit"
            ? $LL.lotForm.editTitle()
            : $LL.lotForm.createTitle()}
    </h3>

    {#if mode === "edit"}
        <p class="metadata-only-notice" role="note">
            {$LL.lotForm.quantityUseMovementHint()}
        </p>
    {/if}

    {#if errorMsg}
        <Alert variant="error">{errorMsg}</Alert>
    {/if}

    {#if loadingStores}
        <p class="loading">{$LL.lotForm.loadingStores()}</p>
    {:else if stores.length === 0}
        <Alert variant="error">{$LL.lotForm.noStoresAvailable()}</Alert>
    {:else}
        <!-- Store selection. Locked when the Scanner passes
             `lockedStoreId` (Scanner Registration flow) — the user
             cannot change the store from inside this form. Otherwise
             show the themed Listbox when multiple stores exist, or a
             static label when only one store is available. PR
             `scanner-default-store-lock`. -->
        {#if lockedStoreId}
            <div class="store-hint store-hint-locked" role="note">
                <span class="store-hint-label">
                    {$LL.lotForm.lockedStoreLabel()}:
                    <strong>{lockedStoreName || lockedStoreId}</strong>
                </span>
                <span class="store-hint-detail">
                    {$LL.lotForm.lockedStoreHint()}
                </span>
            </div>
        {:else if stores.length > 1}
            <Listbox
                bind:value={selectedStoreId}
                options={storeOptions}
                size="md"
                aria-label={$LL.lotForm.selectStore()}
                required
            />
        {:else}
            <!-- Single store: remember the selection implicitly. -->
            <p class="store-hint">
                {$LL.lotForm.selectStore()}: <strong>{stores[0].name}</strong>
            </p>
        {/if}

        <!-- Location picker — shown when the store has locations -->
        {#if selectedStoreId && locations.length > 0}
            <Listbox
                bind:value={selectedLocationId}
                options={locationOptions}
                size="md"
                aria-label={$LL.lotForm.internalLocation()}
                required={requireInitialLocation}
            />
            {#if requireInitialLocation}
                <p class="hint-required">{$LL.lotForm.locationRequired()}</p>
            {:else}
                <p class="hint-optional">{$LL.lotForm.locationOptional()}</p>
            {/if}
        {/if}

        <div class="grid-2">
            {#if mode === "edit" && lot}
                <!-- Edit mode: quantity is read-only. Quantity changes
                     must go through movement/adjustment/resolve flows. -->
                <div class="quantity-readonly">
                    <span class="quantity-label">{$LL.lotForm.quantityReadonly()}</span>
                    <span class="quantity-value">
                        {lot.quantity}
                        {#if lot.unit}<span class="quantity-unit">{lot.unit}</span>{/if}
                    </span>
                    <span class="quantity-hint">
                        {$LL.lotForm.quantityUseMovementHint()}
                    </span>
                </div>
            {:else}
                <Input
                    bind:value={quantityStr}
                    label={$LL.lotForm.quantityStar()}
                    type="number"
                    required
                />
            {/if}

            {#if productUnitKind === null}
                <!-- Product has no catalog link: show editable unit text input. -->
                <Input
                    bind:value={unit}
                    label={$LL.lotForm.selectUnit()}
                    placeholder={$LL.lotForm.placeholders.unit()}
                />
            {:else}
                <!-- Product has a catalog link: show read-only display name. -->
                <div class="readonly-field">
                    <span class="readonly-label">{$LL.lotForm.selectUnit()}</span>
                    <span class="unit-chip">{unit}</span>
                </div>
            {/if}
        </div>

        <div class="grid-2">
            <fieldset class="date-fieldset">
                <legend class="fieldset-legend">{$LL.lotForm.expiryDate()}</legend>
                <DatePicker
                    bind:value={expiryDate}
                    clearable={false}
                    ariaLabel={$LL.lotForm.expiryDate()}
                    id="lot-expiry"
                    name="expiry_date"
                    placeholder={$LL.lotForm.dateFormat()}
                    todayDate={todayIso()}
                />
            </fieldset>

            <Input
                bind:value={alertDaysStr}
                label={$LL.lotForm.alertDaysStar()}
                type="number"
                required
            />
        </div>

        <Input
            bind:value={batchCode}
            label={$LL.lotForm.batchCodeOptional()}
            placeholder={$LL.lotForm.placeholders.batchCode()}
        />
        {#if batchEcho}
            <span class="batch-echo-chip" aria-live="polite">
                {$LL.lotForm.createTitle()}: <code>{batchEcho}</code>
            </span>
        {/if}

        <fieldset class="fieldset">
            <legend class="fieldset-legend">{$LL.lotForm.notesOptional()}</legend>
            <textarea
                bind:value={notes}
                placeholder={$LL.common.optional()}
                rows="2"
                class="textarea textarea-md w-full motion-reduce:transition-none"
            ></textarea>
        </fieldset>

        <div class="form-actions">
            <Button
                type="submit"
                variant="primary"
                loading={submitting}
            >
                {submitting
                    ? $LL.lotForm.saving()
                    : mode === "edit"
                      ? $LL.lotForm.saveChanges()
                      : $LL.lotForm.addLot()}
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
    {/if}
</form>

<style>
    .lot-form {
        display: flex;
        flex-direction: column;
        gap: 14px;
        max-width: 520px;
    }

    .form-title {
        margin: 0 0 4px;
        font-size: 1rem;
    }

    .grid-2 {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
    }

    /* DatePicker wrapper. The DatePicker primitive renders its own
       popover; the fieldset-legend preserves the visible label
       association with the popover trigger inside. */
    .date-fieldset {
        border: 0;
        padding: 0;
        margin: 0;
        min-width: 0;
    }

    .hint-required {
        margin: -6px 0 0;
        font-size: 0.78rem;
        color: var(--color-error);
    }

    .hint-optional {
        margin: -6px 0 0;
        font-size: 0.78rem;
        color: var(--color-secondary);
    }

    .loading {
        color: var(--color-secondary);
        font-style: italic;
        font-size: 0.85rem;
        margin: 0;
    }

    .store-hint {
        font-size: 0.85rem;
        color: var(--color-base-content);
        margin: 0;
    }

    /* Scanner-locked store hint: keeps the same baseline as the
       single-store hint but adds a chip-style container with a
       warning-tinted border so the user immediately sees the store
       is fixed by the Scanner context (PR `scanner-default-store-lock`). */
    .store-hint-locked {
        display: flex;
        flex-direction: column;
        gap: 2px;
        padding: 8px 12px;
        border: 1px solid color-mix(in oklch, var(--color-warning) 40%, transparent);
        background: color-mix(in oklch, var(--color-warning) 8%, transparent);
        border-radius: 6px;
    }

    .store-hint-label {
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .store-hint-detail {
        font-size: 0.78rem;
        color: var(--color-secondary);
        line-height: 1.35;
    }

    .unit-chip {
        display: inline-flex;
        align-items: center;
        background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
        border-radius: 4px;
        padding: 4px 10px;
        font-size: 0.9rem;
        color: var(--color-base-content);
        font-weight: 500;
    }

    .metadata-only-notice {
        margin: 0;
        padding: 8px 12px;
        background: color-mix(in oklch, var(--color-info) 8%, transparent);
        border: 1px solid color-mix(in oklch, var(--color-info) 35%, transparent);
        border-radius: 6px;
        font-size: 0.82rem;
        color: var(--color-info);
        line-height: 1.35;
    }

    .quantity-readonly {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 7px 10px;
        border: 1px dashed color-mix(in oklch, var(--color-base-300) 70%, transparent);
        border-radius: 6px;
        background: color-mix(in oklch, var(--color-base-200) 50%, transparent);
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .quantity-label {
        font-weight: 500;
        color: var(--color-secondary);
        font-size: 0.78rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
    }

    .quantity-value {
        font-size: 1rem;
        font-weight: 600;
        color: var(--color-base-content);
    }

    .quantity-unit {
        font-weight: 400;
        color: var(--color-secondary);
        margin-left: 4px;
    }

    .quantity-hint {
        font-size: 0.78rem;
        color: var(--color-secondary);
        font-style: italic;
    }

    .batch-echo-chip {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        font-size: 0.82rem;
        color: var(--color-success);
        background: color-mix(in oklch, var(--color-success) 12%, transparent);
        border: 1px solid color-mix(in oklch, var(--color-success) 40%, transparent);
        border-radius: 4px;
        padding: 3px 8px;
        width: fit-content;
    }

    .batch-echo-chip code {
        font-family: 'Courier New', Courier, monospace;
        font-size: 0.82rem;
        font-weight: 600;
        color: var(--color-success);
    }

    .form-actions {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }
</style>