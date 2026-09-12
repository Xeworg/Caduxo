<script lang="ts">
  import StoresPage from "./components/StoresPage.svelte";

  type Tab = "dashboard" | "stores";
  let activeTab: Tab = "stores";
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
  </nav>

  <!-- Views -->
  {#if activeTab === "dashboard"}
    <main class="shell">
      <section class="hero">
        <p class="eyebrow">Caduxo</p>
        <h1>Product expiry tracking</h1>
        <p>
          Local-first desktop app for tracking product expiry lots by SKU, barcode,
          store, alert window, and reports.
        </p>
      </section>

      <section class="cards" aria-label="Dashboard preview">
        <article>
          <span>Expired</span>
          <strong>—</strong>
        </article>
        <article>
          <span>Today</span>
          <strong>—</strong>
        </article>
        <article>
          <span>Alert window</span>
          <strong>—</strong>
        </article>
        <article>
          <span>Next 30 days</span>
          <strong>—</strong>
        </article>
      </section>

      <p class="coming-soon">Dashboard, scanner, and reports are coming in later slices.</p>
    </main>

  {:else if activeTab === "stores"}
    <StoresPage />
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

  /* Dashboard placeholder styles */
  .shell {
    min-height: calc(100vh - 48px);
    padding: 48px;
  }

  .hero {
    max-width: 760px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 16px;
    margin-top: 32px;
  }

  .cards article {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .cards article span {
    font-size: 0.8rem;
    color: #6b7280;
  }

  .cards article strong {
    font-size: 1.8rem;
    color: #1e293b;
    margin-top: 4px;
  }

  .coming-soon {
    margin-top: 24px;
    color: #9ca3af;
    font-size: 0.85rem;
    font-style: italic;
  }

  .eyebrow {
    color: #2563eb;
    font-weight: 600;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin: 0 0 4px;
  }

  h1 {
    margin: 0 0 12px;
    font-size: 1.6rem;
    color: #0f172a;
  }

  p {
    color: #475569;
    margin: 0;
    line-height: 1.6;
  }
</style>
