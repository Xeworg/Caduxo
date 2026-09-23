//! Desktop lifecycle wiring — tray icon + close-window handler.
//!
//! Implements the `close_behavior` setting (`minimize_to_tray` /
//! `exit_application`) introduced by `scanner-quick-operations` PR 3.
//!
//! ## Architecture
//!
//! - The tray icon is built once during Tauri `setup` using the built-in
//!   [`tauri::tray::TrayIconBuilder`] API (no `tauri-plugin-tray`
//!   dependency). The menu carries two items, `Restore` and `Quit`,
//!   dispatched through the `on_menu_event` handler.
//! - The `WindowEvent::CloseRequested` handler is installed on the main
//!   window via [`tauri::WebviewWindow::on_window_event`]. It reads the
//!   cached [`CloseBehavior`] from [`crate::state::AppState`] so the
//!   configured behaviour takes effect from the very first close attempt
//!   without forcing the handler to await the database pool. The IPC
//!   `update_settings` command refreshes the cache whenever the user
//!   changes the setting via ConfigurationPage, so the next close
//!   observes the new behaviour.
//! - Tray setup is best-effort: if the tray cannot be created on a
//!   platform (headless Linux, missing system tray, etc.) we log a
//!   warning and the close-window handler falls back to a normal exit so
//!   the user is never stranded with a hidden process they cannot
//!   restore. The persisted `close_behavior` setting is NOT rewritten —
//!   the fallback is runtime-only.

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

use crate::dto::stores::CloseBehavior;

/// Menu event ids for the tray menu entries.
const TRAY_MENU_RESTORE_ID: &str = "tray_restore";
const TRAY_MENU_QUIT_ID: &str = "tray_quit";

/// Tray icon id. Used to look up the tray later if a future PR exposes
/// more tray affordances.
const TRAY_ICON_ID: &str = "main";

/// Window label the close handler targets. Must match
/// `app.windows[0].label` in `tauri.conf.json` so the lifecycle code can
/// address the main window reliably.
const MAIN_WINDOW_LABEL: &str = "main";

/// Relative path of the bundled tray icon inside the resource directory.
/// Resolved at runtime against `app.path().resource_dir()` so the path
/// works both in development and in the bundled application.
const TRAY_ICON_REL_PATH: &str = "icons/32x32.png";

/// Installs the tray icon and the close-window handler.
///
/// This is the single entry point called from
/// [`crate::run`] during the Tauri `setup` phase. It MUST be called
/// after the database pool is registered via
/// [`tauri::Manager::manage`] so [`crate::state::AppState`] is resolvable
/// when the close handler reads the cached behaviour.
///
/// # Errors
///
/// Returns an error only when the main window cannot be resolved or
/// resource-dir resolution fails. Tray setup failure is logged as a
/// warning and does NOT propagate; the close handler falls back to a
/// normal exit in that case.
pub fn install(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    // ─── Tray setup (best-effort) ───────────────────────────────────────
    let tray_available = install_tray(app, &handle).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Tray icon setup failed; close-window handler falls back to normal exit");
        false
    });

    // ─── Close-window handler ───────────────────────────────────────────
    //
    // Resolve the main window, then register a per-window handler that
    // reads the cached CloseBehavior from AppState. The IPC
    // update_settings command refreshes the cache, so a user change via
    // ConfigurationPage takes effect on the next close attempt without
    // an async DB read inside the handler.
    let window = app.get_webview_window(MAIN_WINDOW_LABEL).ok_or_else(
        || -> Box<dyn std::error::Error> {
            format!("main window '{MAIN_WINDOW_LABEL}' not found").into()
        },
    )?;
    let close_handle = handle.clone();
    let window_for_close = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            let behaviour = current_close_behavior(&close_handle);
            match behaviour {
                CloseBehavior::MinimizeToTray if tray_available => {
                    api.prevent_close();
                    let _ = window_for_close.hide();
                }
                CloseBehavior::MinimizeToTray => {
                    // Tray unavailable on this platform: fall back to a
                    // normal exit so the user is not stranded with a
                    // hidden process. The persisted setting is NOT
                    // rewritten; this is a runtime-only fallback.
                    tracing::warn!(
                        "close_behavior is minimize_to_tray but tray is unavailable; exiting normally"
                    );
                }
                CloseBehavior::ExitApplication => {
                    // Do NOT call prevent_close() — let the existing
                    // Tauri shutdown path run so the database pool close
                    // hooks and exit code 0 are honoured.
                }
            }
        }
    });

    Ok(())
}

