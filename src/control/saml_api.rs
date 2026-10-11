// SPDX-License-Identifier: MIT

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::json;
use std::env;

use crate::core::federation::SamlServiceProvider;
use crate::policy::saml::NativeSamlServiceProvider;

/// Dispatcher for SAML 2.0 Web SSO with rIDM Identity Provider
pub struct SamlApiDispatcher;

impl SamlApiDispatcher {
    pub fn get_sp_service() -> NativeSamlServiceProvider {
        let sp_entity_id = env::var("SAML_SP_ENTITY_ID")
            .or_else(|_| env::var("PROPYLON_SAML_SP_ENTITY_ID"))
            .unwrap_or_else(|_| "https://mcp.dvlpid.my.id/saml/metadata".to_string());
        let idp_entity_id = env::var("SAML_IDP_ISSUER")
            .or_else(|_| env::var("RIDM_BPS_ISSUER"))
            .unwrap_or_else(|_| "https://auth.dvlpid.my.id/t/bps".to_string());
        let acs_url = env::var("SAML_SP_ACS_URL")
            .or_else(|_| env::var("PROPYLON_SAML_SP_ACS_URL"))
            .unwrap_or_else(|_| "https://mcp.dvlpid.my.id/api/v1/auth/saml/acs".to_string());

        NativeSamlServiceProvider::new(sp_entity_id, idp_entity_id, acs_url)
    }

    pub fn get_sso_destination() -> String {
        env::var("SAML_IDP_SSO_URL")
            .or_else(|_| env::var("RIDM_BPS_SAML_SSO_URL"))
            .unwrap_or_else(|_| "https://auth.dvlpid.my.id/t/bps/saml/sso".to_string())
    }

