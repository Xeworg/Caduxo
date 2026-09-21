<!--
  ReportsPage.svelte — reports configurator + preview
  (PR 8b forms + PR 8a.2 visual correction + PR 9a tables of
  caduxo-daisyui-redesign).

  Filter chrome migration to shared UI primitives:
    - Listbox.svelte for the store / location / urgency filter
      selects (PR 8a.2 visual correction — replaced
      `Select.svelte` because the underlying native `<select>`
      dropdown leaked OS-styled chrome on WebKit / Chromium).
      Empty-string semantics preserved (`storeId=""` represents
      "all stores", `locationId=""` represents "all locations",
      `urgency=""` represents "all urgencies"); the Listbox's
      first option (the "All …" entry) is rendered as the
      default so an empty bound value still shows visible copy.
    - Button.svelte for the preview / export / edit-filters actions.
    - Alert.svelte for the error + success banners (replaces the
      bespoke `.alert-error` / `.alert-success` divs).
    - The DatePicker wrapping (own component) and the CategoryPicker
      (own component) stay verbatim — PR 10 owns the DatePicker
      restyle and PR 9 owns the result table.
    - The `urgency radio group` mentioned in the PR 8b scope is the
      urgency filter select in `custom` mode; the page has no plain
      `<input type="radio">` controls to migrate.

  Result table migration (PR 9a):
    - Table.svelte (zebra, stickyHeader, scrollable) hosts the
      post-filter result rows; numeric columns use the `num`
      utility from PR 1.
    - EmptyState.svelte replaces the bespoke `.empty-state` block
      when no rows match the active filters.
    - The per-row urgency tinting (`.row-expired` / `.row-today` /
      `.row-alert` / `.row-soon`) is preserved on the migrated
      `<tr>` elements so the canonical Dashboard urgency UX
      transfers verbatim. The `.urgency-badge` chip stays inline
      (PR 9 does not migrate it to `Badge.svelte` — the page-level
      styling is unique to Reports).

  Tailwind classes referenced here (for the JIT scanner):
    btn btn-primary btn-secondary btn-ghost btn-lg
    alert alert-error alert-success alert-soft
    table table-zebra table-pin-rows
    overflow-x-auto
    dropdown dropdown-content
    flex items-center gap-2
