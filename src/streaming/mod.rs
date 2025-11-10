//! Streaming client module
//! 
//! Provides Ollama API client and incremental JSON parser.

pub mod client;
pub mod parser;

// Re-export commonly used types
pub use client::{OllamaClient, DEFAULT_OLLAMA_URL, DEFAULT_MODEL};
pub use parser::{JsonParser, MAX_BUFFER_SIZE};
