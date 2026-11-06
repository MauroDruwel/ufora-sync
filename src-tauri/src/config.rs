use crate::fs_util;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

static CONFIG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub sync_dir: String,
    pub interval_minutes: u64,
    pub enabled_courses: Vec<String>,
    pub conflict_strategy: String,
    pub duplicate_suffix: String,
    pub sync_links: bool,
    pub auto_start_tray: bool,
    pub last_sync_time: Option<String>,
    pub last_sync_status: Option<String>,
    #[serde(default)]
    pub show_all_courses: bool,
    #[serde(default)]
    pub show_groups: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Documents")
            .join("Ufora")
            .to_string_lossy()
            .to_string();

        Self {
            sync_dir: default_dir,
            interval_minutes: 30,
            enabled_courses: Vec::new(),
            conflict_strategy: "duplicate".to_string(),
            duplicate_suffix: "_edited".to_string(),
            sync_links: true,
            auto_start_tray: true,
            last_sync_time: None,
            last_sync_status: Some("Ready".to_string()),
            show_all_courses: false,
            show_groups: false,
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("ufora-sync")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ufora-sync")
    }
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn load_config() -> AppConfig {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load_config_unlocked()
}

fn load_config_unlocked() -> AppConfig {
    let path = get_config_path();
    if !path.exists() {
        return AppConfig::default();
    }

    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data)
            .ok()
            .filter(|cfg| validate_config(cfg).is_ok())
            .unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    save_config_unlocked(cfg)
}

/// Persist user-controlled settings without allowing a stale frontend snapshot to roll back
/// sync metadata written by the background engine.
pub fn save_user_config(new_cfg: &AppConfig) -> Result<AppConfig, String> {
    update_config(|current| {
        let last_sync_time = current.last_sync_time.clone();
        let last_sync_status = current.last_sync_status.clone();
        *current = new_cfg.clone();
        current.last_sync_time = last_sync_time;
        current.last_sync_status = last_sync_status;
    })
}

/// Atomically mutate and save the configuration while holding the process-wide config lock.
pub fn update_config<F>(update: F) -> Result<AppConfig, String>
where
    F: FnOnce(&mut AppConfig),
{
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut cfg = load_config_unlocked();
    update(&mut cfg);
    save_config_unlocked(&cfg)?;
    Ok(cfg)
}

fn save_config_unlocked(cfg: &AppConfig) -> Result<(), String> {
    validate_config(cfg)?;
    let dir = get_config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let path = get_config_path();
    let data = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    let tmp_path = dir.join(format!(".config.json.tmp-{}", std::process::id()));
    let mut tmp = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&tmp_path)
        .map_err(|e| format!("Failed to open temporary config: {e}"))?;
    tmp.write_all(data.as_bytes())
        .map_err(|e| format!("Failed to write temporary config: {e}"))?;
    tmp.sync_all()
        .map_err(|e| format!("Failed to flush temporary config: {e}"))?;
    drop(tmp);
    fs_util::atomic_replace(&tmp_path, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to replace config: {e}")
    })?;
    Ok(())
}

fn validate_config(cfg: &AppConfig) -> Result<(), String> {
    if cfg.sync_dir.trim().is_empty() || cfg.sync_dir.chars().any(char::is_control) {
        return Err("Sync directory cannot be empty or contain control characters.".to_string());
    }
    if !(1..=24 * 60).contains(&cfg.interval_minutes) {
        return Err("Sync interval must be between 1 minute and 24 hours.".to_string());
    }
    if !matches!(
        cfg.conflict_strategy.as_str(),
        "duplicate" | "skip" | "overwrite"
    ) {
        return Err("Invalid conflict strategy.".to_string());
    }
    if cfg.enabled_courses.len() > 10_000
        || cfg.enabled_courses.iter().any(|course_id| {
            course_id.is_empty() || course_id.len() > 256 || course_id.chars().any(char::is_control)
        })
    {
        return Err("Course selection contains an invalid course identifier.".to_string());
    }
    if cfg.duplicate_suffix.is_empty()
        || cfg.duplicate_suffix.len() > 64
        || cfg
            .duplicate_suffix
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
        || matches!(cfg.duplicate_suffix.as_str(), "." | "..")
    {
        return Err(
            "Duplicate suffix must be 1-64 characters and cannot contain path separators."
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.interval_minutes, 30);
        assert_eq!(cfg.conflict_strategy, "duplicate");
        assert_eq!(cfg.duplicate_suffix, "_edited");
        assert!(cfg.sync_links);
        assert!(cfg.auto_start_tray);
        assert!(cfg.sync_dir.contains("Ufora"));
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = AppConfig {
            sync_dir: "/tmp/ufora_test".to_string(),
            interval_minutes: 15,
            enabled_courses: vec!["course_1".to_string(), "course_2".to_string()],
            conflict_strategy: "skip".to_string(),
            duplicate_suffix: "_custom".to_string(),
            sync_links: false,
            auto_start_tray: false,
            last_sync_time: Some("2026-10-05T20:00:00Z".to_string()),
            last_sync_status: Some("Success".to_string()),
            show_all_courses: false,
            show_groups: false,
        };

        let json = serde_json::to_string(&cfg).expect("Serialization failed");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(deserialized.sync_dir, "/tmp/ufora_test");
        assert_eq!(deserialized.interval_minutes, 15);
        assert_eq!(deserialized.enabled_courses.len(), 2);
        assert_eq!(deserialized.conflict_strategy, "skip");
        assert_eq!(deserialized.duplicate_suffix, "_custom");
        assert_eq!(deserialized.last_sync_status.as_deref(), Some("Success"));
    }

    #[test]
    fn test_config_validation_rejects_unsafe_values() {
        let cfg = AppConfig {
            sync_dir: " ".to_string(),
            ..AppConfig::default()
        };
        assert!(validate_config(&cfg).is_err());

        let cfg = AppConfig {
            interval_minutes: 0,
            ..AppConfig::default()
        };
        assert!(validate_config(&cfg).is_err());

        let cfg = AppConfig {
            conflict_strategy: "delete".to_string(),
            ..AppConfig::default()
        };
        assert!(validate_config(&cfg).is_err());

        let cfg = AppConfig {
            duplicate_suffix: "../../notes".to_string(),
            ..AppConfig::default()
        };
        assert!(validate_config(&cfg).is_err());

        let cfg = AppConfig {
            enabled_courses: vec!["bad\ncourse".to_string()],
            ..AppConfig::default()
        };
        assert!(validate_config(&cfg).is_err());
    }
}
