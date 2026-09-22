<!--
  StoresPage.svelte — store + location management (PR 9b of
  caduxo-daisyui-redesign).

  Migration to shared UI primitives:
    - Table.svelte (zebra) hosts both the store sidebar and the
      per-store location list. Each row is keyboard-activatable via
      tabindex="0" + Enter/Space; the Name cell renders a <button
      class="store-link"> so screen readers can announce the row
      action without depending on the row-level click handler.
    - Badge.svelte renders the active / inactive status cell.
    - EmptyState.svelte replaces the bespoke .empty-hint text in
      both the store sidebar and the location list (via the Table
      primitive's empty slot).
    - Button.svelte (variant ghost, size icon) replaces the
      bespoke .btn-icon edit button for locations.
    - The form blocks (store-form / location-form) and the first-run
      banner are deliberately left untouched — they are PR 8 scope.

  Tailwind classes referenced here (for the JIT scanner):
    table table-zebra
-->
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
  import { humanizeError } from "../lib/errors.js";
  import Table from "./ui/Table.svelte";
  import Badge from "./ui/Badge.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import Alert from "./ui/Alert.svelte";
  import Button from "./ui/Button.svelte";
  import Icon from "./ui/Icon.svelte";

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
      errorMsg = humanizeError(e);
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
    // Auto-select first store if none selected. Persist via
    // `updateSettings` so the backend scanner lookup (which reads
    // `last_selected_store_id` directly) has a store to use — the
    // previous behaviour left the persisted id empty and the Scanner
    // tab failed with "No active store selected" on the first scan.
    if (!selectedStoreId && stores.length > 0) {
      selectedStoreId = stores[0].id;
      await saveLastSelected(stores[0].id);
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
        // Persist the freshly created store as the active one so the
        // backend scanner lookup has a store to resolve against
        // (regression: previously the new store stayed visible in the
        // sidebar but `last_selected_store_id` stayed empty and the
        // Scanner tab kept failing with "No active store selected").
        await saveLastSelected(created.id);
        if (firstRun) firstRun = false;
        flash($LL.stores.storeCreated(), "success");
      }
          editingStore = null;
          storeFormOpen = false;
        } catch (e: unknown) {

      flash(humanizeError(e), "error");
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

      flash(humanizeError(e), "error");
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
    <Alert variant="error">{errorMsg}</Alert>
  {/if}
  {#if successMsg}
    <Alert variant="success">{successMsg}</Alert>
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
        <Table zebra aria-label={$LL.stores.pageTitle()}>
          {#snippet head()}
            <tr>
              <th>{$LL.stores.storeName()}</th>
              <th>{$LL.stores.storeCode()}</th>
              <th>{$LL.dashboard.status()}</th>
            </tr>
          {/snippet}
          {#snippet body()}
            {#each stores as store (store.id)}
              <tr
                class="store-row"
                class:active={store.id === selectedStoreId}
                tabindex="0"
                on:click={() => selectStore(store.id)}
                on:keydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    selectStore(store.id);
                  }
                }}
              >
                <td>
                  <button
                    type="button"
                    class="store-link"
                    on:click|stopPropagation={() => selectStore(store.id)}
                  >
                    {store.name}
                  </button>
                </td>
                <td>{store.code ?? "—"}</td>
                <td>
                  {#if store.is_active}
                    <Badge semantic="success" size="sm">{$LL.stores.active()}</Badge>
                  {:else}
                    <Badge semantic="neutral" size="sm">{$LL.stores.inactive()}</Badge>
                  {/if}
                </td>
              </tr>
            {/each}
          {/snippet}
          {#snippet empty()}
            <EmptyState
              title={$LL.stores.noStores()}
              icon="inbox"
            />
          {/snippet}
        </Table>
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
              <Table zebra aria-label={$LL.stores.internalLocations()}>
                {#snippet head()}
                  <tr>
                    <th>{$LL.stores.locationName()}</th>
                    <th>{$LL.stores.locationNotes()}</th>
                    <th>{$LL.dashboard.status()}</th>
                    <th>{$LL.common.actions()}</th>
                  </tr>
                {/snippet}
                {#snippet body()}
                  {#each locations as loc (loc.id)}
                    <tr class="location-row">
                      <td>{loc.name}</td>
                      <td class="notes">{loc.notes ?? ""}</td>
                      <td>
                        {#if loc.is_active}
                          <Badge semantic="success" size="sm">{$LL.stores.active()}</Badge>
                        {:else}
                          <Badge semantic="neutral" size="sm">{$LL.stores.inactive()}</Badge>
                        {/if}
                      </td>
                      <td>
                        <Button
                          variant="icon"
                          size="sm"
                          aria-label={$LL.stores.edit()}
                          onclick={() => startEditLocation(loc)}
                        >
                          {#snippet iconStart()}
                            <Icon name="pencil" size="sm" />
                          {/snippet}
                        </Button>
                      </td>
                    </tr>
                  {/each}
                {/snippet}
                {#snippet empty()}
                  <EmptyState
                    title={$LL.stores.noLocationsHint()}
                    icon="tag"
                  />
                {/snippet}
              </Table>

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

  /* ── Loading ───────────────────────────────────────────────────────────── */
  .loading {
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    font-style: italic;
  }

  /* ── First-run ─────────────────────────────────────────────────────────── */
  .first-run-card {
    background: var(--color-base-100);
    border: 1px solid var(--color-base-300);
    border-radius: 10px;
    padding: 32px;
    max-width: 520px;
  }

  .first-run-card h2 {
    margin: 0 0 8px;
    font-size: 1.4rem;
    color: var(--color-base-content);
  }

  .first-run-card p {
    margin: 0 0 20px;
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
  }

  /* ── Store layout ──────────────────────────────────────────────────────── */
  .stores-layout {
    display: grid;
    /* 340 px gives the store table enough room for the Spanish headers
       (Nombre de tienda / Código / Estado) plus the status badge without
       visually colliding at desktop widths. */
    grid-template-columns: 340px 1fr;
    gap: 20px;
    align-items: start;
  }

  /* Grid items default to min-width: auto, which prevents them from
     shrinking below the natural width of their content. The sidebar's
     Table.svelte contains long store names / codes that would push
     the column past the declared sidebar width; min-width: 0 lets the
     grid track keep its declared width. */
  .store-list {
    min-width: 0;
  }

  /* The Table primitive renders a native <table> (class table).
     Native tables auto-size to their content; force them to fill the
     sidebar and use table-layout: fixed with explicit column widths so
     the header labels cannot collide. The :global() is required because
     Svelte CSS scoping does not reach into the Table primitive's
     rendered HTML. */
  .store-list :global(table) {
    width: 100%;
    table-layout: fixed;
  }

  .store-list :global(table th:nth-child(1)),
  .store-list :global(table td:nth-child(1)) {
    width: 48%;
    overflow-wrap: anywhere;
  }
  .store-list :global(table th:nth-child(2)),
  .store-list :global(table td:nth-child(2)) {
    width: 22%;
    white-space: nowrap;
    text-align: right;
  }
  .store-list :global(table th:nth-child(3)),
  .store-list :global(table td:nth-child(3)) {
    width: 30%;
    white-space: nowrap;
  }

  /* ── Store sidebar (Table primitive hosts the table; only the
     row-level hover / focus / active states stay local because
     the Table primitive does not own row-state styling) ──────── */
  .store-row {
    cursor: pointer;
  }

  .store-row:hover {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
  }

  .store-row.active {
    background: color-mix(in oklch, var(--color-primary) 12%, transparent);
  }

  .store-row:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .store-link {
    background: none;
    border: none;
    padding: 0;
    color: inherit;
    cursor: pointer;
    font: inherit;
    text-align: left;
  }

  .store-link:hover {
    text-decoration: underline;
  }

  /* ── Store detail ──────────────────────────────────────────────────────── */
  .store-detail {
    background: var(--color-base-100);
    border: 1px solid var(--color-base-300);
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
    color: var(--color-base-content);
  }

  .detail-code {
    font-size: 0.8rem;
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    margin-left: 8px;
  }

  .detail-notes {
    margin: 6px 0 0;
    font-size: 0.85rem;
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
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
    color: var(--color-base-content);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    color: var(--color-base-content);
  }

  label input[type="text"],
  label textarea {
    padding: 7px 10px;
    border: 1px solid var(--color-base-300);
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: inherit;
    background: var(--color-base-100);
    color: var(--color-base-content);
  }

  label input:focus,
  label textarea:focus {
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

  /* ── Buttons ───────────────────────────────────────────────────────────── */
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

  .btn-primary:hover {
    background: color-mix(in oklch, var(--color-primary) 88%, black);
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

  .btn-secondary:hover {
    background: var(--color-base-200);
  }

  .btn-small {
    padding: 5px 12px;
    font-size: 0.82rem;
  }

  /* ── Locations ─────────────────────────────────────────────────────────── */
  .locations-section {
    border-top: 1px solid var(--color-base-200);
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
    color: var(--color-base-content);
  }

  /* Local styling for the per-cell muted notes column. The
     location-row wrapper is reserved for any future per-row state
     (e.g. hover / focus-visible) the migration may add; today only
     the .notes child needs local styling because the rest of the
     location list inherits from the Table primitive's DaisyUI
     chrome. */
  .location-row .notes {
    color: color-mix(in oklch, var(--color-base-content) 75%, transparent);
    font-size: 0.85rem;
  }

  .empty-hint {
    color: color-mix(in oklch, var(--color-base-content) 55%, transparent);
    font-size: 0.85rem;
    font-style: italic;
    margin: 0;
  }
</style>
