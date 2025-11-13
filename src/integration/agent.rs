// PRD 11 Phase 4: Agent Integration for RAG-enhanced execution
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::memory::knowledge_manager::{KnowledgeCategory, KnowledgeEntry, KnowledgeManager};
use crate::rag::pipeline::{RAGConfig as PipelineConfig, RAGPipeline, RAGResult};
use crate::session::learning::{LearningConfig, LearningSystem};
use crate::session::recording::TaskRecord;

/// Complete RAG configuration for agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGConfig {
    /// Enable RAG pipeline
    pub enabled: bool,
    /// Knowledge database path
    pub db_path: PathBuf,
    /// RAG pipeline configuration
    pub pipeline: PipelineConfig,
    /// Learning system configuration
    pub learning: LearningConfig,
}

impl Default for RAGConfig {
    fn default() -> Self {
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ollamabuddy")
            .join("knowledge")
            .join("vector.db");

        Self {
            enabled: true,
            db_path,
            pipeline: PipelineConfig::default(),
            learning: LearningConfig::default(),
        }
    }
}

/// RAG-enhanced agent coordinator
pub struct RAGAgent {
    knowledge_manager: Arc<KnowledgeManager>,
    rag_pipeline: RAGPipeline,
    learning_system: LearningSystem,
    config: RAGConfig,
}

impl RAGAgent {
    /// Create new RAG-enhanced agent
    pub async fn new(config: RAGConfig) -> Result<Self> {
        // Initialize knowledge manager
        let knowledge_manager = Arc::new(
            KnowledgeManager::new(config.db_path.clone()).await?
        );

        // Initialize RAG pipeline
        let rag_pipeline = RAGPipeline::with_config(
            knowledge_manager.clone(),
            config.pipeline.clone(),
        );

        // Initialize learning system
        let learning_system = LearningSystem::new(config.learning.clone())?;

        Ok(Self {
            knowledge_manager,
            rag_pipeline,
            learning_system,
            config,
        })
    }

    /// Create with default configuration
    pub async fn default_config() -> Result<Self> {
        Self::new(RAGConfig::default()).await
    }

    /// Augment user query with relevant context
    pub async fn augment_query(&self, query: &str) -> Result<String> {
        if !self.config.enabled {
            return Ok(query.to_string());
        }

        self.rag_pipeline.augment(query).await
    }

    /// Execute RAG pipeline and get full result
    pub async fn retrieve_context(&self, query: &str) -> Result<RAGResult> {
        self.rag_pipeline.execute(query).await
    }

    /// Store knowledge entry
    pub async fn store_knowledge(&self, entry: KnowledgeEntry) -> Result<()> {
        self.knowledge_manager.store(entry).await
    }

    /// Store multiple knowledge entries
    pub async fn store_knowledge_batch(&self, entries: Vec<KnowledgeEntry>) -> Result<()> {
        self.knowledge_manager.store_batch(entries).await
    }

    /// Search knowledge base
    pub async fn search_knowledge(
        &self,
        query: &str,
        category: KnowledgeCategory,
        top_k: usize,
    ) -> Result<Vec<crate::memory::vector_db::QueryResult>> {
        self.knowledge_manager.search(query, category, top_k, 0.7).await
    }

    /// Record task execution
    pub async fn record_task(&self, task: TaskRecord) {
        self.learning_system.record_task(task).await;
    }

    /// End current session
    pub async fn end_session(&self) -> Result<()> {
        self.learning_system.end_session().await
    }

    /// Get current session statistics
    pub async fn session_stats(&self) -> crate::session::recording::SessionStats {
        self.learning_system.current_session_stats().await
    }

    /// Get cumulative statistics
    pub async fn cumulative_stats(&self) -> crate::session::statistics::CumulativeStats {
        self.learning_system.cumulative_stats().await
    }

    /// Get knowledge count by category
    pub async fn knowledge_count(&self, category: KnowledgeCategory) -> Result<u64> {
        self.knowledge_manager.stats(category).await
    }

    /// Get total knowledge count
    pub async fn total_knowledge_count(&self) -> Result<u64> {
        self.knowledge_manager.total_count().await
    }

    /// Check if RAG is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get configuration
    pub fn config(&self) -> &RAGConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_agent() -> (RAGAgent, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let config = RAGConfig {
            enabled: true,
            db_path,
            pipeline: PipelineConfig::default(),
            learning: LearningConfig {
                persistence: crate::session::persistence::PersistenceConfig {
                    storage_dir: temp_dir.path().join("sessions"),
                    max_sessions: 10,
                    auto_save: true,
                },
                auto_save: true,
                track_statistics: true,
            },
        };

        let agent = RAGAgent::new(config).await.unwrap();
        (agent, temp_dir)
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_agent_creation() {
        let (agent, _temp) = create_test_agent().await;
        assert!(agent.is_enabled());
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_augment_query() {
        let (agent, _temp) = create_test_agent().await;
        let augmented = agent.augment_query("test query").await.unwrap();
        assert!(augmented.contains("test query"));
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_record_task() {
        let (agent, _temp) = create_test_agent().await;
        
        let task = TaskRecord::new("test".to_string()).success();
        agent.record_task(task).await;
        
        let stats = agent.session_stats().await;
        assert_eq!(stats.total_tasks, 1);
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_knowledge_count() {
        let (agent, _temp) = create_test_agent().await;
        let count = agent.total_knowledge_count().await.unwrap();
        assert_eq!(count, 0); // Empty initially
    }
}
