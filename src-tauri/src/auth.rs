use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::webview::cookie::{time::OffsetDateTime, SameSite};
use tauri::webview::Cookie;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;

pub const UFORA_HOME_URL: &str = "https://ufora.ugent.be/d2l/home";
pub const UFORA_SSO_LOGIN_URL: &str =
    "https://elosp.ugent.be/welcome/uforalogin?target=%2fd2l%2fhome";
pub const LOGIN_WINDOW_LABEL: &str = "ufora-login";
pub const SILENT_REFRESH_LABEL: &str = "ufora-silent-refresh";

const SESSION_FILENAME: &str = "session.json";
const MIN_CAPTURE_LIFETIME_SECONDS: f64 = 10.0 * 60.0;
const SILENT_REFRESH_TIMEOUT_SECONDS: u64 = 25;
const AUTH_COOKIE_DOMAINS: [&str; 2] = ["ugent.be", "microsoftonline.com"];
const FORCE_TOKEN_REFRESH_SCRIPT: &str = "window.__ufora_force_token_refresh = true;";

const AUTO_AUTH_AND_TOKEN_SCRIPT: &str = r##"
(function() {
    if (window.__ufora_auth_installed) return;
    window.__ufora_auth_installed = true;

    function parseJwtExp(jwt) {
        try {
            const parts = jwt.split('.');
            if (parts.length < 2) return 0;
            const b64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
            const json = atob(b64);
            const payload = JSON.parse(json);
            return Number(payload.exp || 0);
        } catch (_) {
            return 0;
        }
    }

    function submitToken(token) {
        if (window.__ufora_auth_completed) return;
        window.__ufora_auth_completed = true;
        window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(token);
    }

    function checkAuth() {
        const host = window.location.hostname || "";
        const href = window.location.href || "";
        const nowSec = Date.now() / 1000;

        // 1. Microsoft Entra SSO (login.microsoftonline.com / login.microsoft.com)
        if (host.includes("microsoftonline.com") || host.includes("microsoft.com")) {
            // Auto-click account tile once if presented with account picker
            try {
                if (!sessionStorage.getItem('__ufora_tile_clicked')) {
                    const tile = document.querySelector(
                        "[data-test-id$='@ugent.be'], [data-test-id$='@UGent.be'], [data-test-id*='ugent'], .tile [role='button']:not(#otherTile), div[data-report-value='Email_Picker_Tile']"
                    );
                    if (tile) {
                        sessionStorage.setItem('__ufora_tile_clicked', '1');
                        setTimeout(function() {
                            try { tile.click(); } catch (_) {}
                        }, 400);
                    }
                }
            } catch (_) {}

            // Auto-click "Yes" on "Stay signed in?" (KMSI)
            try {
                if (!sessionStorage.getItem('__ufora_kmsi_clicked')) {
                    const dontShowCheckbox = document.querySelector("input[name='DontShowAgain'], #KmsiCheckboxField");
                    if (dontShowCheckbox && !dontShowCheckbox.checked) {
                        try { dontShowCheckbox.click(); } catch (_) {}
                    }
                    const yesBtn = document.querySelector("#idSIButton9, input[id='idSIButton9'], button[id='idSIButton9']");
                    if (yesBtn) {
                        sessionStorage.setItem('__ufora_kmsi_clicked', '1');
                        setTimeout(function() {
                            try { yesBtn.click(); } catch (_) {}
                        }, 400);
                    }
                }
            } catch (_) {}
        }

        // 2. UGent Elosp portal
        if (host.includes("elosp.ugent.be")) {
            try {
                if (!sessionStorage.getItem('__ufora_elosp_clicked')) {
                    const uforaLink = document.querySelector("a[href*='uforalogin'], a[href*='d2l']");
                    if (uforaLink) {
                        sessionStorage.setItem('__ufora_elosp_clicked', '1');
                        setTimeout(function() {
                            try { uforaLink.click(); } catch (_) {}
                        }, 300);
                    }
                }
            } catch (_) {}
        }

        // 3. Ufora Login Landing page (if redirected to /d2l/login or /d2l/lp/auth/login)
        if (host.includes("ufora.ugent.be") && (href.includes("/d2l/login") || href.includes("/d2l/lp/auth/login"))) {
            try {
                if (!sessionStorage.getItem('__ufora_saml_clicked')) {
                    const loginBtn = document.querySelector("a[href*='uforalogin'], a[href*='saml']");
                    if (loginBtn) {
                        sessionStorage.setItem('__ufora_saml_clicked', '1');
                        setTimeout(function() {
                            try { loginBtn.click(); } catch (_) {}
                        }, 300);
                    }
                }
            } catch (_) {}
        }

        // 4. Ufora Authenticated page -> extract token
        if (host.includes("ufora.ugent.be") && (href.includes("/d2l/home") || href.includes("/d2l/"))) {
            // Interactive sign-in may reuse a valid token already held by Brightspace. A forced
            // refresh deliberately skips this cache and requests a new token below.
            if (!window.__ufora_force_token_refresh) {
                try {
                    const raw = window.localStorage.getItem('D2L.Fetch.Tokens');
                    if (raw) {
                        const data = JSON.parse(raw);
                        let bestToken = null;
                        let bestExp = 0;
                        for (const item of Object.values(data)) {
                            if (item && item.access_token && item.access_token.startsWith('eyJ')) {
                                const exp = parseJwtExp(item.access_token);
                                if (exp > (nowSec + 600) && exp > bestExp) {
                                    bestToken = item.access_token;
                                    bestExp = exp;
                                }
                            }
                        }
                        if (bestToken) {
                            submitToken(bestToken);
                            return;
                        }
                    }
                } catch (_) {}
            }

            // Request fresh token from D2L oauth endpoint
            if (window.__ufora_token_requesting) return;
            window.__ufora_token_requesting = true;
            (async function() {
                try {
                    let xsrf = null;
                    try {
                        xsrf = window.localStorage.getItem('XSRF.Token');
                    } catch (_) {}

                    if (!xsrf) {
                        const xsrfResp = await window.fetch('/d2l/lp/auth/xsrf-tokens', { credentials: 'include' });
                        if (xsrfResp.ok) {
                            const xsrfData = await xsrfResp.json();
                            xsrf = xsrfData.referrerToken;
                            try {
                                window.localStorage.setItem('XSRF.Token', xsrf);
                            } catch (_) {}
                        }
                    }
                    if (xsrf) {
                        const tokenResp = await window.fetch('/d2l/lp/auth/oauth2/token', {
                            method: 'POST',
                            credentials: 'include',
                            headers: {
                                'Content-Type': 'application/x-www-form-urlencoded',
                                'X-Csrf-Token': xsrf,
                            },
                            body: 'scope=*:*:*',
                        });
                        if (tokenResp.ok) {
                            const tokenData = await tokenResp.json();
                            if (tokenData && tokenData.access_token) {
                                const exp = parseJwtExp(tokenData.access_token);
                                if (exp > (nowSec + 600)) {
                                    submitToken(tokenData.access_token);
                                }
                            }
                        }
                    }
                } catch (_) {
                    // A later interval retries while the saved browser session remains valid.
                } finally {
                    window.__ufora_token_requesting = false;
                }
            })();
        }
    }

    // Intercept fetch ONLY on ufora.ugent.be to capture Bearer tokens from live requests
    try {
        if (window.location.hostname.includes("ufora.ugent.be") && !window.__ufora_force_token_refresh) {
            const origFetch = window.fetch;
            window.fetch = function(...args) {
                try {
                    const req = args[0];
                    const opts = args[1] || {};
                    let auth = null;
                    if (opts.headers) {
                        if (typeof opts.headers.get === 'function') {
                            auth = opts.headers.get('authorization') || opts.headers.get('Authorization');
                        } else {
                            auth = opts.headers['authorization'] || opts.headers['Authorization'];
                        }
                    } else if (req && req.headers && typeof req.headers.get === 'function') {
                        auth = req.headers.get('authorization') || req.headers.get('Authorization');
                    }
                    if (auth && typeof auth === 'string' && auth.startsWith('Bearer eyJ')) {
                        const tok = auth.substring(7);
                        const exp = parseJwtExp(tok);
                        if (exp > ((Date.now() / 1000) + 600)) {
                            submitToken(tok);
                        }
                    }
                } catch (_) {}
                return origFetch.apply(this, args);
            };
        }
    } catch (_) {}

    checkAuth();
    setInterval(checkAuth, 1000);
})();
"##;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D2lToken {
    pub token: String,
    pub exp: Option<f64>,
    pub sub: Option<String>,
    pub tenant: Option<String>,
    #[serde(default)]
    pub captured_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedSession {
    #[serde(default)]
    saved_at: i64,
    #[serde(default)]
    cookies: Vec<SavedCookie>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedCookie {
    name: String,
    value: String,
    domain: String,
    #[serde(default = "default_cookie_path")]
    path: String,
    #[serde(default = "session_cookie_expiry")]
    expires: f64,
    #[serde(default, rename = "httpOnly", alias = "http_only")]
    http_only: bool,
    #[serde(default)]
    secure: bool,
    #[serde(default, rename = "sameSite", alias = "same_site")]
    same_site: Option<String>,
}

fn default_cookie_path() -> String {
    "/".to_string()
}

fn session_cookie_expiry() -> f64 {
    -1.0
}

pub fn get_token_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".d2l")
        .join("token.json")
}

