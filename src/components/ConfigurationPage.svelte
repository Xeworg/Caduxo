<!--
  ConfigurationPage.svelte — settings surface (PR 8a finish of
  caduxo-daisyui-redesign).

  PR 5 already migrated the language selector (Listbox — themed
  popover, not a wrapped native `<select>`), the location-required
  toggle (Toggle), and the theme switcher section (Listbox + Alert).
  PR 8a finishes the page chrome:
    - Each settings section now lives inside a Card.svelte primitive
      (`tone="default"`).
    - The `.loading-msg` plain-text placeholder is replaced with
      `LoadingState.svelte` (text variant).
    - The `.saving-msg` is replaced with `LoadingState.svelte`
      (spinner variant) so the in-flight save reads as a status.
    - The `.error-msg` plain-text surface is replaced with
      `Alert.svelte variant="error"` for both the load-error path
      and the locale-save-error path.
    - The `.theme-error` wrapper becomes a plain block (the
      Alert.svelte inside already owns the error chrome).
    - The page-header + page-title remain as plain elements; the
      page itself is the Configuration route, not a DaisyUI surface.

  No business logic changes — only the form chrome.

  Tailwind classes referenced here (for the JIT scanner):
    toggle toggle-primary toggle-md
    alert alert-error alert-info alert-soft
    menu menu-sm rounded-box
    card card-body card-title bg-base-100 border-base-300
    shadow-sm flex items-start justify-between gap-5
    loading loading-spinner loading-md
    skeleton
