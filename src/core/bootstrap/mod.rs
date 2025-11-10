//! Bootstrap system - Ollama detection and model management

use crate::errors::{AgentError, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

/// Ollama detector and bootstrap manager
pub struct Bootstrap {
    client: Client,
    base_url: String,
    model_tag: String,
}

impl Bootstrap {
    /// Create new bootstrap manager
    pub fn new(host: String, port: u16, model_tag: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base_url: format!("http://{}:{}", host, port),
            model_tag,
        }
    }

    /// Check if Ollama API is reachable
    pub async fn is_ollama_running(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Check if model is available
    pub async fn is_model_available(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client.get(&url).send().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        if !response.status().is_success() {
            return Ok(false);
        }
        
        let tags: TagsResponse = response.json().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        Ok(tags.models.iter().any(|m| m.name == self.model_tag))
    }

    /// Ensure Ollama is running
    pub async fn ensure_ollama(&self) -> Result<()> {
        if self.is_ollama_running().await? {
            Ok(())
        } else {
            Err(AgentError::ConfigError(
                "Ollama not running. Start with: ollama serve".to_string()
            ))
        }
    }

    /// Ensure model is available
    pub async fn ensure_model(&self) -> Result<()> {
        if self.is_model_available().await? {
            Ok(())
        } else {
            Err(AgentError::ConfigError(
                format!("Model {} not found. Pull with: ollama pull {}", 
                    self.model_tag, self.model_tag)
            ))
        }
    }

    /// Get Ollama version
    pub async fn get_version(&self) -> Result<String> {
        let url = format!("{}/api/version", self.base_url);
        
        let response = self.client.get(&url).send().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        let version: serde_json::Value = response.json().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        Ok(version["version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string())
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client.get(&url).send().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        let tags: TagsResponse = response.json().await
            .map_err(|e| AgentError::HttpError(e))?;
        
        Ok(tags.models.iter().map(|m| m.name.clone()).collect())
    }

    /// Print installation instructions
    pub fn print_install_instructions() {
        #[cfg(target_os = "linux")]
        Self::print_linux_instructions();
        
        #[cfg(target_os = "macos")]
        Self::print_macos_instructions();
    }

    #[cfg(target_os = "linux")]
    fn print_linux_instructions() {
        println!("
╔═══════════════════════════════════════════════════════╗");
        println!("║ Ollama Installation Required                          ║");
        println!("╠═══════════════════════════════════════════════════════╣");
        println!("║                                                       ║");
        println!("║ Install: curl -fsSL https://ollama.com/install.sh |sh║");
        println!("║ Start:   ollama serve &                              ║");
        println!("║ Pull:    ollama pull qwen2.5:7b-instruct            ║");
        println!("║                                                       ║");
        println!("╚═══════════════════════════════════════════════════════╝
");
    }

    #[cfg(target_os = "macos")]
    fn print_macos_instructions() {
        println!("
╔═══════════════════════════════════════════════════════╗");
        println!("║ Ollama Installation Required                          ║");
        println!("╠═══════════════════════════════════════════════════════╣");
        println!("║                                                       ║");
        println!("║ Install: brew install ollama                         ║");
        println!("║ Start:   brew services start ollama                  ║");
        println!("║ Pull:    ollama pull qwen2.5:7b-instruct            ║");
        println!("║                                                       ║");
        println!("╚═══════════════════════════════════════════════════════╝
");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_creation() {
        let bootstrap = Bootstrap::new(
            "127.0.0.1".to_string(),
            11434,
            "qwen2.5:7b-instruct".to_string(),
        );
        
        assert_eq!(bootstrap.base_url, "http://127.0.0.1:11434");
    }
}
