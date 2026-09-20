/**
 * TypeScript API wrapper for backup and restore commands (Slice 12).
 */

import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";

// ─── DTOs (mirror Rust DTOs in src-tauri/src/dto/backup_restore.rs) ────────────

/** Result of a successful backup export. */
export interface BackupResult {
 path: string;
 bytes: number;
 schemaVersion: number;
}

/** Input for the restore command. */
export interface RestoreInput {
 backupPath: string;
 confirmed: boolean;
}

/** Validation result for a backup file before restoring. */
export interface RestoreValidation {
 isValidSqlite: boolean;
 hasExpectedSchema: boolean;
 isVersionCompatible: boolean;
 /** Human-readable English fallback text for each check. */
 checks: string[];
 /**
  * Stable check codes aligned by index with `checks`. Each entry is `Some`
  * (a string) when the service emits a typed code for that check, or `null`
  * when the row has no localized counterpart. The frontend dispatches by
  * code and falls back to the matching `checks[i]` string when the code is
  * `null` / `undefined` or unrecognized.
  */
 checkCodes: (string | null)[];
 detectedSchemaVersion: number | null;
 canRestore: boolean;
}

/** Result of a successful restore operation. */
export interface RestoreResult {
 databasePath: string;
 schemaVersion: number;
}

// ─── Command wrappers ─────────────────────────────────────────────────────────

/**
 * Exports a copy of the active database to a user-chosen destination.
 * Uses a native save dialog to let the user pick the destination.
 */
export async function exportBackupWithDialog(): Promise<BackupResult | null> {
 const path = await save({
  defaultPath: `caduxo-backup-${new Date().toISOString().slice(0, 10)}.db`,
  filters: [{ name: "SQLite Database", extensions: ["db"] }],
  title: "Export database backup",
 });

 if (!path) {
  return null; // User cancelled
 }

 return invoke<BackupResult>("export_backup", { destination: path });
}

/**
 * Validates a backup file before restoring.
 * Opens a native file picker so the user can select a backup file.
 */
export async function validateBackupWithDialog(): Promise<{
 path: string;
 validation: RestoreValidation;
} | null> {
 const path = await open({
  multiple: false,
  filters: [
   { name: "SQLite Database", extensions: ["db", "sqlite", "sqlite3"] },
  ],
  title: "Select backup file to validate",
 });

 if (!path || typeof path !== "string") {
  return null; // User cancelled or multiple files returned
 }

 const validation = await invoke<RestoreValidation>("validate_backup", {
  backupPath: path,
 });

 return { path, validation };
}

/**
 * Restores the database from a validated backup file.
 * Requires explicit user confirmation (the frontend is responsible for
 * displaying a confirmation dialog).
 */
export async function restoreBackup(
 input: RestoreInput,
): Promise<RestoreResult> {
 return invoke<RestoreResult>("restore_backup", { input });
}
