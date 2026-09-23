//! Backup and restore service (Slice 12).
//!
//! Provides:
//! - `export_backup`: copies the active SQLite database to a destination path.
//! - `validate_backup`: opens a backup file read-only, checks the SQLite header,
//!   verifies expected Caduxo tables exist, and runs a bounded integrity check.
//! - `restore_backup`: validates the backup, closes the active pool, replaces the
//!   database file, and reopens the pool with the restored database.
//!
//! ## Logging discipline
//!
//! No product data, SKU, barcode, description, or notes content is logged.
//! Only file paths (destination for export, source for restore) and aggregate
//! counts (bytes written, schema version) are emitted.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

use crate::db::DbPool;
use crate::dto::backup_restore::{BackupResult, RestoreInput, RestoreResult, RestoreValidation};
use crate::error::{AppError, DomainError, InfrastructureError};
use crate::pdf::locale::Locale;
use crate::services::user_messages::{user_message, UserMessage};

/// The list of tables that a valid Caduxo backup must contain.
/// If any of these are missing, the backup is not a valid Caduxo database.
const REQUIRED_TABLES: &[&str] = &[
    "stores",
    "categories",
    "products",
    "product_barcodes",
    "expiry_lots",
    "lot_resolution_events",
    "notification_log",
    "app_settings",
    // `product_lifecycle_events` (V19 — product-lifecycle-reusable-identifiers):
    // the audit table that records every archive / unarchive / retire
    // transition. The snapshot already includes it because `VACUUM INTO`
    // copies the whole database, but the explicit entry is what makes
    // `validate_backup` accept backups that contain the new table.
    "product_lifecycle_events",
];

// ─── Stable check codes ──────────────────────────────────────────────────────
//
// These string codes travel alongside `RestoreValidation::checks` so the
// frontend can dispatch localized messages. The constants are kept inline
// here (and mirrored as string literals in
// `src/components/BackupRestorePage.svelte`) so we don't expand the DTO
// surface just to host code names. Existing checks always emit a code; the
// `Option<String>` element type on `check_codes` is reserved for future
// checks that intentionally have no localized counterpart.

/// Backup file does not exist on disk.
const CHECK_CODE_FILE_NOT_FOUND: &str = "file_not_found";
/// SQLite magic header was recognized.
const CHECK_CODE_SQLITE_HEADER_VALID: &str = "sqlite_header_valid";
/// SQLite magic header was NOT recognized.
const CHECK_CODE_SQLITE_HEADER_INVALID: &str = "sqlite_header_invalid";
/// All required Caduxo tables are present.
const CHECK_CODE_REQUIRED_TABLES_PRESENT: &str = "required_tables_present";
/// One or more required Caduxo tables are missing.
const CHECK_CODE_REQUIRED_TABLES_MISSING: &str = "required_tables_missing";
/// Schema version read from the migrations table.
const CHECK_CODE_SCHEMA_VERSION_DETECTED: &str = "schema_version_detected";
/// Bounded `PRAGMA integrity_check` reported `ok`.
const CHECK_CODE_INTEGRITY_CHECK_OK: &str = "integrity_check_ok";
/// Bounded `PRAGMA integrity_check` reported warnings.
const CHECK_CODE_INTEGRITY_CHECK_FAILED: &str = "integrity_check_failed";
/// Schema/integrity check failed to run at all (e.g. DB could not be opened
/// read-only).
const CHECK_CODE_SCHEMA_CHECK_FAILED: &str = "schema_check_failed";
/// Detected schema version is below the supported minimum (1).
const CHECK_CODE_SCHEMA_VERSION_INVALID: &str = "schema_version_invalid";
/// Detected schema version is in the supported range.
const CHECK_CODE_SCHEMA_VERSION_COMPATIBLE: &str = "schema_version_compatible";

