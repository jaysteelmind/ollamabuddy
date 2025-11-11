//! Adaptive re-planning with failure pattern detection
//!
//! Detects when current strategy is failing and switches to alternatives
//! with statistical thresholds for re-planning triggers.

use crate::planning::types::{FailurePattern, ReplanningAction, Strategy, GoalTree};
use std::collections::HashMap;

/// Adaptive re-planner with failure detection
pub struct AdaptiveReplanner {
    /// History of tool calls: tool_name -> (args_hash, count)
    call_history: HashMap<String, HashMap<u64, usize>>,
    
    /// History of empty results: tool_name -> count
    empty_results: HashMap<String, usize>,
    
    /// History of errors: error_type -> count
    error_history: HashMap<String, usize>,
    
    /// Iterations without progress
    stagnant_iterations: usize,
    
    /// Last progress value for comparison
    last_progress: f64,
    
    /// Threshold for repeated calls (3+)
    repeated_call_threshold: usize,
    
    /// Threshold for empty results (2+)
    empty_result_threshold: usize,
    
    /// Threshold for error streak (3+)
    error_streak_threshold: usize,
    
    /// Threshold for stagnant iterations (4+)
    stagnant_threshold: usize,
}

impl AdaptiveReplanner {
    /// Create new adaptive re-planner
    pub fn new() -> Self {
        Self {
            call_history: HashMap::new(),
            empty_results: HashMap::new(),
            error_history: HashMap::new(),
            stagnant_iterations: 0,
            last_progress: 0.0,
            repeated_call_threshold: 3,
            empty_result_threshold: 2,
            error_streak_threshold: 3,
            stagnant_threshold: 4,
        }
    }
    
    /// Check if re-planning should be triggered
    ///
    /// Returns: Some(FailurePattern) if failure detected, None otherwise
    pub fn should_replan(
        &mut self,
        tool: &str,
        args: &serde_json::Value,
        result: &str,
        current_progress: f64,
    ) -> Option<FailurePattern> {
        // Check for progress stagnation FIRST (lower threshold, checked every call)
        if let Some(pattern) = self.check_stagnant_progress(current_progress) {
            return Some(pattern);
        }
        
        // Check for empty results
        if let Some(pattern) = self.check_empty_results(tool, result) {
            return Some(pattern);
        }
        
        // Check for error patterns (in result text)
        if let Some(pattern) = self.check_error_streak(result) {
            return Some(pattern);
        }
        
        // Check for repeated calls (this has highest threshold)
        if let Some(pattern) = self.check_repeated_calls(tool, args) {
            return Some(pattern);
        }
        
        None
    }
    
