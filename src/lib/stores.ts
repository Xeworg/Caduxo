/**
 * TypeScript API wrapper for Caduxo Tauri commands (Slice 3 — stores & settings).
 * All commands are thin wrappers around the Rust command layer.
 */

import { invoke } from "@tauri-apps/api/core";

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
}

export interface SettingsUpdate {
 last_selected_store_id?: string | null;
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
