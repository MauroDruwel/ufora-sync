pub mod api;
pub mod auth;
pub mod commands;
pub mod config;
pub mod fs_util;
pub mod manifest;
pub mod sanitize;
pub mod sync;
pub mod tray;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

fn sync_is_due(cfg: &config::AppConfig) -> bool {
    let Some(last_sync_time) = cfg.last_sync_time.as_deref() else {
        return true;
    };
    let Ok(last_sync_time) = chrono::DateTime::parse_from_rfc3339(last_sync_time) else {
        return true;
    };
    let interval_seconds = cfg.interval_minutes.saturating_mul(60).min(i64::MAX as u64) as i64;
    chrono::Utc::now()
        .signed_duration_since(last_sync_time.with_timezone(&chrono::Utc))
        .num_seconds()
        >= interval_seconds
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            // Sync autostart configuration with system
            let cfg = config::load_config();
            use tauri_plugin_autostart::ManagerExt;
            let manager = app.autolaunch();
            let system_autostart = manager.is_enabled().unwrap_or(false);
            if cfg.auto_start_tray && !system_autostart {
                let _ = manager.enable();
            } else if !cfg.auto_start_tray && system_autostart {
                let _ = manager.disable();
            }

            // Check if launched with --minimized flag (from autostart LaunchAgent)
            let is_minimized = std::env::args().any(|a| a == "--minimized");
            if is_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{
                    apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState,
                };
                if let Some(window) = app.get_webview_window("main") {
                    let _ = apply_vibrancy(
                        &window,
                        NSVisualEffectMaterial::Sidebar,
                        Some(NSVisualEffectState::Active),
                        Some(18.0),
                    );
                }

                use tauri::menu::{AboutMetadataBuilder, Menu, PredefinedMenuItem, Submenu};
                let about_meta = AboutMetadataBuilder::new()
                    .name(Some("Ufora Sync"))
                    .version(Some(env!("CARGO_PKG_VERSION")))
                    .authors(Some(vec!["Mauro Druwel".to_string()]))
                    .copyright(Some("MIT License · Mauro Druwel"))
                    .comments(Some("Mauro Druwel — MIT License\nNative background sync service for UGent Ufora"))
                    .website(Some("https://github.com/MauroDruwel/ufora-sync"))
                    .website_label(Some("GitHub Repository"))
                    .license(Some("MIT"))
                    .build();

                let app_sub = Submenu::with_items(
                    app.handle(),
                    "Ufora Sync",
                    true,
                    &[
                        &PredefinedMenuItem::about(app.handle(), Some("About Ufora Sync"), Some(about_meta))?,
                        &PredefinedMenuItem::separator(app.handle())?,
                        &PredefinedMenuItem::services(app.handle(), None)?,
                        &PredefinedMenuItem::separator(app.handle())?,
                        &PredefinedMenuItem::hide(app.handle(), None)?,
                        &PredefinedMenuItem::hide_others(app.handle(), None)?,
                        &PredefinedMenuItem::show_all(app.handle(), None)?,
                        &PredefinedMenuItem::separator(app.handle())?,
                        &PredefinedMenuItem::quit(app.handle(), None)?,
                    ],
                )?;

                let edit_sub = Submenu::with_items(
                    app.handle(),
                    "Edit",
                    true,
                    &[
                        &PredefinedMenuItem::undo(app.handle(), None)?,
                        &PredefinedMenuItem::redo(app.handle(), None)?,
                        &PredefinedMenuItem::separator(app.handle())?,
                        &PredefinedMenuItem::cut(app.handle(), None)?,
                        &PredefinedMenuItem::copy(app.handle(), None)?,
                        &PredefinedMenuItem::paste(app.handle(), None)?,
                        &PredefinedMenuItem::select_all(app.handle(), None)?,
                    ],
                )?;

                let win_sub = Submenu::with_items(
                    app.handle(),
                    "Window",
                    true,
                    &[
                        &PredefinedMenuItem::minimize(app.handle(), None)?,
                        &PredefinedMenuItem::fullscreen(app.handle(), None)?,
                        &PredefinedMenuItem::separator(app.handle())?,
                        &PredefinedMenuItem::close_window(app.handle(), None)?,
                    ],
                )?;

                let menu = Menu::with_items(app.handle(), &[&app_sub, &edit_sub, &win_sub])?;
                let _ = app.set_menu(menu);
            }

            let sync_engine = Arc::new(sync::SyncEngine::new(app.handle().clone()));
            app.manage(Arc::clone(&sync_engine));

            // Initialize tray
            let _ = tray::init(app.handle());

            // Background interval scheduler
            let engine_bg = Arc::clone(&sync_engine);
            tauri::async_runtime::spawn(async move {
                // Initial check after 5 seconds
                tokio::time::sleep(Duration::from_secs(5)).await;
                let cfg = config::load_config();
                if !engine_bg.is_paused.load(Ordering::Relaxed) && sync_is_due(&cfg) {
                    let _ = engine_bg.run_sync_pass(&cfg).await;
                }

                loop {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    let current_cfg = config::load_config();
                    if !engine_bg.is_paused.load(Ordering::Relaxed) && sync_is_due(&current_cfg) {
                        let _ = engine_bg.run_sync_pass(&current_cfg).await;
                    }
                }
            });

            // Silent token keep-alive background task: proactively renews before expiry
            let app_keepalive = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(300)).await;
                    if let Ok(token) = auth::read_token() {
                        if let Some(exp) = token.exp {
                            let now = chrono::Utc::now().timestamp() as f64;
                            if now + 900.0 >= exp {
                                let _ = auth::ensure_valid_token(&app_keepalive, 180.0).await;
                            }
                        }
                    }
                }
            });

            Ok(())
        });

    let app = builder
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_auth_status,
            commands::open_login,
            commands::refresh_session,
            commands::logout,
            commands::list_courses,
            commands::sync_now,
            commands::pause_sync,
            commands::resume_sync,
            commands::get_recent_logs,
            commands::open_folder,
            commands::open_course_folder,
            commands::open_url,
            commands::check_for_updates,
            commands::get_autostart_status,
            commands::set_autostart,
        ])
        .build(tauri::generate_context!())
        .expect("error while building ufora-sync application");

    app.run(|_app_handle, _event| {});
}