    /// Determine re-planning action based on failure pattern
    pub fn replan(
        &self,
        pattern: FailurePattern,
        current_strategy: &Strategy,
        fallback_strategies: &[Strategy],
        goal_tree: &GoalTree,
    ) -> ReplanningAction {
        match pattern {
            FailurePattern::RepeatedCall { tool, count } => {
                if count >= 5 {
                    // Too many attempts, switch strategy
                    self.switch_to_fallback(
                        &format!("Tool '{}' called {} times without progress", tool, count),
                        fallback_strategies,
                    )
                } else {
                    // Suggest modification
                    ReplanningAction::ModifyApproach {
                        suggestion: format!("Tool '{}' not making progress. Try different arguments or alternative approach.", tool),
                        modified_plan: self.suggest_alternative_tools(&tool, goal_tree),
                    }
                }
            }
            
            FailurePattern::EmptyResults { tool, count } => {
                if count >= 3 {
                    ReplanningAction::SwitchStrategy {
                        reason: format!("Tool '{}' returned empty results {} times", tool, count),
                        new_strategy: fallback_strategies.first().cloned(),
                    }
                } else {
                    ReplanningAction::ModifyApproach {
                        suggestion: format!("Tool '{}' returning empty results. Verify inputs or try exploration.", tool),
                        modified_plan: vec![
                            "Verify input parameters are correct".to_string(),
                            "Check if target exists".to_string(),
                            "Try alternative search criteria".to_string(),
                        ],
                    }
                }
            }
            
            FailurePattern::ErrorStreak { error_type, count } => {
                if count >= self.error_streak_threshold {
                    if error_type.contains("not found") || error_type.contains("does not exist") {
                        ReplanningAction::Terminate {
                            reason: format!("Target not found after {} attempts. Task may be impossible with current context.", count),
                        }
                    } else {
                        self.switch_to_fallback(
                            &format!("Error '{}' occurred {} times", error_type, count),
                            fallback_strategies,
                        )
                    }
                } else {
                    ReplanningAction::ModifyApproach {
                        suggestion: "Errors detected. Adjust approach or verify prerequisites.".to_string(),
                        modified_plan: vec![
                            "Verify prerequisites are met".to_string(),
                            "Check permissions and access".to_string(),
                            "Retry with modified parameters".to_string(),
                        ],
                    }
                }
            }
            
            FailurePattern::StuckProgress { iterations } => {
                if iterations >= 6 {
                    ReplanningAction::Terminate {
                        reason: format!("No progress for {} iterations. Current approach not viable.", iterations),
                    }
                } else if current_strategy.strategy_type != crate::planning::types::StrategyType::Exploratory {
                    // Switch to exploratory if not already
                    ReplanningAction::SwitchStrategy {
                        reason: format!("No progress for {} iterations. Switching to exploratory approach.", iterations),
                        new_strategy: fallback_strategies.iter()
                            .find(|s| s.strategy_type == crate::planning::types::StrategyType::Exploratory)
                            .cloned(),
                    }
                } else {
                    self.switch_to_fallback(
                        &format!("No progress for {} iterations", iterations),
                        fallback_strategies,
                    )
                }
            }
        }
    }
    
    /// Reset history for new task
    pub fn reset(&mut self) {
        self.call_history.clear();
        self.empty_results.clear();
        self.error_history.clear();
        self.stagnant_iterations = 0;
        self.last_progress = 0.0;
    }
    
    /// Check for repeated identical calls
    fn check_repeated_calls(&mut self, tool: &str, args: &serde_json::Value) -> Option<FailurePattern> {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        // Hash the arguments
        let mut hasher = DefaultHasher::new();
        args.to_string().hash(&mut hasher);
        let args_hash = hasher.finish();
        
        // Track this call
        let tool_calls = self.call_history.entry(tool.to_string()).or_insert_with(HashMap::new);
        let count = tool_calls.entry(args_hash).or_insert(0);
        *count += 1;
        
        if *count >= self.repeated_call_threshold {
            Some(FailurePattern::RepeatedCall {
                tool: tool.to_string(),
                count: *count,
            })
        } else {
            None
        }
    }
    
    /// Check for empty results
    fn check_empty_results(&mut self, tool: &str, result: &str) -> Option<FailurePattern> {
        let is_empty = result.trim().is_empty() 
            || result.contains("No such file")
            || result.contains("not found")
            || result == "null"
            || result == "[]"
            || result == "{}";
        
        if is_empty {
            let count = self.empty_results.entry(tool.to_string()).or_insert(0);
            *count += 1;
            
            if *count >= self.empty_result_threshold {
                Some(FailurePattern::EmptyResults {
                    tool: tool.to_string(),
                    count: *count,
                })
            } else {
                None
            }
        } else {
            // Reset counter on success
            self.empty_results.remove(tool);
            None
        }
    }
    
    /// Check for error streaks
    fn check_error_streak(&mut self, result: &str) -> Option<FailurePattern> {
        let error_indicators = [
            "error", "failed", "Error", "Failed", "FAILED",
            "not found", "does not exist", "permission denied",
        ];
        
        let mut error_type = String::new();
        for indicator in &error_indicators {
            if result.contains(indicator) {
                error_type = indicator.to_string();
                break;
            }
        }
        
        if !error_type.is_empty() {
            let count = self.error_history.entry(error_type.clone()).or_insert(0);
            *count += 1;
            
            if *count >= self.error_streak_threshold {
                Some(FailurePattern::ErrorStreak {
                    error_type: error_type.clone(),
                    count: *count,
                })
            } else {
                None
            }
        } else {
            // Clear all error counters on success
            self.error_history.clear();
            None
        }
    }
    
