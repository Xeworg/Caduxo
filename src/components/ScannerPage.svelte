<!--
  ScannerPage.svelte — top-level Scanner surface
  (PR 2 of `scanner-quick-operations`).

  Three modes (Sale, Registration, Stock-out) share a single primary
  scan input that submits on Enter and debounces repeated identical
  scans within 400 ms of the previous successful resolution. The
  resolved scan state, chosen lot, quantity, and in-progress form
  survive mode switches — switching modes MUST NOT issue a fresh
  lookup for the previously scanned value (per spec scenario
  `switching modes preserves the in-progress scan`).

  Flow per mode (mirrored from the spec):
  - **Sale** — resolves to a LotMatch (no lot picker; FEFO bypassed)
    or ProductMatch (lot picker honours the active FEFO policy).
    Quantity defaults to 1, editable. Source location is auto-
    selected when the lot has a single balance, otherwise the user
    must choose. Confirm calls `create_lot_movement` with
    `kind = "exit:sale"`. No mutation before confirm.
  - **Registration** — LotMatch/ProductMatch opens the existing-lot
    creation flow (LotForm) with the resolved product, the active
    store, and the scanned value as a candidate `batch_code`.
    Unknown opens a quick product creation flow that preserves the
    scanned value verbatim and lets the user route it to either
    SKU or barcode.
  - **Stock-out** — same resolution rules as Sale. Renders a reason
    picker that lists the seven non-sale exit reasons; `exit:sale`
    is intentionally absent. Notes are required for
    `exit:inventory_adjustment` and `exit:other`.

  FEFO policy is read from `SettingsResponse` on mount. The Scanner
  tab MUST NOT issue a separate IPC call to read the policy — it
  always reflects what the backend last reported.

  Scanner input is disabled when `hasStore()` returns false; the
  page renders a translated "create the first store first" notice
  in that case (per spec scenario `Scanner tab is unavailable until
  first store exists`).
