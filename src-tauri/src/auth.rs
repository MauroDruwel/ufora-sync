use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const UFORA_HOME_URL: &str = "https://ufora.ugent.be/d2l/home";
pub const UFORA_SSO_LOGIN_URL: &str =
    "https://elosp.ugent.be/welcome/uforalogin?target=%2fd2l%2fhome";
pub const LOGIN_WINDOW_LABEL: &str = "ufora-login";
pub const SILENT_REFRESH_LABEL: &str = "ufora-silent-refresh";

const AUTO_AUTH_AND_TOKEN_SCRIPT: &str = r##"
(function() {
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

    function triggerClick(el) {
        if (!el) return;
        try {
            el.focus();
        } catch (_) {}
        try {
            const opts = { bubbles: true, cancelable: true, view: window };
            el.dispatchEvent(new MouseEvent('mousedown', opts));
            el.dispatchEvent(new MouseEvent('mouseup', opts));
            el.dispatchEvent(new MouseEvent('click', opts));
        } catch (_) {}
        try {
            el.click();
        } catch (_) {}
    }

    function tryAuth() {
        try {
            const host = window.location.hostname || "";
            const href = window.location.href || "";
            const nowSec = Date.now() / 1000;

            // 1. Microsoft Entra SSO (login.microsoftonline.com / login.microsoft.com)
            if (host.includes("microsoftonline.com") || host.includes("microsoft.com")) {
                // Auto-click saved UGent account tile if presented with account picker
                const tile = document.querySelector(
                    "[data-test-id*='ugent'], [data-test-id*='@UGent.be'], [data-test-id*='@ugent.be'], .tile [role='button']:not(#otherTile), div[data-report-value='Email_Picker_Tile'], table.tile, .table-row.tile"
                );
                if (tile) {
                    triggerClick(tile);
                    return true;
                }

                // Auto-click "Yes" on "Stay signed in?" prompt (#idSIButton9)
                const dontShowCheckbox = document.querySelector("input[name='DontShowAgain'], #KmsiCheckboxField");
                if (dontShowCheckbox && !dontShowCheckbox.checked) {
                    triggerClick(dontShowCheckbox);
                }
                const yesBtn = document.querySelector("#idSIButton9, input[id='idSIButton9'], button[id='idSIButton9'], input[value='Yes'], input[value='Ja']");
                if (yesBtn) {
                    triggerClick(yesBtn);
                    return true;
                }
            }

            // 2. UGent Elosp portal
            if (host.includes("elosp.ugent.be")) {
                const uforaLink = document.querySelector("a[href*='uforalogin'], a[href*='d2l']");
                if (uforaLink) {
                    triggerClick(uforaLink);
                    return true;
                }
            }

            // 3. Ufora Login Landing page (if redirected to /d2l/login or /d2l/lp/auth/login)
            if (host.includes("ufora.ugent.be") && (href.includes("/d2l/login") || href.includes("/d2l/lp/auth/login"))) {
                const loginBtn = document.querySelector("a[href*='uforalogin'], a[href*='saml']");
                if (loginBtn) {
                    triggerClick(loginBtn);
                    return true;
                } else if (!href.includes("target=")) {
                    window.location.href = "https://elosp.ugent.be/welcome/uforalogin?target=%2fd2l%2fhome";
                    return true;
                }
            }

            // 4. Ufora Authenticated page -> extract token
            if (host.includes("ufora.ugent.be") && (href.includes("/d2l/home") || href.includes("/d2l/"))) {
                // Try local storage first (validate exp > nowSec + 60)
                try {
                    const raw = window.localStorage.getItem('D2L.Fetch.Tokens');
                    if (raw) {
                        const data = JSON.parse(raw);
                        let bestToken = null;
                        let bestExp = 0;
                        for (const item of Object.values(data)) {
                            if (item && item.access_token && item.access_token.startsWith('eyJ')) {
                                const exp = parseJwtExp(item.access_token);
                                if (exp > (nowSec + 60) && exp > bestExp) {
                                    bestToken = item.access_token;
                                    bestExp = exp;
                                }
                            }
                        }
                        if (bestToken) {
                            window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(bestToken);
                            return true;
                        }
                    }
                } catch (_) {}

                // Fetch fresh token from D2L oauth endpoint
                (async () => {
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
                                    if (exp > (nowSec + 60)) {
                                        window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(tokenData.access_token);
                                    }
                                }
                            }
                        }
                    } catch (e) {
                        console.error('Ufora token capture error:', e);
                    }
                })();
            }
        } catch (e) {
            console.error('Auto auth helper error:', e);
        }
        return false;
    }

    // Intercept fetch to capture Bearer tokens live from any background request
    try {
        const origFetch = window.fetch;
        window.fetch = function(...args) {
            const req = args[0];
            const opts = args[1] || {};
            const headers = opts.headers || (req && req.headers);
            let auth = null;
            if (headers) {
                if (typeof headers.get === 'function') {
                    auth = headers.get('authorization') || headers.get('Authorization');
                } else if (headers['authorization']) {
                    auth = headers['authorization'];
                } else if (headers['Authorization']) {
                    auth = headers['Authorization'];
                }
            }
            if (auth && typeof auth === 'string' && auth.startsWith('Bearer eyJ')) {
                const tok = auth.substring(7);
                const exp = parseJwtExp(tok);
                if (exp > ((Date.now() / 1000) + 60)) {
                    window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(tok);
                }
            }
            return origFetch.apply(this, args);
        };
    } catch (_) {}

    tryAuth();
    setInterval(tryAuth, 600);
    window.addEventListener('DOMContentLoaded', tryAuth);
    window.addEventListener('load', tryAuth);
})();
"##;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D2lToken {
    pub token: String,
    pub exp: Option<f64>,
    pub sub: Option<String>,
    pub tenant: Option<String>,
}

