//! Agent orchestration module
//! 
//! Core agent components including state machine, memory, and orchestrator.

pub mod state;
pub mod memory;
pub mod orchestrator;

// Re-export commonly used types
pub use state::{AgentState, StateEvent};
pub use memory::{MemoryManager, MAX_MEMORY_ENTRIES};
pub use orchestrator::AgentOrchestrator;
