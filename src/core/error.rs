// SPDX-License-Identifier: MIT

use thiserror::Error;

pub type AegisResult<T> = Result<T, AegisError>;

#[derive(Error, Debug)]
pub enum AegisError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Policy violation [ABAC]: {0}")]
    PolicyDenied(String),

    #[error("Rate limit exceeded for tenant '{tenant}': {message}")]
    RateLimitExceeded { tenant: String, message: String },

    #[error("Circuit breaker is OPEN for backend '{0}'")]
    CircuitOpen(String),

    #[error("Data loss prevention (DLP) violation: {0}")]
    DlpViolation(String),

    #[error("Tool '{0}' not found in registry")]
    ToolNotFound(String),

    #[error("Skill '{0}' not found in registry")]
    SkillNotFound(String),

    #[error("Prompt poisoning or injection detected: {0}")]
    SecurityPoisoningDetected(String),

    #[error("Storage/State error: {0}")]
    StateError(String),

    #[error("Audit sink delivery error: {0}")]
    AuditError(String),

    #[error("Secret management error: {0}")]
    SecretError(String),

    #[error("Session is revoked or deactivated: {0}")]
    SessionRevoked(String),

    #[error("Drain timeout during shutdown: {0}")]
    DrainTimeout(String),

    #[error("Budget exceeded for tenant '{tenant}': spent ${current_usd:.2} of limit ${limit_usd:.2}")]
    BudgetExceeded {
        tenant: String,
        current_usd: f64,
        limit_usd: f64,
    },

    #[error("Tenant '{tenant}' is frozen due to budget cutoff: {reason}")]
    BudgetFrozen {
        tenant: String,
        reason: String,
    },

    #[error("Skill verification failed: {0}")]
    SkillVerificationFailed(String),

    #[error("Skill '{skill}' is missing required prerequisite: {missing}")]
    SkillDependencyMissing {
        skill: String,
        missing: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal gateway error: {0}")]
    Internal(String),
}