pub fn get_token_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".d2l")
        .join("token.json")
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

pub fn save_token_str(token: &str) -> Result<(), String> {
    let (exp, sub, tenant) = parse_jwt_claims(token).unwrap_or((None, None, None));
    let path = get_token_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::json!({
        "token": token,
        "exp": exp,
        "sub": sub,
        "tenant": tenant,
        "captured_at": chrono::Utc::now().timestamp(),
    });
    let s = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&path, s).map_err(|e| format!("Failed to save token file: {e}"))?;
    Ok(())
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
    logout_at(&get_token_path())
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

pub fn open_login_window(app: &AppHandle) -> Result<(), String> {
    // If background silent refresh is open, close it so they don't collide
    if let Some(silent_win) = app.get_webview_window(SILENT_REFRESH_LABEL) {
        let _ = silent_win.close();
    }

    if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return Ok(());
    }

    let url: tauri::Url = UFORA_SSO_LOGIN_URL.parse().map_err(|e| format!("{e}"))?;
    let handle_nav = app.clone();

    let builder = WebviewWindowBuilder::new(app, LOGIN_WINDOW_LABEL, WebviewUrl::External(url))
        .title("Log in to UGent Ufora")
        .inner_size(760.0, 800.0)
        .center()
        .visible(true)
        .initialization_script(AUTO_AUTH_AND_TOKEN_SCRIPT)
        .on_navigation(move |url| {
            if url.as_str().contains("__auth_done?token=") {
                if let Some(query) = url.query() {
                    for pair in query.split('&') {
                        if let Some(token_val) = pair.strip_prefix("token=") {
                            let decoded = percent_encoding::percent_decode_str(token_val)
                                .decode_utf8_lossy()
                                .to_string();
                            if !decoded.is_empty() {
                                let _ = save_token_str(&decoded);
                                let _ = handle_nav.emit("auth-status-changed", true);
                                let _ = crate::tray::update_tray_menu(&handle_nav);
                                if let Some(w) = handle_nav.get_webview_window(LOGIN_WINDOW_LABEL) {
                                    let _ = w.close();
                                }
                                return false;
                            }
                        }
                    }
                }
                return false;
            }
            true
        })
        .on_page_load(|window, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = window.eval(AUTO_AUTH_AND_TOKEN_SCRIPT);
            }
        });

    let win = builder.build().map_err(|e| e.to_string())?;
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
    Ok(())
}

pub fn trigger_silent_refresh(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(LOGIN_WINDOW_LABEL).is_some()
        || app.get_webview_window(SILENT_REFRESH_LABEL).is_some()
    {
        return Ok(());
    }

    let url: tauri::Url = UFORA_SSO_LOGIN_URL.parse().map_err(|e| format!("{e}"))?;
    let handle_nav = app.clone();

    let builder = WebviewWindowBuilder::new(app, SILENT_REFRESH_LABEL, WebviewUrl::External(url))
        .visible(false)
        .title("Ufora Background Refresh")
        .inner_size(400.0, 400.0)
        .initialization_script(AUTO_AUTH_AND_TOKEN_SCRIPT)
        .on_navigation(move |url| {
            if url.as_str().contains("__auth_done?token=") {
                if let Some(query) = url.query() {
                    for pair in query.split('&') {
                        if let Some(token_val) = pair.strip_prefix("token=") {
                            let decoded = percent_encoding::percent_decode_str(token_val)
                                .decode_utf8_lossy()
                                .to_string();
                            if !decoded.is_empty() {
                                let _ = save_token_str(&decoded);
                                let _ = handle_nav.emit("auth-status-changed", true);
                                let _ = crate::tray::update_tray_menu(&handle_nav);
                                if let Some(w) = handle_nav.get_webview_window(SILENT_REFRESH_LABEL)
                                {
                                    let _ = w.close();
                                }
                                return false;
                            }
                        }
                    }
                }
                return false;
            }
            true
        })
        .on_page_load(|window, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = window.eval(AUTO_AUTH_AND_TOKEN_SCRIPT);
            }
        });

    builder.build().map_err(|e| e.to_string())?;

    // Auto-timeout after 12 seconds so background webview never hangs indefinitely
    let handle_timeout = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(12)).await;
        if let Some(w) = handle_timeout.get_webview_window(SILENT_REFRESH_LABEL) {
            let _ = w.close();
        }
    });

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

    // 2. Token is expired or near expiry -> attempt fast silent background refresh first (4 seconds)
    let _ = trigger_silent_refresh(app);

    for _ in 0..8 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Ok(token) = read_token() {
            if let Some(exp) = token.exp {
                let now = chrono::Utc::now().timestamp() as f64;
                if now + min_remaining_seconds < exp {
                    return Ok(token.token);
                }
            }
        }
    }

    // 3. Silent renewal did not conclude quickly -> close silent window and open visible login window
    // so student sees account chooser or login prompt immediately with zero delay
    if let Some(w) = app.get_webview_window(SILENT_REFRESH_LABEL) {
        let _ = w.close();
    }
    let _ = open_login_window(app);

    // Poll for up to 60 seconds while user or SSO helper finishes
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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