    /// Check for progress stagnation
    fn check_stagnant_progress(&mut self, current_progress: f64) -> Option<FailurePattern> {
        const PROGRESS_EPSILON: f64 = 0.01; // 1% minimum progress
        
        if (current_progress - self.last_progress).abs() < PROGRESS_EPSILON {
            self.stagnant_iterations += 1;
        } else {
            self.stagnant_iterations = 0;
        }
        
        self.last_progress = current_progress;

        if self.stagnant_iterations >= self.stagnant_threshold {
            Some(FailurePattern::StuckProgress {
                iterations: self.stagnant_iterations,
            })
        } else {
            None
        }
    }
    
    /// Switch to fallback strategy
    fn switch_to_fallback(
        &self,
        reason: &str,
        fallback_strategies: &[Strategy],
    ) -> ReplanningAction {
        if let Some(fallback) = fallback_strategies.first() {
            ReplanningAction::SwitchStrategy {
                reason: reason.to_string(),
                new_strategy: Some(fallback.clone()),
            }
        } else {
            ReplanningAction::Terminate {
                reason: format!("{} and no fallback strategies available", reason),
            }
        }
    }
    
    /// Suggest alternative tools
    fn suggest_alternative_tools(&self, failing_tool: &str, _goal_tree: &GoalTree) -> Vec<String> {
        match failing_tool {
            "read_file" => vec![
                "Try list_dir to verify file exists".to_string(),
                "Check file path and permissions".to_string(),
                "Use run_command with 'cat' as alternative".to_string(),
            ],
            "list_dir" => vec![
                "Verify directory path is correct".to_string(),
                "Try run_command with 'ls' for more details".to_string(),
                "Check parent directory exists".to_string(),
            ],
            "run_command" => vec![
                "Verify command syntax is correct".to_string(),
                "Check if command is available on system".to_string(),
                "Try breaking command into smaller steps".to_string(),
            ],
            "write_file" => vec![
                "Check write permissions on target directory".to_string(),
                "Verify directory exists before writing".to_string(),
                "Try writing to a different location".to_string(),
            ],
            _ => vec![
                "Review goal and verify approach is correct".to_string(),
                "Try exploratory strategy to gather more info".to_string(),
                "Consider breaking task into smaller steps".to_string(),
            ],
        }
    }
}

