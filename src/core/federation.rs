use crate::core::data_governance::PolicyTier;
use crate::core::error::AegisError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed security context extracted from a verified SAML 2.0 assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SamlAssertionContext {
    pub name_id: String,
    pub session_index: String,
    pub issuer: String,
    pub roles: Vec<String>,
    pub attributes: HashMap<String, String>,
}

/// Interface contract for SAML 2.0 Service Provider (SP) operations.
pub trait SamlServiceProvider: Send + Sync {
    /// Generate an XML AuthnRequest for redirecting a user to the enterprise IdP.
    fn create_authn_request(&self, destination_url: &str) -> Result<String, AegisError>;

    /// Parse and cryptographically validate an incoming SAML Response XML assertion.
    fn validate_assertion(&self, saml_xml: &str) -> Result<SamlAssertionContext, AegisError>;
}

/// Event types supported by the SCIM 2.0 (RFC 7644) inbound receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScimEventType {
    CreateUser,
    UpdateUser,
    DeprovisionUser,
    SyncGroup,
}

/// Inbound SCIM 2.0 user representation from corporate directory webhooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScimUserRecord {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub active: bool,
    pub groups: Vec<String>,
}

/// Result of an applied SCIM lifecycle provisioning or deprovisioning action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScimProvisionResult {
    pub affected_id: String,
    pub active_status: bool,
    pub mapped_tier: PolicyTier,
    pub message: String,
}

/// Interface contract for SCIM 2.0 (RFC 7644) automated directory synchronization.
pub trait ScimInboundReceiver: Send + Sync {
    /// Process an inbound SCIM user event (provision, update, or deprovision).
    fn process_user_event(
        &self,
        event_type: ScimEventType,
        record: ScimUserRecord,
    ) -> Result<ScimProvisionResult, AegisError>;

    /// Determine the effective policy tier assigned to an enterprise group.
    fn map_group_to_tier(&self, group_name: &str) -> PolicyTier;
}
