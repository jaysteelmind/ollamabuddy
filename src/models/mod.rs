//! Ollama model management module
//!
//! This module provides functionality for managing Ollama models:
//! - Listing installed models
//! - Downloading (pulling) models
//! - Deleting models
//! - Switching active models
//! - Viewing model information

pub mod client;
pub mod manager;
pub mod types;

// Re-export key types for convenience
pub use client::OllamaModelClient;
pub use manager::ModelManager;
pub use types::{ModelInfo, ModelOperation, PullProgress};
