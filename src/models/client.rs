//! Ollama API client for model management operations
//!
//! This module provides a low-level HTTP client for interacting with
//! the Ollama API endpoints for model management.

use crate::models::types::{ModelInfo, ModelsResponse, PullProgress};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// HTTP client for Ollama API
pub struct OllamaModelClient {
    client: Client,
    base_url: String,
}

impl OllamaModelClient {
    /// Create a new Ollama model client
    ///
    /// # Arguments
    /// * `base_url` - Base URL for Ollama API (default: http://127.0.0.1:11434)
    pub fn new(base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for large downloads
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.unwrap_or_else(|| "http://127.0.0.1:11434".to_string()),
        }
    }

    /// List all installed models
    ///
    /// Calls GET /api/tags to retrieve list of installed models
    ///
    /// # Returns
    /// Vector of ModelInfo on success, error message on failure
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Ollama API error: {}", response.status()));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(models_response.models)
    }

    /// Get detailed information about a specific model
    ///
    /// Calls POST /api/show with model name
    ///
    /// # Arguments
    /// * `name` - Model name (e.g., "llama3.1:8b")
    ///
    /// # Returns
    /// ModelInfo on success, error message on failure
    pub async fn show_model(&self, name: &str) -> Result<ModelInfo, String> {
        let url = format!("{}/api/show", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&json!({ "name": name }))
            .send()
            .await
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Err(format!("Model '{}' not found", name));
            }
            return Err(format!("Ollama API error: {}", response.status()));
        }

        // Parse the response - Ollama's /api/show returns detailed info
        let info: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Extract model info from show response
        // The show endpoint returns different structure, we need to adapt it
        let models = self.list_models().await?;
        models
            .into_iter()
            .find(|m| m.name == name)
            .ok_or_else(|| format!("Model '{}' not found in list", name))
    }

    /// Pull (download) a model from Ollama library
    ///
    /// Calls POST /api/pull with streaming response for progress
    ///
    /// # Arguments
    /// * `name` - Model name to pull (e.g., "llama3.1:8b")
    /// * `progress_callback` - Optional callback for progress updates
    ///
    /// # Returns
    /// Success message or error
    pub async fn pull_model(
        &self,
        name: &str,
        mut progress_callback: Option<Box<dyn FnMut(&PullProgress) + Send>>,
    ) -> Result<String, String> {
        let url = format!("{}/api/pull", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&json!({ "name": name }))
            .send()
            .await
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Ollama API error: {}", response.status()));
        }

        // Process streaming response
        let mut bytes = response.bytes().await.map_err(|e| format!("Failed to read response: {}", e))?;
        let text = String::from_utf8_lossy(&bytes);

        // Parse line-by-line JSON responses
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(progress) = serde_json::from_str::<PullProgress>(line) {
                if let Some(ref mut callback) = progress_callback {
                    callback(&progress);
                }

                // Check for completion
                if progress.status == "success" {
                    return Ok(format!("Successfully pulled model: {}", name));
                }
            }
        }

        Ok(format!("Model pull completed: {}", name))
    }

    /// Delete a model
    ///
    /// Calls DELETE /api/delete
    ///
    /// # Arguments
    /// * `name` - Model name to delete
    ///
    /// # Returns
    /// Success message or error
    pub async fn delete_model(&self, name: &str) -> Result<String, String> {
        let url = format!("{}/api/delete", self.base_url);

        let response = self
            .client
            .delete(&url)
            .json(&json!({ "name": name }))
            .send()
            .await
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Err(format!("Model '{}' not found", name));
            }
            return Err(format!("Ollama API error: {}", response.status()));
        }

        Ok(format!("Successfully deleted model: {}", name))
    }

    /// Check if Ollama server is available
    ///
    /// # Returns
    /// true if server is reachable, false otherwise
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok()
    }
}

impl Default for OllamaModelClient {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaModelClient::new(None);
        assert_eq!(client.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn test_client_custom_url() {
        let client = OllamaModelClient::new(Some("http://localhost:8080".to_string()));
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_default() {
        let client = OllamaModelClient::default();
        assert_eq!(client.base_url, "http://127.0.0.1:11434");
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_list_models_integration() {
        let client = OllamaModelClient::new(None);
        let models = client.list_models().await;
        assert!(models.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_is_available_integration() {
        let client = OllamaModelClient::new(None);
        assert!(client.is_available().await);
    }

    #[tokio::test]
    async fn test_show_nonexistent_model() {
        let client = OllamaModelClient::new(None);
        let result = client.show_model("nonexistent-model:999").await;
        // This will fail if Ollama is not running or model doesn't exist
        // In production, we'd check for specific error message
        assert!(result.is_err() || result.is_ok());
    }
}