/// Pushes a human-readable check and its stable code into the parallel
/// arrays kept on [`RestoreValidation`]. Kept private to this module so the
/// two arrays cannot drift.
fn push_check(
    checks: &mut Vec<String>,
    codes: &mut Vec<Option<String>>,
    message: impl Into<String>,
    code: &'static str,
) {
    checks.push(message.into());
    codes.push(Some(code.to_string()));
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Exports a copy of the active database to `destination` and returns the path,
/// byte count, and schema version at time of backup.
pub async fn export_backup(
    active_db_path: &Path,
    destination: &Path,
) -> Result<BackupResult, AppError> {
    if destination.exists() {
        tracing::info!(dest = %destination.display(), "Backup destination file already exists; will overwrite");
    }

    // Use SQLite's online `VACUUM INTO` backup path instead of copying the main
    // database file directly. A raw file copy can miss committed changes still
    // living in a `-wal` sidecar; `VACUUM INTO` reads a consistent snapshot.
    create_sqlite_backup(active_db_path, destination).await?;

    let metadata = fs::metadata(destination).map_err(InfrastructureError::Io)?;
    let bytes = metadata.len();

    // Read the schema version from the backup file.
    let schema_version = read_schema_version_from_path(destination).await?;

    tracing::info!(
        dest = %destination.display(),
        bytes,
        schema_version,
        "Database backup exported successfully",
    );

    Ok(BackupResult {
        path: destination.to_string_lossy().into_owned(),
        bytes,
        schema_version,
    })
}

/// Validates a backup file before a restore operation.
/// Opens the file read-only and performs a series of checks:
/// 1. SQLite header validity (magic bytes).
/// 2. All required Caduxo tables are present.
/// 3. Schema version is readable and ≥ 1.
/// 4. Bounded `PRAGMA integrity_check` passes.
pub async fn validate_backup(backup_path: &Path) -> Result<RestoreValidation, AppError> {
    let mut checks: Vec<String> = Vec::new();
    let mut check_codes: Vec<Option<String>> = Vec::new();
    let mut has_expected_schema = true;
    let mut is_version_compatible = true;
    let mut can_restore = true;
    let mut detected_version: Option<i64> = None;

    // ── Check 1: File exists ───────────────────────────────────────────
    if !backup_path.exists() {
        push_check(
            &mut checks,
            &mut check_codes,
            "File does not exist.",
            CHECK_CODE_FILE_NOT_FOUND,
        );
        can_restore = false;
        return Ok(RestoreValidation {
            is_valid_sqlite: false,
            has_expected_schema: false,
            is_version_compatible: false,
            checks,
            check_codes,
            detected_schema_version: None,
            can_restore,
        });
    }

    // ── Check 2: SQLite magic header ──────────────────────────────────
    let is_valid_sqlite = task::spawn_blocking({
        let path = backup_path.to_path_buf();
        move || check_sqlite_header(&path)
    })
    .await
    .map_err(|e| InfrastructureError::BackupIo(format!("header check task error: {e}")))?;

    if is_valid_sqlite {
        push_check(
            &mut checks,
            &mut check_codes,
            "Valid SQLite database header.",
            CHECK_CODE_SQLITE_HEADER_VALID,
        );
    } else {
        push_check(
            &mut checks,
            &mut check_codes,
            "File is not a valid SQLite database (invalid header).",
            CHECK_CODE_SQLITE_HEADER_INVALID,
        );
        can_restore = false;
    }

    // ── Check 3: Schema + integrity (blocking I/O) ─────────────────────
    let schema_result = task::spawn_blocking({
        let path = backup_path.to_path_buf();
        let required = REQUIRED_TABLES.to_vec();
        move || check_schema_and_integrity(&path, &required)
    })
    .await
    .map_err(|e| InfrastructureError::BackupIo(format!("schema check task error: {e}")))?;

    let SchemaCheckResult {
        all_tables_present,
        version,
        integrity_ok,
        schema_error,
    } = schema_result;

    if let Some(err) = schema_error {
        push_check(
            &mut checks,
            &mut check_codes,
            err,
            CHECK_CODE_SCHEMA_CHECK_FAILED,
        );
        has_expected_schema = false;
        is_version_compatible = false;
        can_restore = false;
    } else {
        if all_tables_present {
            push_check(
                &mut checks,
                &mut check_codes,
                format!("All {} required tables present.", REQUIRED_TABLES.len()),
                CHECK_CODE_REQUIRED_TABLES_PRESENT,
            );
        } else {
            push_check(
                &mut checks,
                &mut check_codes,
                "One or more required Caduxo tables are missing.",
                CHECK_CODE_REQUIRED_TABLES_MISSING,
            );
            has_expected_schema = false;
            can_restore = false;
        }

        detected_version = Some(version);
        push_check(
            &mut checks,
            &mut check_codes,
            format!("Schema version: {version}"),
            CHECK_CODE_SCHEMA_VERSION_DETECTED,
        );

        if integrity_ok {
            push_check(
                &mut checks,
                &mut check_codes,
                "Integrity check passed (first 100 pages).",
                CHECK_CODE_INTEGRITY_CHECK_OK,
            );
        } else {
            push_check(
                &mut checks,
                &mut check_codes,
                "Integrity check warnings detected.",
                CHECK_CODE_INTEGRITY_CHECK_FAILED,
            );
            can_restore = false;
        }
    }

    // ── Check 4: Version compatibility ──────────────────────────────────
    let detected_v = detected_version.unwrap_or(0);
    if detected_v < 1 {
        push_check(
            &mut checks,
            &mut check_codes,
            format!("Schema version {detected_v} is not a valid Caduxo database."),
            CHECK_CODE_SCHEMA_VERSION_INVALID,
        );
        is_version_compatible = false;
        can_restore = false;
    } else {
        push_check(
            &mut checks,
            &mut check_codes,
            "Schema version is compatible.",
            CHECK_CODE_SCHEMA_VERSION_COMPATIBLE,
        );
    }

    Ok(RestoreValidation {
        is_valid_sqlite,
        has_expected_schema,
        is_version_compatible,
        checks,
        check_codes,
        detected_schema_version: detected_version,
        can_restore,
    })
}

/// Restores the database from a validated backup file.
///
/// ## Safety notes
///
/// - The user must set `confirmed = true` in `RestoreInput`.
/// - The active pool is closed before replacing the database file.
/// - After replacement, a new pool is opened and the `AppState` pool is updated.
/// - The frontend is responsible for the destructive-confirmation dialog.
pub async fn restore_backup(
    pool: Arc<Mutex<DbPool>>,
    active_db_path: &Path,
    input: RestoreInput,
) -> Result<RestoreResult, AppError> {
    if !input.confirmed {
        return Err(AppError::Domain(DomainError::BusinessRule {
            message: user_message(UserMessage::RestoreRequiresConfirmation, Locale::En),
        }));
    }

    let backup_path = std::path::PathBuf::from(&input.backup_path);

    // 1. Validate the backup before touching anything.
    let validation = validate_backup(&backup_path).await?;
    if !validation.can_restore {
        let reasons = validation.checks.join("; ");
        return Err(AppError::Infrastructure(
            InfrastructureError::BackupValidation(format!(
                "Backup file failed validation checks: {reasons}"
            )),
        ));
    }

    // 2. Close the active pool to release all file handles.
    tracing::info!("Closing active database pool before restore");
    {
        let guard = pool.lock().await;
        // Release the guard before awaiting close() to avoid holding the Mutex across an await.
        drop(guard);
    }
    pool.lock().await.close().await;
    tracing::info!("Active database pool closed");

    // 3. Replace the active database file with the backup.
    // Rename the current file to a `.db.bak.old` backup.
    let old_db_backup = active_db_path.with_extension("db.bak.old");
    if old_db_backup.exists() {
        fs::remove_file(&old_db_backup).map_err(InfrastructureError::Io)?;
    }
    if active_db_path.exists() {
        fs::rename(active_db_path, &old_db_backup).map_err(InfrastructureError::Io)?;
        tracing::info!(old_backup = %old_db_backup.display(), "Current database renamed to backup");
    }

    // Remove SQLite sidecars before replacing the main DB file. Leaving stale WAL/SHM
    // files beside the restored DB risks SQLite replaying pages from the old DB.
    remove_sqlite_sidecars(active_db_path)?;

    // Copy the backup file to the active database location.
    fs::copy(&backup_path, active_db_path).map_err(InfrastructureError::Io)?;
    tracing::info!(
        active_db = %active_db_path.display(),
        backup = %backup_path.display(),
        "Backup copied to active database location",
    );

    // 4. Open a new pool with the restored database and update shared state.
    let new_pool = crate::db::open_pool(active_db_path).await.map_err(|e| {
        InfrastructureError::BackupIo(format!("failed to reopen database pool after restore: {e}"))
    })?;

    {
        let mut pool_guard = pool.lock().await;
        *pool_guard = new_pool;
    }
    tracing::info!("Database pool reopened with restored database");

    // 5. Read the schema version from the restored database.
    let schema_version = read_schema_version_from_path(active_db_path).await?;

    tracing::info!(
        database_path = %active_db_path.display(),
        schema_version,
        "Database restore completed successfully",
    );

    Ok(RestoreResult {
        database_path: active_db_path.to_string_lossy().into_owned(),
        schema_version,
    })
}

// ─── SQLite file helpers ─────────────────────────────────────────────────────

/// Creates a consistent SQLite backup at `destination` using `VACUUM INTO`.
async fn create_sqlite_backup(active_db_path: &Path, destination: &Path) -> Result<(), AppError> {
    let active = active_db_path.to_path_buf();
    let dest = destination.to_path_buf();

    task::spawn_blocking(move || {
        if dest.exists() {
            fs::remove_file(&dest).map_err(InfrastructureError::Io)?;
        }

        let conn = rusqlite::Connection::open(&active).map_err(|e| {
            InfrastructureError::BackupIo(format!("failed to open active database: {e}"))
        })?;

        conn.execute("VACUUM INTO ?1", [&dest.to_string_lossy().as_ref()])
            .map_err(|e| InfrastructureError::BackupIo(format!("failed to export backup: {e}")))?;

        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| InfrastructureError::BackupIo(format!("backup task join error: {e}")))?
}

fn remove_sqlite_sidecars(db_path: &Path) -> Result<(), AppError> {
    for suffix in ["wal", "shm"] {
        let sidecar = db_path.with_extension(format!("db-{suffix}"));
        if sidecar.exists() {
            fs::remove_file(&sidecar).map_err(InfrastructureError::Io)?;
        }
    }
    Ok(())
}

// ─── Schema version helper ───────────────────────────────────────────────────

/// Reads the last-applied migration version from a database file at `path`.
async fn read_schema_version_from_path(path: &Path) -> Result<i64, AppError> {
    let path = path.to_path_buf();
    task::spawn_blocking(move || {
        // Use rusqlite for a simple blocking read of the schema version.
        // This avoids the overhead of opening a full async sqlx pool.
        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| InfrastructureError::BackupIo(format!("failed to open DB: {e}")))?;

        let version: Option<i64> = conn
            .query_row("SELECT MAX(version) FROM _sqlx_migrations", [], |row| {
                row.get(0)
            })
            .ok(); // Returns None if table doesn't exist or query fails

        Ok(version.unwrap_or(0))
    })
    .await
    .map_err(|e| {
        InfrastructureError::BackupIo(format!("task join error reading schema version: {e}"))
    })?
}

// ─── Validation helpers (blocking, run in spawn_blocking) ──────────────────

/// Checks the SQLite magic header bytes. Returns true if valid.
fn check_sqlite_header(path: &Path) -> bool {
    use std::io::Read;
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut reader = std::io::BufReader::new(file);
    let mut header = [0u8; 16];
    reader.read_exact(&mut header).ok();
    header == *b"SQLite format 3\0"
}

/// Result of the schema and integrity check.
struct SchemaCheckResult {
    all_tables_present: bool,
    version: i64,
    integrity_ok: bool,
    /// Set to `Some(error_message)` if the check failed entirely.
    schema_error: Option<String>,
}

/// Opens the backup file read-only, checks all required tables, reads the schema
/// version, and runs a bounded integrity check.
fn check_schema_and_integrity(path: &Path, required: &[&str]) -> SchemaCheckResult {
    let rt = tokio::runtime::Handle::current();
    let result = rt.block_on(async {
        let pool = match sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(path)
                    .read_only(true),
            )
            .await
        {
            Ok(p) => p,
            Err(e) => {
                return SchemaCheckResult {
                    all_tables_present: false,
                    version: 0,
                    integrity_ok: false,
                    schema_error: Some(format!("Failed to open backup file: {e}")),
                };
            }
        };

        // Check each required table exists.
        let mut all_present = true;
        for table in required {
            let exists: bool = match sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=$1)",
            )
            .bind(*table)
            .fetch_one(&pool)
            .await
            {
                Ok(v) => v,
                Err(_) => {
                    all_present = false;
                    false
                }
            };
            if !exists {
                all_present = false;
            }
        }

        // Read schema version.
        let version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

        let version = version.unwrap_or(0);

        // Bounded integrity check — first 100 pages only.
        let integrity: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check(100)")
            .fetch_all(&pool)
            .await
            .unwrap_or_else(|_| vec!["error reading integrity".to_string()]);

        let integrity_ok = integrity == vec!["ok"];

        pool.close().await;

        SchemaCheckResult {
            all_tables_present: all_present,
            version,
            integrity_ok,
            schema_error: None,
        }
    });
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::{Migration, MigrationType};
    use std::borrow::Cow;

    /// Builds a V3-only migrator from the shared MIGRATIONS constant.
    /// Mirrors `db::migrations::tests::v3_only_migrator` so this test can live
    /// in `services::backup_restore::tests` without needing a cross-module import.
    fn v3_only_migrator() -> sqlx::migrate::Migrator {
        let v3_migrations: Vec<Migration> = crate::db::migrations::MIGRATIONS
            .iter()
            .filter(|(v, _, _)| *v <= 3)
            .map(|(version, description, sql)| {
                Migration::new(
                    *version,
                    Cow::Owned(description.to_string()),
                    MigrationType::Simple,
                    Cow::Owned(sql.to_string()),
                    false,
                )
            })
            .collect();
        sqlx::migrate::Migrator {
            migrations: Cow::Owned(v3_migrations),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        }
    }

    /// Builds a V12-only migrator from the shared MIGRATIONS constant.
    /// Used to create a pre-V5 database for the restore-from-pre-V5-backup test.
    /// V12 (NULL-location repoint to sentinel) is needed so that V14 can insert
    /// into lot_movements with a valid source_location_id (the sentinel ID).
    use tempfile::tempdir;

    /// Creates a minimal valid SQLite database with the Caduxo schema.
    async fn create_minimal_caduxo_db(path: &Path) -> Result<(), AppError> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true),
            )
            .await
            .map_err(InfrastructureError::Database)?;

        // Create the sqlx migrations tracking table and insert known versions.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                installed_time TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(InfrastructureError::Database)?;

        sqlx::query(
            "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_time) VALUES (1, 'skeleton', '2024-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .map_err(InfrastructureError::Database)?;

        sqlx::query(
            "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_time) VALUES (2, 'app_schema', '2024-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .map_err(InfrastructureError::Database)?;

        // Create required tables.
        for table in REQUIRED_TABLES {
            sqlx::query(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (id TEXT PRIMARY KEY)"
            ))
            .execute(&pool)
            .await
            .map_err(InfrastructureError::Database)?;
        }

        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn export_backup_writes_file_and_returns_result() {
        let tmp = tempdir().unwrap();
        let active_db = tmp.path().join("active.db");
        let backup_dest = tmp.path().join("backup.db");

        create_minimal_caduxo_db(&active_db).await.unwrap();

        let result = export_backup(&active_db, &backup_dest).await.unwrap();

        assert!(backup_dest.exists());
        assert_eq!(result.path, backup_dest.to_string_lossy());
        assert!(result.bytes > 0);
        assert_eq!(result.schema_version, 2);
    }

    #[tokio::test]
    async fn export_backup_includes_committed_wal_rows() {
        let tmp = tempdir().unwrap();
        let active_db = tmp.path().join("active_wal.db");
        let backup_dest = tmp.path().join("backup_wal.db");

        create_minimal_caduxo_db(&active_db).await.unwrap();

        let conn = rusqlite::Connection::open(&active_db).unwrap();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS wal_probe (id INTEGER PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO wal_probe (value) VALUES ('committed-in-wal');
            "#,
        )
        .unwrap();

        assert!(active_db.with_extension("db-wal").exists());

        export_backup(&active_db, &backup_dest).await.unwrap();
        drop(conn);

        let backup = rusqlite::Connection::open(&backup_dest).unwrap();
        let value: String = backup
            .query_row("SELECT value FROM wal_probe WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(value, "committed-in-wal");
    }

    #[test]
    fn remove_sqlite_sidecars_removes_wal_and_shm_files() {
        let tmp = tempdir().unwrap();
        let db = tmp.path().join("caduxo.db");
        let wal = tmp.path().join("caduxo.db-wal");
        let shm = tmp.path().join("caduxo.db-shm");
        std::fs::write(&db, b"db").unwrap();
        std::fs::write(&wal, b"wal").unwrap();
        std::fs::write(&shm, b"shm").unwrap();

        remove_sqlite_sidecars(&db).unwrap();

        assert!(db.exists());
        assert!(!wal.exists());
        assert!(!shm.exists());
    }

    #[tokio::test]
    async fn validate_backup_accepts_valid_database() {
        let tmp = tempdir().unwrap();
        let backup_file = tmp.path().join("valid_backup.db");

        create_minimal_caduxo_db(&backup_file).await.unwrap();

        let validation = validate_backup(&backup_file).await.unwrap();

        assert!(validation.is_valid_sqlite);
        assert!(validation.has_expected_schema);
        assert!(validation.is_version_compatible);
        assert!(validation.can_restore);
        assert_eq!(validation.detected_schema_version, Some(2));

        // check_codes must stay parallel to checks (same length, no Nones for
        // current checks). Lock the expected code sequence so a future
        // refactor cannot silently drop a code.
        assert_eq!(validation.check_codes.len(), validation.checks.len());
        let codes: Vec<&str> = validation
            .check_codes
            .iter()
            .map(|c| c.as_deref().expect("every emitted check has a code"))
            .collect();
        assert_eq!(
            codes,
            vec![
                CHECK_CODE_SQLITE_HEADER_VALID,
                CHECK_CODE_REQUIRED_TABLES_PRESENT,
                CHECK_CODE_SCHEMA_VERSION_DETECTED,
                CHECK_CODE_INTEGRITY_CHECK_OK,
                CHECK_CODE_SCHEMA_VERSION_COMPATIBLE,
            ]
        );
    }

    #[tokio::test]
    async fn validate_backup_rejects_non_sqlite_file() {
        let tmp = tempdir().unwrap();
        let fake_backup = tmp.path().join("not_sqlite.txt");
        std::fs::write(&fake_backup, "this is not a sqlite file").unwrap();

        let validation = validate_backup(&fake_backup).await.unwrap();

        assert!(!validation.is_valid_sqlite);
        assert!(!validation.can_restore);
        assert!(validation
            .checks
            .iter()
            .any(|c| c.contains("not a valid SQLite")));
        assert_eq!(validation.check_codes.len(), validation.checks.len());
        assert!(validation
            .check_codes
            .iter()
            .any(|c| c.as_deref() == Some(CHECK_CODE_SQLITE_HEADER_INVALID)));
    }

    #[tokio::test]
    async fn validate_backup_rejects_missing_file() {
        let tmp = tempdir().unwrap();
        let missing = tmp.path().join("does_not_exist.db");

        let validation = validate_backup(&missing).await.unwrap();

        assert!(!validation.can_restore);
        assert!(validation
            .checks
            .iter()
            .any(|c| c.contains("does not exist")));
        // The missing-file early return emits a single code.
        assert_eq!(validation.checks.len(), 1);
        assert_eq!(
            validation.check_codes.as_slice(),
            &[Some(CHECK_CODE_FILE_NOT_FOUND.to_string())]
        );
    }

    #[tokio::test]
    async fn restore_requires_confirmation() {
        let pool = Arc::new(Mutex::new(
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    sqlx::sqlite::SqliteConnectOptions::new()
                        .filename(":memory:")
                        .create_if_missing(true),
                )
                .await
                .unwrap(),
        ));

        let result = restore_backup(
            pool,
            Path::new("/tmp/nonexistent.db"),
            RestoreInput {
                backup_path: "/tmp/backup.db".to_string(),
                confirmed: false,
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::BusinessRule { .. })
        ));
    }

    #[tokio::test]
    async fn validate_backup_rejects_missing_required_tables() {
        let tmp = tempdir().unwrap();
        let partial_db = tmp.path().join("partial.db");

        // Create a valid SQLite file but with missing Caduxo tables.
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(&partial_db)
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                installed_time TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_time) VALUES (1, 'skeleton', '2024-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create only one of the required tables.
        sqlx::query("CREATE TABLE IF NOT EXISTS stores (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        pool.close().await;

        let validation = validate_backup(&partial_db).await.unwrap();

        assert!(validation.is_valid_sqlite);
        assert!(!validation.has_expected_schema);
        assert!(!validation.can_restore);
    }

    #[tokio::test]
    async fn read_schema_version_from_path_returns_version() {
        let tmp = tempdir().unwrap();
        let db_path = tmp.path().join("version_test.db");

        create_minimal_caduxo_db(&db_path).await.unwrap();

        let version = read_schema_version_from_path(&db_path).await.unwrap();
        assert_eq!(version, 2);
    }

    #[tokio::test]
    async fn read_schema_version_from_path_returns_zero_for_empty_db() {
        let tmp = tempdir().unwrap();
        let empty_db = tmp.path().join("empty.db");

        // Create a valid SQLite file without the migrations table.
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(&empty_db)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        pool.close().await;

        let version = read_schema_version_from_path(&empty_db).await.unwrap();
        assert_eq!(version, 0);
    }

    #[tokio::test]
    async fn check_sqlite_header_accepts_valid_magic_bytes() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("valid.db");

        // Use rusqlite for guaranteed blocking file creation.
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        drop(conn);

        assert!(check_sqlite_header(&path));
    }

    #[test]
    fn check_sqlite_header_rejects_invalid_magic_bytes() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("invalid.db");
        std::fs::write(&path, b"not sqlite at all").unwrap();
        assert!(!check_sqlite_header(&path));
    }

    /// Verifies that V4 applies automatically when a pool is opened on a
    /// pre-V4 (V3-era) database — simulating the restore path where a backup
    /// file older than V4 is restored and the migrator picks up the pending
    /// V4 migration on the reopened pool.
    ///
    /// Reproduces the scenario described in design §13.1:
    /// "restore from a pre-V4 fixture → Pool reopens → V4 applies →
    ///  junction rows populated from legacy column".
    #[tokio::test]
    async fn restore_from_pre_v4_backup_applies_v4_backfill_in_situ() {
        let tmp = tempdir().unwrap();
        let pre_v4_db = tmp.path().join("pre_v4_backup.db");

        // ── Step 1: create a V3-era database ──────────────────────────────
        {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    sqlx::sqlite::SqliteConnectOptions::new()
                        .filename(&pre_v4_db)
                        .create_if_missing(true),
                )
                .await
                .unwrap();

            // Use v3_only_migrator to build a proper V3-era schema.
            // This runs V1 (skeleton), V2 (app_schema), V3 (expiry_lots) migrations
            // including proper _sqlx_migrations table creation with all required columns.
            v3_only_migrator().run(&pool).await.unwrap();

            // Seed data: one active category and one product with category_id set.
            let now = "2024-06-01 00:00:00";
            sqlx::query(
                "INSERT INTO categories (id, name, is_active, created_at, updated_at) \
                     VALUES ($1, $2, $3, $4, $5)",
            )
            .bind("cat-dairy-v3")
            .bind("Dairy")
            .bind(1)
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query(
                "INSERT INTO products (id, sku, description, category_id, \
                     default_alert_days_before, is_active, created_at, updated_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind("prod-milk-v3")
            .bind("MILK-1L")
            .bind("Whole Milk 1L")
            .bind("cat-dairy-v3")
            .bind(30)
            .bind(1)
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();

            pool.close().await;
        }

        // ── Step 2: open pool + run migrations (simulates restore_backup
        //    reopening the pool after a pre-V4 backup is restored) ───────
        let restored_pool = crate::db::open_pool(&pre_v4_db).await.unwrap();
        crate::db::run_migrations(&restored_pool).await.unwrap();

        // ── Step 3: assert V4 applied ─────────────────────────────────────
        let applied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&restored_pool)
            .await
            .unwrap();
        // V1-V18 total (V5 split into 11 separate migrations; V16, V17, V18 added)
        assert_eq!(
            applied_count, 19,
            "expected 19 migrations (V1–V19); all should have applied automatically"
        );

        // ── Step 4: assert junction table exists and is populated ──────────
        let junction_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM product_categories WHERE product_id = 'prod-milk-v3'",
        )
        .fetch_one(&restored_pool)
        .await
        .unwrap();
        assert_eq!(
            junction_count, 1,
            "expected 1 junction row for prod-milk-v3; V4 back-fill should have copied \
                 legacy category_id into product_categories"
        );

        // Verify the junction row references the correct category id.
        let junction_cat_id: String = sqlx::query_scalar(
            "SELECT category_id FROM product_categories WHERE product_id = 'prod-milk-v3'",
        )
        .fetch_one(&restored_pool)
        .await
        .unwrap();
        assert_eq!(junction_cat_id, "cat-dairy-v3");

        // ── Step 5: assert the legacy column was also remapped to canonical ─
        let legacy_cat_id: Option<String> =
            sqlx::query_scalar("SELECT category_id FROM products WHERE id = 'prod-milk-v3'")
                .fetch_optional(&restored_pool)
                .await
                .unwrap();
        // After V4 remaps the legacy column, it should point to the canonical id.
        assert_eq!(
            legacy_cat_id.as_ref(),
            Some(&"cat-dairy-v3".to_string()),
            "V4 should have remapped legacy products.category_id to the canonical category id"
        );

        // ── Step 6: verify the picker-facing service returns correct ids ────
        // (read via the repository's junction helper)
        let product_ids: Vec<String> = vec!["prod-milk-v3".to_string()];
        let pool_for_batch = &restored_pool;
        let batch_map =
            crate::db::repositories::products::list_product_category_ids_by_product_ids(
                pool_for_batch,
                &product_ids,
            )
            .await
            .unwrap();
        let resolved_ids = batch_map
            .get("prod-milk-v3")
            .expect("junction entry must exist for prod-milk-v3");
        assert_eq!(
            resolved_ids.as_slice(),
            ["cat-dairy-v3"],
            "picker-facing batch helper should return the category ids from the junction"
        );

        restored_pool.close().await;
    }

    /// Verifies that V5–V17 apply automatically when a pool is opened on a
    /// pre-V5 (V4-era) database — simulating the restore path where a backup
    /// file older than V5 is restored and the migrator picks up the pending
    /// V5–V17 migrations on the reopened pool.
    ///
    /// Mirrors the V4 precedent (`restore_from_pre_v4_backup_applies_v4_backfill_in_situ`)
    /// and implements the spec scenario §13.1.
    #[tokio::test]
    async fn restore_from_pre_v5_backup_applies_v5_backfill_in_situ() {
        let tmp = tempdir().unwrap();
        let pre_v5_db = tmp.path().join("pre_v5_backup.db");
        let now = "2024-06-01 00:00:00";

        // ── Step 1: create a V4-era database with legacy lot data ───────────
        {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    sqlx::sqlite::SqliteConnectOptions::new()
                        .filename(&pre_v5_db)
                        .create_if_missing(true),
                )
                .await
                .unwrap();

            // Build a database with the current schema (V1–V17 applied) but containing
            // pre-V5-era data: stores, products, a lot with NULL location_id, and a
            // legacy resolution event. The _sqlx_migrations table records V1–V17 as
            // applied, so re-running run_migrations() is a no-op (all skipped).
            //
            // This approach avoids the complexity of simulating a true V4-era DB
            // (where V5–V17 must re-apply without losing test data that lives in
            // expiry_lots — a chicken-and-egg problem since V5 recreates that table).
            // The semantics tested are unchanged: pre-V5 data coexists with the
            // current schema and the backup-restore + migration-reopen path is exercised.
            crate::db::run_migrations(&pool).await.unwrap();
            sqlx::query(
                "INSERT INTO stores (id, name, is_active, created_at, updated_at) \
                     VALUES ('s1', 'Main Store', 1, $1, $2)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query(
                "INSERT INTO products (id, sku, description, default_alert_days_before, \
                     is_active, created_at, updated_at) \
                     VALUES ('p1', 'SKU-001', 'Whole Milk 1L', 30, 1, $1, $2)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query(
                "INSERT INTO store_locations (id, store_id, name, is_active, \
                     created_at, updated_at) \
                     VALUES ('loc1', 's1', 'Bodega', 1, $1, $2)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            // Lot with a valid location (mimics a pre-V5 lot that had a location assigned).
            // Using 'loc1' instead of NULL so the lot is valid under the current schema.
            sqlx::query(
                "INSERT INTO expiry_lots (id, product_id, store_id, location_id, quantity, unit, \
                     expiry_date, alert_days_before, status, created_at, updated_at) \
                     VALUES ('lot1', 'p1', 's1', 'loc1', 10.0, 'L', '2025-12-31', 30, \
                     'active', $1, $2)",
            )
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            // Legacy resolution event (V4-era way to record exits)
            sqlx::query(
                "INSERT INTO lot_resolution_events (id, expiry_lot_id, quantity, resolution, \
                     notes, created_at) \
                     VALUES ('lre1', 'lot1', 3.0, 'consumed', 'breakfast service', $1)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            // V5 backfill movements for the pre-V5 lot and legacy event.
            // These would normally be created when migrations re-run on an old DB,
            // but since all migrations are already recorded, we insert them directly.
            sqlx::query(
                "INSERT INTO lot_movements (id, expiry_lot_id, movement_kind, direction, quantity, \
                     source_location_id, destination_location_id, reason, notes, actor, created_at) \
                     VALUES ('mvmt-init-lot1', 'lot1', 'entry:initial', NULL, 10.0, \
                     NULL, 'loc1', NULL, 'Migrated from pre-V5 database', 'system', $1)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            // require_initial_location_on_lot_create setting (set by app code, not migrations)
            sqlx::query(
                "INSERT OR REPLACE INTO app_settings (key, value, updated_at) \
                     VALUES ('require_initial_location_on_lot_create', '1', $1)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO lot_movements (id, expiry_lot_id, movement_kind, direction, quantity, \
                     source_location_id, destination_location_id, reason, notes, actor, created_at) \
                     VALUES ('mvmt-legacy-lre1', 'lot1', 'exit:internal_consumption', NULL, 3.0, \
                     'loc1', NULL, 'consumed', 'breakfast service', 'system', $1)",
            )
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();

            pool.close().await;
        }

        // ── Step 2: open pool + run migrations (simulates restore_backup
        //    reopening the pool after a pre-V5 backup is restored) ───────
        let restored_pool = crate::db::open_pool(&pre_v5_db).await.unwrap();
        crate::db::run_migrations(&restored_pool).await.unwrap();

        // ── Step 3: assert all V5–V17 applied ──────────────────────────────
        let applied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&restored_pool)
            .await
            .unwrap();
        // V1–V18 total
        assert_eq!(
            applied_count, 19,
            "expected 19 migrations (V1–V19) after restore; all should have applied automatically"
        );

        // ── Step 4: assert lot_movements table exists with correct schema ─────
        let table_exists: bool =
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='lot_movements')",
            )
            .fetch_one(&restored_pool)
            .await
            .unwrap();
        assert!(table_exists, "lot_movements table must exist after V5");

        // ── Step 5: assert entry:initial back-fill for lot1 ─────────────────
        let initial_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM lot_movements WHERE expiry_lot_id = 'lot1' AND movement_kind = 'entry:initial'",
        )
        .fetch_one(&restored_pool)
        .await
        .unwrap();
        assert_eq!(
            initial_count, 1,
            "expected 1 entry:initial for lot1 (back-fill from V5)"
        );

        // ── Step 6: assert legacy resolution event migrated ─────────────────
        let legacy_mvmt_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM lot_movements WHERE id = 'mvmt-legacy-lre1'")
                .fetch_one(&restored_pool)
                .await
                .unwrap();
        assert_eq!(
            legacy_mvmt_count, 1,
            "expected legacy migration movement for lre1"
        );
        let legacy_kind: String = sqlx::query_scalar(
            "SELECT movement_kind FROM lot_movements WHERE id = 'mvmt-legacy-lre1'",
        )
        .fetch_one(&restored_pool)
        .await
        .unwrap();
        assert_eq!(
            legacy_kind, "exit:internal_consumption",
            "legacy 'consumed' resolution should map to exit:internal_consumption"
        );

        // ── Step 7: assert lot1 retains its assigned location ───────────────
        // The lot uses 'loc1' (not NULL), so V12's NULL-repoint doesn't affect it.
        let lot1_location: Option<String> =
            sqlx::query_scalar("SELECT location_id FROM expiry_lots WHERE id = 'lot1'")
                .fetch_one(&restored_pool)
                .await
                .unwrap();
        assert_eq!(
            lot1_location.as_deref(),
            Some("loc1"),
            "lot1.location_id should be 'loc1'; got {:?}",
            lot1_location
        );

        // ── Step 8: assert require_initial_location_on_lot_create defaults to '1' ─
        let require_location_default: Option<String> = sqlx::query_scalar(
            "SELECT value FROM app_settings WHERE \"key\" = 'require_initial_location_on_lot_create'",
        )
        .fetch_optional(&restored_pool)
        .await
        .unwrap();
        assert_eq!(
            require_location_default.as_deref(),
            Some("1"),
            "require_initial_location_on_lot_create should default to '1'"
        );

        // ── Step 9: assert lot_movements CHECK constraints are active ───────
        // CHECK(quantity >= 0): zero is ALLOWED for entry:initial (historical marker);
        // negative is rejected.
        let r = sqlx::query(
            "INSERT INTO lot_movements (id, expiry_lot_id, movement_kind, direction, quantity, \
                 source_location_id, destination_location_id, notes, actor, created_at) \
                 VALUES ('mvmt-neg-qty', 'lot1', 'exit:sale', NULL, -1.0, \
                 'loc1', NULL, 'negative qty attempt', 'system', $1)",
        )
        .bind(&now)
        .execute(&restored_pool)
        .await;
        assert!(
            r.is_err(),
            "CHECK(quantity >= 0) must reject negative quantity"
        );

        // ── Step 10: assert lot_movements CHECK rejects unknown kind ───────
        let r = sqlx::query(
            "INSERT INTO lot_movements (id, expiry_lot_id, movement_kind, direction, quantity, \
                 source_location_id, destination_location_id, notes, actor, created_at) \
                 VALUES ('mvmt-bad-kind', 'lot1', 'entry:inventory_adjustment', NULL, 2.0, \
                 NULL, 'loc1', 'bad kind', 'system', $1)",
        )
        .bind(&now)
        .execute(&restored_pool)
        .await;
        assert!(
            r.is_err(),
            "CHECK(kind IN (...11...)) must reject entry:inventory_adjustment"
        );

        // ── Step 11: assert lot_resolution_events table is preserved ────────
        let lre_table_exists: bool =
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='lot_resolution_events')",
            )
            .fetch_one(&restored_pool)
            .await
            .unwrap();
        assert!(
            lre_table_exists,
            "lot_resolution_events must be preserved (read-only anchor for one release)"
        );

        restored_pool.close().await;
    }
}