impl Default for AdaptiveReplanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::types::{GoalTree, StrategyType};
    use serde_json::json;
    
    #[test]
    fn test_replanner_creation() {
        let replanner = AdaptiveReplanner::new();
        assert_eq!(replanner.repeated_call_threshold, 3);
        assert_eq!(replanner.empty_result_threshold, 2);
        assert_eq!(replanner.error_streak_threshold, 3);
        assert_eq!(replanner.stagnant_threshold, 4);
    }
    
    #[test]
    fn test_repeated_calls_detection() {
        let mut replanner = AdaptiveReplanner::new();
        let args = json!({"path": "test.txt"});
        
        // First two calls - no detection
        assert!(replanner.should_replan("read_file", &args, "success", 0.5).is_none());
        assert!(replanner.should_replan("read_file", &args, "success", 0.5).is_none());
        
        // Third call - should detect
        let pattern = replanner.should_replan("read_file", &args, "success", 0.5);
        assert!(pattern.is_some());
        match pattern.unwrap() {
            FailurePattern::RepeatedCall { tool, count } => {
                assert_eq!(tool, "read_file");
                assert_eq!(count, 3);
            }
            _ => panic!("Expected RepeatedCall pattern"),
        }
    }
    
    #[test]
    fn test_empty_results_detection() {
        let mut replanner = AdaptiveReplanner::new();
        let args = json!({"path": "test.txt"});
        
        // First empty result
        assert!(replanner.should_replan("read_file", &args, "", 0.5).is_none());
        
        // Second empty result - should detect
        let pattern = replanner.should_replan("read_file", &args, "", 0.5);
        assert!(pattern.is_some());
        match pattern.unwrap() {
            FailurePattern::EmptyResults { tool, count } => {
                assert_eq!(tool, "read_file");
                assert_eq!(count, 2);
            }
            _ => panic!("Expected EmptyResults pattern"),
        }
    }
    
    #[test]
    fn test_error_streak_detection() {
        let mut replanner = AdaptiveReplanner::new();
        
        // Multiple errors with different args and tools to avoid other detection
        let args1 = json!({"path": "test1.txt"});
        let args2 = json!({"path": "test2.txt"});
        let args3 = json!({"path": "test3.txt"});
        
        assert!(replanner.should_replan("read_file", &args1, "Error: not found", 0.0).is_none());
        assert!(replanner.should_replan("list_dir", &args2, "Error: not found", 0.01).is_none());
        
        // Third error - should detect error streak
        let pattern = replanner.should_replan("write_file", &args3, "Error: not found", 0.02);
        assert!(pattern.is_some());
        match pattern.unwrap() {
            FailurePattern::ErrorStreak { error_type, count } => {
                // "Error" is detected first in the error indicators list
                assert!(error_type == "Error" || error_type == "not found");
                assert_eq!(count, 3);
            }
            _ => panic!("Expected ErrorStreak pattern"),
        }
    }
    
    #[test]
    fn test_stagnant_progress_detection() {
        let mut replanner = AdaptiveReplanner::new();
        
        // Use different args each time to avoid repeated call detection
        let args1 = json!({"path": "test1.txt"});
        let args2 = json!({"path": "test2.txt"});
        let args3 = json!({"path": "test3.txt"});
        let args4 = json!({"path": "test4.txt"});
        let args5 = json!({"path": "test5.txt"});
        
        // No progress for multiple iterations (same progress value 0.5)
        // First call initializes, then 1, 2, 3, 4 to reach threshold
        assert!(replanner.should_replan("read_file", &args1, "content 1", 0.5).is_none());
        assert!(replanner.should_replan("read_file", &args2, "content 2", 0.5).is_none());
        assert!(replanner.should_replan("read_file", &args3, "content 3", 0.5).is_none());
        assert!(replanner.should_replan("read_file", &args4, "content 4", 0.5).is_none());
        
        // Fifth iteration with no progress - should detect (iterations=4)
        let pattern = replanner.should_replan("read_file", &args5, "content 5", 0.5);
        assert!(pattern.is_some());
        match pattern.unwrap() {
            FailurePattern::StuckProgress { iterations } => {
                assert_eq!(iterations, 4);
            }
            _ => panic!("Expected StuckProgress pattern"),
        }
    }
    
    #[test]
    fn test_replan_action_switch_strategy() {
        let replanner = AdaptiveReplanner::new();
        let tree = GoalTree::new("Test goal".to_string(), 0.5);
        
        let current_strategy = Strategy {
            name: "Direct".to_string(),
            strategy_type: StrategyType::Direct,
            confidence: 0.8,
            cost: 0.3,
            applicability: 0.7,
            steps: vec![],
        };
        
        let fallback = Strategy {
            name: "Exploratory".to_string(),
            strategy_type: StrategyType::Exploratory,
            confidence: 0.7,
            cost: 0.5,
            applicability: 0.8,
            steps: vec![],
        };
        
        let pattern = FailurePattern::RepeatedCall {
            tool: "read_file".to_string(),
            count: 5,
        };
        
        let action = replanner.replan(pattern, &current_strategy, &[fallback], &tree);
        
        match action {
            ReplanningAction::SwitchStrategy { reason, new_strategy } => {
                assert!(reason.contains("read_file"));
                assert!(new_strategy.is_some());
            }
            _ => panic!("Expected SwitchStrategy action"),
        }
    }
    
    #[test]
    fn test_reset_clears_history() {
        let mut replanner = AdaptiveReplanner::new();
        let args = json!({"path": "test.txt"});
        
        // Build up history
        replanner.should_replan("read_file", &args, "error", 0.5);
        replanner.should_replan("read_file", &args, "error", 0.5);
        
        // Reset
        replanner.reset();
        
        // Should not detect pattern after reset
        assert!(replanner.should_replan("read_file", &args, "error", 0.5).is_none());
    }
}
