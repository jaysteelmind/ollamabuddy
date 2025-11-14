//! Type definitions for Ollama model management
//!
//! This module defines the core data structures for interacting with
//! the Ollama API and managing model operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Information about an Ollama model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name (e.g., "llama3.1:8b")
    pub name: String,
    
    /// Model size in bytes
    pub size: u64,
    
    /// Last modification time
    pub modified_at: DateTime<Utc>,
    
    /// Model digest/hash
    pub digest: String,
    
    /// Model details (optional, from API)
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

/// Detailed model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    /// Model format (e.g., "gguf")
    #[serde(default)]
    pub format: Option<String>,
    
    /// Model family (e.g., "llama", "qwen2")
    #[serde(default)]
    pub family: Option<String>,
    
    /// Parameter size (e.g., "7B", "13B")
    #[serde(default)]
    pub parameter_size: Option<String>,
    
    /// Quantization level (e.g., "Q4_0", "Q4_K_M")
    #[serde(default)]
    pub quantization_level: Option<String>,
}

/// Response from Ollama /api/tags endpoint
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Progress update during model pull operation
#[derive(Debug, Deserialize)]
pub struct PullProgress {
    /// Status message
    pub status: String,
    
    /// Digest being pulled
    #[serde(default)]
    pub digest: Option<String>,
    
    /// Total bytes to download
    #[serde(default)]
    pub total: Option<u64>,
    
    /// Bytes completed
    #[serde(default)]
    pub completed: Option<u64>,
}

/// Result of a model operation
#[derive(Debug)]
pub enum ModelOperation {
    /// Successfully listed models
    List(Vec<ModelInfo>),
    
    /// Successfully retrieved model info
    Info(Box<ModelInfo>),
    
    /// Successfully pulled model
    Pulled(String),
    
    /// Successfully deleted model
    Deleted(String),
    
    /// Successfully switched model
    Switched(String),
    
    /// Operation failed with error
    Error(String),
}

impl ModelInfo {
    /// Format the model size in human-readable format
    pub fn formatted_size(&self) -> String {
        format_size(self.size)
    }
    
    /// Get a short description of the model
    pub fn description(&self) -> String {
        if let Some(ref details) = self.details {
            let mut parts = Vec::new();
            
            if let Some(ref param) = details.parameter_size {
                parts.push(param.clone());
            }
            
            if let Some(ref quant) = details.quantization_level {
                parts.push(quant.clone());
            }
            
            if !parts.is_empty() {
                return parts.join(" ");
            }
        }
        
        self.formatted_size()
    }
}

impl fmt::Display for ModelInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.formatted_size())
    }
}

impl fmt::Display for ModelOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelOperation::List(models) => {
                write!(f, "Listed {} model(s)", models.len())
            }
            ModelOperation::Info(info) => {
                write!(f, "Model info: {}", info.name)
            }
            ModelOperation::Pulled(name) => {
                write!(f, "Successfully pulled: {}", name)
            }
            ModelOperation::Deleted(name) => {
                write!(f, "Successfully deleted: {}", name)
            }
            ModelOperation::Switched(name) => {
                write!(f, "Switched to model: {}", name)
            }
            ModelOperation::Error(err) => {
                write!(f, "Error: {}", err)
            }
        }
    }
}

/// Format bytes into human-readable size
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let base: f64 = 1024.0;
    let exponent = (bytes as f64).log(base).floor() as usize;
    let exponent = exponent.min(UNITS.len() - 1);
    
    let size = bytes as f64 / base.powi(exponent as i32);
    
    format!("{:.2} {}", size, UNITS[exponent])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500.00 B");
        assert_eq!(format_size(1023), "1023.00 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(5242880), "5.00 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(4683087332), "4.36 GB"); // qwen2.5:7b size
    }

    #[test]
    fn test_model_info_formatted_size() {
        let info = ModelInfo {
            name: "test:latest".to_string(),
            size: 1073741824,
            modified_at: Utc::now(),
            digest: "abc123".to_string(),
            details: None,
        };
        
        assert_eq!(info.formatted_size(), "1.00 GB");
    }

    #[test]
    fn test_model_info_description_with_details() {
        let info = ModelInfo {
            name: "test:latest".to_string(),
            size: 1073741824,
            modified_at: Utc::now(),
            digest: "abc123".to_string(),
            details: Some(ModelDetails {
                format: Some("gguf".to_string()),
                family: Some("llama".to_string()),
                parameter_size: Some("7B".to_string()),
                quantization_level: Some("Q4_K_M".to_string()),
            }),
        };
        
        assert_eq!(info.description(), "7B Q4_K_M");
    }

    #[test]
    fn test_model_info_description_without_details() {
        let info = ModelInfo {
            name: "test:latest".to_string(),
            size: 1073741824,
            modified_at: Utc::now(),
            digest: "abc123".to_string(),
            details: None,
        };
        
        assert_eq!(info.description(), "1.00 GB");
    }

    #[test]
    fn test_model_operation_display() {
        let op = ModelOperation::Pulled("llama3.1:8b".to_string());
        assert_eq!(op.to_string(), "Successfully pulled: llama3.1:8b");
        
        let op = ModelOperation::Error("Not found".to_string());
        assert_eq!(op.to_string(), "Error: Not found");
    }

    #[test]
    fn test_model_info_serialization() {
        let info = ModelInfo {
            name: "test:latest".to_string(),
            size: 1073741824,
            modified_at: Utc::now(),
            digest: "abc123".to_string(),
            details: None,
        };
        
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info.name, deserialized.name);
        assert_eq!(info.size, deserialized.size);
    }
}
