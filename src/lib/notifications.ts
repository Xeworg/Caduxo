/**
 * TypeScript API wrapper for local notification commands (Slice 7b).
 *
 * - `listDueNotifications` / `markNotificationShown` — thin wrappers around
 *   the Rust Tauri commands from Slice 7 (backend).
 * - `checkAndShowDueNotifications` — orchestrates the full check-and-notify flow:
 *   permission check → backend query → OS notification per lot → mark-as-shown.
 * - `startPeriodicNotificationCheck` — starts a periodic interval and returns a
 *   cleanup function to cancel it (safe for Svelte onMount / onDestroy).
 *
 * No sensitive product/SKU/barcode/description/notes content is logged.
 */

import { invoke } from "@tauri-apps/api/core";
import {
 isPermissionGranted,
 requestPermission,
 sendNotification,
} from "@tauri-apps/plugin-notification";

// ─── DTOs (mirror Rust DTOs in src-tauri/src/dto/notifications.rs) ────────────

/** A single lot that is a candidate for a local OS notification. */
export interface DueNotificationLot {
 lot_id: string;
 product_id: string;
 sku: string;
 description: string;
 store_id: string;
 store_name: string;
 location_id: string | null;
 location_name: string | null;
 quantity: number;
 unit: string;
 expiry_date: string;
 alert_days_before: number;
 /** The date this notification was computed for (usually today). */
 notification_date: string;
}

/** Input for `mark_notification_shown`. */
export interface MarkNotificationShownInput {
 expiry_lot_id: string;
 /** ISO date string (YYYY-MM-DD). Defaults to today (UTC) when omitted. */
 notification_date?: string | null;
}

// ─── Backend command wrappers ──────────────────────────────────────────────────

/**
 * Returns active lots whose alert window contains `today`, excluding any lot
 * already logged for `today`. Defaults to today (UTC) when `today` is omitted.
 */
export async function listDueNotifications(
 today?: string | null,
): Promise<DueNotificationLot[]> {
 return invoke<DueNotificationLot[]>("list_due_notifications", { today });
}

/**
 * Records that a notification was shown for (lot_id, date). Idempotent.
 * Returns NotFound for unknown lot ids.
 */
export async function markNotificationShown(
 input: MarkNotificationShownInput,
): Promise<void> {
 // The Rust command returns NotificationLogResponse but the frontend does not
 // need to read it; `void` in TypeScript matches the caller's intent.
 await invoke("mark_notification_shown", { input });
}

// ─── OS notification permission ───────────────────────────────────────────────

/**
 * Checks whether OS notifications are currently permitted.
 * Returns true only when the OS has granted permission explicitly.
 */
export async function isNotificationPermissionGranted(): Promise<boolean> {
 try {
  return await isPermissionGranted();
 } catch {
  return false;
 }
}

/**
 * Requests OS notification permission from the user.
 * Returns the granted permission string (e.g. "granted", "denied").
 * Safe to call even when permission is already granted.
 */
export async function requestNotificationPermission(): Promise<string> {
 try {
  return await requestPermission();
 } catch {
  return "denied";
 }
}

// ─── Notification check and show ───────────────────────────────────────────────

/**
 * Formats a short notification body for a due lot.
 * Omits sensitive product/SKU/barcode from any logs — the body is shown to
 * the user so it intentionally includes the product description.
 */
function formatNotificationBody(lot: DueNotificationLot): string {
 const location = lot.location_name
  ? ` (${lot.store_name} / ${lot.location_name})`
  : ` (${lot.store_name})`;
 const qty = `${lot.quantity} ${lot.unit}`;
 return `${qty} expires ${lot.expiry_date}${location}`;
}

/**
 * Shows an OS notification for a single due lot and, on success, records it
 * in the backend notification log so it is not shown again today.
 *
 * Errors are swallowed silently — the lot remains a candidate for the next
 * periodic check.
 */
async function showAndRecordNotification(
 lot: DueNotificationLot,
): Promise<void> {
 try {
  await sendNotification({
   title: `⚠️ Expiry alert: ${lot.sku}`,
   body: formatNotificationBody(lot),
  });
  // Only mark as shown AFTER the OS notification attempt succeeds.
  await markNotificationShown({
   expiry_lot_id: lot.lot_id,
   notification_date: lot.notification_date,
  });
 } catch {
  // Notification delivery failed or backend mark failed — do not update the
  // notification log so the lot remains a candidate for the next check.
 }
}

/**
 * Full notification check-and-show flow:
 *
 * 1. Check OS notification permission.
 * 2. Request permission if not granted (user may deny).
 * 3. If denied, exit early (no further action).
 * 4. Query due notification candidates from the backend.
 * 5. For each candidate, show an OS notification and mark-as-shown.
 *
 * Safe to call on startup and on each periodic tick. Idempotent at the
 * backend level — `listDueNotifications` already excludes lots logged today.
 */
export async function checkAndShowDueNotifications(): Promise<void> {
 // Step 1: check current permission.
 const granted = await isNotificationPermissionGranted();
 if (!granted) {
  // Step 2: request permission; if denied, abort silently.
  const result = await requestNotificationPermission();
  if (result !== "granted") {
   return;
  }
 }

 // Step 4: query due candidates from the backend.
 const dueLots = await listDueNotifications();
 if (dueLots.length === 0) {
  return;
 }

 // Step 5: show a notification and record it for each due lot.
 // Sequential to avoid flooding the OS notification queue.
 for (const lot of dueLots) {
  await showAndRecordNotification(lot);
 }
}

// ─── Periodic check ────────────────────────────────────────────────────────────

/**
 * Default check interval: 15 minutes (in milliseconds).
 */
export const NOTIFICATION_CHECK_INTERVAL_MS = 15 * 60 * 1000;

/**
 * Starts a periodic notification check.
 *
 * Calls `checkAndShowDueNotifications` once immediately, then again every
 * `intervalMs` milliseconds. Returns a cleanup function that cancels the
 * interval — call it from Svelte's `onDestroy` to prevent memory leaks.
 *
 * @param intervalMs - milliseconds between checks (default: 15 minutes)
 * @returns cleanup function that cancels the interval
 */
export function startPeriodicNotificationCheck(
 intervalMs: number = NOTIFICATION_CHECK_INTERVAL_MS,
): () => void {
 // Run once immediately so the startup check is covered.
 checkAndShowDueNotifications().catch(() => {
  // Startup check is fire-and-forget; periodic retries cover failures.
 });

 const handle = setInterval(() => {
  checkAndShowDueNotifications().catch(() => {
   // Periodic check failures are swallowed; the next tick will retry.
  });
 }, intervalMs);

 // Return a stable cleanup function.
 return () => clearInterval(handle);
}
