<script lang="ts">
  import { onMount, tick } from "svelte";
  import CalendarMonth from "./CalendarMonth.svelte";
  import {
    listDashboardLots,
    type DashboardFilters,
    type DashboardLotRow,
  } from "../lib/dashboard.js";
  import {
    getExpiryLot,
    resolveExpiryLot,
    type ExpiryLotResponse,
  } from "../lib/expiry_lots.js";
  import { listStoreLocations, type StoreLocationResponse } from "../lib/stores.js";
  import LotForm from "./LotForm.svelte";
  import LotMovementsPanel from "./LotMovementsPanel.svelte";

  // ── Helpers ─────────────────────────────────────────────────────────────────

  function todayIso(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  /** Format an ISO date as a human-readable string. */
  function formatDate(iso: string): string {
    if (!iso) return "";
    const [y, m, day] = iso.split("-").map(Number);
    const months = [
      "January", "February", "March", "April", "May", "June",
      "July", "August", "September", "October", "November", "December",
    ];
    return `${months[m - 1]} ${day}, ${y}`;
  }

  /** Days remaining until expiry_date (negative = expired). */
  function daysRemaining(expiryDate: string): number {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const exp = new Date(expiryDate + "T00:00:00");
    return Math.round((exp.getTime() - today.getTime()) / 86_400_000);
  }

  /** Status label for a lot row. */
  function urgencyLabel(row: DashboardLotRow): string {
    switch (row.urgency) {
      case "expired":   return "Expired";
      case "today":     return "Today";
      case "alert_window": return "Alert";
      case "next_30_days": return "Soon";
      case "future":    return "Future";
      default:          return row.status;
    }
  }

  function urgencyClass(row: DashboardLotRow): string {
    switch (row.urgency) {
      case "expired":   return "status-expired";
      case "today":     return "status-today";
      case "alert_window": return "status-alert";
      case "next_30_days": return "status-soon";
      case "future":    return "status-future";
      default:          return "status-unknown";
    }
  }

  // ── Resolve dialog (mirrors DashboardPage) ───────────────────────────────────

  const RESOLVE_TYPES = [
    { value: "consumed", label: "Consumed / Used" },
    { value: "sold", label: "Sold" },
    { value: "discarded", label: "Discarded" },
    { value: "donated", label: "Donated" },
    { value: "transferred", label: "Transferred" },
    { value: "other", label: "Other" },
  ];

      // ── State ───────────────────────────────────────────────────────────────────

      let loading = true;
      let errorMsg = "";
      let lots: DashboardLotRow[] = [];
      /**
       * Monotonic counter incremented on every loadLots invocation and on unmount.
       * Each load captures its generation at start and bails out if the captured
       * value no longer matches `loadGeneration` by the time the await resolves,
       * which prevents stale promises from mutating `lots` / `errorMsg` /
       * `loading` after a refresh click, fast remount, or component teardown.
       */
      let loadGeneration = 0;

      // ── Load diagnostics (in-UI surface for `Loading lots…` hang) ──────────
      // Browser `console.*` output runs inside the Tauri webview and is NOT
      // forwarded to the terminal that hosts `npm run tauri dev`. Without an
      // in-UI surface the user cannot tell whether the mount hook, the
      // `loadLots` call, or the underlying `listDashboardLots` invoke is the
      // failing stage. These variables render a muted line under
      // `Loading lots…` (and under the error branch when the 10 s timeout
      // fires) so the exact stage is reportable from the screenshot alone.
          let loadStage = "idle";
          let loadAttempts = 0;
          let lastLoadStartedAt: number | null = null;
          let lastLoadFinishedAt: number | null = null;

          // Live "now" used by the running-elapsed diagnostic. Updated by
          // `nowTickHandle` only while we are mid-load so the suffix
          // `({N.N}s running)` ticks up under `Loading lots…`. Independent
          // of the watchdog timeout below — its only job is cosmetic.
          //
          // NOTE: lifecycle is imperative (see `startElapsedTicker` /
          // `stopElapsedTicker`). The previous `$:` block watching
          // `loading && lots.length === 0` is removed because, under a
          // stalled Svelte reactive scheduler (HMR re-mount, WebView tick
          // starvation), the block can fail to re-run and the suffix stays
          // frozen at `0s running` even though the JS event loop is alive.
          // Driving the interval directly from `loadLots()` / the watchdog
          // / unmount guarantees it runs whenever a load is in flight,
          // regardless of Svelte's reactive flush state.
          let nowMs = Date.now();
          let nowTickHandle: number | null = null;

          function startElapsedTicker() {
            if (nowTickHandle !== null) return;
            nowMs = Date.now();
            nowTickHandle = window.setInterval(() => {
              // Top-level reassignment — Svelte should invalidate the
              // binding used by the diagnostic `<p>` tag on the next flush.
              nowMs = Date.now();
              // Defensive flush: if the microtask queue is starved we still
              // surface the suffix promptly. Cheap and idempotent.
              void tick();
            }, 250);
          }

          function stopElapsedTicker() {
            if (nowTickHandle !== null) {
              window.clearInterval(nowTickHandle);
              nowTickHandle = null;
            }
          }

      const todayDate: string = todayIso();
  let selectedDate: string = todayIso();
  let viewYear: number;
  let viewMonth: number;

  {
    const now = new Date();
    viewYear = now.getFullYear();
    viewMonth = now.getMonth() + 1;
  }

  // Day-detail panel
  let showLotDetail = false;
  let detailLot: ExpiryLotResponse | null = null;
  let detailLoading = false;
  let lotDetailTab: "detail" | "history" = "detail";
  let lotDetailLocations: StoreLocationResponse[] = [];

  // Resolve dialog (mirrors DashboardPage.resolve dialog state)
  let showResolveDialog = false;
  let resolveLot: DashboardLotRow | null = null;
  let resolveQty = 0;
  let resolveType = "consumed";
  let resolveNotes = "";
  let resolveLoading = false;
  let resolveError = "";

  // ── Derived ─────────────────────────────────────────────────────────────────

  /** dayBadges: YYYY-MM-DD → count of active lots expiring that day. */
  $: dayBadges = lots.reduce<Record<string, number>>((acc, lot) => {
    const k = lot.expiry_date;
    acc[k] = (acc[k] ?? 0) + 1;
    return acc;
  }, {});

  /** Rows for the selected day. */
  $: dayRows = lots.filter((l) => l.expiry_date === selectedDate);

      // ── Load ────────────────────────────────────────────────────────────────────

      /**
       * Per-attempt timeout in milliseconds. The watchdog below is the SOLE
       * timeout for the Calendar load. It is a parallel `window.setTimeout`
       * that mutates component state directly — not a `Promise.race` wrapper
       * around the invoke. This way, even if the underlying Tauri invoke
       * never settles AND any Promise.race plumbing happens to be broken, the
       * UI still transitions out of `loading` within this window.
       */
      const LOAD_TIMEOUT_MS = 10_000;

      async function loadLots() {
        // Capture the current generation; any later increment invalidates us.
        const myGen = ++loadGeneration;
        // Diagnostic counters — render a muted line so the persistent
        // `Loading lots…` screen distinguishes "mounted, awaiting invoke"
        // from "invoke timed out" in real time.
loadAttempts += 1;
        lastLoadStartedAt = Date.now();
        lastLoadFinishedAt = null;
        loadStage = "loading";
        // Imperatively start the elapsed-suffix ticker. Idempotent: if a
        // ticker is already running (refresh clicked mid-load) we keep it.
        startElapsedTicker();
        const filters: DashboardFilters = {
          store_id: null,
          location_id: null,
          preset: null,
          urgency: null,
        };
        // `console.log` (not `console.debug`) so DevTools shows the line by
        // default at the standard "Info" level. Browser console output is not
        // forwarded to the terminal — see the diagnostic state block above.
        console.log("[CalendarPage] loadLots start", filters);

        // Independent UI watchdog — runs in parallel with the Tauri invoke
        // and mutates component state DIRECTLY if it fires. This is a safety
        // net independent of any Promise.race / `withTimeout` wrapper: even
        // if the underlying `listDashboardLots` invoke never settles, this
        // timer will surface the failure within `LOAD_TIMEOUT_MS`. The stale
        // check (`myGen !== loadGeneration`) bails if a newer `loadLots()`
        // call has already taken over; the subsequent `loadGeneration++`
        // invalidates the in-flight invoke so its late resolve cannot
        // overwrite the timeout state we surface here.
const watchdog = window.setTimeout(() => {
          if (myGen !== loadGeneration) return;
          loadGeneration++;
          loadStage = "failed: timeout waiting for list_dashboard_lots";
          errorMsg =
            "Calendar load timed out after 10 seconds while waiting for list_dashboard_lots.";
          lastLoadFinishedAt = Date.now();
          loading = false;
          // Stop the elapsed ticker — the load is over from the UI's POV.
          stopElapsedTicker();
          // Belt-and-suspenders flush so the error/diagnostic line appears
          // promptly even if Svelte's microtask scheduling is delayed.
          void tick();
          console.error(
            "[CalendarPage] loadLots watchdog fired (10 s); invoke did not settle",
          );
        }, LOAD_TIMEOUT_MS);

        // We are the active generation; it is safe to flip the UI into loading.
        loading = true;
        if (myGen !== loadGeneration) {
          window.clearTimeout(watchdog);
          return;
        }
        errorMsg = "";

        try {
          const data = await listDashboardLots(filters);
          // A refresh click or unmount while we awaited makes us stale.
          window.clearTimeout(watchdog);
          if (myGen !== loadGeneration) return;
          lots = data.lots;
          loadStage = `loaded ${data.lots.length} lots`;
          console.log("[CalendarPage] loadLots success", { lots: data.lots.length });
        } catch (e) {
          window.clearTimeout(watchdog);
          if (myGen !== loadGeneration) return;
          console.error("[CalendarPage] loadLots failed", e);
          const message = e instanceof Error ? e.message : String(e);
          loadStage = `failed: ${message}`;
          errorMsg = message;
} finally {
          lastLoadFinishedAt = Date.now();
          // Only the live generation may release the loading flag; otherwise a
          // newer load is still in flight and must keep the spinner visible.
          if (myGen === loadGeneration) {
            loading = false;
            // We are the active load and we are done — stop the ticker so a
            // stale callback cannot keep `nowMs` mutating after the UI has
            // moved past the loading branch.
            stopElapsedTicker();
          }
        }
      }

  onMount(() => {
    // Mark the mounted stage before the first `loadLots` call so the
    // diagnostic line flips from `idle` to `mounted` even if the invoke
    // never resolves — that distinguishes "code never ran" from "code is
    // stuck awaiting the backend".
    loadStage = "mounted";
    console.log("[CalendarPage] mounted, dispatching loadLots");
    void loadLots();
    // Cleanup invalidates any outstanding load and stops the elapsed-seconds
    // ticker so a stale resolve cannot poke state on a torn-down component
    // (HMR, fast tab switch, unmount).
    return () => {
      loadGeneration++;
      stopElapsedTicker();
    };
  });

  // ── Navigation ──────────────────────────────────────────────────────────────

  function onMonthChange(e: CustomEvent<{ year: number; month: number }>) {
    viewYear = e.detail.year;
    viewMonth = e.detail.month;
  }

  function onViewYearChange(e: CustomEvent<number>) {
    viewYear = e.detail;
  }

  function onDaySelect(e: CustomEvent<string>) {
    selectedDate = e.detail;
  }

  // ── Lot detail ───────────────────────────────────────────────────────────────

  async function openLot(row: DashboardLotRow) {
    detailLoading = true;
    showLotDetail = true;
    lotDetailTab = "detail";
    detailLot = null;
    lotDetailLocations = [];
    try {
      [detailLot, lotDetailLocations] = await Promise.all([
    getExpiryLot(row.lot_id),
    listStoreLocations(row.store_id),
      ]);
    } catch (e) {
      errorMsg = String(e);
      showLotDetail = false;
    } finally {
      detailLoading = false;
    }
  }

function onLotCancel() {
    showLotDetail = false;
    detailLot = null;
  }

  // ── Resolve dialog handlers (mirrors DashboardPage) ──────────────────────────

  function openResolveFromDetail(detail: ExpiryLotResponse) {
    // Build a minimal DashboardLotRow from the ExpiryLotResponse for the resolve dialog.
    const row: DashboardLotRow = {
      lot_id: detail.id,
      product_id: detail.product_id,
      sku: "",       // resolved from product; not needed for resolve dialog
      description: "",
      store_id: detail.store_id,
      store_name: "",
      location_id: detail.location_id,
      location_name: null,
      quantity: detail.quantity,
      unit: detail.unit,
      expiry_date: detail.expiry_date,
      alert_days_before: detail.alert_days_before,
      batch_code: detail.batch_code,
      status: detail.status,
      urgency: "",
      days_remaining: 0,
      default_unit_id: null,
      unit_type: null,
    };
    resolveLot = row;
    resolveQty = 0;
    resolveType = "consumed";
    resolveNotes = "";
    resolveError = "";
    resolveLoading = false;
    showResolveDialog = true;
  }

  async function submitResolve() {
    if (!resolveLot) return;
    resolveError = "";
    if (resolveQty <= 0) {
      resolveError = "Quantity must be greater than zero.";
      return;
    }
    if (resolveQty > resolveLot.quantity) {
      resolveError = `Cannot resolve more than the remaining quantity (${resolveLot.quantity} ${resolveLot.unit}).`;
      return;
    }
    resolveLoading = true;
    try {
      await resolveExpiryLot({
        lot_id: resolveLot.lot_id,
        quantity: resolveQty,
        resolution: resolveType,
        notes: resolveNotes || null,
      });
      showResolveDialog = false;
      resolveLot = null;
      showLotDetail = false;
      detailLot = null;
      await loadLots();
    } catch (e) {
      resolveError = String(e);
    } finally {
      resolveLoading = false;
    }
  }

  function closeResolve() {
    showResolveDialog = false;
    resolveLot = null;
    resolveError = "";
    resolveLoading = false;
  }
</script>

<div class="cal-page">
  <div class="cal-page-header">
    <h2 class="page-title">Calendar</h2>
    <button
      type="button"
      class="btn-refresh"
      on:click={loadLots}
      disabled={loading}
      aria-label="Refresh lot data"
    >
      ⟳ Refresh
    </button>
  </div>

      {#if loading && lots.length === 0}
        <p class="loading-msg">Loading lots…</p>
        <p class="loading-diag" role="status" aria-live="polite" data-testid="cal-load-diag">
          Calendar load status: {loadStage}, attempt #{loadAttempts}{#if lastLoadStartedAt !== null && lastLoadFinishedAt === null}
            ({Math.max(0, Math.round((nowMs - lastLoadStartedAt) / 100) / 10)}s running){:else if lastLoadStartedAt !== null && lastLoadFinishedAt !== null}
            (took {Math.max(0, Math.round((lastLoadFinishedAt - lastLoadStartedAt) / 100) / 10)}s){/if}
        </p>
      {:else if errorMsg && lots.length === 0}
        <p class="error-msg">Failed to load lots: {errorMsg}</p>
        <p class="loading-diag" role="status" aria-live="polite" data-testid="cal-load-diag">
          Calendar load status: {loadStage}, attempt #{loadAttempts}{#if lastLoadStartedAt !== null && lastLoadFinishedAt !== null}
            (failed after {Math.max(0, Math.round((lastLoadFinishedAt - lastLoadStartedAt) / 100) / 10)}s){/if}
        </p>
      {:else}
    <div class="cal-layout">
      <!-- Calendar grid -->
      <div class="cal-grid-wrapper">
        <CalendarMonth
          {viewYear}
          {viewMonth}
          {selectedDate}
          {todayDate}
          {dayBadges}
          ariaLabel="Expiry calendar"
          on:daySelect={onDaySelect}
          on:monthChange={onMonthChange}
          on:viewYearChange={onViewYearChange}
        />
      </div>

      <!-- Day detail panel -->
      <div class="day-panel">
        <h3 class="day-panel-title">
          {formatDate(selectedDate)}
          {#if dayRows.length > 0}
            <span class="badge-count">{dayRows.length}</span>
          {/if}
        </h3>

        {#if dayRows.length === 0}
          <p class="day-empty">No expirations on {selectedDate}</p>
        {:else}
          <table class="day-table">
            <thead>
              <tr>
                <th>Product</th>
                <th>Qty</th>
                <th>Unit</th>
                <th>Store</th>
                <th>Location</th>
                <th>Days</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {#each dayRows as row (row.lot_id)}
                <tr>
                  <td>
                    <button
                      type="button"
                      class="lot-link"
                      on:click={() => openLot(row)}
                    >
                      {row.description || row.sku || row.product_id}
                    </button>
                  </td>
                  <td class="num">{row.quantity}</td>
                  <td>{row.unit}</td>
                  <td>{row.store_name}</td>
                  <td>{row.location_name ?? "—"}</td>
                  <td class="num" class:days-neg={row.days_remaining < 0}>
                    {row.days_remaining}
                  </td>
                  <td>
                    <span class="status-badge {urgencyClass(row)}">
                      {urgencyLabel(row)}
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Lot edit overlay — reusing the existing LotForm pattern -->
{#if showLotDetail}
  <div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Lot detail">
    <div class="modal-box modal-box-wide">
      <div class="modal-header">
        <h3>Lot Detail</h3>
        <button class="modal-close" on:click={onLotCancel}>✕</button>
      </div>
      {#if detailLoading}
        <p class="modal-loading">Loading…</p>
      {:else if detailLot}
        <!-- Tabs -->
        <div class="detail-tabs">
          <button
            type="button"
            class="tab-btn"
            class:active={lotDetailTab === "detail"}
            on:click={() => (lotDetailTab = "detail")}
          >
            Detalle
          </button>
          <button
            type="button"
            class="tab-btn"
            class:active={lotDetailTab === "history"}
            on:click={() => (lotDetailTab = "history")}
          >
            Historial
          </button>
        </div>

        {#if lotDetailTab === "detail"}
          <dl class="detail-grid">
            <dt>Product</dt>
            <dd>{detailLot.product_id}</dd>
            <dt>Store</dt>
            <dd>{detailLot.store_id}</dd>
            <dt>Location</dt>
            <dd>{detailLot.location_id ?? "—"}</dd>
            <dt>Quantity</dt>
            <dd>{detailLot.quantity}</dd>
            <dt>Expiry date</dt>
            <dd>{detailLot.expiry_date}</dd>
            <dt>Alert days</dt>
            <dd>{detailLot.alert_days_before}</dd>
            {#if detailLot.batch_code}
              <dt>Batch</dt>
              <dd>{detailLot.batch_code}</dd>
            {/if}
          </dl>
          <div class="modal-actions">
            <button
              type="button"
              class="btn-secondary"
              on:click={onLotCancel}
            >
              Close
            </button>
            <button
              type="button"
              class="btn-primary"
              on:click={() => openResolveFromDetail(detailLot!)}
            >
              Resolve quantity
            </button>
          </div>
        {:else}
          <!-- Historial tab -->
          <div class="tab-content">
            <LotMovementsPanel
              lotId={detailLot.id}
              lotQuantity={detailLot.quantity}
              lotUnit={detailLot.unit}
              lotStatus={detailLot.status}
              locations={lotDetailLocations}
              allLocations={lotDetailLocations}
              onMovementCreated={async () => {
                // Reload lot data after movement
                detailLot = await getExpiryLot(detailLot!.id);
              }}
            />
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}

    <!-- Resolve quantity dialog (mirrors DashboardPage) -->
    {#if showResolveDialog}
      <div class="modal-overlay" role="dialog" aria-modal="true" aria-label="Resolve quantity">
        <div class="modal-box">
          <div class="modal-header">
            <h3>Resolve Quantity</h3>
            <button class="modal-close" on:click={closeResolve}>✕</button>
          </div>
          {#if resolveLot}
            <div class="resolve-info">
              Remaining: <strong>{resolveLot.quantity} {resolveLot.unit}</strong>
              — Expires: <strong>{resolveLot.expiry_date}</strong>
            </div>

            <div class="form-group">
              <label for="cal-resolve-qty">Quantity to resolve</label>
              <input
                id="cal-resolve-qty"
                type="number"
                min="0.01"
                step="0.01"
                bind:value={resolveQty}
                disabled={resolveLoading}
              />
            </div>

            <div class="form-group">
              <label for="cal-resolve-type">Resolution type</label>
              <select id="cal-resolve-type" bind:value={resolveType} disabled={resolveLoading}>
                {#each RESOLVE_TYPES as rt}
                  <option value={rt.value}>{rt.label}</option>
                {/each}
              </select>
            </div>

            <div class="form-group">
              <label for="cal-resolve-notes">Notes (optional)</label>
              <textarea
                id="cal-resolve-notes"
                rows="2"
                bind:value={resolveNotes}
                disabled={resolveLoading}
              ></textarea>
            </div>

            {#if resolveError}
              <div class="error-inline" role="alert">{resolveError}</div>
            {/if}

            <div class="modal-actions">
              <button
                type="button"
                class="btn-secondary"
                on:click={closeResolve}
                disabled={resolveLoading}
              >
                Cancel
              </button>
              <button
                type="button"
                class="btn-primary"
                on:click={submitResolve}
                disabled={resolveLoading}
              >
                {resolveLoading ? "Saving…" : "Save"}
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <style>
  .cal-page {
    padding: 20px 24px;
    max-width: 1100px;
    margin: 0 auto;
  }

  .cal-page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }

  .page-title {
    font-size: 1.3rem;
    font-weight: 700;
    color: #1e293b;
    margin: 0;
  }

  .btn-refresh {
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 0.85rem;
    font-family: inherit;
    color: #2563eb;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-refresh:hover:not(:disabled) {
    background: #e2e8f0;
  }

  .btn-refresh:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── Layout ──────────────────────────────────────────────────────────────── */

  .cal-layout {
    display: flex;
    gap: 24px;
    align-items: flex-start;
  }

  .cal-grid-wrapper {
    flex-shrink: 0;
  }

  .day-panel {
    flex: 1;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 12px;
    padding: 16px;
    min-width: 0;
  }

  .day-panel-title {
    font-size: 1rem;
    font-weight: 600;
    color: #1e293b;
    margin: 0 0 12px 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .badge-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: #2563eb;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 700;
    width: 20px;
    height: 20px;
    border-radius: 50%;
  }

  .day-empty {
    color: #64748b;
    font-size: 0.9rem;
    text-align: center;
    padding: 20px 0;
  }

  /* ── Day table ─────────────────────────────────────────────────────────── */

  .day-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .day-table th {
    text-align: left;
    font-weight: 600;
    color: #64748b;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 4px 8px;
    border-bottom: 1px solid #e2e8f0;
  }

  .day-table td {
    padding: 6px 8px;
    border-bottom: 1px solid #f1f5f9;
    color: #1e293b;
    vertical-align: middle;
  }

  .day-table tr:last-child td {
    border-bottom: none;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .days-neg {
    color: #ef4444;
  }

  .lot-link {
    background: none;
    border: none;
    padding: 0;
    color: #2563eb;
    cursor: pointer;
    font-size: 0.85rem;
    font-family: inherit;
    text-align: left;
    transition: color 0.15s;
  }

  .lot-link:hover {
    color: #1d4ed8;
    text-decoration: underline;
  }

  .lot-link:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 2px;
  }

  /* ── Status badges ──────────────────────────────────────────────────────── */

  .status-badge {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 12px;
    font-size: 0.7rem;
    font-weight: 600;
  }

  .status-expired { background: #fee2e2; color: #b91c1c; }
  .status-today   { background: #fef3c7; color: #92400e; }
  .status-alert   { background: #fff7ed; color: #c2410c; }
  .status-soon    { background: #eff6ff; color: #1d4ed8; }
  .status-future  { background: #f0fdf4; color: #15803d; }
  .status-unknown { background: #f1f5f9; color: #475569; }

  /* ── Loading / error ──────────────────────────────────────────────────── */

      .loading-msg,
      .error-msg {
        text-align: center;
        padding: 40px;
        color: #64748b;
        font-size: 0.95rem;
      }

      .error-msg {
        color: #ef4444;
      }

      /* Muted diagnostic line rendered under `Loading lots…` (and the error
         branch) so a persistent hang is distinguishable from an infinite
         spinner. Pulled up via `margin-top` to sit close to the headline. */
      .loading-diag {
        max-width: 600px;
        margin: -24px auto 40px;
        padding: 0 40px;
        text-align: center;
        color: #94a3b8;
        font-size: 0.8rem;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      }

  /* ── Modal ─────────────────────────────────────────────────────────────── */

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal-box {
    background: #fff;
    border-radius: 12px;
    padding: 24px;
    width: 480px;
    max-width: 95vw;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-box-wide {
    width: 720px;
    max-width: 95vw;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .modal-header h3 {
    font-size: 1.1rem;
    font-weight: 700;
    color: #1e293b;
    margin: 0;
  }

  .modal-close {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    color: #64748b;
    padding: 4px 8px;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .modal-close:hover {
    background: #f1f5f9;
    color: #1e293b;
  }

  .modal-close:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 1px;
  }

  .modal-loading {
    text-align: center;
    padding: 20px;
    color: #64748b;
  }

  /* ── Detail tabs ────────────────────────────────────────────────────── */
  .detail-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
    border-bottom: 1px solid #e5e7eb;
  }

  .tab-btn {
    background: none;
    border: none;
    padding: 8px 16px;
    font-size: 0.88rem;
    cursor: pointer;
    color: #6b7280;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    font-family: inherit;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab-btn:hover {
    color: #374151;
  }

  .tab-btn.active {
    color: #2563eb;
    border-bottom-color: #2563eb;
    font-weight: 500;
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
    color: #64748b;
    font-weight: 500;
  }

  .detail-grid dd {
    color: #1e293b;
    margin: 0;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-primary {
    background: #2563eb;
    border: none;
    border-radius: 6px;
    padding: 7px 16px;
    font-size: 0.875rem;
    font-family: inherit;
    color: #fff;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-primary:hover {
    background: #1d4ed8;
  }

  .btn-primary:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 2px;
  }

  .btn-primary:disabled {
    background: #93c5fd;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: #fff;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 7px 16px;
    font-size: 0.875rem;
    font-family: inherit;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #f9fafb;
  }

  .btn-secondary:disabled {
    color: #9ca3af;
    cursor: not-allowed;
  }

  .btn-secondary:focus-visible {
    outline: 2px solid #2563eb;
    outline-offset: 2px;
  }

  /* ── Resolve dialog (mirrors DashboardPage) ───────────────────────────── */

  .resolve-info {
    background: #f8fafc;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    padding: 10px 12px;
    font-size: 0.85rem;
    color: #475569;
    margin-bottom: 16px;
    line-height: 1.6;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }

  .form-group label {
    font-size: 0.82rem;
    color: #374151;
    font-weight: 500;
  }

  .form-group input,
  .form-group select,
  .form-group textarea {
    border: 1px solid #d1d5db;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 0.85rem;
    font-family: inherit;
    background: #fff;
    color: #1e293b;
  }

  .form-group input:focus,
  .form-group select:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #2563eb;
    box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15);
  }

  .error-inline {
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 4px;
    padding: 6px 10px;
    color: #dc2626;
    font-size: 0.82rem;
    margin-bottom: 12px;
  }
</style>
