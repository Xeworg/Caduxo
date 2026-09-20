import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// `tailwindcss()` is registered *before* `svelte()` so Tailwind's
// class-collection pass sees every Svelte component's class usage during
// the same build pass that compiles the components. Reversing the order
// does not break the build but causes Tailwind to miss classes referenced
// only inside Svelte files, leading to "ghost classes" that render
// un-styled. See docs/redesign-baseline.md and the foundation gate.
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
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