pub fn get_session_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".d2l")
        .join(SESSION_FILENAME)
}

pub fn parse_jwt_claims(token: &str) -> Option<(Option<f64>, Option<String>, Option<String>)> {
    let payload_part = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload_part)
        .or_else(|_| URL_SAFE.decode(payload_part))
        .ok()?;

    let val: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let exp = val.get("exp").and_then(|v| v.as_f64());
    let sub = val.get("sub").and_then(|v| match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    });
    let tenant = val
        .get("tenantid")
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    Some((exp, sub, tenant))
}

fn token_has_minimum_lifetime(token: &str, min_remaining_seconds: f64) -> bool {
    let Some((Some(exp), _, _)) = parse_jwt_claims(token) else {
        return false;
    };
    chrono::Utc::now().timestamp() as f64 + min_remaining_seconds < exp
}

fn is_auth_cookie_domain(domain: &str) -> bool {
    let host = domain.trim_start_matches('.').to_ascii_lowercase();
    AUTH_COOKIE_DOMAINS
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

fn same_site_name(same_site: Option<SameSite>) -> Option<String> {
    same_site.map(|value| match value {
        SameSite::Strict => "Strict".to_string(),
        SameSite::Lax => "Lax".to_string(),
        SameSite::None => "None".to_string(),
    })
}

fn saved_cookie_from_webview(cookie: &Cookie<'_>) -> Option<SavedCookie> {
    let domain = cookie.domain()?.to_string();
    if !is_auth_cookie_domain(&domain) {
        return None;
    }

    Some(SavedCookie {
        name: cookie.name().to_string(),
        value: cookie.value().to_string(),
        domain,
        path: cookie.path().unwrap_or("/").to_string(),
        expires: cookie
            .expires_datetime()
            .map(|expiry| expiry.unix_timestamp() as f64)
            .unwrap_or_else(session_cookie_expiry),
        http_only: cookie.http_only().unwrap_or(false),
        secure: cookie.secure().unwrap_or(false),
        same_site: same_site_name(cookie.same_site()),
    })
}

fn webview_cookie_from_saved(saved: &SavedCookie, now: f64) -> Option<Cookie<'static>> {
    if !is_auth_cookie_domain(&saved.domain) || (saved.expires > 0.0 && saved.expires <= now) {
        return None;
    }

    let mut builder = Cookie::build((saved.name.clone(), saved.value.clone()))
        .domain(saved.domain.clone())
        .path(if saved.path.is_empty() {
            default_cookie_path()
        } else {
            saved.path.clone()
        })
        .http_only(saved.http_only)
        .secure(saved.secure);

    if saved.expires > 0.0 {
        if let Ok(expiry) = OffsetDateTime::from_unix_timestamp(saved.expires as i64) {
            builder = builder.expires(expiry);
        }
    }

    builder = match saved.same_site.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("strict") => builder.same_site(SameSite::Strict),
        Some(value) if value.eq_ignore_ascii_case("lax") => builder.same_site(SameSite::Lax),
        Some(value) if value.eq_ignore_ascii_case("none") => builder.same_site(SameSite::None),
        _ => builder,
    };

    Some(builder.build())
}

