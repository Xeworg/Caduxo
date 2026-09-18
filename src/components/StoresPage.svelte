<script lang="ts">
  import { onMount } from "svelte";
  import {
    isFirstRun,
    listStores,
    createStore,
    updateStore,
    listStoreLocations,
    createStoreLocation,
    updateStoreLocation,
    getSettings,
    updateSettings,
    type StoreResponse,
    type StoreLocationResponse,
    type StoreCreate,
    type StoreUpdate,
    type SettingsResponse,
  } from "../lib/stores.js";
  import { LL } from "../i18n/i18n-svelte.js";

  // ── State ──────────────────────────────────────────────────────────────────

  let loading = true;
  let firstRun = false;
  let settings: SettingsResponse | null = null;
  let stores: StoreResponse[] = [];
  let selectedStoreId: string | null = null;
  let locations: StoreLocationResponse[] = [];

  let editingStore: StoreResponse | null = null;
  let editingLocation: StoreLocationResponse | null = null;

  let errorMsg = "";
  let successMsg = "";

  // Form fields
  let storeName = "";
  let storeCode = "";
  let storeNotes = "";
  let storeActive = true;
  let storeFormOpen = false;

  let locationName = "";
  let locationNotes = "";
  let locationActive = true;
  let locationFormOpen = false;

  // ── Derived ─────────────────────────────────────────────────────────────────

  $: selectedStore = stores.find((s) => s.id === selectedStoreId) ?? null;
  $: if (selectedStoreId) loadLocations(selectedStoreId);

  // ── Init ───────────────────────────────────────────────────────────────────

  onMount(async () => {
    try {
      firstRun = await isFirstRun();
      const s = await getSettings();
      settings = s;
      if (s.last_selected_store_id) {
        selectedStoreId = s.last_selected_store_id;
      }
      await refreshStores();
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      loading = false;
    }
  });

  // ── Helpers ────────────────────────────────────────────────────────────────

  function flash(msg: string, kind: "success" | "error") {
    if (kind === "success") {
      successMsg = msg;
      setTimeout(() => (successMsg = ""), 3000);
    } else {
      errorMsg = msg;
      setTimeout(() => (errorMsg = ""), 5000);
    }
  }

  async function refreshStores() {
    stores = await listStores();
    // Auto-select first store if none selected
    if (!selectedStoreId && stores.length > 0) {
      selectedStoreId = stores[0].id;
    }
  }

  async function loadLocations(storeId: string) {
    try {
      locations = await listStoreLocations(storeId);
    } catch {
      locations = [];
    }
  }

  function selectStore(id: string) {
    selectedStoreId = id;
    saveLastSelected(id);
  }

  async function saveLastSelected(id: string) {
    try {
      await updateSettings({ last_selected_store_id: id });
    } catch {
      // Non-critical — ignore
    }
  }

  function startCreateStore() {
    editingStore = null;
    storeName = "";
    storeCode = "";
    storeNotes = "";
    storeActive = true;
    storeFormOpen = true;
  }

  function startEditStore(store: StoreResponse) {
    editingStore = store;
    storeName = store.name;
    storeCode = store.code ?? "";
    storeNotes = store.notes ?? "";
    storeActive = store.is_active;
    storeFormOpen = true;
  }

  async function submitStore() {
    if (!storeName.trim()) {
      flash($LL.stores.storeNameRequired(), "error");
      return;
    }
    try {
      if (editingStore) {
        const updated = await updateStore({
          id: editingStore.id,
          name: storeName.trim(),
          code: storeCode.trim() || null,
          notes: storeNotes.trim() || null,
          is_active: storeActive,
        });
        stores = stores.map((s) => (s.id === updated.id ? updated : s));
        flash($LL.stores.storeUpdated(), "success");
      } else {
        const created = await createStore({
          name: storeName.trim(),
          code: storeCode.trim() || null,
          notes: storeNotes.trim() || null,
        });
        stores = [...stores, created];
        selectedStoreId = created.id;
        if (firstRun) firstRun = false;
        flash($LL.stores.storeCreated(), "success");
      }
          editingStore = null;
          storeFormOpen = false;
        } catch (e: unknown) {

      flash(String(e), "error");
    }
  }

  function cancelStoreForm() {
    editingStore = null;
    storeName = "";
    storeCode = "";
    storeNotes = "";
    storeActive = true;
    storeFormOpen = false;
  }

  function startCreateLocation() {
    editingLocation = null;
    locationName = "";
    locationNotes = "";
    locationActive = true;
    locationFormOpen = true;
  }

  function startEditLocation(loc: StoreLocationResponse) {
    editingLocation = loc;
    locationName = loc.name;
    locationNotes = loc.notes ?? "";
    locationActive = loc.is_active;
    locationFormOpen = true;
  }

  async function submitLocation() {
    if (!locationName.trim() || !selectedStoreId) return;
    try {
      if (editingLocation) {
        const updated = await updateStoreLocation({
          id: editingLocation.id,
          store_id: selectedStoreId,
          name: locationName.trim(),
          notes: locationNotes.trim() || null,
          is_active: locationActive,
        });
        locations = locations.map((l) => (l.id === updated.id ? updated : l));
        flash($LL.stores.locationUpdated(), "success");
      } else {
        const created = await createStoreLocation({
          store_id: selectedStoreId,
          name: locationName.trim(),
          notes: locationNotes.trim() || null,
        });
        locations = [...locations, created];
        flash($LL.stores.locationCreated(), "success");
      }
          editingLocation = null;
          locationFormOpen = false;
        } catch (e: unknown) {

      flash(String(e), "error");
    }
  }

      function cancelLocationForm() {
        editingLocation = null;
        locationName = "";
        locationNotes = "";
        locationActive = true;
        locationFormOpen = false;
      }

