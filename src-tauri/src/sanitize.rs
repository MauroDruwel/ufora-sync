use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static RE_ILLEGAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[\\/*?:"<>|~#%&{}`^]"#).expect("valid regex"));
static RE_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" +").expect("valid regex"));
static RE_DASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-{2,}").expect("valid regex"));
static RE_SPACE_DOT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+\.").expect("valid regex"));
static RE_CD_STAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)filename\*\s*=\s*['"]?(?:UTF-8|utf-8)''([^"';]+)"#).expect("valid regex")
});
static RE_CD_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"filename\s*=\s*(?:"([^"]+)"|'([^']+)'|([^;]+))"#).expect("valid regex")
});

/// Exact replica of Python `_sanitize_folder_name` from `ufora_sync/sync.py`
pub fn sanitize_folder_name(name: &str) -> String {
    let mut clean = name.to_string();
    clean = clean.replace('°', "");

    // Replace illegal filesystem & SharePoint characters
    clean = RE_ILLEGAL.replace_all(&clean, "-").to_string();

    clean = clean
        .replace(['’', '‘'], "'")
        .replace("'s", "s")
        .replace('\'', "");

    // Strip leading and trailing hyphens, dots, spaces, underscores
    clean = clean
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '-' || c == '_')
        .to_string();

    // Collapse multiple spaces and dashes
    clean = RE_SPACES.replace_all(&clean, " ").to_string();
    clean = RE_DASHES.replace_all(&clean, "-").to_string();

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
    if raw
        .chars()
        .any(|c| (c as u32) < 32 || (127..160).contains(&(c as u32)))
    {
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
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
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
    cleaned = RE_ILLEGAL.replace_all(&cleaned, "-").to_string();
    cleaned = cleaned.replace(['’', '‘'], "'").trim().to_string();

    let p = Path::new(&cleaned);
    let stem_raw = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let suffix_raw = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    let mut stem = RE_SPACE_DOT.replace_all(stem_raw, ".").to_string();
    stem = stem
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '-' || c == '_')
        .to_string();

    let suffix = suffix_raw.trim_end_matches([' ', '.']).to_string();

    stem = RE_SPACES.replace_all(&stem, " ").to_string();
    stem = RE_DASHES.replace_all(&stem, "-").to_string();

    if stem.is_empty() {
        stem = "unnamed".to_string();
    }

    format!("{stem}{suffix}")
}

pub fn parse_content_disposition(header: &str) -> Option<String> {
    // 1. Look for filename*=UTF-8''... or filename*=utf-8''...
    if let Some(caps) = RE_CD_STAR.captures(header) {
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
    if let Some(caps) = RE_CD_PLAIN.captures(header) {
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
        let name = resolve_topic_filename(
            Some("H00_Intro Materialen_26-27.pdf"),
            Some("/content/enforced/123/H00_Intro.pdf"),
            "H00_Intro Materialen_26-27",
            "3402032",
        );
        assert_eq!(name, "H00_Intro Materialen_26-27.pdf");

        let name2 = resolve_topic_filename(
            None,
            Some("/content/enforced/123/H02_Structuur%20van%20Materialen.pdf"),
            "H02_Structuur van Materialen",
            "3402033",
        );
        assert_eq!(name2, "H02_Structuur van Materialen.pdf");
    }
}