fn write_private_file(path: &std::path::Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create auth directory: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("Failed to secure auth directory: {e}"))?;
        }
    }

    let temporary = path.with_extension("tmp");
    fs::write(&temporary, contents).map_err(|e| format!("Failed to write auth state: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to secure auth state: {e}"))?;
    }
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Failed to replace auth state: {e}"))?;
    }
    fs::rename(&temporary, path).map_err(|e| format!("Failed to save auth state: {e}"))
}

fn save_session_cookies(window: &tauri::WebviewWindow) -> Result<usize, String> {
    let cookies = window
        .cookies()
        .map_err(|e| format!("Failed to read browser session cookies: {e}"))?;
    if !ACCEPT_AUTH_RESULTS.load(Ordering::SeqCst) {
        return Ok(0);
    }
    let cookies: Vec<SavedCookie> = cookies
        .iter()
        .filter_map(saved_cookie_from_webview)
        .collect();
    let count = cookies.len();
    let session = SavedSession {
        saved_at: chrono::Utc::now().timestamp(),
        cookies,
    };
    let contents = serde_json::to_vec_pretty(&session)
        .map_err(|e| format!("Failed to serialize browser session: {e}"))?;
    write_private_file(&get_session_path(), &contents)?;
    Ok(count)
}

fn load_session_cookies() -> Vec<Cookie<'static>> {
    let Ok(contents) = fs::read_to_string(get_session_path()) else {
        return Vec::new();
    };
    let Ok(session) = serde_json::from_str::<SavedSession>(&contents) else {
        return Vec::new();
    };
    let now = chrono::Utc::now().timestamp() as f64;
    session
        .cookies
        .iter()
        .filter_map(|cookie| webview_cookie_from_saved(cookie, now))
        .collect()
}

