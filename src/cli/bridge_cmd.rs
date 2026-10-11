// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};
use crate::cli::args::{BridgeArgs, LoginArgs};
use crate::core::error::{PropylonError, PropylonResult};

fn get_token_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".config").join("propylon").join("token")
}

fn write_token_file(path: &Path, token: &str) -> PropylonResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| PropylonError::Internal(format!("Failed to create config dir: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }

    std::fs::write(path, token.trim().as_bytes())
        .map_err(|e| PropylonError::Internal(format!("Failed to write token: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

pub fn run_login(args: LoginArgs) -> PropylonResult<()> {
    if let Some(token) = args.token {
        let token_path = get_token_path();
        write_token_file(&token_path, &token)?;
        println!("[PROPYLON] Enterprise session token saved to {} (POSIX 0600 mode)", token_path.display());
        return Ok(());
    }

    println!("[PROPYLON] Enterprise SAML 2.0 Web SSO Target: rIDM BPS");
    println!("1. Buka tautan berikut di browser: {}/saml/login", args.remote.trim_end_matches('/'));
    println!("2. Masukkan akun SSO BPS di portal rIDM.");
    println!("3. Setelah berhasil, salin token dan jalankan: propylon login --token <TOKEN>");
    Ok(())
}

fn refresh_remote_token(remote: &str, old_token: &str) -> Option<String> {
    let refresh_url = format!("{}/api/v1/auth/refresh", remote.trim_end_matches('/'));
    let payload = serde_json::json!({ "token": old_token }).to_string();

    let mut cmd = std::process::Command::new("curl");
    cmd.arg("-s").arg("-X").arg("POST")
        .arg("-H").arg("Content-Type: application/json")
        .arg(&refresh_url)
        .arg("-d").arg(&payload);

    if let Ok(out) = cmd.output() {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
            if let Some(new_tok) = val.get("access_token").and_then(|v| v.as_str()) {
                return Some(new_tok.to_string());
            }
        }
    }
    None
}

fn is_jwt_expiring_soon(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    if let Ok(raw) = URL_SAFE_NO_PAD.decode(parts[1]) {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&raw) {
            if let Some(exp) = val.get("exp").and_then(|v| v.as_i64()) {
                let now = chrono::Utc::now().timestamp();
                return exp - now < 300;
            }
        }
    }
    false
}

pub async fn run_bridge(args: BridgeArgs) -> PropylonResult<()> {
    let token_path = get_token_path();
    let mut token = args.token.or_else(|| {
        std::env::var("PROPYLON_TOKEN").ok().or_else(|| {
            std::fs::read_to_string(&token_path).ok().map(|s| s.trim().to_string())
        })
    });

    let endpoint = format!("{}/mcp", args.remote.trim_end_matches('/'));
    eprintln!("[PROPYLON] Zero-Knowledge Stdio Bridge connected to remote {}", endpoint);
    if token.is_none() {
        eprintln!("[PROPYLON] Notice: No Bearer token found. Run 'propylon login' if strict zero-trust is active.");
    }

    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        if let Some(ref current_tok) = token {
            if is_jwt_expiring_soon(current_tok) {
                if let Some(fresh_tok) = refresh_remote_token(&args.remote, current_tok) {
                    eprintln!("[PROPYLON] Access token rolled proactively via RTR");
                    let _ = write_token_file(&token_path, &fresh_tok);
                    token = Some(fresh_tok);
                }
            }
        }

        let send_req = |tok: Option<&str>| {
            let mut cmd = std::process::Command::new("curl");
            cmd.arg("-s").arg("-X").arg("POST")
                .arg("-H").arg("Content-Type: application/json")
                .arg("-H").arg("mcp-protocol-version: 2024-11-05")
                .arg(&endpoint)
                .arg("-d").arg(&line);

            if let Some(t) = tok {
                cmd.arg("-H").arg(format!("Authorization: Bearer {}", t));
            }
            cmd.output()
        };

        let mut out = send_req(token.as_deref());

        if let Ok(ref res) = out {
            let body_str = String::from_utf8_lossy(&res.stdout);
            if body_str.contains("-32001") || body_str.contains("expired") {
                if let Some(ref current_tok) = token {
                    if let Some(fresh_tok) = refresh_remote_token(&args.remote, current_tok) {
                        eprintln!("[PROPYLON] Access token rolled reactively on 401 via RTR");
                        let _ = write_token_file(&token_path, &fresh_tok);
                        token = Some(fresh_tok);
                        out = send_req(token.as_deref());
                    }
                }
            }
        }

        if let Ok(res) = out {
            let resp_str = String::from_utf8_lossy(&res.stdout);
            if !resp_str.trim().is_empty() {
                let _ = writeln!(stdout, "{}", resp_str.trim());
                let _ = stdout.flush();
            }
        }
    }

    Ok(())
}