-->
<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import { locale } from "../i18n/locale.svelte.js";
  import { humanizeError } from "../lib/errors.js";
  import {
    resolveScannerCode,
    type ScannerResolveResult,
    type ScannerProductMatch,
  } from "../lib/scanner.js";
  import {
    getSettings,
    hasStore as hasStoreCommand,
    type FefoPolicy,
    type SettingsResponse,
  } from "../lib/stores.js";
  import {
    createLotMovement,
    getLotLocationBalances,
    type LotLocationBalance,
    type MovementKind,
  } from "../lib/lot_movements.js";
  import type { ProductResponse } from "../lib/products.js";
  import type { ExpiryLotResponse } from "../lib/expiry_lots.js";
  import {
    createProduct,
    addProductBarcodeOnCreate,
    suggestedProductAlertDays,
  } from "../lib/products.js";
  import Button from "./ui/Button.svelte";
  import Select from "./ui/Select.svelte";
  import Input from "./ui/Input.svelte";
  import Alert from "./ui/Alert.svelte";
  import LoadingState from "./ui/LoadingState.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import LotForm from "./LotForm.svelte";

  // ── Mode union ────────────────────────────────────────────────────────────
  type Mode = "sale" | "registration" | "stock_out";

  // Non-sale exit reasons for Stock-out mode (mirrors the spec scenario
  // `Stock-out cannot reference the Sale reason`).
  const STOCK_OUT_REASONS: ReadonlyArray<MovementKind> = [
    "exit:waste",
    "exit:expired",
    "exit:damaged",
    "exit:internal_consumption",
    "exit:return_to_supplier",
    "exit:inventory_adjustment",
    "exit:other",
  ];

  // Reasons that require notes (mirrors RegisterExitModal's REQUIRES_NOTES).
  const REQUIRES_NOTES: ReadonlyArray<MovementKind> = [
    "exit:inventory_adjustment",
    "exit:other",
  ];

  function reasonLabel(kind: MovementKind): string {
    switch (kind) {
      case "exit:waste":
        return $LL.lotMovements.exitReasons.waste();
      case "exit:expired":
        return $LL.lotMovements.exitReasons.expired();
      case "exit:damaged":
        return $LL.lotMovements.exitReasons.damaged();
      case "exit:internal_consumption":
        return $LL.lotMovements.exitReasons.internalConsumption();
      case "exit:return_to_supplier":
        return $LL.lotMovements.exitReasons.returnToSupplier();
      case "exit:inventory_adjustment":
        return $LL.lotMovements.exitReasons.inventoryAdjustmentExit();
      case "exit:other":
        return $LL.lotMovements.exitReasons.other();
      default:
        return kind;
    }
  }

  // ── State ─────────────────────────────────────────────────────────────────

  let mode: Mode = $state("sale");

  // Scanner input + lookup bookkeeping
  let scanInput = $state("");
  let scanBusy = $state(false);
  let scanError = $state("");
  let debounceNotice = $state("");

  // 400 ms debounce guard. Per spec scenario `rapid double-scan is debounced`,
  // a repeat of the same trimmed value within 400 ms of the previous
  // successful resolution is ignored.
  let lastResolvedAt = 0;
  let lastResolvedValue = "";

  // Persisted active settings + page-level "no store" gate.
  let settings = $state<SettingsResponse | null>(null);
  let loadingSettings = $state(true);
  let settingsError = $state("");
  let hasStoreValue: boolean | null = $state(null);

  // Resolved scan state. Persisted across mode switches so the user can
  // move from Sale to Registration without losing their work.
  let resolved = $state<ScannerResolveResult | null>(null);

  // Per-lot state for Sale and Stock-out modes.
  let selectedLotId = $state("");
  let quantityStr = $state("1");
  let quantity = $derived(
    Number.parseFloat(quantityStr) || 0,
  );
  let locationId = $state("");
  let lotBalances = $state<LotLocationBalance[]>([]);
  let loadingBalances = $state(false);

  // Stock-out specific state.
  let exitReason = $state("");
  let notes = $state("");

  // Common mutation bookkeeping.
  let submitting = $state(false);
  let mutationError = $state("");
  let successNotice = $state("");

  // Registration-specific state.
  // For Unknown scans: route the scanned value into either SKU or barcode.
  type QuickRoute = "sku" | "barcode" | "unrouted";
  let quickRoute = $state<QuickRoute>("unrouted");
  let quickSku = $state("");
  let quickBarcode = $state("");
  let quickDescription = $state("");
  let quickAlertDaysStr = $state("30");
  let quickAlertDays = $derived(
    Number.parseInt(quickAlertDaysStr, 10) || 0,
  );
  let quickNotes = $state("");
  let quickCreateBusy = $state(false);
  let quickCreateError = $state("");

  // LotMatch/ProductMatch registration flow: open the existing LotForm.
  let registrationLotFormOpen = $state(false);

  // ── Derived ───────────────────────────────────────────────────────────────

  // Effective FEFO policy from settings (falls back to `suggest_fefo` on
  // fresh installs — the backend returns the same default).
  let fefoPolicy: FefoPolicy = $derived(
    (settings?.scanner_fefo_policy ?? "suggest_fefo") as FefoPolicy,
  );

  // The lot the Sale/Stock-out modes are operating on. `LotMatch` always
  // uses the scanned lot (FEFO bypass). `ProductMatch` honours the policy.
  type ResolvedLot = { lot: ExpiryLotResponse; source: "scanned" | "fefo" | "manual" };

  let resolvedLot: ResolvedLot | null = $derived.by((): ResolvedLot | null => {
    if (!resolved) return null;
    if (resolved.match_type === "lot_match") {
      return { lot: resolved.lot, source: "scanned" };
    }
    if (resolved.match_type === "product_match") {
      const lots = resolved.lots;
      if (lots.length === 0) return null;
      if (selectedLotId) {
        const found = lots.find((l) => l.id === selectedLotId);
        if (found) return { lot: found, source: "manual" };
      }
      if (fefoPolicy === "manual_lot_choice") return null;
      // suggest_fefo / require_fefo: preselect the FEFO lot.
      return { lot: lots[0], source: "fefo" };
    }
    return null;
  });

  // Active product from the resolve result. Drives the Registration flow.
  let activeProduct: ProductResponse | null = $derived.by((): ProductResponse | null => {
    if (!resolved) return null;
    if (resolved.match_type === "lot_match") return resolved.product;
    if (resolved.match_type === "product_match") return resolved.product;
    return null;
  });

  // Reason picker options for Stock-out (excludes `exit:sale`).
  let stockOutReasonOptions = $derived(
    STOCK_OUT_REASONS.map((r) => ({
      value: r,
      label: reasonLabel(r),
    })),
  );

  // Lot picker options for ProductMatch (read-only when `require_fefo`).
  let lotPickerOptions = $derived.by((): Array<{ value: string; label: string }> => {
    if (!resolved || resolved.match_type !== "product_match") return [];
    return resolved.lots.map((lot) => {
      const exp = lot.expiry_date;
      const qty = lot.quantity;
      const unit = lot.unit ? ` ${lot.unit}` : "";
      return {
        value: lot.id,
        label: `${exp} · ${qty}${unit}${lot.batch_code ? ` · ${lot.batch_code}` : ""}`,
      };
    });
  });

  // When a resolved lot is in effect, the picker is editable only when the
  // policy is `suggest_fefo` or `manual_lot_choice`.
  let lotPickerDisabled = $derived.by((): boolean => {
    if (!resolved) return false;
    if (resolved.match_type === "lot_match") return true;
    if (resolved.match_type === "product_match") {
      if (fefoPolicy === "require_fefo") return true;
      return false;
    }
    return false;
  });

  // Single-location balance auto-preselection.
  let availableBalances = $derived(
    lotBalances.filter((b) => b.balance > 0),
  );
  let preselectedLocationId = $derived.by((): string => {
    if (availableBalances.length === 1) return availableBalances[0].location_id;
    return "";
  });

  // The lot balance for the currently selected source location.
  let selectedBalance = $derived.by((): number => {
    if (!locationId) return 0;
    return lotBalances.find((b) => b.location_id === locationId)?.balance ?? 0;
  });

  // Location picker options for Sale and Stock-out.
  let locationOptions = $derived([
    { value: "", label: $LL.scanner.stockOut.reasonPlaceholder() },
    ...availableBalances.map((b) => ({
      value: b.location_id,
      label: `${b.location_id} (${b.balance})`,
    })),
  ]);

  // Confirms gated by mode-specific validation.
  let canConfirmSale = $derived.by((): boolean => {
    if (!resolvedLot) return false;
    if (quantity <= 0) return false;
    if (!locationId) return false;
    if (quantity > selectedBalance) return false;
    return true;
  });

  let canConfirmStockOut = $derived.by((): boolean => {
    if (!resolvedLot) return false;
    if (!exitReason) return false;
    if (quantity <= 0) return false;
    if (!locationId) return false;
    if (quantity > selectedBalance) return false;
    if (REQUIRES_NOTES.includes(exitReason as MovementKind) && !notes.trim()) return false;
    return true;
  });

  let requiresNotes = $derived(
    REQUIRES_NOTES.includes(exitReason as MovementKind),
  );

  // Scanner-tab availability: the page is interactive only when settings
  // loaded and an active store exists.
  let pageReady = $derived(
    !loadingSettings && !settingsError && hasStoreValue === true,
  );

  // ── Init ──────────────────────────────────────────────────────────────────

  onMount(async () => {
    await Promise.all([loadSettings(), loadHasStore()]);
  });

  async function loadSettings(): Promise<void> {
    loadingSettings = true;
    settingsError = "";
    try {
      settings = await getSettings();
    } catch (e) {
      settingsError = $LL.scanner.errors.loadSettingsFailed({
        msg: humanizeError(e),
      });
      settings = null;
    } finally {
      loadingSettings = false;
    }
  }

  async function loadHasStore(): Promise<void> {
    try {
      hasStoreValue = await hasStoreCommand();
    } catch {
      hasStoreValue = false;
    }
  }

  async function loadLotBalances(lotId: string): Promise<void> {
    loadingBalances = true;
    try {
      lotBalances = await getLotLocationBalances(lotId);
    } catch (e) {
      mutationError = $LL.scanner.errors.loadBalancesFailed({
        msg: humanizeError(e),
      });
      lotBalances = [];
    } finally {
      loadingBalances = false;
    }
  }

  // When the resolved lot changes, refresh balances + auto-pick a location.
  $effect(() => {
    const lot = resolvedLot?.lot;
    if (lot) {
      untrack(() => {
        locationId = "";
        void loadLotBalances(lot.id);
      });
    } else {
      lotBalances = [];
      locationId = "";
    }
  });

  // Auto-preselect single-balance locations.
  $effect(() => {
    if (preselectedLocationId && !locationId) {
      locationId = preselectedLocationId;
    }
  });

  // ── Debounce / scan ───────────────────────────────────────────────────────

  async function handleScanSubmit(): Promise<void> {
    const trimmed = scanInput.trim();
    if (!trimmed) return;
    if (!hasStoreValue) return;

    const now = Date.now();
    if (
      trimmed === lastResolvedValue &&
      now - lastResolvedAt < 400
    ) {
      debounceNotice = $LL.scanner.debouncedNotice();
      scanInput = "";
      return;
    }

    scanBusy = true;
    scanError = "";
    debounceNotice = "";
    successNotice = "";
    mutationError = "";

    try {
      const result = await resolveScannerCode(
        { scanned_value: trimmed },
        locale.current,
      );
      lastResolvedAt = Date.now();
      lastResolvedValue = trimmed;
      applyResolveResult(result);
    } catch (e) {
      scanError = $LL.scanner.errors.lookupFailed({
        msg: humanizeError(e),
      });
    } finally {
      scanBusy = false;
      scanInput = "";
    }
  }

  function applyResolveResult(result: ScannerResolveResult): void {
    resolved = result;
    successNotice = "";
    mutationError = "";

    if (result.match_type === "product_match") {
      // Apply the FEFO policy at the resolution boundary so mode-specific
      // derived gates see the right state. For `suggest_fefo` /
      // `require_fefo` the FEFO lot is at index 0; for `manual_lot_choice`
      // we leave `selectedLotId` empty until the user picks one.
      selectedLotId =
        fefoPolicy === "manual_lot_choice" ? "" : result.lots[0]?.id ?? "";
      locationId = "";
    } else if (result.match_type === "lot_match") {
      // FEFO is bypassed for direct lot scans.
      selectedLotId = result.lot.id;
      locationId = "";
    } else {
      // Unknown — clear the selection for the previous resolve so the
      // Registration quick-create form starts clean.
      selectedLotId = "";
      locationId = "";
      quickRoute = "unrouted";
      quickSku = result.scanned_value;
      quickBarcode = "";
      quickDescription = "";
      quickAlertDaysStr = "30";
      quickNotes = "";
    }
  }

  function handleScanKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleScanSubmit();
    }
  }

  // ── Mode switching ───────────────────────────────────────────────────────

  function setMode(next: Mode): void {
    mode = next;
    // Per spec, switching modes MUST NOT discard the resolved scan,
    // chosen lot, or in-progress confirmation form. We intentionally
    // do NOT reset `resolved`, `selectedLotId`, `quantityStr`,
    // `exitReason`, `notes`, or `locationId` here. Per-mode validation
    // and confirm-gating derive from the mode on the fly.
  }

  // ── Sale confirm ─────────────────────────────────────────────────────────

  async function confirmSale(): Promise<void> {
    if (!resolvedLot) return;
    if (!canConfirmSale) {
      mutationError = invalidSaleMessage();
      return;
    }
    submitting = true;
    mutationError = "";
    try {
      await createLotMovement(
        {
          lot_id: resolvedLot.lot.id,
          kind: "exit:sale" as MovementKind,
          direction: null,
          quantity,
          source_location_id: locationId || null,
          destination_location_id: null,
          notes: null,
        },
        locale.current,
      );
      successNotice = $LL.scanner.sale.success({
        qty: quantity,
        sku: activeProduct?.sku ?? resolvedLot.lot.batch_code ?? "",
      });
      resetMutationForm();
    } catch (e) {
      mutationError = $LL.scanner.errors.saveFailed({
        msg: humanizeError(e),
      });
    } finally {
      submitting = false;
    }
  }

  function invalidSaleMessage(): string {
    if (!resolvedLot) return $LL.scanner.sale.invalid.lot();
    if (quantity <= 0) return $LL.scanner.sale.invalid.quantity();
    if (!locationId) return $LL.scanner.sale.invalid.location();
    if (quantity > selectedBalance) {
      return $LL.scanner.sale.invalid.quantityExceeds({
        available: selectedBalance,
      });
    }
    return $LL.scanner.sale.invalid.lot();
  }

  // ── Stock-out confirm ─────────────────────────────────────────────────────

  async function confirmStockOut(): Promise<void> {
    if (!resolvedLot) return;
    if (!canConfirmStockOut) {
      mutationError = invalidStockOutMessage();
      return;
    }
    submitting = true;
    mutationError = "";
    try {
      await createLotMovement(
        {
          lot_id: resolvedLot.lot.id,
          kind: exitReason as MovementKind,
          direction: null,
          quantity,
          source_location_id: locationId || null,
          destination_location_id: null,
          notes: notes.trim() || null,
        },
        locale.current,
      );
      successNotice = $LL.scanner.stockOut.success({
        qty: quantity,
        sku: activeProduct?.sku ?? resolvedLot.lot.batch_code ?? "",
        reason: reasonLabel(exitReason as MovementKind),
      });
      resetMutationForm();
    } catch (e) {
      mutationError = $LL.scanner.errors.saveFailed({
        msg: humanizeError(e),
      });
    } finally {
      submitting = false;
    }
  }

  function invalidStockOutMessage(): string {
    if (!resolvedLot) return $LL.scanner.stockOut.invalid.lot();
    if (!exitReason) return $LL.scanner.stockOut.invalid.reason();
    if (quantity <= 0) return $LL.scanner.stockOut.invalid.quantity();
    if (!locationId) return $LL.scanner.stockOut.invalid.location();
    if (quantity > selectedBalance) {
      return $LL.scanner.stockOut.invalid.quantityExceeds({
        available: selectedBalance,
      });
    }
    if (REQUIRES_NOTES.includes(exitReason as MovementKind) && !notes.trim()) {
      return $LL.scanner.stockOut.invalid.notes();
    }
    return $LL.scanner.stockOut.invalid.lot();
  }

  function resetMutationForm(): void {
    // Per spec `Sale success resets the form for the next scan` and
    // `Stock-out success resets the form`. We clear only the per-confirm
    // state — the resolved scan + lot selection become stale after the
    // quantity change, so we drop them as well to avoid reusing a
    // mutated lot for the next confirm. The debounce guard is also
    // reset so a subsequent identical scan starts a fresh resolution.
    resolved = null;
    selectedLotId = "";
    quantityStr = "1";
    locationId = "";
    exitReason = "";
    notes = "";
    lotBalances = [];
    lastResolvedAt = 0;
    lastResolvedValue = "";
  }

  // ── Registration: existing-product lot creation ───────────────────────────

  function openRegistrationLotForm(): void {
    registrationLotFormOpen = true;
  }

  function closeRegistrationLotForm(): void {
    registrationLotFormOpen = false;
  }

  function onRegistrationLotSaved(lot: ExpiryLotResponse): void {
    registrationLotFormOpen = false;
    successNotice = $LL.scanner.registration.successLot({
      batchCode: lot.batch_code ?? "",
    });
    resolved = null;
    selectedLotId = "";
    quantityStr = "1";
    lotBalances = [];
    lastResolvedAt = 0;
    lastResolvedValue = "";
  }

  // ── Registration: quick product creation (Unknown) ────────────────────────

  function setQuickRoute(next: QuickRoute): void {
    quickRoute = next;
    if (next === "sku") {
      quickSku = resolved && resolved.match_type === "unknown" ? resolved.scanned_value : "";
      if (!quickBarcode.trim()) {
        quickBarcode = "";
      }
    } else if (next === "barcode") {
      quickBarcode = resolved && resolved.match_type === "unknown" ? resolved.scanned_value : "";
      if (!quickSku.trim()) {
        quickSku = "";
      }
    }
    // "unrouted" leaves the inputs alone so the user can decide.
  }

  async function confirmQuickCreate(): Promise<void> {
    if (!resolved || resolved.match_type !== "unknown") return;
    if (quickRoute === "unrouted") {
      quickCreateError = $LL.scanner.registration.routeUnrouted();
      return;
    }
    if (!quickSku.trim()) {
      quickCreateError = $LL.scanner.registration.invalid.sku();
      return;
    }
    if (!quickDescription.trim()) {
      quickCreateError = $LL.scanner.registration.invalid.description();
      return;
    }
    if (quickAlertDays < 0) {
      quickCreateError = $LL.scanner.registration.invalid.alertDays();
      return;
    }

    quickCreateBusy = true;
    quickCreateError = "";
    try {
      const sku = quickRoute === "sku" ? resolved.scanned_value : quickSku.trim();
      const created = await createProduct({
        sku,
        description: quickDescription.trim(),
        category_ids: null,
        default_unit: null,
        default_unit_id: null,
        default_alert_days_before: quickAlertDays,
        notes: quickNotes.trim() || null,
      });
      successNotice = $LL.scanner.registration.successProduct({ sku: created.sku });

      // Attach the barcode when the user routed the scanned value to the
      // barcode slot. The wrapper returns a structured failure rather
      // than throwing on duplicate so the user gets inline feedback.
      if (quickRoute === "barcode" && quickBarcode.trim()) {
        const result = await addProductBarcodeOnCreate({
          product_id: created.id,
          barcode: quickBarcode.trim(),
          barcode_type: null,
          is_primary: false,
        });
        if (result.ok) {
          successNotice = `${successNotice} ${$LL.scanner.registration.successBarcode({
            barcode: result.barcode.barcode,
          })}`;
        } else {
          // Non-fatal — the product is already created. Surface the
          // barcode failure as an inline notice so the user can retry
          // via the Products page.
          quickCreateError = result.message;
        }
      }

      // Reset the form so the next scan resumes the empty state.
      resolved = null;
      quickRoute = "unrouted";
      quickSku = "";
      quickBarcode = "";
      quickDescription = "";
      quickAlertDaysStr = "30";
      quickNotes = "";
      lastResolvedAt = 0;
      lastResolvedValue = "";
    } catch (e) {
      quickCreateError = $LL.scanner.errors.saveFailed({
        msg: humanizeError(e),
      });
    } finally {
      quickCreateBusy = false;
    }
  }

  // Helper: suggested alert days default (mirrors the dashboard default).
  async function loadSuggestedAlertDays(): Promise<void> {
    try {
      const v = await suggestedProductAlertDays();
      quickAlertDaysStr = String(v);
    } catch {
      // Keep the local default of 30.
    }
  }

  // When the user enters Registration mode for an Unknown scan, pull the
  // suggested alert days from the backend so the form starts with a sane
  // default (matches the existing ProductForm behaviour).
  $effect(() => {
    if (
      mode === "registration" &&
      resolved?.match_type === "unknown" &&
      quickAlertDaysStr === ""
    ) {
      void loadSuggestedAlertDays();
    }
  });
