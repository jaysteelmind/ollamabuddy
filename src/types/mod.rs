//! Type definitions module
//! 
//! Core types for agent communication and memory management.

pub mod messages;

// Re-export commonly used types
pub use messages::{AgentMsg, MemoryEntry};
