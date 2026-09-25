<script lang="ts">
  import { onMount, tick } from "svelte";
  import { LL } from "../i18n/i18n-svelte.js";
  import CalendarMonth from "./CalendarMonth.svelte";
  import CalendarYearGrid from "./CalendarYearGrid.svelte";
  import {
    listDashboardLots,
    type DashboardFilters,
    type DashboardLotRow,
  } from "../lib/dashboard.js";
  import {
    getExpiryLot,
    type ExpiryLotResponse,
  } from "../lib/expiry_lots.js";
  import { listStores, listStoreLocations, type StoreLocationResponse } from "../lib/stores.js";
  import { loadAllActiveStoreLocations, type EnrichedLocation } from "../lib/locations.js";
  import { resolveLocationDisplay } from "../lib/lotDisplay.js";
  import LotMovementsPanel from "./LotMovementsPanel.svelte";
  import { humanizeError } from "../lib/errors.js";
  import Table from "./ui/Table.svelte";
  import Badge from "./ui/Badge.svelte";
  import Button from "./ui/Button.svelte";
  import Modal from "./ui/Modal.svelte";
  import Tabs from "./ui/Tabs.svelte";
  import Tooltip from "./ui/Tooltip.svelte";
  import Icon from "./ui/Icon.svelte";

  // ── Helpers ─────────────────────────────────────────────────────────────────

  function todayIso(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  /** Format an ISO date as a locale-aware human-readable string. */
  function formatDate(iso: string): string {
    if (!iso) return "";
    const [y, m, day] = iso.split("-").map(Number);
    return new Date(y, m - 1, day).toLocaleDateString(undefined, {
      month: "long",
      day: "numeric",
      year: "numeric",
    });
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
      case "expired":   return $LL.calendar.urgencyLabels.expired();
      case "today":     return $LL.calendar.urgencyLabels.today();
      case "alert_window": return $LL.calendar.urgencyLabels.alertWindow();
      case "next_30_days": return $LL.calendar.urgencyLabels.next30Days();
      case "future":    return $LL.calendar.urgencyLabels.future();
      default:          return row.status;
    }
  }

  /**
   * Maps the backend urgency bucket (`alert_window | next_30_days | future`)
   * to the closed `BadgeUrgency` union consumed by the shared `Badge.svelte`
   * primitive. The mapping is local to this page so the dashboard / ledger
   * surfaces can keep their bespoke buckets; only the CalendarPage's
   * day-detail panel needs the Badge primitive today.
   */
  function urgencyToBadge(row: DashboardLotRow): "expired" | "today" | "alert" | "soon" | "normal" {
    switch (row.urgency) {
      case "expired":   return "expired";
      case "today":     return "today";
      case "alert_window": return "alert";
      case "next_30_days": return "soon";
      case "future":    return "normal";
      default:          return "normal";
    }
  }

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
  /**
   * Row context captured when `openLot(row)` is called. Provides
   * human-readable display values for the lot detail panel
   * (`description`, `sku`, `store_name`, `location_name`) without an
   * extra IPC round-trip — the row data already carries them from the
   * `listDashboardLots` fetch. Cleared alongside `detailLot` when the
   * modal closes.
   */
  let detailRow: DashboardLotRow | null = null;
  let detailLoading = false;
  let lotDetailTab: "detail" | "history" = "detail";
  let lotDetailLocations: EnrichedLocation[] = [];

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
          const message = humanizeError(e);
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
    // Close the year-picker overlay on any pointerdown outside the
    // year-nav toolbar. Mirrors CalendarMonth.svelte's
    // `onDocumentPointerDown` pattern (capture phase, scoped to the
    // toolbar via `[data-cal-year-nav]`).
    function onDocumentPointerDown(event: PointerEvent) {
      if (!showYearPicker) return;
      const target = event.target as Node | null;
      if (target instanceof Element && target.closest("[data-cal-year-nav]")) return;
      closeYearPicker();
    }
    document.addEventListener("pointerdown", onDocumentPointerDown, true);
    // Cleanup invalidates any outstanding load and stops the elapsed-seconds
    // ticker so a stale resolve cannot poke state on a torn-down component
    // (HMR, fast tab switch, unmount).
    return () => {
      loadGeneration++;
      stopElapsedTicker();
      document.removeEventListener("pointerdown", onDocumentPointerDown, true);
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

  // ── Year navigation (annual view) ───────────────────────────────────────────

  function prevYear() {
    if (showYearPicker) closeYearPicker();
    viewYear = Math.max(1900, viewYear - 1);
  }

  function nextYear() {
    if (showYearPicker) closeYearPicker();
    viewYear = Math.min(2100, viewYear + 1);
  }

  function goToday() {
    if (showYearPicker) closeYearPicker();
    viewYear = new Date().getFullYear();
    selectedDate = todayDate;
  }

  function onSelectDate(e: CustomEvent<string>) {
    selectedDate = e.detail;
  }

  // ── Year picker overlay (annual view) ──────────────────────────────────────
  // Modeled on CalendarMonth.svelte's year picker: clicking the year
  // label opens a decade picker; prev/next-decade chevrons jump by 10
  // years, clamped to 1900..2100. Selecting a year updates `viewYear`
  // and closes the picker without touching `selectedDate` (Today is the
  // only action that re-anchors the selection).

  let showYearPicker = false;
  let pickerYear: number = viewYear;

  $: decadeStart = Math.floor(pickerYear / 10) * 10;
  $: decadeYears = Array.from({ length: 10 }, (_, i) => decadeStart + i);

  function openYearPicker() {
    pickerYear = viewYear;
    showYearPicker = true;
  }

  function closeYearPicker() {
    showYearPicker = false;
  }

  function selectYear(year: number) {
    viewYear = year;
    showYearPicker = false;
  }

  function prevDecade() {
    pickerYear = Math.max(1900, pickerYear - 10);
  }

  function nextDecade() {
    pickerYear = Math.min(2100, pickerYear + 10);
  }

  function onYearPickerKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeYearPicker();
    }
  }

  // ── Lot detail ───────────────────────────────────────────────────────────────

  async function loadAllStoreLocations(): Promise<EnrichedLocation[]> {
    try {
      const result = await loadAllActiveStoreLocations();
      return result.allLocations;
    } catch {
      return [];
    }
  }

  async function openLot(row: DashboardLotRow) {
    detailLoading = true;
    showLotDetail = true;
    lotDetailTab = "history";
    detailLot = null;
    // Capture the row context so the detail panel can show
    // description / SKU / store name / location name without an
    // extra IPC round-trip.
    detailRow = row;
    lotDetailLocations = [];
    try {
      [detailLot, lotDetailLocations] = await Promise.all([
    getExpiryLot(row.lot_id),
    loadAllStoreLocations(),
      ]);
    } catch (e) {
      errorMsg = humanizeError(e);
      showLotDetail = false;
    } finally {
      detailLoading = false;
    }
  }

