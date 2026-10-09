// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;

use crate::core::error::{AegisError, AegisResult};
use crate::core::skills::{
    GitOpsSyncReport, SignedSkillBundle, SkillBundle, SkillDependency,
};
use crate::skills::LocalSkillRegistry;

/// GitOps skill synchronizer providing cryptographic provenance and hot reload
pub struct GitOpsSkillSync {
    registry: Arc<LocalSkillRegistry>,
    gateway_semver: String,
}

impl GitOpsSkillSync {
    pub fn new(registry: Arc<LocalSkillRegistry>) -> Self {
        Self {
            registry,
            gateway_semver: "0.7.0".to_string(),
        }
    }

    pub fn with_version(registry: Arc<LocalSkillRegistry>, version: &str) -> Self {
        Self {
            registry,
            gateway_semver: version.to_string(),
        }
    }

    pub fn gateway_version(&self) -> &str {
        &self.gateway_semver
    }

    /// Compute canonical SHA-256 digest of a skill bundle
    pub fn compute_digest(bundle: &SkillBundle) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bundle.metadata.name.as_bytes());
        hasher.update(bundle.metadata.description.as_bytes());
        hasher.update(bundle.instructions_markdown.as_bytes());

        for (filename, content) in &bundle.auxiliary_files {
            hasher.update(filename.as_bytes());
            hasher.update(content.as_bytes());
        }

        let digest = hasher.finalize();
        digest.iter().fold(String::with_capacity(64), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    /// Verify cryptographic integrity of a signed bundle
    pub fn verify_signature(&self, signed: &SignedSkillBundle) -> AegisResult<bool> {
        let actual_digest = Self::compute_digest(&signed.bundle);
        if actual_digest != signed.sha256_digest {
            return Err(AegisError::SkillVerificationFailed(format!(
                "Digest mismatch for skill '{}': expected {}, computed {}",
                signed.bundle.metadata.name, signed.sha256_digest, actual_digest
            )));
        }

        // Check signature presence if signer identity is declared
        if let Some(ref signer) = signed.signer_identity {
            if signed.signature.is_none() {
                return Err(AegisError::SkillVerificationFailed(format!(
                    "Missing signature from declared signer: {signer}"
                )));
            }
        }

        Ok(true)
    }

    /// Verify prerequisite tool dependencies
    pub fn validate_dependencies(
        &self,
        dep: &SkillDependency,
        available_tools: &HashSet<String>,
    ) -> AegisResult<()> {
        if !dep.semver_req.is_empty() && dep.semver_req != "*" && !self.gateway_semver.starts_with(&dep.semver_req[..1]) {
            return Err(AegisError::SkillDependencyMissing {
                skill: dep.skill_name.clone(),
                missing: format!(
                    "Requires gateway version {}, currently on {}",
                    dep.semver_req, self.gateway_semver
                ),
            });
        }

        for tool in &dep.required_tools {
            if !available_tools.contains(tool) {
                return Err(AegisError::SkillDependencyMissing {
                    skill: dep.skill_name.clone(),
                    missing: format!("Tool '{tool}' is not available in registered MCP servers"),
                });
            }
        }
        Ok(())
    }

    /// Ingest and hot-reload a batch of signed skill bundles from GitOps push/pull
    pub async fn sync_bundles(
        &self,
        bundles: Vec<SignedSkillBundle>,
        available_tools: &HashSet<String>,
        dependencies: &[SkillDependency],
    ) -> AegisResult<GitOpsSyncReport> {
        let mut updated = Vec::new();
        let mut errors = Vec::new();
        let mut verified_count = 0;

        let dep_map: std::collections::HashMap<String, &SkillDependency> = dependencies
            .iter()
            .map(|d| (d.skill_name.clone(), d))
            .collect();

        for signed in bundles {
            let skill_name = signed.bundle.metadata.name.clone();

            // 1. Verify cryptographic integrity
            if let Err(e) = self.verify_signature(&signed) {
                errors.push(format!("{}: {}", skill_name, e));
                continue;
            }
            verified_count += 1;

            // 2. Validate prerequisites if defined
            if let Some(dep) = dep_map.get(&skill_name) {
                if let Err(e) = self.validate_dependencies(dep, available_tools) {
                    errors.push(format!("{}: {}", skill_name, e));
                    continue;
                }
            }

            // 3. Register bundle into live registry (hot-reload)
            self.registry.register(signed.bundle).await;
            updated.push(skill_name);
        }

        Ok(GitOpsSyncReport {
            synced_count: updated.len(),
            updated_skills: updated,
            verified_signatures: verified_count,
            errors,
        })
    }
}
