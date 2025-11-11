//! Bootstrap system for OllamaBuddy
//! 
//! Detects Ollama installation, checks model availability, and handles setup.

use crate::errors::{AgentError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Ollama detector and bootstrap manager
pub struct Bootstrap {
    client: Client,
    ollama_url: String,
    
    pub model_tag: String,
}

/// Ollama API tags response
#[derive(Debug, Deserialize, Serialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

/// Model information from Ollama API
#[derive(Debug, Deserialize, Serialize)]
struct ModelInfo {
    name: String,
    size: u64,
    digest: String,
    #[serde(default)]
    modified_at: String,
}

/// Bootstrap check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapStatus {
    Ready,
    OllamaNotRunning,
    ModelNotAvailable(String),
}

impl Bootstrap {
    /// Create a new bootstrap detector
    pub fn new(host: String, port: u16, model_tag: String) -> Self {
        let ollama_url = format!("http://{}:{}", host, port);
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());
        
        Self {
            client,
            ollama_url,
            model_tag,
        }
    }

    /// Check if Ollama API is reachable
    pub async fn check_ollama_running(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.ollama_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Check if specific model is available
    pub async fn check_model_available(&self, model_tag: &str) -> Result<bool> {
        let url = format!("{}/api/tags", self.ollama_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to query models: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AgentError::OllamaApiError(
                format!("API returned status: {}", response.status())
            ));
        }

        let tags: TagsResponse = response
            .json()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to parse response: {}", e)))?;
        
        Ok(tags.models.iter().any(|m| m.name == model_tag))
    }

    /// Get list of available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.ollama_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to query models: {}", e)))?;
        
        let tags: TagsResponse = response
            .json()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to parse response: {}", e)))?;
        
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    /// Run complete bootstrap check
    pub async fn check(&self, model_tag: &str) -> Result<BootstrapStatus> {
        // Step 1: Check if Ollama is running
        if !self.check_ollama_running().await? {
            return Ok(BootstrapStatus::OllamaNotRunning);
        }

        // Step 2: Check if model is available
        if !self.check_model_available(model_tag).await? {
            return Ok(BootstrapStatus::ModelNotAvailable(model_tag.to_string()));
        }

        Ok(BootstrapStatus::Ready)
    }

    /// Display installation instructions for Ollama
    pub fn show_ollama_install_instructions() {
        eprintln!("
❌ Ollama not found or not running!");
        eprintln!("
Ollama is required to run OllamaBuddy.");
        eprintln!("
📦 Installation:");
        eprintln!("   Linux:   curl -fsSL https://ollama.com/install.sh | sh");
        eprintln!("   macOS:   brew install ollama");
        eprintln!("
🚀 Start Ollama:");
        eprintln!("   ollama serve");
        eprintln!("
📚 More info: https://ollama.com/download");
        eprintln!();
    }

    /// Display instructions for pulling a model
    pub fn show_model_pull_instructions(model_tag: &str) {
        eprintln!("
❌ Model '{}' not found!", model_tag);
        eprintln!("
To download this model, run:");
        eprintln!("   ollama pull {}", model_tag);
        eprintln!("
Or choose a different model with:");
        eprintln!("   ollamabuddy -m <model> <task>");
        eprintln!("
Available models at: https://ollama.com/library");
        eprintln!();
    }
}

/// Exit code for setup needed
pub const EXIT_CODE_SETUP_NEEDED: i32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = Bootstrap::new("localhost".to_string(), 11434, "qwen2.5:7b".to_string());
        assert_eq!(detector.ollama_url, "http://localhost:11434");
    }

    #[test]
    fn test_bootstrap_status_equality() {
        assert_eq!(BootstrapStatus::Ready, BootstrapStatus::Ready);
        assert_eq!(
            BootstrapStatus::OllamaNotRunning,
            BootstrapStatus::OllamaNotRunning
        );
        assert_eq!(
            BootstrapStatus::ModelNotAvailable("test".to_string()),
            BootstrapStatus::ModelNotAvailable("test".to_string())
        );
    }

    #[test]
    fn test_exit_code_constant() {
        assert_eq!(EXIT_CODE_SETUP_NEEDED, 2);
    }

    // Note: Integration tests for actual API calls would require
    // a running Ollama instance and are better suited for E2E tests
}
