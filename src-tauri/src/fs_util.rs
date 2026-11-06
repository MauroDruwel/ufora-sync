use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SIDECAR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn unique_sidecar_path(destination: &Path, marker: &str) -> PathBuf {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let filename = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    loop {
        let sequence = SIDECAR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{filename}.{marker}-{}-{sequence}",
            std::process::id()
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
}

/// Atomically installs a same-directory temporary file, replacing an existing file if present.
pub fn atomic_replace(temp_path: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(temp_path, destination).map_err(|e| format!("Failed to install new file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_replace_installs_and_replaces() {
        let directory = std::env::temp_dir().join(format!(
            "ufora-sync-atomic-replace-{}-{}",
            std::process::id(),
            SIDECAR_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).expect("create test directory");
        let destination = directory.join("state.json");
        let first_temp = directory.join("first.tmp");
        fs::write(&first_temp, b"first").expect("write first temporary file");
        atomic_replace(&first_temp, &destination).expect("install first file");
        assert_eq!(fs::read(&destination).expect("read first file"), b"first");

        let second_temp = directory.join("second.tmp");
        fs::write(&second_temp, b"second").expect("write second temporary file");
        atomic_replace(&second_temp, &destination).expect("replace file");
        assert_eq!(fs::read(&destination).expect("read second file"), b"second");

        fs::remove_dir_all(directory).expect("remove test directory");
    }
}
