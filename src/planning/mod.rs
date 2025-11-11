//! Advanced planning and reasoning system for OllamaBuddy v0.3
//!
//! Provides hierarchical task decomposition, multi-strategy planning,
//! adaptive re-planning, and progress tracking with mathematical guarantees.

pub mod types;
pub mod hierarchical;
pub mod complexity;
pub mod strategies;
pub mod replanner;
pub mod progress;

// Re-export commonly used types
pub use types::{
    GoalTree, GoalNode, NodeId, NodeType, GoalStatus,
    Strategy, StrategyType, PlanStep,
    FailurePattern, ReplanningAction, ProgressMetrics,
};
