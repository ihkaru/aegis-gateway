// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::{AegisError, AegisResult};
use crate::core::skills::{PoisonScanner, SkillBundle, SkillMetadata, SkillRegistry};

/// In-memory skill registry with hot-reloading capability
#[derive(Clone, Default)]
pub struct LocalSkillRegistry {
    skills: Arc<RwLock<HashMap<String, SkillBundle>>>,
}

impl LocalSkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, bundle: SkillBundle) {
        let mut write = self.skills.write().await;
        write.insert(bundle.metadata.name.clone(), bundle);
    }
}

#[async_trait]
impl SkillRegistry for LocalSkillRegistry {
    async fn list_skills(&self) -> AegisResult<Vec<SkillMetadata>> {
        let read = self.skills.read().await;
        Ok(read.values().map(|s| s.metadata.clone()).collect())
    }

    async fn load_skill(&self, name: &str) -> AegisResult<SkillBundle> {
        let read = self.skills.read().await;
        read.get(name)
            .cloned()
            .ok_or_else(|| AegisError::SkillNotFound(name.to_string()))
    }

    async fn reload(&self) -> AegisResult<usize> {
        let read = self.skills.read().await;
        Ok(read.len())
    }
}

/// Anti-poisoning & prompt injection scanner
#[derive(Default)]
pub struct DefaultPoisonScanner;

impl DefaultPoisonScanner {
    pub fn new() -> Self {
        Self
    }
}

impl PoisonScanner for DefaultPoisonScanner {
    fn detect_injection(&self, text: &str) -> AegisResult<()> {
        let lower = text.to_lowercase();
        let triggers = [
            "<important>",
            "<system>",
            "ignore previous instructions",
            "/etc/passwd",
            "~/.ssh",
            "id_rsa",
        ];

        for trigger in triggers {
            if lower.contains(trigger) {
                return Err(AegisError::SecurityPoisoningDetected(format!(
                    "Found prohibited prompt injection pattern: '{trigger}'"
                )));
            }
        }

        Ok(())
    }
}
