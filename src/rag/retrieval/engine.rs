// PRD 11 Phase 2: Retrieval Engine for semantic search
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::memory::knowledge_manager::{KnowledgeCategory, KnowledgeManager};
use crate::memory::vector_db::QueryResult;

/// Search parameters for retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    /// Maximum number of results to retrieve
    pub top_k: usize,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub threshold: f64,
    /// Categories to search (None = all categories)
    pub categories: Option<Vec<KnowledgeCategory>>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            top_k: 10,
            threshold: 0.7,
            categories: None,
        }
    }
}

/// Retrieved document with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub id: String,
    pub content: String,
    pub category: String,
    pub score: f32,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl From<QueryResult> for RetrievedDocument {
    fn from(result: QueryResult) -> Self {
        Self {
            id: result.id,
            content: result.document,
            category: String::new(), // Will be set by caller
            score: result.score,
            metadata: result.metadata.into_iter().collect(),
        }
    }
}

/// Retrieval engine for semantic search
pub struct RetrievalEngine {
    knowledge_manager: Arc<KnowledgeManager>,
    default_params: SearchParams,
}

impl RetrievalEngine {
    /// Create new retrieval engine
    pub fn new(knowledge_manager: Arc<KnowledgeManager>) -> Self {
        Self {
            knowledge_manager,
            default_params: SearchParams::default(),
        }
    }

    /// Create with custom default parameters
    pub fn with_params(knowledge_manager: Arc<KnowledgeManager>, params: SearchParams) -> Self {
        Self {
            knowledge_manager,
            default_params: params,
        }
    }

    /// Retrieve documents matching query
    pub async fn retrieve(&self, query: &str) -> Result<Vec<RetrievedDocument>> {
        self.retrieve_with_params(query, &self.default_params).await
    }

    /// Retrieve with custom parameters
    pub async fn retrieve_with_params(
        &self,
        query: &str,
        params: &SearchParams,
    ) -> Result<Vec<RetrievedDocument>> {
        let results = if let Some(categories) = &params.categories {
            // Search specific categories
            let mut all_results = Vec::new();
            for category in categories {
                let category_results = self
                    .knowledge_manager
                    .search(query, *category, params.top_k, params.threshold)
                    .await
                    .context(format!("Failed to search category: {:?}", category))?;

                for result in category_results {
                    let mut doc = RetrievedDocument::from(result);
                    doc.category = format!("{:?}", category);
                    all_results.push(doc);
                }
            }

            // Sort by score and take top_k
            all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            all_results.truncate(params.top_k);
            all_results
        } else {
            // Search all categories
            let results = self
                .knowledge_manager
                .search_all(query, params.top_k, params.threshold)
                .await
                .context("Failed to search all categories")?;

            results
                .into_iter()
                .map(|r| {
                    let mut doc = RetrievedDocument::from(r);
                    doc.category = "mixed".to_string();
                    doc
                })
                .collect()
        };

        Ok(results)
    }

    /// Retrieve from specific category
    pub async fn retrieve_from_category(
        &self,
        query: &str,
        category: KnowledgeCategory,
        top_k: usize,
        threshold: f64,
    ) -> Result<Vec<RetrievedDocument>> {
        let results = self
            .knowledge_manager
            .search(query, category, top_k, threshold)
            .await
            .context(format!("Failed to search category: {:?}", category))?;

        let documents = results
            .into_iter()
            .map(|r| {
                let mut doc = RetrievedDocument::from(r);
                doc.category = format!("{:?}", category);
                doc
            })
            .collect();

        Ok(documents)
    }

    /// Get default search parameters
    pub fn default_params(&self) -> &SearchParams {
        &self.default_params
    }

    /// Update default search parameters
    pub fn set_default_params(&mut self, params: SearchParams) {
        self.default_params = params;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_default() {
        let params = SearchParams::default();
        assert_eq!(params.top_k, 10);
        assert_eq!(params.threshold, 0.7);
        assert!(params.categories.is_none());
    }

    #[test]
    fn test_search_params_custom() {
        let params = SearchParams {
            top_k: 5,
            threshold: 0.8,
            categories: Some(vec![KnowledgeCategory::Code]),
        };
        assert_eq!(params.top_k, 5);
        assert_eq!(params.threshold, 0.8);
        assert!(params.categories.is_some());
    }

    #[test]
    fn test_retrieved_document_from_query_result() {
        let result = QueryResult {
            id: "test1".to_string(),
            score: 0.95,
            document: "Test content".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let doc = RetrievedDocument::from(result);
        assert_eq!(doc.id, "test1");
        assert_eq!(doc.content, "Test content");
        assert_eq!(doc.score, 0.95);
    }
}
