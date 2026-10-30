use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const UFORA_HOME_URL: &str = "https://ufora.ugent.be/d2l/home";
pub const UFORA_SSO_LOGIN_URL: &str =
    "https://elosp.ugent.be/welcome/uforalogin?target=%2fd2l%2fhome";
pub const LOGIN_WINDOW_LABEL: &str = "ufora-login";
pub const SILENT_REFRESH_LABEL: &str = "ufora-silent-refresh";

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
            // Check local storage first
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
                        return;
                    }
                }
            } catch (_) {}

            // Request fresh token from D2L oauth endpoint
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
                } catch (_) {}
            })();
        }
    }

    // Intercept fetch ONLY on ufora.ugent.be to capture Bearer tokens from live requests
    try {
        if (window.location.hostname.includes("ufora.ugent.be")) {
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
                        if (exp > ((Date.now() / 1000) + 60)) {
                            window.location.href = 'https://ufora.ugent.be/__auth_done?token=' + encodeURIComponent(tok);
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

/// Headless silent background renewal using the system `ufora refresh` CLI (from ufora-ai / python).
/// Returns true if a fresh valid token was captured into ~/.d2l/token.json.
pub async fn try_ufora_cli_refresh() -> bool {
    let candidates = [
        "ufora",
        "/Library/Frameworks/Python.framework/Versions/3.14/bin/ufora",
        "/Library/Frameworks/Python.framework/Versions/Current/bin/ufora",
        "/usr/local/bin/ufora",
        "/opt/homebrew/bin/ufora",
    ];

    for exe in candidates {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(35),
            tokio::process::Command::new(exe).arg("refresh").output(),
        )
        .await;

        if let Ok(Ok(out)) = output {
            if out.status.success() {
                return true;
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        let local_ufora = home.join(".local").join("bin").join("ufora");
        if local_ufora.exists() {
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(35),
                tokio::process::Command::new(&local_ufora)
                    .arg("refresh")
                    .output(),
            )
            .await;

            if let Ok(Ok(out)) = output {
                if out.status.success() {
                    return true;
                }
            }
        }
    }

    false
}

pub fn open_login_window(app: &AppHandle) -> Result<(), String> {
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
        });

    let win = builder.build().map_err(|e| e.to_string())?;
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

    // 2. Token is expired or near expiry -> attempt silent CLI renewal
    if try_ufora_cli_refresh().await {
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
