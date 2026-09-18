<script lang="ts">
  import { onDestroy } from "svelte";
  import StoresPage from "./components/StoresPage.svelte";
  import ProductCatalogPage from "./components/ProductCatalogPage.svelte";
  import DashboardPage from "./components/DashboardPage.svelte";
  import CsvImportPage from "./components/CsvImportPage.svelte";
  import ReportsPage from "./components/ReportsPage.svelte";
  import BackupRestorePage from "./components/BackupRestorePage.svelte";
  import CalendarPage from "./components/CalendarPage.svelte";
  import ConfigurationPage from "./components/ConfigurationPage.svelte";
  import { startPeriodicNotificationCheck } from "./lib/notifications.js";
  import { LL } from "./i18n/i18n-svelte.js";

  type Tab = "dashboard" | "stores" | "products" | "calendar" | "reports" | "import" | "backup" | "settings";
  let activeTab: Tab = "stores";

  // Slice 7b: start notification permission check and periodic polling.
  // The cleanup function is stable and safe to call from onDestroy.
  const stopPeriodicCheck = startPeriodicNotificationCheck();
  onDestroy(() => stopPeriodicCheck());
</script>

<div class="app-shell">
  <!-- Navigation -->
  <nav class="nav" aria-label="Main navigation">
    <span class="nav-brand">Caduxo</span>
    <button
      class="nav-btn"
      class:active={activeTab === "dashboard"}
      on:click={() => (activeTab = "dashboard")}
    >
      {$LL.nav.dashboard()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "stores"}
      on:click={() => (activeTab = "stores")}
    >
      {$LL.nav.stores()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "products"}
      on:click={() => (activeTab = "products")}
    >
      {$LL.nav.products()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "calendar"}
      on:click={() => (activeTab = "calendar")}
    >
      {$LL.nav.calendar()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "reports"}
      on:click={() => (activeTab = "reports")}
    >
      {$LL.nav.reports()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "import"}
      on:click={() => (activeTab = "import")}
    >
      {$LL.nav.import()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "backup"}
      on:click={() => (activeTab = "backup")}
    >
      {$LL.nav.backup()}
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "settings"}
      on:click={() => (activeTab = "settings")}
    >
      {$LL.nav.settings()}
    </button>
  </nav>

  <!-- Views -->
  {#if activeTab === "dashboard"}
    <DashboardPage />
  {:else if activeTab === "stores"}
    <StoresPage />
  {:else if activeTab === "products"}
    <ProductCatalogPage />
  {:else if activeTab === "calendar"}
    <CalendarPage />
  {:else if activeTab === "reports"}
    <ReportsPage />
  {:else if activeTab === "import"}
    <CsvImportPage />
  {:else if activeTab === "backup"}
    <BackupRestorePage />
  {:else if activeTab === "settings"}
    <ConfigurationPage />
  {/if}
</div>

<style>
  .app-shell {
    min-height: 100vh;
    background: #f6f8fb;
  }

  .nav {
    background: #1e293b;
    color: #f1f5f9;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 20px;
    height: 48px;
    position: sticky;
    top: 0;
    z-index: 100;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }

  .nav-brand {
    font-weight: 700;
    font-size: 1rem;
    margin-right: 24px;
    letter-spacing: 0.02em;
    color: #93c5fd;
  }

  .nav-btn {
    background: none;
    border: none;
    color: #94a3b8;
    font-size: 0.875rem;
    font-family: inherit;
    padding: 6px 14px;
    border-radius: 5px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .nav-btn:hover {
    background: #334155;
    color: #f1f5f9;
  }

  .nav-btn.active {
    background: #2563eb;
    color: #fff;
  }

</style>