</script>

<!-- ── Layout ──────────────────────────────────────────────────────────────── -->

<div class="page">
  <!-- Header -->
  <header class="page-header">
    <h1>{$LL.stores.pageTitle()}</h1>
        {#if !editingStore && !storeFormOpen}
          <button class="btn-primary" on:click={startCreateStore}>

        + {$LL.stores.createStore()}
      </button>
    {/if}
  </header>

  <!-- Messages -->
  {#if errorMsg}
    <div class="alert alert-error" role="alert">{errorMsg}</div>
  {/if}
  {#if successMsg}
    <div class="alert alert-success" role="status">{successMsg}</div>
  {/if}

  {#if loading}
    <p class="loading">{$LL.stores.loading()}</p>
  {:else if firstRun}
    <!-- ── First-run banner ──────────────────────────────────────────────── -->
    <section class="first-run">
      <div class="first-run-card">
        <h2>{$LL.stores.welcomeTitle()}</h2>
        <p>{$LL.stores.firstRun.title()}</p>

        {#if storeFormOpen}
          <!-- Show store form for first run -->
          <form class="store-form" on:submit|preventDefault={submitStore}>
            <h3>{$LL.stores.firstRun.subtitle()}</h3>
            <label>
              {$LL.stores.storeName()} *
              <input
                type="text"
                bind:value={storeName}
                placeholder={$LL.stores.placeholders.storeName()}
                required
              />
            </label>
            <label>
              {$LL.stores.storeCode()} {$LL.common.optional()}
              <input
                type="text"
                bind:value={storeCode}
                placeholder={$LL.stores.placeholders.storeCode()}
              />
            </label>
            <label>
              {$LL.stores.storeNotes()} {$LL.common.optional()}
              <textarea
                bind:value={storeNotes}
                placeholder={$LL.stores.placeholders.storeNotes()}
                rows="2"
              ></textarea>
            </label>
            <div class="form-actions">
              <button type="submit" class="btn-primary">
                {editingStore ? $LL.stores.saveChanges() : $LL.stores.createStore()}
              </button>
              {#if editingStore}
                <button type="button" class="btn-secondary" on:click={cancelStoreForm}>
                  {$LL.common.cancel()}
                </button>
              {/if}
            </div>
          </form>
        {:else}
          <button class="btn-primary" on:click={startCreateStore}>
            + {$LL.stores.createStore()}
          </button>
        {/if}
      </div>
    </section>

  {:else}
    <!-- ── Store list + detail ───────────────────────────────────────────── -->
    <div class="stores-layout">
      <!-- Left: store list -->
      <aside class="store-list">
        {#each stores as store (store.id)}
          <button
            class="store-item"
            class:active={store.id === selectedStoreId}
            on:click={() => selectStore(store.id)}
          >
            <span class="store-name">{store.name}</span>
            {#if store.code}
              <span class="store-code">{store.code}</span>
            {/if}
            {#if !store.is_active}
              <span class="badge-inactive">{$LL.stores.inactive()}</span>
            {/if}
          </button>
        {/each}
        {#if stores.length === 0}
          <p class="empty-hint">{$LL.stores.noStores()}</p>
        {/if}
      </aside>

      <!-- Right: store detail / form -->
      <main class="store-detail">
{#if editingStore}
          <!-- Edit form -->
          <form class="store-form" on:submit|preventDefault={submitStore}>
            <h3>{$LL.stores.editStore()}</h3>
            <label>
              {$LL.stores.storeName()} *
              <input type="text" bind:value={storeName} required />
            </label>
            <label>
              {$LL.stores.storeCode()}
              <input type="text" bind:value={storeCode} />
            </label>
            <label>
              {$LL.stores.storeNotes()}
              <textarea bind:value={storeNotes} rows="2"></textarea>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={storeActive} />
              {$LL.stores.active()}
            </label>
            <div class="form-actions">
              <button type="submit" class="btn-primary">{$LL.stores.saveChanges()}</button>
              <button type="button" class="btn-secondary" on:click={cancelStoreForm}>
                {$LL.common.cancel()}
              </button>
            </div>
              </form>

            {:else if storeFormOpen}
              <form class="store-form" on:submit|preventDefault={submitStore}>
                <h3>{$LL.stores.createStore()}</h3>
                <label>
                  {$LL.stores.storeName()} *
                  <input type="text" bind:value={storeName} required />
                </label>
                <label>
                  {$LL.stores.storeCode()}
                  <input type="text" bind:value={storeCode} />
                </label>
                <label>
                  {$LL.stores.storeNotes()}
                  <textarea bind:value={storeNotes} rows="2"></textarea>
                </label>
                <div class="form-actions">
                  <button type="submit" class="btn-primary">{$LL.stores.createStore()}</button>
                  <button type="button" class="btn-secondary" on:click={cancelStoreForm}>
                    {$LL.common.cancel()}
                  </button>
                </div>
              </form>

            {:else if selectedStore}
              <!-- Store detail header -->
          <div class="detail-header">
            <div>
              <h2>{selectedStore.name}</h2>
              {#if selectedStore.code}<span class="detail-code">{selectedStore.code}</span>{/if}
              {#if !selectedStore.is_active}<span class="badge-inactive">{$LL.stores.inactive()}</span>{/if}
              {#if selectedStore.notes}<p class="detail-notes">{selectedStore.notes}</p>{/if}
            </div>
            <div class="detail-actions">
              <button class="btn-secondary" on:click={() => startEditStore(selectedStore!)}>
                {$LL.stores.edit()}
              </button>
            </div>
          </div>

          <!-- Locations section -->
          <section class="locations-section">
            <div class="section-header">
              <h3>{$LL.stores.internalLocations()}</h3>
              {#if !editingLocation && !locationFormOpen}
                <button class="btn-small" on:click={startCreateLocation}>
                  + {$LL.stores.createLocation()}
                </button>
              {/if}
            </div>

            {#if !locationFormOpen}
              <!-- Location list -->
              {#if locations.length === 0}
                <p class="empty-hint">{$LL.stores.noLocationsHint()}</p>
              {:else}
                <ul class="location-list">
                  {#each locations as loc (loc.id)}
                    <li class="location-item">
                      <div class="location-info">
                        <span class="location-name">{loc.name}</span>
                        {#if loc.notes}<span class="location-notes">{loc.notes}</span>{/if}
                        {#if !loc.is_active}<span class="badge-inactive">{$LL.stores.inactive()}</span>{/if}
                      </div>
                      <button
                        class="btn-icon"
                        title={$LL.stores.edit()}
                        on:click={() => startEditLocation(loc)}
                      >
                        ✏️
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}

            {:else}
              <!-- Location form -->
              <form class="location-form" on:submit|preventDefault={submitLocation}>
                <label>
                  {$LL.stores.locationName()} *
                  <input
                    type="text"
                    bind:value={locationName}
                    placeholder={$LL.stores.placeholders.locationName()}
                    required
                  />
                </label>
                <label>
                  {$LL.stores.locationNotes()}
                  <textarea bind:value={locationNotes} rows="1" placeholder={$LL.stores.placeholders.locationNotes()}></textarea>
                </label>
                <label class="checkbox-label">
                  <input type="checkbox" bind:checked={locationActive} />
                  {$LL.stores.active()}
                </label>
                <div class="form-actions">
                  <button type="submit" class="btn-primary btn-small">
                    {editingLocation ? $LL.stores.saveChanges() : $LL.stores.createLocation()}
                  </button>
                  <button type="button" class="btn-secondary btn-small" on:click={cancelLocationForm}>
                    {$LL.common.cancel()}
                  </button>
                </div>
              </form>
            {/if}
          </section>

        {:else}
          <p class="empty-hint">{$LL.stores.selectToManage()}</p>
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  /* ── Layout ─────────────────────────────────────────────────────────────── */
  .page {
    padding: 24px 32px;
    max-width: 1100px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 20px;
  }

  .page-header h1 {
    margin: 0;
    font-size: 1.5rem;
  }

  /* ── Alerts ────────────────────────────────────────────────────────────── */
  .alert {
    padding: 10px 14px;
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 0.9rem;
  }

  .alert-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
  }

  .alert-success {
    background: #dcfce7;
    color: #166534;
    border: 1px solid #86efac;
  }

  /* ── Loading ───────────────────────────────────────────────────────────── */
  .loading {
    color: #6b7280;
    font-style: italic;
  }

  /* ── First-run ─────────────────────────────────────────────────────────── */
  .first-run-card {
    background: #fff;
    border: 1px solid #d1d5db;
    border-radius: 10px;
    padding: 32px;
    max-width: 520px;
  }

  .first-run-card h2 {
    margin: 0 0 8px;
    font-size: 1.4rem;
  }

  .first-run-card p {
    margin: 0 0 20px;
    color: #4b5563;
  }

  /* ── Store layout ──────────────────────────────────────────────────────── */
  .stores-layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 20px;
    align-items: start;
  }

  /* ── Store list sidebar ────────────────────────────────────────────────── */
  .store-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .store-item {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 7px;
    padding: 10px 12px;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .store-item:hover {
    background: #f9fafb;
  }

  .store-item.active {
    background: #eff6ff;
    border-color: #3b82f6;
  }

  .store-name {
    font-weight: 500;
    font-size: 0.9rem;
  }

  .store-code {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .badge-inactive {
    font-size: 0.7rem;
    background: #f3f4f6;
    color: #9ca3af;
    border-radius: 4px;
    padding: 1px 5px;
    align-self: flex-start;
  }

  /* ── Store detail ──────────────────────────────────────────────────────── */
  .store-detail {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 24px;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 24px;
    gap: 12px;
  }

  .detail-header h2 {
    margin: 0 0 4px;
    font-size: 1.2rem;
  }

  .detail-code {
    font-size: 0.8rem;
    color: #6b7280;
    margin-left: 8px;
  }

  .detail-notes {
    margin: 6px 0 0;
    font-size: 0.85rem;
    color: #4b5563;
  }

  .detail-actions {
    flex-shrink: 0;
  }

  /* ── Forms ─────────────────────────────────────────────────────────────── */
  form.store-form,
  form.location-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 440px;
  }

  form h3 {
    margin: 0 0 4px;
    font-size: 1rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    color: #374151;
  }

  label input[type="text"],
  label textarea {
    padding: 7px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
  }

  label input:focus,
  label textarea:focus {
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

  /* ── Buttons ───────────────────────────────────────────────────────────── */
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

  .btn-primary:hover {
    background: #1d4ed8;
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

  .btn-secondary:hover {
    background: #f9fafb;
  }

  .btn-small {
    padding: 5px 12px;
    font-size: 0.82rem;
  }

  .btn-icon {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    padding: 2px 6px;
  }

  /* ── Locations ─────────────────────────────────────────────────────────── */
  .locations-section {
    border-top: 1px solid #f3f4f6;
    padding-top: 20px;
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

  .location-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .location-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: #f9fafb;
    border-radius: 6px;
    gap: 8px;
  }

  .location-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .location-name {
    font-size: 0.9rem;
    font-weight: 500;
  }

  .location-notes {
    font-size: 0.78rem;
    color: #6b7280;
  }

  .empty-hint {
    color: #9ca3af;
    font-size: 0.85rem;
    font-style: italic;
    margin: 0;
  }
</style>
