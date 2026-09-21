/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 3 — stores & settings).
 * All commands are thin wrappers around the Rust command layer.
 */

import { invoke } from "@tauri-apps/api/core";
import type { SupportedLocale } from "../i18n/locale.svelte.js";

/**
 * Curated DaisyUI theme set shipped at v1 GA. Mirrors the backend
 * `{"caduxo-light", "dark", "dracula", "valentine", "luxury", "sunset", "nord"}`
 * whitelist enforced at the IPC boundary.
 *
 * Kept narrow on purpose: a future accent-theme follow-up would extend
 * this union. The Rust backend (`update_settings` command and
 * `validate_theme_value` in `src-tauri/src/commands/stores.rs`) rejects
 * anything outside this set at the IPC boundary.
 */
export type ThemeName =
  | "caduxo-light"
  | "dark"
  | "dracula"
  | "valentine"
  | "luxury"
  | "sunset"
  | "nord";

/**
 * FEFO (First-Expired, First-Out) lot-selection policy persisted in
 * `app_settings.scanner_fefo_policy`. Wire-serialised as the snake_case
 * strings the backend accepts (`suggest_fefo`, `require_fefo`,
 * `manual_lot_choice`); defaults to `"suggest_fefo"` on fresh installs.
 *
 * The Scanner tab honours the policy for `ProductMatch` results only;
 * `LotMatch` results bypass FEFO because the scanned code already names
 * the lot. PR 2 of `scanner-quick-operations` ships the Scanner tab
 * consumer; PR 3 ships the Configuration selector.
 */
export type FefoPolicy = "suggest_fefo" | "require_fefo" | "manual_lot_choice";

/**
 * Close-window behaviour persisted in `app_settings.close_behavior`.
 * Wire-serialised as the snake_case strings the backend accepts
 * (`minimize_to_tray`, `exit_application`); defaults to
 * `"minimize_to_tray"` on fresh installs. PR 3 wires the Tauri runtime
 * so the configured behaviour takes effect from the very first close
 * attempt.
 */
export type CloseBehavior = "minimize_to_tray" | "exit_application";

export interface StoreResponse {
 id: string;
 name: string;
 code: string | null;
 notes: string | null;
 is_active: boolean;
 created_at: string;
 updated_at: string;
}

export interface StoreCreate {
 name: string;
 code?: string | null;
 notes?: string | null;
}

export interface StoreUpdate {
 id: string;
 name: string;
 code?: string | null;
 notes?: string | null;
 is_active: boolean;
}

export interface StoreLocationResponse {
 id: string;
 store_id: string;
 name: string;
 notes: string | null;
 is_active: boolean;
 created_at: string;
 updated_at: string;
}

export interface StoreLocationCreate {
 store_id: string;
 name: string;
 notes?: string | null;
}

export interface StoreLocationUpdate {
 id: string;
 store_id: string;
 name: string;
 notes?: string | null;
 is_active: boolean;
}

export interface SettingsResponse {
 last_selected_store_id: string | null;
 /** When true, lot creation requires a location to be chosen. */
 require_initial_location_on_lot_create: boolean;
 /** Effective interface locale. Falls back to `DEFAULT_LOCALE` on fresh installs
  *  when no `app_settings.language` row exists. Use `language_configured` to
  *  disambiguate the fallback from a manual pick. */
 language: SupportedLocale;
 /** True only when the user persisted a language preference via
  *  `updateSettings({ language })`. False on fresh installs so the
  *  frontend can run OS / WebView detection instead of treating the
  *  fallback locale as a manual choice. */
 language_configured: boolean;
 /** Effective active theme. Falls back to `"caduxo-light"` on fresh installs
  *  when no `app_settings.theme` row exists. Use `theme_configured` to
  *  disambiguate the fallback from a manual pick. */
 theme: ThemeName;
 /** True only when the user persisted a theme preference via
  *  `updateSettings({ theme })`. False on fresh installs so the frontend
  *  can run `prefers-color-scheme` detection instead of treating the
  *  fallback theme as a manual choice. The backend also reports `false`
  *  when a stored row carries an unsupported value (e.g. legacy `"fr"`
  *  for language or `"synthwave"` for theme). */
  theme_configured: boolean;
 /** Effective FEFO lot-selection policy for the Scanner tab. Defaults to
  *  `"suggest_fefo"` on fresh installs (matching the spec) and when a
  *  stored row carries an unrecognised value (mirroring the `language`
  *  fallback pattern). PR 2 of `scanner-quick-operations`. */
 scanner_fefo_policy: FefoPolicy;
 /** Effective close-window behaviour. Defaults to `"minimize_to_tray"`
  *  on fresh installs (matching the spec) and when a stored row carries
  *  an unrecognised value. PR 3 wires the Tauri runtime consumer;
  *  PR 2 ships the TS mirror so the Scanner tab can read the policy. */
 close_behavior: CloseBehavior;
}

export interface SettingsUpdate {
 last_selected_store_id?: string | null;
 /** Optional: toggles the require_initial_location_on_lot_create setting. */
 require_initial_location_on_lot_create?: boolean;
 /** Optional: sets the interface language preference. */
 language?: SupportedLocale;
 /** Optional: sets the active theme preference. The IPC boundary rejects
  *  values outside the curated `ThemeName` set with a `CommandError::Validation`
  *  so the persisted row stays untouched. PR 2 of `caduxo-daisyui-redesign`. */
  theme?: ThemeName;
 /** Optional: sets the Scanner FEFO lot-selection policy. The IPC boundary
  *  rejects values outside the curated `FefoPolicy` set with a
  *  `CommandError::Validation` so the persisted row stays untouched.
  *  PR 2 of `scanner-quick-operations`. */
 scanner_fefo_policy?: FefoPolicy;
 /** Optional: sets the close-window behaviour. The IPC boundary rejects
  *  values outside the curated `CloseBehavior` set with a
  *  `CommandError::Validation` so the persisted row stays untouched.
  *  PR 2 of `scanner-quick-operations`. */
 close_behavior?: CloseBehavior;
}

// ─── First-run / settings ────────────────────────────────────────────────────

export async function isFirstRun(): Promise<boolean> {
 return invoke<boolean>("is_first_run");
}

export async function getSettings(): Promise<SettingsResponse> {
 return invoke<SettingsResponse>("get_settings");
}

export async function updateSettings(
 input: SettingsUpdate,
): Promise<SettingsResponse> {
 return invoke<SettingsResponse>("update_settings", { input });
}

export async function hasStore(): Promise<boolean> {
 return invoke<boolean>("has_store");
}

// ─── Store CRUD ─────────────────────────────────────────────────────────────

export async function listStores(): Promise<StoreResponse[]> {
 return invoke<StoreResponse[]>("list_stores");
}

export async function createStore(input: StoreCreate): Promise<StoreResponse> {
 return invoke<StoreResponse>("create_store", { input });
}

export async function updateStore(input: StoreUpdate): Promise<StoreResponse> {
 return invoke<StoreResponse>("update_store", { input });
}

// ─── Store location CRUD ─────────────────────────────────────────────────────

export async function listStoreLocations(
 storeId: string,
): Promise<StoreLocationResponse[]> {
 return invoke<StoreLocationResponse[]>("list_store_locations", {
  storeId,
 });
}

export async function createStoreLocation(
 input: StoreLocationCreate,
): Promise<StoreLocationResponse> {
 return invoke<StoreLocationResponse>("create_store_location", { input });
}

export async function updateStoreLocation(
 input: StoreLocationUpdate,
): Promise<StoreLocationResponse> {
 return invoke<StoreLocationResponse>("update_store_location", { input });
}
