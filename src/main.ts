import { mount } from "svelte";
import "./style.css";
import App from "./App.svelte";
import { initLocale } from "./i18n/locale.js";

// Initialise the locale rune before the first paint so that $LL.* helpers
// resolve to the correct language on mount.
await initLocale();

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