fn restore_session_cookies(window: &tauri::WebviewWindow) -> usize {
    load_session_cookies()
        .into_iter()
        .filter(|cookie| window.set_cookie(cookie.clone()).is_ok())
        .count()
}

fn persist_session_and_close(app: AppHandle, label: &'static str) {
    // Cookie reads can deadlock on WebView2 when called synchronously from a navigation
    // callback. Dispatching them from a worker also lets the navigation handler return first.
    std::thread::spawn(move || {
        if let Some(window) = app.get_webview_window(label) {
            if ACCEPT_AUTH_RESULTS.load(Ordering::SeqCst) {
                let _ = save_session_cookies(&window);
            }
            let _ = window.close();
        }
    });
}

async fn clear_auth_cookies(app: &AppHandle) -> Result<(), String> {
    let window = app
        .webview_windows()
        .into_iter()
        .find_map(|(label, window)| {
            (label != LOGIN_WINDOW_LABEL && label != SILENT_REFRESH_LABEL).then_some(window)
        });
    let Some(window) = window else {
        return Ok(());
    };

    // WebView2 cookie reads must not block its event thread.
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let cookies = window
            .cookies()
            .map_err(|e| format!("Failed to read authentication cookies: {e}"))?;
        for cookie in cookies {
            if cookie.domain().is_some_and(is_auth_cookie_domain) {
                window
                    .delete_cookie(cookie)
                    .map_err(|e| format!("Failed to clear authentication cookie: {e}"))?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Failed to clear browser authentication state: {e}"))?
}

pub fn save_token_str(token: &str) -> Result<(), String> {
    let (exp, sub, tenant) = parse_jwt_claims(token).unwrap_or((None, None, None));
    let path = get_token_path();
    let data = serde_json::json!({
        "token": token,
        "exp": exp,
        "sub": sub,
        "tenant": tenant,
        "captured_at": chrono::Utc::now().timestamp(),
    });
    let s = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    write_private_file(&path, s.as_bytes())
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

pub fn logout() -> Result<(), String> {
    logout_at(&get_token_path())?;
    let session_path = get_session_path();
    if session_path.exists() {
        fs::remove_file(session_path)
            .map_err(|e| format!("Failed to delete saved browser session: {e}"))?;
    }
    Ok(())
}

pub fn logout_at(path: &std::path::Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("Failed to delete token file: {e}"))?;
    }
    Ok(())
}

pub fn is_token_expired(token: &D2lToken) -> bool {
    if let Some(exp) = token.exp {
        let now = chrono::Utc::now().timestamp() as f64;
        now >= (exp - 180.0)
    } else {
        false
    }
}

pub fn is_authenticated_and_valid(min_remaining_seconds: f64) -> bool {
    if let Ok(token) = read_token() {
        if let Some(exp) = token.exp {
            let now = chrono::Utc::now().timestamp() as f64;
            return now + min_remaining_seconds < exp;
        }
    }
    false
}

static REFRESH_LOCK: Mutex<()> = Mutex::const_new(());
static ACCEPT_AUTH_RESULTS: AtomicBool = AtomicBool::new(true);
static TOKEN_CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub async fn logout_and_clear(app: &AppHandle) -> Result<(), String> {
    let _refresh_guard = REFRESH_LOCK.lock().await;
    ACCEPT_AUTH_RESULTS.store(false, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = window.close();
    }
    if let Some(window) = app.get_webview_window(SILENT_REFRESH_LABEL) {
        let _ = window.close();
    }

    let cookie_result = clear_auth_cookies(app).await;
    let file_result = logout();
    file_result?;
    cookie_result
}

fn handle_auth_navigation(app: &AppHandle, label: &'static str, url: &tauri::Url) -> bool {
    if url.host_str() != Some("ufora.ugent.be") || url.path() != "/__auth_done" {
        return true;
    }

    let token = url
        .query_pairs()
        .find_map(|(key, value)| (key == "token").then(|| value.into_owned()));
    let Some(token) = token else {
        return false;
    };

    if ACCEPT_AUTH_RESULTS.load(Ordering::SeqCst)
        && token_has_minimum_lifetime(&token, MIN_CAPTURE_LIFETIME_SECONDS)
        && save_token_str(&token).is_ok()
    {
        TOKEN_CAPTURE_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let _ = app.emit("auth-status-changed", true);
        let _ = crate::tray::update_tray_menu(app);
        persist_session_and_close(app.clone(), label);
    }

    false
}

/// Renew the Brightspace access token in an app-owned hidden WebView.
///
/// Session cookies captured during interactive login are restored before following the same
/// UGent/Entra SSO route as a normal sign-in. This keeps refresh fully native and removes the
/// former runtime dependency on the Python `ufora` CLI and Playwright.
async fn native_token_refresh(app: &AppHandle, min_remaining_seconds: f64, force: bool) -> bool {
    let _refresh_guard = REFRESH_LOCK.lock().await;

    // A concurrent caller may have completed refresh while this caller waited for the lock.
    if !force && is_authenticated_and_valid(min_remaining_seconds) {
        return true;
    }

    // Background renewal never competes with an interactive sign-in. A user-requested refresh
    // replaces any stale sign-in window so the operation is deterministic and remains hidden.
    if let Some(login_window) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        if !force {
            return false;
        }
        let _ = login_window.close();
    }
    if let Some(stale_window) = app.get_webview_window(SILENT_REFRESH_LABEL) {
        let _ = stale_window.close();
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    ACCEPT_AUTH_RESULTS.store(true, Ordering::SeqCst);
    let capture_sequence = TOKEN_CAPTURE_SEQUENCE.load(Ordering::SeqCst);

    let blank_url: tauri::Url = "about:blank".parse().expect("about:blank is a valid URL");
    let handle_nav = app.clone();
    let mut builder =
        WebviewWindowBuilder::new(app, SILENT_REFRESH_LABEL, WebviewUrl::External(blank_url))
            .title("Ufora Background Refresh")
            .inner_size(400.0, 400.0)
            .visible(false)
            .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled);
    if force {
        builder = builder.initialization_script(FORCE_TOKEN_REFRESH_SCRIPT);
    }
    builder = builder
        .initialization_script(AUTO_AUTH_AND_TOKEN_SCRIPT)
        .on_navigation(move |url| handle_auth_navigation(&handle_nav, SILENT_REFRESH_LABEL, url));

    let Ok(window) = builder.build() else {
        return false;
    };
    restore_session_cookies(&window);

    let Ok(login_url) = UFORA_HOME_URL.parse() else {
        let _ = window.close();
        return false;
    };
    if window.navigate(login_url).is_err() {
        let _ = window.close();
        return false;
    }

    let attempts = SILENT_REFRESH_TIMEOUT_SECONDS * 4;
    for _ in 0..attempts {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let captured_during_refresh =
            TOKEN_CAPTURE_SEQUENCE.load(Ordering::SeqCst) > capture_sequence;
        if is_authenticated_and_valid(min_remaining_seconds) && (!force || captured_during_refresh)
        {
            let _ = app.emit("auth-status-changed", true);
            let _ = crate::tray::update_tray_menu(app);
            return true;
        }
    }

    if let Some(window) = app.get_webview_window(SILENT_REFRESH_LABEL) {
        let _ = window.close();
    }
    false
}

