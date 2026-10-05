use crate::api;
use crate::config::{get_config_dir, AppConfig};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

const MANIFEST_FILENAME: &str = ".ufora_sync_manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedFile {
    pub remote_id: String,
    pub remote_path: String,
    pub local_path: String,
    pub sha256: String,
    pub synced_at: String,
    pub remote_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncManifest {
    pub files: HashMap<String, SyncedFile>,
}

impl SyncManifest {
    pub fn load(course_dir: &Path) -> Self {
        let manifest_path = course_dir.join(MANIFEST_FILENAME);
        if !manifest_path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&manifest_path) {
            Ok(content) => {
                #[derive(Deserialize)]
                struct Raw {
                    files: HashMap<String, SyncedFile>,
                }
                match serde_json::from_str::<Raw>(&content) {
                    Ok(raw) => {
                        let mut files = HashMap::new();
                        for (_k, mut entry) in raw.files {
                            // Ensure relative key
                            let clean_k = normalize_rel_path(&entry.local_path);
                            entry.local_path = clean_k.clone();
                            files.insert(clean_k, entry);
                        }
                        Self { files }
                    }
                    Err(_) => Self::default(),
                }
            }
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, course_dir: &Path) -> Result<(), String> {
        let manifest_path = course_dir.join(MANIFEST_FILENAME);
        let tmp_path = course_dir.join(format!(
            ".{MANIFEST_FILENAME}.tmp_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
        fs::rename(&tmp_path, &manifest_path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_by_remote_id(&self, remote_id: &str) -> Option<&SyncedFile> {
        self.files.values().find(|f| f.remote_id == remote_id)
    }

    pub fn is_locally_edited(&self, course_dir: &Path, rel_path: &str) -> bool {
        let clean_rel = normalize_rel_path(rel_path);
        let Some(entry) = self.files.get(&clean_rel) else {
            return false;
        };
        let full_path = course_dir.join(&clean_rel);
        if !full_path.exists() {
            return false;
        }

        if is_dataless_placeholder(&full_path) {
            return false;
        }

        match calculate_sha256(&full_path) {
            Ok(current_sha) => current_sha != entry.sha256,
            Err(_) => false,
        }
    }

    pub fn record(
        &mut self,
        remote_id: &str,
        rel_path: &str,
        sha: &str,
        remote_modified: Option<String>,
    ) {
        let clean_rel = normalize_rel_path(rel_path);
        self.files.insert(
            clean_rel.clone(),
            SyncedFile {
                remote_id: remote_id.to_string(),
                remote_path: clean_rel.clone(),
                local_path: clean_rel,
                sha256: sha.to_string(),
                synced_at: Utc::now().to_rfc3339(),
                remote_modified,
            },
        );
    }
}

pub fn normalize_rel_path(p: &str) -> String {
    let replaced = p.replace('\\', "/");
    let stripped = replaced.strip_prefix("./").unwrap_or(&replaced);
    stripped.trim_start_matches('/').to_string()
}

pub fn calculate_sha256(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn is_dataless_placeholder(_path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::darwin::fs::MetadataExt;
        if let Ok(meta) = fs::metadata(_path) {
            let flags = meta.st_flags();
            // SF_DATALESS = 0x40000000 on Darwin
            return (flags & 0x4000_0000) != 0;
        }
    }
    false
}

pub fn append_log(msg: &str) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{timestamp}] {msg}\n");
    let log_file = get_config_dir().join("activity.log");
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

pub struct SyncEngine {
    pub is_running: Arc<AtomicBool>,
    pub is_paused: Arc<AtomicBool>,
    app_handle: AppHandle,
}

impl SyncEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            app_handle,
        }
    }

    pub fn log(&self, msg: &str) {
        append_log(msg);
        let timestamp = Utc::now().format("%H:%M:%S").to_string();
        let payload = serde_json::json!({
            "timestamp": timestamp,
            "message": msg,
        });
        let _ = self.app_handle.emit("log-message", payload);
    }

