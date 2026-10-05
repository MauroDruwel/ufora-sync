use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub sync_dir: String,
    pub interval_minutes: u64,
    pub enabled_courses: Vec<String>,
    pub conflict_strategy: String,
    pub duplicate_suffix: String,
    pub sync_descriptions: bool,
    pub sync_links: bool,
    pub auto_start_tray: bool,
    pub last_sync_time: Option<String>,
    pub last_sync_status: Option<String>,
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
            sync_descriptions: true,
            sync_links: true,
            auto_start_tray: true,
            last_sync_time: None,
            last_sync_status: Some("Ready".to_string()),
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
    let path = get_config_path();
    if !path.exists() {
        return AppConfig::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let path = get_config_path();
    let data = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("Failed to write config: {e}"))?;
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
        assert!(cfg.sync_descriptions);
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
            sync_descriptions: false,
            sync_links: false,
            auto_start_tray: false,
            last_sync_time: Some("2026-10-05T20:00:00Z".to_string()),
            last_sync_status: Some("Success".to_string()),
        };

        let json = serde_json::to_string(&cfg).expect("Serialization failed");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(deserialized.sync_dir, "/tmp/ufora_test");
        assert_eq!(deserialized.interval_minutes, 15);
        assert_eq!(deserialized.enabled_courses.len(), 2);
        assert_eq!(deserialized.conflict_strategy, "skip");
        assert_eq!(deserialized.duplicate_suffix, "_custom");
        assert!(!deserialized.sync_descriptions);
        assert_eq!(deserialized.last_sync_status.as_deref(), Some("Success"));
    }
}
