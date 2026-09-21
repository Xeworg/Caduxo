<!--
  ConfigurationPage.svelte — settings surface (PR 8a finish of
  caduxo-daisyui-redesign).

  PR 5 already migrated the language selector (Select), the
  location-required toggle (Toggle), and the theme switcher section
  (Select + Alert). PR 8a finishes the page chrome:
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
    select select-md select-error
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
    import Select from "./ui/Select.svelte";
    import Toggle from "./ui/Toggle.svelte";
    import Alert from "./ui/Alert.svelte";
    import Card from "./ui/Card.svelte";
    import LoadingState from "./ui/LoadingState.svelte";
    import {
        AVAILABLE_THEMES,
        setTheme,
        theme,
        type ThemeName,
    } from "./ui/theme/themeStore.svelte.js";

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

    let loading = true;
    let savingLocation = false;
    let settings: SettingsResponse | null = null;
    let errorMsg = "";
    let localeErrorMsg = "";

    // Local toggle value; updated optimistically on click.
    // Reverted if the save fails.
    let requireLocation: boolean = true;

    // Local locale value for the selector; kept in sync with the rune.
    let currentLocale: SupportedLocale = "en";

    // Theme switcher state. `switchingTheme` is set true during the
    // optimistic apply; it clears when the IPC call resolves or
    // rejects. `themeError` carries the human-readable error message
    // surfaced in the inline `Alert.svelte` when `setTheme` rejects.
    let switchingTheme = false;
    let themeError = "";

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
        } catch (e) {
            errorMsg = $LL.configuration.language.loadErrorPrefix() + humanizeError(e);
        } finally {
            loading = false;
        }
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
                  Select.svelte wraps the native <select>; the visible
                  surface is the DaisyUI `select select-md` shell while
                  keyboard / mobile OS sheet / screen-reader semantics
                  stay intact. `options` carries per-entry display
                  labels bound to the `configuration.language.names`
                  dictionary so adding a new locale only requires a
                  matching dictionary entry.
                -->
                <div class="setting-control">
                    <Select
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
                  `Select.svelte` list so the switcher inherits the
                  same keyboard / mobile sheet / screen-reader
                  semantics as the language picker. The currently
                  active theme is the selected value, and the
                  switcher is disabled while an IPC save is in
                  flight (`switchingTheme`).
                -->
                <div class="setting-control">
                    <Select
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
</style>