/// Installs the tray icon. Returns `Ok(true)` if the tray is available
/// for the runtime session, `Ok(false)` if the icon was not loaded but
/// the build still succeeded, and `Err(_)` only when the tray menu could
/// not be built at all.
fn install_tray(
    app: &tauri::App,
    handle: &tauri::AppHandle,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Resolve the tray icon. Failure to load the icon does NOT abort
    // tray setup — Linux in particular accepts a tray without an icon
    // as long as a menu is attached (see Tauri docs).
    let tray_icon = load_tray_icon(app);

    // ─── Tray menu ──────────────────────────────────────────────────────
    let restore_item = MenuItemBuilder::with_id(TRAY_MENU_RESTORE_ID, "Restore").build(handle)?;
    let quit_item = MenuItemBuilder::with_id(TRAY_MENU_QUIT_ID, "Quit").build(handle)?;
    let menu = MenuBuilder::new(handle)
        .items(&[&restore_item, &quit_item])
        .build()?;

    // ─── Tray icon ──────────────────────────────────────────────────────
    //
    // We always set a menu (Linux requires it for visibility) and use
    // the cached icon when available. Failure here falls back safely
    // per the `close on a platform without tray support fails safely`
    // spec scenario.
    let menu_for_event = menu.clone();
    let handle_for_menu = handle.clone();
    let mut builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            TRAY_MENU_RESTORE_ID => {
                if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            TRAY_MENU_QUIT_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| {
            // On Windows, auto-showing the menu and focusing the window on
            // the same click races the native popup menu and dismisses it.
            // Keep the platform-standard split: left click restores the
            // window, right click opens the tray menu.
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                if let Some(window) = tray.app_handle().get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            // Keep the menu clone alive for the lifetime of the tray by
            // capturing it (a workaround for the `tray-icon` ownership
            // model where the menu is moved into the tray on Linux).
            let _ = &menu_for_event;
            let _ = &handle_for_menu;
        });
    if let Some(icon) = tray_icon {
        builder = builder.icon(icon);
    } else {
        tracing::warn!(
            "Tray icon PNG not loaded; tray will be built without an icon (Linux fallback)"
        );
    }
    builder.build(app)?;
    tracing::info!("Tray icon installed");
    Ok(true)
}

/// Reads the cached close-window behaviour from [`crate::state::AppState`].
/// Falls back to [`CloseBehavior::MinimizeToTray`] when the state is
/// missing (should never happen post-setup) or the cache lock is
/// poisoned.
fn current_close_behavior(handle: &tauri::AppHandle) -> CloseBehavior {
    match handle.try_state::<crate::state::AppState>() {
        Some(state) => state.close_behavior(),
        None => {
            tracing::warn!("AppState missing during close-handler read; using safe fallback");
            CloseBehavior::MinimizeToTray
        }
    }
}

/// Resolves the tray icon from the bundled resources. In development,
/// `resource_dir()` points at `src-tauri/target/debug`, where Tauri does
/// not copy the bundle icon files, so we fall back to the compile-time
/// icon bytes generated under `src-tauri/icons/`.
fn load_tray_icon(app: &tauri::App) -> Option<Image<'static>> {
    let resource_dir = match app.path().resource_dir() {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to resolve resource_dir for tray icon");
            return load_embedded_tray_icon();
        }
    };
    let icon_path = resource_dir.join(TRAY_ICON_REL_PATH);
    match Image::from_path(&icon_path) {
        Ok(img) => Some(img),
        Err(e) => {
            tracing::debug!(
                path = %icon_path.display(),
                error = %e,
                "Bundled tray icon path unavailable; falling back to embedded icon bytes"
            );
            load_embedded_tray_icon()
        }
    }
}

/// Loads the generated 32×32 tray icon directly from the source tree at
/// compile time. This keeps `tauri dev` and `cargo run` from depending on
/// `target/debug/icons/32x32.png` existing on disk.
fn load_embedded_tray_icon() -> Option<Image<'static>> {
    match Image::from_bytes(include_bytes!("../icons/32x32.png")) {
        Ok(img) => Some(img),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to decode embedded tray icon");
            None
        }
    }
}

// ─── Local imports ────────────────────────────────────────────────────────
//
// `WindowEvent` is referenced by the close handler closure above; the
// `use` statement lives here so the import block at the top of the file
// stays grouped with the other tauri imports.
use tauri::WindowEvent;
