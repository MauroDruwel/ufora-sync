use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const UFORA_HOME_URL: &str = "https://ufora.ugent.be/d2l/home";
pub const LOGIN_WINDOW_LABEL: &str = "ufora-login";
pub const SILENT_REFRESH_LABEL: &str = "ufora-silent-refresh";

const TOKEN_CAPTURE_SCRIPT: &str = r#"
(async () => {
    try {
        // 1. Try local storage first
        const raw = window.localStorage.getItem('D2L.Fetch.Tokens');
        if (raw) {
            try {
                const data = JSON.parse(raw);
                for (const item of Object.values(data)) {
                    if (item && item.access_token && item.access_token.startsWith('eyJ')) {
                        window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(item.access_token);
                        return;
                    }
                }
            } catch (_) {}
        }

        // 2. Fetch fresh token from D2L oauth endpoint
        let xsrf = window.localStorage.getItem('XSRF.Token');
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
                    window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(tokenData.access_token);
                    return;
                }
            }
        }
    } catch (e) {
        console.error('Ufora token capture error:', e);
    }
})();
"#;

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

pub fn is_token_expired(token: &D2lToken) -> bool {
    if let Some(exp) = token.exp {
        let now = chrono::Utc::now().timestamp() as f64;
        now >= (exp - 60.0)
    } else {
        false
    }
}

pub fn open_login_window(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = win.set_focus();
        let _ = win.unminimize();
        return Ok(());
    }

    let url: tauri::Url = UFORA_HOME_URL.parse().map_err(|e| format!("{e}"))?;
    let handle_nav = app.clone();

    let builder = WebviewWindowBuilder::new(app, LOGIN_WINDOW_LABEL, WebviewUrl::External(url))
        .title("Log in to UGent Ufora")
        .inner_size(720.0, 780.0)
        .center()
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
                let url_str = payload.url().as_str();
                if url_str.contains("ufora.ugent.be/d2l/home")
                    || url_str.contains("ufora.ugent.be/d2l/")
                {
                    let _ = window.eval(TOKEN_CAPTURE_SCRIPT);
                }
            }
        });

    builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn trigger_silent_refresh(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(LOGIN_WINDOW_LABEL).is_some()
        || app.get_webview_window(SILENT_REFRESH_LABEL).is_some()
    {
        return Ok(());
    }

    let url: tauri::Url = UFORA_HOME_URL.parse().map_err(|e| format!("{e}"))?;
    let handle_nav = app.clone();

    let builder = WebviewWindowBuilder::new(app, SILENT_REFRESH_LABEL, WebviewUrl::External(url))
        .visible(false)
        .title("Ufora Background Refresh")
        .inner_size(400.0, 400.0)
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
                let url_str = payload.url().as_str();
                if url_str.contains("ufora.ugent.be/d2l/home")
                    || url_str.contains("ufora.ugent.be/d2l/")
                {
                    let _ = window.eval(TOKEN_CAPTURE_SCRIPT);
                }
            }
        });

    builder.build().map_err(|e| e.to_string())?;

    // Auto-timeout after 20 seconds so background webview never hangs
    let handle_timeout = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
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
        if now + min_remaining_seconds >= exp && now >= exp {
            return Err(
                "Session expired. Please click 'Log In with UGent' to re-authenticate.".to_string(),
            );
        }
    }
    Ok(token.token)
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
}
