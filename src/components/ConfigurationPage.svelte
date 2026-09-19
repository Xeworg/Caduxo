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
        type SupportedLocale,
    } from "../i18n/locale.svelte.js";
    import { LL } from "../i18n/i18n-svelte.js";

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
            errorMsg = $LL.configuration.language.loadErrorPrefix() + String(e);
        } finally {
            loading = false;
        }
    });

    // ─── Locale selector handler ────────────────────────────────────────────────

    async function handleLocaleChange(next: SupportedLocale) {
        const prev = currentLocale;
        // Optimistic update
        currentLocale = next;
        localeErrorMsg = "";

        try {
            await setLocale(next);
            // Update the persisted settings reference; after a successful
            // `setLocale` the backend will report the language as configured.
            if (settings) {
                settings = {
                    ...settings,
                    language: next,
                    language_configured: true,
                };
            }
        } catch {
            // Roll back on failure
            currentLocale = prev;
            localeErrorMsg = $LL.configuration.language.saveErrorPrefix() + String("Failed");
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
            errorMsg = $LL.common.error() + ": " + String(e);
        } finally {
            savingLocation = false;
        }
    }
</script>

<div class="page">
    <div class="page-header">
        <h1 class="page-title">{$LL.configuration.pageTitle()}</h1>
    </div>

    {#if loading}
        <p class="loading-msg">{$LL.common.loading()}</p>
    {:else if errorMsg && !settings}
        <p class="error-msg">{errorMsg}</p>
    {:else}
        <!-- ─── Language section ────────────────────────────────────────────── -->
        <section class="settings-section">
            <h2 class="section-title">{$LL.configuration.language.sectionTitle()}</h2>

            <!-- Detected hint: shown only when the locale came from OS/browser detection -->
            {#if translationSource.current === "detected"}
                <p class="detected-hint">
                    {#if currentLocale === "es"}
                        {$LL.configuration.language.detectedHintEs()}
                    {:else}
                        {$LL.configuration.language.detectedHintEn()}
                    {/if}
                </p>
            {/if}

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">{$LL.configuration.language.label()}</span>
                </div>

                <select
                    class="locale-select"
                    bind:value={currentLocale}
                    on:change={(e) => handleLocaleChange(e.currentTarget.value as SupportedLocale)}
                    aria-label={$LL.configuration.language.label()}
                >
                    <option value="en">{$LL.configuration.language.english()}</option>
                    <option value="es">{$LL.configuration.language.spanish()}</option>
                </select>
            </div>

            {#if localeErrorMsg}
                <p class="error-msg">{localeErrorMsg}</p>
            {/if}
        </section>

        <!-- ─── Lotes section ─────────────────────────────────────────────── -->
        <section class="settings-section">
            <h2 class="section-title">{$LL.configuration.section.lots()}</h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">{$LL.configuration.locationRequired.label()}</span>
                    <span class="setting-desc">
                        {$LL.configuration.locationRequired.description()}
                    </span>
                </div>

                <label class="toggle-wrap" aria-label={$LL.configuration.locationRequired.label()}>
                    <input
                        type="checkbox"
                        class="toggle-input"
                        checked={requireLocation}
                        disabled={savingLocation}
                        on:change={handleToggle}
                    />
                    <span class="toggle-track">
                        <span class="toggle-thumb"></span>
                    </span>
                </label>
            </div>

            {#if savingLocation}
                <p class="saving-msg">{$LL.configuration.language.saving()}</p>
            {/if}
            {#if errorMsg}
                <p class="error-msg">{errorMsg}</p>
            {/if}
        </section>
    {/if}
</div>

<style>
    .page {
        max-width: 680px;
        margin: 0 auto;
        padding: 28px 24px;
    }

    .page-header {
        margin-bottom: 28px;
    }

    .page-title {
        font-size: 1.5rem;
        font-weight: 700;
        color: #1e293b;
        margin: 0;
    }

    .loading-msg {
        color: #64748b;
        font-size: 0.9rem;
    }

    /* ─── Section ──────────────────────────────────────────────────────────────── */

    .settings-section {
        background: #fff;
        border: 1px solid #e2e8f0;
        border-radius: 10px;
        padding: 20px 24px;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
        margin-bottom: 16px;
    }

    .section-title {
        font-size: 0.95rem;
        font-weight: 600;
        color: #475569;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        margin: 0 0 16px 0;
    }

    /* ─── Detected hint ──────────────────────────────────────────────────────── */

    .detected-hint {
        font-size: 0.82rem;
        color: #2563eb;
        background: #eff6ff;
        border: 1px solid #bfdbfe;
        border-radius: 6px;
        padding: 6px 12px;
        margin: 0 0 12px 0;
    }

    /* ─── Locale select ──────────────────────────────────────────────────────── */

    .locale-select {
        font-size: 0.9rem;
        padding: 6px 10px;
        border: 1px solid #cbd5e1;
        border-radius: 6px;
        background: #fff;
        color: #1e293b;
        cursor: pointer;
        min-width: 120px;
    }

    .locale-select:focus {
        outline: 2px solid #2563eb;
        outline-offset: 1px;
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
        color: #1e293b;
    }

    .setting-desc {
        font-size: 0.82rem;
        color: #64748b;
        line-height: 1.5;
    }

    /* ─── Toggle (CSS only, no Tailwind) ─────────────────────────────────────── */

    .toggle-wrap {
        display: flex;
        align-items: center;
        cursor: pointer;
        flex-shrink: 0;
    }

    .toggle-input {
        position: absolute;
        opacity: 0;
        width: 0;
        height: 0;
    }

    .toggle-track {
        display: block;
        width: 44px;
        height: 24px;
        border-radius: 12px;
        background: #cbd5e1;
        position: relative;
        transition: background 0.2s;
    }

    .toggle-input:checked + .toggle-track {
        background: #2563eb;
    }

    .toggle-input:disabled + .toggle-track {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .toggle-thumb {
        position: absolute;
        top: 2px;
        left: 2px;
        width: 20px;
        height: 20px;
        border-radius: 50%;
        background: #fff;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
        transition: left 0.2s;
    }

    .toggle-input:checked + .toggle-track .toggle-thumb {
        left: 22px;
    }

    /* ─── Status messages ─────────────────────────────────────────────────────── */

    .saving-msg {
        margin-top: 10px;
        font-size: 0.82rem;
        color: #64748b;
    }

    .error-msg {
        margin-top: 10px;
        font-size: 0.82rem;
        color: #dc2626;
        background: #fef2f2;
        border: 1px solid #fca5a5;
        border-radius: 6px;
        padding: 8px 12px;
    }
</style>
