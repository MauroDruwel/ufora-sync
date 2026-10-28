use crate::api;
use crate::auth;
use crate::config::{get_config_dir, AppConfig};
use crate::manifest::{self, SyncManifest};
use crate::sanitize;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourseSyncStats {
    pub downloaded: usize,
    pub up_to_date: usize,
    pub errors: usize,
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
    pub url: Option<String>,
}

pub fn extract_file_topics(toc: &serde_json::Value) -> Vec<ExtractedTopic> {
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
                    .and_then(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| v.as_i64().map(|n| n.to_string()))
                    })
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
                let url = t.get("Url").and_then(|v| v.as_str()).map(|s| s.to_string());

                if !id.is_empty() {
                    topics.push(ExtractedTopic {
                        id,
                        title: topic_title,
                        module_path: current_path.clone(),
                        remote_modified: remote_mod,
                        url,
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

pub fn build_subfolder_path(course_dir: &Path, module_path: &[String]) -> PathBuf {
    let mut p = course_dir.to_path_buf();
    for seg in module_path {
        p.push(sanitize::sanitize_folder_name(seg));
    }
    p
}

/// Parameters for syncing a single course (groups options cleanly to satisfy Clippy)
pub struct CourseSyncOptions<'a> {
    pub course_id: &'a str,
    pub course_name: &'a str,
    pub course_folder_name: &'a str,
    pub base_sync_dir: &'a Path,
    pub token: &'a str,
    pub conflict_strategy: &'a str,
    pub duplicate_suffix: &'a str,
}

/// Core reusable course sync function
pub async fn sync_course_files<F>(
    opts: &CourseSyncOptions<'_>,
    mut on_log: F,
) -> Result<CourseSyncStats, String>
where
    F: FnMut(&str),
{
    let course_dir = opts.base_sync_dir.join(opts.course_folder_name);
    fs::create_dir_all(&course_dir)
        .map_err(|e| format!("Failed to create course directory: {e}"))?;

    on_log(&format!("Scanning course: {}", opts.course_name));
    let mut manifest = SyncManifest::load(&course_dir);
    let mut manifest_dirty = false;

    let toc = match api::fetch_course_toc(opts.token, opts.course_id).await {
        Ok(t) => t,
        Err(e) => {
            on_log(&format!(
                "  ✗ Failed to fetch TOC for {}: {e}",
                opts.course_name
            ));
            return Ok(CourseSyncStats {
                downloaded: 0,
                up_to_date: 0,
                errors: 1,
            });
        }
    };

    let topics = extract_file_topics(&toc);
    on_log(&format!(
        "  Found {} downloadable material(s)",
        topics.len()
    ));

    let mut downloaded = 0;
    let mut up_to_date = 0;
    let mut errors = 0;

    for topic in topics {
        let target_dir = build_subfolder_path(&course_dir, &topic.module_path);
        let _ = fs::create_dir_all(&target_dir);

        // Fast incremental check if existing in manifest
        if let Some(entry) = manifest.get_by_remote_id(&topic.id) {
            let full_dest = course_dir.join(&entry.local_path);
            if full_dest.exists() && entry.remote_modified == topic.remote_modified {
                up_to_date += 1;
                continue;
            }
        }

        // Download topic file
        match api::download_topic_file(opts.token, opts.course_id, &topic.id).await {
            Ok(file_data) => {
                let resolved_filename = sanitize::resolve_topic_filename(
                    file_data.filename.as_deref(),
                    topic.url.as_deref(),
                    &topic.title,
                    &topic.id,
                );

                let dest_path = target_dir.join(&resolved_filename);
                let rel_path = match dest_path.strip_prefix(&course_dir) {
                    Ok(p) => manifest::normalize_rel_path(&p.to_string_lossy()),
                    Err(_) => resolved_filename.clone(),
                };

                let sha = {
                    let mut hasher = Sha256::new();
                    hasher.update(&file_data.bytes);
                    format!("{:x}", hasher.finalize())
                };

                // Safety & conflict detection: ensure NO local files are ever lost or blindly overwritten
                if dest_path.exists() {
                    let is_placeholder = manifest::is_dataless_placeholder(&dest_path);
                    let local_sha = if is_placeholder {
                        manifest
                            .get_by_remote_id(&topic.id)
                            .map(|e| e.sha256.clone())
                    } else {
                        manifest::calculate_sha256(&dest_path).ok()
                    };

                    if let Some(ref l_sha) = local_sha {
                        if l_sha == &sha {
                            manifest.record(&topic.id, &rel_path, &sha, topic.remote_modified);
                            manifest_dirty = true;
                            up_to_date += 1;
                            continue;
                        }
                    }

                    // Content differs: protect local edits or existing version
                    if opts.conflict_strategy == "skip" {
                        on_log(&format!("  ↷ Kept local file: {rel_path}"));
                        continue;
                    } else if opts.conflict_strategy == "overwrite" {
                        on_log(&format!("  ⚠ Overwrote local file: {rel_path}"));
                    } else {
                        // Default safe strategy: duplicate (preserves local file with suffix)
                        let edited_path =
                            manifest::make_conflict_path(&dest_path, opts.duplicate_suffix);
                        let _ = fs::rename(&dest_path, &edited_path);
                        on_log(&format!(
                            "  ✎ Preserved existing file → {}",
                            edited_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ));
                    }
                }

                // Atomic write using a hidden temporary file
                let tmp_file = dest_path.with_file_name(format!(
                    ".{}.tmp_{}",
                    dest_path.file_name().unwrap_or_default().to_string_lossy(),
                    Utc::now().timestamp_nanos_opt().unwrap_or(0)
                ));

                match fs::write(&tmp_file, &file_data.bytes) {
                    Ok(_) => {
                        if let Err(e) = fs::rename(&tmp_file, &dest_path) {
                            on_log(&format!("  ✗ Failed to save {rel_path}: {e}"));
                            let _ = fs::remove_file(&tmp_file);
                            errors += 1;
                            continue;
                        }
                        manifest.record(&topic.id, &rel_path, &sha, topic.remote_modified);
                        manifest_dirty = true;
                        downloaded += 1;
                        on_log(&format!("  ✓ {rel_path}"));
                    }
                    Err(e) => {
                        on_log(&format!("  ✗ Write error for {rel_path}: {e}"));
                        let _ = fs::remove_file(&tmp_file);
                        errors += 1;
                    }
                }
            }
            Err(e) => {
                on_log(&format!("  ✗ Error downloading item {}: {e}", topic.id));
                errors += 1;
            }
        }
    }

    if manifest_dirty {
        let _ = manifest.save(&course_dir);
    }

    Ok(CourseSyncStats {
        downloaded,
        up_to_date,
        errors,
    })
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

        let token = match auth::ensure_authenticated(180.0) {
            Ok(t) => t,
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
            let opts = CourseSyncOptions {
                course_id: &course.id,
                course_name: &course.name,
                course_folder_name: &course.folder_name,
                base_sync_dir: &base_sync_dir,
                token: &token,
                conflict_strategy: &cfg.conflict_strategy,
                duplicate_suffix: &cfg.duplicate_suffix,
            };

            let stats = sync_course_files(&opts, |msg| self.log(msg)).await?;

            total_downloaded += stats.downloaded;
            total_up_to_date += stats.up_to_date;
            total_errors += stats.errors;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_topics_nested() {
        let val = serde_json::json!({
            "Modules": [
                {
                    "Title": "Hoofdstuk 1",
                    "Topics": [
                        {
                            "TopicId": 9991,
                            "Title": "Les1",
                            "TypeIdentifier": "File",
                            "Url": "/content/enforced/123/Les1.pdf",
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
        assert_eq!(topics[0].title, "Les1");
        assert_eq!(
            topics[0].url,
            Some("/content/enforced/123/Les1.pdf".to_string())
        );
        assert_eq!(topics[0].module_path, vec!["Hoofdstuk 1"]);
        assert_eq!(topics[1].id, "9993");
        assert_eq!(topics[1].title, "Oefening 1.pdf");
        assert_eq!(topics[1].module_path, vec!["Hoofdstuk 1", "Oefeningen"]);
    }
}
