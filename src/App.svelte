<!--
  App.svelte — top-level shell (PR 5 of caduxo-daisyui-redesign).

  Migrates the bespoke .nav / .nav-btn / .nav-brand markup to a
  DaisyUI navbar bg-base-200 with navbar-start (Caduxo brand),
  navbar-center (tab buttons rendered as DaisyUI ghost buttons so
  they pick up aria-current="page"), and navbar-end (a ≤720 px
  dropdown that holds the overflow tabs so every tab remains
  reachable via keyboard on small viewports).

  Renders the dashboard gradient band
  (linear-gradient from primary 5% → base-100) behind the brand
  area on the Dashboard landing tab only, per design §5.9.

  Tailwind classes referenced here (for the JIT scanner):
    navbar navbar-start navbar-center navbar-end bg-base-200
    dropdown dropdown-end dropdown-content
    btn btn-ghost btn-sm btn-square
    menu menu-sm rounded-box w-56 shadow
    motion-reduce:transition-none
-->
<script lang="ts">
  import { onDestroy } from "svelte";
  import caduxoMark from "./assets/caduxo-mark.png";
  import StoresPage from "./components/StoresPage.svelte";
  import ProductCatalogPage from "./components/ProductCatalogPage.svelte";
  import DashboardPage from "./components/DashboardPage.svelte";
  import CsvImportPage from "./components/CsvImportPage.svelte";
  import ReportsPage from "./components/ReportsPage.svelte";
  import BackupRestorePage from "./components/BackupRestorePage.svelte";
  import CalendarPage from "./components/CalendarPage.svelte";
  import ConfigurationPage from "./components/ConfigurationPage.svelte";
  import ScannerPage from "./components/ScannerPage.svelte";
  import { startPeriodicNotificationCheck } from "./lib/notifications.js";
  import { LL } from "./i18n/i18n-svelte.js";

  type Tab = "dashboard" | "stores" | "products" | "calendar" | "reports" | "scanner" | "import" | "backup" | "settings";

  // Tab metadata — the destination tab rune value and the label
  // accessor. Plain <button> elements render the label as visible
  // text; no icon is used at the navbar level (icons appear in the
  // dashboard urgent-card affordance in PR 6, not here).
  const tabs: Array<{
    id: Tab;
    label: () => string;
  }> = [
    { id: "dashboard", label: () => $LL.nav.dashboard() },
    { id: "stores", label: () => $LL.nav.stores() },
    { id: "products", label: () => $LL.nav.products() },
    { id: "calendar", label: () => $LL.nav.calendar() },
    { id: "reports", label: () => $LL.nav.reports() },
    { id: "scanner", label: () => $LL.nav.scanner() },
    { id: "import", label: () => $LL.nav.import() },
    { id: "backup", label: () => $LL.nav.backup() },
    { id: "settings", label: () => $LL.nav.settings() },
  ];

  let activeTab: Tab = "stores";

  // Slice 7b: start notification permission check and periodic polling.
  // The cleanup function is stable and safe to call from onDestroy.
  const stopPeriodicCheck = startPeriodicNotificationCheck();
  onDestroy(() => stopPeriodicCheck());

  function setTab(next: Tab) {
    activeTab = next;
  }
</script>

<!--
  The dashboard gradient band lives on a single absolutely-positioned
  div behind the navbar so the navbar keeps its sticky positioning
  intact. Only rendered on the dashboard tab per the design.
