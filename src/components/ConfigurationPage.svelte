<script lang="ts">
    import { onMount } from "svelte";
    import {
        getSettings,
        updateSettings,
        type SettingsResponse,
    } from "../lib/stores.js";

    // ─── State ───────────────────────────────────────────────────────────────────

    let loading = true;
    let saving = false;
    let settings: SettingsResponse | null = null;
    let errorMsg = "";

    // Local toggle value; updated optimistically on click.
    // Reverted if the save fails.
    let requireLocation: boolean = true;

    // ─── Init ───────────────────────────────────────────────────────────────────

    onMount(async () => {
        try {
            settings = await getSettings();
            requireLocation = settings.require_initial_location_on_lot_create;
        } catch (e) {
            errorMsg = "No se pudieron cargar los ajustes: " + String(e);
        } finally {
            loading = false;
        }
    });

    // ─── Toggle handler ─────────────────────────────────────────────────────────

    async function handleToggle() {
        const newValue = !requireLocation;
        // Optimistic update
        requireLocation = newValue;
        errorMsg = "";
        saving = true;

        try {
            await updateSettings({
                require_initial_location_on_lot_create: newValue,
            });
            // Persist the confirmed value
            settings = { ...settings!, require_initial_location_on_lot_create: newValue };
        } catch (e) {
            // Rollback on failure
            requireLocation = !newValue;
            errorMsg = "Error al guardar el ajuste: " + String(e);
        } finally {
            saving = false;
        }
    }
</script>

<div class="page">
    <div class="page-header">
        <h1 class="page-title">Configuración</h1>
    </div>

    {#if loading}
        <p class="loading-msg">Cargando…</p>
    {:else if errorMsg && !settings}
        <p class="error-msg">{errorMsg}</p>
    {:else}
        <!-- ─── Lotes section ─────────────────────────────────────────────── -->
        <section class="settings-section">
            <h2 class="section-title">Lotes</h2>

            <div class="setting-row">
                <div class="setting-info">
                    <span class="setting-label">Ubicación inicial obligatoria al crear lote</span>
                    <span class="setting-desc">
                        Cuando está activado, el formulario de creación de lote requiere que se seleccione una ubicación.
                        Cuando está desactivado, se permite crear lotes sin ubicación (se asigna una ubicación predeterminada).
                    </span>
                </div>

                <label class="toggle-wrap" aria-label="Ubicación inicial obligatoria al crear lote">
                    <input
                        type="checkbox"
                        class="toggle-input"
                        checked={requireLocation}
                        disabled={saving}
                        on:change={handleToggle}
                    />
                    <span class="toggle-track">
                        <span class="toggle-thumb"></span>
                    </span>
                </label>
            </div>

            {#if saving}
                <p class="saving-msg">Guardando…</p>
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
    }

    .section-title {
        font-size: 0.95rem;
        font-weight: 600;
        color: #475569;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        margin: 0 0 16px 0;
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
