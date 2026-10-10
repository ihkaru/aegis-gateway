use crate::core::data_governance::PolicyTier;
use crate::core::error::AegisError;
use crate::core::federation::{ScimEventType, ScimInboundReceiver, ScimProvisionResult, ScimUserRecord};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// Production SCIM 2.0 (RFC 7644) Inbound Lifecycle Provisioning Receiver.
pub struct NativeScimInboundReceiver {
    group_tier_mappings: RwLock<HashMap<String, PolicyTier>>,
    revoked_users: RwLock<HashSet<String>>,
}

impl Default for NativeScimInboundReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeScimInboundReceiver {
    pub fn new() -> Self {
        let mut default_mappings = HashMap::new();
        default_mappings.insert("Security-Admins".to_string(), PolicyTier::StrictAirgapped);
        default_mappings.insert("Finance-Audit".to_string(), PolicyTier::StrictAirgapped);
        default_mappings.insert("Data-Engineers".to_string(), PolicyTier::Hybrid);
        default_mappings.insert("Product-Support".to_string(), PolicyTier::Hybrid);
        default_mappings.insert("Developers".to_string(), PolicyTier::Developer);

        Self {
            group_tier_mappings: RwLock::new(default_mappings),
            revoked_users: RwLock::new(HashSet::new()),
        }
    }

    /// Register a custom corporate directory group to policy tier mapping.
    pub fn register_group_mapping(&self, group: impl Into<String>, tier: PolicyTier) {
        if let Ok(mut map) = self.group_tier_mappings.write() {
            map.insert(group.into(), tier);
        }
    }

    /// Check if a user has been deprovisioned and tombstoned by SCIM.
    pub fn is_user_revoked(&self, user_id: &str) -> bool {
        self.revoked_users
            .read()
            .map(|set| set.contains(user_id))
            .unwrap_or(false)
    }
}

impl ScimInboundReceiver for NativeScimInboundReceiver {
    fn process_user_event(
        &self,
        event_type: ScimEventType,
        record: ScimUserRecord,
    ) -> Result<ScimProvisionResult, AegisError> {
        if record.user_id.trim().is_empty() {
            return Err(AegisError::Validation("SCIM user_id cannot be empty".to_string()));
        }

        // Handle deprovisioning / inactive state
        if event_type == ScimEventType::DeprovisionUser || !record.active {
            let mut set = self
                .revoked_users
                .write()
                .map_err(|e| AegisError::Internal(format!("SCIM lock error: {e}")))?;
            set.insert(record.user_id.clone());

            return Ok(ScimProvisionResult {
                affected_id: record.user_id,
                active_status: false,
                mapped_tier: PolicyTier::StrictAirgapped,
                message: "User account deactivated via SCIM; all active sessions tombstoned".to_string(),
            });
        }

        // Determine effective policy tier based on directory groups
        let mut highest_tier = PolicyTier::Developer;
        for g in &record.groups {
            let tier = self.map_group_to_tier(g);
            if tier == PolicyTier::StrictAirgapped {
                highest_tier = PolicyTier::StrictAirgapped;
                break;
            } else if tier == PolicyTier::Hybrid && highest_tier != PolicyTier::StrictAirgapped {
                highest_tier = PolicyTier::Hybrid;
            }
        }

        let action_name = match event_type {
            ScimEventType::CreateUser => "Provisioned",
            ScimEventType::UpdateUser => "Updated",
            _ => "Synchronized",
        };

        Ok(ScimProvisionResult {
            affected_id: record.user_id,
            active_status: true,
            mapped_tier: highest_tier,
            message: format!("{action_name} user with effective tier '{highest_tier:?}'"),
        })
    }

    fn map_group_to_tier(&self, group_name: &str) -> PolicyTier {
        if let Ok(map) = self.group_tier_mappings.read() {
            if let Some(tier) = map.get(group_name) {
                return *tier;
            }
        }

        let lower = group_name.to_lowercase();
        if lower.contains("sec") || lower.contains("audit") || lower.contains("finance") || lower.contains("compliance") {
            PolicyTier::StrictAirgapped
        } else if lower.contains("prod") || lower.contains("ops") || lower.contains("support") {
            PolicyTier::Hybrid
        } else {
            PolicyTier::Developer
        }
    }
}
