// SPDX-License-Identifier: MIT

pub mod approval;
pub mod audit;
pub mod backend;
pub mod data_governance;
pub mod delegation;
pub mod dlp;
pub mod error;
pub mod health;
pub mod identity;
pub mod k8s;
pub mod notification;
pub mod oauth_connect;
pub mod policy;
pub mod proxy;
pub mod sandbox;
pub mod secrets;
pub mod session;
pub mod skills;
pub mod state;
pub mod transport;
pub mod trust;
pub mod types;
pub mod vault;

pub use approval::{ApprovalDecision, ApprovalGate, ApprovalTicket, RiskTier};
pub use data_governance::{
    DataClassification, DataEgressDecision, DataEgressPolicyEngine, InSituDataEnclave,
    InSituExecutionResult, PolicyTier,
};
pub use delegation::{DelegatedToken, IdentityDelegationBroker, ResourceScoper};
pub use notification::{
    ApprovalChannelTarget, ApprovalNotificationDispatcher, ApprovalNotificationPayload,
    DurableResumeRouter,
};
pub use oauth_connect::{AuthProviderConfig, AuthRequiredResponse, ConnectSession, OAuthConnectEngine};
pub use proxy::{CredentialProxyEngine, ProxyBinding};

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
pub use vault::{
    BinaryBlobMetadata, BinaryStreamingVault, ChunkAck, EphemeralEgressTicket,
    MimeInspectionResult, StoredBlobDescriptor, UploadSession,
};
pub use k8s::{
    AdmissionDecision, AegisBackendCrd, AegisCrdReconciler, AegisPolicyCrd, AegisTenantCrd,
    ReconcileOutcome,
};
pub use trust::{
    AgentCertEnvelope, AgentTrustLevel, AgentTrustVerifier, AgentVerificationOutcome,
    CorsEvaluationResult, CorsPolicyEnforcer, ThreatEvidenceEnricher, ThreatEvidenceReport,
};