function onLotCancel() {
    showLotDetail = false;
    detailLot = null;
    detailRow = null;
  }

</script>

<div class="cal-page">
  <div class="cal-page-header">
    <h2 class="page-title">{$LL.calendar.pageTitle()}</h2>
    <Tooltip text={$LL.calendar.refreshAria()} position="bottom">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.calendar.refreshAria()}
        disabled={loading}
        onclick={loadLots}
      >
        {#snippet iconStart()}
          <Icon name="arrow-path" size="sm" />
        {/snippet}
        {$LL.calendar.refresh()}
      </Button>
    </Tooltip>
  </div>

  <!-- Page-level year navigation (owned by CalendarPage, not the grid primitive) -->
  <div
    class="cal-year-nav"
    data-cal-year-nav
    role="toolbar"
    aria-label={$LL.calendar.annualHeaderLabel({ year: viewYear })}
  >
    <Tooltip text={$LL.calendar.ariaPreviousYear()} position="bottom">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.calendar.ariaPreviousYear()}
        onclick={prevYear}
      >
        ‹
      </Button>
    </Tooltip>
    <Tooltip text={$LL.calendar.ariaOpenYearPicker()} position="bottom">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.calendar.ariaOpenYearPicker()}
        onclick={openYearPicker}
      >
        {viewYear}{#if !showYearPicker} ▼{/if}
      </Button>
    </Tooltip>
    <Tooltip text={$LL.calendar.ariaNextYear()} position="bottom">
      <Button
        variant="ghost"
        size="sm"
        aria-label={$LL.calendar.ariaNextYear()}
        onclick={nextYear}
      >
        ›
      </Button>
    </Tooltip>
    <span data-cal-nav-today>
      <Button
        variant="primary"
        size="sm"
        aria-label={$LL.calendar.today()}
        onclick={goToday}
      >
        {$LL.calendar.today()}
      </Button>
    </span>
    {#if showYearPicker}
      <div
        class="year-picker"
        role="dialog"
        aria-label={$LL.calendar.ariaOpenYearPicker()}
        tabindex="-1"
        onkeydown={onYearPickerKeydown}
      >
        <div class="year-picker-header">
          <Tooltip text={$LL.calendar.ariaPreviousDecade()} position="bottom">
            <Button
              variant="ghost"
              size="sm"
              aria-label={$LL.calendar.ariaPreviousDecade()}
              onclick={prevDecade}
            >
              ‹
            </Button>
          </Tooltip>
          <span class="decade-label">{decadeStart}–{decadeStart + 9}</span>
          <Tooltip text={$LL.calendar.ariaNextDecade()} position="bottom">
            <Button
              variant="ghost"
              size="sm"
              aria-label={$LL.calendar.ariaNextDecade()}
              onclick={nextDecade}
            >
              ›
            </Button>
          </Tooltip>
        </div>
        <div class="year-grid">
          {#each decadeYears as yr}
            {@const disabled = yr < 1900 || yr > 2100}
            <button
              type="button"
              class="year-chip"
              class:year-selected={yr === viewYear}
              class:year-disabled={disabled}
              aria-label={$LL.calendar.ariaYear({ year: yr })}
              aria-pressed={yr === viewYear}
              {disabled}
              onclick={() => !disabled && selectYear(yr)}
            >
              {yr}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

      {#if loading && lots.length === 0}
        <p class="loading-msg">{$LL.calendar.loadingLots()}</p>
        <p class="loading-diag" role="status" aria-live="polite" data-testid="cal-load-diag">
          Calendar load status: {loadStage}, attempt #{loadAttempts}{#if lastLoadStartedAt !== null && lastLoadFinishedAt === null}
            ({Math.max(0, Math.round((nowMs - lastLoadStartedAt) / 100) / 10)}s running){:else if lastLoadStartedAt !== null && lastLoadFinishedAt !== null}
            (took {Math.max(0, Math.round((lastLoadFinishedAt - lastLoadStartedAt) / 100) / 10)}s){/if}
        </p>
      {:else if errorMsg && lots.length === 0}
        <p class="error-msg">{$LL.calendar.loadFailed({ msg: errorMsg })}</p>
        <p class="loading-diag" role="status" aria-live="polite" data-testid="cal-load-diag">
          Calendar load status: {loadStage}, attempt #{loadAttempts}{#if lastLoadStartedAt !== null && lastLoadFinishedAt !== null}
            (failed after {Math.max(0, Math.round((lastLoadFinishedAt - lastLoadStartedAt) / 100) / 10)}s){/if}
        </p>
      {:else}
    <div class="cal-layout">
      <!-- Calendar grid -->
      <div class="cal-grid-wrapper">
        <CalendarYearGrid
          {viewYear}
          {todayDate}
          {selectedDate}
          {dayBadges}
          ariaLabel={$LL.calendar.annualHeaderLabel({ year: viewYear })}
          on:selectDate={onSelectDate}
        />
      </div>

      <!-- Day detail panel -->
      <div class="day-panel">
        <h3 class="day-panel-title">
          {$LL.calendar.dayPanelTitle({ date: formatDate(selectedDate) })}
          {#if dayRows.length > 0}
            <Badge semantic="info" size="sm">{dayRows.length}</Badge>
          {/if}
        </h3>

        {#if dayRows.length === 0}
          <p class="day-empty">{$LL.calendar.noLotsOnDate()}</p>
        {:else}
          <Table zebra stickyHeader aria-label={$LL.calendar.dayPanelTitle({ date: formatDate(selectedDate) })}>
            {#snippet head()}
              <tr>
                <th>{$LL.calendar.table.product()}</th>
                <th>{$LL.calendar.table.qty()}</th>
                <th>{$LL.calendar.table.unit()}</th>
                <th>{$LL.calendar.table.store()}</th>
                <th>{$LL.calendar.table.location()}</th>
                <th>{$LL.calendar.table.days()}</th>
                <th>{$LL.calendar.table.status()}</th>
              </tr>
            {/snippet}
            {#snippet body()}
              {#each dayRows as row (row.lot_id)}
                <tr>
                  <td>
                    <button
                      type="button"
                      class="lot-link"
                      onclick={() => openLot(row)}
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
                    <Badge urgency={urgencyToBadge(row)} size="sm">
                      {urgencyLabel(row)}
                    </Badge>
                  </td>
                </tr>
              {/each}
            {/snippet}
          </Table>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Lot edit overlay — reused Modal + Tabs primitives (PR 13 cleanup) -->
<Modal
  bind:open={showLotDetail}
  size="wide"
  showClose
  closeLabel={$LL.lotMovements.modal.close()}
  aria-label={$LL.dashboard.lotDetail()}
  oncancel={() => (showLotDetail = false)}
  onclose={onLotCancel}
>
  {#snippet children()}
    <header class="dialog-header">
      <h3>{$LL.dashboard.lotDetail()}</h3>
    </header>

    {#if detailLoading}
      <p class="modal-loading">{$LL.lotsDetail.loading()}</p>
    {:else if detailLot}
      <Tabs
        items={[
          {
            id: "detail",
            label: $LL.lotsDetail.detail(),
            panel: lotDetailTabPanel,
          },
          {
            id: "history",
            label: $LL.lotsDetail.history(),
            panel: lotHistoryTabPanel,
          },
        ]}
        bind:activeId={lotDetailTab}
        style="bordered"
        aria-label={$LL.dashboard.lotDetail()}
      />
    {/if}
  {/snippet}
</Modal>

{#snippet lotDetailTabPanel()}
  {#if detailLot}
    <dl class="detail-grid">
      <dt>{$LL.dashboard.product()}</dt>
      <dd>
        {#if detailRow?.description}
          {detailRow.description}
          <small class="detail-muted"> · {detailRow.sku}</small>
        {:else}
          {detailLot.product_id}
        {/if}
      </dd>
      <dt>{$LL.dashboard.store()}</dt>
      <dd>
        {#if detailRow?.store_name}
          {detailRow.store_name}
        {:else}
          {detailLot.store_id}
        {/if}
      </dd>
      <dt>{$LL.dashboard.location()}</dt>
      <dd>
        {#if detailLot.location_id}
          {resolveLocationDisplay(detailLot.location_id, lotDetailLocations, $LL.common.noLocation())}
        {:else}
          —
        {/if}
      </dd>
      <dt>{$LL.lotsDetail.quantity()}</dt>
      <dd>{detailLot.quantity}</dd>
      <dt>{$LL.dashboard.expiryDate()}</dt>
      <dd>{detailLot.expiry_date}</dd>
      <dt>{$LL.dashboard.alertDaysBefore()}</dt>
      <dd>{detailLot.alert_days_before}</dd>
      {#if detailLot.batch_code}
        <dt>{$LL.lotsDetail.batch()}</dt>
        <dd>{detailLot.batch_code}</dd>
      {/if}
    </dl>
    <div class="modal-actions">
      <Button variant="ghost" onclick={onLotCancel}>
        {$LL.common.close()}
      </Button>
      <Button variant="primary" onclick={() => (lotDetailTab = "history")}>
        {$LL.lotMovements.panelTitle()}
      </Button>
    </div>
  {/if}
{/snippet}

{#snippet lotHistoryTabPanel()}
  {#if detailLot}
    <div class="tab-content">
      <LotMovementsPanel
        lotId={detailLot.id}
        lotQuantity={detailLot.quantity}
        lotUnit={detailLot.unit}
        lotStatus={detailLot.status}
        locations={lotDetailLocations}
        allLocations={lotDetailLocations}
        unitType={detailLot.unit_type}
        onMovementCreated={async () => {
          // Reload modal and calendar data after a movement.
          detailLot = await getExpiryLot(detailLot!.id);
          await loadLots();
        }}
      />
    </div>
  {/if}
{/snippet}

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
    color: var(--color-base-content);
    margin: 0;
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

  /* ── Year navigation (annual view) ─────────────────────────────────────── */

  .cal-year-nav {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-bottom: 12px;
    padding: 4px 0;
    border-bottom: 1px solid var(--color-base-200);
    /* Position context so the absolute-positioned year picker overlay
       anchors below the nav row (mirrors `.cal-title { position: relative }`
       in CalendarMonth.svelte). */
    position: relative;
  }

  /* ── Year picker overlay (mirrors CalendarMonth.svelte's `.year-picker`
     visual vocabulary so the two pickers look like siblings) ───────── */

  .year-picker {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-200);
    border-radius: 8px;
    box-shadow: 0 4px 12px color-mix(in oklch, var(--color-base-content) 12%, transparent);
    padding: 8px;
    width: 220px;
  }

  .year-picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .decade-label {
    font-size: 0.8rem;
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    font-weight: 600;
  }

  .year-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 2px;
  }

  .year-chip {
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 4px 2px;
    border-radius: 4px;
    color: var(--color-base-content);
    transition: background 0.1s;
    text-align: center;
    font-family: inherit;
  }

  .year-chip:hover:not(.year-disabled) {
    background: color-mix(in oklch, var(--color-base-300) 50%, transparent);
  }

  .year-chip.year-selected {
    background: var(--color-primary);
    color: var(--color-base-100);
    border-color: var(--color-primary);
  }

  .year-chip.year-disabled {
    color: color-mix(in oklch, var(--color-base-content) 20%, transparent);
    cursor: not-allowed;
  }

  .year-chip:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  /* Responsive breakpoint — same threshold as App.svelte (max-width: 1024px) */
  @media (max-width: 1024px) {
    .cal-layout {
      flex-direction: column;
    }
    .day-panel {
      width: 100%;
    }
  }

  .day-panel {
    flex: 1;
    background: var(--color-base-100);
    border: 1px solid var(--color-base-200);
    border-radius: 12px;
    padding: 16px;
    min-width: 0;
  }

  .day-panel-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-base-content);
    margin: 0 0 12px 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .day-empty {
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    font-size: 0.9rem;
    text-align: center;
    padding: 20px 0;
  }

  /* Negative-days accent. The `num` utility (PR 1) and the
     table chrome come from `Table.svelte` + DaisyUI; only the
     per-cell tint stays local because it composes with the table
     primitive's `text-base-content` default. */
  .days-neg {
    color: var(--color-error);
  }

  .lot-link {
    background: none;
    border: none;
    padding: 0;
    color: var(--color-primary);
    cursor: pointer;
    font-size: 0.85rem;
    font-family: inherit;
    text-align: left;
    transition: color 0.15s;
  }

  .lot-link:hover {
    color: color-mix(in oklch, var(--color-primary) 80%, black);
    text-decoration: underline;
  }

  .lot-link:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* ── Loading / error ──────────────────────────────────────────────────── */

      .loading-msg,
      .error-msg {
        text-align: center;
        padding: 40px;
        color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
        font-size: 0.95rem;
      }

      .error-msg {
        color: var(--color-error);
      }

      /* Muted diagnostic line rendered under `Loading lots…` (and the error
         branch) so a persistent hang is distinguishable from an infinite
         spinner. Pulled up via `margin-top` to sit close to the headline. */
      .loading-diag {
        max-width: 600px;
        margin: -24px auto 40px;
        padding: 0 40px;
        text-align: center;
        color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
        font-size: 0.8rem;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      }

  /* ── Modal (PR 13: modal shell from `<Modal>` primitive; only inner
     panel styles remain) ────────────────────────────────────────────────── */

  .dialog-header {
    display: flex;
    align-items: center;
    margin-bottom: 16px;
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--color-base-content);
  }

  .modal-loading {
    text-align: center;
    padding: 20px;
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
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
    color: color-mix(in oklch, var(--color-base-content) 70%, transparent);
    font-weight: 500;
  }

  .detail-grid dd {
    color: var(--color-base-content);
    margin: 0;
  }

  /* SKU suffix shown after the description in the product cell so
     the description stays the primary text without losing the SKU
     context. */
  .detail-muted {
    color: color-mix(in oklch, var(--color-base-content) 60%, transparent);
    font-size: 0.82rem;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

</style>
