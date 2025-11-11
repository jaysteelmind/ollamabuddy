//! Progress tracking with milestone monitoring
//!
//! Tracks progress toward goal completion with mathematical guarantee:
//! Progress is monotonic non-decreasing (never goes backward).

use crate::planning::types::{ProgressMetrics, GoalTree, GoalStatus};
use std::collections::HashSet;

/// Progress tracker with milestone monitoring
pub struct ProgressTracker {
    /// Metrics for progress calculation
    metrics: ProgressMetrics,
    
    /// Total number of goals in tree
    total_goals: usize,
    
    /// Completed goal IDs
    completed_goals: HashSet<usize>,
    
    /// Total expected tool executions
    expected_tools: usize,
    
    /// Successful tool executions
    successful_tools: usize,
    
    /// Total milestones defined
    total_milestones: usize,
    
    /// Reached milestones
    reached_milestones: HashSet<String>,
    
    /// Weights for progress components
    weight_goals: f64,
    weight_tools: f64,
    weight_milestones: f64,
}

impl ProgressTracker {
    /// Create new progress tracker for a goal tree
    pub fn new(goal_tree: &GoalTree) -> Self {
        let total_goals = goal_tree.nodes.len();
        let expected_tools = Self::estimate_tool_count(goal_tree);
        
        Self {
            metrics: ProgressMetrics::new(),
            total_goals,
            completed_goals: HashSet::new(),
            expected_tools,
            successful_tools: 0,
            total_milestones: Self::identify_milestones(goal_tree),
            reached_milestones: HashSet::new(),
            weight_goals: 0.40,
            weight_tools: 0.30,
            weight_milestones: 0.30,
        }
    }
    
    /// Update progress after goal completion
    pub fn update_goal_completion(&mut self, goal_id: usize) {
        if self.completed_goals.insert(goal_id) {
            self.recalculate_progress();
        }
    }
    
    /// Update progress after tool execution
    pub fn update_tool_execution(&mut self, success: bool) {
        if success {
            self.successful_tools += 1;
        }
        self.recalculate_progress();
    }
    
    /// Update progress after milestone reached
    pub fn update_milestone(&mut self, milestone: String) {
        if self.reached_milestones.insert(milestone) {
            self.recalculate_progress();
        }
    }
    
    /// Get current progress metrics
    pub fn get_metrics(&self) -> &ProgressMetrics {
        &self.metrics
    }
    
    /// Check if progress is stagnant
    pub fn is_stagnant(&self) -> bool {
        self.metrics.stagnant_iterations >= 4
    }
    
    /// Increment stagnant counter
    pub fn increment_stagnant(&mut self) {
        self.metrics.stagnant_iterations += 1;
    }
    
    /// Reset stagnant counter (progress was made)
    pub fn reset_stagnant(&mut self) {
        self.metrics.stagnant_iterations = 0;
    }
    
    /// Recalculate all progress metrics
    fn recalculate_progress(&mut self) {
        // Calculate goal completion ratio
        self.metrics.goal_completion = if self.total_goals > 0 {
            self.completed_goals.len() as f64 / self.total_goals as f64
        } else {
            0.0
        };
        
        // Calculate tool success ratio
        self.metrics.tool_success_rate = if self.expected_tools > 0 {
            (self.successful_tools as f64 / self.expected_tools as f64).min(1.0)
        } else {
            0.0
        };
        
        // Calculate milestone progress
        self.metrics.milestone_progress = if self.total_milestones > 0 {
            self.reached_milestones.len() as f64 / self.total_milestones as f64
        } else {
            0.0
        };
        
        // Calculate overall progress (weighted)
        self.metrics.overall_progress = 
            self.weight_goals * self.metrics.goal_completion +
            self.weight_tools * self.metrics.tool_success_rate +
            self.weight_milestones * self.metrics.milestone_progress;
        
        // Ensure bounded [0.0, 1.0]
        self.metrics.overall_progress = self.metrics.overall_progress.max(0.0).min(1.0);
    }
    
    /// Estimate tool count from goal tree
    fn estimate_tool_count(tree: &GoalTree) -> usize {
        // Count leaf nodes (atomic goals) as tool operations
        tree.nodes.values()
            .filter(|node| !tree.edges.contains_key(&node.id))
            .count()
            .max(1) // At least 1 tool expected
    }
    
    /// Identify milestones from goal tree structure
    fn identify_milestones(tree: &GoalTree) -> usize {
        // Milestones: non-leaf composite nodes at depth 1-2
        tree.nodes.values()
            .filter(|node| {
                node.depth >= 1 && node.depth <= 2 &&
                tree.edges.get(&node.id).map(|c| !c.is_empty()).unwrap_or(false)
            })
            .count()
            .max(1) // At least 1 milestone
    }
    
    /// Get progress percentage (0-100)
    pub fn get_progress_percentage(&self) -> f64 {
        self.metrics.overall_progress * 100.0
    }
    
