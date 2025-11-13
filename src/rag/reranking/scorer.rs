// PRD 11 Phase 2: Re-ranking scorer for retrieved documents
use serde::{Deserialize, Serialize};

use crate::rag::retrieval::RetrievedDocument;

/// Re-ranking strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RankingStrategy {
    /// Use original similarity scores only
    Similarity,
    /// Boost recent documents
    Recency,
    /// Combined similarity + recency
    Hybrid,
}

/// Re-ranking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReRankConfig {
    /// Ranking strategy to use
    pub strategy: RankingStrategy,
    /// Weight for recency (0.0 to 1.0) in hybrid mode
    pub recency_weight: f32,
    /// Boost factor for exact keyword matches
    pub keyword_boost: f32,
}

impl Default for ReRankConfig {
    fn default() -> Self {
        Self {
            strategy: RankingStrategy::Hybrid,
            recency_weight: 0.3,
            keyword_boost: 0.1,
        }
    }
}

/// Document with re-ranked score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedDocument {
    pub document: RetrievedDocument,
    pub original_score: f32,
    pub reranked_score: f32,
    pub boost_applied: f32,
}

/// Re-ranker for improving retrieval results
pub struct ReRanker {
    config: ReRankConfig,
}

impl ReRanker {
    /// Create new re-ranker with default config
    pub fn new() -> Self {
        Self {
            config: ReRankConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ReRankConfig) -> Self {
        Self { config }
    }

    /// Re-rank documents based on strategy
    pub fn rerank(&self, documents: Vec<RetrievedDocument>, query: &str) -> Vec<RankedDocument> {
        let mut ranked: Vec<RankedDocument> = documents
            .into_iter()
            .map(|doc| {
                let original_score = doc.score;
                let reranked_score = self.compute_score(&doc, query);
                let boost_applied = reranked_score - original_score;

                RankedDocument {
                    document: doc,
                    original_score,
                    reranked_score,
                    boost_applied,
                }
            })
            .collect();

        // Sort by reranked score descending
        ranked.sort_by(|a, b| {
            b.reranked_score
                .partial_cmp(&a.reranked_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ranked
    }

    /// Compute reranked score for a document
    fn compute_score(&self, doc: &RetrievedDocument, query: &str) -> f32 {
        let base_score = doc.score;

        match self.config.strategy {
            RankingStrategy::Similarity => base_score,
            RankingStrategy::Recency => {
                let recency_score = self.compute_recency_score(doc);
                recency_score
            }
            RankingStrategy::Hybrid => {
                let recency_score = self.compute_recency_score(doc);
                let keyword_boost = self.compute_keyword_boost(doc, query);

                // Combine scores
                let similarity_weight = 1.0 - self.config.recency_weight;
                let combined = (base_score * similarity_weight)
                    + (recency_score * self.config.recency_weight)
                    + keyword_boost;

                combined.min(1.0) // Cap at 1.0
            }
        }
    }

    /// Compute recency score from metadata timestamp
    fn compute_recency_score(&self, doc: &RetrievedDocument) -> f32 {
        // Try to get timestamp from metadata
        if let Some(timestamp_value) = doc.metadata.get("timestamp") {
            if let Some(timestamp) = timestamp_value.as_i64() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                let age_seconds = now - timestamp;
                let age_days = age_seconds as f32 / 86400.0;

                // Exponential decay: score = e^(-age_days/30)
                // Recent docs (0 days) = 1.0, 30 days old = 0.37, 90 days = 0.05
                let recency = (-age_days / 30.0).exp();
                return recency;
            }
        }

        // No timestamp or invalid - assume recent
        0.5
    }

    /// Compute keyword boost for exact matches
    fn compute_keyword_boost(&self, doc: &RetrievedDocument, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = doc.content.to_lowercase();

        // Count exact keyword matches
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matches = query_words
            .iter()
            .filter(|word| word.len() > 3 && content_lower.contains(*word))
            .count();

        if matches > 0 {
            let boost_per_match = self.config.keyword_boost / query_words.len() as f32;
            (matches as f32 * boost_per_match).min(self.config.keyword_boost)
        } else {
            0.0
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ReRankConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ReRankConfig) {
        self.config = config;
    }
}

impl Default for ReRanker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_doc(id: &str, content: &str, score: f32) -> RetrievedDocument {
        RetrievedDocument {
            id: id.to_string(),
            content: content.to_string(),
            category: "test".to_string(),
            score,
            metadata: serde_json::Map::new(),
        }
    }

    #[test]
    fn test_reranker_creation() {
        let ranker = ReRanker::new();
        assert_eq!(ranker.config.strategy, RankingStrategy::Hybrid);
        assert_eq!(ranker.config.recency_weight, 0.3);
    }

    #[test]
    fn test_rerank_similarity_strategy() {
        let config = ReRankConfig {
            strategy: RankingStrategy::Similarity,
            recency_weight: 0.0,
            keyword_boost: 0.0,
        };
        let ranker = ReRanker::with_config(config);

        let docs = vec![
            create_test_doc("1", "content", 0.9),
            create_test_doc("2", "content", 0.8),
        ];

        let ranked = ranker.rerank(docs, "query");

        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].document.id, "1");
        assert_eq!(ranked[0].reranked_score, 0.9);
    }

    #[test]
    fn test_keyword_boost() {
        let ranker = ReRanker::new();
        let doc = create_test_doc("1", "rust programming language", 0.8);

        let boost = ranker.compute_keyword_boost(&doc, "rust programming");
        assert!(boost > 0.0);
    }

    #[test]
    fn test_rerank_sorts_by_score() {
        let ranker = ReRanker::with_config(ReRankConfig {
            strategy: RankingStrategy::Similarity,
            recency_weight: 0.0,
            keyword_boost: 0.0,
        });

        let docs = vec![
            create_test_doc("1", "content", 0.6),
            create_test_doc("2", "content", 0.9),
            create_test_doc("3", "content", 0.7),
        ];

        let ranked = ranker.rerank(docs, "query");

        assert_eq!(ranked[0].document.id, "2"); // Highest score
        assert_eq!(ranked[1].document.id, "3");
        assert_eq!(ranked[2].document.id, "1"); // Lowest score
    }
}
