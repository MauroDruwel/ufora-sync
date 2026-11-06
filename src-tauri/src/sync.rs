use crate::api;
use crate::auth;
use crate::config::{self, get_config_dir, AppConfig};
use crate::fs_util;
use crate::manifest::{self, SyncManifest};
use crate::sanitize;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::fs::{self};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
static LOG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn append_log(msg: &str) {
    let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{timestamp}] {msg}\n");
    let log_dir = get_config_dir();
    if fs::create_dir_all(&log_dir).is_err() {
        return;
    }
    let log_file = log_dir.join("activity.log");
    if fs::metadata(&log_file).is_ok_and(|meta| meta.len() >= MAX_LOG_BYTES) {
        let previous = log_dir.join("activity.log.1");
        let _ = fs::remove_file(&previous);
        let _ = fs::rename(&log_file, previous);
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractedTopicKind {
    File,
    Link,
}

#[derive(Debug, Clone)]
pub struct ExtractedTopic {
    pub id: String,
    pub title: String,
    pub module_path: Vec<String>,
    pub remote_modified: Option<String>,
    pub url: Option<String>,
    pub kind: ExtractedTopicKind,
}

pub fn extract_topics(toc: &serde_json::Value) -> Vec<ExtractedTopic> {
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
            let is_link = t.get("TypeIdentifier").and_then(|v| v.as_str()) == Some("Link")
                || t.get("TopicType").and_then(|v| v.as_i64()) == Some(3);
            if is_file || is_link {
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
                        kind: if is_file {
                            ExtractedTopicKind::File
                        } else {
                            ExtractedTopicKind::Link
                        },
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

fn normalize_shortcut_url(raw_url: &str) -> Result<String, String> {
    if raw_url.starts_with('/') {
        return Ok(format!("https://ufora.ugent.be{raw_url}"));
    }

    let parsed = reqwest::Url::parse(raw_url).map_err(|_| "link has an invalid URL".to_string())?;
    if matches!(parsed.scheme(), "http" | "https") {
        Ok(parsed.to_string())
    } else {
        Err(format!(
            "link uses unsupported URL scheme {}",
            parsed.scheme()
        ))
    }
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn build_link_shortcut(topic: &ExtractedTopic) -> Result<(String, Vec<u8>), String> {
    let raw_url = topic
        .url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| "link has no URL".to_string())?;
    let url = normalize_shortcut_url(raw_url)?;
    let escaped_url = escape_html_attribute(&url);
    let escaped_title = escape_html_attribute(&topic.title);
    let filename = format!("{}.html", sanitize::sanitize_folder_name(&topic.title));
    let html = format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0; url={escaped_url}\"><title>{escaped_title}</title></head><body><p><a href=\"{escaped_url}\" rel=\"noreferrer\">Open {escaped_title}</a></p></body></html>\n"
    );
    Ok((filename, html.into_bytes()))
}

pub fn build_subfolder_path(course_dir: &Path, module_path: &[String]) -> PathBuf {
    let mut p = course_dir.to_path_buf();
    for seg in module_path {
        p.push(sanitize::sanitize_folder_name(seg));
    }
    p
}

fn temporary_file_path(directory: &Path, purpose: &str) -> PathBuf {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    directory.join(format!(
        ".ufora-{purpose}-{}-{sequence}.tmp",
        std::process::id()
    ))
}

/// Installs a completed same-directory download.
fn install_download(temp_path: &Path, destination: &Path) -> Result<(), String> {
    fs_util::atomic_replace(temp_path, destination)
        .map_err(|e| format!("Failed to install downloaded file: {e}"))
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
    pub sync_links: bool,
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

    let topics = extract_topics(&toc);
    let selected_topic_count = topics
        .iter()
        .filter(|topic| topic.kind == ExtractedTopicKind::File || opts.sync_links)
        .count();
    on_log(&format!(
        "  Found {selected_topic_count} syncable material(s)"
    ));

    let mut downloaded = 0;
    let mut up_to_date = 0;
    let mut errors = 0;

    for topic in topics {
        if topic.kind == ExtractedTopicKind::Link && !opts.sync_links {
            continue;
        }
        let target_dir = build_subfolder_path(&course_dir, &topic.module_path);
        if let Err(e) = fs::create_dir_all(&target_dir) {
            on_log(&format!(
                "  ✗ Failed to create module directory {}: {e}",
                target_dir.display()
            ));
            errors += 1;
            continue;
        }

        // Fast incremental check if existing in manifest
        let previous_entry = manifest.get_by_remote_id(&topic.id).cloned();
        if topic.kind == ExtractedTopicKind::File {
            if let Some(entry) = previous_entry.as_ref() {
                let full_dest = course_dir.join(&entry.local_path);
                if full_dest.exists()
                    && topic.remote_modified.is_some()
                    && entry.remote_modified == topic.remote_modified
                {
                    up_to_date += 1;
                    continue;
                }
            }
        }

        // Stream each response directly to disk so large course files never have to fit in RAM.
        let tmp_file = temporary_file_path(&target_dir, "download");
        let prepared_file = match topic.kind {
            ExtractedTopicKind::File => {
                api::download_topic_file_to_path(opts.token, opts.course_id, &topic.id, &tmp_file)
                    .await
            }
            ExtractedTopicKind::Link => match build_link_shortcut(&topic) {
                Ok((filename, contents)) => match fs::write(&tmp_file, &contents) {
                    Ok(()) => Ok(api::DownloadedTopicFile {
                        filename: Some(filename),
                        sha256: format!("{:x}", sha2::Sha256::digest(&contents)),
                    }),
                    Err(e) => Err(format!("Failed to create link shortcut: {e}")),
                },
                Err(e) => Err(e),
            },
        };
        match prepared_file {
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

                let sha = file_data.sha256;
                let tracked_path = previous_entry
                    .as_ref()
                    .map(|entry| course_dir.join(&entry.local_path));
                let existing_path = if dest_path.exists() {
                    Some(dest_path.clone())
                } else {
                    tracked_path.filter(|path| path.exists())
                };
                let mut preserved_path = None;
                let mut renamed_preserved_path = None;
                let mut remove_old_path = None;
                let mut did_overwrite = false;

                // Safety & conflict detection: ensure NO local files are ever lost or blindly overwritten
                if let Some(existing_path) = existing_path {
                    let is_placeholder = manifest::is_dataless_placeholder(&existing_path);
                    let local_sha = if is_placeholder {
                        previous_entry.as_ref().map(|e| e.sha256.clone())
                    } else {
                        manifest::calculate_sha256(&existing_path).ok()
                    };

                    if let Some(ref l_sha) = local_sha {
                        if l_sha == &sha && existing_path == dest_path {
                            let _ = fs::remove_file(&tmp_file);
                            manifest.record(
                                &topic.id,
                                &rel_path,
                                &sha,
                                topic.remote_modified.clone(),
                            );
                            manifest_dirty = true;
                            up_to_date += 1;
                            continue;
                        }
                    }

                    let locally_edited = match previous_entry.as_ref() {
                        Some(entry) => local_sha.as_deref() != Some(entry.sha256.as_str()),
                        None => true,
                    };

                    // Content differs: protect local edits or an untracked existing version.
                    if opts.conflict_strategy == "skip" {
                        let _ = fs::remove_file(&tmp_file);
                        on_log(&format!("  ↷ Kept local file: {rel_path}"));
                        continue;
                    } else if opts.conflict_strategy == "overwrite" {
                        did_overwrite = true;
                        if existing_path != dest_path {
                            remove_old_path = Some(existing_path);
                        }
                    } else if locally_edited {
                        if existing_path == dest_path {
                            // Default safe strategy: duplicate (preserves local file with suffix)
                            let edited_path =
                                manifest::make_conflict_path(&dest_path, opts.duplicate_suffix);
                            if let Err(e) = fs::rename(&dest_path, &edited_path) {
                                let _ = fs::remove_file(&tmp_file);
                                on_log(&format!(
                                    "  ✗ Could not preserve local file {rel_path}: {e}"
                                ));
                                errors += 1;
                                continue;
                            }
                            preserved_path = Some(edited_path.clone());
                            renamed_preserved_path = Some(edited_path);
                        } else {
                            preserved_path = Some(existing_path);
                        }
                    } else if existing_path != dest_path {
                        remove_old_path = Some(existing_path);
                    }
                }

                match install_download(&tmp_file, &dest_path) {
                    Ok(()) => {
                        if let Some(old_path) = remove_old_path {
                            if let Err(e) = fs::remove_file(&old_path) {
                                on_log(&format!(
                                    "  ⚠ Installed {rel_path}, but could not remove previous path {}: {e}",
                                    old_path.display()
                                ));
                                errors += 1;
                            }
                        }
                        if let Some(preserved_path) = preserved_path {
                            on_log(&format!(
                                "  ✎ Preserved existing file → {}",
                                preserved_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                            ));
                        } else if did_overwrite {
                            on_log(&format!("  ⚠ Overwrote local file: {rel_path}"));
                        }
                        manifest.record(&topic.id, &rel_path, &sha, topic.remote_modified.clone());
                        manifest_dirty = true;
                        downloaded += 1;
                        on_log(&format!("  ✓ {rel_path}"));
                    }
                    Err(e) => {
                        if let Some(preserved_path) = renamed_preserved_path {
                            if !dest_path.exists() {
                                let _ = fs::rename(preserved_path, &dest_path);
                            }
                        }
                        on_log(&format!("  ✗ Failed to save {rel_path}: {e}"));
                        let _ = fs::remove_file(&tmp_file);
                        errors += 1;
                    }
                }
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp_file);
                on_log(&format!("  ✗ Error downloading item {}: {e}", topic.id));
                errors += 1;
            }
        }
    }

    if manifest_dirty {
        if let Err(e) = manifest.save(&course_dir) {
            on_log(&format!("  ✗ Failed to save sync manifest: {e}"));
            errors += 1;
        }
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

struct RunningGuard(Arc<AtomicBool>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn select_courses_for_sync(courses: Vec<api::Course>, cfg: &AppConfig) -> Vec<api::Course> {
    let current_year = api::current_academic_year();
    let enabled_set: std::collections::HashSet<_> =
        cfg.enabled_courses.iter().map(String::as_str).collect();

    courses
        .into_iter()
        .filter(|course| cfg.show_groups || course.is_course_offering)
        .filter(|course| {
            cfg.show_all_courses
                || !course.is_course_offering
                || match course.academic_year {
                    Some(year) => year == current_year,
                    None => true,
                }
        })
        .filter(|course| enabled_set.is_empty() || enabled_set.contains(course.id.as_str()))
        .collect()
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
        let running_guard = RunningGuard(Arc::clone(&self.is_running));

        let _ = self.app_handle.emit("sync-status", "syncing");
        let _ = crate::tray::update_tray_menu(&self.app_handle);
        self.log("Starting Ufora sync pass…");

        let token = match auth::ensure_valid_token(&self.app_handle, 180.0).await {
            Ok(t) => t,
            Err(e) => {
                self.log(&format!("✗ Auth error: {e}"));
                if let Err(config_error) = config::update_config(|cfg| {
                    cfg.last_sync_time = Some(chrono::Utc::now().to_rfc3339());
                    cfg.last_sync_status = Some("error".to_string());
                }) {
                    self.log(&format!(
                        "✗ Failed to persist authentication error status: {config_error}"
                    ));
                }
                drop(running_guard);
                let _ = self.app_handle.emit("sync-status", "error");
                let _ = self.app_handle.emit("auth-status-changed", false);
                let _ = crate::tray::update_tray_menu(&self.app_handle);
                return Err(e);
            }
        };

        let base_sync_dir = PathBuf::from(&cfg.sync_dir);
        if let Err(e) = fs::create_dir_all(&base_sync_dir) {
            self.log(&format!(
                "✗ Failed to create sync directory {}: {e}",
                base_sync_dir.display()
            ));
            if let Err(config_error) = config::update_config(|cfg| {
                cfg.last_sync_time = Some(chrono::Utc::now().to_rfc3339());
                cfg.last_sync_status = Some("error".to_string());
            }) {
                self.log(&format!(
                    "✗ Failed to persist sync error status: {config_error}"
                ));
            }
            drop(running_guard);
            let _ = self.app_handle.emit("sync-status", "error");
            let _ = crate::tray::update_tray_menu(&self.app_handle);
            return Err(e.to_string());
        }

        let courses = match api::fetch_enrolled_courses(&token).await {
            Ok(c) => c,
            Err(e) => {
                self.log(&format!("✗ Failed to fetch courses: {e}"));
                if let Err(config_error) = config::update_config(|cfg| {
                    cfg.last_sync_time = Some(chrono::Utc::now().to_rfc3339());
                    cfg.last_sync_status = Some("error".to_string());
                }) {
                    self.log(&format!(
                        "✗ Failed to persist sync error status: {config_error}"
                    ));
                }
                drop(running_guard);
                let _ = self.app_handle.emit("sync-status", "error");
                let _ = crate::tray::update_tray_menu(&self.app_handle);
                return Err(e);
            }
        };

        let active_courses = select_courses_for_sync(courses, cfg);

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
                sync_links: cfg.sync_links,
            };

            match sync_course_files(&opts, |msg| self.log(msg)).await {
                Ok(stats) => {
                    total_downloaded += stats.downloaded;
                    total_up_to_date += stats.up_to_date;
                    total_errors += stats.errors;
                }
                Err(e) => {
                    self.log(&format!("  ✗ Failed to sync {}: {e}", course.name));
                    total_errors += 1;
                }
            }
        }

        let summary = SyncSummary {
            downloaded: total_downloaded,
            up_to_date: total_up_to_date,
            errors: total_errors,
        };

        // Persist last sync time and status
        if let Err(e) = config::update_config(|cfg| {
            cfg.last_sync_time = Some(chrono::Utc::now().to_rfc3339());
            cfg.last_sync_status = Some(if summary.errors > 0 {
                "warning".to_string()
            } else {
                "success".to_string()
            });
        }) {
            self.log(&format!("✗ Failed to persist sync result: {e}"));
        }

        self.log(&format!(
            "Sync complete: {} downloaded/updated, {} up to date, {} errors.",
            summary.downloaded, summary.up_to_date, summary.errors
        ));
        drop(running_guard);
        let _ = self.app_handle.emit("sync-status", "idle");
        let _ = self.app_handle.emit("sync-complete", &summary);
        let _ = crate::tray::update_tray_menu(&self.app_handle);

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn course(id: &str, is_course_offering: bool, academic_year: Option<i32>) -> api::Course {
        api::Course {
            id: id.to_string(),
            name: format!("Course {id}"),
            code: id.to_string(),
            home_url: String::new(),
            folder_name: format!("Course {id}"),
            is_course_offering,
            academic_year,
        }
    }

    #[test]
    fn test_extract_topics_nested() {
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

        let topics = extract_topics(&val);
        assert_eq!(topics.len(), 3);
        assert_eq!(topics[0].id, "9991");
        assert_eq!(topics[0].kind, ExtractedTopicKind::File);
        assert_eq!(topics[0].title, "Les1");
        assert_eq!(
            topics[0].url,
            Some("/content/enforced/123/Les1.pdf".to_string())
        );
        assert_eq!(topics[0].module_path, vec!["Hoofdstuk 1"]);
        assert_eq!(topics[1].id, "9992");
        assert_eq!(topics[1].kind, ExtractedTopicKind::Link);
        assert_eq!(topics[2].id, "9993");
        assert_eq!(topics[2].title, "Oefening 1.pdf");
        assert_eq!(topics[2].module_path, vec!["Hoofdstuk 1", "Oefeningen"]);
    }

    #[test]
    fn test_link_shortcut_normalizes_and_escapes_url() {
        let topic = ExtractedTopic {
            id: "42".to_string(),
            title: "Quiz & review".to_string(),
            module_path: Vec::new(),
            remote_modified: None,
            url: Some("/d2l/quiz?one=1&two=2".to_string()),
            kind: ExtractedTopicKind::Link,
        };

        let (filename, contents) = build_link_shortcut(&topic).expect("valid shortcut");
        let html = String::from_utf8(contents).expect("UTF-8 shortcut");
        assert_eq!(filename, "Quiz - review.html");
        assert!(html.contains("https://ufora.ugent.be/d2l/quiz?one=1&amp;two=2"));
        assert!(html.contains("Quiz &amp; review"));

        let mut unsafe_topic = topic;
        unsafe_topic.url = Some("javascript:alert(1)".to_string());
        assert!(build_link_shortcut(&unsafe_topic).is_err());
    }

    #[test]
    fn test_course_selection_applies_visibility_and_explicit_selection() {
        let current_year = api::current_academic_year();
        let courses = vec![
            course("current", true, Some(current_year)),
            course("undated", true, None),
            course("old", true, Some(current_year - 1)),
            course("group", false, None),
        ];

        let cfg = AppConfig::default();
        let selected = select_courses_for_sync(courses.clone(), &cfg);
        assert_eq!(
            selected.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            vec!["current", "undated"]
        );

        let mut cfg = AppConfig {
            show_all_courses: true,
            show_groups: true,
            ..AppConfig::default()
        };
        assert_eq!(select_courses_for_sync(courses.clone(), &cfg).len(), 4);

        cfg.enabled_courses = vec!["old".to_string()];
        let selected = select_courses_for_sync(courses.clone(), &cfg);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "old");

        cfg.enabled_courses = vec!["__ufora_sync_no_courses_selected__".to_string()];
        assert!(select_courses_for_sync(courses, &cfg).is_empty());
    }
}
