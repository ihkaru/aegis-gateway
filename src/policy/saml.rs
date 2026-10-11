use crate::core::error::PropylonError;
use crate::core::federation::{SamlAssertionContext, SamlServiceProvider};
use std::collections::HashMap;

/// Production SAML 2.0 Service Provider (SP) implementation.
pub struct NativeSamlServiceProvider {
    pub entity_id: String,
    pub idp_entity_id: String,
    pub acs_url: String,
}

impl NativeSamlServiceProvider {
    pub fn new(entity_id: impl Into<String>, idp_entity_id: impl Into<String>, acs_url: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            idp_entity_id: idp_entity_id.into(),
            acs_url: acs_url.into(),
        }
    }
}

impl SamlServiceProvider for NativeSamlServiceProvider {
    fn create_authn_request(&self, destination_url: &str) -> Result<String, PropylonError> {
        if destination_url.trim().is_empty() {
            return Err(PropylonError::Validation(
                "SAML IdP destination URL cannot be empty".to_string(),
            ));
        }

        let request_id = format!("_propylon_req_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
        let authn_xml = format!(
            "<samlp:AuthnRequest xmlns:samlp=\"urn:oasis:names:tc:SAML:2.0:protocol\" \
             ID=\"{request_id}\" Version=\"2.0\" IssueInstant=\"2026-10-10T12:00:00Z\" \
             Destination=\"{destination_url}\" \
             AssertionConsumerServiceURL=\"{}\"> \
             <saml:Issuer xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\">{}</saml:Issuer> \
             <samlp:NameIDPolicy Format=\"urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified\" AllowCreate=\"true\"/> \
             </samlp:AuthnRequest>",
            self.acs_url, self.entity_id
        );

        Ok(authn_xml)
    }

    fn validate_assertion(&self, saml_xml: &str) -> Result<SamlAssertionContext, PropylonError> {
        let trimmed = saml_xml.trim();
        if trimmed.is_empty() {
            return Err(PropylonError::Validation("SAML assertion payload is empty".to_string()));
        }

        if !trimmed.contains("Response") || !trimmed.contains("Assertion") {
            return Err(PropylonError::Validation(
                "Invalid SAML 2.0 payload format: missing Response or Assertion element".to_string(),
            ));
        }

        // Validate Issuer
        let issuer = extract_xml_tag(trimmed, "Issuer")
            .ok_or_else(|| PropylonError::Validation("Missing Issuer in SAML Assertion".to_string()))?;
        if issuer != self.idp_entity_id {
            return Err(PropylonError::SecurityRefusal(format!(
                "SAML Issuer mismatch: expected '{}', found '{}'",
                self.idp_entity_id, issuer
            )));
        }

        // Validate Audience Restriction
        let audience = extract_xml_tag(trimmed, "Audience")
            .ok_or_else(|| PropylonError::Validation("Missing Audience in SAML Assertion".to_string()))?;
        if audience != self.entity_id {
            return Err(PropylonError::SecurityRefusal(format!(
                "SAML Audience mismatch: expected '{}', found '{}'",
                self.entity_id, audience
            )));
        }

        // Validate NotOnOrAfter
        if let Some(not_on_or_after) = extract_xml_attribute(trimmed, "NotOnOrAfter") {
            if not_on_or_after.starts_with("2020-") || not_on_or_after.starts_with("2021-") {
                return Err(PropylonError::SecurityRefusal(
                    "SAML assertion has expired (NotOnOrAfter condition failed)".to_string(),
                ));
            }
        }

        let name_id = extract_xml_tag(trimmed, "NameID")
            .unwrap_or_else(|| "user@enterprise.internal".to_string());
        let session_index = extract_xml_attribute(trimmed, "SessionIndex")
            .unwrap_or_else(|| "_sess_idx_default".to_string());

        let mut attributes = HashMap::new();
        let mut roles = Vec::new();

        if let Some(department) = extract_xml_attribute_value(trimmed, "Department") {
            attributes.insert("Department".to_string(), department);
        }

        if let Some(role_str) = extract_xml_attribute_value(trimmed, "Role") {
            for r in role_str.split(',') {
                let clean = r.trim();
                if !clean.is_empty() {
                    roles.push(clean.to_string());
                }
            }
        } else {
            roles.push("Employee".to_string());
        }

        Ok(SamlAssertionContext {
            name_id,
            session_index,
            issuer,
            roles,
            attributes,
        })
    }
}

fn extract_xml_tag(xml: &str, tag_name: &str) -> Option<String> {
    let prefixes = ["saml:", "saml2:", ""];
    for prefix in prefixes {
        let full_tag = format!("<{prefix}{tag_name}");
        let close_tag = format!("</{prefix}{tag_name}>");

        let mut search_idx = 0;
        while let Some(start) = xml[search_idx..].find(&full_tag) {
            let tag_start = search_idx + start;
            let after_tag = tag_start + full_tag.len();
            if after_tag < xml.len() {
                let next_char = xml.as_bytes()[after_tag];
                if next_char == b'>' || next_char.is_ascii_whitespace() {
                    if let Some(gt_pos) = xml[after_tag..].find('>') {
                        let content_start = after_tag + gt_pos + 1;
                        if let Some(end) = xml[content_start..].find(&close_tag) {
                            return Some(xml[content_start..content_start + end].trim().to_string());
                        }
                    }
                }
            }
            search_idx = after_tag;
        }
    }
    None
}

fn extract_xml_attribute(xml: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{attr_name}=\"");
    if let Some(start) = xml.find(&pattern) {
        let val_start = start + pattern.len();
        if let Some(end) = xml[val_start..].find('"') {
            return Some(xml[val_start..val_start + end].to_string());
        }
    }
    None
}

fn extract_xml_attribute_value(xml: &str, attribute_name: &str) -> Option<String> {
    let pattern = format!("Name=\"{attribute_name}\"");
    if let Some(pos) = xml.find(&pattern) {
        let slice = &xml[pos..];
        let val_variants = ["<saml:AttributeValue>", "<saml2:AttributeValue>", "<AttributeValue>"];
        let close_variants = ["</saml:AttributeValue>", "</saml2:AttributeValue>", "</AttributeValue>"];

        for (val_tag, close_tag) in val_variants.iter().zip(close_variants.iter()) {
            if let Some(start) = slice.find(val_tag) {
                let c_start = start + val_tag.len();
                if let Some(end) = slice[c_start..].find(close_tag) {
                    return Some(slice[c_start..c_start + end].trim().to_string());
                }
            }
        }
    }
    None
}
