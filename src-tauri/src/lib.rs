pub mod api;
pub mod commands;
pub mod config;
pub mod sync;
pub mod tray;

use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
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
                let _ = engine_bg.run_sync_pass(&cfg).await;

                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    let current_cfg = config::load_config();
                    let interval_secs = current_cfg.interval_minutes * 60;
                    // Trigger pass if not already syncing
                    tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                    let _ = engine_bg.run_sync_pass(&current_cfg).await;
                }
            });

            Ok(())
        });

    let app = builder
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_auth_status,
            commands::list_courses,
            commands::sync_now,
            commands::pause_sync,
            commands::resume_sync,
            commands::get_recent_logs,
            commands::open_folder,
            commands::open_course_folder,
            commands::open_url,
        ])
        .build(tauri::generate_context!())
        .expect("error while building ufora-sync application");

    app.run(|_app_handle, _event| {});
}
