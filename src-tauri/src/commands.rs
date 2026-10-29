use crate::api::{self, Course};
use crate::auth;
use crate::config::{self, AppConfig};
use crate::sync::SyncEngine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{Emitter, State};

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
    match auth::read_token() {
        Ok(t) => {
            if auth::is_token_expired(&t) {
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
pub fn open_login(app: tauri::AppHandle) -> Result<(), String> {
    auth::open_login_window(&app)
}

#[tauri::command]
pub fn logout(app: tauri::AppHandle, engine: State<'_, Arc<SyncEngine>>) -> Result<(), String> {
    auth::logout()?;
    engine.log("Logged out from UGent Ufora. Local token cleared.");
    let _ = app.emit("auth-status-changed", false);
    Ok(())
}

#[tauri::command]
pub async fn list_courses() -> Result<Vec<Course>, String> {
    let token = auth::ensure_authenticated(180.0)?;
    api::fetch_enrolled_courses(&token).await
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: Option<String>,
    pub html_url: String,
    pub published_at: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = reqwest::Client::builder()
        .user_agent(format!("ufora-sync/{}", current_version))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("https://api.github.com/repos/MauroDruwel/ufora-sync/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Failed to reach GitHub: {e}"))?;

    if !res.status().is_success() {
        return Ok(UpdateInfo {
            current_version: current_version.clone(),
            latest_version: current_version,
            has_update: false,
            release_notes: None,
            html_url: "https://github.com/MauroDruwel/ufora-sync/releases".to_string(),
            published_at: None,
        });
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let raw_tag = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("");
    let tag = raw_tag.trim_start_matches('v').to_string();
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/MauroDruwel/ufora-sync/releases")
        .to_string();
    let release_notes = json
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let published_at = json
        .get("published_at")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let has_update = !tag.is_empty() && tag != current_version;

    Ok(UpdateInfo {
        current_version,
        latest_version: if tag.is_empty() {
            env!("CARGO_PKG_VERSION").to_string()
        } else {
            tag
        },
        has_update,
        release_notes,
        html_url,
        published_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_info_serialization() {
        let info = UpdateInfo {
            current_version: "1.0.0".to_string(),
            latest_version: "1.0.1".to_string(),
            has_update: true,
            release_notes: Some("Fixes and enhancements".to_string()),
            html_url: "https://github.com/MauroDruwel/ufora-sync/releases/v1.0.1".to_string(),
            published_at: Some("2026-10-29T10:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize");
        assert!(json.contains("\"has_update\":true"));
        assert!(json.contains("\"latest_version\":\"1.0.1\""));
    }
}
