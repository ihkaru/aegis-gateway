// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::sync::Arc;

use crate::core::abac::{
    AbacContext, AbacEffect, AbacPolicyEngine, ActionAttributes, EnvironmentAttributes,
    PrincipalAttributes, ResourceAttributes,
};
use crate::core::error::PropylonResult;
use crate::core::scoping::CatalogScopingProvider;
use crate::core::types::ToolDefinition;

/// Dynamic Tool Catalog Scoper implementing Least-Privilege Exposure
pub struct AbacCatalogScoper {
    policy_engine: Arc<dyn AbacPolicyEngine>,
}

impl AbacCatalogScoper {
    pub fn new(policy_engine: Arc<dyn AbacPolicyEngine>) -> Self {
        Self { policy_engine }
    }
}

#[async_trait]
impl CatalogScopingProvider for AbacCatalogScoper {
    async fn scope_catalog(
        &self,
        principal: &PrincipalAttributes,
        raw_tools: &[ToolDefinition],
    ) -> PropylonResult<Vec<ToolDefinition>> {
        let mut allowed = Vec::with_capacity(raw_tools.len());

        for tool in raw_tools {
            let ctx = AbacContext {
                principal: principal.clone(),
                action: ActionAttributes {
                    tool_name: tool.name.clone(),
                    operation_type: "read".to_string(),
                },
                resource: ResourceAttributes {
                    backend_name: "gateway".to_string(),
                    data_classification: "internal".to_string(),
                },
                environment: EnvironmentAttributes::default(),
            };

            let decision = self.policy_engine.evaluate(&ctx).await?;

            // Denied tools are pruned from discovery catalog to prevent LLM context bloat
            if decision.effect != AbacEffect::Deny {
                allowed.push(tool.clone());
            }
        }

        Ok(allowed)
    }
}
