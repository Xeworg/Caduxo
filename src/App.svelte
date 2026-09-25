<!--
  App.svelte — top-level shell (PR 5 of caduxo-daisyui-redesign).

  Migrates the bespoke .nav / .nav-btn / .nav-brand markup to a
  DaisyUI navbar bg-base-200 with navbar-start (Caduxo brand),
  navbar-center (tab buttons rendered as DaisyUI ghost buttons so
  they pick up aria-current="page"), and navbar-end (a dropdown that
  holds the overflow tabs so every tab remains reachable whenever
  the viewport is too narrow to render every Spanish tab label +
  the brand area without clipping — see the `Responsive overflow`
  section below).

  Renders the dashboard gradient band
  (linear-gradient from primary 5% → base-100) behind the brand
  area on the Dashboard landing tab only, per design §5.9.

  Tailwind classes referenced here (for the JIT scanner):
    navbar navbar-start navbar-center navbar-end bg-base-200
    dropdown dropdown-end dropdown-content
    btn btn-ghost btn-sm btn-square
    menu menu-sm rounded-box w-56 shadow
    motion-reduce:transition-none

  Also installs the desktop-app layout guard: an `onMount`-scoped
  zoom guard blocks Ctrl/Cmd + wheel, Linux/WebKitGTK pinch gesture
  events, and Ctrl/Cmd + `+` / `-` / `=` / `0` (including their
  numpad variants) so the webview cannot be accidentally zoomed —
  see the `Zoom guard` section below. The native Tauri window keeps `minWidth: 960` /
  `minHeight: 640` so the OS still clamps the viewport; the CSS
  intentionally does NOT pin `.app-shell` / `html` / `body` to a
  minimum width because that combination forced body-level
  horizontal scrolling at constrained widths and hid the nav
  buttons — see `Responsive overflow` and the follow-up commit
  history in `odd/tasks/fix-category-picker-popovers.md`.

  Responsive overflow
  ------------------
  Nine Spanish nav labels (Panel / Tiendas / Productos / Calendario
  / Reportes / Escáner / Importar / Respaldo / Configuración) plus
  the brand mark and wordmark need roughly 850–1100 px of navbar
  width to render without clipping, depending on font and padding.
  The previous `@media (max-width: 720px)` switch was below both
  the native 960 px `minWidth` and the user-reported failing
  width (≈ 873 px on the screenshot they pasted). The breakpoint
  is now `@media (max-width: 1024px)`: at and below 1024 px the
  desktop tab row hides and the DaisyUI `navbar-end` overflow
  dropdown shows, so every destination stays one menu click
  away. Above 1024 px the desktop tab row is rendered. The CSS
  no longer pins `.app-shell` or `html` / `body` to a minimum
  width; that was the source of the previous bug (the body
  became wider than the viewport, the navbar stayed in desktop
  mode, the right-most tab buttons were clipped off-screen with
  no way to scroll to them).

  Zoom guard
  ----------
  The guard registers listeners on both `window` and `document`
  in the capture phase (`{ passive: false, capture: true }` for
  `wheel` / WebKit `gesture*`, `{ capture: true }` for `keydown`)
  so it fires before any inner bubbling handler can call
  `stopPropagation`. Linux/KDE touchpad pinch can reach WebKitGTK
  as `gesturestart` / `gesturechange` / `gestureend` instead of a
  Ctrl+wheel event, so those gestures are cancelled explicitly. The
  keydown handler matches `event.code` (physical-key identifiers
  — `Equal`, `Minus`, `Digit0`, `NumpadAdd`, `NumpadSubtract`,
  `Numpad0`) so locale-dependent layouts (AZERTY, Dvorak, numpad)
  all map to the same blocked keys. The modifier check is
  `ctrlKey || metaKey`, so every other shortcut (Ctrl+S, Ctrl+R,
  Ctrl+P, Cmd+Q, Cmd+W) and every plain keypress (typing, scanner
  Enter, form shortcuts) passes through untouched. On mount we
  also force `document.documentElement.style.zoom = "1"` as a
  one-shot reset (typed as `string` on `CSSStyleDeclaration`,
  supported on Chromium / WebKit / WebView2 / WebKitGTK — the
  four backends Tauri ships) so any inherited OS-level zoom is
  cleared before the listeners take over. The cleanup returned by `onMount`
  cleanup tears down every listener when the component unmounts,
  so the guard never leaks into HMR or future routing. This is a
  desktop-app layout guard, not an accessibility policy: Caduxo
  is a fixed-layout desktop application, not a responsive
  website.
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
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
  import { scannerNavigation } from "./lib/navigation.js";

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

  let activeTab: Tab = $state("stores");

  // Slice 7b: start notification permission check and periodic polling.
  // The cleanup function is stable and safe to call from onDestroy.
  const stopPeriodicCheck = startPeriodicNotificationCheck();
  onDestroy(() => stopPeriodicCheck());

  // Desktop-app layout guard: prevent the user from accidentally
  // zooming the webview via Ctrl/Cmd + wheel, Linux/WebKitGTK
  // touchpad pinch gestures, Ctrl/Cmd + the browser zoom hotkeys
  // (+ / - / = / 0, including their numpad variants), or opening
  // browser-native context menu actions such as Inspect / Print.
  // Caduxo is a fixed-layout desktop application
  // (not a responsive website), so users have no reason to zoom
  // the document and the resulting layout deformation is a real
  // bug, not an accessibility feature.
  //
  // The guard is installed in `onMount` (returned cleanup tears it
  // down on unmount) so registration is tied to the component
  // lifecycle and does not depend on Svelte reactive tracking
  // path. Listeners are attached in the capture phase on BOTH
  // `window` and `document`, so the gesture is caught before any
  // inner bubbling handler can call `stopPropagation` and the
  // wheel and gesture handlers are non-passive so `preventDefault`
  // actually suppresses browser zoom on Chromium / WebKit / WebView2 /
  // WebKitGTK (the Tauri WebKitGTK backend can receive KDE touchpad
  // pinch as WebKit `gesture*` events instead of Ctrl+wheel).
  //
  // The keydown handler checks `event.code` (the physical-key
  // identifier) instead of `event.key` (the produced character)
  // so locale-dependent layouts (AZERTY, Dvorak, numpad) all map
  // to the same blocked keys:
  //   Equal / Minus / Digit0      — main row (= with Shift is +, - is -, 0)
  //   NumpadAdd / NumpadSubtract / Numpad0 — numpad equivalents
  // The modifier check is `ctrlKey || metaKey`. Browser-native
  // print (`KeyP`) is also suppressed because Caduxo does not expose
  // printing through the webview chrome; every other shortcut
  // (Ctrl+S, Ctrl+R, Cmd+Q, Cmd+W, …) and every plain keypress
  // (normal typing, scanner Enter, form shortcuts) passes through
  // untouched.
  //
  // On mount we also force `document.documentElement.style.zoom =
  // "1"` as a one-shot reset. The `style.zoom` CSS property is
  // non-standard (CSS Zoom draft) but is supported on Chromium /
  // WebKit / WebView2 / WebKitGTK — the four backends Tauri ships
  // — and is typed as `string` on `CSSStyleDeclaration`, so the
  // assignment type-checks. The reset clears any zoom level the
  // webview may have inherited from the OS before the listeners
  // take over.
  const ZOOM_KEY_CODES = new Set([
    "Equal", // = / + (with Shift)
    "Minus", // -
    "Digit0", // 0 (main row)
    "NumpadAdd", // numpad +
    "NumpadSubtract", // numpad -
    "Numpad0", // numpad 0
  ]);

  onMount(() => {
    if (document.documentElement.style.zoom !== "1") {
      document.documentElement.style.zoom = "1";
    }

    const resetDocumentZoom = () => {
      if (document.documentElement.style.zoom !== "1") {
        document.documentElement.style.zoom = "1";
      }
    };
    const blockWheelZoom = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        resetDocumentZoom();
      }
    };
    const blockGestureZoom = (event: Event) => {
      event.preventDefault();
      resetDocumentZoom();
    };
    const blockContextMenu = (event: MouseEvent) => {
      event.preventDefault();
    };
    const blockKeyZoom = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      if (ZOOM_KEY_CODES.has(event.code) || event.code === "KeyP") {
        event.preventDefault();
      }
    };

    window.addEventListener("wheel", blockWheelZoom, {
      passive: false,
      capture: true,
    });
    document.addEventListener("wheel", blockWheelZoom, {
      passive: false,
      capture: true,
    });
    window.addEventListener("keydown", blockKeyZoom, { capture: true });
    document.addEventListener("keydown", blockKeyZoom, { capture: true });
    window.addEventListener("contextmenu", blockContextMenu, { capture: true });
    document.addEventListener("contextmenu", blockContextMenu, {
      capture: true,
    });
    window.addEventListener("gesturestart", blockGestureZoom, {
      passive: false,
      capture: true,
    });
    document.addEventListener("gesturestart", blockGestureZoom, {
      passive: false,
      capture: true,
    });
    window.addEventListener("gesturechange", blockGestureZoom, {
      passive: false,
      capture: true,
    });
    document.addEventListener("gesturechange", blockGestureZoom, {
      passive: false,
      capture: true,
    });
    window.addEventListener("gestureend", blockGestureZoom, {
      passive: false,
      capture: true,
    });
    document.addEventListener("gestureend", blockGestureZoom, {
      passive: false,
      capture: true,
    });

    return () => {
      window.removeEventListener("wheel", blockWheelZoom, {
        capture: true,
      });
      document.removeEventListener("wheel", blockWheelZoom, {
        capture: true,
      });
      window.removeEventListener("keydown", blockKeyZoom, { capture: true });
      document.removeEventListener("keydown", blockKeyZoom, {
        capture: true,
      });
      window.removeEventListener("contextmenu", blockContextMenu, {
        capture: true,
      });
      document.removeEventListener("contextmenu", blockContextMenu, {
        capture: true,
      });
      window.removeEventListener("gesturestart", blockGestureZoom, {
        capture: true,
      });
      document.removeEventListener("gesturestart", blockGestureZoom, {
        capture: true,
      });
      window.removeEventListener("gesturechange", blockGestureZoom, {
        capture: true,
      });
      document.removeEventListener("gesturechange", blockGestureZoom, {
        capture: true,
      });
      window.removeEventListener("gestureend", blockGestureZoom, {
        capture: true,
      });
      document.removeEventListener("gestureend", blockGestureZoom, {
        capture: true,
      });
    };
  });

  function setTab(next: Tab) {
    activeTab = next;
  }

  // React to in-memory Dashboard → Scanner navigation requests.
  // When Dashboard writes a lot/product request into `scannerNavigation`,
  // the shell switches to the Scanner tab so ScannerPage can consume it.
  $effect(() => {
    const unsub = scannerNavigation.subscribe((req) => {
      if (req !== null) {
        activeTab = "scanner";
      }
    });
    return unsub;
  });
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
          The DaisyUI dropdown-content opens via :focus-within on the
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

  /* No document- or shell-level `min-width` is set: the previous
     `.app-shell` + `html` / `body { min-width: 960px }` guard
     forced body-level horizontal scrolling at constrained widths
     (e.g. the user-reported ≈ 873 px window) and the navbar stayed
     in desktop mode, so the right-side tab buttons ended up clipped
     off-screen with no way to scroll to them. The responsive
     overflow dropdown (see `.app-tabs` / `.app-overflow` /
     `@media (max-width: 1024px)` below) now owns the constrained-
     width UX, while the native Tauri window with `minWidth: 960`
     (see `src-tauri/tauri.conf.json`) keeps the OS from
     resizing the window below the app floor in the first place.
     Individual page components (the DashboardPage grid and the
     ConfigurationPage two-column row, for example) carry their own
     internal `min-width` rules where they need a minimum content
     width and scroll horizontally inside their containers as
     needed. */

  .app-navbar {
    position: sticky;
    top: 0;
    z-index: 100;
    /* Subtle elevation; the DaisyUI navbar does not ship a shadow by
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
     The DaisyUI btn-ghost class already provides the hover / focus
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

  /* Desktop row visible above 1024 px, overflow dropdown below.
     The previous `@media (max-width: 720px)` switch was below the
     native 960 px Tauri `minWidth` and the ≈ 873 px width the user
     pasted in their screenshot, so the desktop row rendered in a
     clipped state with the right-side buttons cut off. 1024 px is
     the new cutoff: at and below it, the desktop row hides and the
     DaisyUI `navbar-end` dropdown shows so every destination is
     one menu click away. Above 1024 px the desktop row fits the
     nine Spanish labels + brand area comfortably. */
  .app-tabs {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .app-overflow {
    display: none;
  }

  @media (max-width: 1024px) {
    .app-tabs {
      display: none;
    }
    .app-overflow {
      display: flex;
    }
  }

  /* DaisyUI dropdown menu uses inert-style behaviour; we ensure the
     trigger is keyboard-visible when focused. The tabindex="0"
     trigger from the markup gets a focus ring via the DaisyUI global
     focus-visible contract, but we re-assert it here so reduced-
     motion users still see the ring. */
  .app-overflow :global(.btn:focus-visible) {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
</style>