-->
<script lang="ts">
  import { onMount } from "svelte";
  import DatePicker from "./DatePicker.svelte";
  import Listbox from "./ui/Listbox.svelte";
  import Button from "./ui/Button.svelte";
  import Alert from "./ui/Alert.svelte";
  import Table from "./ui/Table.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import Icon from "./ui/Icon.svelte";
  import {
    previewReport,
    exportReportPdfWithDialog,
    type ReportData,
    type ReportFilters,
    type ReportMetadata,
    type ReportRequest,
    type ReportType,
  } from "../lib/reports.js";
  import {
    listUnitDefinitions,
    type UnitDefinitionResponse,
  } from "../lib/unit_definitions.js";
  import {
    listCategories,
    type CategoryResponse,
  } from "../lib/products.js";
  import { UNCATEGORIZED_SENTINEL } from "../lib/categories.js";
  import CategoryPicker from "./inputs/CategoryPicker.svelte";
  import {
    listStores,
    listStoreLocations,
    type StoreResponse,
    type StoreLocationResponse,
  } from "../lib/stores.js";
  import { locale } from "../i18n/locale.svelte.js";
  import { LL } from "../i18n/i18n-svelte.js";
  import { humanizeError } from "../lib/errors.js";

  // ─── View state ──────────────────────────────────────────────────────────

  type View = "configure" | "preview";
  let view: View = "configure";

  let selectedReportType: ReportType = "expired";
  // Backing state for the Listbox primitive. The first option in each
  // option list is the "All …" entry with value="" so a `null` store /
  // location / urgency is represented as an empty string. We translate
  // back to `null` at the submit boundary.
  let storeId: string = "";
  let locationId: string = "";
  let categoryIds: string[] = [];
  let urgency: string = "";
  function todayIso(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  let dateFrom: string = "";
  let dateTo: string = "";

  let stores: StoreResponse[] = [];
  let locations: StoreLocationResponse[] = [];
  let categories: CategoryResponse[] = [];

  let preview: ReportData | null = null;
  let loading = false;
  let exporting = false;
  let errorMsg = "";
  let successMsg = "";

  /** Unit catalog for display-name resolution. */
  let unitCatalog: UnitDefinitionResponse[] = [];

  // ─── Report type options ─────────────────────────────────────────────────

  $: REPORT_TYPES = [
    {
      value: "in_alert_window" as ReportType,
      label: $LL.reports.reportTypes.inAlertWindow(),
      description: $LL.reports.reportTypes.inAlertWindowDesc(),
    },
    {
      value: "expired" as ReportType,
      label: $LL.reports.reportTypes.expired(),
      description: $LL.reports.reportTypes.expiredDesc(),
    },
    {
      value: "next_30_days" as ReportType,
      label: $LL.reports.reportTypes.next30Days(),
      description: $LL.reports.reportTypes.next30DaysDesc(),
    },
    {
      value: "custom" as ReportType,
      label: $LL.reports.reportTypes.custom(),
      description: $LL.reports.reportTypes.customDesc(),
    },
  ];

  $: URGENCY_OPTIONS = [
    { value: "", label: $LL.reports.urgencyOptions.all() },
    { value: "expired", label: $LL.reports.urgencyOptions.expired() },
    { value: "today", label: $LL.reports.urgencyOptions.today() },
    { value: "alert_window", label: $LL.reports.urgencyOptions.alertWindow() },
    { value: "next_30_days", label: $LL.reports.urgencyOptions.next30Days() },
    { value: "future", label: $LL.reports.urgencyOptions.future() },
  ];

  // ─── Derived select option lists ─────────────────────────────────────────

  $: storeOptions = [
    { value: "", label: $LL.dashboard.allStores() },
    ...stores.map((s) => ({ value: s.id, label: s.name })),
  ];

  $: locationOptions = [
    { value: "", label: $LL.dashboard.allLocations() },
    ...locations.map((loc) => ({ value: loc.id, label: loc.name })),
  ];

  // ─── Lifecycle ───────────────────────────────────────────────────────────

  onMount(async () => {
    try {
      const [storeList, categoryList] = await Promise.all([
        listStores().catch(() => []),
        listCategories().catch(() => []),
      ]);
      stores = storeList;
      categories = categoryList;
      unitCatalog = await listUnitDefinitions().catch(() => []);
    } catch (e: unknown) {
      errorMsg = humanizeError(e);
    }
  });

  function getUnitDisplayName(lot: ReportData["lots"][number]): string {
    if (lot.default_unit_id && lot.default_unit_id !== "") {
      const match = unitCatalog.find((u) => u.id === lot.default_unit_id);
      if (match) return match.display_name;
    }
    return lot.unit;
  }

  // When the store selection changes, refresh the location list. Empty
  // store id means "all stores" so we drop the location filter.
  $: if (storeId) {
    loadLocations(storeId);
  } else {
    locations = [];
    locationId = "";
  }

  async function loadLocations(forStoreId: string) {
    try {
      locations = await listStoreLocations(forStoreId);
      // Drop the previous location id if it belonged to a different store.
      if (locationId && !locations.find((l) => l.id === locationId)) {
        locationId = "";
      }
    } catch {
      locations = [];
      locationId = "";
    }
  }

  // ─── Preview ─────────────────────────────────────────────────────────────

  function buildFilters(): ReportFilters | null {
    const filters: ReportFilters = {
      store_id: storeId || null,
      location_id: locationId || null,
      category_ids: categoryIds.length > 0 ? categoryIds : null,
      urgency: selectedReportType === "custom" ? (urgency || null) : null,
      date_from: dateFrom.trim() || null,
      date_to: dateTo.trim() || null,
    };
    const allEmpty =
      !filters.store_id &&
      !filters.location_id &&
      !(filters.category_ids && filters.category_ids.length > 0) &&
      !filters.urgency &&
      !filters.date_from &&
      !filters.date_to;
    return allEmpty ? null : filters;
  }

  async function runPreview() {
    errorMsg = "";
    successMsg = "";
    loading = true;
    preview = null;
    try {
      const request: ReportRequest = {
        kind: selectedReportType,
        filters: buildFilters(),
      };
      preview = await previewReport(request, locale.current);
      view = "preview";
    } catch (e: unknown) {
      errorMsg = humanizeError(e);
    } finally {
      loading = false;
    }
  }

  async function exportPdf() {
    if (!preview) return;
    errorMsg = "";
    successMsg = "";
    exporting = true;
    try {
      const request: ReportRequest = {
        kind: selectedReportType,
        filters: buildFilters(),
      };
      const result = await exportReportPdfWithDialog(request, locale.current);
      if (result) {
        // Pluralization is handled by typesafe-i18n's inline plural
        // parts inside `reports.exportSuccess`, so we just hand over the
        // numeric counts — no caller-composed singular/plural words.
        successMsg = $LL.reports.exportSuccess({
          rows: result.rows_written,
          pages: result.page_count,
        });
        setTimeout(() => (successMsg = ""), 5000);
      }
    } catch (e: unknown) {
      errorMsg = humanizeError(e);
    } finally {
      exporting = false;
    }
  }

  function backToConfigure() {
    view = "configure";
    errorMsg = "";
    successMsg = "";
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────

  function formatDate(dateStr: string): string {
    // Delegate to the locale-aware typesafe-i18n `shortDate` formatter
    // exposed via `$LL.reports.table.shortDate`. Pass-through for empty
    // input so the table can render an empty cell without an error.
    if (!dateStr) return "";
    return $LL.reports.table.shortDate({ date: dateStr });
  }

  function formatQty(qty: number): string {
    if (qty === null || qty === undefined) return "";
    return $LL.reports.table.qtyFormatted({ qty });
  }

  function formatDays(days: number): string {
    if (days < 0) {
      return $LL.pdf.daysAgo({ n: Math.abs(days) });
    }
    return days.toString();
  }

  function urgencyLabel(u: string): string {
    switch (u) {
      case "expired": return $LL.dashboard.urgency.expired();
      case "today": return $LL.dashboard.urgency.today();
      case "alert_window": return $LL.dashboard.urgency.alertWindow();
      case "next_30_days": return $LL.dashboard.urgency.next30Days();
      default: return $LL.dashboard.urgency.future();
    }
  }

  function urgencyClass(u: string): string {
    switch (u) {
      case "expired": return "row-expired";
      case "today": return "row-today";
      case "alert_window": return "row-alert";
      case "next_30_days": return "row-soon";
      default: return "row-normal";
    }
  }

  function formatGeneratedAt(rfc3339: string): string {
    if (!rfc3339) return "";
    if (rfc3339.length >= 16 && rfc3339[10] === "T") {
      const date = rfc3339.substring(0, 10);
      const time = rfc3339.substring(11, 19);
      return `${date} ${time}`;
    }
    return rfc3339;
  }

      function filterSummary(meta: ReportMetadata | null): string {
        if (!meta) return "";
        const f = meta.filters_used;
        const parts: string[] = [];
        const storePrefix = $LL.reports.filterSummary.store().replace("=", "");
        const locPrefix = $LL.reports.filterSummary.location().replace("=", "");
        const urgencyPrefix = $LL.reports.filterSummary.urgency().replace("=", "");
        const fromPrefix = $LL.reports.filterSummary.from().replace("=", "");
        const toPrefix = $LL.reports.filterSummary.to().replace("=", "");
        if (f.store_id) parts.push(`${storePrefix}=${f.store_id.slice(0, 8)}…`);
        if (f.location_id) parts.push(`${locPrefix}=${f.location_id.slice(0, 8)}…`);
        if (f.category_ids && f.category_ids.length > 0) {
          const n = f.category_ids.length;
          if (f.category_ids.includes(UNCATEGORIZED_SENTINEL)) {
            parts.push($LL.reports.filterSummary.categoriesUnc({ n }));
          } else {
            parts.push($LL.reports.filterSummary.categories({ n }));
          }
        }
        if (f.urgency) parts.push(`${urgencyPrefix}=${f.urgency}`);
        if (f.date_from) parts.push(`${fromPrefix}=${f.date_from}`);
        if (f.date_to) parts.push(`${toPrefix}=${f.date_to}`);
        return parts.length === 0 ? $LL.reports.table.noFilters() : parts.join(" · ");
      }

  function reportTypeLabel(t: ReportType): string {
    return REPORT_TYPES.find((r) => r.value === t)?.label ?? t;
  }
</script>

<div class="page">
  <header class="page-header">
    <h1>{$LL.reports.pageTitle()}</h1>
    {#if view === "preview" && preview}
      <div class="page-header-actions">
        <Button variant="ghost" size="sm" onclick={backToConfigure}>
          {#snippet iconStart()}
            <Icon name="pencil-square" size="sm" />
          {/snippet}
          {$LL.reports.actions.editFilters()}
        </Button>
        <Button
          variant="primary"
          size="sm"
          onclick={exportPdf}
          disabled={exporting || preview.lots.length === 0}
          loading={exporting}
        >
          {#snippet iconStart()}
            <Icon name="document-arrow-down" size="sm" />
          {/snippet}
          {exporting ? $LL.reports.actions.exporting() : $LL.reports.actions.exportPdf()}
        </Button>
      </div>
    {/if}
  </header>

  {#if errorMsg}
    <Alert variant="error">{errorMsg}</Alert>
  {/if}
  {#if successMsg}
    <Alert variant="success" role="status">{successMsg}</Alert>
  {/if}

  {#if view === "configure"}
    <!-- ── Configure view ─────────────────────────────────────────── -->
    <section class="panel">
      <h2 class="panel-title">{$LL.reports.chooseReport()}</h2>
      <div class="report-types">
        {#each REPORT_TYPES as opt}
          <button
            type="button"
            class="report-type"
            class:selected={selectedReportType === opt.value}
            on:click={() => (selectedReportType = opt.value)}
          >
            <span class="report-type-label">{opt.label}</span>
            <span class="report-type-desc">{opt.description}</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="panel">
      <h2 class="panel-title">{$LL.reports.configureFilters()}</h2>
      <div class="filters-grid">
        <div class="filter-field">
          <label class="filter-label" for="reports-store">{$LL.dashboard.store()}</label>
          <Listbox
            bind:value={storeId}
            options={storeOptions}
            size="md"
            id="reports-store"
          />
        </div>

        <div class="filter-field" class:disabled={!storeId || locations.length === 0}>
          <label class="filter-label" for="reports-location">{$LL.dashboard.location()}</label>
          <Listbox
            bind:value={locationId}
            options={locationOptions}
            size="md"
            id="reports-location"
            disabled={!storeId || locations.length === 0}
          />
        </div>

        <div class="filter-field">
          <span class="filter-label">{$LL.dashboard.category()}</span>
          <CategoryPicker
            bind:value={categoryIds}
            {categories}
            includeUncategorized={true}
          />
        </div>

        <div class="filter-field" class:disabled={selectedReportType !== "custom"}>
          <label class="filter-label" for="reports-urgency">{$LL.reports.fields.urgency()}</label>
          <Listbox
            bind:value={urgency}
            options={URGENCY_OPTIONS}
            size="md"
            id="reports-urgency"
            disabled={selectedReportType !== "custom"}
          />
        </div>

        <div class="filter-field">
          <label class="filter-label" for="reports-date-from">{$LL.reports.fields.dateFrom()}</label>
          <DatePicker
            bind:value={dateFrom}
            id="reports-date-from"
            ariaLabel={$LL.reports.fields.dateFrom()}
            placeholder={$LL.reports.fields.datePlaceholder()}
            clearable={true}
            todayDate={todayIso()}
          />
        </div>

        <div class="filter-field">
          <label class="filter-label" for="reports-date-to">{$LL.reports.fields.dateTo()}</label>
          <DatePicker
            bind:value={dateTo}
            id="reports-date-to"
            ariaLabel={$LL.reports.fields.dateTo()}
            placeholder={$LL.reports.fields.datePlaceholder()}
            clearable={true}
            todayDate={todayIso()}
          />
        </div>
      </div>
    </section>

    <div class="actions-row">
      <Button
        variant="primary"
        size="lg"
        onclick={runPreview}
        disabled={loading}
        loading={loading}
      >
        {#snippet iconStart()}
          <Icon name="eye" size="sm" />
        {/snippet}
        {loading ? $LL.reports.actions.generating() : $LL.reports.actions.preview()}
      </Button>
    </div>
  {:else if preview}
    <!-- ── Preview view ────────────────────────────────────────────── -->
    <section class="panel report-meta-panel">
      <h2 class="panel-title">
        {reportTypeLabel(preview.metadata.report_type)}
        <span class="muted">— {preview.metadata.description}</span>
      </h2>
      <dl class="meta-grid">
        <dt>{$LL.reports.table.generatedAt()}</dt>
        <dd>{formatGeneratedAt(preview.metadata.generated_at)}</dd>
        <dt>{$LL.reports.table.rows()}</dt>
        <dd><strong>{preview.metadata.row_count}</strong></dd>
        <dt>{$LL.reports.table.filters()}</dt>
        <dd class="meta-filters">{filterSummary(preview.metadata)}</dd>
      </dl>
    </section>

    {#if preview.lots.length === 0}
      <EmptyState
        title={$LL.reports.emptyState.noRowsMatch()}
        body={$LL.reports.emptyState.adjustFilters()}
        icon="search"
      />
    {:else}
      <Table zebra stickyHeader scrollable aria-label={$LL.reports.table.reportRows()}>
        {#snippet head()}
          <tr>
            <th>{$LL.reports.table.sku()}</th>
            <th>{$LL.reports.table.description()}</th>
            <th>{$LL.reports.table.storeLocation()}</th>
            <th class="num">{$LL.reports.table.qty()}</th>
            <th>{$LL.reports.table.expiry()}</th>
            <th class="num">{$LL.reports.table.days()}</th>
            <th>{$LL.reports.table.urgency()}</th>
            <th>{$LL.reports.table.batch()}</th>
          </tr>
        {/snippet}
        {#snippet body()}
          {#each preview!.lots as lot (lot.lot_id)}
            <tr class={urgencyClass(lot.urgency)}>
              <td class="cell-sku">{lot.sku}</td>
              <td class="cell-desc">{lot.description}</td>
              <td class="cell-store">
                {lot.store_name}
                {#if lot.location_name}
                  <span class="loc-name">/ {lot.location_name}</span>
                {/if}
              </td>
              <td class="cell-qty num">{formatQty(lot.quantity)} {getUnitDisplayName(lot)}</td>
              <td class="cell-date num">{formatDate(lot.expiry_date)}</td>
              <td class="cell-days num">{formatDays(lot.days_remaining)}</td>
              <td>
                <span class="urgency-badge {urgencyClass(lot.urgency)}">
                  {urgencyLabel(lot.urgency)}
                </span>
              </td>
              <td class="cell-batch">{lot.batch_code ?? "—"}</td>
            </tr>
          {/each}
        {/snippet}
      </Table>
    {/if}
  {/if}
</div>

<style>
  .page {
    padding: 24px 32px;
    max-width: 1100px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }

  .page-header h1 {
    margin: 0;
    font-size: 1.5rem;
  }

  .page-header-actions {
    display: flex;
    gap: 8px;
  }

  .panel {
    background: var(--color-base-100);
    border: 1px solid color-mix(in oklch, var(--color-base-300) 70%, transparent);
    border-radius: 10px;
    padding: 20px 24px;
    margin-bottom: 16px;
  }

  .panel-title {
    margin: 0 0 12px;
    font-size: 1rem;
    color: var(--color-base-content);
  }

  .muted {
    color: var(--color-secondary);
    font-weight: 400;
  }

  /* ── Report type cards ────────────────────────────────────────────── */
  .report-types {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 10px;
  }

  .report-type {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    border: 1px solid color-mix(in oklch, var(--color-base-300) 80%, transparent);
    border-radius: 8px;
    background: var(--color-base-100);
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    transition: border-color 0.12s, background 0.12s;
    color: var(--color-base-content);
  }

  .report-type:hover {
    background: color-mix(in oklch, var(--color-base-200) 80%, transparent);
    border-color: color-mix(in oklch, var(--color-base-300) 90%, transparent);
  }

  .report-type.selected {
    background: color-mix(in oklch, var(--color-primary) 8%, transparent);
    border-color: var(--color-primary);
  }

  .report-type-label {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--color-base-content);
  }

  .report-type-desc {
    font-size: 0.78rem;
    color: var(--color-secondary);
    line-height: 1.4;
  }

  /* ── Filters ──────────────────────────────────────────────────────── */
  .filters-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 14px;
  }

  .filter-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .filter-label {
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--color-base-content);
  }

  .filter-field.disabled .filter-label {
    color: color-mix(in oklch, var(--color-base-content) 40%, transparent);
  }

  .actions-row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  /* ── Meta panel ───────────────────────────────────────────────────── */
  .report-meta-panel {
    background: color-mix(in oklch, var(--color-base-200) 70%, transparent);
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 4px 12px;
    margin: 0;
  }

  .meta-grid dt {
    font-size: 0.78rem;
    color: var(--color-secondary);
    font-weight: 500;
  }

  .meta-grid dd {
    margin: 0;
    font-size: 0.85rem;
    color: var(--color-base-content);
  }

  .meta-filters {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.78rem;
  }

  .cell-desc {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cell-store {
    white-space: nowrap;
  }

  .loc-name {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.78rem;
  }

  .cell-qty {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .cell-date {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .cell-days {
    font-variant-numeric: tabular-nums;
  }

  .cell-batch {
    color: color-mix(in oklch, var(--color-base-content) 50%, transparent);
    font-size: 0.8rem;
  }

  /* ── Urgency colours ──────────────────────────────────────────────── */
  .row-expired td {
    background: color-mix(in oklch, var(--color-error) 8%, transparent);
  }

  .row-expired:hover td {
    background: color-mix(in oklch, var(--color-error) 14%, transparent);
  }

  .row-today td {
    background: color-mix(in oklch, var(--color-warning) 14%, transparent);
  }

  .row-today:hover td {
    background: color-mix(in oklch, var(--color-warning) 20%, transparent);
  }

  .row-alert td {
    background: color-mix(in oklch, var(--color-info) 8%, transparent);
  }

  .row-alert:hover td {
    background: color-mix(in oklch, var(--color-info) 14%, transparent);
  }

  .row-soon td {
    background: color-mix(in oklch, var(--color-secondary) 10%, transparent);
  }

  .row-soon:hover td {
    background: color-mix(in oklch, var(--color-secondary) 16%, transparent);
  }

  .urgency-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.74rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .urgency-badge.row-expired {
    background: color-mix(in oklch, var(--color-error) 18%, transparent);
    color: var(--color-error);
  }

  .urgency-badge.row-today {
    background: color-mix(in oklch, var(--color-warning) 22%, transparent);
    color: var(--color-warning);
  }

  .urgency-badge.row-alert {
    background: color-mix(in oklch, var(--color-info) 18%, transparent);
    color: var(--color-info);
  }

  .urgency-badge.row-soon {
    background: color-mix(in oklch, var(--color-secondary) 18%, transparent);
    color: var(--color-secondary);
  }

  .urgency-badge.row-normal {
    background: color-mix(in oklch, var(--color-base-300) 50%, transparent);
    color: var(--color-secondary);
  }
</style>
