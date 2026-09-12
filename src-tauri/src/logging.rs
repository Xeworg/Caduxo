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

use std::path::PathBuf;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    EnvFilter,
};

/// Resolves the app-local log directory path.
///
/// Creates the directory if it does not exist.
/// Returns `None` if resolution fails.
pub fn resolve_log_dir(app_data: &PathBuf) -> Option<PathBuf> {
    let log_dir = app_data.join("logs");
    std::fs::create_dir_all(&log_dir).ok()?;
    Some(log_dir)
}

/// Initializes the tracing subscriber.
///
/// - `log_dir`: directory for the rotating log file (e.g. `logs/`)
/// - `app_version`: current application version for log metadata
/// - `is_dev`: whether the app is running in development mode
pub fn init(log_dir: Option<&PathBuf>, app_version: &str, is_dev: bool) {
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
