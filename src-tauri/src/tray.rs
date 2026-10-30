use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "ufora-sync-main-tray";
static TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-icon.png");

fn tray_icon() -> Option<tauri::image::Image<'static>> {
    tauri::image::Image::from_bytes(TRAY_ICON_BYTES).ok()
}

pub fn init(app: &AppHandle) -> Result<(), String> {
    let menu = build_menu(app).map_err(|e| e.to_string())?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Ufora Sync — Background Service")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "sync_now" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if !crate::auth::is_authenticated_and_valid(180.0) {
                        let refreshed = crate::auth::try_ufora_cli_refresh().await;
                        if !refreshed || !crate::auth::is_authenticated_and_valid(180.0) {
                            let _ = crate::auth::open_login_window(&handle);
                            return;
                        }
                    }
                    if let Some(engine) = handle.try_state::<Arc<crate::sync::SyncEngine>>() {
                        let cfg = crate::config::load_config();
                        let _ = engine.run_sync_pass(&cfg).await;
                    }
                });
            }
            "open_folder" => {
                let cfg = crate::config::load_config();
                let _ = open::that(&cfg.sync_dir);
            }
            "open_ufora" => {
                let _ = open::that("https://ufora.ugent.be");
            }
            "open_login" => {
                let _ = crate::auth::open_login_window(app);
            }
            "toggle_autostart" => {
                use tauri_plugin_autostart::ManagerExt;
                let manager = app.autolaunch();
                let current = manager.is_enabled().unwrap_or(false);
                let new_state = !current;
                if new_state {
                    let _ = manager.enable();
                } else {
                    let _ = manager.disable();
                }
                let mut cfg = crate::config::load_config();
                cfg.auto_start_tray = new_state;
                let _ = crate::config::save_config(&cfg);
                let _ = update_tray_menu(app);
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = tray_icon().or_else(|| app.default_window_icon().cloned()) {
        builder = builder.icon(icon).icon_as_template(true);
    }

    builder.build(app).map_err(|e| e.to_string())?;
    Ok(())
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let is_auth = crate::auth::is_authenticated_and_valid(180.0);
    let engine = app.try_state::<Arc<crate::sync::SyncEngine>>();
    let is_syncing = engine
        .as_ref()
        .map(|e| e.is_running.load(std::sync::atomic::Ordering::SeqCst))
        .unwrap_or(false);

    let status_str = if is_syncing {
        "🔄 Ufora Sync — Syncing…".to_string()
    } else if !is_auth {
        "⚠️ Ufora Sync — Sign-in Required".to_string()
    } else {
        let cfg = crate::config::load_config();
        if let Some(ref t) = cfg.last_sync_time {
            let parsed = chrono::DateTime::parse_from_rfc3339(t).ok();
            if let Some(dt) = parsed {
                let local_time = dt.with_timezone(&chrono::Local);
                format!("● Ufora Sync — Up to date ({})", local_time.format("%H:%M"))
            } else {
                "● Ufora Sync — Up to date".to_string()
            }
        } else {
            "● Ufora Sync — Ready".to_string()
        }
    };

    let status_item = MenuItem::with_id(app, "status", status_str, false, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sync_item = MenuItem::with_id(app, "sync_now", "🔄 Sync Now", true, None::<&str>)?;
    let open_folder = MenuItem::with_id(
        app,
        "open_folder",
        "📂 Open Sync Folder",
        true,
        None::<&str>,
    )?;
    let open_ufora = MenuItem::with_id(
        app,
        "open_ufora",
        "🌐 Open Ufora in Browser",
        true,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let settings_item =
        MenuItem::with_id(app, "show", "⚙️ Settings & Courses…", true, None::<&str>)?;

    let is_autostart = {
        use tauri_plugin_autostart::ManagerExt;
        app.autolaunch().is_enabled().unwrap_or(false)
    };
    let autostart_item = CheckMenuItem::with_id(
        app,
        "toggle_autostart",
        "Start at Login",
        true,
        is_autostart,
        None::<&str>,
    )?;

    let about_meta = tauri::menu::AboutMetadataBuilder::new()
        .name(Some("Ufora Sync"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .authors(Some(vec!["Mauro Druwel".to_string()]))
        .copyright(Some("MIT License · Mauro Druwel"))
        .comments(Some(
            "Mauro Druwel — MIT License\nNative background sync service for UGent Ufora",
        ))
        .website(Some("https://github.com/MauroDruwel/ufora-sync"))
        .website_label(Some("GitHub Repository"))
        .license(Some("MIT"))
        .build();
    let about_item = PredefinedMenuItem::about(app, Some("About Ufora Sync"), Some(about_meta))?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "❌ Quit Ufora Sync", true, None::<&str>)?;

    if !is_auth {
        let login_item = MenuItem::with_id(
            app,
            "open_login",
            "🔑 Sign in to UGent Ufora…",
            true,
            None::<&str>,
        )?;
        Menu::with_items(
            app,
            &[
                &status_item,
                &sep1,
                &login_item,
                &sync_item,
                &open_folder,
                &open_ufora,
                &sep2,
                &settings_item,
                &autostart_item,
                &about_item,
                &sep3,
                &quit_item,
            ],
        )
    } else {
        Menu::with_items(
            app,
            &[
                &status_item,
                &sep1,
                &sync_item,
                &open_folder,
                &open_ufora,
                &sep2,
                &settings_item,
                &autostart_item,
                &about_item,
                &sep3,
                &quit_item,
            ],
        )
    }
}

pub fn update_tray_menu(app: &AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let menu = build_menu(app).map_err(|e| e.to_string())?;
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
