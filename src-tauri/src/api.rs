use crate::sanitize;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::io::AsyncWriteExt;

const UFORA_HOST: &str = "https://ufora.ugent.be";
const LP_API_VERSION: &str = "1.49";
const LE_API_VERSION: &str = "1.82";
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
    pub filename: Option<String>,
    pub sha256: String,
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

#[derive(Debug, Deserialize)]
struct EnrollmentPagePayload {
    #[serde(rename = "Items", default)]
    items: Vec<serde_json::Value>,
    #[serde(rename = "PagingInfo")]
    paging_info: Option<EnrollmentPagingInfo>,
}

#[derive(Debug, Deserialize)]
struct EnrollmentPagingInfo {
    #[serde(rename = "HasMoreItems", default)]
    has_more_items: bool,
    #[serde(rename = "Bookmark")]
    bookmark: Option<String>,
}

#[derive(Debug)]
struct ParsedEnrollmentPage {
    courses: Vec<Course>,
    paging_info: Option<EnrollmentPagingInfo>,
}

fn enrollment_url(bookmark: Option<&str>) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!(
        "{UFORA_HOST}/d2l/api/lp/{LP_API_VERSION}/enrollments/myenrollments/"
    ))
    .map_err(|e| format!("Failed to build enrollments URL: {e}"))?;

    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("isActive", "true")
            .append_pair("canAccess", "true")
            .append_pair("sortBy", "-StartDate");
        if let Some(bookmark) = bookmark {
            query.append_pair("bookmark", bookmark);
        }
    }

    Ok(url)
}

fn course_from_enrollment(item: &serde_json::Value) -> Option<Course> {
    let org = item.get("OrgUnit")?;
    let id = org
        .get("Id")
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_i64().map(|number| number.to_string()))
        })
        .unwrap_or_default();
    let name = org
        .get("Name")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned();
    if id.is_empty() || name.is_empty() {
        return None;
    }

    let code = org
        .get("Code")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned();
    let home_url = org
        .get("HomeUrl")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned();

    Some(Course {
        id,
        folder_name: sanitize::sanitize_folder_name(&name),
        is_course_offering: is_course_offering(org),
        academic_year: extract_academic_year(&code),
        name,
        code,
        home_url,
    })
}

fn parse_enrollment_page(payload: serde_json::Value) -> Result<ParsedEnrollmentPage, String> {
    let page: EnrollmentPagePayload = serde_json::from_value(payload)
        .map_err(|e| format!("Failed to parse enrollments JSON: {e}"))?;

    Ok(ParsedEnrollmentPage {
        courses: page
            .items
            .iter()
            .filter_map(course_from_enrollment)
            .collect(),
        paging_info: page.paging_info,
    })
}

fn next_enrollment_bookmark(
    paging_info: Option<EnrollmentPagingInfo>,
    seen_bookmarks: &mut HashSet<String>,
) -> Result<Option<String>, String> {
    let Some(paging_info) = paging_info else {
        return Ok(None);
    };
    if !paging_info.has_more_items {
        return Ok(None);
    }

    let bookmark = paging_info
        .bookmark
        .map(|bookmark| bookmark.trim().to_owned())
        .filter(|bookmark| !bookmark.is_empty())
        .ok_or_else(|| {
            "Brightspace indicated more enrollments but did not provide a valid bookmark"
                .to_string()
        })?;

    if !seen_bookmarks.insert(bookmark.clone()) {
        return Err(format!(
            "Brightspace returned a repeated enrollment bookmark ({bookmark}); stopping pagination"
        ));
    }

    Ok(Some(bookmark))
}

fn collect_enrollment_page(
    payload: serde_json::Value,
    courses: &mut Vec<Course>,
    seen_bookmarks: &mut HashSet<String>,
) -> Result<Option<String>, String> {
    let page = parse_enrollment_page(payload)?;
    courses.extend(page.courses);
    next_enrollment_bookmark(page.paging_info, seen_bookmarks)
}

fn assign_unique_course_folder_names(courses: &mut [Course]) {
    let mut base_counts = HashMap::new();
    for course in courses.iter() {
        *base_counts
            .entry(course.folder_name.to_lowercase())
            .or_insert(0usize) += 1;
    }

    for course in courses.iter_mut() {
        let base_name = course.folder_name.clone();
        if base_counts
            .get(&base_name.to_lowercase())
            .copied()
            .unwrap_or_default()
            <= 1
        {
            continue;
        }

        let code = if course.code.trim().is_empty() {
            String::new()
        } else {
            sanitize::sanitize_folder_name(&course.code)
        };
        let course_id = sanitize::sanitize_folder_name(&course.id);
        course.folder_name = if code.is_empty() {
            format!("{base_name} [{course_id}]")
        } else {
            format!("{base_name} [{code}] [{course_id}]")
        };
    }
}

