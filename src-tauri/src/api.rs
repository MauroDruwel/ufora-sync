use crate::sanitize;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

const UFORA_HOST: &str = "https://ufora.ugent.be";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: String,
    pub name: String,
    pub code: String,
    pub home_url: String,
    pub folder_name: String,
    #[serde(default = "default_true")]
    pub is_course_offering: bool,
    #[serde(default)]
    pub academic_year: Option<i32>,
}

fn default_true() -> bool {
    true
}

pub fn is_course_offering(org: &serde_json::Value) -> bool {
    if let Some(t) = org.get("Type") {
        let code = t.get("Code").and_then(|v| v.as_str()).unwrap_or("");
        let name = t.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        code == "Course Offering" || name == "Course Offering" || name == "Cursuseditie"
    } else {
        true
    }
}

pub fn extract_academic_year(code: &str) -> Option<i32> {
    if let Some(idx) = code.rfind('_') {
        let suffix = &code[idx + 1..];
        if suffix.len() == 4 && (suffix.starts_with("20") || suffix.starts_with("19")) {
            if let Ok(y) = suffix.parse::<i32>() {
                return Some(y);
            }
        }
    }
    None
}

pub fn current_academic_year() -> i32 {
    let now = chrono::Local::now();
    let year = now.year();
    let month = now.month();
    if month >= 9 {
        year
    } else {
        year - 1
    }
}

#[derive(Debug, Clone)]
pub struct DownloadedTopicFile {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
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
            let id = org
                .get("Id")
                .and_then(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
                .unwrap_or_default();
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
                let is_offering = is_course_offering(org);
                let academic_year = extract_academic_year(&code);
                let folder_name = sanitize::sanitize_folder_name(&name);
                courses.push(Course {
                    id,
                    name,
                    code,
                    home_url,
                    folder_name,
                    is_course_offering: is_offering,
                    academic_year,
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
        .and_then(sanitize::parse_content_disposition);

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
    fn test_is_course_offering() {
        let offering = serde_json::json!({
            "Type": { "Id": 3, "Code": "Course Offering", "Name": "Course Offering" }
        });
        assert!(is_course_offering(&offering));

        let dutch_offering = serde_json::json!({
            "Type": { "Id": 3, "Code": "Course Offering", "Name": "Cursuseditie" }
        });
        assert!(is_course_offering(&dutch_offering));

        let group = serde_json::json!({
            "Type": { "Id": 4, "Code": "Group", "Name": "Groep" }
        });
        assert!(!is_course_offering(&group));
    }

    #[test]
    fn test_extract_academic_year() {
        assert_eq!(extract_academic_year("E610004A_2026"), Some(2026));
        assert_eq!(extract_academic_year("E610014A_2025"), Some(2025));
        assert_eq!(extract_academic_year("E610004A"), None);
        assert_eq!(extract_academic_year("GR01_GROEP12"), None);
    }

    #[test]
    fn test_current_academic_year() {
        let year = current_academic_year();
        assert!((2024..=2030).contains(&year));
    }
}