    /// Get completion status summary
    pub fn get_summary(&self) -> ProgressSummary {
        ProgressSummary {
            completed_goals: self.completed_goals.len(),
            total_goals: self.total_goals,
            successful_tools: self.successful_tools,
            expected_tools: self.expected_tools,
            reached_milestones: self.reached_milestones.len(),
            total_milestones: self.total_milestones,
            overall_progress: self.metrics.overall_progress,
            stagnant: self.is_stagnant(),
        }
    }
}

/// Progress summary for display
#[derive(Debug, Clone)]
pub struct ProgressSummary {
    pub completed_goals: usize,
    pub total_goals: usize,
    pub successful_tools: usize,
    pub expected_tools: usize,
    pub reached_milestones: usize,
    pub total_milestones: usize,
    pub overall_progress: f64,
    pub stagnant: bool,
}

impl ProgressSummary {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        format!(
            "Progress: {:.1}% | Goals: {}/{} | Tools: {}/{} | Milestones: {}/{}{}",
            self.overall_progress * 100.0,
            self.completed_goals,
            self.total_goals,
            self.successful_tools,
            self.expected_tools,
            self.reached_milestones,
            self.total_milestones,
            if self.stagnant { " [STAGNANT]" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::types::{GoalTree, NodeType};
    
    #[test]
    fn test_tracker_creation() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let tracker = ProgressTracker::new(&tree);
        
        assert_eq!(tracker.total_goals, 1);
        assert_eq!(tracker.completed_goals.len(), 0);
        assert!(tracker.metrics.overall_progress >= 0.0);
        assert!(tracker.metrics.overall_progress <= 1.0);
    }
    
    #[test]
    fn test_goal_completion_updates_progress() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        let initial_progress = tracker.metrics.overall_progress;
        
        tracker.update_goal_completion(0);
        
        // Progress should increase
        assert!(tracker.metrics.overall_progress > initial_progress);
        assert_eq!(tracker.completed_goals.len(), 1);
    }
    
    #[test]
    fn test_tool_execution_updates_progress() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        let initial_progress = tracker.metrics.overall_progress;
        
        tracker.update_tool_execution(true);
        
        // Progress should increase
        assert!(tracker.metrics.overall_progress >= initial_progress);
        assert_eq!(tracker.successful_tools, 1);
    }
    
    #[test]
    fn test_milestone_updates_progress() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        let initial_progress = tracker.metrics.overall_progress;
        
        tracker.update_milestone("Phase 1 complete".to_string());
        
        // Progress should increase
        assert!(tracker.metrics.overall_progress >= initial_progress);
        assert_eq!(tracker.reached_milestones.len(), 1);
    }
    
    #[test]
    fn test_progress_is_monotonic() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        let mut last_progress = tracker.metrics.overall_progress;
        
        // Multiple updates
        tracker.update_tool_execution(true);
        assert!(tracker.metrics.overall_progress >= last_progress);
        last_progress = tracker.metrics.overall_progress;
        
        tracker.update_goal_completion(0);
        assert!(tracker.metrics.overall_progress >= last_progress);
        last_progress = tracker.metrics.overall_progress;
        
        tracker.update_milestone("Test".to_string());
        assert!(tracker.metrics.overall_progress >= last_progress);
    }
    
    #[test]
    fn test_progress_bounded() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        // Even with excessive updates, should stay bounded
        for _ in 0..100 {
            tracker.update_tool_execution(true);
        }
        
        assert!(tracker.metrics.overall_progress >= 0.0);
        assert!(tracker.metrics.overall_progress <= 1.0);
    }
    
    #[test]
    fn test_stagnant_detection() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        assert!(!tracker.is_stagnant());
        
        // Increment stagnant counter
        for _ in 0..4 {
            tracker.increment_stagnant();
        }
        
        assert!(tracker.is_stagnant());
        
        // Reset should clear
        tracker.reset_stagnant();
        assert!(!tracker.is_stagnant());
    }
    
    #[test]
    fn test_progress_summary() {
        let mut tree = GoalTree::new("Root".to_string(), 0.5);
        tree.add_child(0, "Child 1".to_string(), NodeType::Atomic, 0.2).unwrap();
        tree.add_child(0, "Child 2".to_string(), NodeType::Atomic, 0.2).unwrap();
        
        let mut tracker = ProgressTracker::new(&tree);
        tracker.update_goal_completion(1);
        tracker.update_tool_execution(true);
        
        let summary = tracker.get_summary();
        assert_eq!(summary.completed_goals, 1);
        assert_eq!(summary.total_goals, 3);
        assert_eq!(summary.successful_tools, 1);
        assert!(!summary.stagnant);
        
        let formatted = summary.format();
        assert!(formatted.contains("Progress:"));
        assert!(formatted.contains("Goals:"));
    }
    
    #[test]
    fn test_duplicate_updates_ignored() {
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        let mut tracker = ProgressTracker::new(&tree);
        
        tracker.update_goal_completion(0);
        let progress_after_first = tracker.metrics.overall_progress;
        
        // Duplicate update should not change progress
        tracker.update_goal_completion(0);
        assert_eq!(tracker.metrics.overall_progress, progress_after_first);
    }
}
