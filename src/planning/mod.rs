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

/// Advanced planning system integration
pub struct AdvancedPlanner {
    pub hierarchical: hierarchical::HierarchicalPlanner,
    pub complexity: complexity::ComplexityEstimator,
    pub strategies: strategies::StrategyGenerator,
    pub replanner: replanner::AdaptiveReplanner,
    pub progress: Option<progress::ProgressTracker>,
}

impl AdvancedPlanner {
    /// Create new advanced planner
    pub fn new() -> Self {
        Self {
            hierarchical: hierarchical::HierarchicalPlanner::new(),
            complexity: complexity::ComplexityEstimator::new(),
            strategies: strategies::StrategyGenerator::new(),
            replanner: replanner::AdaptiveReplanner::new(),
            progress: None,
        }
    }
    
    /// Initialize planning for a new goal
    pub fn initialize(&mut self, goal: &str, context: &[String]) -> crate::errors::Result<()> {
        // Decompose goal into tree
        let goal_tree = self.hierarchical.decompose(goal, context)?;
        
        // Initialize progress tracker
        self.progress = Some(progress::ProgressTracker::new(&goal_tree));
        
        Ok(())
    }
    
    /// Get current progress
    pub fn get_progress(&self) -> Option<&progress::ProgressTracker> {
        self.progress.as_ref()
    }
    
    /// Reset for new task
    pub fn reset(&mut self) {
        self.replanner.reset();
        self.progress = None;
    }
}

impl Default for AdvancedPlanner {
    fn default() -> Self {
        Self::new()
    }
}
