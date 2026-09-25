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

  When stores exist but the persisted `last_selected_store_id` is
  empty or stale, the page renders an inline store picker that
  calls `updateSettings` and refreshes settings without leaving the
  Scanner tab. The backend `resolve_scanner_code` command requires
  `last_selected_store_id` to point at an active store, so this
  branch is the only way to keep scanning in that state.
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
    updateSettings,
    listStores,
    listStoreLocations,
    type FefoPolicy,
    type SettingsResponse,
    type StoreResponse,
  } from "../lib/stores.js";
  import {
    loadAllActiveStoreLocations,
    type EnrichedLocation,
  } from "../lib/locations.js";
  import {
    NOTE_REQUIRED_EXITS,
    getExitKindLabel,
    EXIT_KINDS,
    type ExitKind,
  } from "../lib/movementRules.js";
  import {
    createLotMovement,
    getLotLocationBalances,
    type LotLocationBalance,
    type MovementKind,
  } from "../lib/lot_movements.js";
  import { getProduct, listCategories, type CategoryResponse, type ProductResponse } from "../lib/products.js";
  import { getExpiryLot, type ExpiryLotResponse } from "../lib/expiry_lots.js";
  import { scannerNavigation, clearScannerNavigation } from "../lib/navigation.js";
  import LotContextPanel from "./scanner/LotContextPanel.svelte";
  import type { UnitKind } from "../lib/products.js";
  import { resolveLocationDisplay, type LocationRef } from "../lib/lotDisplay.js";
  import Button from "./ui/Button.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Input from "./ui/Input.svelte";
  import Alert from "./ui/Alert.svelte";
  import LoadingState from "./ui/LoadingState.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import LotForm from "./LotForm.svelte";
  import ProductForm from "./ProductForm.svelte";

  // ── Mode union ────────────────────────────────────────────────────────────
  type Mode = "sale" | "registration" | "stock_out";

  // Non-sale exit reasons for Stock-out mode. `exit:sale` is intentionally
  // absent — Stock-out cannot record a sale (per spec scenario).
  // Derived from the shared EXIT_KINDS; `getExitKindLabel` provides labels.
  const STOCK_OUT_KINDS: readonly ExitKind[] = EXIT_KINDS.filter(
    (k) => k !== "exit:sale",
  );

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

  // Stores available to pick as the active store. Loaded only when
  // `hasStoreValue === true` AND the persisted `last_selected_store_id`
  // is empty / stale, so the Scanner lets the user pick without
  // leaving the tab (regression: prior behaviour surfaced a backend
  // "No active store selected" error on the first scan).
  let availableStores = $state<StoreResponse[]>([]);
  let loadingStores = $state(false);
  let storeSelectId = $state("");
  let savingStoreSelection = $state(false);
  let storeSelectError = $state("");

  // Per-store location name lookup for Sale/Stock-out balance labels
  // (per `odd/tasks/lot-location-name.md`). `LotLocationBalance` does
  // not yet project `location_name` on the Rust side, so we hydrate
  // `id → name` from `listStoreLocations` whenever the active store
  // changes and feed it to `resolveLocationDisplay`. Sentinel balances
  // and missing ids still render the localized "No location" label
  // through the helper; `value: location_id` is preserved.
  let locationNameById = $state<Record<string, string>>({});

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
  // For Unknown scans: mount the canonical ProductForm (PR
  // `scanner-form-reuse`). The form owns SKU/description/notes/barcode
  // editing; the Scanner only tracks whether the form is open and what
  // scanned value seeded its UPC field via `prefillUpc`.
  let quickCreateOpen = $state(false);
  let quickCreateScannedValue = $state("");
  // Category list fed into ProductForm's CategoryPicker. Fetched once
  // per Unknown scan; mutated by inline category creates so the
  // picker's chip set stays in sync within the same mount.
  let quickCreateCategories = $state<CategoryResponse[]>([]);

  // LotMatch/ProductMatch registration flow: open the existing LotForm.
  let registrationLotFormOpen = $state(false);

  // All active store locations (all stores). Hydrates the LotContextPanel
  // `allLocations` prop so MoveStockModal cross-store destinations are
  // available without a separate fetch. Loaded once on mount.
  let allStoreLocations = $state<EnrichedLocation[]>([]);

  // Pinned lot-context from a resolved sale/stock-out lot. Cleared on
  // active-store change, on resolving an unknown item, or when the user
  // explicitly closes the panel. Survives successful mutations so the user
  // can read back the updated balances after the action.
  type PinnedContext = {
    lot: ExpiryLotResponse;
    product: ProductResponse;
    unitType: UnitKind | null;
  };
  let pinnedContext = $state<PinnedContext | null>(null);

  // Always-visible active-store context. Mirrors the persisted
  // `last_selected_store_id` so the user can see which store Scanner
  // operations apply to and switch it without leaving the tab. PR
  // `scanner-default-store-lock`.
  let activeStoreChangeBusy = $state(false);
  let activeStoreChangeError = $state("");

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
    STOCK_OUT_KINDS.map((r) => ({
      value: r,
      label: getExitKindLabel(r, $LL),
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

  // Lookup array derived from `locationNameById`. Materialised lazily so
  // `resolveLocationDisplay` can match real balances against the in-memory
  // names; rebuilt only when the active store's location list changes.
  let locationLookup = $derived.by((): LocationRef[] => {
    const refs: LocationRef[] = [];
    for (const id of Object.keys(locationNameById)) {
      const name = locationNameById[id];
      if (typeof name === "string") refs.push({ id, name });
    }
    return refs;
  });

  // Location picker options for Sale and Stock-out. Sentinel balances
  // (loc-sentinel-*) render the localized "No location" label but stay
  // selectable so stock at the sentinel can still be sold / stocked out.
  // Real balances now resolve to the store location name (loaded via
  // `listStoreLocations` reactive effect) instead of leaking the raw
  // UUID; `value: location_id` is preserved for the createLotMovement
  // payload.
  let locationOptions = $derived([
    { value: "", label: $LL.scanner.stockOut.reasonPlaceholder() },
    ...availableBalances.map((b) => ({
      value: b.location_id,
      label: `${resolveLocationDisplay(b.location_id, locationLookup, $LL.common.noLocation())} (${b.balance})`,
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
    if (NOTE_REQUIRED_EXITS.includes(exitReason as ExitKind) && !notes.trim()) return false;
    return true;
  });

  let requiresNotes = $derived(
    NOTE_REQUIRED_EXITS.includes(exitReason as ExitKind),
  );

  // Scanner-tab availability: the page is interactive only when settings
  // loaded, an active store exists, and the persisted
  // `last_selected_store_id` still points at one of the available
  // stores. A missing / stale id surfaces the store picker instead of
  // letting scans hit the backend "No active store selected" guard.
  let activeStoreSelected = $derived.by((): boolean => {
    const id = settings?.last_selected_store_id;
    if (!id) return false;
    return availableStores.some((s) => s.id === id);
  });

  let pageReady = $derived(
    !loadingSettings &&
      !settingsError &&
      hasStoreValue === true &&
      activeStoreSelected,
  );

  // Active stores, formatted for the picker. Inactive stores are
  // filtered out because the backend `has_store` guard already only
  // counts active ones, and persisting an inactive id would re-trigger
  // the same "No active store" error on the next scan.
  let pickerStoreOptions = $derived.by((): Array<{ value: string; label: string }> => {
    return availableStores
      .filter((s) => s.is_active)
      .map((s) => ({
        value: s.id,
        label: s.code ? `${s.name} · ${s.code}` : s.name,
      }));
  });

  // Resolved active store (SettingsResponse.last_selected_store_id).
  // Used by the always-visible context panel and by the LotForm lock.
  // Falls back to `null` while settings are loading so consumers can
  // guard against a stale derived.
  let activeStore: StoreResponse | null = $derived.by((): StoreResponse | null => {
    const id = settings?.last_selected_store_id;
    if (!id) return null;
    return availableStores.find((s) => s.id === id) ?? null;
  });

  // Mirror of the active store id for the always-visible Select so the
  // dropdown stays in sync with `settings.last_selected_store_id` on
  // mount, refresh, and post-switch.
  let activeStoreSelectId = $derived(settings?.last_selected_store_id ?? "");

  // ── Lot context ───────────────────────────────────────────────────────────

  // ── Dashboard → Scanner navigation (ODD task 7) ─────────────────────────
  // Consumes in-memory `scannerNavigation` requests written by Dashboard.
  // Resolves the lot + product, populates `pinnedContext` (the canonical
  // LotContextPanel state), then clears the request so it is not re-served
  // on future visits to the Scanner tab. Preserves existing active-store
  // clearing behaviour (the active-store-change effect handles that separately).
  $effect(() => {
    const unsub = scannerNavigation.subscribe(async (req) => {
      if (req === null) return;
      // Must wait for the Scanner to be ready before populating state.
      // The page-ready guard is a derived; use untrack to avoid false
      // reactive dependencies on every intermediate settings change.
      if (!pageReady) return;

      if (req.lotId) {
        // Lot context: fetch lot + product and pin for LotContextPanel.
        try {
          const [lot, productDetail] = await Promise.all([
            getExpiryLot(req.lotId),
            getProduct(req.productId),
          ]);
          pinnedContext = {
            lot,
            product: productDetail.product,
            unitType: (productDetail.product.unit_type ?? null) as UnitKind | null,
          };
        } catch {
          // Resolution failures are surfaced silently; the pinned context
          // remains null and the user stays on Scanner to retry.
        }
      } else {
        // Product-only or unknown scan: open Scanner's canonical
        // product-creation flow (quick-create) directly.
        // The scanned value seeds the UPC field so the user does not
        // need to re-type it.
        untrack(() => {
          quickCreateOpen = true;
          quickCreateScannedValue = req.scannedValue ?? "";
        });
      }
      clearScannerNavigation();
    });
    return unsub;
  });

  /**
   * Loads locations from every active store so MoveStockModal can offer
   * cross-store transfer destinations without a separate round-trip.
   * Delegates to the shared `loadAllActiveStoreLocations` loader from
   * `locations.ts`. Called once on mount; the list refreshes only if
   * the user navigates away and back (onMount re-runs).
   */
  async function loadAllStoreLocations(): Promise<void> {
    try {
      const result = await loadAllActiveStoreLocations();
      allStoreLocations = result.allLocations;
    } catch {
      allStoreLocations = [];
    }
  }

  /**
   * Clears the pinned lot context. Called from the LotContextPanel close
   * button and from the active-store-change effect.
   */
  function clearPinnedContext(): void {
    pinnedContext = null;
  }

  /**
   * Pins the current resolved lot for lot-context inspection. The panel
   * stays open after successful actions so the user can read back the
   * updated balances; `resetMutationForm` drops the scan state but does
   * not close the panel (task 5 contract).
   */
  function openLotContext(): void {
    if (!resolvedLot || !activeProduct) return;
    pinnedContext = {
      lot: resolvedLot.lot,
      product: activeProduct,
      unitType: (activeProduct.unit_type ?? null) as UnitKind | null,
    };
  }

  // Clear pinned context when the active store changes — a different store
  // means the lot context is no longer meaningful.
  $effect(() => {
    const storeId = settings?.last_selected_store_id;
    if (!storeId) return;
    // untrack so this effect does not re-run on every state change inside
    // the reactive block; only when the store id actually changes.
    untrack(() => {
      if (pinnedContext !== null) {
        clearPinnedContext();
      }
    });
  });

  // ── Init ──────────────────────────────────────────────────────────────────

  onMount(async () => {
    await Promise.all([
      loadSettings(),
      loadHasStore(),
      loadAvailableStores(),
      loadAllStoreLocations(),
    ]);
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

  async function loadAvailableStores(): Promise<void> {
    loadingStores = true;
    try {
      availableStores = await listStores();
      const current = settings?.last_selected_store_id ?? "";
      if (current && availableStores.some((s) => s.id === current)) {
        storeSelectId = current;
      } else {
        storeSelectId = "";
      }
    } catch {
      availableStores = [];
      storeSelectId = "";
    } finally {
      loadingStores = false;
    }
  }

  // Persist the user's pick as `last_selected_store_id`. The Scanner
  // becomes interactive (pageReady flips true) once `settings` is
  // refreshed with the new value.
  async function applyActiveStore(): Promise<void> {
    const next = storeSelectId;
    if (!next) return;
    savingStoreSelection = true;
    storeSelectError = "";
    scanError = "";
    try {
      const updated = await updateSettings({ last_selected_store_id: next });
      settings = updated;
      successNotice = $LL.scanner.selectActiveStore.success({
        name:
          availableStores.find((s) => s.id === next)?.name ?? next,
      });
    } catch (e) {
      storeSelectError = $LL.scanner.selectActiveStore.errors.failed({
        msg: humanizeError(e),
      });
    } finally {
      savingStoreSelection = false;
    }
  }

  // Persist a new `last_selected_store_id` from the always-visible
  // active-store context (different from `applyActiveStore` because
  // the context lives inside the interactive Scanner surface: any
  // in-progress scan / lot must be cleared so the user does not see a
  // resolved value from the previous store context). PR
  // `scanner-default-store-lock`.
  async function changeActiveStore(next: string): Promise<void> {
    const previousId = settings?.last_selected_store_id ?? "";
    if (!next || next === previousId) return;
    activeStoreChangeBusy = true;
    activeStoreChangeError = "";
    scanError = "";
    try {
      const updated = await updateSettings({ last_selected_store_id: next });
      settings = updated;
      // The previously resolved scan no longer matches the active
      // store's context — drop it so the next scan starts fresh.
      resetMutationForm();
      successNotice = $LL.scanner.activeStoreContext.changeNotice({
        name:
          availableStores.find((s) => s.id === next)?.name ?? next,
      });
    } catch (e) {
      activeStoreChangeError = $LL.scanner.activeStoreContext.errors.failed({
        msg: humanizeError(e),
      });
    } finally {
      activeStoreChangeBusy = false;
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

  // Refresh the per-store location lookup whenever the active store
  // changes (including the post-switch from `changeActiveStore`).
  // Failures fall back to an empty map; sentinel balances and the
  // missing-id path still render correctly through `resolveLocationDisplay`,
  // and the option `value` is never altered — only the display label.
  $effect(() => {
    const storeId = settings?.last_selected_store_id;
    if (!storeId) {
      locationNameById = {};
      return;
    }
    untrack(async () => {
      try {
        const list = await listStoreLocations(storeId);
        const next: Record<string, string> = {};
        for (const l of list) next[l.id] = l.name;
        locationNameById = next;
      } catch {
        locationNameById = {};
      }
    });
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
      // Unknown — clear the lot/location for the previous resolve and
      // open the canonical ProductForm for quick-create. The form is
      // keyed by the scanned value so each new Unknown scan remounts a
      // fresh form that picks up the new UPC prefill. Category list
      // is fetched lazily (one round-trip per Unknown resolution).
      selectedLotId = "";
      locationId = "";
      quickCreateOpen = true;
      quickCreateScannedValue = result.scanned_value;
      void loadQuickCreateCategories();
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
      resetMutationFormWithoutContext();
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
        reason: getExitKindLabel(exitReason as ExitKind, $LL),
      });
      resetMutationFormWithoutContext();
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
    if (NOTE_REQUIRED_EXITS.includes(exitReason as ExitKind) && !notes.trim()) {
      return $LL.scanner.stockOut.invalid.notes();
    }
    return $LL.scanner.stockOut.invalid.lot();
  }

  function resetMutationFormWithoutContext(): void {
    // Clears per-confirm state without touching the pinned lot context.
    // Used after sale/stock-out confirmation so the panel stays open for
    // immediate readback.
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

  function resetMutationForm(): void {
    // Per spec `Sale success resets the form for the next scan` and
    // `Stock-out success resets the form`. We clear the full scan state
    // (resolved value, lot selection, quantity, location, reason, notes,
    // and balance cache) so the next scan starts completely fresh. The
    // debounce guard is also reset so an identical scan triggers a new
    // resolution. The pinned lot context is cleared separately — see
    // `clearPinnedContext` and the active-store-change effect.
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

  // Reuses the canonical ProductForm (PR `scanner-form-reuse`).
  // The Scanner no longer maintains SKU/barcode/description/alert-days
  // state; ProductForm owns that surface plus barcode attach via its
  // own `prefillUpc` + `addProductBarcodeOnCreate` flow.

  /**
   * Fetches the master category list for the ProductForm's
   * CategoryPicker. Triggered lazily on each Unknown resolve so the
   * Scanner only pays the round-trip when it actually needs to show
   * the form. Errors fall back to an empty list — the CategoryPicker
   * already handles that case.
   */
  async function loadQuickCreateCategories(): Promise<void> {
    try {
      quickCreateCategories = await listCategories();
    } catch {
      quickCreateCategories = [];
    }
  }

  /**
   * ProductForm callback after a successful save. Mirrors the
   * success-reset behaviour of the previous inline flow: show the
   * success notice, drop the resolved scan so the next scan starts
   * fresh, and clear the debounce guard.
   */
  function onQuickCreateSaved(product: ProductResponse): void {
    successNotice = $LL.scanner.registration.successProduct({ sku: product.sku });
    quickCreateOpen = false;
    resolved = null;
    selectedLotId = "";
    quantityStr = "1";
    locationId = "";
    lotBalances = [];
    lastResolvedAt = 0;
    lastResolvedValue = "";
  }

  /**
   * ProductForm cancel callback. Closes the form and drops the
   * resolved scan so the user can scan a different value or switch
   * modes without seeing a stale Unknown result.
   */
  function closeQuickCreate(): void {
    quickCreateOpen = false;
    resolved = null;
    lastResolvedAt = 0;
    lastResolvedValue = "";
  }

  /**
   * ProductForm callback for inline category creates from inside the
   * CategoryPicker. Merges the new category into our local list so
   * the picker's chip set stays in sync within the same mount.
   */
  function onQuickCreateCategoryCreated(category: CategoryResponse): void {
    if (!quickCreateCategories.find((c) => c.id === category.id)) {
      quickCreateCategories = [...quickCreateCategories, category];
    }
  }
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
  {:else if hasStoreValue === true && !activeStoreSelected}
    <!-- Stores exist but the persisted `last_selected_store_id` is
         empty or stale. Render a picker so the user can pick an
         active store without leaving the Scanner tab. The backend
         scanner command requires `last_selected_store_id` to point
         at an active store, so this branch is the only way to keep
         scanning in this state. -->
    <section
      class="store-picker"
      aria-labelledby="store-picker-title"
    >
      <h2 id="store-picker-title" class="picker-title">
        {$LL.scanner.selectActiveStore.title()}
      </h2>
      <p class="picker-body">{$LL.scanner.selectActiveStore.body()}</p>

      {#if loadingStores}
        <LoadingState variant="text" label={$LL.common.loading()} />
      {:else if pickerStoreOptions.length === 0}
        <Alert variant="warning">{$LL.scanner.noStore.body()}</Alert>
      {:else}
        <Listbox
          value={storeSelectId}
          options={[
            { value: "", label: $LL.lotForm.selectStorePlaceholder(), disabled: true },
            ...pickerStoreOptions,
          ]}
          aria-label={$LL.scanner.selectActiveStore.label()}
          onchange={(v: string) => (storeSelectId = v)}
        />

        {#if storeSelectError}
          <Alert variant="error">{storeSelectError}</Alert>
        {/if}

        <div class="picker-actions">
          <Button
            variant="primary"
            disabled={!storeSelectId || savingStoreSelection}
            loading={savingStoreSelection}
            onclick={() => void applyActiveStore()}
          >
            {$LL.scanner.selectActiveStore.label()}
          </Button>
        </div>
      {/if}
    </section>
  {:else if pageReady}
    <!-- Always-visible active-store context. Renders inside the
         interactive Scanner surface so the user can see which store
         Scanner operations apply to and switch it without leaving
         the tab. Lot creations opened from the Scanner are locked to
         this store via `LotForm.lockedStoreId`. PR
         `scanner-default-store-lock`. -->
    <section
      class="store-picker active-store-context"
      aria-labelledby="active-store-context-title"
    >
      <div class="active-store-context-header">
        <h2 id="active-store-context-title" class="picker-title">
          {$LL.scanner.activeStoreContext.title()}
        </h2>
        <span
          class="locked-badge"
          aria-label={$LL.scanner.activeStoreContext.lockedBadge()}
          title={$LL.scanner.activeStoreContext.lockedBadge()}
        >
          {$LL.scanner.activeStoreContext.lockedBadge()}
        </span>
      </div>
      <p class="picker-body">{$LL.scanner.activeStoreContext.body()}</p>

      {#if loadingStores}
        <LoadingState variant="text" label={$LL.common.loading()} />
      {:else if pickerStoreOptions.length > 1}
        <!-- Multiple active stores: show the Select so the user can
             switch the Scanner context in place. We do NOT navigate
             away — the page just re-resolves the next scan against
             the new active store, dropping any in-progress scan. -->
        <Listbox
          value={activeStoreSelectId}
          options={pickerStoreOptions}
          aria-label={$LL.scanner.activeStoreContext.label()}
          disabled={activeStoreChangeBusy}
          onchange={(v: string) => void changeActiveStore(v)}
        />
      {:else}
        <!-- Single store: read-only display so the user still sees
             the store the Scanner is operating against. -->
        <p class="active-store-name" data-testid="active-store-name">
          <strong>{activeStore?.name ?? settings?.last_selected_store_id ?? ""}</strong>
        </p>
      {/if}

      {#if activeStoreChangeError}
        <Alert variant="error">{activeStoreChangeError}</Alert>
      {/if}
    </section>

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
        {scanBusy ? $LL.scanner.input.looking() : $LL.scanner.input.lookupLabel()}
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
        {:else if resolved?.match_type === "lot_match" && activeProduct}
          <!-- Direct lot scan: FEFO bypassed, but the user still needs
               to see which product was matched. Render a compact
               resolved-summary so the lot row is not the only
               affordance. -->
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.sale.selectedProduct()}:</strong>
              {activeProduct.description}
              <span class="muted">({activeProduct.sku})</span>
            </p>
            <p class="resolved-line">
              <strong>{$LL.scanner.sale.selectedLot()}:</strong>
              {resolvedLot?.lot.expiry_date ?? "—"}
              {#if resolvedLot?.lot.batch_code}
                · {resolvedLot.lot.batch_code}
              {/if}
            </p>
            <p class="hint-required">{$LL.scanner.sale.selectedLotHint()}</p>
          </div>
        {:else if resolved?.match_type === "product_match" && activeProduct && resolved.lots.length === 0}
          <!-- Product resolved but no active lots in the active store.
               Sale mode has no action to take; surface the resolved
               product so the user knows what was scanned and explain why
               nothing else is interactive. -->
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.sale.selectedProduct()}:</strong>
              {activeProduct.description}
              <span class="muted">({activeProduct.sku})</span>
            </p>
            <Alert variant="info">{$LL.scanner.noLotsAvailable()}</Alert>
          </div>
        {:else if resolved && activeProduct && resolvedLot}
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.sale.selectedProduct()}:</strong>
              {activeProduct.description}
              <span class="muted">({activeProduct.sku})</span>
            </p>
            {#if resolved.match_type === "product_match" && resolved.lots.length > 1}
              <Listbox
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
              <Listbox
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
                max={selectedBalance}
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
              <Button
                variant="secondary"
                onclick={openLotContext}
              >
                {$LL.scanner.lotContext.openButton()}
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

            {#if quickCreateOpen}
              <!-- Reuses the canonical ProductForm (PR
                   `scanner-form-reuse`). Keyed by the scanned value so
                   each new Unknown resolution remounts a fresh form
                   that seeds the barcode field via `prefillUpc`.
                   ProductForm owns the create + barcode-attach flow
                   internally and surfaces its own errors. -->
              {#key quickCreateScannedValue}
                <ProductForm
                  mode="create"
                  initial={null}
                  categories={quickCreateCategories}
                  prefillUpc={quickCreateScannedValue}
                  onSaved={onQuickCreateSaved}
                  onCancel={closeQuickCreate}
                  onCategoryCreated={onQuickCreateCategoryCreated}
                />
              {/key}
            {/if}
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
            {#if registrationLotFormOpen}
              <LotForm
                mode="create"
                productId={activeProduct.id}
                defaultUnit={activeProduct.default_unit ?? ""}
                defaultAlertDays={activeProduct.default_alert_days_before}
                productUnitKind={activeProduct.unit_type ?? "decimal"}
                lockedStoreId={settings?.last_selected_store_id ?? null}
                lockedStoreName={activeStore?.name ?? ""}
                productDescription={activeProduct.description}
                productSku={activeProduct.sku}
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
        {:else if resolved?.match_type === "lot_match" && activeProduct}
          <!-- Direct lot scan: render the product + lot so the user
               has the same context the multi-lot picker would have
               given them. Stock-out is also FEFO-bypassed for
               direct lot scans, so the lot picker is not shown. -->
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.stockOut.selectedProduct()}:</strong>
              {activeProduct.description}
              <span class="muted">({activeProduct.sku})</span>
            </p>
            <p class="resolved-line">
              <strong>{$LL.scanner.stockOut.selectedLot()}:</strong>
              {resolvedLot?.lot.expiry_date ?? "—"}
              {#if resolvedLot?.lot.batch_code}
                · {resolvedLot.lot.batch_code}
              {/if}
            </p>
          </div>
        {:else if resolved?.match_type === "product_match" && activeProduct && resolved.lots.length === 0}
          <!-- Product resolved but no active lots in the active store.
               Surface what was scanned and explain why the rest of
               the form is not actionable. -->
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.stockOut.selectedProduct()}:</strong>
              {activeProduct.description}
              <span class="muted">({activeProduct.sku})</span>
            </p>
            <Alert variant="info">{$LL.scanner.noLotsAvailable()}</Alert>
          </div>
        {:else if resolved && activeProduct && resolvedLot}
          <div class="resolved-summary">
            <p class="resolved-line">
              <strong>{$LL.scanner.stockOut.selectedProduct()}:</strong>
              {activeProduct.description}
              <span class="muted">({activeProduct.sku})</span>
            </p>
            {#if resolved.match_type === "product_match" && resolved.lots.length > 1}
              <Listbox
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

            <Listbox
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
              <Listbox
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
                max={selectedBalance}
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
              <Button
                variant="secondary"
                onclick={openLotContext}
              >
                {$LL.scanner.lotContext.openButton()}
              </Button>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Lot-context panel. Rendered below the mode panels so the user
         retains access to the scanner input and mode tabs while the
         panel is open. Survives successful sale/stock-out confirmation
         so the updated balances are immediately visible. -->
    {#if pinnedContext}
      <LotContextPanel
        lot={pinnedContext.lot}
        unitType={pinnedContext.unitType}
        allLocations={allStoreLocations}
        onClose={clearPinnedContext}
      />
    {/if}
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

  /* Secondary label inside a resolved line — used for the SKU
     suffix on `description (sku)` rows so the SKU stays glanceable
     without competing with the description text. */
  .muted {
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    font-size: 0.88rem;
    margin-left: 4px;
  }

  .hint-required {
    margin: -6px 0 0;
    font-size: 0.78rem;
    color: var(--color-error);
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

  /* ── Store picker (active store selection) ─────────────────────── */
  .store-picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 8px;
    background: color-mix(in oklch, var(--color-base-200) 50%, transparent);
  }

  .picker-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-base-content);
  }

  .picker-body {
    margin: 0;
    font-size: 0.88rem;
    color: var(--color-secondary);
    line-height: 1.45;
  }

  .picker-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* ── Active-store context (always-visible Scanner surface) ─────── */

  /* The active-store context panel reuses the store-picker block to
     keep the visual contract aligned with the missing/stale picker:
     same border, padding, and body copy styling. Only the header
     grows a "Locked for new lots" badge and the actions row is
     replaced with an inline name or a Select. */
  .active-store-context {
    margin-bottom: 4px;
  }

  .active-store-context-header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .locked-badge {
    display: inline-flex;
    align-items: center;
    font-size: 0.74rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-warning);
    background: color-mix(in oklch, var(--color-warning) 12%, transparent);
    border: 1px solid color-mix(in oklch, var(--color-warning) 35%, transparent);
    border-radius: 999px;
    padding: 2px 8px;
  }

  .active-store-name {
    margin: 0;
    font-size: 0.95rem;
    color: var(--color-base-content);
  }
</style>
