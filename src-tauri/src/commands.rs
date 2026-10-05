use crate::api::{self, Course};
use crate::config::{self, AppConfig};
use crate::sync::SyncEngine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub user_id: Option<String>,
    pub message: String,
}

#[tauri::command]
pub fn get_config() -> AppConfig {
    config::load_config()
}

#[tauri::command]
pub fn save_config(new_cfg: AppConfig, engine: State<'_, Arc<SyncEngine>>) -> Result<(), String> {
    config::save_config(&new_cfg)?;
    engine.log("Settings saved.");
    Ok(())
}

#[tauri::command]
pub fn get_auth_status() -> AuthStatus {
    match api::read_token() {
        Ok(t) => {
            if api::is_token_expired(&t) {
                AuthStatus {
                    is_authenticated: false,
                    user_id: t.sub,
                    message: "Session expired".to_string(),
                }
            } else {
                AuthStatus {
                    is_authenticated: true,
                    user_id: t.sub,
                    message: "Authenticated".to_string(),
                }
            }
        }
        Err(e) => AuthStatus {
            is_authenticated: false,
            user_id: None,
            message: e,
        },
    }
}

#[tauri::command]
pub async fn list_courses() -> Result<Vec<Course>, String> {
    let token = api::read_token()?;
    if api::is_token_expired(&token) {
        return Err("Session expired. Please log in again.".to_string());
    }
    api::fetch_enrolled_courses(&token.token).await
}

#[tauri::command]
pub fn sync_now(engine: State<'_, Arc<SyncEngine>>) -> Result<(), String> {
    let engine_clone = Arc::clone(&engine);
    tauri::async_runtime::spawn(async move {
        let cfg = config::load_config();
        let _ = engine_clone.run_sync_pass(&cfg).await;
    });
    Ok(())
}

#[tauri::command]
pub fn pause_sync(engine: State<'_, Arc<SyncEngine>>) {
    engine.is_paused.store(true, Ordering::SeqCst);
    engine.log("Sync paused by user.");
}

#[tauri::command]
pub fn resume_sync(engine: State<'_, Arc<SyncEngine>>) {
    engine.is_paused.store(false, Ordering::SeqCst);
    engine.log("Sync resumed by user.");
}

#[tauri::command]
pub fn get_recent_logs() -> Vec<String> {
    let log_path = config::get_config_dir().join("activity.log");
    if !log_path.exists() {
        return vec!["[Ready] No prior sync logs found.".to_string()];
    }

    match fs::read_to_string(log_path) {
        Ok(content) => {
            let lines: Vec<String> = content
                .lines()
                .rev()
                .take(60)
                .map(|s| s.to_string())
                .collect();
            let mut forward = lines;
            forward.reverse();
            forward
        }
        Err(_) => vec![],
    }
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("Failed to open folder: {e}"))
}

#[tauri::command]
pub fn open_course_folder(folder_name: String) -> Result<(), String> {
    let cfg = config::load_config();
    let course_dir = PathBuf::from(&cfg.sync_dir).join(&folder_name);
    let _ = fs::create_dir_all(&course_dir);
    open::that(&course_dir).map_err(|e| format!("Failed to open folder: {e}"))
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open link: {e}"))
}
