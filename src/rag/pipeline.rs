// PRD 11 Phase 2: End-to-end RAG Pipeline
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::memory::knowledge_manager::{KnowledgeCategory, KnowledgeManager};
use crate::rag::context::{AssembledContext, ContextBuilder, ContextConfig};
use crate::rag::reranking::{ReRanker, ReRankConfig};
use crate::rag::retrieval::{RetrievalEngine, SearchParams};

/// RAG pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGConfig {
    /// Search parameters for retrieval
    pub search: SearchParams,
    /// Re-ranking configuration
    pub rerank: ReRankConfig,
    /// Context assembly configuration
    pub context: ContextConfig,
    /// Enable/disable RAG pipeline
    pub enabled: bool,
}

impl Default for RAGConfig {
    fn default() -> Self {
        Self {
            search: SearchParams::default(),
            rerank: ReRankConfig::default(),
            context: ContextConfig::default(),
            enabled: true,
        }
    }
}

/// RAG pipeline result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGResult {
    /// Original query
    pub query: String,
    /// Augmented prompt with context
    pub augmented_prompt: String,
    /// Assembled context
    pub context: AssembledContext,
    /// Number of documents retrieved
    pub documents_retrieved: usize,
    /// Number of documents after re-ranking
    pub documents_reranked: usize,
}

/// End-to-end RAG pipeline
pub struct RAGPipeline {
    retrieval_engine: RetrievalEngine,
    reranker: ReRanker,
    context_builder: ContextBuilder,
    config: RAGConfig,
}

impl RAGPipeline {
    /// Create new RAG pipeline
    pub fn new(knowledge_manager: Arc<KnowledgeManager>) -> Self {
        let config = RAGConfig::default();
        
        Self {
            retrieval_engine: RetrievalEngine::with_params(
                knowledge_manager,
                config.search.clone(),
            ),
            reranker: ReRanker::with_config(config.rerank.clone()),
            context_builder: ContextBuilder::with_config(config.context.clone()),
            config,
        }
    }

    /// Create with custom configuration
    pub fn with_config(knowledge_manager: Arc<KnowledgeManager>, config: RAGConfig) -> Self {
        Self {
            retrieval_engine: RetrievalEngine::with_params(
                knowledge_manager,
                config.search.clone(),
            ),
            reranker: ReRanker::with_config(config.rerank.clone()),
            context_builder: ContextBuilder::with_config(config.context.clone()),
            config,
        }
    }

    /// Execute RAG pipeline: retrieve -> rerank -> build context
    pub async fn execute(&self, query: &str) -> Result<RAGResult> {
        if !self.config.enabled {
            // RAG disabled - return empty result
            return Ok(RAGResult {
                query: query.to_string(),
                augmented_prompt: query.to_string(),
                context: AssembledContext {
                    text: String::new(),
                    document_count: 0,
                    estimated_tokens: 0,
                    document_ids: Vec::new(),
                },
                documents_retrieved: 0,
                documents_reranked: 0,
            });
        }

        // Step 1: Retrieve relevant documents
        let retrieved_docs = self
            .retrieval_engine
            .retrieve(query)
            .await
            .context("Failed to retrieve documents")?;

        let documents_retrieved = retrieved_docs.len();

        // Step 2: Re-rank documents
        let ranked_docs = self.reranker.rerank(retrieved_docs, query);
        let documents_reranked = ranked_docs.len();

        // Step 3: Build context from ranked documents
        let context = self.context_builder.build(&ranked_docs);

        // Step 4: Augment prompt with context
        let augmented_prompt = self.context_builder.augment_prompt(query, &ranked_docs);

        Ok(RAGResult {
            query: query.to_string(),
            augmented_prompt,
            context,
            documents_retrieved,
            documents_reranked,
        })
    }

    /// Execute with custom search parameters
    pub async fn execute_with_params(
        &self,
        query: &str,
        params: &SearchParams,
    ) -> Result<RAGResult> {
        if !self.config.enabled {
            return Ok(RAGResult {
                query: query.to_string(),
                augmented_prompt: query.to_string(),
                context: AssembledContext {
                    text: String::new(),
                    document_count: 0,
                    estimated_tokens: 0,
                    document_ids: Vec::new(),
                },
                documents_retrieved: 0,
                documents_reranked: 0,
            });
        }

        // Retrieve with custom params
        let retrieved_docs = self
            .retrieval_engine
            .retrieve_with_params(query, params)
            .await
            .context("Failed to retrieve documents")?;

        let documents_retrieved = retrieved_docs.len();

        // Re-rank and build context
        let ranked_docs = self.reranker.rerank(retrieved_docs, query);
        let documents_reranked = ranked_docs.len();
        let context = self.context_builder.build(&ranked_docs);
        let augmented_prompt = self.context_builder.augment_prompt(query, &ranked_docs);

        Ok(RAGResult {
            query: query.to_string(),
            augmented_prompt,
            context,
            documents_retrieved,
            documents_reranked,
        })
    }

    /// Execute and return only the augmented prompt
    pub async fn augment(&self, query: &str) -> Result<String> {
        let result = self.execute(query).await?;
        Ok(result.augmented_prompt)
    }

    /// Check if RAG is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable RAG pipeline
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Get current configuration
    pub fn config(&self) -> &RAGConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: RAGConfig) {
        self.config = config.clone();
        self.retrieval_engine.set_default_params(config.search);
        self.reranker.set_config(config.rerank);
        self.context_builder.set_config(config.context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_config_default() {
        let config = RAGConfig::default();
        assert!(config.enabled);
        assert_eq!(config.search.top_k, 10);
        assert_eq!(config.context.max_context_tokens, 2000);
    }

    #[test]
    fn test_rag_config_custom() {
        let config = RAGConfig {
            enabled: false,
            search: SearchParams {
                top_k: 5,
                threshold: 0.8,
                categories: None,
            },
            ..Default::default()
        };
        assert!(!config.enabled);
        assert_eq!(config.search.top_k, 5);
    }

    #[test]
    fn test_rag_result_structure() {
        let result = RAGResult {
            query: "test query".to_string(),
            augmented_prompt: "augmented".to_string(),
            context: AssembledContext {
                text: "context".to_string(),
                document_count: 2,
                estimated_tokens: 100,
                document_ids: vec!["1".to_string(), "2".to_string()],
            },
            documents_retrieved: 5,
            documents_reranked: 3,
        };

        assert_eq!(result.query, "test query");
        assert_eq!(result.documents_retrieved, 5);
        assert_eq!(result.context.document_count, 2);
    }
}
