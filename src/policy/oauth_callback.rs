// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for OAuth Callback Server and Redirect URI generation (MikkoParkkola/mcp-gateway#2578)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallbackServerConfig {
    /// Configured host or IP literal (e.g., "localhost", "127.0.0.1", "::1", "0.0.0.0")
    pub callback_host: String,
    /// Configured or dynamically bound port
    pub callback_port: u16,
    /// Callback path (defaults to "/oauth/callback")
    pub callback_path: String,
    /// Protocol scheme ("http" or "https")
    pub scheme: String,
    /// Whether to honor X-Forwarded-* headers when behind an enterprise reverse proxy
    pub trust_proxy_headers: bool,
}

impl Default for CallbackServerConfig {
    fn default() -> Self {
        Self {
            callback_host: "localhost".to_string(),
            callback_port: 8080,
            callback_path: "/oauth/callback".to_string(),
            scheme: "http".to_string(),
            trust_proxy_headers: false,
        }
    }
}

/// Resolves the exact redirect URI advertised to identity providers without unintended "localhost" fallback
#[derive(Debug, Clone)]
pub struct OAuthCallbackResolver {
    config: CallbackServerConfig,
}

impl OAuthCallbackResolver {
    pub fn new(config: CallbackServerConfig) -> Self {
        Self { config }
    }

    pub fn with_host_and_port(host: impl Into<String>, port: u16) -> Self {
        Self {
            config: CallbackServerConfig {
                callback_host: host.into(),
                callback_port: port,
                ..Default::default()
            },
        }
    }

    /// Return the current configured host
    pub fn callback_host(&self) -> &str {
        &self.config.callback_host
    }

    /// Return the current configured port
    pub fn callback_port(&self) -> u16 {
        self.config.callback_port
    }

    /// Check if configured host is a local loopback target
    pub fn is_loopback(&self) -> bool {
        let h = self.config.callback_host.to_lowercase();
        h == "localhost" || h == "127.0.0.1" || h == "::1" || h == "[::1]" || h.starts_with("127.")
    }

    /// Formats an IP or hostname for HTTP URL authority, bracketing unbracketed IPv6 literals
    pub fn format_host_authority(host: &str) -> String {
        if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
            format!("[{}]", host)
        } else {
            host.to_string()
        }
    }

    /// Resolves the canonical advertised redirect URI.
    /// If reverse proxy headers are present and trusted, uses the forwarded host and proto.
    /// Otherwise, preserves the exact configured host without forcing "localhost".
    pub fn resolve_redirect_uri(&self, headers: Option<&HashMap<String, String>>) -> String {
        let path = if self.config.callback_path.starts_with('/') {
            self.config.callback_path.clone()
        } else {
            format!("/{}", self.config.callback_path)
        };

        if self.config.trust_proxy_headers {
            if let Some(hdrs) = headers {
                let fwd_host = hdrs.get("x-forwarded-host").or_else(|| hdrs.get("X-Forwarded-Host"));
                let fwd_proto = hdrs.get("x-forwarded-proto").or_else(|| hdrs.get("X-Forwarded-Proto"));
                let fwd_port = hdrs.get("x-forwarded-port").or_else(|| hdrs.get("X-Forwarded-Port"));

                if let Some(host) = fwd_host {
                    let proto = fwd_proto.map(|p| p.as_str()).unwrap_or("https");
                    let host_clean = host.split(',').next().unwrap_or(host).trim();

                    if host_clean.contains(':') || fwd_port.is_none() {
                        return format!("{}://{}{}", proto, host_clean, path);
                    } else if let Some(port) = fwd_port {
                        let p = port.trim();
                        if (proto == "http" && p == "80") || (proto == "https" && p == "443") {
                            return format!("{}://{}{}", proto, host_clean, path);
                        } else {
                            return format!("{}://{}:{}{}", proto, host_clean, p, path);
                        }
                    }
                }
            }
        }

        let host_formatted = Self::format_host_authority(&self.config.callback_host);
        let port = self.config.callback_port;
        let scheme = &self.config.scheme;

        let port_suffix = if (scheme == "http" && port == 80) || (scheme == "https" && port == 443) {
            String::new()
        } else {
            format!(":{}", port)
        };

        format!("{}://{}{}{}", scheme, host_formatted, port_suffix, path)
    }
}