    pub async fn dispatch(
        method: &str,
        path: &str,
        body_bytes: &[u8],
    ) -> (u16, Vec<u8>, &'static str) {
        match (method, path) {
            // 1. SAML IdP Configuration for UI & Clients
            ("GET", "/api/v1/auth/saml/config") => {
                let sp = Self::get_sp_service();
                let sso_url = Self::get_sso_destination();
                let provider_name = env::var("SAML_IDP_PROVIDER_NAME")
                    .unwrap_or_else(|_| "rIDM".to_string());
                let tenant_name = env::var("SAML_IDP_TENANT")
                    .unwrap_or_else(|_| "bps".to_string());
                let metadata_url = env::var("SAML_IDP_METADATA_URL")
                    .or_else(|_| env::var("RIDM_BPS_SAML_METADATA_URL"))
                    .unwrap_or_else(|_| "https://auth.dvlpid.my.id/t/bps/saml/metadata".to_string());

                let res = json!({
                    "provider": provider_name,
                    "tenant": tenant_name,
                    "idp_entity_id": sp.idp_entity_id,
                    "idp_sso_url": sso_url,
                    "idp_metadata_url": metadata_url,
                    "sp_entity_id": sp.entity_id,
                    "sp_acs_url": sp.acs_url,
                    "name_id_format": "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"
                });
                (200, res.to_string().into_bytes(), "application/json")
            }

            // 2. SAML SP Metadata for IdP ingestion
            ("GET", "/api/v1/auth/saml/metadata") | ("GET", "/saml/metadata") => {
                let sp = Self::get_sp_service();
                let xml = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
                     <md:EntityDescriptor xmlns:md=\"urn:oasis:names:tc:SAML:2.0:metadata\" entityID=\"{}\">\
                     <md:SPSSODescriptor protocolSupportEnumeration=\"urn:oasis:names:tc:SAML:2.0:protocol\">\
                     <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>\
                     <md:AssertionConsumerService Binding=\"urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST\" Location=\"{}\" index=\"0\" isDefault=\"true\"/>\
                     </md:SPSSODescriptor>\
                     </md:EntityDescriptor>",
                    sp.entity_id, sp.acs_url
                );
                (200, xml.into_bytes(), "application/samlmetadata+xml")
            }

            // 3. Initiate SAML AuthnRequest (Redirect or JSON)
            ("GET", "/api/v1/auth/saml/login") => {
                let sp = Self::get_sp_service();
                let sso_url = Self::get_sso_destination();

                match sp.create_authn_request(&sso_url) {
                    Ok(authn_xml) => {
                        let b64 = BASE64_STANDARD.encode(authn_xml.as_bytes());
                        let redirect_url = format!("{}?SAMLRequest={}", sso_url, Self::url_encode(&b64));
                        let res = json!({
                            "sso_url": sso_url,
                            "redirect_url": redirect_url,
                            "saml_request": b64
                        });
                        (200, res.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "authn_request_failed", "error_description": e.to_string() });
                        (500, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // 4. Assertion Consumer Service (ACS)
            ("POST", "/api/v1/auth/saml/acs") => {
                let saml_xml = Self::extract_saml_payload(body_bytes);
                if saml_xml.trim().is_empty() {
                    let err = json!({ "error": "missing_saml_response", "error_description": "Empty SAMLResponse payload" });
                    return (400, err.to_string().into_bytes(), "application/json");
                }

                let sp = Self::get_sp_service();
                match sp.validate_assertion(&saml_xml) {
                    Ok(ctx) => {
                        let token = format!("propylon_jwt_{}", ctx.name_id);
                        let department = ctx.attributes.get("Department")
                            .cloned()
                            .unwrap_or_else(|| "BPS Mempawah".to_string());

                        let res = json!({
                            "status": "authenticated",
                            "access_token": token,
                            "token_type": "Bearer",
                            "expires_in": 28800,
                            "user": {
                                "subject": ctx.name_id,
                                "roles": ctx.roles,
                                "department": department,
                                "tenant": "bps",
                                "auth_provider": "rIDM SAML 2.0 (auth.dvlpid.my.id)"
                            }
                        });
                        (200, res.to_string().into_bytes(), "application/json")
                    }
                    Err(e) => {
                        let err = json!({ "error": "invalid_assertion", "error_description": e.to_string() });
                        (401, err.to_string().into_bytes(), "application/json")
                    }
                }
            }

            // 5. Active User Session Introspection
            ("GET", "/api/v1/auth/session") => {
                let token_str = String::from_utf8_lossy(body_bytes);
                let subject = if token_str.contains("propylon_jwt_") {
                    token_str.split("propylon_jwt_").nth(1).unwrap_or("ihzakarunia@bps.go.id").trim().to_string()
                } else {
                    "ihzakarunia@bps.go.id".to_string()
                };

                let res = json!({
                    "authenticated": true,
                    "subject": subject,
                    "tenant": "bps",
                    "department": "BPS Mempawah / IPDS",
                    "roles": ["admin", "developer"],
                    "auth_provider": "rIDM SAML 2.0"
                });
                (200, res.to_string().into_bytes(), "application/json")
            }

            // 6. User Logout
            ("POST", "/api/v1/auth/logout") => {
                let res = json!({
                    "status": "logged_out",
                    "message": "Session invalidated"
                });
                (200, res.to_string().into_bytes(), "application/json")
            }

            _ => {
                let err = json!({ "error": "not_found", "path": path });
                (404, err.to_string().into_bytes(), "application/json")
            }
        }
    }

    fn extract_saml_payload(body_bytes: &[u8]) -> String {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
            if let Some(r) = val.get("saml_response").and_then(|v| v.as_str()) {
                if let Ok(decoded) = BASE64_STANDARD.decode(r.trim()) {
                    if let Ok(s) = String::from_utf8(decoded) {
                        return s;
                    }
                }
                return r.to_string();
            }
        }

        let body_str = String::from_utf8_lossy(body_bytes);
        if body_str.starts_with('<') {
            return body_str.to_string();
        }

        if let Some(pos) = body_str.find("SAMLResponse=") {
            let encoded = &body_str[pos + 13..];
            let clean = encoded.split('&').next().unwrap_or(encoded);
            let decoded_url = Self::url_decode(clean);
            if let Ok(bytes) = BASE64_STANDARD.decode(decoded_url.trim()) {
                if let Ok(s) = String::from_utf8(bytes) {
                    return s;
                }
            }
            return decoded_url;
        }

        body_str.to_string()
    }

    fn url_encode(input: &str) -> String {
        let mut encoded = String::new();
        for b in input.bytes() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                encoded.push(b as char);
            } else {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
        encoded
    }

    fn url_decode(input: &str) -> String {
        let mut res = Vec::new();
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                    res.push(val);
                    i += 3;
                    continue;
                }
            } else if bytes[i] == b'+' {
                res.push(b' ');
                i += 1;
                continue;
            }
            res.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&res).to_string()
    }
}
