//! Tool execution system (PRD 2)
//! 
//! Provides secure, parallel tool execution with:
//! - 6 production tools (filesystem + process)
//! - Path jail security (escape impossibility proof)
//! - Parallel executor (4 concurrent operations)
//! - Retry manager (exponential backoff)
//! - Tool runtime coordinator

pub mod types;
pub mod registry;
pub mod security;
pub mod retry;
pub mod executor;
pub mod runtime;
pub mod implementations;

// Re-export commonly used types
pub use types::{ToolResult, ToolContext, ToolSchema, ToolStats};
pub use registry::ToolRegistry;
pub use security::PathJail;
pub use retry::RetryManager;
pub use executor::ParallelExecutor;
pub use runtime::ToolRuntime;
