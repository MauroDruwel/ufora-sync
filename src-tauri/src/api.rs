use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

const UFORA_HOST: &str = "https://ufora.ugent.be";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D2lToken {
    pub token: String,
    pub exp: Option<f64>,
    pub sub: Option<String>,
    pub tenant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: String,
    pub name: String,
    pub code: String,
    pub home_url: String,
    pub folder_name: String,
}

#[derive(Debug, Clone)]
pub struct DownloadedTopicFile {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
}

pub fn get_token_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".d2l")
        .join("token.json")
}

pub fn read_token() -> Result<D2lToken, String> {
    let path = get_token_path();
    if !path.exists() {
        return Err("Not logged in. Token file (~/.d2l/token.json) not found.".to_string());
    }
    let data =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read token file: {e}"))?;
    let token: D2lToken =
        serde_json::from_str(&data).map_err(|e| format!("Invalid token format: {e}"))?;
    Ok(token)
}

pub fn is_token_expired(token: &D2lToken) -> bool {
    if let Some(exp) = token.exp {
        let now = chrono::Utc::now().timestamp() as f64;
        // Expired or expiring within 60 seconds
        now >= (exp - 60.0)
    } else {
        false
    }
}

pub fn find_ufora_exe() -> Option<PathBuf> {
    // 1. PATH lookup
    if let Ok(path) = std::process::Command::new("which")
        .arg("ufora")
        .output()
    {
        if path.status.success() {
            let s = String::from_utf8_lossy(&path.stdout).trim().to_string();
            if !s.is_empty() {
                let p = PathBuf::from(s);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // 2. Common macOS Python locations
    let candidates = [
        "/Library/Frameworks/Python.framework/Versions/3.14/bin/ufora",
        "/Library/Frameworks/Python.framework/Versions/3.13/bin/ufora",
        "/Library/Frameworks/Python.framework/Versions/3.12/bin/ufora",
        "/Library/Frameworks/Python.framework/Versions/Current/bin/ufora",
        "/usr/local/bin/ufora",
        "/opt/homebrew/bin/ufora",
    ];

    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let user_bins = [
            home.join(".local/bin/ufora"),
            home.join("bin/ufora"),
            home.join("Library/Python/3.14/bin/ufora"),
            home.join("Library/Python/3.13/bin/ufora"),
        ];
        for ub in &user_bins {
            if ub.exists() {
                return Some(ub.clone());
            }
        }
    }

    None
}

pub fn refresh_session() -> bool {
    if let Some(exe) = find_ufora_exe() {
        if let Ok(out) = Command::new(exe).arg("refresh").output() {
            return out.status.success();
        }
    }
    false
}

pub fn ensure_authenticated(min_remaining_seconds: f64) -> Result<String, String> {
    let token_res = read_token();
    let mut needs_refresh = false;

    match &token_res {
        Ok(t) => {
            if let Some(exp) = t.exp {
                let now = chrono::Utc::now().timestamp() as f64;
                if now + min_remaining_seconds >= exp {
                    needs_refresh = true;
                }
            }
        }
        Err(_) => {
            needs_refresh = true;
        }
    }

    if needs_refresh {
        let _ = refresh_session();
    }

    let token = read_token()?;
    if is_token_expired(&token) {
        return Err("Session expired. Please re-authenticate via `ufora login`.".to_string());
    }
    Ok(token.token)
}

/// Exact replica of Python `_sanitize_folder_name` from `ufora_sync/sync.py`
pub fn sanitize_folder_name(name: &str) -> String {
    let mut clean = name.to_string();
    clean = clean.replace('°', "");

    // Replace illegal filesystem & SharePoint characters
    let re_illegal = Regex::new(r#"[\\/*?:"<>|~#%&{}`^]"#).unwrap();
    clean = re_illegal.replace_all(&clean, "-").to_string();

    clean = clean
        .replace('’', "'")
        .replace('‘', "'")
        .replace("'s", "s")
        .replace('\'', "");

    // Strip leading and trailing hyphens, dots, spaces, underscores
    clean = clean
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '-' || c == '_')
        .to_string();

    // Collapse multiple spaces and dashes
    let re_spaces = Regex::new(r" +").unwrap();
    clean = re_spaces.replace_all(&clean, " ").to_string();

    let re_dashes = Regex::new(r"-{2,}").unwrap();
    clean = re_dashes.replace_all(&clean, "-").to_string();

    if clean.is_empty() {
        "Unnamed Folder".to_string()
    } else {
        clean
    }
}

/// Stage 1 filename safety check replicating `ufora-ai/materials.py`: `_safe_filename`
pub fn safe_filename_stage1(value: &str, fallback: &str) -> String {
    let raw = value.to_string();
    // Non-printable or control characters
    if raw.chars().any(|c| (c as u32) < 32 || (127..160).contains(&(c as u32))) {
        return fallback.to_string();
    }

    // Basename of path
    let name_str = raw.replace('\\', "/");
    let name = match Path::new(&name_str).file_name().and_then(|n| n.to_str()) {
        Some(n) => n.trim(),
        None => return fallback.to_string(),
    };

    if name.is_empty() || name == "." || name == ".." || name.starts_with('.') {
        return fallback.to_string();
    }

    // Replace <>:"/\|?* with _
    let mut clean: String = name
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    clean = clean.trim_end_matches([' ', '.']).to_string();
    if clean.is_empty() {
        return fallback.to_string();
    }

    let p = Path::new(&clean);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim_end_matches([' ', '.']);
    let suffix = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.trim_end_matches([' ', '.'])))
        .unwrap_or_default();

    if stem.is_empty() {
        return fallback.to_string();
    }

    let stem_upper = stem.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&stem_upper.as_str()) {
        return fallback.to_string();
    }

    let combined = format!("{stem}{suffix}");
    if combined.len() > 200 {
        return fallback.to_string();
    }

    combined
}