</script>

<div class="scanner-page">
  <header class="page-header">
    <h1 class="page-title">{$LL.scanner.pageTitle()}</h1>
    <p class="page-subtitle">{$LL.scanner.pageSubtitle()}</p>
  </header>

  {#if loadingSettings}
    <LoadingState variant="text" label={$LL.scanner.loadingSettings()} />
  {:else if settingsError}
    <Alert variant="error">{settingsError}</Alert>
  {:else if hasStoreValue === false}
    <EmptyState
      icon="info"
      title={$LL.scanner.noStore.title()}
      body={$LL.scanner.noStore.body()}
    />
  {:else if pageReady}
    <!-- Mode tabs (Sale / Registration / Stock-out). Inline DaisyUI
         tabs with manual ARIA wiring because the Tabs primitive
         expects a Snippet per item and our panels depend on per-mode
         derived state. -->
    <div
      role="tablist"
      class="tabs tabs-border mode-tabs"
      aria-label={$LL.scanner.pageTitle()}
      aria-orientation="horizontal"
    >
      <button
        type="button"
        role="tab"
        id="mode-sale-tab"
        data-mode-tab="sale"
        class="tab"
        class:tab-active={mode === "sale"}
        aria-selected={mode === "sale"}
        aria-controls="mode-sale-panel"
        tabindex={mode === "sale" ? 0 : -1}
        onclick={() => setMode("sale")}
      >
        {$LL.scanner.modes.sale()}
      </button>
      <button
        type="button"
        role="tab"
        id="mode-registration-tab"
        data-mode-tab="registration"
        class="tab"
        class:tab-active={mode === "registration"}
        aria-selected={mode === "registration"}
        aria-controls="mode-registration-panel"
        tabindex={mode === "registration" ? 0 : -1}
        onclick={() => setMode("registration")}
      >
        {$LL.scanner.modes.registration()}
      </button>
      <button
        type="button"
        role="tab"
        id="mode-stock-out-tab"
        data-mode-tab="stock_out"
        class="tab"
        class:tab-active={mode === "stock_out"}
        aria-selected={mode === "stock_out"}
        aria-controls="mode-stock-out-panel"
        tabindex={mode === "stock_out" ? 0 : -1}
        onclick={() => setMode("stock_out")}
      >
        {$LL.scanner.modes.stockOut()}
      </button>
    </div>

    <!-- Scanner input. Single primary input across all three modes;
         the active mode decides the post-resolution flow. -->
    <div class="scanner-input-row">
      <Input
        bind:value={scanInput}
        label={$LL.scanner.input.label()}
        placeholder={$LL.scanner.input.placeholder()}
        aria-label={$LL.scanner.input.ariaLabel()}
        disabled={scanBusy}
      />
      <input
        type="text"
        aria-hidden="true"
        tabindex="-1"
        class="hidden-anchor"
        onkeydown={handleScanKeydown}
        bind:value={scanInput}
      />
      <Button
        variant="primary"
        disabled={scanBusy || !scanInput.trim()}
        onclick={() => void handleScanSubmit()}
      >
        {$LL.common.save()}
      </Button>
    </div>

    {#if scanBusy}
      <LoadingState variant="text" label={$LL.common.loading()} />
    {/if}
    {#if debounceNotice}
      <Alert variant="info">{debounceNotice}</Alert>
    {/if}
    {#if scanError}
      <Alert variant="error">{scanError}</Alert>
    {/if}
    {#if successNotice}
      <Alert variant="success">{successNotice}</Alert>
    {/if}
    {#if mutationError}
      <Alert variant="error">{mutationError}</Alert>
    {/if}

    {#if resolved?.match_type === "lot_match"}
      <Alert variant="info">{$LL.scanner.sale.selectedLotHint()}</Alert>
    {/if}

    <!-- Sale panel -->
    <div
      role="tabpanel"
      id="mode-sale-panel"
      aria-labelledby="mode-sale-tab"
      hidden={mode !== "sale"}
      tabindex={mode === "sale" ? 0 : -1}
    >
      {#if mode === "sale"}
        {#if resolved?.match_type === "unknown"}
          <Alert variant="warning">
            <strong>{$LL.scanner.unknownTitle()}:</strong>
            {resolved.scanned_value}
          </Alert>
        {:else if resolved && activeProduct && resolvedLot}
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.sale.selectedProduct()}:</strong>
              {activeProduct.sku}
            </p>
            {#if resolved.match_type === "product_match" && resolved.lots.length > 1}
              <Select
                value={selectedLotId}
                options={lotPickerOptions}
                aria-label={$LL.scanner.sale.lotPickerLabel()}
                disabled={lotPickerDisabled}
                onchange={(v: string) => (selectedLotId = v)}
              />
              {#if fefoPolicy === "require_fefo"}
                <p class="hint-required">{$LL.scanner.sale.fefoRequiredNotice()}</p>
              {/if}
            {:else}
              <p class="resolved-line">
                <strong>{$LL.scanner.sale.selectedLot()}:</strong>
                {resolvedLot.lot.expiry_date}
                {#if resolvedLot.lot.batch_code}
                  · {resolvedLot.lot.batch_code}
                {/if}
              </p>
            {/if}

            {#if loadingBalances}
              <LoadingState variant="text" label={$LL.common.loading()} />
            {:else if availableBalances.length === 0}
              <Alert variant="warning">{$LL.scanner.sale.invalid.quantity()}</Alert>
            {:else}
              <Select
                value={locationId}
                options={locationOptions}
                aria-label={$LL.scanner.sale.locationLabel()}
                disabled={availableBalances.length === 1}
                onchange={(v: string) => (locationId = v)}
              />
            {/if}

            <fieldset class="fieldset">
              <legend class="fieldset-legend">
                {$LL.scanner.sale.quantityLabel()}
              </legend>
              <input
                type="number"
                class="input input-md motion-reduce:transition-none w-full"
                min="0"
                step="0.01"
                inputmode="decimal"
                value={quantityStr}
                oninput={(e: Event) => {
                  const target = e.currentTarget as HTMLInputElement;
                  quantityStr = target.value;
                }}
              />
              {#if selectedBalance > 0}
                <p class="label">
                  {$LL.lotMovements.available({ available: selectedBalance })}
                </p>
              {/if}
            </fieldset>

            <div class="confirm-row">
              <Button
                variant="primary"
                disabled={!canConfirmSale || submitting}
                loading={submitting}
                onclick={() => void confirmSale()}
              >
                {submitting ? $LL.scanner.sale.confirming() : $LL.scanner.sale.confirm()}
              </Button>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Registration panel -->
    <div
      role="tabpanel"
      id="mode-registration-panel"
      aria-labelledby="mode-registration-tab"
      hidden={mode !== "registration"}
      tabindex={mode === "registration" ? 0 : -1}
    >
      {#if mode === "registration"}
        {#if resolved?.match_type === "unknown"}
          <div class="registration-unknown">
            <h3 class="section-title">{$LL.scanner.registration.unknownHeading()}</h3>
            <p class="section-body">{$LL.scanner.registration.unknownBody()}</p>

            <p class="resolved-line">
              <strong>{$LL.scanner.registration.scannedValueLabel()}:</strong>
              <code class="scanned-code">{resolved.scanned_value || $LL.scanner.registration.scannedValuePlaceholder()}</code>
            </p>

            <fieldset class="fieldset">
              <legend class="fieldset-legend sr-only">{$LL.scanner.registration.unknownHeading()}</legend>
              <div class="route-row">
                <label class="route-option">
                  <input
                    type="radio"
                    name="quick-route"
                    class="radio radio-primary radio-sm"
                    checked={quickRoute === "sku"}
                    onchange={() => setQuickRoute("sku")}
                  />
                  <span>{$LL.scanner.registration.routeAsSku()}</span>
                </label>
                <label class="route-option">
                  <input
                    type="radio"
                    name="quick-route"
                    class="radio radio-primary radio-sm"
                    checked={quickRoute === "barcode"}
                    onchange={() => setQuickRoute("barcode")}
                  />
                  <span>{$LL.scanner.registration.routeAsBarcode()}</span>
                </label>
              </div>
            </fieldset>

            <div class="grid-2">
              <Input
                bind:value={quickSku}
                label={$LL.scanner.registration.skuLabel()}
                required
                disabled={quickRoute === "barcode"}
              />
              <Input
                bind:value={quickBarcode}
                label={$LL.scanner.registration.barcodeLabel()}
                disabled={quickRoute === "sku"}
              />
            </div>

            <Input
              bind:value={quickDescription}
              label={$LL.scanner.registration.descriptionLabel()}
              required
            />

            <fieldset class="fieldset">
              <legend class="fieldset-legend">
                {$LL.scanner.registration.alertDaysLabel()}
              </legend>
              <input
                type="number"
                class="input input-md motion-reduce:transition-none w-full"
                min="0"
                step="1"
                inputmode="numeric"
                value={quickAlertDaysStr}
                oninput={(e: Event) => {
                  const target = e.currentTarget as HTMLInputElement;
                  quickAlertDaysStr = target.value;
                }}
              />
            </fieldset>

            <fieldset class="fieldset">
              <legend class="fieldset-legend">{$LL.scanner.registration.notesLabel()}</legend>
              <textarea
                class="textarea textarea-md w-full motion-reduce:transition-none"
                rows="2"
                bind:value={quickNotes}
              ></textarea>
            </fieldset>

            {#if quickCreateError}
              <Alert variant="error">{quickCreateError}</Alert>
            {/if}

            <div class="confirm-row">
              <Button
                variant="primary"
                disabled={quickCreateBusy || quickRoute === "unrouted"}
                loading={quickCreateBusy}
                onclick={() => void confirmQuickCreate()}
              >
                {quickCreateBusy
                  ? $LL.scanner.registration.creating()
                  : $LL.scanner.registration.createProduct()}
              </Button>
            </div>
          </div>
        {:else if resolved && activeProduct}
          <div class="registration-existing">
            <h3 class="section-title">{$LL.scanner.registration.newLotHeading()}</h3>
            <p class="section-body">{$LL.scanner.registration.newLotBody()}</p>

            <p class="resolved-line">
              <strong>{$LL.scanner.registration.scannedValueLabel()}:</strong>
              <code class="scanned-code">
                {#if resolved.match_type === "lot_match"}
                  {resolved.lot.batch_code ?? $LL.scanner.registration.scannedValuePlaceholder()}
                {:else}
                  {(resolved as ScannerProductMatch).lots[0]?.batch_code ?? $LL.scanner.registration.scannedValuePlaceholder()}
                {/if}
              </code>
            </p>
            <p class="resolved-line">
              <strong>{$LL.scanner.registration.skuLabel()}:</strong>
              {activeProduct.sku}
            </p>

            {#if registrationLotFormOpen}
              <LotForm
                mode="create"
                productId={activeProduct.id}
                defaultUnit={activeProduct.default_unit ?? ""}
                defaultAlertDays={activeProduct.default_alert_days_before}
                productUnitKind={activeProduct.unit_type ?? "decimal"}
                onSaved={onRegistrationLotSaved}
                onCancel={closeRegistrationLotForm}
              />
            {:else}
              <div class="confirm-row">
                <Button
                  variant="primary"
                  onclick={openRegistrationLotForm}
                >
                  {$LL.scanner.registration.newLotHeading()}
                </Button>
              </div>
            {/if}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Stock-out panel -->
    <div
      role="tabpanel"
      id="mode-stock-out-panel"
      aria-labelledby="mode-stock-out-tab"
      hidden={mode !== "stock_out"}
      tabindex={mode === "stock_out" ? 0 : -1}
    >
      {#if mode === "stock_out"}
        {#if resolved?.match_type === "unknown"}
          <Alert variant="warning">
            <strong>{$LL.scanner.unknownTitle()}:</strong>
            {resolved.scanned_value}
          </Alert>
        {:else if resolved && activeProduct && resolvedLot}
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.stockOut.selectedProduct()}:</strong>
              {activeProduct.sku}
            </p>
            {#if resolved.match_type === "product_match" && resolved.lots.length > 1}
              <Select
                value={selectedLotId}
                options={lotPickerOptions}
                aria-label={$LL.scanner.stockOut.lotPickerLabel()}
                disabled={lotPickerDisabled}
                onchange={(v: string) => (selectedLotId = v)}
              />
              {#if fefoPolicy === "require_fefo"}
                <p class="hint-required">{$LL.scanner.stockOut.fefoRequiredNotice()}</p>
              {/if}
            {:else}
              <p class="resolved-line">
                <strong>{$LL.scanner.stockOut.selectedLot()}:</strong>
                {resolvedLot.lot.expiry_date}
                {#if resolvedLot.lot.batch_code}
                  · {resolvedLot.lot.batch_code}
                {/if}
              </p>
            {/if}

            <Select
              value={exitReason}
              options={[
                { value: "", label: $LL.scanner.stockOut.reasonPlaceholder(), disabled: true },
                ...stockOutReasonOptions,
              ]}
              aria-label={$LL.scanner.stockOut.reasonLabel()}
              required
              onchange={(v: string) => (exitReason = v)}
            />

            {#if loadingBalances}
              <LoadingState variant="text" label={$LL.common.loading()} />
            {:else if availableBalances.length === 0}
              <Alert variant="warning">{$LL.scanner.stockOut.invalid.quantity()}</Alert>
            {:else}
              <Select
                value={locationId}
                options={locationOptions}
                aria-label={$LL.scanner.stockOut.locationLabel()}
                disabled={availableBalances.length === 1}
                onchange={(v: string) => (locationId = v)}
              />
            {/if}

            <fieldset class="fieldset">
              <legend class="fieldset-legend">
                {$LL.scanner.stockOut.quantityLabel()}
              </legend>
              <input
                type="number"
                class="input input-md motion-reduce:transition-none w-full"
                min="0"
                step="0.01"
                inputmode="decimal"
                value={quantityStr}
                oninput={(e: Event) => {
                  const target = e.currentTarget as HTMLInputElement;
                  quantityStr = target.value;
                }}
              />
              {#if selectedBalance > 0}
                <p class="label">
                  {$LL.lotMovements.available({ available: selectedBalance })}
                </p>
              {/if}
            </fieldset>

            <fieldset class="fieldset">
              <legend class="fieldset-legend">
                {requiresNotes
                  ? $LL.scanner.stockOut.notesLabel()
                  : $LL.scanner.stockOut.notesOptionalLabel()}
              </legend>
              <textarea
                class="textarea textarea-md w-full motion-reduce:transition-none"
                rows="3"
                bind:value={notes}
                placeholder={requiresNotes
                  ? $LL.scanner.stockOut.notesRequiredHint()
                  : ""}
              ></textarea>
              {#if requiresNotes && !notes.trim()}
                <p class="hint-required">{$LL.scanner.stockOut.notesRequiredHint()}</p>
              {/if}
            </fieldset>

            <div class="confirm-row">
              <Button
                variant="primary"
                disabled={!canConfirmStockOut || submitting}
                loading={submitting}
                onclick={() => void confirmStockOut()}
              >
                {submitting
                  ? $LL.scanner.stockOut.confirming()
                  : $LL.scanner.stockOut.confirm()}
              </Button>
            </div>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .scanner-page {
    max-width: 760px;
    margin: 0 auto;
    padding: 24px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .page-header {
    margin-bottom: 4px;
  }

  .page-title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-base-content);
  }

  .page-subtitle {
    margin: 4px 0 0;
    font-size: 0.9rem;
    color: var(--color-secondary);
  }

  .scanner-input-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
  }

  .scanner-input-row :global(.fieldset) {
    flex: 1 1 280px;
  }

  /* The hidden-anchor input forwards Enter keypresses from the visible
     Input.svelte so handheld scanners that emit raw Enter close to the
     primary control still trigger the scan. The element is visually
     hidden but stays in the DOM. */
  .hidden-anchor {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .resolved-summary {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 8px;
    background: color-mix(in oklch, var(--color-base-200) 50%, transparent);
  }

  .resolved-line {
    margin: 0;
    font-size: 0.92rem;
    color: var(--color-base-content);
  }

  .scanned-code {
    font-family: 'Courier New', Courier, monospace;
    font-size: 0.9rem;
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .section-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-base-content);
  }

  .section-body {
    margin: 0;
    font-size: 0.88rem;
    color: var(--color-secondary);
    line-height: 1.45;
  }

  .hint-required {
    margin: -6px 0 0;
    font-size: 0.78rem;
    color: var(--color-error);
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .route-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    padding: 8px 0;
  }

  .route-option {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.92rem;
    color: var(--color-base-content);
    cursor: pointer;
  }

  .confirm-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .registration-unknown,
  .registration-existing {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
