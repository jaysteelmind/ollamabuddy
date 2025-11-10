//! Context management module
//! 
//! Handles token counting, context window management, and compression.

pub mod counter;
pub mod compressor;

// Re-export commonly used types
pub use counter::{TokenCounter, TokenEstimate};
pub use compressor::{ContextCompressor, CompressionStats};
pub use compressor::{MAX_CONTEXT_TOKENS, COMPRESS_THRESHOLD, TARGET_AFTER_COMPRESSION};
