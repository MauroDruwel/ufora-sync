use crate::sanitize;
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
                let folder_name = sanitize::sanitize_folder_name(&name);
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
