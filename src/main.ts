import { mount } from "svelte";
// PR 1 (foundation) of the daisyui redesign switches the stylesheet
// entry from the legacy `./style.css` to `./app.css` (Tailwind v4 +
// DaisyUI v5 + caduxo-light custom theme + motion tokens + reduced-
// motion reset). `./style.css` is retained in place during the
// migration and retired in PR 13 once the grep gate confirms zero
// production references to its classes.
import "./app.css";
import App from "./App.svelte";
import { initLocale } from "./i18n/locale.svelte.js";
import { initTheme } from "./components/ui/theme/themeStore.svelte.js";

let app: ReturnType<typeof mount> | undefined;

async function bootstrap() {
  // Initialise the locale + theme runes before the first paint so that
  // $LL.* helpers resolve to the correct language and the navbar /
  // dashboard gradient band render with the right theme on mount.
  // Avoid top-level await here: older WebKitGTK builds used by Tauri
  // can fail to evaluate modules that contain it, leaving the dev
  // window blank even though Vite is running. `initTheme()` mirrors
  // the `initLocale()` precedence (persisted > OS > fallback) and is
  // idempotent, so a second call from HMR or route changes is safe.
  await Promise.all([initLocale(), initTheme()]);

  app = mount(App, {
    target: document.getElementById("app")!,
  });
}

void bootstrap().catch((error) => {
  console.error("Caduxo failed to start", error);
});

export default app;
