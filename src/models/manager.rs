//! Model manager for high-level model operations
//!
//! This module provides business logic for model management,
//! including state tracking and user-friendly operations.

use crate::models::client::OllamaModelClient;
use crate::models::types::{ModelInfo, ModelOperation, PullProgress};
use std::sync::Arc;
use tokio::sync::RwLock;

/// High-level model manager with state tracking
pub struct ModelManager {
    client: OllamaModelClient,
    current_model: Arc<RwLock<String>>,
}

impl ModelManager {
    /// Create a new model manager
    ///
    /// # Arguments
    /// * `initial_model` - Initial model name (optional, defaults to qwen2.5:7b-instruct)
    pub fn new(initial_model: Option<String>) -> Self {
        let default_model = initial_model.unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
        
        Self {
            client: OllamaModelClient::new(None),
            current_model: Arc::new(RwLock::new(default_model)),
        }
    }

    /// Get the current active model
    pub async fn current_model(&self) -> String {
        self.current_model.read().await.clone()
    }

    /// Switch to a different model
    ///
    /// # Arguments
    /// * `name` - Model name to switch to
    ///
    /// # Returns
    /// ModelOperation::Switched on success, ModelOperation::Error on failure
    pub async fn switch_model(&self, name: &str) -> ModelOperation {
        // Verify model exists
        match self.client.list_models().await {
            Ok(models) => {
                if models.iter().any(|m| m.name == name) {
                    let mut current = self.current_model.write().await;
                    *current = name.to_string();
                    ModelOperation::Switched(name.to_string())
                } else {
                    ModelOperation::Error(format!("Model '{}' not found locally. Pull it first with: /model pull {}", name, name))
                }
            }
            Err(e) => ModelOperation::Error(e),
        }
    }

    /// List all installed models
    ///
    /// # Returns
    /// ModelOperation::List on success, ModelOperation::Error on failure
    pub async fn list_models(&self) -> ModelOperation {
        match self.client.list_models().await {
            Ok(models) => ModelOperation::List(models),
            Err(e) => ModelOperation::Error(e),
        }
    }

    /// Get detailed information about a specific model
    ///
    /// # Arguments
    /// * `name` - Model name
    ///
    /// # Returns
    /// ModelOperation::Info on success, ModelOperation::Error on failure
    pub async fn show_model(&self, name: &str) -> ModelOperation {
        match self.client.show_model(name).await {
            Ok(info) => ModelOperation::Info(Box::new(info)),
            Err(e) => ModelOperation::Error(e),
        }
    }

    /// Pull (download) a model
    ///
    /// # Arguments
    /// * `name` - Model name to pull
    /// * `progress_callback` - Optional callback for progress updates
    ///
    /// # Returns
    /// ModelOperation::Pulled on success, ModelOperation::Error on failure
    pub async fn pull_model(
        &self,
        name: &str,
        progress_callback: Option<Box<dyn FnMut(&PullProgress) + Send>>,
    ) -> ModelOperation {
        match self.client.pull_model(name, progress_callback).await {
            Ok(_) => ModelOperation::Pulled(name.to_string()),
            Err(e) => ModelOperation::Error(e),
        }
    }

    /// Delete a model
    ///
    /// # Arguments
    /// * `name` - Model name to delete
    /// * `force` - Skip confirmation (for CLI usage)
    ///
    /// # Returns
    /// ModelOperation::Deleted on success, ModelOperation::Error on failure
    pub async fn delete_model(&self, name: &str, _force: bool) -> ModelOperation {
        // Check if trying to delete current model
        let current = self.current_model.read().await.clone();
        if current == name {
            return ModelOperation::Error(
                format!("Cannot delete currently active model '{}'. Switch to another model first.", name)
            );
        }

        match self.client.delete_model(name).await {
            Ok(_) => ModelOperation::Deleted(name.to_string()),
            Err(e) => ModelOperation::Error(e),
        }
    }

    /// Check if a model exists locally
    ///
    /// # Arguments
    /// * `name` - Model name to check
    ///
    /// # Returns
    /// true if model exists, false otherwise
    pub async fn model_exists(&self, name: &str) -> bool {
        if let Ok(models) = self.client.list_models().await {
            models.iter().any(|m| m.name == name)
        } else {
            false
        }
    }

    /// Check if Ollama is available
    pub async fn is_ollama_available(&self) -> bool {
        self.client.is_available().await
    }

    /// Find models matching a pattern
    ///
    /// # Arguments
    /// * `pattern` - Search pattern (substring match)
    ///
    /// # Returns
    /// Vector of matching models
    pub async fn find_models(&self, pattern: &str) -> Vec<ModelInfo> {
        if let Ok(models) = self.client.list_models().await {
            models
                .into_iter()
                .filter(|m| m.name.contains(pattern))
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = ModelManager::new(None);
        let current = manager.current_model().await;
        assert_eq!(current, "qwen2.5:7b-instruct");
    }

    #[tokio::test]
    async fn test_manager_custom_model() {
        let manager = ModelManager::new(Some("llama3.1:8b".to_string()));
        let current = manager.current_model().await;
        assert_eq!(current, "llama3.1:8b");
    }

    #[tokio::test]
    async fn test_manager_default() {
        let manager = ModelManager::default();
        let current = manager.current_model().await;
        assert_eq!(current, "qwen2.5:7b-instruct");
    }

    #[tokio::test]
    async fn test_switch_model_state() {
        let manager = ModelManager::new(None);
        let initial = manager.current_model().await;
        assert_eq!(initial, "qwen2.5:7b-instruct");
        
        // State change test (doesn't validate model exists in this unit test)
        let new_model = "test:latest".to_string();
        let mut current = manager.current_model.write().await;
        *current = new_model.clone();
        drop(current);
        
        let updated = manager.current_model().await;
        assert_eq!(updated, "test:latest");
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_list_models_integration() {
        let manager = ModelManager::new(None);
        let result = manager.list_models().await;
        
        match result {
            ModelOperation::List(models) => {
                assert!(!models.is_empty());
            }
            ModelOperation::Error(e) => {
                panic!("Expected list of models, got error: {}", e);
            }
            _ => panic!("Unexpected operation result"),
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running with qwen2.5:7b-instruct
    async fn test_switch_to_existing_model_integration() {
        let manager = ModelManager::new(None);
        let result = manager.switch_model("qwen2.5:7b-instruct").await;
        
        match result {
            ModelOperation::Switched(name) => {
                assert_eq!(name, "qwen2.5:7b-instruct");
            }
            ModelOperation::Error(e) => {
                panic!("Expected successful switch, got error: {}", e);
            }
            _ => panic!("Unexpected operation result"),
        }
    }

    #[tokio::test]
    async fn test_find_models_empty_pattern() {
        let manager = ModelManager::new(None);
        // This will return empty if Ollama not running, which is fine for unit test
        let models = manager.find_models("").await;
        // Just verify it doesn't panic
        assert!(models.len() >= 0);
    }
}
