//! OllamaBuddy v0.5.0 - Terminal Agent Platform
//! 
//! A production-ready Rust terminal agent that transforms local Ollama models
//! into capable autonomous assistants with mathematical guarantees.
//! 
//! # Architecture
//! 
//! - **PRD 1**: Core streaming agent + context management
//! - **PRD 2**: Tool runtime + parallel execution (not yet implemented)
//! - **PRD 3**: Model advisor + telemetry (not yet implemented)

// Module declarations for PRD 1
pub mod errors;
pub mod types;
pub mod budget;
pub mod validation;
pub mod repl;
pub mod analysis;
pub mod recovery;
pub mod agent;
pub mod streaming;
pub mod context;
pub mod tools;

// Re-export commonly used types
pub use errors::{AgentError, Result};

// PRD 3: Intelligence & Interface Layer
pub mod advisor;
pub mod telemetry;
pub mod bootstrap;
pub mod doctor;
pub mod cli;
pub mod planning;
pub mod memory;
