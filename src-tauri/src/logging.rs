//! Structured logging configuration for Caduxo.
//!
//! Logs are written to `logs/caduxo.log` inside the app-local data directory.
//! Console logging is enabled in debug builds only.
//!
//! ## Safety guidelines
//!
//! Avoid logging sensitive business data:
//! - Product names, descriptions, and SKUs
//! - Barcode values
//! - Imported CSV row contents
//! - Free-form notes fields
//! - Store names and location names (unless diagnostics require it)
//!
//! Log safe identifiers: UUIDs, internal IDs, operation names,
//! counts, durations, and error categories.

use std::path::{Path, PathBuf};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    EnvFilter,
};

/// Resolves the app-local log directory path.
///
/// Creates the directory if it does not exist.
/// Returns `None` if resolution fails.
pub fn resolve_log_dir(app_data: &Path) -> Option<PathBuf> {
    let log_dir = app_data.join("logs");
    std::fs::create_dir_all(&log_dir).ok()?;
    Some(log_dir)
}

/// Initializes the tracing subscriber.
///
/// - `log_dir`: directory for the rotating log file (e.g. `logs/`)
/// - `app_version`: current application version for log metadata
/// - `is_dev`: whether the app is running in development mode
pub fn init(log_dir: Option<&Path>, app_version: &str, is_dev: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("caduxo=info,tauri=info"));

    let file_layer = log_dir.and_then(|dir| {
        let file = dir.join("caduxo.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .ok()?;

        Some(
            fmt::layer()
                .with_writer(std::sync::Mutex::new(file))
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::CLOSE),
        )
    });

    let console_layer = if is_dev {
        Some(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_span_events(FmtSpan::CLOSE),
        )
    } else {
        None
    };

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(console_layer);

    // Suppress the default `console_error_panic_hook` warning if already set.
    // Use the global `set_global_default` which is safe after a single thread starts.
    let _ = tracing::subscriber::set_global_default(subscriber);

    tracing::info!(
        version = %app_version,
        log_dir = ?log_dir.as_ref().map(|d| d.display().to_string()),
        dev_mode = is_dev,
        "Caduxo logging initialized"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_log_dir_creates_logs_subdirectory() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let app_data = temp_dir.path();

        let result = resolve_log_dir(app_data);
        assert!(
            result.is_some(),
            "resolve_log_dir should return Some for a valid app_data path"
        );

        let log_dir = result.expect("log_dir is Some");
        assert_eq!(
            log_dir,
            app_data.join("logs"),
            "log_dir should be app_data/logs"
        );

        // Verify the directory was actually created on disk
        assert!(
            log_dir.is_dir(),
            "logs directory should be created on the filesystem"
        );
    }

    #[test]
    fn resolve_log_dir_idempotent_when_already_exists() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let app_data = temp_dir.path();

        // First call creates it
        let first = resolve_log_dir(app_data).expect("first call succeeds");
        assert!(first.is_dir());

        // Second call also succeeds (idempotent)
        let second = resolve_log_dir(app_data).expect("second call succeeds");
        assert_eq!(first, second, "second call should return the same path");
    }

    #[test]
    fn resolve_log_dir_nested_path() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        // Simulate nested app data: ~/.local/share/Caduxo
        let nested = temp_dir.path().join("subdir").join("nested");

        let result = resolve_log_dir(&nested);
        assert!(
            result.is_some(),
            "resolve_log_dir should work with nested paths"
        );

        let log_dir = result.expect("log_dir is Some");
        assert_eq!(
            log_dir,
            nested.join("logs"),
            "log_dir should be nested/logs"
        );
        assert!(log_dir.is_dir(), "nested logs directory should be created");
    }
}
