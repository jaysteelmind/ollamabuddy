// PRD 11: Knowledge Manager - Orchestrates embeddings and vector storage
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::embedding::EmbeddingEngine;
use crate::memory::vector_db::{VectorDBManager, QueryResult};

/// Categories for knowledge storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeCategory {
    Episode,
    Knowledge,
    Code,
    Document,
}

impl KnowledgeCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Episode => "episodes",
            Self::Knowledge => "knowledge",
            Self::Code => "code",
            Self::Document => "documents",
        }
    }
}

/// Knowledge entry for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub content: String,
    pub category: KnowledgeCategory,
    pub metadata: HashMap<String, JsonValue>,
    pub timestamp: i64,
}

/// Knowledge manager coordinates embedding and vector storage
pub struct KnowledgeManager {
    embedding_engine: Arc<EmbeddingEngine>,
    vector_db: Arc<RwLock<VectorDBManager>>,
}

impl KnowledgeManager {
    /// Create new knowledge manager
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Initialize embedding engine
        let embedding_engine = Arc::new(
            EmbeddingEngine::new().context("Failed to create embedding engine")?
        );
        
        // Initialize vector database
        let vector_db = Arc::new(RwLock::new(
            VectorDBManager::new(db_path)
                .await
                .context("Failed to create vector database")?
        ));
        
        Ok(Self {
            embedding_engine,
            vector_db,
        })
    }
    
    /// Store a knowledge entry
    pub async fn store(&self, entry: KnowledgeEntry) -> Result<()> {
        // Generate embedding
        let embedding = self.embedding_engine
            .embed(&entry.content)
            .context("Failed to generate embedding")?;
        
        // Store in vector database
        let db = self.vector_db.read().await;
        db.add(
            entry.category.as_str(),
            &entry.id,
            &entry.content,
            &embedding,
            entry.metadata,
        )
        .await
        .context("Failed to store in vector database")?;
        
        Ok(())
    }
    
    /// Store multiple entries in batch
    pub async fn store_batch(&self, entries: Vec<KnowledgeEntry>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        
        // Group by category
        let mut by_category: HashMap<KnowledgeCategory, Vec<KnowledgeEntry>> = HashMap::new();
        for entry in entries {
            by_category.entry(entry.category).or_insert_with(Vec::new).push(entry);
        }
        
        // Process each category
        for (category, entries) in by_category {
            // Generate embeddings for all entries
            let texts: Vec<&str> = entries.iter().map(|e| e.content.as_str()).collect();
            let embeddings = self.embedding_engine
                .embed_batch(&texts)
                .context("Failed to generate batch embeddings")?;
            
            // Prepare batch items
            let items: Vec<_> = entries
                .into_iter()
                .zip(embeddings.into_iter())
                .map(|(entry, embedding)| {
                    (entry.id, entry.content, embedding, entry.metadata)
                })
                .collect();
            
            // Store batch
            let db = self.vector_db.read().await;
            db.add_batch(category.as_str(), items)
                .await
                .context("Failed to store batch")?;
        }
        
        Ok(())
    }
    
    /// Search for similar knowledge
    pub async fn search(
        &self,
        query: &str,
        category: KnowledgeCategory,
        n_results: usize,
        threshold: f64,
    ) -> Result<Vec<QueryResult>> {
        // Generate query embedding
        let query_embedding = self.embedding_engine
            .embed(query)
            .context("Failed to generate query embedding")?;
        
        // Search vector database
        let db = self.vector_db.read().await;
        let results = db
            .query(category.as_str(), &query_embedding, n_results, threshold)
            .await
            .context("Failed to search vector database")?;
        
        Ok(results)
    }
    
    /// Search across all categories
    pub async fn search_all(
        &self,
        query: &str,
        n_results: usize,
        threshold: f64,
    ) -> Result<Vec<QueryResult>> {
        let query_embedding = self.embedding_engine
            .embed(query)
            .context("Failed to generate query embedding")?;
        
        let mut all_results = Vec::new();
        let categories = [
            KnowledgeCategory::Episode,
            KnowledgeCategory::Knowledge,
            KnowledgeCategory::Code,
            KnowledgeCategory::Document,
        ];
        
        let db = self.vector_db.read().await;
        for category in categories {
            let results = db
                .query(category.as_str(), &query_embedding, n_results, threshold)
                .await
                .context("Failed to search category")?;
            all_results.extend(results);
        }
        
        // Sort by score descending
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_results.truncate(n_results);
        
        Ok(all_results)
    }
    
    /// Delete a knowledge entry
    pub async fn delete(&self, category: KnowledgeCategory, id: &str) -> Result<()> {
        let db = self.vector_db.read().await;
        db.delete(category.as_str(), id)
            .await
            .context("Failed to delete entry")?;
        Ok(())
    }
    
    /// Get statistics for a category
    pub async fn stats(&self, category: KnowledgeCategory) -> Result<u64> {
        let db = self.vector_db.read().await;
        db.collection_stats(category.as_str())
            .await
            .context("Failed to get stats")
    }
    
    /// Get total knowledge count across all categories
    pub async fn total_count(&self) -> Result<u64> {
        let mut total = 0;
        let categories = [
            KnowledgeCategory::Episode,
            KnowledgeCategory::Knowledge,
            KnowledgeCategory::Code,
            KnowledgeCategory::Document,
        ];
        
        let db = self.vector_db.read().await;
        for category in categories {
            total += db.collection_stats(category.as_str()).await?;
        }
        
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_manager() -> (KnowledgeManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_knowledge.db");
        let manager = KnowledgeManager::new(db_path).await.unwrap();
        (manager, temp_dir)
    }
    
    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_manager_creation() {
        let (_manager, _temp) = create_test_manager().await;
        // If we get here, creation succeeded
    }
    
    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_store_and_search() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = KnowledgeEntry {
            id: "test1".to_string(),
            content: "Rust is a systems programming language".to_string(),
            category: KnowledgeCategory::Knowledge,
            metadata: HashMap::new(),
            timestamp: 1000,
        };
        
        manager.store(entry).await.unwrap();
        
        let results = manager
            .search("programming language", KnowledgeCategory::Knowledge, 5, 0.5)
            .await
            .unwrap();
        
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "test1");
    }
    
    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_batch_store() {
        let (manager, _temp) = create_test_manager().await;
        
        let entries = vec![
            KnowledgeEntry {
                id: "1".to_string(),
                content: "First entry".to_string(),
                category: KnowledgeCategory::Knowledge,
                metadata: HashMap::new(),
                timestamp: 1000,
            },
            KnowledgeEntry {
                id: "2".to_string(),
                content: "Second entry".to_string(),
                category: KnowledgeCategory::Knowledge,
                metadata: HashMap::new(),
                timestamp: 2000,
            },
        ];
        
        manager.store_batch(entries).await.unwrap();
        
        let count = manager.stats(KnowledgeCategory::Knowledge).await.unwrap();
        assert_eq!(count, 2);
    }
    
    #[tokio::test]
    #[ignore]  // Integration test - requires model download
    async fn test_total_count() {
        let (manager, _temp) = create_test_manager().await;
        
        let entry = KnowledgeEntry {
            id: "test1".to_string(),
            content: "Test".to_string(),
            category: KnowledgeCategory::Code,
            metadata: HashMap::new(),
            timestamp: 1000,
        };
        
        manager.store(entry).await.unwrap();
        
        let total = manager.total_count().await.unwrap();
        assert_eq!(total, 1);
    }
}
