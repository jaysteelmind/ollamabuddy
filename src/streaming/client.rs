//! Ollama API streaming client
//! 
//! Provides real-time token streaming from Ollama with:
//! - HTTP/1.1 streaming via reqwest
//! - Endpoint: POST /api/generate
//! - Performance: P99 first token < 200ms
//! - Throughput: ≥ 15 tok/s

use crate::errors::{AgentError, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default Ollama API endpoint
pub const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";

/// Default model
pub const DEFAULT_MODEL: &str = "qwen2.5:7b-instruct";

/// Request timeout (30 seconds)
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Ollama streaming client
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    /// Create new Ollama client with default settings
    pub fn new() -> Result<Self> {
        Self::with_config(DEFAULT_OLLAMA_URL, DEFAULT_MODEL)
    }

    /// Create Ollama client with custom configuration
    pub fn with_config(base_url: &str, model: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(AgentError::HttpError)?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            model: model.to_string(),
        })
    }

    /// Generate streaming response from Ollama
    /// 
    /// # Performance Targets
    /// - First token: P99 < 200ms
    /// - Throughput: ≥ 15 tok/s
    /// 
    /// # Returns
    /// Stream of bytes chunks
    pub async fn generate_stream(
        &self,
        prompt: String,
    ) -> Result<impl futures_util::Stream<Item = Result<Vec<u8>>>> {
        let url = format!("{}/api/generate", self.base_url);

        let request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt,
            stream: true,
            options: None,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AgentError::OllamaApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let stream = response
            .bytes_stream()
            .map(|result| {
                result
                    .map(|bytes| bytes.to_vec())
                    .map_err(|e| AgentError::StreamingError(e.to_string()))
            });

        Ok(stream)
    }

    /// Check if Ollama is available
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/version", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to list models: {}", e)))?;

        if !response.status().is_success() {
            return Err(AgentError::OllamaApiError(
                "Failed to retrieve model list".to_string(),
            ));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| AgentError::OllamaApiError(format!("Failed to parse models: {}", e)))?;

        Ok(models_response
            .models
            .into_iter()
            .map(|m| m.name)
            .collect())
    }

    /// Get current model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default OllamaClient")
    }
}

/// Ollama generate request
#[derive(Debug, Clone, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<serde_json::Value>,
}

/// Ollama models list response
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

/// Model information
#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new();
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.model(), DEFAULT_MODEL);
        assert_eq!(client.base_url(), DEFAULT_OLLAMA_URL);
    }

    #[test]
    fn test_client_with_config() {
        let client = OllamaClient::with_config(
            "http://localhost:11434",
            "llama2:7b",
        );
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.model(), "llama2:7b");
        assert_eq!(client.base_url(), "http://localhost:11434");
    }
}
