//! Data Transfer Objects for backup and restore operations (Slice 12).

use serde::{Deserialize, Serialize};

/// Result of a successful backup export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    /// Absolute path where the backup was written.
    pub path: String,
    /// Number of bytes written.
    pub bytes: u64,
    /// Schema version at time of backup (last applied migration version).
    pub schema_version: i64,
}

/// Input for the restore command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreInput {
    /// Absolute path to the backup file to restore.
    pub backup_path: String,
    /// User must explicitly confirm this is a destructive operation.
    pub confirmed: bool,
}

/// Validation result for a backup file before restoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreValidation {
    /// Whether the file appears to be a valid SQLite database.
    pub is_valid_sqlite: bool,
    /// Whether all expected Caduxo tables are present.
    pub has_expected_schema: bool,
    /// Whether the schema version is compatible (same as or lower than current).
    pub is_version_compatible: bool,
    /// Human-readable description of each validation check (English canonical
    /// fallback text). Kept as the source of truth for older frontends.
    pub checks: Vec<String>,
    /// Stable check codes aligned with `checks`. Each entry is `Some(code)`
    /// when the service emits a typed code for that check, or `None` when
    /// the row has no localized counterpart (reserved for future additions).
    /// The frontend dispatches by code and falls back to the matching
    /// `checks[i]` string when the code is `None` or unrecognized.
    pub check_codes: Vec<Option<String>>,
    /// The schema version detected in the backup, if readable.
    pub detected_schema_version: Option<i64>,
    /// Whether the backup passed all checks and is safe to restore.
    pub can_restore: bool,
}

/// Result of a successful restore operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// The path to the restored database file.
    pub database_path: String,
    /// Schema version after restore.
    pub schema_version: i64,
}