pub async fn fetch_enrolled_courses(token: &str) -> Result<Vec<Course>, String> {
    let c = client(token)?;
    let mut courses = Vec::new();
    let mut bookmark = None;
    let mut seen_bookmarks = HashSet::new();

    loop {
        let url = enrollment_url(bookmark.as_deref())?;
        let res = c
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Enrollments request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("Brightspace API returned status {}", res.status()));
        }

        let payload = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse enrollments JSON: {e}"))?;
        bookmark = collect_enrollment_page(payload, &mut courses, &mut seen_bookmarks)?;
        if bookmark.is_none() {
            break;
        }
    }

    assign_unique_course_folder_names(&mut courses);
    Ok(courses)
}

pub async fn fetch_course_toc(token: &str, course_id: &str) -> Result<serde_json::Value, String> {
    let c = client(token)?;
    let url = format!("{UFORA_HOST}/d2l/api/le/{LE_API_VERSION}/{course_id}/content/toc");

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

pub async fn download_topic_file_to_path(
    token: &str,
    course_id: &str,
    topic_id: &str,
    destination: &Path,
) -> Result<DownloadedTopicFile, String> {
    let c = client(token)?;
    let url = format!(
        "{UFORA_HOST}/d2l/api/le/{LE_API_VERSION}/{course_id}/content/topics/{topic_id}/file"
    );

    let mut res = c
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

    let write_result = async {
        let mut file = tokio::fs::File::create(destination)
            .await
            .map_err(|e| format!("Failed to create temporary download: {e}"))?;
        let mut hasher = Sha256::new();

        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write temporary download: {e}"))?;
            hasher.update(&chunk);
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush temporary download: {e}"))?;
        file.sync_all()
            .await
            .map_err(|e| format!("Failed to persist temporary download: {e}"))?;
        Ok::<String, String>(format!("{:x}", hasher.finalize()))
    }
    .await;

    let sha256 = match write_result {
        Ok(sha256) => sha256,
        Err(error) => {
            let _ = tokio::fs::remove_file(destination).await;
            return Err(error);
        }
    };

    Ok(DownloadedTopicFile {
        filename: disposition,
        sha256,
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

    #[test]
    fn enrollment_url_preserves_filters_on_every_page() {
        let first_page = enrollment_url(None).expect("valid first-page URL");
        let first_query: HashMap<_, _> = first_page.query_pairs().into_owned().collect();
        assert_eq!(
            first_query.get("isActive").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            first_query.get("canAccess").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            first_query.get("sortBy").map(String::as_str),
            Some("-StartDate")
        );
        assert!(!first_query.contains_key("bookmark"));
        assert!(first_page.path().contains("/api/lp/1.49/"));

        let next_page = enrollment_url(Some("next page/+=")).expect("valid bookmarked URL");
        let next_query: HashMap<_, _> = next_page.query_pairs().into_owned().collect();
        assert_eq!(next_query.get("isActive").map(String::as_str), Some("true"));
        assert_eq!(
            next_query.get("canAccess").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            next_query.get("sortBy").map(String::as_str),
            Some("-StartDate")
        );
        assert_eq!(
            next_query.get("bookmark").map(String::as_str),
            Some("next page/+=")
        );
    }

    #[test]
    fn parses_enrollment_items_and_paging_info() {
        let payload = serde_json::json!({
            "Items": [
                {
                    "OrgUnit": {
                        "Id": 123,
                        "Name": "Algorithms",
                        "Code": "C001_2026",
                        "HomeUrl": "/d2l/home/123",
                        "Type": { "Code": "Course Offering", "Name": "Course Offering" }
                    }
                },
                { "OrgUnit": { "Id": 456, "Name": "" } }
            ],
            "PagingInfo": {
                "HasMoreItems": true,
                "Bookmark": "page-2"
            }
        });

        let page = parse_enrollment_page(payload).expect("valid enrollment page");
        assert_eq!(page.courses.len(), 1);
        assert_eq!(page.courses[0].id, "123");
        assert_eq!(page.courses[0].name, "Algorithms");
        assert_eq!(page.courses[0].academic_year, Some(2026));

        let bookmark = next_enrollment_bookmark(page.paging_info, &mut HashSet::new())
            .expect("valid paging info");
        assert_eq!(bookmark.as_deref(), Some("page-2"));
    }

    #[test]
    fn collects_courses_across_multiple_pages() {
        let first_page = serde_json::json!({
            "Items": [{
                "OrgUnit": { "Id": 101, "Name": "First course", "Code": "C101_2025" }
            }],
            "PagingInfo": { "HasMoreItems": true, "Bookmark": "page-2" }
        });
        let second_page = serde_json::json!({
            "Items": [{
                "OrgUnit": { "Id": "202", "Name": "Second course", "Code": "C202_2026" }
            }],
            "PagingInfo": { "HasMoreItems": false, "Bookmark": null }
        });
        let mut courses = Vec::new();
        let mut seen = HashSet::new();

        let bookmark =
            collect_enrollment_page(first_page, &mut courses, &mut seen).expect("valid first page");
        assert_eq!(bookmark.as_deref(), Some("page-2"));

        let bookmark = collect_enrollment_page(second_page, &mut courses, &mut seen)
            .expect("valid second page");
        assert_eq!(bookmark, None);
        assert_eq!(
            courses
                .iter()
                .map(|course| course.id.as_str())
                .collect::<Vec<_>>(),
            vec!["101", "202"]
        );
    }

    #[test]
    fn pagination_rejects_missing_blank_and_repeated_bookmarks() {
        let missing = EnrollmentPagingInfo {
            has_more_items: true,
            bookmark: None,
        };
        assert!(next_enrollment_bookmark(Some(missing), &mut HashSet::new()).is_err());

        let blank = EnrollmentPagingInfo {
            has_more_items: true,
            bookmark: Some("  ".to_string()),
        };
        assert!(next_enrollment_bookmark(Some(blank), &mut HashSet::new()).is_err());

        let mut seen = HashSet::new();
        let first = EnrollmentPagingInfo {
            has_more_items: true,
            bookmark: Some("page-2".to_string()),
        };
        assert_eq!(
            next_enrollment_bookmark(Some(first), &mut seen)
                .expect("first bookmark")
                .as_deref(),
            Some("page-2")
        );

        let repeated = EnrollmentPagingInfo {
            has_more_items: true,
            bookmark: Some("page-2".to_string()),
        };
        let error = next_enrollment_bookmark(Some(repeated), &mut seen)
            .expect_err("repeated bookmarks must stop pagination");
        assert!(error.contains("repeated enrollment bookmark"));
    }

    #[test]
    fn pagination_stops_when_there_are_no_more_items() {
        let paging = EnrollmentPagingInfo {
            has_more_items: false,
            bookmark: Some("unused".to_string()),
        };
        assert_eq!(
            next_enrollment_bookmark(Some(paging), &mut HashSet::new())
                .expect("terminal paging info"),
            None
        );
        assert_eq!(
            next_enrollment_bookmark(None, &mut HashSet::new()).expect("missing paging info"),
            None
        );
    }

    #[test]
    fn duplicate_course_names_get_stable_unique_folder_names() {
        let mut courses = vec![
            Course {
                id: "101".to_string(),
                name: "Programming I".to_string(),
                code: "C001_2025".to_string(),
                home_url: String::new(),
                folder_name: sanitize::sanitize_folder_name("Programming I"),
                is_course_offering: true,
                academic_year: Some(2025),
            },
            Course {
                id: "202".to_string(),
                name: "Programming I".to_string(),
                code: "C001/2026".to_string(),
                home_url: String::new(),
                folder_name: sanitize::sanitize_folder_name("Programming I"),
                is_course_offering: true,
                academic_year: None,
            },
            Course {
                id: "303".to_string(),
                name: "Unique course".to_string(),
                code: "UNIQUE_2026".to_string(),
                home_url: String::new(),
                folder_name: sanitize::sanitize_folder_name("Unique course"),
                is_course_offering: true,
                academic_year: Some(2026),
            },
        ];

        assign_unique_course_folder_names(&mut courses);

        assert_eq!(courses[0].folder_name, "Programming I [C001_2025] [101]");
        assert_eq!(courses[1].folder_name, "Programming I [C001-2026] [202]");
        assert_eq!(courses[2].folder_name, "Unique course");
    }

    #[test]
    fn duplicate_course_name_without_code_uses_course_id() {
        let mut courses = vec![
            Course {
                id: "101".to_string(),
                name: "Unnamed code".to_string(),
                code: String::new(),
                home_url: String::new(),
                folder_name: "Unnamed code".to_string(),
                is_course_offering: true,
                academic_year: None,
            },
            Course {
                id: "202".to_string(),
                name: "unnamed CODE".to_string(),
                code: "  ".to_string(),
                home_url: String::new(),
                folder_name: "unnamed CODE".to_string(),
                is_course_offering: true,
                academic_year: None,
            },
        ];

        assign_unique_course_folder_names(&mut courses);

        assert_eq!(courses[0].folder_name, "Unnamed code [101]");
        assert_eq!(courses[1].folder_name, "unnamed CODE [202]");
    }

    #[test]
    fn content_urls_use_supported_learning_environment_version() {
        assert_eq!(LE_API_VERSION, "1.82");
        assert_eq!(LP_API_VERSION, "1.49");
    }
}
