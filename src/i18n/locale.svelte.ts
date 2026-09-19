/**
 * Reactive locale rune and bootstrap helpers.
 *
 * - `locale` — the rune holding the active locale.
 * - `translationSource` — `"detected"` when the active locale came from the OS,
 *   `"manual"` after the user picks one in the Configuration page.
 * - `initLocale()` — call once in main.ts before mounting the app.
 * - `setLocale()` — call from the Configuration page to change locale.
 *
 * `translationSource` is NOT persisted. It is recomputed on every app start
 * from the presence / absence of the `app_settings.language` row.
 */

import { setLocale as i18nSetLocale } from "./i18n-svelte.js";
import { loadAllLocales } from "./i18n-util.sync.js";
import { detectSupportedLocale } from "./detect.js";
import { getSettings, updateSettings } from "../lib/stores.js";

export type SupportedLocale = "en" | "es";

const DEFAULT: SupportedLocale = "en";

// The generated Svelte i18n store is backed by the in-memory `loadedLocales`
// registry. In dev/prod startup we use the synchronous dictionaries, so load
// them once before any component evaluates `$LL.*`.
loadAllLocales();

/** Active locale; read from templates via `$LL.*` or directly when needed. */
export const locale = $state<{ current: SupportedLocale }>({
  current: DEFAULT,
});

/**
 * How the active locale was resolved at boot.
 * - `"detected"` — no `app_settings.language` row; fell back to OS / navigator.
 * - `"manual"` — `app_settings.language` row was present; user made a prior pick.
 *
 * NOT persisted; recomputed on every app start.
 */
export const translationSource = $state<{
  current: "detected" | "manual";
}>({
  current: "detected",
});

/**
 * Initialise the locale rune.
 *
 * 1. Try to read `app_settings.language` from the backend.
 *    If `language_configured` is true (the user persisted a manual pick),
 *    honour `s.language` and mark the source as `"manual"`.
 * 2. Otherwise detect from the OS / WebView and mark the source as `"detected"`.
 *
 * Must be called and awaited in `main.ts` before `mount(App, …)`.
 */
export async function initLocale(): Promise<SupportedLocale> {
  try {
    const s = await getSettings();
    // `language_configured` is the single source of truth: the backend
    // reports false when the row is absent, empty, or holds a tag outside
    // {"en", "es"}. On fresh installs we therefore fall through to OS /
    // WebView detection instead of locking the UI to the `"en"` fallback.
    if (s.language_configured && (s.language === "en" || s.language === "es")) {
      i18nSetLocale(s.language);
      locale.current = s.language;
      translationSource.current = "manual";
      return s.language;
    }
  } catch {
    // getSettings threw (no store created yet) — fall through to detection.
  }

  const detected = await detectSupportedLocale();
  i18nSetLocale(detected);
  locale.current = detected;
  translationSource.current = "detected";
  return detected;
}

/**
 * Change the active locale and persist the choice.
 *
 * Called from the Configuration page locale selector. Rolls back the rune
 * and the typesafe-i18n runtime if `updateSettings` fails.
 */
export async function setLocale(
  next: SupportedLocale
): Promise<void> {
  const prev = locale.current;
  i18nSetLocale(next);
  locale.current = next;
  translationSource.current = "manual";

  try {
    await updateSettings({ language: next });
  } catch {
    // Roll back on persistence failure.
    i18nSetLocale(prev);
    locale.current = prev;
    translationSource.current = "manual";
    throw new Error(
      `Failed to persist locale preference: ${next}`
    );
  }
}
