// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::types::{DisclosureTier, ProjectedTool, ToolDefinition};
use crate::discovery::catalog_index::CatalogSearchIndex;

/// Result of hybrid search combining lexical and semantic rankings via RRF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub query: String,
    pub tools: Vec<ProjectedTool>,
    pub total_candidates: usize,
    pub lexical_hits: usize,
    pub semantic_hits: usize,
}

/// Advanced Hybrid Retrieval Engine combining Lexical Inverted Index and Semantic Vector Search via Reciprocal Rank Fusion
#[derive(Clone, Default)]
pub struct HybridSearchEngine {
    catalog_index: CatalogSearchIndex,
    dense_embeddings: HashMap<String, Vec<f32>>,
}

impl HybridSearchEngine {
    pub const RRF_K: f64 = 60.0;

    pub fn new() -> Self {
        Self::default()
    }

    /// Index tools and generate dense semantic vectors for them
    pub fn index_tools(&mut self, server: &str, tools: Vec<ToolDefinition>) {
        for tool in &tools {
            let vec = Self::embed_tool(tool);
            self.dense_embeddings.insert(tool.name.clone(), vec);
        }
        self.catalog_index.index_tools_for_backend(server, tools);
    }

    /// Synthesizes a deterministic dense embedding vector from functional semantic concepts
    pub fn embed_tool(tool: &ToolDefinition) -> Vec<f32> {
        let text = format!(
            "{} {} {} {} {}",
            tool.name,
            tool.server,
            tool.description,
            tool.when_to_use,
            tool.tags.join(" ")
        );
        Self::text_to_dense_vector(&text)
    }

    /// Converts text into a normalized dense embedding vector across 32 semantic dimensions
    pub fn text_to_dense_vector(text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; 32];
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        for word in words {
            let hash = Self::fnv1a_hash(word.as_bytes());
            let dim1 = (hash % 32) as usize;
            let dim2 = ((hash >> 5) % 32) as usize;
            vec[dim1] += 1.0;
            vec[dim2] += 0.5;

            // Entity Database & Tabular schema (dim 0, 1)
            if word.contains("sql") || word.contains("db") || word.contains("data") || word.contains("tabel") || word.contains("table") || word.contains("ledger") {
                vec[0] += 1.5;
                vec[1] += 1.0;
            }
            // Action Read / Query (dim 4, 5)
            if word.contains("query") || word.contains("select") || word.contains("read") || word.contains("fetch") || word.contains("cari") || word.contains("ambil") {
                vec[4] += 2.5;
                vec[5] += 2.0;
            }
            // Entity File / Bucket / S3 Storage (dim 12, 13)
            if word.contains("file") || word.contains("berkas") || word.contains("bucket") || word.contains("s3") || word.contains("media") {
                vec[12] += 1.5;
                vec[13] += 1.0;
            }
            // Action Write / Upload / Store (dim 8, 9)
            if word.contains("upload") || word.contains("store") || word.contains("simpan") || word.contains("unggah") || word.contains("save") || word.contains("write") {
                vec[8] += 2.5;
                vec[9] += 2.0;
            }
            // Action Alert / Notify (dim 16, 17)
            if word.contains("alert") || word.contains("notify") || word.contains("slack") || word.contains("pesan") || word.contains("kirim") {
                vec[16] += 2.5;
                vec[17] += 2.0;
            }
            // Action Delete / Drop / Destroy / Remove (dim 24, 25)
            if word.contains("delete") || word.contains("drop") || word.contains("remove") || word.contains("hapus") || word.contains("destructive") || word.contains("destroy") {
                vec[24] += 3.0;
                vec[25] += 2.5;
            }
        }

        // L2 Normalization
        let norm_sq: f32 = vec.iter().map(|v| v * v).sum();
        let norm = norm_sq.sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }

        vec
    }

    fn fnv1a_hash(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Calculate Cosine Similarity between two dense normalized vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Perform hybrid retrieval combining lexical and semantic search via Reciprocal Rank Fusion (RRF)
    pub fn search(&self, query: &str, tier: DisclosureTier, top_k: usize) -> HybridSearchResult {
        // 1. Lexical retrieval ranking
        let lex_res = self.catalog_index.search(query, tier, 100);
        let mut lex_ranks: HashMap<String, usize> = HashMap::new();
        for (i, tool) in lex_res.tools.iter().enumerate() {
            lex_ranks.insert(tool.name.clone(), i + 1);
        }

        // 2. Semantic vector retrieval ranking
        let query_vec = Self::text_to_dense_vector(query);
        let mut sem_scored: Vec<(String, f32)> = self
            .dense_embeddings
            .iter()
            .map(|(name, emb)| {
                let sim = Self::cosine_similarity(&query_vec, emb);
                (name.clone(), sim)
            })
            .collect();

        sem_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut sem_ranks: HashMap<String, usize> = HashMap::new();
        for (i, (name, sim)) in sem_scored.iter().enumerate() {
            if *sim > 0.15 {
                sem_ranks.insert(name.clone(), i + 1);
            }
        }

        // 3. Reciprocal Rank Fusion (RRF)
        let mut rrf_scores: HashMap<String, f64> = HashMap::new();
        let mut all_tools: HashMap<String, &ProjectedTool> = HashMap::new();
        for t in &lex_res.tools {
            all_tools.insert(t.name.clone(), t);
        }

        // Merge candidates
        let mut all_candidate_names: Vec<String> = lex_ranks.keys().cloned().collect();
        for name in sem_ranks.keys() {
            if !all_candidate_names.contains(name) {
                all_candidate_names.push(name.clone());
            }
        }

        for name in &all_candidate_names {
            let mut score = 0.0;
            if let Some(r_lex) = lex_ranks.get(name) {
                score += 1.0 / (Self::RRF_K + *r_lex as f64);
            }
            if let Some(r_sem) = sem_ranks.get(name) {
                score += 1.2 / (Self::RRF_K + *r_sem as f64); // Boost semantic affinity
            }
            rrf_scores.insert(name.clone(), score);
        }

        let mut final_ranked: Vec<(&String, &f64)> = rrf_scores.iter().collect();
        final_ranked.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));

        let projected_tools: Vec<ProjectedTool> = final_ranked
            .into_iter()
            .take(top_k)
            .filter_map(|(name, score)| {
                if let Some(p) = all_tools.get(name) {
                    let mut cloned = (*p).clone();
                    cloned.score = *score;
                    Some(cloned)
                } else {
                    // Re-project if candidate originated from semantic channel only
                    let mut single_res = self.catalog_index.search(name, tier, 1);
                    if let Some(mut first) = single_res.tools.pop() {
                        first.score = *score;
                        Some(first)
                    } else {
                        None
                    }
                }
            })
            .collect();

        HybridSearchResult {
            query: query.to_string(),
            tools: projected_tools,
            total_candidates: all_candidate_names.len(),
            lexical_hits: lex_ranks.len(),
            semantic_hits: sem_ranks.len(),
        }
    }
}
