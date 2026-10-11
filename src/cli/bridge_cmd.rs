// SPDX-License-Identifier: MIT

use std::path::Path;
use crate::cli::args::{BridgeArgs, LoginArgs};
use crate::core::error::{PropylonError, PropylonResult};

pub fn run_login(args: LoginArgs) -> PropylonResult<()> {
    if let Some(token) = args.token {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_dir = Path::new(&home).join(".config").join("propylon");
        let _ = std::fs::create_dir_all(&config_dir);
        let token_path = config_dir.join("token");
        std::fs::write(&token_path, token.trim().as_bytes())
            .map_err(|e| PropylonError::Internal(format!("Failed to write token: {e}")))?;
        println!("[PROPYLON] Enterprise session token saved to {}", token_path.display());
        return Ok(());
    }

    println!("[PROPYLON] Enterprise SAML 2.0 Web SSO Target: rIDM BPS");
    println!("1. Buka tautan berikut di browser: {}/saml/login", args.remote.trim_end_matches('/'));
    println!("2. Masukkan akun SSO BPS di portal rIDM.");
    println!("3. Setelah berhasil, salin token dan jalankan: propylon login --token <TOKEN>");
    Ok(())
}

pub async fn run_bridge(args: BridgeArgs) -> PropylonResult<()> {
    let token = args.token.or_else(|| {
        std::env::var("PROPYLON_TOKEN").ok().or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let token_path = Path::new(&home).join(".config").join("propylon").join("token");
            std::fs::read_to_string(token_path).ok().map(|s| s.trim().to_string())
        })
    });

    let endpoint = format!("{}/mcp", args.remote.trim_end_matches('/'));
    eprintln!("[PROPYLON] Stdio Bridge connected to remote {}", endpoint);
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

        let mut cmd = std::process::Command::new("curl");
        cmd.arg("-s").arg("-X").arg("POST")
            .arg("-H").arg("Content-Type: application/json")
            .arg("-H").arg("mcp-protocol-version: 2024-11-05")
            .arg(&endpoint)
            .arg("-d").arg(&line);

        if let Some(ref tok) = token {
            cmd.arg("-H").arg(format!("Authorization: Bearer {}", tok));
        }

        if let Ok(out) = cmd.output() {
            let resp_str = String::from_utf8_lossy(&out.stdout);
            if !resp_str.trim().is_empty() {
                let _ = writeln!(stdout, "{}", resp_str.trim());
                let _ = stdout.flush();
            }
        }
    }

    Ok(())
}
