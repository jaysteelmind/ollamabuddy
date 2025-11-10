//! Error types for OllamaBuddy v0.2
//! 
//! Provides comprehensive error handling with context propagation
//! following the Universal Mathematical Development Framework.

use thiserror::Error;

/// Main error type for the OllamaBuddy agent system
#[derive(Error, Debug)]
pub enum AgentError {
    /// State machine transition errors
    #[error("Invalid state transition from {from:?} to {to:?}: {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    /// Context management errors
    #[error("Context window overflow: {current} tokens exceeds maximum {max} tokens")]
    ContextOverflow { current: usize, max: usize },

    /// Token counting errors
    #[error("Token counting failed: {0}")]
    TokenCountError(String),

    /// Memory management errors
    #[error("Memory limit exceeded: {current} entries > {max} entries")]
    MemoryOverflow { current: usize, max: usize },

    /// Streaming errors
    #[error("Streaming error: {0}")]
    StreamingError(String),

    /// JSON parsing errors
    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    /// Ollama API errors
    #[error("Ollama API error: {0}")]
    OllamaApiError(String),

    /// HTTP client errors
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// I/O errors
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Timeout errors
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Generic errors with context
    #[error("Agent error: {0}")]
    Generic(String),
}

/// Result type alias for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Convert anyhow errors to AgentError
impl From<anyhow::Error> for AgentError {
    fn from(err: anyhow::Error) -> Self {
        AgentError::Generic(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AgentError::ContextOverflow {
            current: 9000,
            max: 8000,
        };
        assert!(err.to_string().contains("9000"));
        assert!(err.to_string().contains("8000"));
    }

    #[test]
    fn test_invalid_transition_error() {
        let err = AgentError::InvalidTransition {
            from: "Planning".to_string(),
            to: "Init".to_string(),
            reason: "Cannot go backwards".to_string(),
        };
        assert!(err.to_string().contains("Planning"));
        assert!(err.to_string().contains("Init"));
    }
}