pub async fn try_native_token_refresh(app: &AppHandle, min_remaining_seconds: f64) -> bool {
    native_token_refresh(app, min_remaining_seconds, false).await
}

pub async fn refresh_session(app: &AppHandle) -> Result<(), String> {
    if native_token_refresh(app, MIN_CAPTURE_LIFETIME_SECONDS, true).await {
        Ok(())
    } else {
        Err("Could not renew the Ufora session silently. Please sign in again.".to_string())
    }
}

pub fn open_login_window(app: &AppHandle) -> Result<(), String> {
    ACCEPT_AUTH_RESULTS.store(true, Ordering::SeqCst);
    if let Some(silent_window) = app.get_webview_window(SILENT_REFRESH_LABEL) {
        let _ = silent_window.close();
    }
    if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return Ok(());
    }

    let blank_url: tauri::Url = "about:blank".parse().map_err(|e| format!("{e}"))?;
    let handle_nav = app.clone();

    let builder =
        WebviewWindowBuilder::new(app, LOGIN_WINDOW_LABEL, WebviewUrl::External(blank_url))
            .title("Log in to UGent Ufora")
            .inner_size(760.0, 800.0)
            .center()
            .visible(false)
            .initialization_script(AUTO_AUTH_AND_TOKEN_SCRIPT)
            .on_navigation(move |url| handle_auth_navigation(&handle_nav, LOGIN_WINDOW_LABEL, url));

    let win = builder.build().map_err(|e| e.to_string())?;
    restore_session_cookies(&win);
    let login_url = UFORA_SSO_LOGIN_URL
        .parse()
        .map_err(|e| format!("Invalid Ufora login URL: {e}"))?;
    win.navigate(login_url).map_err(|e| e.to_string())?;
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
    Ok(())
}

