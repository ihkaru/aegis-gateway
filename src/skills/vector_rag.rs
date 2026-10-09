// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::error::AegisResult;
use crate::core::skills::{SkillMetadata, SkillVectorRetriever};

/// Semantic Vector / BM25 term retriever for on-demand skill discovery
#[derive(Clone, Default)]
pub struct SemanticSkillRetriever {
    indexed_skills: Arc<RwLock<Vec<SkillMetadata>>>,
}

impl SemanticSkillRetriever {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn index_skill(&self, meta: SkillMetadata) {
        let mut write = self.indexed_skills.write().await;
        write.retain(|s| s.name != meta.name);
        write.push(meta);
    }

    pub async fn index_all(&self, metas: Vec<SkillMetadata>) {
        let mut write = self.indexed_skills.write().await;
        for meta in metas {
            write.retain(|s| s.name != meta.name);
            write.push(meta);
        }
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    fn term_vector(tokens: &[String]) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        for t in tokens {
            *map.entry(t.clone()).or_insert(0.0) += 1.0;
        }
        let norm: f64 = map.values().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 0.0 {
            for v in map.values_mut() {
                *v /= norm;
            }
        }
        map
    }

    fn cosine_similarity(v1: &HashMap<String, f64>, v2: &HashMap<String, f64>) -> f64 {
        let mut dot = 0.0;
        for (k, val1) in v1 {
            if let Some(val2) = v2.get(k) {
                dot += val1 * val2;
            }
        }
        dot
    }
}

#[async_trait]
impl SkillVectorRetriever for SemanticSkillRetriever {
    async fn search_skills(&self, query_prompt: &str, top_k: usize) -> AegisResult<Vec<SkillMetadata>> {
        let read = self.indexed_skills.read().await;
        let q_tokens = Self::tokenize(query_prompt);
        let q_vec = Self::term_vector(&q_tokens);

        let mut scored: Vec<(f64, SkillMetadata)> = read
            .iter()
            .map(|meta| {
                let mut text = format!("{} {} {}", meta.name, meta.description, meta.category);
                for tag in &meta.tags {
                    text.push(' ');
                    text.push_str(tag);
                }

                let d_tokens = Self::tokenize(&text);
                let d_vec = Self::term_vector(&d_tokens);
                let sim = Self::cosine_similarity(&q_vec, &d_vec);

                (sim, meta.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SkillMetadata> = scored
            .into_iter()
            .filter(|(score, _)| *score > 0.05)
            .take(top_k)
            .map(|(_, meta)| meta)
            .collect();

        Ok(results)
    }
}