/// Stage 2 filename sanitization replicating `ufora_sync/sync.py`: `_sanitize_filename`
pub fn sanitize_filename_stage2(name: &str) -> String {
    let raw_str = name.to_string();
    if raw_str.starts_with('.') {
        return raw_str;
    }

    let mut cleaned = raw_str.replace('°', "");
    let re_illegal = Regex::new(r#"[\\/*?:"<>|~#%&{}`^]"#).unwrap();
    cleaned = re_illegal.replace_all(&cleaned, "-").to_string();
    cleaned = cleaned
        .replace('’', "'")
        .replace('‘', "'")
        .trim()
        .to_string();

    let p = Path::new(&cleaned);
    let stem_raw = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let suffix_raw = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    let re_space_dot = Regex::new(r"\s+\.").unwrap();
    let mut stem = re_space_dot.replace_all(stem_raw, ".").to_string();
    stem = stem
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '-' || c == '_')
        .to_string();

    let suffix = suffix_raw.trim_end_matches([' ', '.']).to_string();

    let re_spaces = Regex::new(r" +").unwrap();
    stem = re_spaces.replace_all(&stem, " ").to_string();

    let re_dashes = Regex::new(r"-{2,}").unwrap();
    stem = re_dashes.replace_all(&stem, "-").to_string();

    if stem.is_empty() {
        stem = "unnamed".to_string();
    }

    format!("{stem}{suffix}")
}

