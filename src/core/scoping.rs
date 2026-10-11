// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use crate::core::abac::PrincipalAttributes;
use crate::core::error::AegisResult;
use crate::core::types::ToolDefinition;

/// Contract for scoping tool discovery responses to least privilege
#[async_trait]
pub trait CatalogScopingProvider: Send + Sync {
    /// Filters tools dynamically according to caller clearance and department
    async fn scope_catalog(
        &self,
        principal: &PrincipalAttributes,
        raw_tools: &[ToolDefinition],
    ) -> AegisResult<Vec<ToolDefinition>>;
}
