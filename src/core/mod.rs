// SPDX-License-Identifier: MIT

pub mod audit;
pub mod dlp;
pub mod error;
pub mod health;
pub mod identity;
pub mod policy;
pub mod secrets;
pub mod session;
pub mod skills;
pub mod state;
pub mod types;

pub use audit::{AuditAction, AuditChainVerifier, AuditEvent, AuditSink, HashChainedEvent, OtelTraceContext};
pub use dlp::{ComplianceProfile, DlpFinding, DlpPipeline, FastPatternMatcher, MaskingStrategy, SensitivityLevel};
pub use error::{AegisError, AegisResult};
pub use health::{HealthProbe, HealthReport, ProbeState};
pub use identity::TokenValidator;

pub use policy::{PolicyContext, PolicyDecision, PolicyEngine};
pub use secrets::SecretStore;
pub use session::SessionRevocationRegistry;
pub use skills::{
    GitOpsSyncReport, PoisonScanner, SignedSkillBundle, SkillBundle, SkillDependency,
    SkillMetadata, SkillRegistry, SkillVectorRetriever,
};
pub use state::{
    BudgetManager, BudgetStatus, DepartmentChargeback, DistributedCache, DistributedCircuitBreaker,
    DistributedRateLimiter, DistributedState, QuotaEngine, TenantBudget,
};
pub use types::{
    CallerContext, DisclosureTier, ProjectedTool, ProgressiveDisclosure, TenantId, TokenSavings,
    ToolCallRequest, ToolCallResponse, ToolDefinition,
};

