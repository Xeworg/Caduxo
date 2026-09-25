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
    import DistributionEditor, {
        type DistributionAllocationRow,
    } from "./lot-creation/DistributionEditor.svelte";
    import {
        createExpiryLot,
        createExpiryLotDistributed,
        updateExpiryLot,
        type ExpiryLotCreate,
        type ExpiryLotDistributedCreate,
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
    /**
     * Optional product context. When supplied, a read-only summary is
     * rendered above the form fields so the user sees which product
     * the lot belongs to (e.g. when the form is opened from the
     * Scanner Registration flow). Both fields are optional and
     * independently renderable so the caller can pass either or both.
     */
    export let productDescription: string = "";
    export let productSku: string = "";
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

    /**
     * Optional initial-distribution state (ODD task 3 of
     * `scanner-first-inventory`). Only meaningful in create mode; the
     * editor is rendered when:
     *   - `mode === "create"`, AND
     *   - a store is selected, AND
     *   - at least one active same-store location exists.
     * In edit mode the toggle is hidden and `distributionEnabled`
     * stays `false`, so `createExpiryLot` is always used (the existing
     * metadata-only contract). PR `scanner-first-inventory`.
     */
    let distributionEnabled = false;
    /**
     * Editor-owned allocation rows. The editor mutates this array via
     * two-way binding; LotForm reads it at submit time to build the
     * `ExpiryLotDistributedCreate` payload. Seeded from the existing
     * single-location selection whenever the user toggles distribution
     * ON so the anchor location carries through; cleared when the
     * user toggles OFF so the simple picker resumes ownership.
     */
    let distributionAllocations: DistributionAllocationRow[] = [];
    /**
     * Editor-validity signal. Mirrors the editor's `onValidityChange`
     * callback so LotForm can disable the submit button + surface a
     * human-readable summary while the user is editing rows. Re-checked
     * at submit time as a defensive belt-and-suspenders.
     */
    let distributionValid = false;
    let distributionSummary: string | null = null;

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

    // ── Distribution editor helpers ──────────────────────────────────────
    //
    // The toggle is create-only (edit mode hides it) and only enables
    // when at least two active same-store locations are available — a
    // single-location distribution has no semantic difference from the
    // simple path, so the toggle stays disabled and the editor never
    // appears in that case.
    $: activeLocations = locations.filter((loc) => loc.is_active);
    $: canDistribute = mode === "create" && activeLocations.length >= 2;
    // When the active locations change (store switch / refetch), reset
    // distribution state so a stale anchor / allocations never survive
    // a store switch. Same pattern as `selectedLocationId` reset.
    $: if (activeLocations) {
        // Defer reset to a microtask so the reactive expression does
        // not cascade with the `loadLocations` assignment above.
        queueMicrotask(resetDistributionState);
    }

    function resetDistributionState() {
        if (!canDistribute && distributionEnabled) {
            distributionEnabled = false;
        }
        // Drop allocations referencing locations the new store does
        // not own — anchors become stale and the backend would reject
        // them with `DistributionLocationNotInStore`. The simpler
        // reset is to clear the whole array; the user re-toggles to
        // seed the editor again.
        const available = new Set(activeLocations.map((l) => l.id));
        const filtered = distributionAllocations.filter((row) =>
            available.has(row.locationId),
        );
        if (filtered.length !== distributionAllocations.length) {
            distributionAllocations = filtered;
        }
        if (distributionAllocations.length === 0) {
            distributionValid = false;
            distributionSummary = null;
        }
    }

    /**
     * Toggle the distribution editor on / off while preserving the
     * anchor location so the user does not lose their selection
     * across toggles. Mirrors the same pattern as the Scanner's
     * locked-store hint: the anchor location is shared state between
     * the simple picker and the editor.
     */
    function toggleDistribution() {
        if (!canDistribute) return;
        distributionEnabled = !distributionEnabled;
        if (distributionEnabled) {
            // Seed the editor with the current simple-picker state
            // when there is one, otherwise the first active location.
            // Anchor quantity defaults to the full lot total so the
            // common "all stock at one location" case is one click.
            const anchorLocationId =
                selectedLocationId || activeLocations[0]?.id || "";
            if (anchorLocationId) {
                distributionAllocations = [
                    {
                        id: `alloc-${Date.now()}`,
                        locationId: anchorLocationId,
                        quantityStr: String(quantity || ""),
                    },
                ];
            } else {
                distributionAllocations = [];
            }
        } else {
            // Toggling OFF: push the editor's anchor back into the
            // simple picker so the existing `selectedLocationId`
            // contract resumes ownership. Discard any additional
            // allocations — the user opted back into the simple path.
            const anchor = distributionAllocations[0];
            if (anchor && anchor.locationId) {
                selectedLocationId = anchor.locationId;
            }
            distributionAllocations = [];
            distributionValid = false;
            distributionSummary = null;
        }
    }

    function handleDistributionValidityChange(
        valid: boolean,
        summary: string | null,
    ): void {
        distributionValid = valid;
        distributionSummary = summary;
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
        if (!expiryDate) {
            errorMsg = $LL.lotForm.expiryDateRequired();
            return;
        }
        if (mode === "create" && quantity <= 0) {
            errorMsg = $LL.lotForm.quantityGreaterThanZero();
            return;
        }
        // Distribution validation lives in the editor; re-check here as
        // a defensive belt-and-suspenders so the submit button is the
        // only path that can produce a misleading state.
        if (distributionEnabled) {
            if (!canDistribute) {
                errorMsg = $LL.lotForm.distribution.distributionDisabled();
                return;
            }
            if (!distributionValid) {
                // The editor emits a stable error key in
                // `distributionSummary`; surface it verbatim when we
                // can resolve it, otherwise fall back to the
                // canonical "quantity must match" message.
                errorMsg = resolveDistributionSummary();
                if (!errorMsg) {
                    errorMsg = $LL.lotForm.distribution.totalMismatch({
                        allocated: distributionAllocations
                            .map((row) =>
                                Number.parseFloat(row.quantityStr) || 0,
                            )
                            .reduce((s, q) => s + q, 0)
                            .toString(),
                        total: String(quantity),
                    });
                }
                return;
            }
        } else if (requireInitialLocation && !selectedLocationId) {
            errorMsg = $LL.lotForm.selectLocationRequired();
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
            } else if (distributionEnabled) {
                // Distributed create: one lot, one entry:initial,
                // transfers for the additional rows. The editor owns
                // allocation ordering; the backend takes the first row
                // as the anchor.
                const allocationsPayload = distributionAllocations.map(
                    (row) => ({
                        location_id: row.locationId,
                        quantity:
                            Number.parseFloat(row.quantityStr) || 0,
                    }),
                );
                const payload: ExpiryLotDistributedCreate = {
                    product_id: productId,
                    store_id: selectedStoreId,
                    total_quantity: quantity,
                    allocations: allocationsPayload,
                    unit: unit.trim() || null,
                    expiry_date: expiryDate,
                    alert_days_before: alertDaysBefore,
                    batch_code: userProvidedBatch || null,
                    notes: notes.trim() || null,
                };
                const result = await createExpiryLotDistributed(payload);
                // Batch echo chip mirrors the simple-create path so the
                // UX stays consistent across both code paths.
                if (!userProvidedBatch && result.lot.batch_code) {
                    batchEcho = result.lot.batch_code;
                }
                onSaved(result.lot);
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

    /**
     * Resolve the editor's `distributionSummary` key into a localised
     * string for the form-level alert. Returns the empty string when
     * the key does not map to a known distribution error so the caller
     * can fall back to its own canonical message.
     */
    function resolveDistributionSummary(): string {
        const key = distributionSummary;
        if (!key) return "";
        switch (key) {
            case "rowLocationRequired":
                return $LL.lotForm.distribution.rowLocationRequired();
            case "rowDuplicateLocation":
                return $LL.lotForm.distribution.rowDuplicateLocation();
            case "rowLocationUnavailable":
                return $LL.lotForm.distribution.rowLocationUnavailable();
            case "rowQuantityRequired":
                return $LL.lotForm.distribution.rowQuantityRequired();
            case "rowQuantityNonPositive":
                return $LL.lotForm.distribution.rowQuantityNonPositive();
            case "rowQuantityFractional":
                return $LL.lotForm.distribution.rowQuantityFractional();
            case "lotForm.distribution.totalShort":
                return $LL.lotForm.distribution.totalShort({
                    remaining: String(
                        Math.max(0, quantity - distributionAllocations.reduce(
                            (s, row) =>
                                s + (Number.parseFloat(row.quantityStr) || 0),
                            0,
                        )),
                    ),
                });
            case "lotForm.distribution.totalOver":
                return $LL.lotForm.distribution.totalOver({
                    overflow: String(
                        Math.max(0, distributionAllocations.reduce(
                            (s, row) =>
                                s + (Number.parseFloat(row.quantityStr) || 0),
                            0,
                        ) - quantity),
                    ),
                });
            case "lotForm.quantityGreaterThanZero":
                return $LL.lotForm.quantityGreaterThanZero();
            default:
                return "";
        }
    }
</script>

<form class="lot-form" onsubmit={(e) => { e.preventDefault(); void submit(); }}>
    <h3 class="form-title">
        {mode === "edit"
            ? $LL.lotForm.editTitle()
            : $LL.lotForm.createTitle()}
    </h3>

    {#if productDescription || productSku}
        <!-- Optional product context. Shown when the caller passes
             either the SKU or the description (e.g. Scanner
             Registration flow) so the user always sees which product
             the lot belongs to before they fill in the per-lot fields. -->
        <p class="product-context">
            {#if productSku}
                <span class="product-context-label">{$LL.products.productSku()}:</span>
                <strong>{productSku}</strong>
            {/if}
            {#if productDescription}
                <span class="product-context-description">{productDescription}</span>
            {/if}
        </p>
    {/if}

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

        <!-- Location picker — shown when the store has locations AND
             the user has not opted into multi-location distribution.
             The DistributionEditor (below) owns the location selection
             when `distributionEnabled` is true, so the simple picker
             is hidden in that case to avoid two conflicting location
             controls on the same screen. PR `scanner-first-inventory`,
             ODD task 3. -->
        {#if selectedStoreId && locations.length > 0 && !distributionEnabled}
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

        <!--
          Optional initial-distribution toggle (create-only). Hidden
          in edit mode; disabled when the active store has fewer than
          two active locations (a single-location distribution has no
          semantic difference from the simple path, so the toggle is
          never surfaced). The toggle uses a native checkbox so it
          inherits the platform's keyboard + screen-reader contract
          without an extra primitive.
        -->
        {#if mode === "create"}
            <div class="distribution-toggle-row">
                <label class="distribution-toggle-label">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-sm motion-reduce:transition-none"
                        checked={distributionEnabled}
                        disabled={!canDistribute}
                        onchange={toggleDistribution}
                        aria-describedby="distribution-toggle-hint"
                    />
                    <span class="distribution-toggle-text">
                        {$LL.lotForm.distribution.toggleLabel()}
                    </span>
                </label>
                <p
                    class="distribution-toggle-hint"
                    id="distribution-toggle-hint"
                >
                    {#if !canDistribute}
                        {$LL.lotForm.distribution.noLocationsAvailable()}
                    {:else}
                        {$LL.lotForm.distribution.toggleHint()}
                    {/if}
                </p>
            </div>
        {/if}

        <!--
          Distribution editor. Rendered when the toggle is on AND the
          active store has at least two active locations. The editor
          owns the anchor + additional rows and reports its validity
          via the `onValidityChange` callback; LotForm re-checks at
          submit time as a defensive belt-and-suspenders.
        -->
        {#if distributionEnabled && canDistribute}
            <DistributionEditor
                locations={activeLocations}
                totalQuantity={quantity}
                unitKind={productUnitKind}
                disabled={submitting}
                bind:allocations={distributionAllocations}
                onValidityChange={handleDistributionValidityChange}
            />
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
                disabled={submitting || (distributionEnabled && !distributionValid)}
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

    /* Optional product context: read-only summary rendered above the
       form when the caller passes `productDescription` or `productSku`.
       Mirrors the inline-note pattern from the locked-store hint so the
       Scanner Registration flow can show "this lot belongs to product
       X" without the user losing track of which product they are
       creating a lot for. */
    .product-context {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 8px;
        margin: 0 0 4px;
        padding: 6px 10px;
        background: color-mix(in oklch, var(--color-info) 8%, transparent);
        border: 1px solid color-mix(in oklch, var(--color-info) 35%, transparent);
        border-radius: 6px;
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .product-context-label {
        font-size: 0.74rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--color-info);
    }

    .product-context-description {
        color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
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

    /*
      Distribution toggle (PR `scanner-first-inventory`, ODD task 3).
      Native checkbox + label so the platform's keyboard / focus /
      screen-reader contract applies without an extra primitive. The
      hint paragraph mirrors the locked-store hint pattern: a short
      contextual sentence under the control.
    */
    .distribution-toggle-row {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 8px 12px;
        border: 1px solid color-mix(in oklch, var(--color-info) 35%, transparent);
        background: color-mix(in oklch, var(--color-info) 6%, transparent);
        border-radius: 6px;
    }

    .distribution-toggle-label {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        font-size: 0.88rem;
        color: var(--color-base-content);
        cursor: pointer;
    }

    .distribution-toggle-label:has(input:disabled) {
        cursor: not-allowed;
        opacity: 0.7;
    }

    .distribution-toggle-text {
        font-weight: 500;
    }

    .distribution-toggle-hint {
        margin: 0;
        padding: 0 4px;
        font-size: 0.76rem;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        line-height: 1.35;
    }
</style>