//! Caduxo — Product expiry tracker library.

mod commands;
mod db;
mod domain;
mod dto;
mod error;
mod logging;
mod services;
mod state;

use std::sync::Arc;
use tauri::Manager;

use crate::db::{open_pool, run_migrations};
use crate::logging::resolve_log_dir;
use crate::state::AppState;

/// Runs the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async_init(app)).map_err(|e| {
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
    let pool = open_pool(&app_data).await?;
    tracing::info!("Database pool initialized");

    // 4. Run migrations.
    run_migrations(&pool).await?;
    tracing::info!("Migrations applied");

    // 5. Register application state with Tauri.
    app.manage(AppState::new(Arc::new(pool)));
    tracing::info!("Caduxo application started successfully");
    Ok(())
}
