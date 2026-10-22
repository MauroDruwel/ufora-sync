use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

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

pub fn find_ufora_exe() -> Option<PathBuf> {
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
