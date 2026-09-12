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
    import {
        listExpiryLotsByProduct,
        archiveExpiryLot,
        type ExpiryLotResponse,
    } from "../lib/expiry_lots.js";
    import LotForm from "./LotForm.svelte";
    import ResolveQuantityDialog from "./ResolveQuantityDialog.svelte";

    // ── Props ──────────────────────────────────────────────────────────────────

    export let productId: string;
    export let onBack: () => void;
    export let onEdit: (product: ProductResponse) => void;
    export let onArchived: () => void;

    // ── State ──────────────────────────────────────────────────────────────────

    let detail: ProductDetailResponse | null = null;
    let lots: ExpiryLotResponse[] = [];
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

    // Lot sub-views
    let showLotForm = false;
    let editingLot: ExpiryLotResponse | null = null;
    let resolvingLot: ExpiryLotResponse | null = null;
    let lotError = "";

    // ── Load ───────────────────────────────────────────────────────────────────

    async function load() {
        loading = true;
        errorMsg = "";
        try {
            detail = await getProduct(productId);
            lots = await listExpiryLotsByProduct(productId);
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

    // ── Lot handlers ───────────────────────────────────────────────────────────

    async function onLotSaved(_lot: ExpiryLotResponse) {
        showLotForm = false;
        editingLot = null;
        lotError = "";
        await load();
    }

    async function onLotResolved(_result: unknown) {
        resolvingLot = null;
        lotError = "";
        await load();
    }

    async function archiveLot(lot: ExpiryLotResponse) {
        try {
            await archiveExpiryLot(lot.id);
            await load();
        } catch (e: unknown) {
            lotError = String(e);
        }
    }

    function formatDate(dateStr: string): string {
        try {
            return new Date(dateStr + "T00:00:00").toLocaleDateString();
        } catch {
            return dateStr;
        }
    }

    function daysUntilExpiry(expiryDate: string): number {
        const today = new Date();
        today.setHours(0, 0, 0, 0);
        const exp = new Date(expiryDate + "T00:00:00");
        return Math.round((exp.getTime() - today.getTime()) / 86_400_000);
    }

    function urgencyClass(expiryDate: string): string {
        const days = daysUntilExpiry(expiryDate);
        if (days < 0) return "urgency-expired";
        if (days <= 7) return "urgency-critical";
        if (days <= 30) return "urgency-warning";
        return "urgency-normal";
    }

    function urgencyLabel(expiryDate: string): string {
        const days = daysUntilExpiry(expiryDate);
        if (days < 0) return "Expired";
        if (days === 0) return "Today";
        if (days === 1) return "Tomorrow";
        return `${days}d`;
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

        <!-- ── Expiry lots ──────────────────────────────────────────────────── -->
        <section class="section">
            <div class="section-header">
                <h3>Expiry lots</h3>
                {#if product.is_active}
                    <button
                        type="button"
                        class="btn-primary btn-small"
                        on:click={() => {
                            editingLot = null;
                            showLotForm = true;
                        }}
                    >
                        + New lot
                    </button>
                {/if}
            </div>

            {#if lotError}
                <div class="alert alert-error" role="alert">{lotError}</div>
            {/if}

            {#if showLotForm}
                <div class="sub-form">
                    <LotForm
                        mode={editingLot ? "edit" : "create"}
                        lot={editingLot}
                        productId={product.id}
                        defaultUnit={product.default_unit ?? ""}
                        defaultAlertDays={product.default_alert_days_before}
                        onSaved={onLotSaved}
                        onCancel={() => {
                            showLotForm = false;
                            editingLot = null;
                        }}
                    />
                </div>
            {:else if lots.length === 0}
                <p class="empty-hint">No expiry lots yet.</p>
            {:else}
                <ul class="lot-list">
                    {#each lots as lot (lot.id)}
                        <li class="lot-item">
                            <div class="lot-urgency {urgencyClass(lot.expiry_date)}">
                                <span class="urgency-badge">{urgencyLabel(lot.expiry_date)}</span>
                            </div>
                            <div class="lot-info">
                                <span class="lot-qty">
                                    {lot.quantity} {lot.unit || "unit(s)"}
                                </span>
                                <span class="lot-date">Exp: {formatDate(lot.expiry_date)}</span>
                                {#if lot.batch_code}
                                    <span class="lot-batch">{lot.batch_code}</span>
                                {/if}
                                {#if lot.location_id}
                                    <span class="lot-location">{lot.location_id}</span>
                                {/if}
                                {#if lot.status === "archived"}
                                    <span class="badge-archived">Archived</span>
                                {:else if lot.status === "resolved"}
                                    <span class="badge-resolved">Resolved</span>
                                {/if}
                            </div>
                            <div class="lot-actions">
                                {#if lot.status === "active"}
                                    <button
                                        type="button"
                                        class="btn-icon"
                                        title="Resolve quantity"
                                        on:click={() => (resolvingLot = lot)}
                                    >
                                        ↓
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-icon"
                                        title="Edit lot"
                                        on:click={() => {
                                            editingLot = lot;
                                            showLotForm = true;
                                        }}
                                    >
                                        ✏️
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-icon btn-danger-icon"
                                        title="Archive lot"
                                        on:click={() => archiveLot(lot)}
                                    >
                                        🗄
                                    </button>
                                {/if}
                            </div>
                        </li>
                    {/each}
                </ul>
            {/if}
        </section>
    {/if}
</div>

<!-- ── Resolve dialog ─────────────────────────────────────────────────────── -->
{#if resolvingLot}
    <ResolveQuantityDialog
        lot={resolvingLot}
        onResolved={onLotResolved}
        onClose={() => (resolvingLot = null)}
    />
{/if}

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

    .badge-archived {
        font-size: 0.7rem;
        background: #f3f4f6;
        color: #9ca3af;
        padding: 1px 6px;
        border-radius: 4px;
    }

    .badge-resolved {
        font-size: 0.7rem;
        background: #dcfce7;
        color: #166534;
        padding: 1px 6px;
        border-radius: 4px;
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
        color: #374151;
    }

    .btn-danger-icon {
        color: #991b1b;
    }

    .btn-danger-icon:hover {
        color: #7f1d1d;
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

    /* Sub-form wrapper */
    .sub-form {
        background: #f9fafb;
        border: 1px solid #e5e7eb;
        border-radius: 8px;
        padding: 16px;
    }

    /* Expiry lots list */
    .lot-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .lot-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 12px;
        background: #f9fafb;
        border-radius: 7px;
        border: 1px solid #e5e7eb;
    }

    .lot-urgency {
        flex-shrink: 0;
    }

    .urgency-badge {
        display: inline-block;
        font-size: 0.7rem;
        font-weight: 600;
        padding: 2px 7px;
        border-radius: 999px;
        min-width: 48px;
        text-align: center;
    }

    .urgency-expired .urgency-badge {
        background: #fee2e2;
        color: #991b1b;
    }

    .urgency-critical .urgency-badge {
        background: #fee2e2;
        color: #dc2626;
    }

    .urgency-warning .urgency-badge {
        background: #fef9c3;
        color: #a16207;
    }

    .urgency-normal .urgency-badge {
        background: #f0fdf4;
        color: #166534;
    }

    .lot-info {
        flex: 1;
        display: flex;
        align-items: center;
        gap: 8px;
        flex-wrap: wrap;
        font-size: 0.85rem;
    }

    .lot-qty {
        font-weight: 500;
    }

    .lot-date {
        color: #4b5563;
    }

    .lot-batch {
        font-size: 0.74rem;
        background: #e5e7eb;
        color: #374151;
        padding: 1px 6px;
        border-radius: 4px;
    }

    .lot-location {
        font-size: 0.74rem;
        color: #6b7280;
        background: #f3f4f6;
        padding: 1px 6px;
        border-radius: 4px;
    }

    .lot-actions {
        display: flex;
        align-items: center;
        gap: 2px;
        flex-shrink: 0;
    }
</style>
