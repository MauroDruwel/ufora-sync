use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILENAME: &str = ".ufora_sync_manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedFile {
    pub remote_id: String,
    pub remote_path: String,
    pub local_path: String,
    pub sha256: String,
    pub synced_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncManifest {
    pub files: BTreeMap<String, SyncedFile>,
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
                    files: BTreeMap<String, SyncedFile>,
                }
                match serde_json::from_str::<Raw>(&content) {
                    Ok(raw) => {
                        let mut files = BTreeMap::new();
                        for (_k, mut entry) in raw.files {
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
            // SF_DATALESS = 0x40000000 on Darwin APFS
            return (flags & 0x4000_0000) != 0;
        }
    }
    false
}

pub fn make_conflict_path(original: &Path, suffix: &str) -> PathBuf {
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
    }

    #[test]
    fn test_make_conflict_path() {
        let p = PathBuf::from("/tmp/lecture.pdf");
        let conflict = make_conflict_path(&p, "_edited");
        assert_eq!(conflict, PathBuf::from("/tmp/lecture_edited.pdf"));
    }
}