-->
<div class="app-shell">
  <header
    class="navbar bg-base-200 relative app-navbar"
    aria-label={$LL.common.mainNav()}
  >
    <!-- Brand area. On the dashboard tab we render the gradient band
         behind the brand via app-brand-band (an absolute child of
         the navbar). The brand itself is a plain text mark — clicking
         it jumps the user back to the dashboard, which is the project
         convention preserved from the legacy nav. -->
    <div class="navbar-start gap-2">
      {#if activeTab === "dashboard"}
        <span class="app-brand-band" aria-hidden="true"></span>
      {/if}
      <button
        type="button"
        class="app-brand"
        onclick={() => setTab("dashboard")}
        aria-label={$LL.nav.dashboard()}
      >
        <img
          src={caduxoMark}
          alt=""
          class="app-brand-mark"
          aria-hidden="true"
          width="24"
          height="24"
        />
        Caduxo
      </button>
    </div>

    <!-- Desktop tab buttons. On ≥ 720 px they live in navbar-center
         and stay fully visible; on ≤ 720 px they collapse into the
         dropdown trigger inside navbar-end (next block). The CSS
         hides the row via a media query so the keyboard order
         matches the visual order: brand → tabs (desktop) / brand →
         dropdown (mobile) → overflow dropdown (mobile tabs).

         We render plain <button> elements instead of Button.svelte
         because the active-tab affordance uses the WAI-ARIA
         aria-current="page" pattern (the canonical signal for nav
         tabs), and Button.svelte is scoped to action buttons that
         do not need this attribute. The btn btn-ghost btn-sm
         class triplet still gives us the DaisyUI visual surface. -->
    <nav class="navbar-center app-tabs" aria-label={$LL.common.mainNav()}>
      {#each tabs as tab (tab.id)}
        <button
          type="button"
          class="btn btn-ghost btn-sm motion-reduce:transition-none app-tab"
          class:app-tab-active-btn={activeTab === tab.id}
          onclick={() => setTab(tab.id)}
          aria-current={activeTab === tab.id ? "page" : undefined}
        >
          {tab.label()}
        </button>
      {/each}
    </nav>

    <!-- Mobile overflow dropdown (≤ 720 px). DaisyUI v5 ships the
         dropdown + dropdown-end + dropdown-content shell with
         focus + hover handling; the trigger button is keyboard-
         reachable, and every tab inside the menu keeps its label so
         screen readers can announce each destination. The CSS hides
         this block on ≥ 720 px and shows the desktop row. -->
    <div class="navbar-end app-overflow">
      <div class="dropdown dropdown-end">
        <div
          tabindex="0"
          role="button"
          class="btn btn-ghost btn-square"
          aria-label={$LL.common.mainNav()}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-5 w-5"
            aria-hidden="true"
            focusable="false"
          >
            <line x1="3" y1="6" x2="21" y2="6"></line>
            <line x1="3" y1="12" x2="21" y2="12"></line>
            <line x1="3" y1="18" x2="21" y2="18"></line>
          </svg>
        </div>
        <!--
          DaisyUI's dropdown-content opens via :focus-within on the
          trigger, so closing requires removing focus. The
          tabindex="0" trigger above keeps the menu keyboard-
          reachable. We render the same eight tab buttons inside the
          menu so every destination stays one focus stop away.
        -->
        <ul
          tabindex="0"
          class="menu menu-sm dropdown-content bg-base-200 rounded-box z-10 mt-3 w-56 p-2 shadow"
          role="menu"
          aria-label={$LL.common.mainNav()}
        >
          {#each tabs as tab (tab.id)}
            <li role="none">
              <button
                type="button"
                role="menuitem"
                class={activeTab === tab.id ? "active" : ""}
                onclick={() => setTab(tab.id)}
                aria-current={activeTab === tab.id ? "page" : undefined}
              >
                {tab.label()}
              </button>
            </li>
          {/each}
        </ul>
      </div>
    </div>
  </header>

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
  {:else if activeTab === "scanner"}
    <ScannerPage />
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
    background: var(--color-base-100);
  }

  .app-navbar {
    position: sticky;
    top: 0;
    z-index: 100;
    /* Subtle elevation; DaisyUI's navbar does not ship a shadow by
       default so the legacy visual weight is preserved without
       re-introducing the bespoke .nav styles. */
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.15);
  }

  .app-brand {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    font-weight: 700;
    font-size: 1rem;
    letter-spacing: 0.02em;
    color: var(--color-primary);
    cursor: pointer;
    /* Inline-flex keeps the brand mark and the wordmark vertically
       aligned regardless of font-metric drift between platforms. */
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    /* The brand sits on top of the dashboard gradient band, which is
       pointer-events: none, so clicks always reach the button. */
    position: relative;
    z-index: 1;
  }

  /* Decorative brand mark inside the navbar brand button. Sized to
     match the 1rem text on the navbar baseline; the asset file is
     exported at 64px (1x) and 128px (2x) so HiDPI screens still
     look crisp. */
  .app-brand-mark {
    height: 1.5rem;
    width: 1.5rem;
    flex: none;
  }

  /* Dashboard-only gradient band. Absolute child of .navbar-start
     so it spans the brand area at the left of the navbar; the brand
     button itself is lifted above it with position: relative;
     z-index: 1. The band is decorative and does not animate, per
     design §5.9. */
  .app-brand-band {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to bottom right,
      color-mix(in oklch, var(--color-primary) 5%, transparent),
      var(--color-base-100)
    );
    pointer-events: none;
  }

  /* Active tab visual state. Applied via the app-tab-active-btn
     class on the desktop <button> element when activeTab matches.
     DaisyUI's btn-ghost already provides the hover / focus
     background, so we only paint the accent dot + bolder weight for
     the active tab. */
  .app-tab-active-btn {
    font-weight: 600;
    color: var(--color-primary);
  }
  .app-tab-active-btn::before {
    content: "";
    display: inline-block;
    width: 0.375rem;
    height: 0.375rem;
    border-radius: 9999px;
    background: var(--color-primary);
    margin-right: 0.5rem;
    vertical-align: middle;
  }

  /* Desktop row visible at ≥ 720 px. The mobile overflow dropdown
     (.app-overflow) hides at the same breakpoint. */
  .app-tabs {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .app-overflow {
    display: none;
  }

  @media (max-width: 720px) {
    .app-tabs {
      display: none;
    }
    .app-overflow {
      display: flex;
    }
  }

  /* DaisyUI dropdown menu uses inert-style behaviour; we ensure the
     trigger is keyboard-visible when focused. The tabindex="0"
     trigger from the markup gets a focus ring via DaisyUI's global
     focus-visible contract, but we re-assert it here so reduced-
     motion users still see the ring. */
  .app-overflow :global(.btn:focus-visible) {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
</style>