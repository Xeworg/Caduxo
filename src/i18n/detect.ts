/**
 * OS / WebView locale detection.
 * Production uses @tauri-apps/plugin-os; development falls back to navigator.language.
 *
 * The supported-locale table (`AVAILABLE_LOCALES`) is the single source of
 * truth. Detection iterates it in declaration order and matches on the primary
 * IETF subtag, falling back to `DEFAULT_LOCALE` when nothing matches. Adding a
 * new locale to `AVAILABLE_LOCALES` automatically extends detection without
 * touching this file.
 */

import { locale as osLocale } from "@tauri-apps/plugin-os";
import {
  AVAILABLE_LOCALES,
  DEFAULT_LOCALE,
  type SupportedLocale,
} from "./locale.svelte.js";

/**
 * Detect the best-supported locale from the OS or WebView.
 * Returns the first `AVAILABLE_LOCALES` entry whose primary subtag matches
 * the platform tag, or `DEFAULT_LOCALE` when nothing matches.
 */
export async function detectSupportedLocale(): Promise<SupportedLocale> {
  // 1. Production: Tauri OS locale (e.g. "es-MX", "en-GB")
  try {
    const tag = await osLocale();
    if (tag) return mapTag(tag);
  } catch {
    // fall through
  }

  // 2. Dev / web fallback
  if (typeof navigator !== "undefined" && navigator.language) {
    return mapTag(navigator.language);
  }

  return DEFAULT_LOCALE;
}

/**
 * Map an IETF language tag (e.g. "es-MX", "en", "fr-FR") to a `SupportedLocale`.
 *
 * Iterates `AVAILABLE_LOCALES` in declaration order so the first matching
 * primary subtag wins. Unknown primaries fall back to `DEFAULT_LOCALE`.
 */
function mapTag(tag: string): SupportedLocale {
  const primary = tag.split("-")[0].toLowerCase();
  for (const supported of AVAILABLE_LOCALES) {
    if (supported.toLowerCase() === primary) return supported;
  }
  return DEFAULT_LOCALE;
}