pub fn ensure_authenticated(min_remaining_seconds: f64) -> Result<String, String> {
    let token = read_token()?;
    if let Some(exp) = token.exp {
        let now = chrono::Utc::now().timestamp() as f64;
        if now + min_remaining_seconds >= exp {
            return Err(
                "Session expired. Please click 'Log In with UGent' to re-authenticate.".to_string(),
            );
        }
    }
    Ok(token.token)
}

pub async fn ensure_valid_token(
    app: &AppHandle,
    min_remaining_seconds: f64,
) -> Result<String, String> {
    // 1. Return immediately if token is currently valid
    if let Ok(token) = read_token() {
        if let Some(exp) = token.exp {
            let now = chrono::Utc::now().timestamp() as f64;
            if now + min_remaining_seconds < exp {
                return Ok(token.token);
            }
        }
    }

    // 2. Token is expired or near expiry -> attempt app-owned silent renewal
    if try_native_token_refresh(app, min_remaining_seconds).await {
        if let Ok(token) = read_token() {
            if let Some(exp) = token.exp {
                let now = chrono::Utc::now().timestamp() as f64;
                if now + min_remaining_seconds < exp {
                    let _ = app.emit("auth-status-changed", true);
                    let _ = crate::tray::update_tray_menu(app);
                    return Ok(token.token);
                }
            }
        }
    }

    // 3. Silent renewal failed or unavailable:
    // Mark auth status as false and update tray, but DO NOT pop up a window during background sync!
    let _ = app.emit("auth-status-changed", false);
    let _ = crate::tray::update_tray_menu(app);
    Err("Authentication required. Please sign in with your UGent account.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims_decoding() {
        // Standard JWT sample: {"sub":"1234567890","name":"John Doe","iat":1516239022,"exp":1999999999,"tenantid":"ugent-test"}
        let sample = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjE5OTk5OTk5OTksInRlbmFudGlkIjoidWdlbnQtdGVzdCJ9.4flK_i1xR1wU40Rk9E8vG3P1_zD4jYg6Gf8Xb1v_3v8";
        let claims = parse_jwt_claims(sample);
        assert!(claims.is_some());
        let (exp, sub, tenant) = claims.unwrap();
        assert_eq!(exp, Some(1999999999.0));
        assert_eq!(sub, Some("1234567890".to_string()));
        assert_eq!(tenant, Some("ugent-test".to_string()));
        assert!(token_has_minimum_lifetime(sample, 60.0));
        assert!(!token_has_minimum_lifetime(sample, 10_000_000_000.0));
    }

    #[test]
    fn test_auth_cookie_domain_filter_is_strict() {
        assert!(is_auth_cookie_domain("ufora.ugent.be"));
        assert!(is_auth_cookie_domain(".login.microsoftonline.com"));
        assert!(!is_auth_cookie_domain("ugent.be.example.com"));
        assert!(!is_auth_cookie_domain("microsoft.com"));
    }

    #[test]
    fn test_cli_session_cookie_schema_migrates_to_native_cookie() {
        let future = chrono::Utc::now().timestamp() as f64 + 3600.0;
        let json = serde_json::json!({
            "saved_at": chrono::Utc::now().timestamp(),
            "cookies": [{
                "name": "d2lSessionVal",
                "value": "session-secret",
                "domain": "ufora.ugent.be",
                "path": "/",
                "expires": future,
                "httpOnly": true,
                "secure": true,
                "sameSite": "None"
            }]
        });

        let session: SavedSession = serde_json::from_value(json).unwrap();
        let cookie = webview_cookie_from_saved(&session.cookies[0], future - 10.0).unwrap();
        assert_eq!(cookie.name(), "d2lSessionVal");
        assert_eq!(cookie.value(), "session-secret");
        assert_eq!(cookie.domain(), Some("ufora.ugent.be"));
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::None));
        assert!(webview_cookie_from_saved(&session.cookies[0], future + 10.0).is_none());
    }

    #[test]
    fn test_webview_session_cookie_round_trip_preserves_session_attributes() {
        let cookie = Cookie::build(("SimpleSAMLSessionID", "secret"))
            .domain("welkom.ugent.be")
            .path("/")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .build();

        let saved = saved_cookie_from_webview(&cookie).unwrap();
        assert_eq!(saved.expires, session_cookie_expiry());
        let restored =
            webview_cookie_from_saved(&saved, chrono::Utc::now().timestamp() as f64).unwrap();
        assert_eq!(restored.name(), cookie.name());
        assert_eq!(restored.value(), cookie.value());
        assert_eq!(restored.domain(), cookie.domain());
        assert_eq!(restored.http_only(), cookie.http_only());
        assert_eq!(restored.secure(), cookie.secure());
        assert_eq!(restored.same_site(), cookie.same_site());
    }

    #[test]
    fn test_logout_removes_file_safely() {
        // Must use an isolated temporary path so unit tests never touch ~/.d2l/token.json
        let test_dir = std::env::temp_dir().join(format!("ufora_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&test_dir);
        let token_path = test_dir.join("token.json");

        // 1. Should succeed if file does not exist
        assert!(!token_path.exists());
        let res1 = logout_at(&token_path);
        assert!(res1.is_ok());

        // 2. Should remove file if it exists
        std::fs::write(&token_path, "{}").unwrap();
        assert!(token_path.exists());
        let res2 = logout_at(&token_path);
        assert!(res2.is_ok());
        assert!(!token_path.exists());

        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