pub fn parse_content_disposition(header: &str) -> Option<String> {
    // 1. Look for filename*=UTF-8''... or filename*=utf-8''...
    let re_star = Regex::new(r#"(?i)filename\*\s*=\s*['"]?(?:UTF-8|utf-8)''([^"';]+)"#).unwrap();
    if let Some(caps) = re_star.captures(header) {
        if let Some(m) = caps.get(1) {
            let decoded = percent_encoding::percent_decode_str(m.as_str())
                .decode_utf8_lossy()
                .trim()
                .to_string();
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }

    // 2. Look for filename="..." or filename=...
    let re_plain = Regex::new(r#"filename\s*=\s*(?:"([^"]+)"|'([^']+)'|([^;]+))"#).unwrap();
    if let Some(caps) = re_plain.captures(header) {
        let raw = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .map(|m| m.as_str().trim())
            .unwrap_or("");
        if !raw.is_empty() {
            let decoded = percent_encoding::percent_decode_str(raw)
                .decode_utf8_lossy()
                .trim()
                .to_string();
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }

    None
}

pub fn resolve_topic_filename(
    disposition_filename: Option<&str>,
    topic_url: Option<&str>,
    topic_title: &str,
    topic_id: &str,
) -> String {
    let fallback = format!("topic-{topic_id}");

    // Priority 1: Content-Disposition
    if let Some(df) = disposition_filename {
        let trimmed = df.trim();
        if !trimmed.is_empty() {
            let stage1 = safe_filename_stage1(trimmed, &fallback);
            return sanitize_filename_stage2(&stage1);
        }
    }

    // Priority 2: Url (e.g. /content/enforced/.../H00_Intro.pdf)
    if let Some(u) = topic_url {
        let u_trimmed = u.trim();
        if !u_trimmed.is_empty() {
            if let Some(last) = u_trimmed.rsplit('/').next() {
                let decoded = percent_encoding::percent_decode_str(last)
                    .decode_utf8_lossy()
                    .trim()
                    .to_string();
                if !decoded.is_empty() && decoded.contains('.') {
                    let stage1 = safe_filename_stage1(&decoded, &fallback);
                    return sanitize_filename_stage2(&stage1);
                }
            }
        }
    }

    // Priority 3: Title
    let stage1 = safe_filename_stage1(topic_title, &fallback);
    sanitize_filename_stage2(&stage1)
}

fn client(token: &str) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {token}")
            .parse()
            .map_err(|e| format!("{e}"))?,
    );
    headers.insert(
        reqwest::header::ORIGIN,
        UFORA_HOST.parse().map_err(|e| format!("{e}"))?,
    );
    headers.insert(
        reqwest::header::REFERER,
        format!("{UFORA_HOST}/")
            .parse()
            .map_err(|e| format!("{e}"))?,
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        USER_AGENT.parse().map_err(|e| format!("{e}"))?,
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub async fn fetch_enrolled_courses(token: &str) -> Result<Vec<Course>, String> {
    let c = client(token)?;
    let url = format!("{UFORA_HOST}/d2l/api/lp/1.47/enrollments/myenrollments/?isActive=true&canAccess=true&sortBy=-StartDate");

    let res = c
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Brightspace API returned status {}", res.status()));
    }

    let payload: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {e}"))?;
    let items = payload
        .get("Items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut courses = Vec::new();
    for item in items {
        if let Some(org) = item.get("OrgUnit") {
            let id = org.get("Id").map(|v| v.to_string()).unwrap_or_default();
            let name = org
                .get("Name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let code = org
                .get("Code")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let home_url = org
                .get("HomeUrl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !id.is_empty() && !name.is_empty() {
                let folder_name = sanitize_folder_name(&name);
                courses.push(Course {
                    id,
                    name,
                    code,
                    home_url,
                    folder_name,
                });
            }
        }
    }
    Ok(courses)
}

pub async fn fetch_course_toc(token: &str, course_id: &str) -> Result<serde_json::Value, String> {
    let c = client(token)?;
    let url = format!("{UFORA_HOST}/d2l/api/le/1.80/{course_id}/content/toc");

    let res = c
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("TOC request failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Brightspace TOC returned {}", res.status()));
    }

    res.json()
        .await
        .map_err(|e| format!("Failed to parse TOC JSON: {e}"))
}

pub async fn download_topic_file(
    token: &str,
    course_id: &str,
    topic_id: &str,
) -> Result<DownloadedTopicFile, String> {
    let c = client(token)?;
    let url = format!("{UFORA_HOST}/d2l/api/le/1.80/{course_id}/content/topics/{topic_id}/file");

    let res = c
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Download returned {}", res.status()));
    }

    let disposition = res
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_disposition);

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    Ok(DownloadedTopicFile {
        bytes: bytes.to_vec(),
        filename: disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_folder_name() {
        assert_eq!(
            sanitize_folder_name("E610004A: Wiskunde I / 2024-2025"),
            "E610004A- Wiskunde I - 2024-2025"
        );
        assert_eq!(
            sanitize_folder_name("  °Werkcolleges & Oefeningen*?<>|  "),
            "Werkcolleges - Oefeningen"
        );
        assert_eq!(sanitize_folder_name("Sturing/controle"), "Sturing-controle");
        assert_eq!(sanitize_folder_name(""), "Unnamed Folder");
    }

    #[test]
    fn test_parse_content_disposition() {
        let h1 = r#"attachment; filename*=UTF-8''H00_Intro%20Materialen_26-27.pdf; filename="H00_Intro%20Materialen_26-27.pdf""#;
        assert_eq!(
            parse_content_disposition(h1),
            Some("H00_Intro Materialen_26-27.pdf".to_string())
        );

        let h2 = r#"attachment; filename="document_test.pdf""#;
        assert_eq!(
            parse_content_disposition(h2),
            Some("document_test.pdf".to_string())
        );

        let h3 = r#"attachment; filename=plain_filename.docx"#;
        assert_eq!(
            parse_content_disposition(h3),
            Some("plain_filename.docx".to_string())
        );
    }

    #[test]
    fn test_resolve_topic_filename() {
        // When header exists
        let name = resolve_topic_filename(
            Some("H00_Intro Materialen_26-27.pdf"),
            Some("/content/enforced/123/H00_Intro.pdf"),
            "H00_Intro Materialen_26-27",
            "3402032",
        );
        assert_eq!(name, "H00_Intro Materialen_26-27.pdf");

        // When header is None but Url has extension
        let name2 = resolve_topic_filename(
            None,
            Some("/content/enforced/123/H02_Structuur%20van%20Materialen.pdf"),
            "H02_Structuur van Materialen",
            "3402033",
        );
        assert_eq!(name2, "H02_Structuur van Materialen.pdf");
    }
}