    pub async fn run_sync_pass(&self, cfg: &AppConfig) -> Result<SyncSummary, String> {
        if self.is_paused.load(Ordering::Relaxed) {
            self.log("Sync skipped: syncing is currently paused.");
            return Ok(SyncSummary::default());
        }

        if self.is_running.swap(true, Ordering::SeqCst) {
            self.log("Sync already in progress, skipping duplicate trigger.");
            return Ok(SyncSummary::default());
        }

        let _ = self.app_handle.emit("sync-status", "syncing");
        self.log("Starting Ufora sync pass…");

        let token_result = api::read_token();
        let token = match token_result {
            Ok(t) => {
                if api::is_token_expired(&t) {
                    self.log("⚠ Token expired. Please re-authenticate via Ufora.");
                    self.is_running.store(false, Ordering::SeqCst);
                    let _ = self.app_handle.emit("sync-status", "error");
                    return Err("Session expired. Please log in again.".to_string());
                }
                t.token
            }
            Err(e) => {
                self.log(&format!("✗ Auth error: {e}"));
                self.is_running.store(false, Ordering::SeqCst);
                let _ = self.app_handle.emit("sync-status", "error");
                return Err(e);
            }
        };

        let base_sync_dir = PathBuf::from(&cfg.sync_dir);
        if let Err(e) = fs::create_dir_all(&base_sync_dir) {
            self.log(&format!(
                "✗ Failed to create sync directory {}: {e}",
                base_sync_dir.display()
            ));
            self.is_running.store(false, Ordering::SeqCst);
            let _ = self.app_handle.emit("sync-status", "error");
            return Err(e.to_string());
        }

        // Fetch courses
        let courses = match api::fetch_enrolled_courses(&token).await {
            Ok(c) => c,
            Err(e) => {
                self.log(&format!("✗ Failed to fetch courses: {e}"));
                self.is_running.store(false, Ordering::SeqCst);
                let _ = self.app_handle.emit("sync-status", "error");
                return Err(e);
            }
        };

        let enabled_set: std::collections::HashSet<_> =
            cfg.enabled_courses.iter().cloned().collect();
        let active_courses: Vec<_> = if enabled_set.is_empty() {
            courses
        } else {
            courses
                .into_iter()
                .filter(|c| enabled_set.contains(&c.id))
                .collect()
        };

        self.log(&format!(
            "Inspecting {} enabled course(s)…",
            active_courses.len()
        ));

        let mut total_downloaded = 0;
        let mut total_up_to_date = 0;
        let mut total_errors = 0;

        for course in active_courses {
            let course_dir = base_sync_dir.join(&course.folder_name);
            let _ = fs::create_dir_all(&course_dir);

            self.log(&format!("Scanning course: {}", course.name));
            let mut manifest = SyncManifest::load(&course_dir);
            let mut manifest_dirty = false;

            let toc = match api::fetch_course_toc(&token, &course.id).await {
                Ok(t) => t,
                Err(e) => {
                    self.log(&format!("  ✗ Failed to fetch TOC for {}: {e}", course.name));
                    total_errors += 1;
                    continue;
                }
            };

            let topics = extract_file_topics(&toc);
            self.log(&format!(
                "  Found {} downloadable material(s)",
                topics.len()
            ));

            for topic in topics {
                let target_dir = build_subfolder_path(&course_dir, &topic.module_path);
                let _ = fs::create_dir_all(&target_dir);

                let clean_title = api::sanitize_filename(&topic.title);
                let dest_path = target_dir.join(&clean_title);
                let rel_path = match dest_path.strip_prefix(&course_dir) {
                    Ok(p) => p.to_string_lossy().to_string(),
                    Err(_) => clean_title.clone(),
                };

                let existing = manifest.get_by_remote_id(&topic.id);
                if let Some(entry) = existing {
                    let full_dest = course_dir.join(&entry.local_path);
                    if full_dest.exists() && entry.remote_modified == topic.remote_modified {
                        total_up_to_date += 1;
                        continue;
                    }
                }

                // Download topic file
                match api::download_topic_file(&token, &course.id, &topic.id).await {
                    Ok(bytes) => {
                        let sha = {
                            let mut hasher = Sha256::new();
                            hasher.update(&bytes);
                            format!("{:x}", hasher.finalize())
                        };

                        if dest_path.exists() && manifest.is_locally_edited(&course_dir, &rel_path)
                        {
                            if cfg.conflict_strategy == "skip" {
                                self.log(&format!("  ↷ Kept local edit: {rel_path}"));
                                continue;
                            } else if cfg.conflict_strategy == "duplicate" {
                                let edited_path =
                                    make_conflict_path(&dest_path, &cfg.duplicate_suffix);
                                let _ = fs::rename(&dest_path, &edited_path);
                                self.log(&format!(
                                    "  ✎ Preserved edit → {}",
                                    edited_path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                ));
                            }
                        }

                        // Atomic write
                        let tmp_file = dest_path.with_file_name(format!(
                            ".{}.tmp_{}",
                            dest_path.file_name().unwrap_or_default().to_string_lossy(),
                            Utc::now().timestamp_nanos_opt().unwrap_or(0)
                        ));
                        if let Ok(mut f) = File::create(&tmp_file) {
                            let _ = f.write_all(&bytes);
                            let _ = fs::rename(&tmp_file, &dest_path);
                            manifest.record(&topic.id, &rel_path, &sha, topic.remote_modified);
                            manifest_dirty = true;
                            total_downloaded += 1;
                            self.log(&format!("  ✓ {rel_path}"));
                        }
                    }
                    Err(e) => {
                        self.log(&format!("  ✗ Error downloading item {}: {e}", topic.id));
                        total_errors += 1;
                    }
                }
            }

            if manifest_dirty {
                let _ = manifest.save(&course_dir);
            }
        }

        self.is_running.store(false, Ordering::SeqCst);
        let summary = SyncSummary {
            downloaded: total_downloaded,
            up_to_date: total_up_to_date,
            errors: total_errors,
        };

        self.log(&format!(
            "Sync complete: {} downloaded/updated, {} up to date, {} errors.",
            summary.downloaded, summary.up_to_date, summary.errors
        ));
        let _ = self.app_handle.emit("sync-status", "idle");
        let _ = self.app_handle.emit("sync-complete", &summary);

        Ok(summary)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncSummary {
    pub downloaded: usize,
    pub up_to_date: usize,
    pub errors: usize,
}

#[derive(Debug, Clone)]
pub struct ExtractedTopic {
    pub id: String,
    pub title: String,
    pub module_path: Vec<String>,
    pub remote_modified: Option<String>,
}

fn extract_file_topics(toc: &serde_json::Value) -> Vec<ExtractedTopic> {
    let mut topics = Vec::new();
    if let Some(modules) = toc.get("Modules").and_then(|v| v.as_array()) {
        for m in modules {
            walk_module(m, &[], &mut topics);
        }
    }
    topics
}

fn walk_module(module: &serde_json::Value, path: &[String], topics: &mut Vec<ExtractedTopic>) {
    let title = module
        .get("Title")
        .and_then(|v| v.as_str())
        .unwrap_or("Module")
        .to_string();
    let mut current_path = path.to_vec();
    current_path.push(title);

    if let Some(top_list) = module.get("Topics").and_then(|v| v.as_array()) {
        for t in top_list {
            let is_file = t.get("TypeIdentifier").and_then(|v| v.as_str()) == Some("File")
                || t.get("TopicType").and_then(|v| v.as_i64()) == Some(1);
            if is_file {
                let id = t
                    .get("TopicId")
                    .or_else(|| t.get("Id"))
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                let topic_title = t
                    .get("Title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled")
                    .to_string();
                let remote_mod = t
                    .get("LastModifiedDate")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                if !id.is_empty() {
                    topics.push(ExtractedTopic {
                        id,
                        title: topic_title,
                        module_path: current_path.clone(),
                        remote_modified: remote_mod,
                    });
                }
            }
        }
    }

    if let Some(sub_modules) = module.get("Modules").and_then(|v| v.as_array()) {
        for sub in sub_modules {
            walk_module(sub, &current_path, topics);
        }
    }
}

fn build_subfolder_path(course_dir: &Path, module_path: &[String]) -> PathBuf {
    let mut p = course_dir.to_path_buf();
    for seg in module_path {
        p.push(api::sanitize_folder_name(seg));
    }
    p
}

fn make_conflict_path(original: &Path, suffix: &str) -> PathBuf {
    let stem = original.file_stem().unwrap_or_default().to_string_lossy();
    let ext = original
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = original.parent().unwrap_or_else(|| Path::new("."));

    let candidate = parent.join(format!("{stem}{suffix}{ext}"));
    if !candidate.exists() {
        return candidate;
    }

    for i in 1..100 {
        let numbered = parent.join(format!("{stem}{suffix}_{i}{ext}"));
        if !numbered.exists() {
            return numbered;
        }
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_rel_path() {
        assert_eq!(normalize_rel_path("foo/bar/baz.pdf"), "foo/bar/baz.pdf");
        assert_eq!(normalize_rel_path(r"foo\bar\baz.pdf"), "foo/bar/baz.pdf");
        assert_eq!(normalize_rel_path("./foo/bar.pdf"), "foo/bar.pdf");
        assert_eq!(normalize_rel_path("/foo/bar.pdf"), "foo/bar.pdf");
        assert_eq!(normalize_rel_path(r".\foo\bar.pdf"), "foo/bar.pdf");
    }

    #[test]
    fn test_make_conflict_path_simple() {
        let p = PathBuf::from("/some/path/document.pdf");
        let conflict = make_conflict_path(&p, "_edited");
        assert_eq!(conflict, PathBuf::from("/some/path/document_edited.pdf"));
    }

    #[test]
    fn test_extract_file_topics_empty() {
        let val = serde_json::json!({
            "Modules": []
        });
        let topics = extract_file_topics(&val);
        assert!(topics.is_empty());
    }

    #[test]
    fn test_extract_file_topics_nested() {
        let val = serde_json::json!({
            "Modules": [
                {
                    "Title": "Hoofdstuk 1",
                    "Topics": [
                        {
                            "TopicId": 9991,
                            "Title": "Les1.pdf",
                            "TypeIdentifier": "File",
                            "LastModifiedDate": "2024-09-20T10:00:00.000Z"
                        },
                        {
                            "TopicId": 9992,
                            "Title": "Link naar website",
                            "TypeIdentifier": "Link"
                        }
                    ],
                    "Modules": [
                        {
                            "Title": "Oefeningen",
                            "Topics": [
                                {
                                    "TopicId": 9993,
                                    "Title": "Oefening 1.pdf",
                                    "TypeIdentifier": "File"
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        let topics = extract_file_topics(&val);
        assert_eq!(topics.len(), 2);
        assert_eq!(topics[0].id, "9991");
        assert_eq!(topics[0].title, "Les1.pdf");
        assert_eq!(topics[0].module_path, vec!["Hoofdstuk 1"]);
        assert_eq!(topics[1].id, "9993");
        assert_eq!(topics[1].title, "Oefening 1.pdf");
        assert_eq!(topics[1].module_path, vec!["Hoofdstuk 1", "Oefeningen"]);
    }
}
