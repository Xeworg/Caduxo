/**
 * OS / WebView locale detection.
 * Production uses @tauri-apps/plugin-os; development falls back to navigator.language.
 */

import { locale as osLocale } from "@tauri-apps/plugin-os";
import type { SupportedLocale } from "./locale.js";

/**
 * Detect the best-supported locale from the OS or WebView.
 * Returns `"es"` when the primary subtag is `es` (any region variant),
 * `"en"` for everything else.
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

  return "en";
}

/** Map an IETF language tag (e.g. "es-MX", "en", "fr-FR") to a SupportedLocale. */
function mapTag(tag: string): SupportedLocale {
  const primary = tag.split("-")[0].toLowerCase();
  if (primary === "es") return "es";
  return "en";
}
