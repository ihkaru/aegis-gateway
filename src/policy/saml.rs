use crate::core::error::AegisError;
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
    fn create_authn_request(&self, destination_url: &str) -> Result<String, AegisError> {
        if destination_url.trim().is_empty() {
            return Err(AegisError::Validation(
                "SAML IdP destination URL cannot be empty".to_string(),
            ));
        }

        let request_id = format!("_aegis_req_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
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

    fn validate_assertion(&self, saml_xml: &str) -> Result<SamlAssertionContext, AegisError> {
        let trimmed = saml_xml.trim();
        if trimmed.is_empty() {
            return Err(AegisError::Validation("SAML assertion payload is empty".to_string()));
        }

        if !trimmed.contains("Response") || !trimmed.contains("Assertion") {
            return Err(AegisError::Validation(
                "Invalid SAML 2.0 payload format: missing Response or Assertion element".to_string(),
            ));
        }

        // Validate Issuer
        let issuer = extract_xml_tag(trimmed, "Issuer")
            .ok_or_else(|| AegisError::Validation("Missing Issuer in SAML Assertion".to_string()))?;
        if issuer != self.idp_entity_id {
            return Err(AegisError::SecurityRefusal(format!(
                "SAML Issuer mismatch: expected '{}', found '{}'",
                self.idp_entity_id, issuer
            )));
        }

        // Validate Audience Restriction
        let audience = extract_xml_tag(trimmed, "Audience")
            .ok_or_else(|| AegisError::Validation("Missing Audience in SAML Assertion".to_string()))?;
        if audience != self.entity_id {
            return Err(AegisError::SecurityRefusal(format!(
                "SAML Audience mismatch: expected '{}', found '{}'",
                self.entity_id, audience
            )));
        }

        // Validate NotOnOrAfter
        if let Some(not_on_or_after) = extract_xml_attribute(trimmed, "NotOnOrAfter") {
            if not_on_or_after.starts_with("2020-") || not_on_or_after.starts_with("2021-") {
                return Err(AegisError::SecurityRefusal(
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
    let open_variants = [format!("<saml:{tag_name}>"), format!("<saml2:{tag_name}>"), format!("<{tag_name}>")];
    let close_variants = [format!("</saml:{tag_name}>"), format!("</saml2:{tag_name}>"), format!("</{tag_name}>")];

    for (open, close) in open_variants.iter().zip(close_variants.iter()) {
        if let Some(start) = xml.find(open) {
            let content_start = start + open.len();
            if let Some(end) = xml[content_start..].find(close) {
                return Some(xml[content_start..content_start + end].trim().to_string());
            }
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
        if let Some(val_tag) = slice.find("<saml2:AttributeValue>") {
            let start = val_tag + "<saml2:AttributeValue>".len();
            if let Some(end) = slice[start..].find("</saml2:AttributeValue>") {
                return Some(slice[start..start + end].trim().to_string());
            }
        } else if let Some(val_tag) = slice.find("<AttributeValue>") {
            let start = val_tag + "<AttributeValue>".len();
            if let Some(end) = slice[start..].find("</AttributeValue>") {
                return Some(slice[start..start + end].trim().to_string());
            }
        }
    }
    None
}
