//! Adaptive recovery and failure handling system
//! Provides intelligent failure pattern recognition and strategy rotation

pub mod adaptive;
pub mod types;

pub use adaptive::AdaptiveRecovery;
pub use types::{FailurePattern, FailureSymptom, RecoveryAction, RecoveryStrategy};
