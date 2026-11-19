//! In-process cosine similarity vector ranking engine for semantic Git commit history search.

use serde::{Deserialize, Serialize};

/// Ranked semantic search result matched against cached commit embeddings.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchHit {
    /// Full 40-character Git commit SHA.
    pub commit_oid: String,
    /// Indexed commit summary message.
    pub summary: String,
    /// Normalized cosine similarity score (`0.0..=1.0`).
    pub similarity_score: f32,
}

/// Zero-allocation cosine similarity computation & top-K nearest-neighbor ranker.
pub struct VectorSearchEngine;

impl VectorSearchEngine {
    /// Computes cosine similarity between `a` and `b` without heap allocations.
    #[inline]
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let mut dot = 0.0_f32;
        let mut norm_a = 0.0_f32;
        let mut norm_b = 0.0_f32;

        for i in 0..a.len() {
            let av = a[i];
            let bv = b[i];
            dot += av * bv;
            norm_a += av * av;
            norm_b += bv * bv;
        }

        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom < f32::EPSILON {
            0.0
        } else {
            dot / denom
        }
    }

    /// Ranks cached commit embedding tuples `(oid, summary, vector)` against `query_vec`
    /// and returns up to `top_k` hits with similarity `>= min_threshold`.
    pub fn rank_matches(
        query_vec: &[f32],
        cached_vectors: &[(String, String, Vec<f32>)],
        top_k: usize,
        min_threshold: f32,
    ) -> Vec<SearchHit> {
        let mut hits = Vec::with_capacity(top_k.min(cached_vectors.len()).max(16));

        for (oid, summary, vec) in cached_vectors {
            let score = Self::cosine_similarity(query_vec, vec);
            if score >= min_threshold {
                hits.push(SearchHit {
                    commit_oid: oid.clone(),
                    summary: summary.clone(),
                    similarity_score: score,
                });
            }
        }

        hits.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(top_k);
        hits
    }
}
