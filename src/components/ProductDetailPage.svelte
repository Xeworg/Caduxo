<script lang="ts">
    import { onMount } from "svelte";
    import {
        getProduct,
        addProductBarcode,
        removeProductBarcode,
        archiveProduct,
        unarchiveProduct,
        retireProduct,
        listProductLifecycleEvents,
        lifecycleOf,
        type ProductDetailResponse,
        type ProductResponse,
        type ProductBarcodeResponse,
        type ProductLifecycleEventResponse,
    } from "../lib/products.js";
    import {
        listExpiryLotsByProduct,
        type ExpiryLotResponse,
    } from "../lib/expiry_lots.js";
    import { scannerNavigation } from "../lib/navigation.js";
    import {
        resolveBatchCodeDisplay,
        resolveLotLocationDisplay,
    } from "../lib/lotDisplay.js";
    import { type UnitKind } from "../lib/products.js";
    import { LL } from "../i18n/i18n-svelte.js";
    import { humanizeError } from "../lib/errors.js";
    import Modal from "./ui/Modal.svelte";
    import Button from "./ui/Button.svelte";
    import Alert from "./ui/Alert.svelte";
    import Badge from "./ui/Badge.svelte";
    import Icon from "./ui/Icon.svelte";
    import Combobox from "./ui/Combobox.svelte";
    import LotForm from "./LotForm.svelte";
    import ResolveQuantityDialog from "./ResolveQuantityDialog.svelte";
    import ArchiveLotDialog from "./ArchiveLotDialog.svelte";

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
    export let onLifecycleChanged: (action: "archived" | "unarchived" | "retired") => void;
    export let onBarcodeChanged: () => void;

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

    // Unarchive confirmation
    let confirmingUnarchive = false;

    // Retire two-step confirmation
    let confirmingRetire = false;   // first step: reason input
    let retireReason = "";          // reason text
    let retireSecondConfirm = false; // second step: irreversible ack

    // Lifecycle events (audit trail)
    let lifecycleEvents: ProductLifecycleEventResponse[] = [];

    // Lot sub-views
    let showLotForm = false;
    let editingLot: ExpiryLotResponse | null = null;
    let resolvingLot: ExpiryLotResponse | null = null;
    let archivingLot: ExpiryLotResponse | null = null;
    let lotError = "";

    // ── Load ───────────────────────────────────────────────────────────────────

    async function load() {
        loading = true;
        errorMsg = "";
        try {
            detail = await getProduct(productId);
            lots = await listExpiryLotsByProduct(productId);
            if (detail) {
                lifecycleEvents = await listProductLifecycleEvents(detail.product.id);
            }
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
            onBarcodeChanged();
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
            onBarcodeChanged();
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        }
    }

    async function confirmArchive() {
        if (!detail) return;
        try {
            await archiveProduct(detail.product.id);
            confirmingArchive = false;
            await load();
            onLifecycleChanged("archived");
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        }
    }

    async function confirmUnarchive() {
        if (!detail) return;
        try {
            await unarchiveProduct(detail.product.id);
            confirmingUnarchive = false;
            await load();
            onLifecycleChanged("unarchived");
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        }
    }

    async function submitRetire() {
        if (!detail) return;
        const reason = retireReason.trim();
        if (!reason) return; // disabled button prevents this, but guard anyway
        try {
            await retireProduct({ id: detail.product.id, reason, actor: null });
            confirmingRetire = false;
            retireReason = "";
            retireSecondConfirm = false;
            await load();
            onLifecycleChanged("retired");
        } catch (e: unknown) {
            errorMsg = humanizeError(e);
        }
    }

    function cancelRetire() {
        confirmingRetire = false;
        retireReason = "";
        retireSecondConfirm = false;
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

    /**
     * Display label for a lot's location in the product-detail lot list.
     * Sentinel ids (`loc-sentinel-*`) and missing `location_id` render the
     * localized "No location" placeholder; real locations prefer the
     * backend-projected `location_name` over the raw UUID (per
     * `odd/tasks/lot-location-name.md`). Falls back to the UUID only if the
     * lot has a real location but the projection is empty (legacy row).
     */
    function lotLocationLabel(lot: ExpiryLotResponse): string {
        return resolveLotLocationDisplay(
            lot.location_id,
            lot.location_name,
            $LL.common.noLocation(),
        );
    }

    /**
     * Display label for a lot's batch code. Missing batch codes render
     * the localized "No batch code" placeholder rather than falling back
     * to the location id.
     */
    function lotBatchCodeLabel(lot: ExpiryLotResponse): string {
        return resolveBatchCodeDisplay(lot.batch_code, $LL.lotsDetail.noBatchCode());
    }

    /**
     * Opens the selected lot in the Scanner view, preserving the
     * product context so the Scanner can display the product header.
     */
    function openSelectedLotInScanner(lot: ExpiryLotResponse): void {
        scannerNavigation.set({
            lotId: lot.id,
            productId: productId,
            unitType: (lot.unit_type ?? null) as UnitKind | null,
        });
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
{#if lifecycleOf(product) === "retired"}
<span class="badge-retired">{$LL.products.lifecycleRetired()}</span>
{:else if !product.is_active}
<span class="badge-inactive">{$LL.products.lifecycleArchived()}</span>
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
{#if lifecycleOf(product) !== "retired"}
<button
type="button"
class="btn-secondary"
on:click={() => onEdit(product)}
>
{$LL.products.detail.edit()}
</button>
{/if}

{#if lifecycleOf(product) === "active"}
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
  <button
  type="button"
  class="btn-danger"
  on:click={() => (confirmingRetire = true)}
  >
  {$LL.products.retire()}
  </button>

{:else if lifecycleOf(product) === "archived"}
  {#if !confirmingUnarchive}
  <button
  type="button"
  class="btn-secondary"
  on:click={() => (confirmingUnarchive = true)}
  >
  {$LL.products.unarchive()}
  </button>
  {:else}
  <span class="archive-confirm">
  {$LL.products.detail.unarchiveConfirm()}
  <button
  type="button"
  class="btn-primary btn-small"
  on:click={confirmUnarchive}
  >
  {$LL.common.confirm()}
  </button>
  <button
  type="button"
  class="btn-secondary btn-small"
  on:click={() => (confirmingUnarchive = false)}
  >
  {$LL.common.cancel()}
  </button>
  </span>
  {/if}
  <button
  type="button"
  class="btn-danger"
  on:click={() => (confirmingRetire = true)}
  >
  {$LL.products.retire()}
  </button>
{/if}
</div>
</header>

        {#if errorMsg}
            <div class="alert alert-error" role="alert">{errorMsg}</div>
        {/if}

        <!-- ── Lifecycle banner ──────────────────────────────────────────────── -->
        {#if lifecycleOf(product) === "archived"}
            <div class="lifecycle-banner banner-archived" role="alert">
                {$LL.products.detail.bannerArchived()}
            </div>
        {:else if lifecycleOf(product) === "retired"}
            <div class="lifecycle-banner banner-retired" role="alert">
                {$LL.products.detail.bannerRetired()}
            </div>
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
                            {#if lifecycleOf(product) === "active"}
                                <Button
                                    variant="icon"
                                    size="sm"
                                    aria-label={$LL.products.detail.removeBarcode()}
                                    onclick={() => removeBarcode(b)}
                                >
                                    {#snippet iconStart()}
                                        <Icon name="x-mark" size="sm" />
                                    {/snippet}
                                </Button>
                            {/if}
                        </li>
                    {/each}
                </ul>
            {/if}

            {#if lifecycleOf(product) === "active"}
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

        <!-- ── Lifecycle history (read-only) ───────────────────────────────────── -->
        {#if lifecycleEvents.length > 0}
            <section class="section">
                <div class="section-header">
                    <h3>{$LL.products.lifecycleEvents()}</h3>
                </div>
                <ul class="lifecycle-events">
                    {#each lifecycleEvents as event (event.id)}
                        <li class="lifecycle-event">
                            <div class="event-icon">
                                {#if event.event_type === "archived"}
                                    <span class="event-icon-archived" aria-hidden="true">▾</span>
                                {:else if event.event_type === "unarchived"}
                                    <span class="event-icon-unarchived" aria-hidden="true">▴</span>
                                {:else}
                                    <span class="event-icon-retired" aria-hidden="true">✕</span>
                                {/if}
                            </div>
                            <div class="event-body">
                                <span class="event-type">
                                    {#if event.event_type === "archived"}
                                        {$LL.products.lifecycleEventArchived()}
                                    {:else if event.event_type === "unarchived"}
                                        {$LL.products.lifecycleEventUnarchived()}
                                    {:else}
                                        {$LL.products.lifecycleEventRetired()}
                                    {/if}
                                </span>
                                <span class="event-arrow">←</span>
                                <span class="event-to">
                                    {#if event.to_state === "active"}
                                        {$LL.products.lifecycleActive()}
                                    {:else if event.to_state === "archived"}
                                        {$LL.products.lifecycleArchived()}
                                    {:else}
                                        {$LL.products.lifecycleRetired()}
                                    {/if}
                                </span>
                                {#if event.reason}
                                    <span class="event-reason">— {event.reason}</span>
                                {/if}
                                {#if event.actor}
                                    <span class="event-actor">by {event.actor}</span>
                                {/if}
                                <span class="event-date">{formatDate(event.created_at.split("T")[0] ?? event.created_at)}</span>
                            </div>
                        </li>
                    {/each}
                </ul>
            </section>
        {/if}

        <!-- ── Expiry lots ──────────────────────────────────────────────────── -->
        <section class="section">
            <div class="section-header">
                <h3>{$LL.products.detail.expiryLots()}</h3>
                {#if lifecycleOf(product) === "active"}
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
                                <span class="lot-batch">{lotBatchCodeLabel(lot)}</span>
                                {#if lot.location_id}
                                    <span class="lot-location">{lotLocationLabel(lot)}</span>
                                {/if}
                                {#if lot.status === "archived"}
                                    <span class="badge-archived">{$LL.products.detail.lot.archived()}</span>
                                {:else if lot.status === "resolved"}
                                    <span class="badge-resolved">{$LL.products.detail.lot.resolved()}</span>
                                {/if}
                            </div>
                            <div class="lot-actions">
                                {#if lot.status === "active"}
                                    {#if lifecycleOf(product) === "active"}
                                        <Button
                                            variant="icon"
                                            size="sm"
                                            aria-label={$LL.products.detail.lot.resolveQty()}
                                            onclick={() => (resolvingLot = lot)}
                                        >
                                            {#snippet iconStart()}
                                                <Icon name="arrow-down-on-square-stack" size="sm" />
                                            {/snippet}
                                        </Button>
                                        <Button
                                            variant="icon"
                                            size="sm"
                                            aria-label={$LL.products.detail.lot.editLot()}
                                            onclick={() => {
                                                editingLot = lot;
                                                showLotForm = true;
                                            }}
                                        >
                                            {#snippet iconStart()}
                                                <Icon name="pencil" size="sm" />
                                            {/snippet}
                                        </Button>
                                    {/if}
                                    <Button
                                        variant="icon"
                                        size="sm"
                                        aria-label={$LL.products.detail.lot.movementHistory()}
                                        onclick={() => openSelectedLotInScanner(lot)}
                                    >
                                        {#snippet iconStart()}
                                            <Icon name="clipboard-document-list" size="sm" />
                                        {/snippet}
                                    </Button>
                                    {#if lifecycleOf(product) === "active"}
                                        <Button
                                            variant="icon"
                                            size="sm"
                                            aria-label={$LL.products.detail.lot.archiveLot()}
                                            onclick={() => (archivingLot = lot)}
                                        >
                                            {#snippet iconStart()}
                                                <Icon name="archive-box-arrow-down" size="sm" />
                                            {/snippet}
                                        </Button>
                                    {/if}
                                {/if}
                            </div>
                        </li>
                    {/each}
                </ul>
            {/if}
        </section>
    {/if}
</div>

<!-- ── Retire two-step confirm modal ──────────────────────────────────────── -->
{#if confirmingRetire}
    <Modal
        bind:open={confirmingRetire}
        size="md"
        showClose
        closeLabel={$LL.common.close()}
        aria-label={$LL.products.retireConfirm()}
        oncancel={cancelRetire}
        onclose={cancelRetire}
    >
        {#snippet children()}
            {#if !retireSecondConfirm}
                <!-- Step 1: reason input -->
                <header class="dialog-header">
                    <h3>{$LL.products.retireConfirm()}</h3>
                </header>
                <p class="dialog-body-text">{$LL.products.retireConfirmBody()}</p>
                <p class="dialog-body-text dialog-warning">{$LL.products.retireIrreversible()}</p>
                <form class="retire-reason-form" on:submit|preventDefault={() => { if (retireReason.trim()) retireSecondConfirm = true; }}>
                    <label class="retire-reason-label">
                        {$LL.products.retireReasonLabel()}
                        <textarea
                            bind:value={retireReason}
                            placeholder={$LL.products.retireReason()}
                            rows="3"
                            class="textarea textarea-md"
                        ></textarea>
                    </label>
                    <div class="modal-actions">
                        <Button
                            type="submit"
                            variant="primary"
                            disabled={!retireReason.trim()}
                        >
                            Next →
                        </Button>
                        <Button type="button" variant="ghost" onclick={cancelRetire}>
                            {$LL.common.cancel()}
                        </Button>
                    </div>
                </form>
            {:else}
                <!-- Step 2: irreversible acknowledgement -->
                <header class="dialog-header">
                    <h3>{$LL.products.retireConfirm()}</h3>
                </header>
                <p class="dialog-body-text">
                    {$LL.products.retireConfirmBody()}
                </p>
                <div class="retire-final-check">
                    <label class="checkbox-wrapper">
                        <input
                            type="checkbox"
                            bind:checked={retireSecondConfirm}
                        />
                        {$LL.products.retireConfirm()}
                    </label>
                </div>
                <div class="modal-actions">
                    <Button
                        variant="danger"
                        disabled={!retireSecondConfirm}
                        onclick={submitRetire}
                    >
                        {$LL.products.retire()} {$LL.products.retired()}
                    </Button>
                    <Button type="button" variant="ghost" onclick={cancelRetire}>
                        {$LL.common.cancel()}
                    </Button>
                </div>
            {/if}
        {/snippet}
    </Modal>
{/if}

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

    /* ── Lifecycle banners ─────────────────────────────────────────────────── */
    .lifecycle-banner {
        padding: 10px 14px;
        border-radius: 6px;
        font-size: 0.88rem;
    }

    .banner-archived {
        background: color-mix(in oklch, var(--color-base-300) 60%, transparent);
        color: var(--color-base-content);
        border: 1px solid var(--color-base-300);
    }

    .banner-retired {
        background: color-mix(in oklch, var(--color-warning) 12%, transparent);
        color: color-mix(in oklch, var(--color-warning) 80%, var(--color-base-content));
        border: 1px solid color-mix(in oklch, var(--color-warning) 35%, transparent);
    }

    .badge-retired {
        font-size: 0.7rem;
        background: color-mix(in oklch, var(--color-warning) 18%, transparent);
        color: color-mix(in oklch, var(--color-warning) 80%, var(--color-base-content));
        border-radius: 4px;
        padding: 2px 7px;
    }

    /* ── Lifecycle history section ────────────────────────────────────────── */
    .lifecycle-events {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .lifecycle-event {
        display: flex;
        align-items: flex-start;
        gap: 10px;
        padding: 8px 10px;
        background: var(--color-base-200);
        border-radius: 6px;
        font-size: 0.85rem;
    }

    .event-icon {
        flex-shrink: 0;
        font-size: 1rem;
        line-height: 1.4;
    }

    .event-icon-archived { color: color-mix(in oklch, var(--color-base-content) 50%, transparent); }
    .event-icon-unarchived { color: var(--color-success); }
    .event-icon-retired { color: var(--color-warning); }

    .event-body {
        display: flex;
        align-items: baseline;
        flex-wrap: wrap;
        gap: 4px 8px;
        color: var(--color-base-content);
    }

    .event-type {
        font-weight: 600;
    }

    .event-arrow {
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    }

    .event-reason {
        font-style: italic;
        color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    }

    .event-actor {
        font-size: 0.8rem;
        color: color-mix(in oklch, var(--color-base-content) 55%, transparent);
    }

    .event-date {
        font-size: 0.8rem;
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
        margin-left: auto;
    }

    /* ── Retire two-step modal ───────────────────────────────────────────── */
    .dialog-body-text {
        margin: 0 0 10px;
        font-size: 0.9rem;
        color: var(--color-base-content);
    }

    .dialog-warning {
        color: var(--color-warning);
        font-weight: 600;
    }

    .retire-reason-form {
        display: flex;
        flex-direction: column;
        gap: 10px;
    }

    .retire-reason-label {
        display: flex;
        flex-direction: column;
        gap: 4px;
        font-size: 0.85rem;
        color: var(--color-base-content);
    }

    .retire-reason-label textarea {
        padding: 8px 10px;
        border: 1px solid var(--color-base-300);
        border-radius: 6px;
        font-family: inherit;
        font-size: 0.9rem;
        resize: vertical;
        background: var(--color-base-100);
        color: var(--color-base-content);
    }

    .retire-reason-label textarea:focus {
        outline: 2px solid var(--color-primary);
        border-color: var(--color-primary);
    }

    .retire-final-check {
        margin-bottom: 10px;
    }

    .retire-final-check .checkbox-wrapper {
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
        font-size: 0.88rem;
        color: var(--color-base-content);
    }
</style>
