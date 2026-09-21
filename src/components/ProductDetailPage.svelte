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
        type ExpiryLotResponse,
    } from "../lib/expiry_lots.js";
    import LotForm from "./LotForm.svelte";
    import ResolveQuantityDialog from "./ResolveQuantityDialog.svelte";
    import ArchiveLotDialog from "./ArchiveLotDialog.svelte";
    import LotMovementsPanel from "./LotMovementsPanel.svelte";
    import Combobox from "./ui/Combobox.svelte";
    import {
        getExpiryLot,
    } from "../lib/expiry_lots.js";
    import {
        listStoreLocations,
        type StoreLocationResponse,
    } from "../lib/stores.js";
    import { LL } from "../i18n/i18n-svelte.js";
    import { humanizeError } from "../lib/errors.js";
    import Modal from "./ui/Modal.svelte";
    import Tabs from "./ui/Tabs.svelte";
    import Button from "./ui/Button.svelte";
    import Alert from "./ui/Alert.svelte";
    import Badge from "./ui/Badge.svelte";

    /** Static barcode-type suggestion list shared by the ProductForm
     *  Combobox (PR 8a.1) and this ProductDetailPage Combobox (PR
     *  8a.2). Centralised here so a future migration can lift it
     *  into i18n catalogues / domain constants in one place. */
    const BARCODE_TYPE_OPTIONS = [
        "EAN13",
        "EAN8",
        "UPC",
        "CODE128",
        "CODE39",
        "QR",
    ];

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
    let archivingLot: ExpiryLotResponse | null = null;
    let lotError = "";

    // Lot detail modal (Historial tab)
    let showLotDetail = false;
    let detailLot: ExpiryLotResponse | null = null;
    let detailLotLoading = false;
    let lotDetailTab: "detail" | "history" = "detail";
    let detailLotLocations: StoreLocationResponse[] = [];

    // ── Load ───────────────────────────────────────────────────────────────────

    async function load() {
        loading = true;
        errorMsg = "";
        try {
            detail = await getProduct(productId);
            lots = await listExpiryLotsByProduct(productId);
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
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
            barcodeError = $LL.products.detail.barcode.valueRequired();
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
            barcodeError = humanizeError(e);
        } finally {
            addingBarcode = false;
        }
    }

    async function removeBarcode(b: ProductBarcodeResponse) {
        try {
            await removeProductBarcode({ id: b.id });
            await load();
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        }
    }

    async function confirmArchive() {
        if (!detail) return;
        try {
            await archiveProduct(detail.product.id);
            confirmingArchive = false;
            onArchived();
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
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

    async function onLotArchived() {
        archivingLot = null;
        lotError = "";
        await load();
    }

    // ── Lot detail modal / Historial ───────────────────────────────────────────

    async function openLotDetail(lot: ExpiryLotResponse) {
        showLotDetail = true;
        detailLotLoading = true;
        lotDetailTab = "detail";
        detailLot = null;
        detailLotLocations = [];
        try {
            detailLot = await getExpiryLot(lot.id);
            // Load all locations for this lot's store for the movements panel.
            if (detailLot) {
                detailLotLocations = await listStoreLocations(detailLot.store_id);
            }
        } catch (e: unknown) {
            lotError = humanizeError(e);
        } finally {
            detailLotLoading = false;
        }
    }

    async function refreshDetailLot() {
        if (!detailLot) return;
        try {
            detailLot = await getExpiryLot(detailLot.id);
        } catch {
            // Silently ignore refresh failures — modal stays open with stale data.
        }
    }

    function formatDate(dateStr: string): string {
        try {
            return new Date(dateStr + "T00:00:00").toLocaleDateString();
        } catch {
            return dateStr;
        }
    }

    function lotQtyUnit(lot: ExpiryLotResponse): string {
        return lot.unit
            ? $LL.products.detail.lot.qtyUnit({ qty: lot.quantity, unit: lot.unit })
            : $LL.products.detail.lot.qtyUnitFallback({ qty: lot.quantity });
    }

    function lotExpLabel(lot: ExpiryLotResponse): string {
        return $LL.products.detail.lot.exp({ date: formatDate(lot.expiry_date) });
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
        if (days < 0) return $LL.products.detail.lot.urgencyExpired();
        if (days === 0) return $LL.products.detail.lot.urgencyToday();
        if (days === 1) return $LL.products.detail.lot.urgencyTomorrow();
        return $LL.products.detail.lot.urgencyDays({ days });
    }
</script>

<div class="detail-page">
<div class="detail-toolbar">
<button type="button" class="btn-secondary btn-small" on:click={onBack}>
{$LL.products.detail.backToList()}
</button>
</div>

{#if loading && !detail}
<p class="loading">{$LL.products.detail.loading()}</p>
{:else if errorMsg && !detail}
<div class="alert alert-error" role="alert">{errorMsg}</div>
<button type="button" class="btn-secondary" on:click={load}>{$LL.products.detail.retry()}</button>
{:else if detail}
{@const product = detail.product}
<header class="detail-header">
<div>
<div class="title-row">
<h2>{product.description}</h2>
{#if !product.is_active}
<span class="badge-inactive">{$LL.products.archived()}</span>
{/if}
</div>
<div class="meta-row">
<span class="sku">{$LL.products.productSku()}: {product.sku}</span>
{#if detail.categories.length === 0}
<span class="category-badge category-badge--uncat">{$LL.categoryPicker.uncategorized()}</span>
{/if}
{#each detail.categories as cat (cat.id)}
<span class="category-badge">{cat.name}</span>
{/each}
{#if product.default_unit}
<span class="meta">{$LL.products.detail.unit()}: {product.default_unit}</span>
{/if}
<span class="meta">{$LL.products.detail.alertDaysBefore({ days: product.default_alert_days_before })}</span>
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
{$LL.products.detail.edit()}
</button>
{#if product.is_active}
{#if !confirmingArchive}
<button
type="button"
class="btn-danger"
on:click={() => (confirmingArchive = true)}
>
{$LL.products.detail.archive()}
</button>
{:else}
<span class="archive-confirm">
{$LL.products.detail.archiveThisProduct()}
<button
type="button"
class="btn-danger btn-small"
on:click={confirmArchive}
>
{$LL.products.detail.yesArchive()}
</button>
<button
type="button"
class="btn-secondary btn-small"
on:click={() => (confirmingArchive = false)}
>
{$LL.common.cancel()}
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
                <h3>{$LL.products.detail.barcode.title()}</h3>
            </div>

            {#if detail.barcodes.length === 0}
                <p class="empty-hint">{$LL.products.detail.noBarcodeYet()}</p>
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
                                    <span class="badge-primary">{$LL.products.detail.primary()}</span>
                                {/if}
                            </div>
                            <button
                                type="button"
                                class="btn-icon"
                                title={$LL.products.detail.removeBarcode()}
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
                            {$LL.products.detail.barcode.valueLabel()}
                            <input
                                type="text"
                                bind:value={barcodeValue}
                                placeholder={$LL.products.placeholders.barcode()}
                                required
                                autocomplete="off"
                            />
                        </label>
                        <Combobox
                            bind:value={barcodeType}
                            label={$LL.products.detail.barcode.typeLabel()}
                            placeholder={$LL.products.detail.barcode.typePlaceholder()}
                            options={BARCODE_TYPE_OPTIONS}
                        />
                    </div>
                    <label class="checkbox-label">
                        <input type="checkbox" bind:checked={isPrimary} />
                        {$LL.products.detail.barcode.setAsPrimary()}
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
                            {addingBarcode ? $LL.products.detail.barcode.adding() : $LL.products.detail.barcode.addBarcode()}
                        </button>
                    </div>
                </form>
            {:else}
                <p class="hint-muted">{$LL.products.archivedProducts()}</p>
            {/if}
        </section>

        <!-- ── Expiry lots ──────────────────────────────────────────────────── -->
        <section class="section">
            <div class="section-header">
                <h3>{$LL.products.detail.expiryLots()}</h3>
                {#if product.is_active}
                    <button
                        type="button"
                        class="btn-primary btn-small"
                        on:click={() => {
                            editingLot = null;
                            showLotForm = true;
                        }}
                    >
                        {$LL.products.detail.newLot()}
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
                        productUnitKind={product.unit_type ?? "decimal"}
                        onSaved={onLotSaved}
                        onCancel={() => {
                            showLotForm = false;
                            editingLot = null;
                        }}
                    />
                </div>
            {:else if lots.length === 0}
                <p class="empty-hint">{$LL.products.detail.noExpiryLotsYet()}</p>
            {:else}
                <ul class="lot-list">
                    {#each lots as lot (lot.id)}
                        <li class="lot-item">
                            <div class="lot-urgency {urgencyClass(lot.expiry_date)}">
                                <span class="urgency-badge">{urgencyLabel(lot.expiry_date)}</span>
                            </div>
                            <div class="lot-info">
                                <span class="lot-qty">
                                    {lotQtyUnit(lot)}
                                </span>
                                <span class="lot-date">{lotExpLabel(lot)}</span>
                                {#if lot.batch_code}
                                    <span class="lot-batch">{lot.batch_code}</span>
                                {/if}
                                {#if lot.location_id}
                                    <span class="lot-location">{lot.location_id}</span>
                                {/if}
                                {#if lot.status === "archived"}
                                    <span class="badge-archived">{$LL.products.detail.lot.archived()}</span>
                                {:else if lot.status === "resolved"}
                                    <span class="badge-resolved">{$LL.products.detail.lot.resolved()}</span>
                                {/if}
                            </div>
                            <div class="lot-actions">
                                {#if lot.status === "active"}
                                    <button
                                        type="button"
                                        class="btn-icon"
                                        title={$LL.products.detail.lot.resolveQty()}
                                        on:click={() => (resolvingLot = lot)}
                                    >
                                        ↓
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-icon"
                                        title={$LL.products.detail.lot.editLot()}
                                        on:click={() => {
                                            editingLot = lot;
                                            showLotForm = true;
                                        }}
                                    >
                                        ✏️
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-icon"
                                        title={$LL.products.detail.lot.movementHistory()}
                                        on:click={() => openLotDetail(lot)}
                                    >
                                        📋
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-icon btn-danger-icon"
                                        title={$LL.products.detail.lot.archiveLot()}
                                        on:click={() => (archivingLot = lot)}
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

<!-- ── Archive dialog ─────────────────────────────────────────────────────── -->
{#if archivingLot}
    <ArchiveLotDialog
        lot={archivingLot}
        onArchived={onLotArchived}
        onClose={() => (archivingLot = null)}
    />
{/if}

<!-- ── Lot detail modal ─────────────────────────────────────────────────────── -->
<Modal
    bind:open={showLotDetail}
    size="wide"
    showClose
    closeLabel={$LL.lotMovements.modal.close()}
    aria-label={$LL.dashboard.lotDetail()}
    oncancel={() => (showLotDetail = false)}
    onclose={() => (showLotDetail = false)}
>
    {#snippet children()}
        <header class="dialog-header">
            <h3>{$LL.lotsDetail.title()}</h3>
        </header>

        {#if detailLotLoading}
            <p class="modal-loading">{$LL.lotsDetail.loading()}</p>
        {:else if detailLot}
            <Tabs
                items={[
                    { id: "detail", label: $LL.lotsDetail.detail(), panel: lotDetailPanel },
                    { id: "history", label: $LL.lotsDetail.history(), panel: lotHistoryPanel },
                ]}
                bind:activeId={lotDetailTab}
                style="bordered"
                aria-label={$LL.dashboard.lotDetail()}
            />
        {/if}
    {/snippet}
</Modal>

{#snippet lotDetailPanel()}
    {#if detailLot}
        <dl class="detail-grid">
            <dt>{$LL.lotsDetail.lotId()}</dt><dd class="cell-sku">{detailLot.id.slice(0, 8)}…</dd>
            <dt>{$LL.lotsDetail.quantity()}</dt><dd>{lotQtyUnit(detailLot)}</dd>
            <dt>{$LL.lotsDetail.expiry()}</dt><dd>{formatDate(detailLot.expiry_date)}</dd>
            <dt>{$LL.lotsDetail.alertDays()}</dt><dd>{detailLot.alert_days_before}</dd>
            <dt>{$LL.lotsDetail.batch()}</dt><dd>{detailLot.batch_code ?? "—"}</dd>
            <dt>{$LL.lotsDetail.status()}</dt><dd>{detailLot.status}</dd>
            {#if detailLot.location_id}
                <dt>{$LL.lotsDetail.location()}</dt><dd>{detailLot.location_id}</dd>
            {/if}
            {#if detailLot.resolution}
                <dt>{$LL.lotsDetail.resolution()}</dt><dd>{detailLot.resolution}</dd>
            {/if}
            {#if detailLot.notes}
                <dt>{$LL.lotsDetail.notes()}</dt><dd>{detailLot.notes}</dd>
            {/if}
        </dl>
        <div class="modal-actions">
            <Button variant="ghost" onclick={() => (showLotDetail = false)}>
                {$LL.lotsDetail.close()}
            </Button>
        </div>
    {/if}
{/snippet}

{#snippet lotHistoryPanel()}
    {#if detailLot}
        <div class="tab-content">
            <LotMovementsPanel
                lotId={detailLot.id}
                lotQuantity={detailLot.quantity}
                lotUnit={detailLot.unit || ""}
                lotStatus={detailLot.status}
                locations={detailLotLocations.filter(
                    (l) => l.store_id === detailLot!.store_id,
                )}
                allLocations={detailLotLocations}
                unitType={detailLot.unit_type}
                onMovementCreated={async () => {
                    await refreshDetailLot();
                }}
            />
        </div>
    {/if}
{/snippet}

<style>
    /* ── Page chrome ─────────────────────────────────────────────────────────── */
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
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        font-style: italic;
    }

    .alert {
        padding: 10px 14px;
        border-radius: 6px;
        font-size: 0.9rem;
    }

    .alert-error {
        background: color-mix(in oklch, var(--color-error) 12%, transparent);
        color: var(--color-error);
        border: 1px solid color-mix(in oklch, var(--color-error) 35%, transparent);
    }

    .inline-error {
        margin: 4px 0 0;
        padding: 6px 10px;
        font-size: 0.8rem;
    }

    /* ── Header ─────────────────────────────────────────────────────────────── */
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
        color: var(--color-base-content);
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
        background: var(--color-base-200);
        color: var(--color-base-content);
        padding: 2px 8px;
        border-radius: 4px;
    }

    .category-badge {
        font-size: 0.78rem;
        background: color-mix(in oklch, var(--color-primary) 12%, transparent);
        color: var(--color-primary);
        padding: 2px 8px;
        border-radius: 999px;
    }

    .meta {
        font-size: 0.82rem;
        color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
    }

    .notes {
        margin: 8px 0 0;
        font-size: 0.88rem;
        color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
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
        color: var(--color-error);
    }

    /* ── Sections ──────────────────────────────────────────────────────────── */
    .section {
        background: var(--color-base-100);
        border: 1px solid var(--color-base-200);
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
        color: var(--color-base-content);
    }

    /* ── Barcodes ──────────────────────────────────────────────────────────── */
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
        background: var(--color-base-200);
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
        color: var(--color-base-content);
    }

    .barcode-type {
        font-size: 0.74rem;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        background: var(--color-base-200);
        padding: 1px 6px;
        border-radius: 4px;
    }

    .badge-primary {
        font-size: 0.7rem;
        background: color-mix(in oklch, var(--color-success) 18%, transparent);
        color: var(--color-success);
        padding: 1px 6px;
        border-radius: 4px;
    }

    .badge-inactive {
        font-size: 0.7rem;
        background: var(--color-base-200);
        color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
        border-radius: 4px;
        padding: 2px 7px;
    }

    .badge-archived {
        font-size: 0.7rem;
        background: var(--color-base-200);
        color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
        padding: 1px 6px;
        border-radius: 4px;
    }

    .badge-resolved {
        font-size: 0.7rem;
        background: color-mix(in oklch, var(--color-success) 18%, transparent);
        color: var(--color-success);
        padding: 1px 6px;
        border-radius: 4px;
    }

    .btn-icon {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 0.95rem;
        padding: 2px 6px;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    }

    .btn-icon:hover {
        color: var(--color-base-content);
    }

    .btn-danger-icon {
        color: var(--color-error);
    }

    .btn-danger-icon:hover {
        color: color-mix(in oklch, var(--color-error) 80%, black);
    }

    /* ── Barcode add form ──────────────────────────────────────────────────── */
    .barcode-form {
        display: flex;
        flex-direction: column;
        gap: 10px;
        border-top: 1px dashed var(--color-base-300);
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
        color: var(--color-base-content);
    }

    label input[type="text"] {
        padding: 7px 10px;
        border: 1px solid var(--color-base-300);
        border-radius: 6px;
        font-size: 0.9rem;
        font-family: inherit;
        background: var(--color-base-100);
        color: var(--color-base-content);
    }

    label input:focus {
        outline: 2px solid var(--color-primary);
        border-color: var(--color-primary);
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

    /* ── Local buttons (kept as-is for markup compatibility; tokens now
       drive the colour so the buttons adapt to `caduxo-light` and `dark`) */
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

    .btn-primary:hover:not(:disabled) {
        background: color-mix(in oklch, var(--color-primary) 88%, black);
    }

    .btn-primary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
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

    .btn-secondary:hover:not(:disabled) {
        background: var(--color-base-200);
    }

    .btn-danger {
        background: var(--color-base-100);
        color: var(--color-error);
        border: 1px solid color-mix(in oklch, var(--color-error) 35%, transparent);
        border-radius: 6px;
        padding: 8px 16px;
        font-size: 0.9rem;
        cursor: pointer;
        font-family: inherit;
    }

    .btn-danger:hover {
        background: color-mix(in oklch, var(--color-error) 10%, transparent);
    }

    .btn-small {
        padding: 5px 12px;
        font-size: 0.82rem;
    }

    .empty-hint {
        color: color-mix(in oklch, var(--color-base-content) 55%, transparent);
        font-size: 0.85rem;
        font-style: italic;
        margin: 0 0 10px;
    }

    .hint-muted {
        color: color-mix(in oklch, var(--color-base-content) 55%, transparent);
        font-size: 0.82rem;
        font-style: italic;
        margin: 8px 0 0;
    }

    /* ── Sub-form wrapper ──────────────────────────────────────────────────── */
    .sub-form {
        background: var(--color-base-200);
        border: 1px solid var(--color-base-300);
        border-radius: 8px;
        padding: 16px;
    }

    /* ── Expiry lots list ──────────────────────────────────────────────────── */
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
        background: var(--color-base-200);
        border-radius: 7px;
        border: 1px solid var(--color-base-300);
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
        background: color-mix(in oklch, var(--color-error) 12%, transparent);
        color: var(--color-error);
    }

    .urgency-critical .urgency-badge {
        background: color-mix(in oklch, var(--color-error) 15%, transparent);
        color: var(--color-error);
    }

    .urgency-warning .urgency-badge {
        background: color-mix(in oklch, var(--color-warning) 18%, transparent);
        color: color-mix(in oklch, var(--color-warning) 70%, var(--color-base-content));
    }

    .urgency-normal .urgency-badge {
        background: color-mix(in oklch, var(--color-success) 15%, transparent);
        color: var(--color-success);
    }

    .lot-info {
        flex: 1;
        display: flex;
        align-items: center;
        gap: 8px;
        flex-wrap: wrap;
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .lot-qty {
        font-weight: 500;
    }

    .lot-date {
        color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
    }

    .lot-batch {
        font-size: 0.74rem;
        background: var(--color-base-300);
        color: var(--color-base-content);
        padding: 1px 6px;
        border-radius: 4px;
    }

    .lot-location {
        font-size: 0.74rem;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        background: var(--color-base-200);
        padding: 1px 6px;
        border-radius: 4px;
    }

    .lot-actions {
        display: flex;
        align-items: center;
        gap: 2px;
        flex-shrink: 0;
    }

    /* ── Lot-detail modal inner shell (outer shell from `<Modal>`) ────────── */
    .dialog-header {
        display: flex;
        align-items: center;
        margin-bottom: 12px;
    }

    .dialog-header h3 {
        margin: 0;
        font-size: 1.05rem;
        color: var(--color-base-content);
    }

    .modal-loading {
        text-align: center;
        padding: 20px;
        color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    }

    .tab-content {
        min-height: 200px;
    }

    .detail-grid {
        display: grid;
        grid-template-columns: auto 1fr;
        gap: 6px 16px;
        font-size: 0.9rem;
        margin-bottom: 16px;
    }

    .detail-grid dt {
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
        font-weight: 500;
    }

    .detail-grid dd {
        color: var(--color-base-content);
        margin: 0;
    }

    .cell-sku {
        font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
        font-size: 0.85rem;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }
</style>
