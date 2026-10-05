use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

pub fn sanitize_folder_name(name: &str) -> String {
    let mut clean = name
        .replace(['/', '\\', ':'], " - ")
        .replace(['*', '?', '"', '<', '>'], "")
        .replace('|', " - ");

    clean = clean
        .trim_start_matches(|c: char| c == '-' || c == '°' || c.is_whitespace())
        .to_string();
    clean = clean
        .trim_end_matches(|c: char| c == '.' || c == '-' || c.is_whitespace())
        .to_string();

    while clean.contains("  ") {
        clean = clean.replace("  ", " ");
    }
    if clean.is_empty() {
        "Unnamed Folder".to_string()
    } else {
        clean
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut clean = name
        .replace(['/', '\\', ':'], "-")
        .replace(['*', '?', '"', '<', '>'], "")
        .replace('|', "-");

    clean = clean
        .trim_matches(|c: char| c == '.' || c == '-' || c.is_whitespace())
        .to_string();
    while clean.contains("..") {
        clean = clean.replace("..", ".");
    }
    while clean.contains("  ") {
        clean = clean.replace("  ", " ");
    }
    if clean.is_empty() {
        "file".to_string()
    } else {
        clean
    }
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
        .timeout(std::time::Duration::from_secs(60))
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
) -> Result<Vec<u8>, String> {
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

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_folder_name() {
        assert_eq!(
            sanitize_folder_name("E610004A: Wiskunde I / 2024-2025"),
            "E610004A - Wiskunde I - 2024-2025"
        );
        assert_eq!(
            sanitize_folder_name("  °Werkcolleges & Oefeningen*?<>|  "),
            "Werkcolleges & Oefeningen"
        );
        assert_eq!(sanitize_folder_name(""), "Unnamed Folder");
        assert_eq!(sanitize_folder_name("   "), "Unnamed Folder");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("les1_syllabus:deel1/2.pdf"),
            "les1_syllabus-deel1-2.pdf"
        );
        assert_eq!(
            sanitize_filename("../../dangerous..path.pdf"),
            "dangerous.path.pdf"
        );
        assert_eq!(sanitize_filename(""), "file");
    }

    #[test]
    fn test_token_expiration() {
        let future = D2lToken {
            token: "valid_abc".to_string(),
            exp: Some(chrono::Utc::now().timestamp() as f64 + 3600.0),
            sub: Some("user123".to_string()),
            tenant: None,
        };
        assert!(!is_token_expired(&future));

        let expired = D2lToken {
            token: "expired_abc".to_string(),
            exp: Some(chrono::Utc::now().timestamp() as f64 - 100.0),
            sub: Some("user123".to_string()),
            tenant: None,
        };
        assert!(is_token_expired(&expired));

        let expiring_soon = D2lToken {
            token: "soon_abc".to_string(),
            exp: Some(chrono::Utc::now().timestamp() as f64 + 30.0),
            sub: Some("user123".to_string()),
            tenant: None,
        };
        assert!(is_token_expired(&expiring_soon));

        let no_exp = D2lToken {
            token: "no_exp_abc".to_string(),
            exp: None,
            sub: None,
            tenant: None,
        };
        assert!(!is_token_expired(&no_exp));
    }
}
