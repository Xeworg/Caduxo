<script lang="ts">
    import DatePicker from "./DatePicker.svelte";
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

    // Form fields
    let selectedStoreId = "";
    let selectedLocationId = "";
    let quantity = 1;
    let unit = "";
    let expiryDate = "";
    let alertDaysBefore = 30;
    let batchCode = "";
    let notes = "";

    let submitting = false;
    let errorMsg = "";

    let loadingStores = true;

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
                quantity = lot.quantity;
                unit = lot.unit;
                expiryDate = lot.expiry_date;
                alertDaysBefore = lot.alert_days_before;
                batchCode = lot.batch_code ?? "";
                notes = lot.notes ?? "";
            } else {
                // Create mode: pre-fill from product defaults.
                unit = defaultUnit;
                alertDaysBefore = defaultAlertDays;
                // Auto-select the last-selected store, or the first available.
                if (stores.length > 0) {
                    selectedStoreId = stores[0].id;
                }
                // Default expiry date: today + alertDaysBefore.
                const d = new Date();
                d.setDate(d.getDate() + (defaultAlertDays > 0 ? defaultAlertDays : 30));
                expiryDate = d.toISOString().slice(0, 10);
            }
        } catch (e: unknown) {
            errorMsg = String(e);
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

    // ── Helpers ────────────────────────────────────────────────────────────────

    async function submit() {
        errorMsg = "";
        if (!selectedStoreId) {
            errorMsg = "Store is required";
            return;
        }
        if (requireInitialLocation && !selectedLocationId) {
            errorMsg = "Selecciona una ubicacion";
            return;
        }
        if (!expiryDate) {
            errorMsg = "Expiry date is required";
            return;
        }
        if (mode === "create" && quantity <= 0) {
            errorMsg = "Quantity must be greater than zero";
            return;
        }
        submitting = true;
        try {
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
                    batch_code: batchCode.trim() || null,
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
                    batch_code: batchCode.trim() || null,
                    notes: notes.trim() || null,
                };
                const saved = await createExpiryLot(payload);
                onSaved(saved);
            }
        } catch (e: unknown) {
            errorMsg = String(e);
        } finally {
            submitting = false;
        }
    }
</script>

