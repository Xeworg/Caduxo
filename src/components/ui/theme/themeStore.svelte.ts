/**
 * Reactive theme rune + bootstrap helpers (PR 5 of
 * openspec/changes/caduxo-daisyui-redesign).
 *
 *  - `theme` — the rune holding the active theme name and source.
 *  - `AVAILABLE_THEMES` — bundled DaisyUI v5 theme whitelist.
 *  - `initTheme()` — call once in `main.ts` before the first paint.
 *  - `setTheme(next)` — call from the Configuration page switcher;
 *    optimistic apply with rollback on `updateSettings` failure.
 *
 * Mirrors the `locale.svelte.ts` contract (single rune + bootstrap
 * helpers + IPC bridge), so consumers can wire the Configuration page
 * switcher with the same pattern.
 *
 * `theme.source` is NOT persisted as its own field; it is recomputed
 * on every `initTheme()` call from the presence / absence of the
 * `app_settings.theme` row.
 */

import { getSettings, updateSettings, type ThemeName } from "../../../lib/stores.js";

/**
 * Curated DaisyUI v5 theme set shipped at v1 GA. Mirrors the backend
 * `{"caduxo-light", "dark"}` whitelist enforced at the IPC boundary
 * (PR 2 of caduxo-daisyui-redesign).
 *
 * The frontend treats this union as the single source of truth: a
 * future accent-theme follow-up would extend `ThemeName` here and
 * the backend's whitelist in lockstep.
 */
export type { ThemeName };

/**
 * Where the active theme came from. `manual` covers the current
 * session after a successful `setTheme` call; `persisted` is the
 * boot path when `app_settings.theme` exists; `os` is the
 * `prefers-color-scheme: dark` detection result; `fallback` is the
 * `caduxo-light` default applied when no preference is known.
 */
export type ThemeSource = "manual" | "persisted" | "os" | "fallback";

/** Bundled theme set — single source of truth for the switcher. */
export const AVAILABLE_THEMES: ThemeName[] = ["caduxo-light", "dark"];

interface ThemeState {
  current: ThemeName;
  source: ThemeSource;
}

/**
 * Narrow-guard: returns true when `name` is one of the curated themes
 * we ship. The backend also enforces this whitelist at the IPC
 * boundary, so the rune is always populated with a valid value —
 * this guard is a defensive belt-and-suspenders check used by
 * `initTheme()` before applying a persisted value.
 */
function isAvailableTheme(name: unknown): name is ThemeName {
  return typeof name === "string"
    && (AVAILABLE_THEMES as string[]).includes(name);
}

/**
 * Read the current `prefers-color-scheme` from the WebView. The Web
 * standard `matchMedia` returns `false` when the user has not made a
 * preference known, so we treat that as the default theme (no
 * preference is `prefers-color-scheme: light`).
 *
 * Safe to call in non-browser contexts (e.g. SSR) — the function
 * short-circuits to `false` when `matchMedia` is undefined.
 */
function readPrefersColorScheme(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return false;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/**
 * Apply a theme to the document root by setting the
 * `data-theme="…"` attribute DaisyUI v5 reads at runtime. The
 * function is the only place that touches the DOM, which keeps the
 * rune / IPC layer independent from rendering.
 */
function applyTheme(name: ThemeName): void {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute("data-theme", name);
}

/**
 * Active theme + source rune. Read from components via
 * `theme.current` / `theme.source` (no `$`-prefix because it is a
 * plain `$state` rune consumed by `$derived` in callers).
 */
export const theme = $state<ThemeState>({
  current: "caduxo-light",
  source: "fallback",
});

/**
 * Initialise the theme rune.
 *
 * Resolution precedence (mirrors `locale.svelte.ts::initLocale`):
 *
 *   1. persisted `app_settings.theme` → "persisted" source
 *   2. `prefers-color-scheme: dark`  → "os" source
 *   3. `caduxo-light` default        → "fallback" source
 *
 * Must be called and awaited in `src/main.ts` before the first paint,
 * next to `initLocale()`, so the navbar / brand colour band render
 * with the right theme on mount. Idempotent: a second call repeats
 * the resolution and re-applies the attribute, which is harmless.
 */
export async function initTheme(): Promise<ThemeName> {
  try {
    const s = await getSettings();
    // `theme_configured` is the single source of truth: the backend
    // reports false when the row is absent, empty, or holds an
    // unsupported theme name. Honour the persisted preference when it
    // is a valid curated theme.
    if (s.theme_configured && isAvailableTheme(s.theme)) {
      applyTheme(s.theme);
      theme.current = s.theme;
      theme.source = "persisted";
      return s.theme;
    }
  } catch {
    // getSettings threw (no store created yet, or no Tauri host) —
    // fall through to OS / fallback detection.
  }

  const prefersDark = readPrefersColorScheme();
  const next: ThemeName = prefersDark ? "dark" : "caduxo-light";
  applyTheme(next);
  theme.current = next;
  theme.source = prefersDark ? "os" : "fallback";
  return next;
}

/**
 * Change the active theme and persist the choice.
 *
 * Called from the Configuration page theme switcher. Optimistic
 * apply with rollback on `updateSettings` failure: if the IPC call
 * rejects, the rune and the document attribute are reverted to the
 * snapshot taken before the optimistic update, and the original
 * error is re-thrown so the consumer can surface it inline.
 *
 * `setTheme` is safe to call multiple times in the same session.
 */
export async function setTheme(next: ThemeName): Promise<void> {
  if (!isAvailableTheme(next)) {
    throw new Error(`Unsupported theme: ${String(next)}`);
  }
  if (next === theme.current) {
    // No-op when the user picks the already-active theme. We still
    // flip the source to "manual" so the switcher label matches the
    // user's intent, then short-circuit to avoid an unnecessary IPC
    // round-trip.
    theme.source = "manual";
    return;
  }

  const prev = theme.current;
  const prevSource = theme.source;

  // Optimistic apply — flip the rune and the DOM attribute first so
  // the user sees the new theme immediately, then attempt to persist.
  applyTheme(next);
  theme.current = next;
  theme.source = "manual";

  try {
    await updateSettings({ theme: next });
  } catch (e) {
    // Rollback on IPC failure — restore both the document attribute
    // and the rune so the rest of the UI stays in sync. Re-throw the
    // original error so the Configuration page can surface it via
    // `humanizeError` inside an `Alert.svelte` primitive.
    applyTheme(prev);
    theme.current = prev;
    theme.source = prevSource;
    throw e;
  }
}