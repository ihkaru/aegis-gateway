// SPDX-License-Identifier: MIT

pub mod audit;
pub mod backend;
pub mod dlp;
pub mod error;
pub mod health;
pub mod identity;
pub mod policy;
pub mod sandbox;
pub mod secrets;
pub mod session;
pub mod skills;
pub mod state;
pub mod transport;
pub mod types;

pub use audit::{AuditAction, AuditChainVerifier, AuditEvent, AuditSink, HashChainedEvent, OtelTraceContext};
pub use backend::{AegisTopologyConfig, BackendConfig, BackendRegistry, BackendTransport};
pub use dlp::{ComplianceProfile, DlpFinding, DlpPipeline, FastPatternMatcher, MaskingStrategy, SensitivityLevel};
pub use error::{AegisError, AegisResult};
pub use health::{HealthProbe, HealthReport, ProbeState};
pub use identity::TokenValidator;

pub use policy::{PolicyContext, PolicyDecision, PolicyEngine};
pub use sandbox::{
    CodeSandboxEngine, CredentialBroker, EgressFirewall, ExecutionLanguage,
    SandboxExecutionRequest, SandboxExecutionResult,
};
pub use secrets::SecretStore;
pub use session::{SessionFence, SessionLifecycleStatus, SessionRevocationRegistry};
pub use skills::{
    GitOpsSyncReport, PoisonScanner, SignedSkillBundle, SkillBundle, SkillDependency,
    SkillMetadata, SkillRegistry, SkillVectorRetriever,
};
pub use state::{
    BudgetManager, BudgetStatus, DepartmentChargeback, DistributedCache, DistributedCircuitBreaker,
    DistributedRateLimiter, DistributedState, QuotaEngine, TenantBudget,
};
pub use transport::{
    IngressTransport, JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, WireProtocolHandler,
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};
pub use types::{
    CallerContext, DisclosureTier, ProjectedTool, ProgressiveDisclosure, TenantId, TokenSavings,
    ToolCallRequest, ToolCallResponse, ToolDefinition,
};

