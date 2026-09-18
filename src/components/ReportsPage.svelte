<script lang="ts">
  import { onMount } from "svelte";
  import DatePicker from "./DatePicker.svelte";
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
  import { locale } from "../i18n/locale.js";
  import { LL } from "../i18n/i18n-svelte.js";

  // ─── View state ──────────────────────────────────────────────────────────

  type View = "configure" | "preview";
  let view: View = "configure";

  let selectedReportType: ReportType = "expired";
  let storeId: string | null = null;
  let locationId: string | null = null;
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
      errorMsg = String(e);
    }
  });

  function getUnitDisplayName(lot: ReportData["lots"][number]): string {
    if (lot.default_unit_id && lot.default_unit_id !== "") {
      const match = unitCatalog.find((u) => u.id === lot.default_unit_id);
      if (match) return match.display_name;
    }
    return lot.unit;
  }

  $: if (storeId) {
    loadLocations(storeId);
  } else {
    locations = [];
    locationId = null;
  }

  async function loadLocations(forStoreId: string) {
    try {
      locations = await listStoreLocations(forStoreId);
      // Drop the previous location id if it belonged to a different store.
      if (locationId && !locations.find((l) => l.id === locationId)) {
        locationId = null;
      }
    } catch {
      locations = [];
      locationId = null;
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
      errorMsg = humanizeError(String(e));
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
        const rowWord = locale.current === "es"
          ? (result.rows_written === 1 ? "fila" : "filas")
          : (result.rows_written === 1 ? "row" : "rows");
        const pageWord = locale.current === "es"
          ? (result.page_count === 1 ? "página" : "páginas")
          : (result.page_count === 1 ? "page" : "pages");
        successMsg = $LL.reports.exportSuccess({
          rows: result.rows_written,
          rowWord,
          pages: result.page_count,
          pagesWord: pageWord,
        });
        setTimeout(() => (successMsg = ""), 5000);
      }
    } catch (e: unknown) {
      errorMsg = humanizeError(String(e));
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

  function humanizeError(raw: string): string {
    // Tauri command errors come back as "{ kind: 'validation', detail: { message: '...' } }".
    try {
      const parsed = JSON.parse(raw);
      if (parsed?.detail?.message) return parsed.detail.message;
      if (parsed?.message) return parsed.message;
    } catch {
      // not JSON, return raw
    }
    return raw;
  }

  function formatDate(dateStr: string): string {
    if (!dateStr || dateStr.length !== 10) return dateStr;
    const [y, m, d] = dateStr.split("-");
    return `${d}/${m}/${y}`;
  }

  function formatQty(qty: number): string {
    if (Number.isInteger(qty)) return qty.toString();
    return qty.toString();
  }

  function formatDays(days: number): string {
    if (days < 0) {
      const abs = Math.abs(days);
      if (locale.current === "es") {
        return $LL.pdf.daysAgo({ n: abs });
      }
      return `${abs} ago`;
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
    <h1>Reports</h1>
    {#if view === "preview" && preview}
      <div class="page-header-actions">
        <button class="btn-secondary" on:click={backToConfigure}>
          ← Edit filters
        </button>
        <button
          class="btn-primary"
          on:click={exportPdf}
          disabled={exporting || preview.lots.length === 0}
        >
          {exporting ? "Exporting…" : "Export PDF"}
        </button>
      </div>
    {/if}
  </header>

  {#if errorMsg}
    <div class="alert alert-error" role="alert">{errorMsg}</div>
  {/if}
  {#if successMsg}
    <div class="alert alert-success" role="status">{successMsg}</div>
  {/if}

  {#if view === "configure"}
    <!-- ── Configure view ─────────────────────────────────────────── -->
    <section class="panel">
      <h2 class="panel-title">1. Choose a report</h2>
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
      <h2 class="panel-title">2. Filters</h2>
      <div class="filters-grid">
        <label class="filter-field">
          <span>Store</span>
          <select bind:value={storeId}>
            <option value={null}>All stores</option>
            {#each stores as s}
              <option value={s.id}>{s.name}</option>
            {/each}
          </select>
        </label>

        <label class="filter-field" class:disabled={!storeId || locations.length === 0}>
          <span>Location</span>
          <select bind:value={locationId} disabled={!storeId || locations.length === 0}>
            <option value={null}>All locations</option>
            {#each locations as loc}
              <option value={loc.id}>{loc.name}</option>
            {/each}
          </select>
        </label>

        <div class="filter-field">
          <span>Category</span>
          <CategoryPicker
            bind:value={categoryIds}
            {categories}
            includeUncategorized={true}
            placeholder="Filter by category…"
          />
        </div>

        <label class="filter-field" class:disabled={selectedReportType !== "custom"}>
          <span>Urgency</span>
          <select
            bind:value={urgency}
            disabled={selectedReportType !== "custom"}
          >
            {#each URGENCY_OPTIONS as u}
              <option value={u.value}>{u.label}</option>
            {/each}
          </select>
        </label>

        <label class="filter-field">
          <span>Date from</span>
          <DatePicker
            bind:value={dateFrom}
            ariaLabel="Date from"
            placeholder="YYYY-MM-DD"
            clearable={true}
            todayDate={todayIso()}
          />
        </label>

        <label class="filter-field">
          <span>Date to</span>
          <DatePicker
            bind:value={dateTo}
            ariaLabel="Date to"
            placeholder="YYYY-MM-DD"
            clearable={true}
            todayDate={todayIso()}
          />
        </label>
      </div>
    </section>

    <div class="actions-row">
      <button
        class="btn-primary btn-large"
        on:click={runPreview}
        disabled={loading}
      >
        {loading ? "Generating preview…" : "Preview report"}
      </button>
    </div>
  {:else if preview}
    <!-- ── Preview view ────────────────────────────────────────────── -->
    <section class="panel report-meta-panel">
      <h2 class="panel-title">
        {reportTypeLabel(preview.metadata.report_type)}
        <span class="muted">— {preview.metadata.description}</span>
      </h2>
      <dl class="meta-grid">
        <dt>Generated at</dt>
        <dd>{formatGeneratedAt(preview.metadata.generated_at)}</dd>
        <dt>Rows</dt>
        <dd><strong>{preview.metadata.row_count}</strong></dd>
        <dt>Filters</dt>
        <dd class="meta-filters">{filterSummary(preview.metadata)}</dd>
      </dl>
    </section>

    {#if preview.lots.length === 0}
      <div class="empty-state">
        <p>No lots match the current report filters.</p>
        <p class="hint">Adjust the filters above and run the preview again.</p>
      </div>
    {:else}
      <div class="table-wrapper" role="region" aria-label="Report rows">
        <table class="report-table">
          <thead>
            <tr>
              <th>SKU</th>
              <th>Description</th>
              <th>Store / Location</th>
              <th class="num">Qty</th>
              <th>Expiry</th>
              <th class="num">Days</th>
              <th>Urgency</th>
              <th>Batch</th>
            </tr>
          </thead>
          <tbody>
            {#each preview.lots as lot (lot.lot_id)}
              <tr class={urgencyClass(lot.urgency)}>
                <td class="cell-sku">{lot.sku}</td>
                <td class="cell-desc">{lot.description}</td>
                <td class="cell-store">
                  {lot.store_name}
                  {#if lot.location_name}
                    <span class="loc-name">/ {lot.location_name}</span>
                  {/if}
                </td>
                <td class="cell-qty">{formatQty(lot.quantity)} {getUnitDisplayName(lot)}</td>
                <td class="cell-date">{formatDate(lot.expiry_date)}</td>
                <td class="cell-days">{formatDays(lot.days_remaining)}</td>
                <td>
                  <span class="urgency-badge {urgencyClass(lot.urgency)}">
                    {urgencyLabel(lot.urgency)}
                  </span>
                </td>
                <td class="cell-batch">{lot.batch_code ?? "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
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

  .panel {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 20px 24px;
    margin-bottom: 16px;
  }

  .panel-title {
    margin: 0 0 12px;
    font-size: 1rem;
    color: #1f2937;
  }

  .muted {
    color: #6b7280;
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
    border: 1px solid #d1d5db;
    border-radius: 8px;
    background: #fafafa;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
    transition: border-color 0.12s, background 0.12s;
  }

  .report-type:hover {
    background: #f3f4f6;
    border-color: #9ca3af;
  }

  .report-type.selected {
    background: #eff6ff;
    border-color: #2563eb;
  }

  .report-type-label {
    font-weight: 600;
    font-size: 0.9rem;
    color: #1f2937;
  }

  .report-type-desc {
    font-size: 0.78rem;
    color: #6b7280;
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

  .filter-field span {
    font-size: 0.78rem;
    font-weight: 500;
    color: #374151;
  }

  .filter-field.disabled span {
    color: #9ca3af;
  }

  .filter-field select {
    padding: 7px 10px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.85rem;
    font-family: inherit;
    background: #fff;
    color: #1e293b;
  }

  .filter-field select:focus {
    outline: none;
    border-color: #2563eb;
    box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.15);
  }

  .filter-field.disabled select {
    background: #f9fafb;
    color: #9ca3af;
    cursor: not-allowed;
  }

  .actions-row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  /* ── Meta panel ───────────────────────────────────────────────────── */
  .report-meta-panel {
    background: #f8fafc;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 4px 12px;
    margin: 0;
  }

  .meta-grid dt {
    font-size: 0.78rem;
    color: #6b7280;
    font-weight: 500;
  }

  .meta-grid dd {
    margin: 0;
    font-size: 0.85rem;
    color: #1f2937;
  }

  .meta-filters {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.78rem;
  }

  /* ── Empty state ──────────────────────────────────────────────────── */
  .empty-state {
    background: #fff;
    border: 1px dashed #d1d5db;
    border-radius: 10px;
    padding: 40px 20px;
    text-align: center;
  }

  .empty-state p {
    margin: 0 0 6px;
    color: #4b5563;
  }

  .empty-state .hint {
    font-size: 0.85rem;
    color: #9ca3af;
  }

  /* ── Table ────────────────────────────────────────────────────────── */
  .table-wrapper {
    overflow-x: auto;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
    background: #fff;
  }

  .report-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .report-table thead {
    background: #f8fafc;
  }

  .report-table th {
    text-align: left;
    padding: 8px 10px;
    font-weight: 600;
    color: #475569;
    border-bottom: 1px solid #e5e7eb;
    white-space: nowrap;
  }

  .report-table th.num,
  .report-table td.cell-qty,
  .report-table td.cell-days {
    text-align: right;
  }

  .report-table td {
    padding: 7px 10px;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: middle;
    color: #1e293b;
  }

  .report-table tr:last-child td {
    border-bottom: none;
  }

  .report-table tr:hover td {
    background: #f8fafc;
  }

  .cell-sku {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.8rem;
    color: #475569;
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
    color: #94a3b8;
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
    color: #94a3b8;
    font-size: 0.8rem;
  }

  /* ── Urgency colours ──────────────────────────────────────────────── */
  .row-expired td {
    background: #fff5f5;
  }

  .row-expired:hover td {
    background: #ffe4e4;
  }

  .row-today td {
    background: #fffbeb;
  }

  .row-today:hover td {
    background: #fef3c7;
  }

  .row-alert td {
    background: #f5faff;
  }

  .row-alert:hover td {
    background: #e0efff;
  }

  .row-soon td {
    background: #faf5ff;
  }

  .row-soon:hover td {
    background: #f3e8ff;
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
    background: #fee2e2;
    color: #991b1b;
  }

  .urgency-badge.row-today {
    background: #fef3c7;
    color: #92400e;
  }

  .urgency-badge.row-alert {
    background: #dbeafe;
    color: #1e40af;
  }

  .urgency-badge.row-soon {
    background: #ede9fe;
    color: #5b21b6;
  }

  .urgency-badge.row-normal {
    background: #f1f5f9;
    color: #475569;
  }

  /* ── Buttons ──────────────────────────────────────────────────────── */
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

  .btn-primary:disabled {
    background: #93c5fd;
    cursor: not-allowed;
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

  .btn-large {
    padding: 10px 22px;
    font-size: 0.95rem;
  }
</style>
