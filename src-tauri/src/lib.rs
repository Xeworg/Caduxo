//! Caduxo — Product expiry tracker library.

mod commands;
mod db;
mod domain;
mod dto;
mod error;
mod logging;
mod pdf;
mod services;
mod state;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::db::{open_pool, run_migrations};
use crate::logging::resolve_log_dir;
use crate::state::AppState;

/// Database file name within the app data directory.
const DB_FILE_NAME: &str = "caduxo.db";

/// Runs the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            tauri::async_runtime::block_on(async_init(app)).map_err(|e| {
                tracing::error!(error = %e, "Setup failed");
                e
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::health_check,
            commands::stores::is_first_run,
            commands::stores::list_stores,
            commands::stores::create_store,
            commands::stores::update_store,
            commands::stores::list_store_locations,
            commands::stores::create_store_location,
            commands::stores::update_store_location,
            commands::stores::get_settings,
            commands::stores::update_settings,
            commands::stores::has_store,
            commands::products::list_categories,
            commands::products::list_categories_search,
            commands::products::create_category,
            commands::products::update_category,
            commands::products::create_product,
            commands::products::update_product,
            commands::products::archive_product,
            commands::products::get_product,
            commands::products::search_products,
            commands::products::suggested_product_alert_days,
            commands::products::find_product_by_scan,
            commands::products::add_product_barcode,
            commands::products::list_product_barcodes,
            commands::products::remove_product_barcode,
            // Expiry lots (Slice 5a)
            commands::expiry_lots::list_expiry_lots,
            commands::expiry_lots::list_expiry_lots_by_store,
            commands::expiry_lots::list_expiry_lots_by_product,
            commands::expiry_lots::get_expiry_lot,
            commands::expiry_lots::create_expiry_lot,
            commands::expiry_lots::update_expiry_lot,
            commands::expiry_lots::archive_expiry_lot,
            commands::expiry_lots::resolve_expiry_lot,
            commands::expiry_lots::list_lot_resolution_events,
            // Lot movements
            commands::lot_movements::create_lot_movement,
            commands::lot_movements::list_lot_movements,
            commands::lot_movements::get_lot_location_balances,
            // Dashboard (Slice 6a)
            commands::dashboard::list_dashboard_lots,
            // Local notifications (Slice 7 — backend only)
            commands::notifications::list_due_notifications,
            commands::notifications::mark_notification_shown,
            // CSV import/export (Slice 10a: preview + exports; Slice 10b: import commit)
            commands::csv_io::read_csv_text,
            commands::csv_io::preview_product_csv,
            commands::csv_io::import_product_csv,
            commands::csv_io::export_products_csv,
            commands::csv_io::export_report_csv,
            // Reports (Slice 11a — backend report data + custom filters;
            // Slice 11b — preview UI + PDF export)
            commands::reports::preview_report,
            commands::reports::export_report_pdf,
            // Backup and restore (Slice 12)
            commands::backup_restore::export_backup,
            commands::backup_restore::validate_backup,
            commands::backup_restore::restore_backup,
            // Unit definitions catalog (caduxo-measurement-unit-options)
            commands::unit_definitions::list_unit_definitions,
            commands::unit_definitions::create_unit_definition,
            commands::unit_definitions::rename_unit_definition,
            commands::unit_definitions::list_unrecognized_units,
            commands::unit_definitions::unit_audit_banner_state,
            commands::unit_definitions::dismiss_unit_audit_banner,
            commands::unit_definitions::apply_unit_review_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Initializes all application-level dependencies.
async fn async_init(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let is_dev = cfg!(debug_assertions);

    // 1. Resolve app-local data directory.
    let app_data = app
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    std::fs::create_dir_all(&app_data).expect("failed to create app data directory");

    // 2. Initialize structured logging.
    let log_dir = resolve_log_dir(&app_data);
    let version = env!("CARGO_PKG_VERSION");
    logging::init(log_dir.as_deref(), version, is_dev);
    tracing::info!(data_dir = %app_data.display(), "App data directory resolved");

    // 3. Initialize the database pool.
    let db_path = app_data.join(DB_FILE_NAME);
    tracing::info!(db_path = %db_path.display(), "Opening database");
    let pool = open_pool(&db_path).await?;
    tracing::info!("Database pool initialized");

    // 4. Run migrations.
    run_migrations(&pool).await?;
    tracing::info!("Migrations applied");

    // 5. Register application state with Tauri.
    // The pool is wrapped in Arc<Mutex<>> so it can be replaced atomically on restore.
    app.manage(AppState::new(Arc::new(Mutex::new(pool)), db_path));
    tracing::info!("Caduxo application started successfully");
    Ok(())
}