-->
<script lang="ts">
    import { onMount } from "svelte";
    import {
        getSettings,
        updateSettings,
        type CloseBehavior,
        type FefoPolicy,
        type SettingsResponse,
    } from "../lib/stores.js";
    import {
        locale,
        translationSource,
        setLocale,
        AVAILABLE_LOCALES,
        type SupportedLocale,
    } from "../i18n/locale.svelte.js";
    import { LL } from "../i18n/i18n-svelte.js";
    import { humanizeError } from "../lib/errors.js";
    import Listbox from "./ui/Listbox.svelte";
    import Toggle from "./ui/Toggle.svelte";
    import Alert from "./ui/Alert.svelte";
    import Card from "./ui/Card.svelte";
    import LoadingState from "./ui/LoadingState.svelte";
    import Modal from "./ui/Modal.svelte";
    import {
        AVAILABLE_THEMES,
        setTheme,
        theme,
        type ThemeName,
    } from "./ui/theme/themeStore.svelte.js";
    import Button from "./ui/Button.svelte";
    import {
        listUnitDefinitions,
        listArchivedUnitDefinitions,
        createUnitDefinition,
        renameUnitDefinition,
        archiveUnitDefinition,
        unarchiveUnitDefinition,
        type UnitDefinitionResponse,
    } from "../lib/unit_definitions.js";
    import type { UnitKind } from "../lib/products.js";
    import {
        listAllStockOutReasons,
        createStockOutReason,
        renameStockOutReason,
        archiveStockOutReason,
        unarchiveStockOutReason,
        type StockOutReason,
        type ExitReasonMovementKind,
    } from "../lib/stock_out_reasons.js";

    // Per-locale display names. The dictionary keys live in
    // `configuration.language.names` keyed by `SupportedLocale` code, so
    // adding a new locale to `AVAILABLE_LOCALES` only requires a matching
    // entry in each locale dictionary — no component branching.
    function languageLabel(code: SupportedLocale): string {
        const names = $LL.configuration.language.names as unknown as Record<
            SupportedLocale,
            () => string
        >;
        return names[code]?.() ?? code;
    }

    // Per-theme display names. The dictionary keys live under the
    // top-level `theme` namespace (e.g. `theme.caduxoLight`,
    // `theme.dark`, `theme.dracula`, `theme.valentine`, `theme.luxury`,
    // `theme.sunset`, `theme.nord`) so future cross-page theme
    // affordances can share the same copy. The switch exhausts every
    // variant of `ThemeName` so a future theme addition triggers a TS
    // error here until the matching i18n key is added.
    function themeLabel(name: ThemeName): string {
        switch (name) {
            case "caduxo-light":
                return $LL.theme.caduxoLight();
            case "dark":
                return $LL.theme.dark();
            case "dracula":
                return $LL.theme.dracula();
            case "valentine":
                return $LL.theme.valentine();
            case "luxury":
                return $LL.theme.luxury();
            case "sunset":
                return $LL.theme.sunset();
            case "nord":
                return $LL.theme.nord();
        }
    }

    // The active source of the active theme, rendered next to the
    // current entry in the switcher so the user can see *why* they
    // got the theme they did (per spec scenario "switcher shows the
    // source of the active theme").
    function themeSourceLabel(source: typeof theme.source): string {
        switch (source) {
            case "manual":
                return $LL.theme.source.manual();
            case "persisted":
                return $LL.theme.source.persisted();
            case "os":
                return $LL.theme.source.os();
            case "fallback":
                return $LL.theme.source.fallback();
        }
    }

    // ─── State ───────────────────────────────────────────────────────────────────

    let loading = $state(true);
    let savingLocation = $state(false);
    let settings: SettingsResponse | null = $state(null);
    let errorMsg = $state("");
    let localeErrorMsg = $state("");

    // Local toggle value; updated optimistically on click.
    // Reverted if the save fails.
    let requireLocation: boolean = $state(true);

    // Local locale value for the selector; kept in sync with the rune.
    let currentLocale: SupportedLocale = $state("en");

    // Theme switcher state. `switchingTheme` is set true during the
    // optimistic apply; it clears when the IPC call resolves or
    // rejects. `themeError` carries the human-readable error message
    // surfaced in the inline `Alert.svelte` when `setTheme` rejects.
    let switchingTheme = $state(false);
    let themeError = $state("");

    // Scanner FEFO policy selector (`scanner-quick-operations` PR 3).
    // `currentFefoPolicy` is the optimistic local copy; `savingFefoPolicy`
    // disables the selector while the IPC save is in flight; `fefoPolicyError`
    // carries the translated inline error surfaced on failure.
    const AVAILABLE_FEFO_POLICIES: FefoPolicy[] = [
        "suggest_fefo",
        "require_fefo",
        "manual_lot_choice",
    ];
    let currentFefoPolicy: FefoPolicy = $state("suggest_fefo");
    let savingFefoPolicy = $state(false);
    let fefoPolicyError = $state("");

    // Close-window behaviour selector (`scanner-quick-operations` PR 3).
    // `currentCloseBehavior` mirrors the persisted value; `savingCloseBehavior`
    // disables the selector while the IPC save is in flight; `closeBehaviorError`
    // carries the translated inline error surfaced on failure.
    const AVAILABLE_CLOSE_BEHAVIORS: CloseBehavior[] = [
        "minimize_to_tray",
        "exit_application",
    ];
    let currentCloseBehavior: CloseBehavior = $state("minimize_to_tray");
    let savingCloseBehavior = $state(false);
    let closeBehaviorError = $state("");

    // ─── Unit catalog state ─────────────────────────────────────────────────────

    // ─── Unit catalog helpers ──────────────────────────────────────────────────

    // Kind-first, display-name-second order for the active unit rows.
    // The sort mirrors the `groupedActive` derived grouping so the table
    // renders in a consistent, predictable order without relying on
    // server-return order.
    const KIND_ORDER: Record<UnitKind, number> = { integer: 0, decimal: 1 };

    function insertActiveSorted(
        units: UnitDefinitionResponse[],
        unit: UnitDefinitionResponse,
    ): UnitDefinitionResponse[] {
        const targetKind = KIND_ORDER[unit.kind];
        for (let i = 0; i < units.length; i++) {
            const u = units[i];
            const uKind = KIND_ORDER[u.kind];
            if (targetKind < uKind || (targetKind === uKind && unit.display_name < u.display_name)) {
                const result = [...units];
                result.splice(i, 0, unit);
                return result;
            }
        }
        return [...units, unit];
    }

    let loadingUnits = $state(true);
    let unitsLoadError = $state("");
    let activeUnits = $state<UnitDefinitionResponse[]>([]);
    let archivedUnits = $state<UnitDefinitionResponse[]>([]);
    let showArchived = $state(false);

    // Inline rename state.
    let renamingId: string | null = $state(null);
    let renameValue = $state("");
    let savingRename = $state(false);
    let renameError = $state("");

    // Archive confirmation state.
    let archiveTarget: UnitDefinitionResponse | null = $state(null);
    let confirmingArchive = $state(false);
    let archivingUnit = $state(false);
    let archiveApiError = $state("");

    // Restore confirmation state.
    let restoreTarget: UnitDefinitionResponse | null = $state(null);
    let confirmingRestore = $state(false);
    let restoringUnit = $state(false);
    let restoreApiError = $state("");

    // Create-unit form state.
    let showCreateForm = $state(false);
    let createKey = $state("");
    let createDisplayName = $state("");
    let createKind: UnitKind = $state("integer");
    let creatingUnit = $state(false);
    let createError = $state("");

    // ─── Stock-out reasons catalog state ───────────────────────────────────────

    // The seven closed stock-out movement kinds (sale excluded).
    const AVAILABLE_MOVEMENT_KINDS: ExitReasonMovementKind[] = [
        "exit:waste",
        "exit:expired",
        "exit:damaged",
        "exit:internal_consumption",
        "exit:return_to_supplier",
        "exit:inventory_adjustment",
        "exit:other",
    ];

    let loadingReasons = $state(true);
    let reasonsLoadError = $state("");
    let activeReasons = $state<StockOutReason[]>([]);
    let archivedReasons = $state<StockOutReason[]>([]);
    let showArchivedReasons = $state(false);

    // Inline rename state.
    let renamingReasonId: string | null = $state(null);
    let renamingReasonValue = $state("");
    let savingReasonRename = $state(false);
    let renameReasonError = $state("");

    // Archive confirmation state.
    let archiveReasonTarget: StockOutReason | null = $state(null);
    let confirmingReasonArchive = $state(false);
    let archivingReason = $state(false);
    let archiveReasonApiError = $state("");

    // Restore confirmation state.
    let restoreReasonTarget: StockOutReason | null = $state(null);
    let confirmingReasonRestore = $state(false);
    let restoringReason = $state(false);
    let restoreReasonApiError = $state("");

    // Create-reason form state.
    let showCreateReasonForm = $state(false);
    let createReasonDisplayName = $state("");
    let createReasonMovementKind: ExitReasonMovementKind = $state("exit:waste");
    let creatingReason = $state(false);
    let createReasonError = $state("");

    // ─── Unit catalog load ────────────────────────────────────────────────────

    async function loadUnitCatalog() {
        loadingUnits = true;
        unitsLoadError = "";
        try {
            [activeUnits, archivedUnits] = await Promise.all([
                listUnitDefinitions(),
                listArchivedUnitDefinitions(),
            ]);
        } catch (e) {
            unitsLoadError = $LL.unitCatalog.loadError({ msg: humanizeError(e) });
        } finally {
            loadingUnits = false;
        }
    }

    // ─── Inline rename ────────────────────────────────────────────────────────

    function startRename(unit: UnitDefinitionResponse) {
        renamingId = unit.id;
        renameValue = unit.display_name;
        renameError = "";
    }

    function cancelRename() {
        renamingId = null;
        renameValue = "";
        renameError = "";
    }

    async function submitRename() {
        if (!renamingId || !renameValue.trim()) return;
        savingRename = true;
        renameError = "";
        try {
            const updated = await renameUnitDefinition({
                id: renamingId,
                display_name: renameValue.trim(),
            });
            // Replace and re-sort so the renamed unit lands in the correct position.
            activeUnits = insertActiveSorted(
                activeUnits.filter((u) => u.id !== renamingId),
                updated,
            );
            renamingId = null;
            renameValue = "";
        } catch (e) {
            renameError = $LL.unitCatalog.renameError({ msg: humanizeError(e) });
        } finally {
            savingRename = false;
        }
    }

    // ─── Archive ─────────────────────────────────────────────────────────────

    function requestArchive(unit: UnitDefinitionResponse) {
        archiveTarget = unit;
        confirmingArchive = true;
        archiveApiError = "";
    }

    function cancelArchive() {
        confirmingArchive = false;
        archiveTarget = null;
        archiveApiError = "";
    }

    async function confirmArchive() {
        if (!archiveTarget) return;
        archivingUnit = true;
        archiveApiError = "";
        const id = archiveTarget.id;
        try {
            await archiveUnitDefinition(id);
            // Refetch to get correct archived_at timestamp and preserve order.
            await loadUnitCatalog();
            confirmingArchive = false;
            archiveTarget = null;
        } catch (e) {
            archiveApiError = $LL.unitCatalog.archiveError({ msg: humanizeError(e) });
        } finally {
            archivingUnit = false;
        }
    }

    // ─── Restore ─────────────────────────────────────────────────────────────

    function requestRestore(unit: UnitDefinitionResponse) {
        restoreTarget = unit;
        confirmingRestore = true;
        restoreApiError = "";
    }

    function cancelRestore() {
        confirmingRestore = false;
        restoreTarget = null;
        restoreApiError = "";
    }

    async function confirmRestore() {
        if (!restoreTarget) return;
        restoringUnit = true;
        restoreApiError = "";
        const id = restoreTarget.id;
        try {
            const restored = await unarchiveUnitDefinition(id);
            archivedUnits = archivedUnits.filter((u) => u.id !== id);
            activeUnits = insertActiveSorted(activeUnits, restored);
            confirmingRestore = false;
            restoreTarget = null;
        } catch (e) {
            restoreApiError = $LL.unitCatalog.restoreError({ msg: humanizeError(e) });
        } finally {
            restoringUnit = false;
        }
    }

    // ─── Create unit ─────────────────────────────────────────────────────────

    function openCreateForm() {
        showCreateForm = true;
        createKey = "";
        createDisplayName = "";
        createKind = "integer";
        createError = "";
    }

    function closeCreateForm() {
        showCreateForm = false;
        createError = "";
    }

    async function submitCreateUnit() {
        if (!createKey.trim() || !createDisplayName.trim()) return;
        creatingUnit = true;
        createError = "";
        try {
            const created = await createUnitDefinition({
                key: createKey.trim(),
                display_name: createDisplayName.trim(),
                kind: createKind,
            });
            activeUnits = insertActiveSorted(activeUnits, created);
            closeCreateForm();
        } catch (e) {
            createError = $LL.unitCatalog.createError({ msg: humanizeError(e) });
        } finally {
            creatingUnit = false;
        }
    }

    // ─── Stock-out reasons catalog load ───────────────────────────────────────

    async function loadReasonsCatalog() {
        loadingReasons = true;
        reasonsLoadError = "";
        try {
            const all = await listAllStockOutReasons();
            activeReasons = all.filter((r) => r.archived_at === null);
            archivedReasons = all.filter((r) => r.archived_at !== null);
        } catch (e) {
            reasonsLoadError = $LL.stockOutReasons.loadError({ msg: humanizeError(e) });
        } finally {
            loadingReasons = false;
        }
    }

    // ─── Inline rename reason ────────────────────────────────────────────────

    function startReasonRename(reason: StockOutReason) {
        renamingReasonId = reason.id;
        renamingReasonValue = reason.display_name;
        renameReasonError = "";
    }

    function cancelReasonRename() {
        renamingReasonId = null;
        renamingReasonValue = "";
        renameReasonError = "";
    }

    async function submitReasonRename() {
        if (!renamingReasonId || !renamingReasonValue.trim()) return;
        savingReasonRename = true;
        renameReasonError = "";
        try {
            const updated = await renameStockOutReason({
                id: renamingReasonId,
                display_name: renamingReasonValue.trim(),
            });
            // Replace in the active list with the updated entry.
            activeReasons = activeReasons.map((r) =>
                r.id === renamingReasonId ? updated : r,
            );
            renamingReasonId = null;
            renamingReasonValue = "";
        } catch (e) {
            renameReasonError = $LL.stockOutReasons.renameError({ msg: humanizeError(e) });
        } finally {
            savingReasonRename = false;
        }
    }

    // ─── Archive reason ──────────────────────────────────────────────────────

    function requestReasonArchive(reason: StockOutReason) {
        archiveReasonTarget = reason;
        confirmingReasonArchive = true;
        archiveReasonApiError = "";
    }

    function cancelReasonArchive() {
        confirmingReasonArchive = false;
        archiveReasonTarget = null;
        archiveReasonApiError = "";
    }

    async function confirmReasonArchive() {
        if (!archiveReasonTarget) return;
        archivingReason = true;
        archiveReasonApiError = "";
        const id = archiveReasonTarget.id;
        try {
            await archiveStockOutReason(id);
            await loadReasonsCatalog();
            confirmingReasonArchive = false;
            archiveReasonTarget = null;
        } catch (e) {
            archiveReasonApiError = $LL.stockOutReasons.archiveError({ msg: humanizeError(e) });
        } finally {
            archivingReason = false;
        }
    }

    // ─── Restore reason ──────────────────────────────────────────────────────

    function requestReasonRestore(reason: StockOutReason) {
        restoreReasonTarget = reason;
        confirmingReasonRestore = true;
        restoreReasonApiError = "";
    }

    function cancelReasonRestore() {
        confirmingReasonRestore = false;
        restoreReasonTarget = null;
        restoreReasonApiError = "";
    }

    async function confirmReasonRestore() {
        if (!restoreReasonTarget) return;
        restoringReason = true;
        restoreReasonApiError = "";
        const id = restoreReasonTarget.id;
        try {
            const restored = await unarchiveStockOutReason(id);
            archivedReasons = archivedReasons.filter((r) => r.id !== id);
            activeReasons = [...activeReasons, restored].sort(
                (a, b) => a.sort_order - b.sort_order,
            );
            confirmingReasonRestore = false;
            restoreReasonTarget = null;
        } catch (e) {
            restoreReasonApiError = $LL.stockOutReasons.restoreError({ msg: humanizeError(e) });
        } finally {
            restoringReason = false;
        }
    }

    // ─── Create reason ───────────────────────────────────────────────────────

    function openCreateReasonForm() {
        showCreateReasonForm = true;
        createReasonDisplayName = "";
        createReasonMovementKind = "exit:waste";
        createReasonError = "";
    }

    function closeCreateReasonForm() {
        showCreateReasonForm = false;
        createReasonError = "";
    }

    async function submitCreateReason() {
        if (!createReasonDisplayName.trim()) return;
        creatingReason = true;
        createReasonError = "";
        try {
            const created = await createStockOutReason({
                display_name: createReasonDisplayName.trim(),
                movement_kind: createReasonMovementKind,
            });
            activeReasons = [...activeReasons, created].sort(
                (a, b) => a.sort_order - b.sort_order,
            );
            closeCreateReasonForm();
        } catch (e) {
            createReasonError = $LL.stockOutReasons.createError({ msg: humanizeError(e) });
        } finally {
            creatingReason = false;
        }
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    function kindBadgeClass(kind: UnitKind): string {
        return kind === "integer" ? "badge-neutral" : "badge-outline";
    }

    // Group active units by kind for ordered display.
    const groupedActive = $derived.by(() => {
        const groups: Record<UnitKind, UnitDefinitionResponse[]> = {
            integer: [],
            decimal: [],
        };
        for (const u of activeUnits) {
            groups[u.kind].push(u);
        }
        return groups;
    });

    // Per-option display names for the FEFO policy selector. The dictionary
    // keys live in `configuration.scannerFefoPolicy.names` keyed by the
    // snake_case wire values (`suggest_fefo`, `require_fefo`,
    // `manual_lot_choice`), so adding a new option only requires a matching
    // entry in each locale dictionary.
    function fefoPolicyLabel(value: FefoPolicy): string {
        const names = $LL.configuration.scannerFefoPolicy.names as unknown as Record<
            FefoPolicy,
            () => string
        >;
        return names[value]?.() ?? value;
    }

    // Per-option display names for the close-behaviour selector. The
    // dictionary keys live in `configuration.closeBehavior.names` keyed by
    // the snake_case wire values (`minimize_to_tray`, `exit_application`),
    // so adding a new option only requires a matching entry in each
    // locale dictionary.
    function closeBehaviorLabel(value: CloseBehavior): string {
        const names = $LL.configuration.closeBehavior.names as unknown as Record<
            CloseBehavior,
            () => string
        >;
        return names[value]?.() ?? value;
    }

    // ─── Init ───────────────────────────────────────────────────────────────────

    onMount(async () => {
        try {
            settings = await getSettings();
            requireLocation = settings.require_initial_location_on_lot_create;
            // When the user has a persisted preference, mirror it. On a fresh
            // install the backend returns `"en"` as a fallback with
            // `language_configured: false`; in that case the active locale is
            // already held by the rune (set by `initLocale()` via OS detection),
            // so keep the dropdown in sync with what the user actually sees.
            currentLocale = settings.language_configured
                ? settings.language
                : locale.current;
            // Mirror the persisted FEFO policy + close behaviour so the two
            // new selectors render the actual current value on first paint.
            // The backend already returns the curated enum values (with
            // lenient parsing falling back to the documented defaults), so a
            // direct cast here is safe.
            currentFefoPolicy = settings.scanner_fefo_policy;
            currentCloseBehavior = settings.close_behavior;
        } catch (e) {
            errorMsg = $LL.configuration.language.loadErrorPrefix() + humanizeError(e);
        } finally {
            loading = false;
        }
        // Unit catalog loads independently of settings so each can fail separately.
        await loadUnitCatalog();
        // Reasons catalog also loads independently so the UI can populate even
        // when the settings or unit catalog calls fail.
        await loadReasonsCatalog();
    });

    // ─── Locale selector handler ────────────────────────────────────────────────

    async function handleLocaleChange(next: string) {
        const prev = currentLocale;
        const code = next as SupportedLocale;
        if (code === prev) return;
        // Optimistic update
        currentLocale = code;
        localeErrorMsg = "";

        try {
            await setLocale(code);
            // Update the persisted settings reference; after a successful
            // `setLocale` the backend will report the language as configured.
            if (settings) {
                settings = {
                    ...settings,
                    language: code,
                    language_configured: true,
                };
            }
        } catch (e) {
            // Roll back on failure — use the actual caught error so the
            // structured `CommandError` shape (Tauri) does not collapse to a
            // hard-coded literal string.
            currentLocale = prev;
            localeErrorMsg = $LL.configuration.language.saveErrorPrefix() + humanizeError(e);
        }
    }

    // ─── Location toggle handler ────────────────────────────────────────────────

    async function handleToggle() {
        const newValue = !requireLocation;
        // Optimistic update
        requireLocation = newValue;
        errorMsg = "";
        savingLocation = true;

        try {
            await updateSettings({
                require_initial_location_on_lot_create: newValue,
            });
            // Persist the confirmed value
            settings = { ...settings!, require_initial_location_on_lot_create: newValue };
        } catch (e) {
            // Rollback on failure
            requireLocation = !newValue;
            errorMsg = $LL.common.error() + ": " + humanizeError(e);
        } finally {
            savingLocation = false;
        }
    }

    // ─── Theme switcher handler ────────────────────────────────────────────────

    async function handleThemeChange(next: string) {
        const name = next as ThemeName;
        if (name === theme.current) return;
        const prevSource = theme.source;
        switchingTheme = true;
        themeError = "";

        try {
            await setTheme(name);
            // After a successful set, the persisted settings reference
            // carries `theme_configured: true`. Mirror that locally so
            // the section reads consistently if a future PR exposes
            // more theme-derived metadata here.
            if (settings) {
                settings = { ...settings, theme: name, theme_configured: true };
            }
        } catch (e) {
            // `setTheme` already rolled back the rune + the document
            // attribute on rejection; we only need to surface the
            // message inline. Restore the prior source label so the
            // switcher shows the correct provenance.
            theme.source = prevSource;
            themeError = $LL.settings.theme.error({ msg: humanizeError(e) });
        } finally {
            switchingTheme = false;
        }
    }

    // ─── FEFO policy selector handler (`scanner-quick-operations` PR 3) ──────

    async function handleFefoPolicyChange(next: string) {
        const value = next as FefoPolicy;
        const prev = currentFefoPolicy;
        if (value === prev) return;
        // Optimistic update
        currentFefoPolicy = value;
        fefoPolicyError = "";
        savingFefoPolicy = true;

        try {
            const updated = await updateSettings({ scanner_fefo_policy: value });
            // Persist the confirmed value; the backend re-emits the full
            // snapshot so the local `settings` mirror stays in sync with the
            // persisted row.
            settings = { ...settings!, scanner_fefo_policy: updated.scanner_fefo_policy };
        } catch (e) {
            // Roll back on failure — use the actual caught error so the
            // structured `CommandError` shape (Tauri) does not collapse to a
            // hard-coded literal string.
            currentFefoPolicy = prev;
            fefoPolicyError =
                $LL.configuration.language.saveErrorPrefix() + humanizeError(e);
        } finally {
            savingFefoPolicy = false;
        }
    }

    // ─── Close-behaviour selector handler (`scanner-quick-operations` PR 3) ──

    async function handleCloseBehaviorChange(next: string) {
        const value = next as CloseBehavior;
        const prev = currentCloseBehavior;
        if (value === prev) return;
        // Optimistic update
        currentCloseBehavior = value;
        closeBehaviorError = "";
        savingCloseBehavior = true;

        try {
            const updated = await updateSettings({ close_behavior: value });
            // Persist the confirmed value; the backend re-emits the full
            // snapshot so the local `settings` mirror stays in sync with the
            // persisted row.
            settings = { ...settings!, close_behavior: updated.close_behavior };
        } catch (e) {
            // Roll back on failure.
            currentCloseBehavior = prev;
            closeBehaviorError =
                $LL.configuration.language.saveErrorPrefix() + humanizeError(e);
        } finally {
            savingCloseBehavior = false;
        }
    }
</script>

<div class="page">
    <div class="page-header">
        <h1 class="page-title">{$LL.configuration.pageTitle()}</h1>
    </div>

    {#if loading}
        <LoadingState variant="text" label={$LL.common.loading()} />
    {:else if errorMsg && !settings}
        <Alert variant="error">{errorMsg}</Alert>
    {:else}
        <!-- ─── Language section ────────────────────────────────────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.configuration.language.sectionTitle()}
            </h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">{$LL.configuration.language.label()}</span>
                    <!-- Shown next to the selector only while the value comes from OS/browser detection. -->
                    {#if translationSource.current === "detected"}
                        <span class="detected-hint">
                            {$LL.configuration.language.detectedHint({
                                locale: languageLabel(currentLocale),
                            })}
                        </span>
                    {/if}
                </div>

                <!--
                  Language selector. We render the themed `Listbox`
                  primitive so the open picker inherits the DaisyUI
                  tokens (no OS-native black dropdown on Chromium /
                  WebKit). The `value` is bound to the local
                  `currentLocale` rune so the optimistic update +
                  rollback in `handleLocaleChange` stays verbatim.
                  `options` carries per-entry display labels bound to
                  the `configuration.language.names` dictionary so
                  adding a new locale only requires a matching
                  dictionary entry.
                -->
                <div class="setting-control">
                    <Listbox
                        value={currentLocale}
                        options={AVAILABLE_LOCALES.map((code) => ({
                            value: code,
                            label: languageLabel(code),
                        }))}
                        size="md"
                        aria-label={$LL.configuration.language.label()}
                        onchange={handleLocaleChange}
                    />
                </div>
            </div>

            {#if localeErrorMsg}
                <div class="section-status">
                    <Alert variant="error">{localeErrorMsg}</Alert>
                </div>
            {/if}
        </Card>

        <!-- ─── Lots section ─────────────────────────────────────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.configuration.section.lots()}
            </h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">{$LL.configuration.locationRequired.label()}</span>
                    <span class="setting-desc">
                        {$LL.configuration.locationRequired.description()}
                    </span>
                </div>

                <!--
                  Toggle.svelte wraps DaisyUI's `toggle toggle-primary`
                  around a native `<input type="checkbox">`. The
                  visible label lives next to the toggle; the `aria-label`
                  variant is used for screen-reader-only labelling.
                  `savingLocation` is forwarded as `disabled` so the
                  user cannot re-submit mid-save.
                -->
                <div class="setting-control">
                    <Toggle
                        checked={requireLocation}
                        label={$LL.configuration.locationRequired.label()}
                        size="md"
                        disabled={savingLocation}
                        onchange={handleToggle}
                    />
                </div>
            </div>

            {#if savingLocation}
                <div class="section-status">
                    <LoadingState
                        variant="spinner"
                        label={$LL.configuration.language.saving()}
                    />
                </div>
            {/if}
            {#if errorMsg}
                <div class="section-status">
                    <Alert variant="error">{errorMsg}</Alert>
                </div>
            {/if}
        </Card>

        <!-- ─── Theme section (PR 5) ────────────────────────────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.settings.theme.title()}
            </h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">{$LL.settings.theme.title()}</span>
                    <span class="setting-desc">
                        {$LL.settings.theme.description()}
                    </span>
                    <!-- Provenance label for the active theme. -->
                    <span class="theme-source">
                        {themeSourceLabel(theme.source)}
                    </span>
                </div>

                <!--
                  Theme switcher. We render the curated theme set as a
                  themed `Listbox` list so the switcher inherits the
                  same WAI-ARIA 1.2 select-only combobox contract
                  (trigger button + popover listbox) as the language
                  picker, but with no native `<select>` chrome
                  leaking OS styling. The currently active theme is
                  the selected value, and the switcher is disabled
                  while an IPC save is in flight (`switchingTheme`).
                -->
                <div class="setting-control">
                    <Listbox
                        value={theme.current}
                        options={AVAILABLE_THEMES.map((name) => ({
                            value: name,
                            label: themeLabel(name),
                        }))}
                        size="md"
                        aria-label={$LL.settings.theme.title()}
                        disabled={switchingTheme}
                        onchange={handleThemeChange}
                    />
                </div>
            </div>

            {#if themeError}
                <div class="section-status">
                    <Alert variant="error">{themeError}</Alert>
                </div>
            {/if}
        </Card>

        <!-- ─── Scanner FEFO policy section (PR 3) ──────────────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.configuration.scannerFefoPolicy.sectionTitle()}
            </h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">
                        {$LL.configuration.scannerFefoPolicy.label()}
                        <!--
                          Rendered FEFO tooltip — pointer + keyboard accessible.
                          The `tooltip` key carries the acronym expansion
                          ("First Expired, First Out" / "Primero Vencido,
                          Primero Fuera") so the user understands the picker
                          label at a glance. `title` provides the native browser
                          tooltip on hover; `aria-label` ensures screen readers
                          also announce the explanation on focus.
                        -->
                        <button
                            type="button"
                            class="fefo-tooltip-trigger"
                            title={$LL.configuration.scannerFefoPolicy.tooltip()}
                            aria-label={$LL.configuration.scannerFefoPolicy.tooltip()}
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 16 16"
                                fill="currentColor"
                                class="fefo-tooltip-icon"
                                aria-hidden="true"
                                focusable="false"
                            >
                                <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1zm0 2.25a.75.75 0 1 1 0 1.5.75.75 0 0 1 0-1.5zM8.75 6a.75.75 0 0 0-1.5 0v3.5a.75.75 0 0 0 1.5 0V6z" />
                            </svg>
                        </button>
                    </span>
                    <span class="setting-desc">
                        {$LL.configuration.scannerFefoPolicy.description()}
                    </span>
                    <!--
                      Dynamic selected-policy explanation. Uses the
                      `selectedDescription` dictionary keyed by the current
                      `currentFefoPolicy` value so the user immediately
                      understands the active policy without having to re-read
                      the general description above.
                    -->
                    <span class="fefo-selected-explanation">
                        {(() => {
                            const descriptions = $LL.configuration.scannerFefoPolicy.selectedDescription as unknown as Record<FefoPolicy, () => string>;
                            return descriptions[currentFefoPolicy]?.() ?? "";
                        })()}
                    </span>
                </div>

                <!--
                  Three-option selector (`suggest_fefo` / `require_fefo` /
                  `manual_lot_choice`) mirroring the language picker pattern:
                  rendered through the themed `Listbox` primitive so the
                  open picker inherits the DaisyUI tokens (no OS-native
                  black dropdown on Chromium / WebKit). The per-option
                  labels come from the `configuration.scannerFefoPolicy.names`
                  dictionary, and `savingFefoPolicy` disables the selector
                  while the IPC save is in flight.
                -->
                <div class="setting-control">
                    <Listbox
                        value={currentFefoPolicy}
                        options={AVAILABLE_FEFO_POLICIES.map((value) => ({
                            value,
                            label: fefoPolicyLabel(value),
                        }))}
                        size="md"
                        aria-label={$LL.configuration.scannerFefoPolicy.label()}
                        disabled={savingFefoPolicy}
                        onchange={handleFefoPolicyChange}
                    />
                </div>
            </div>

            {#if savingFefoPolicy}
                <div class="section-status">
                    <LoadingState
                        variant="spinner"
                        label={$LL.configuration.language.saving()}
                    />
                </div>
            {/if}
            {#if fefoPolicyError}
                <div class="section-status">
                    <Alert variant="error">{fefoPolicyError}</Alert>
                </div>
            {/if}
        </Card>

        <!-- ─── Close-window behaviour section (PR 3) ─────────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.configuration.closeBehavior.sectionTitle()}
            </h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">
                        {$LL.configuration.closeBehavior.label()}
                    </span>
                    <span class="setting-desc">
                        {$LL.configuration.closeBehavior.description()}
                    </span>
                </div>

                <!--
                  Two-option selector (`minimize_to_tray` /
                  `exit_application`) mirroring the language picker
                  pattern, but rendered through the themed `Listbox`
                  primitive so the open picker inherits the DaisyUI
                  tokens (no OS-native black dropdown on Chromium /
                  WebKit). The selected value is read from the
                  persisted `SettingsResponse.close_behavior` field;
                  `savingCloseBehavior` disables the selector while the
                  IPC save is in flight.
                -->
                <div class="setting-control">
                    <Listbox
                        value={currentCloseBehavior}
                        options={AVAILABLE_CLOSE_BEHAVIORS.map((value) => ({
                            value,
                            label: closeBehaviorLabel(value),
                        }))}
                        size="md"
                        aria-label={$LL.configuration.closeBehavior.label()}
                        disabled={savingCloseBehavior}
                        onchange={handleCloseBehaviorChange}
                    />
                </div>
            </div>

            {#if savingCloseBehavior}
                <div class="section-status">
                    <LoadingState
                        variant="spinner"
                        label={$LL.configuration.language.saving()}
                    />
                </div>
            {/if}
            {#if closeBehaviorError}
                <div class="section-status">
                    <Alert variant="error">{closeBehaviorError}</Alert>
                </div>
            {/if}
        </Card>

        <!-- ─── Unit catalog section (ODD task 2.6b) ─────────────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.unitCatalog.sectionTitle()}
            </h2>
            <p class="section-desc">
                {$LL.unitCatalog.description()}
            </p>

            {#if loadingUnits}
                <LoadingState
                    variant="spinner"
                    label={$LL.common.loading()}
                />
            {:else if unitsLoadError}
                <Alert variant="error">{unitsLoadError}</Alert>
            {:else}
                <!-- ─── Active units ─────────────────────────────────────── -->
                <div class="unit-block">
                    <div class="unit-block-header">
                        <span class="unit-block-label">
                            {$LL.unitCatalog.activeLabel()}
                        </span>
                        <span class="unit-count-badge">
                            {activeUnits.length}
                        </span>
                    </div>

                    {#if activeUnits.length === 0}
                        <p class="unit-empty">{$LL.unitCatalog.noActiveUnits()}</p>
                    {:else}
                        {#each ([
                            { kind: 'integer' as const, label: $LL.unitCatalog.kind.integer(), units: groupedActive.integer },
                            { kind: 'decimal' as const, label: $LL.unitCatalog.kind.decimal(), units: groupedActive.decimal },
                        ]) as group (group.kind)}
                            {#if group.units.length > 0}
                                <div class="unit-kind-group">
                                    <span class="unit-kind-label">{group.label}</span>
                                    <div class="unit-list">
                                        {#each group.units as unit (unit.id)}
                                            <div class="unit-row">
                                                <div class="unit-info">
                                                    <span class="unit-badge {kindBadgeClass(unit.kind)}">{group.label}</span>
                                                    {#if renamingId === unit.id}
                                                        <input
                                                            class="input input-sm unit-rename-input"
                                                            type="text"
                                                            bind:value={renameValue}
                                                            onkeydown={(e) => {
                                                                if (e.key === "Enter") submitRename();
                                                                if (e.key === "Escape") cancelRename();
                                                            }}
                                                            disabled={savingRename}
                                                            aria-label={$LL.unitCatalog.displayNameLabel()}
                                                        />
                                                    {:else}
                                                        <span class="unit-display-name">{unit.display_name}</span>
                                                    {/if}
                                                    {#if unit.is_preset}
                                                        <span class="unit-preset-tag">{$LL.unitCatalog.preset()}</span>
                                                    {/if}
                                                </div>
                                                <div class="unit-actions">
                                                    {#if renamingId === unit.id}
                                                        <Button
                                                            size="sm"
                                                            variant="primary"
                                                            loading={savingRename}
                                                            onclick={submitRename}
                                                        >
                                                            {$LL.unitCatalog.saveRename()}
                                                        </Button>
                                                        <Button
                                                            size="sm"
                                                            variant="ghost"
                                                            onclick={cancelRename}
                                                            disabled={savingRename}
                                                        >
                                                            {$LL.unitCatalog.cancelRename()}
                                                        </Button>
                                                    {:else}
                                                        <Button
                                                            size="sm"
                                                            variant="ghost"
                                                            onclick={() => startRename(unit)}
                                                        >
                                                            {$LL.unitCatalog.editDisplayName()}
                                                        </Button>
                                                        <Button
                                                            size="sm"
                                                            variant="ghost"
                                                            onclick={() => requestArchive(unit)}
                                                        >
                                                            {$LL.unitCatalog.archive()}
                                                        </Button>
                                                    {/if}
                                                </div>
                                            </div>
                                        {/each}
                                    </div>
                                </div>
                            {/if}
                        {/each}
                    {/if}

                    {#if renameError}
                        <div class="unit-error">
                            <Alert variant="error">{renameError}</Alert>
                        </div>
                    {/if}
                </div>

                <!-- ─── New unit form ───────────────────────────────────── -->
                {#if showCreateForm}
                    <div class="unit-create-form">
                        <h3 class="unit-create-title">
                            {$LL.unitCatalog.createUnitTitle()}
                        </h3>
                        <p class="unit-create-desc">
                            {$LL.unitCatalog.createUnitDesc()}
                        </p>

                        <div class="unit-create-fields">
                            <div class="unit-create-row">
                                <div class="unit-create-field">
                                    <label
                                        class="fieldset-label"
                                        for="create-unit-key"
                                    >
                                        {$LL.unitCatalog.keyLabel()}
                                        <span class="text-error" aria-hidden="true">*</span>
                                    </label>
                                    <input
                                        id="create-unit-key"
                                        class="input input-sm w-full"
                                        type="text"
                                        placeholder={$LL.unitCatalog.keyPlaceholder()}
                                        bind:value={createKey}
                                        disabled={creatingUnit}
                                        onkeydown={(e) => {
                                            if (
                                                e.key === "Enter" &&
                                                createKey.trim() &&
                                                createDisplayName.trim()
                                            )
                                                submitCreateUnit();
                                        }}
                                    />
                                </div>
                                <div class="unit-create-field">
                                    <label
                                        class="fieldset-label"
                                        for="create-unit-name"
                                    >
                                        {$LL.unitCatalog.displayNameLabel()}
                                        <span class="text-error" aria-hidden="true">*</span>
                                    </label>
                                    <input
                                        id="create-unit-name"
                                        class="input input-sm w-full"
                                        type="text"
                                        placeholder={$LL.unitCatalog.displayNameNewPlaceholder()}
                                        bind:value={createDisplayName}
                                        disabled={creatingUnit}
                                        onkeydown={(e) => {
                                            if (
                                                e.key === "Enter" &&
                                                createKey.trim() &&
                                                createDisplayName.trim()
                                            )
                                                submitCreateUnit();
                                        }}
                                    />
                                </div>
                                <div class="unit-create-field">
                                    <label
                                        class="fieldset-label"
                                        for="create-unit-kind"
                                    >
                                        {$LL.unitCatalog.kindLabel()}
                                    </label>
                                    <select
                                        id="create-unit-kind"
                                        class="select select-sm w-full"
                                        bind:value={createKind}
                                        disabled={creatingUnit}
                                    >
                                        <option value="integer">{$LL.unitCatalog.kind.integer()}</option>
                                        <option value="decimal">{$LL.unitCatalog.kind.decimal()}</option>
                                    </select>
                                </div>
                            </div>
                        </div>

                        {#if createError}
                            <div class="unit-error">
                                <Alert variant="error">{createError}</Alert>
                            </div>
                        {/if}

                        <div class="unit-create-actions">
                            <Button
                                variant="primary"
                                size="sm"
                                loading={creatingUnit}
                                disabled={!createKey.trim() || !createDisplayName.trim()}
                                onclick={submitCreateUnit}
                            >
                                {$LL.unitCatalog.addUnit()}
                            </Button>
                            <Button
                                variant="ghost"
                                size="sm"
                                onclick={closeCreateForm}
                                disabled={creatingUnit}
                            >
                                {$LL.common.cancel()}
                            </Button>
                        </div>
                    </div>
                {:else}
                    <div class="unit-create-trigger">
                        <Button
                            variant="ghost"
                            size="sm"
                            onclick={openCreateForm}
                        >
                            {$LL.unitCatalog.createCustom()}
                        </Button>
                    </div>
                {/if}

                <!-- ─── Archived units ──────────────────────────────────── -->
                <div class="unit-block">
                    <button
                        class="unit-block-header unit-block-toggle"
                        onclick={() => (showArchived = !showArchived)}
                        aria-expanded={showArchived}
                    >
                        <span class="unit-block-label">
                            {$LL.unitCatalog.archivedLabel()}
                        </span>
                        <span class="unit-count-badge">
                            {archivedUnits.length}
                        </span>
                        <span class="unit-toggle-icon">
                            {showArchived ? "▲" : "▼"}
                        </span>
                    </button>

                    {#if showArchived}
                        {#if archivedUnits.length === 0}
                            <p class="unit-empty">{$LL.unitCatalog.noArchivedUnits()}</p>
                        {:else}
                            <div class="unit-list">
                                {#each archivedUnits as unit (unit.id)}
                                    <div class="unit-row unit-row-archived">
                                        <div class="unit-info">
                                            <span class="unit-badge {kindBadgeClass(unit.kind)}">
                                                {unit.kind === "integer"
                                                    ? $LL.unitCatalog.kind.integer()
                                                    : $LL.unitCatalog.kind.decimal()}
                                            </span>
                                            <span class="unit-display-name unit-display-name-muted">
                                                {unit.display_name}
                                            </span>
                                        </div>
                                        <div class="unit-actions">
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                onclick={() => requestRestore(unit)}
                                            >
                                                {$LL.unitCatalog.restore()}
                                            </Button>
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    {/if}
                </div>

                <!-- ─── Archive confirmation modal ─────────────────────── -->
                <Modal
                    bind:open={confirmingArchive}
                    titleId="archive-modal-title"
                    descriptionId="archive-modal-desc"
                    closeOnBackdrop={false}
                    onclose={() => { archiveTarget = null; }}
                >
                    {#snippet children()}
                        <h2 id="archive-modal-title" class="modal-title">
                            {$LL.unitCatalog.confirmArchive({ name: archiveTarget?.display_name ?? '' })}
                        </h2>
                        <p id="archive-modal-desc" class="modal-body">
                            {$LL.unitCatalog.confirmArchiveBody()}
                        </p>
                        {#if archiveApiError}
                            <div class="unit-error">
                                <Alert variant="error">{archiveApiError}</Alert>
                            </div>
                        {/if}
                    {/snippet}
                    {#snippet footer()}
                        <Button
                            variant="ghost"
                            size="sm"
                            onclick={cancelArchive}
                            disabled={archivingUnit}
                        >
                            {$LL.common.cancel()}
                        </Button>
                        <Button
                            variant="primary"
                            size="sm"
                            loading={archivingUnit}
                            onclick={confirmArchive}
                        >
                            {$LL.unitCatalog.confirmArchiveYes()}
                        </Button>
                    {/snippet}
                </Modal>

                <!-- ─── Restore confirmation modal ─────────────────────── -->
                <Modal
                    bind:open={confirmingRestore}
                    titleId="restore-modal-title"
                    descriptionId="restore-modal-desc"
                    closeOnBackdrop={false}
                    onclose={() => { restoreTarget = null; }}
                >
                    {#snippet children()}
                        <h2 id="restore-modal-title" class="modal-title">
                            {$LL.unitCatalog.confirmRestore({ name: restoreTarget?.display_name ?? '' })}
                        </h2>
                        <p id="restore-modal-desc" class="modal-body">
                            {$LL.unitCatalog.confirmRestoreBody()}
                        </p>
                        {#if restoreApiError}
                            <div class="unit-error">
                                <Alert variant="error">{restoreApiError}</Alert>
                            </div>
                        {/if}
                    {/snippet}
                    {#snippet footer()}
                        <Button
                            variant="ghost"
                            size="sm"
                            onclick={cancelRestore}
                            disabled={restoringUnit}
                        >
                            {$LL.common.cancel()}
                        </Button>
                        <Button
                            variant="primary"
                            size="sm"
                            loading={restoringUnit}
                            onclick={confirmRestore}
                        >
                            {$LL.unitCatalog.confirmRestoreYes()}
                        </Button>
                    {/snippet}
                </Modal>
            {/if}
        </Card>

        <!-- ─── Stock-out reasons catalog section (ODD task 2.7e) ────────────── -->
        <Card tone="default">
            <h2 class="section-title">
                {$LL.stockOutReasons.sectionTitle()}
            </h2>
            <p class="section-desc">
                {$LL.stockOutReasons.description()}
            </p>

            {#if loadingReasons}
                <LoadingState
                    variant="spinner"
                    label={$LL.common.loading()}
                />
            {:else if reasonsLoadError}
                <Alert variant="error">{reasonsLoadError}</Alert>
            {:else}
                <!-- ─── Active reasons ───────────────────────────────────────── -->
                <div class="unit-block">
                    <div class="unit-block-header">
                        <span class="unit-block-label">
                            {$LL.stockOutReasons.activeLabel()}
                        </span>
                        <span class="unit-count-badge">
                            {activeReasons.length}
                        </span>
                    </div>

                    {#if activeReasons.length === 0}
                        <p class="unit-empty">{$LL.stockOutReasons.noActiveReasons()}</p>
                    {:else}
                        <div class="unit-list">
                            {#each activeReasons as reason (reason.id)}
                                <div class="unit-row">
                                    <div class="unit-info">
                                        <span class="unit-badge badge-outline">
                                            {(() => {
                                                const kinds = $LL.stockOutReasons.kinds as unknown as Record<
                                                    ExitReasonMovementKind,
                                                    () => string
                                                >;
                                                return kinds[reason.movement_kind as ExitReasonMovementKind]?.() ??
                                                    reason.movement_kind;
                                            })()}
                                        </span>
                                        {#if renamingReasonId === reason.id}
                                            <input
                                                class="input input-sm unit-rename-input"
                                                type="text"
                                                bind:value={renamingReasonValue}
                                                onkeydown={(e) => {
                                                    if (e.key === "Enter") submitReasonRename();
                                                    if (e.key === "Escape") cancelReasonRename();
                                                }}
                                                disabled={savingReasonRename}
                                                aria-label={$LL.stockOutReasons.displayNameLabel()}
                                            />
                                        {:else}
                                            <span class="unit-display-name">{reason.display_name}</span>
                                        {/if}
                                    </div>
                                    <div class="unit-actions">
                                        {#if renamingReasonId === reason.id}
                                            <Button
                                                size="sm"
                                                variant="primary"
                                                loading={savingReasonRename}
                                                onclick={submitReasonRename}
                                            >
                                                {$LL.stockOutReasons.saveRename()}
                                            </Button>
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                onclick={cancelReasonRename}
                                                disabled={savingReasonRename}
                                            >
                                                {$LL.stockOutReasons.cancelRename()}
                                            </Button>
                                        {:else}
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                onclick={() => startReasonRename(reason)}
                                            >
                                                {$LL.stockOutReasons.editDisplayName()}
                                            </Button>
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                onclick={() => requestReasonArchive(reason)}
                                            >
                                                {$LL.stockOutReasons.archive()}
                                            </Button>
                                        {/if}
                                    </div>
                                </div>
                            {/each}
                        </div>
                    {/if}

                    {#if renameReasonError}
                        <div class="unit-error">
                            <Alert variant="error">{renameReasonError}</Alert>
                        </div>
                    {/if}
                </div>

                <!-- ─── New reason form ─────────────────────────────────────── -->
                {#if showCreateReasonForm}
                    <div class="unit-create-form">
                        <h3 class="unit-create-title">
                            {$LL.stockOutReasons.createTitle()}
                        </h3>
                        <p class="unit-create-desc">
                            {$LL.stockOutReasons.createDesc()}
                        </p>

                        <div class="unit-create-fields">
                            <div class="unit-create-row">
                                <div class="unit-create-field">
                                    <label
                                        class="fieldset-label"
                                        for="create-reason-name"
                                    >
                                        {$LL.stockOutReasons.displayNameLabel()}
                                        <span class="text-error" aria-hidden="true">*</span>
                                    </label>
                                    <input
                                        id="create-reason-name"
                                        class="input input-sm w-full"
                                        type="text"
                                        placeholder={$LL.stockOutReasons.displayNameNewPlaceholder()}
                                        bind:value={createReasonDisplayName}
                                        disabled={creatingReason}
                                        onkeydown={(e) => {
                                            if (
                                                e.key === "Enter" &&
                                                createReasonDisplayName.trim()
                                            )
                                                submitCreateReason();
                                        }}
                                    />
                                </div>
                                <div class="unit-create-field">
                                    <label
                                        class="fieldset-label"
                                        for="create-reason-kind"
                                    >
                                        {$LL.stockOutReasons.movementKindLabel()}
                                    </label>
                                    <select
                                        id="create-reason-kind"
                                        class="select select-sm w-full"
                                        bind:value={createReasonMovementKind}
                                        disabled={creatingReason}
                                    >
                                        {#each AVAILABLE_MOVEMENT_KINDS as kind}
                                            <option value={kind}>
                                                {(() => {
                                                    const kinds = $LL.stockOutReasons.kinds as unknown as Record<
                                                        ExitReasonMovementKind,
                                                        () => string
                                                    >;
                                                    return kinds[kind]?.() ?? kind;
                                                })()}
                                            </option>
                                        {/each}
                                    </select>
                                </div>
                            </div>
                        </div>

                        {#if createReasonError}
                            <div class="unit-error">
                                <Alert variant="error">{createReasonError}</Alert>
                            </div>
                        {/if}

                        <div class="unit-create-actions">
                            <Button
                                variant="primary"
                                size="sm"
                                loading={creatingReason}
                                disabled={!createReasonDisplayName.trim()}
                                onclick={submitCreateReason}
                            >
                                {$LL.stockOutReasons.addReason()}
                            </Button>
                            <Button
                                variant="ghost"
                                size="sm"
                                onclick={closeCreateReasonForm}
                                disabled={creatingReason}
                            >
                                {$LL.common.cancel()}
                            </Button>
                        </div>
                    </div>
                {:else}
                    <div class="unit-create-trigger">
                        <Button
                            variant="ghost"
                            size="sm"
                            onclick={openCreateReasonForm}
                        >
                            {$LL.stockOutReasons.createNew()}
                        </Button>
                    </div>
                {/if}

                <!-- ─── Archived reasons ────────────────────────────────────── -->
                <div class="unit-block">
                    <button
                        class="unit-block-header unit-block-toggle"
                        onclick={() => (showArchivedReasons = !showArchivedReasons)}
                        aria-expanded={showArchivedReasons}
                    >
                        <span class="unit-block-label">
                            {$LL.stockOutReasons.archivedLabel()}
                        </span>
                        <span class="unit-count-badge">
                            {archivedReasons.length}
                        </span>
                        <span class="unit-toggle-icon">
                            {showArchivedReasons ? "▲" : "▼"}
                        </span>
                    </button>

                    {#if showArchivedReasons}
                        {#if archivedReasons.length === 0}
                            <p class="unit-empty">{$LL.stockOutReasons.noArchivedReasons()}</p>
                        {:else}
                            <div class="unit-list">
                                {#each archivedReasons as reason (reason.id)}
                                    <div class="unit-row unit-row-archived">
                                        <div class="unit-info">
                                            <span class="unit-badge badge-outline">
                                                {(() => {
                                                    const kinds = $LL.stockOutReasons.kinds as unknown as Record<
                                                        ExitReasonMovementKind,
                                                        () => string
                                                    >;
                                                    return kinds[reason.movement_kind as ExitReasonMovementKind]?.() ??
                                                        reason.movement_kind;
                                                })()}
                                            </span>
                                            <span class="unit-display-name unit-display-name-muted">
                                                {reason.display_name}
                                            </span>
                                        </div>
                                        <div class="unit-actions">
                                            <Button
                                                size="sm"
                                                variant="ghost"
                                                onclick={() => requestReasonRestore(reason)}
                                            >
                                                {$LL.stockOutReasons.restore()}
                                            </Button>
                                        </div>
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    {/if}
                </div>

                <!-- ─── Archive confirmation modal ─────────────────────────── -->
                <Modal
                    bind:open={confirmingReasonArchive}
                    titleId="reason-archive-modal-title"
                    descriptionId="reason-archive-modal-desc"
                    closeOnBackdrop={false}
                    onclose={() => { archiveReasonTarget = null; }}
                >
                    {#snippet children()}
                        <h2 id="reason-archive-modal-title" class="modal-title">
                            {$LL.stockOutReasons.confirmArchive({
                                name: archiveReasonTarget?.display_name ?? "",
                            })}
                        </h2>
                        <p id="reason-archive-modal-desc" class="modal-body">
                            {$LL.stockOutReasons.confirmArchiveBody()}
                        </p>
                        {#if archiveReasonApiError}
                            <div class="unit-error">
                                <Alert variant="error">{archiveReasonApiError}</Alert>
                            </div>
                        {/if}
                    {/snippet}
                    {#snippet footer()}
                        <Button
                            variant="ghost"
                            size="sm"
                            onclick={cancelReasonArchive}
                            disabled={archivingReason}
                        >
                            {$LL.common.cancel()}
                        </Button>
                        <Button
                            variant="primary"
                            size="sm"
                            loading={archivingReason}
                            onclick={confirmReasonArchive}
                        >
                            {$LL.stockOutReasons.confirmArchiveYes()}
                        </Button>
                    {/snippet}
                </Modal>

                <!-- ─── Restore confirmation modal ─────────────────────────── -->
                <Modal
                    bind:open={confirmingReasonRestore}
                    titleId="reason-restore-modal-title"
                    descriptionId="reason-restore-modal-desc"
                    closeOnBackdrop={false}
                    onclose={() => { restoreReasonTarget = null; }}
                >
                    {#snippet children()}
                        <h2 id="reason-restore-modal-title" class="modal-title">
                            {$LL.stockOutReasons.confirmRestore({
                                name: restoreReasonTarget?.display_name ?? "",
                            })}
                        </h2>
                        <p id="reason-restore-modal-desc" class="modal-body">
                            {$LL.stockOutReasons.confirmRestoreBody()}
                        </p>
                        {#if restoreReasonApiError}
                            <div class="unit-error">
                                <Alert variant="error">{restoreReasonApiError}</Alert>
                            </div>
                        {/if}
                    {/snippet}
                    {#snippet footer()}
                        <Button
                            variant="ghost"
                            size="sm"
                            onclick={cancelReasonRestore}
                            disabled={restoringReason}
                        >
                            {$LL.common.cancel()}
                        </Button>
                        <Button
                            variant="primary"
                            size="sm"
                            loading={restoringReason}
                            onclick={confirmReasonRestore}
                        >
                            {$LL.stockOutReasons.confirmRestoreYes()}
                        </Button>
                    {/snippet}
                </Modal>
            {/if}
        </Card>
    {/if}
</div>

<style>
    .page {
        max-width: 680px;
        margin: 0 auto;
        padding: 28px 24px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .page-header {
        margin-bottom: 12px;
    }

    .page-title {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--color-base-content);
        margin: 0;
    }

    /* ─── Section title inside the Card primitive ─────────────────────────────
       The Card primitive owns the outer chrome (border, padding, shadow); the
       h2 stays visible inside the card-body so the section name keeps the
       same affordance as before. */
    :global(.card-body) .section-title {
        font-size: 0.95rem;
        font-weight: 600;
        color: var(--color-secondary);
        text-transform: uppercase;
        letter-spacing: 0.06em;
        margin: 0 0 16px 0;
    }

    /* ─── Detected hint ──────────────────────────────────────────────────────── */

    .detected-hint {
        align-self: flex-start;
        font-size: 0.82rem;
        color: var(--color-primary);
        background: color-mix(in oklch, var(--color-primary) 8%, transparent);
        border: 1px solid color-mix(in oklch, var(--color-primary) 30%, transparent);
        border-radius: 0.375rem;
        padding: 4px 10px;
    }

    /* ─── Setting row ─────────────────────────────────────────────────────────── */

    .setting-row {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 20px;
    }

    .setting-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .setting-label {
        font-size: 0.95rem;
        font-weight: 500;
        color: var(--color-base-content);
    }

    .setting-desc {
        font-size: 0.82rem;
        color: var(--color-secondary);
        line-height: 1.5;
    }

    /* Right-aligned control slot. Sized so Select / Toggle primitives
       share a consistent width across sections. */
    .setting-control {
        flex-shrink: 0;
        min-width: 12rem;
        display: flex;
        justify-content: flex-end;
    }

    /* ─── Theme switcher extras ──────────────────────────────────────────────── */

    .theme-source {
        align-self: flex-start;
        font-size: 0.78rem;
        color: var(--color-secondary);
        margin-top: 0.5rem;
        font-style: italic;
    }

    /* ─── Section status (loading / error) ───────────────────────────────────── */
    .section-status {
        margin-top: 12px;
    }

    /* ─── Section description ─────────────────────────────────────────────────── */
    .section-desc {
        font-size: 0.82rem;
        color: var(--color-secondary);
        line-height: 1.5;
        margin: 0 0 16px 0;
    }

    /* ─── Unit catalog ───────────────────────────────────────────────────────── */

    .unit-block {
        margin-bottom: 16px;
    }

    .unit-block-header {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 8px;
    }

    .unit-block-label {
        font-size: 0.82rem;
        font-weight: 600;
        color: var(--color-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .unit-count-badge {
        font-size: 0.75rem;
        background: color-mix(in oklch, var(--color-secondary) 15%, transparent);
        color: var(--color-secondary);
        border-radius: 999px;
        padding: 1px 7px;
        font-weight: 500;
    }

    .unit-block-toggle {
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        font: inherit;
        text-align: left;
        width: 100%;
    }

    .unit-toggle-icon {
        font-size: 0.7rem;
        color: var(--color-secondary);
        margin-left: auto;
    }

    .unit-kind-group {
        margin-bottom: 12px;
    }

    .unit-kind-label {
        display: block;
        font-size: 0.75rem;
        color: var(--color-secondary);
        margin-bottom: 4px;
        padding-left: 2px;
    }

    .unit-list {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .unit-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 6px 8px;
        border-radius: 0.375rem;
        border: 1px solid color-mix(in oklch, var(--color-base-300) 50%, transparent);
    }

    .unit-row-archived {
        opacity: 0.75;
    }

    .unit-info {
        display: flex;
        align-items: center;
        gap: 8px;
        min-width: 0;
        flex: 1;
    }

    .unit-badge {
        font-size: 0.72rem;
        padding: 2px 6px;
        border-radius: 0.25rem;
        white-space: nowrap;
        flex-shrink: 0;
    }

    .unit-display-name {
        font-size: 0.9rem;
        color: var(--color-base-content);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .unit-display-name-muted {
        color: var(--color-secondary);
        text-decoration: line-through;
        opacity: 0.7;
    }

    .unit-preset-tag {
        font-size: 0.68rem;
        color: var(--color-secondary);
        background: color-mix(in oklch, var(--color-secondary) 10%, transparent);
        border: 1px solid color-mix(in oklch, var(--color-secondary) 25%, transparent);
        border-radius: 0.25rem;
        padding: 1px 5px;
        white-space: nowrap;
        flex-shrink: 0;
    }

    .unit-rename-input {
        min-width: 160px;
        flex-shrink: 0;
    }

    .unit-actions {
        display: flex;
        align-items: center;
        gap: 4px;
        flex-shrink: 0;
    }

    .unit-error {
        margin-top: 8px;
    }

    .unit-empty {
        font-size: 0.82rem;
        color: var(--color-secondary);
        padding: 8px 2px;
        margin: 0;
    }

    /* ─── Create unit form ───────────────────────────────────────────────────── */

    .unit-create-form {
        border: 1px solid color-mix(in oklch, var(--color-base-300) 60%, transparent);
        border-radius: 0.5rem;
        padding: 12px;
        margin-bottom: 16px;
        background: color-mix(in oklch, var(--color-base-200) 30%, transparent);
    }

    .unit-create-title {
        font-size: 0.88rem;
        font-weight: 600;
        color: var(--color-base-content);
        margin: 0 0 4px 0;
    }

    .unit-create-desc {
        font-size: 0.8rem;
        color: var(--color-secondary);
        margin: 0 0 12px 0;
    }

    .unit-create-fields {
        margin-bottom: 12px;
    }

    .unit-create-row {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }

    .unit-create-field {
        display: flex;
        flex-direction: column;
        gap: 2px;
        flex: 1;
        min-width: 120px;
    }

    .unit-create-actions {
        display: flex;
        gap: 8px;
    }

    .unit-create-trigger {
        margin-bottom: 16px;
    }

    /* ─── Modal content ─────────────────────────────────────────────────────── */

    .modal-title {
        font-size: 0.95rem;
        font-weight: 600;
        color: var(--color-base-content);
        margin: 0 0 8px 0;
    }

    .modal-body {
        font-size: 0.85rem;
        color: var(--color-secondary);
        line-height: 1.5;
        margin: 0 0 16px 0;
    }

    /* ─── FEFO tooltip trigger ─────────────────────────────────────────────── */

    .fefo-tooltip-trigger {
        background: none;
        border: none;
        padding: 0 2px;
        cursor: help;
        vertical-align: middle;
        display: inline-flex;
        align-items: center;
        color: var(--color-secondary);
    }

    .fefo-tooltip-trigger:focus-visible {
        outline: 2px solid var(--color-primary);
        outline-offset: 2px;
        border-radius: 2px;
    }

    .fefo-tooltip-icon {
        width: 0.85em;
        height: 0.85em;
        pointer-events: none;
    }

    /* ─── FEFO selected-policy explanation ─────────────────────────────────── */

    .fefo-selected-explanation {
        font-size: 0.8rem;
        color: var(--color-primary);
        display: block;
        font-style: italic;
    }
</style>