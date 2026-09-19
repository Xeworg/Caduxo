import { mount } from "svelte";
import "./style.css";
import App from "./App.svelte";
import { initLocale } from "./i18n/locale.svelte.js";

let app: ReturnType<typeof mount> | undefined;

async function bootstrap() {
  // Initialise the locale rune before the first paint so that $LL.* helpers
  // resolve to the correct language on mount. Avoid top-level await here:
  // older WebKitGTK builds used by Tauri can fail to evaluate modules that
  // contain it, leaving the dev window blank even though Vite is running.
  await initLocale();

  app = mount(App, {
    target: document.getElementById("app")!,
  });
}

void bootstrap().catch((error) => {
  console.error("Caduxo failed to start", error);
});

export default app;
