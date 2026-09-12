<script lang="ts">
  import { onDestroy } from "svelte";
  import StoresPage from "./components/StoresPage.svelte";
  import ProductCatalogPage from "./components/ProductCatalogPage.svelte";
  import DashboardPage from "./components/DashboardPage.svelte";
  import { startPeriodicNotificationCheck } from "./lib/notifications.js";

  type Tab = "dashboard" | "stores" | "products";
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
      Dashboard
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "stores"}
      on:click={() => (activeTab = "stores")}
    >
      Stores
    </button>
    <button
      class="nav-btn"
      class:active={activeTab === "products"}
      on:click={() => (activeTab = "products")}
    >
      Products
    </button>
  </nav>

  <!-- Views -->
  {#if activeTab === "dashboard"}
    <DashboardPage />
  {:else if activeTab === "stores"}
    <StoresPage />
  {:else if activeTab === "products"}
    <ProductCatalogPage />
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