<form class="lot-form" on:submit|preventDefault={submit}>
    <h3>
        {mode === "edit"
? "Edit expiry lot (metadata only)"
: "New expiry lot"}
    </h3>

    {#if mode === "edit"}
        <p class="metadata-only-notice" role="note">
Editing a lot only updates its metadata (location, unit, expiry
date, alert days, batch code, notes). To change the quantity, use
the <strong>movement</strong>, <strong>adjustment</strong>, or
<strong>resolve</strong> actions so the stock ledger stays
accurate.
        </p>
    {/if}

    {#if errorMsg}
        <div class="alert alert-error" role="alert">{errorMsg}</div>
    {/if}

    {#if loadingStores}
        <p class="loading">Loading stores…</p>
    {:else if stores.length === 0}
        <div class="alert alert-error" role="alert">
            No stores available. Create a store before adding expiry lots.
        </div>
    {:else}
        <!-- Store selection — only shown when multiple stores exist -->
        {#if stores.length > 1}
            <label>
                Store *
                <select bind:value={selectedStoreId}>
                    <option value="" disabled>— Select store —</option>
                    {#each stores as store (store.id)}
                        <option value={store.id}>{store.name}</option>
                    {/each}
                </select>
            </label>
        {:else}
            <!-- Single store: remember the selection implicitly. -->
            <p class="store-hint">
                Store: <strong>{stores[0].name}</strong>
            </p>
        {/if}

        <!-- Location picker — shown when the store has locations -->
        {#if selectedStoreId && locations.length > 0}
            <label>
                Internal location
                {#if requireInitialLocation}
                    <span class="required-hint">(required)</span>
                {:else}
                    <span class="optional-hint">(optional)</span>
                {/if}
                <select bind:value={selectedLocationId}>
                    {#if !requireInitialLocation}
                        <option value="">— None —</option>
                    {/if}
                    {#each locations as loc (loc.id)}
                        <option value={loc.id}>{loc.name}</option>
                    {/each}
                </select>
            </label>
        {/if}

                <div class="grid-2">
                    {#if mode === "edit" && lot}
                        <!-- Edit mode: quantity is read-only. Quantity changes
                             must go through movement/adjustment/resolve flows. -->
                        <div class="quantity-readonly">
                            <span class="quantity-label">Quantity</span>
                            <span class="quantity-value">
                                {lot.quantity}
                                {#if lot.unit}<span class="quantity-unit">{lot.unit}</span>{/if}
                            </span>
                            <span class="quantity-hint">
                                Use movement / adjustment / resolve actions to change it.
                            </span>
                        </div>
                    {:else}
                        <label>
                            Quantity *
                            <input
                                type="number"
                                bind:value={quantity}
                                min={productUnitKind === "integer" ? 1 : 0.01}
                                step={productUnitKind === "integer" ? 1 : 0.01}
                                required
                            />
                        </label>
                    {/if}

                    {#if productUnitKind === null}
                        <!-- Product has no catalog link: show editable unit text input. -->
                        <label>
                            Unit
                            <input
                                type="text"
                                bind:value={unit}
                                placeholder="e.g. kg, L, pcs"
                                autocomplete="off"
                            />
                        </label>
                    {:else}
                        <!-- Product has a catalog link: show read-only display name. -->
                        <div class="readonly-field">
                            <span class="readonly-label">Unit</span>
                            <span class="unit-chip">{unit}</span>
                        </div>
                    {/if}
                </div>

        <div class="grid-2">
            <label>
                Expiry date *
                <DatePicker
                    bind:value={expiryDate}
                    clearable={false}
                    ariaLabel="Expiry date"
                    id="lot-expiry"
                    name="expiry_date"
                    placeholder="YYYY-MM-DD"
                    todayDate={todayIso()}
                />
            </label>

            <label>
                Alert days before *
                <input
                    type="number"
                    bind:value={alertDaysBefore}
                    min="0"
                    max="3650"
                    step="1"
                    required
                />
            </label>
        </div>

        <label>
            Batch code (optional)
            <input
                type="text"
                bind:value={batchCode}
                placeholder="e.g. B2024-001"
                autocomplete="off"
            />
        </label>

        <label>
            Notes (optional)
            <textarea
                bind:value={notes}
                placeholder="Optional notes…"
                rows="2"
            ></textarea>
        </label>

        <div class="form-actions">
            <button
                type="submit"
                class="btn-primary"
                disabled={submitting}
            >
                {submitting
                    ? "Saving…"
                    : mode === "edit"
                      ? "Save changes"
                      : "Add lot"}
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
    {/if}
</form>

<style>
    .lot-form {
        display: flex;
        flex-direction: column;
        gap: 14px;
        max-width: 520px;
    }

    .lot-form h3 {
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

    .loading {
        color: #6b7280;
        font-style: italic;
        font-size: 0.85rem;
        margin: 0;
    }

    .store-hint {
        font-size: 0.85rem;
        color: #374151;
        margin: 0;
    }

    .unit-chip {
        display: inline-flex;
        align-items: center;
        background: #e5e7eb;
        border-radius: 4px;
        padding: 4px 10px;
        font-size: 0.9rem;
        color: #374151;
        font-weight: 500;
    }

    .metadata-only-notice {
        margin: 0;
        padding: 8px 12px;
        background: #eff6ff;
        border: 1px solid #bfdbfe;
        border-radius: 6px;
        font-size: 0.82rem;
        color: #1e3a8a;
        line-height: 1.35;
    }

    .quantity-readonly {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 7px 10px;
        border: 1px dashed #d1d5db;
        border-radius: 6px;
        background: #f9fafb;
        font-size: 0.85rem;
        color: #374151;
    }

    .quantity-label {
        font-weight: 500;
        color: #6b7280;
        font-size: 0.78rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
    }

    .quantity-value {
        font-size: 1rem;
        font-weight: 600;
        color: #111827;
    }

    .quantity-unit {
        font-weight: 400;
        color: #6b7280;
        margin-left: 4px;
    }

    .quantity-hint {
        font-size: 0.78rem;
        color: #6b7280;
        font-style: italic;
    }
</style>
