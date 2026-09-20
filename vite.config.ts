import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  // Top-level await is needed for initLocale() in main.ts and typesafe-i18n internals.
  build: {
    target: "es2022",
  },
  server: {
    strictPort: true,
    port: 1420,
  },
});
