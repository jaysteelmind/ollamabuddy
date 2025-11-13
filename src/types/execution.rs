//! Execution result types for shared CLI and REPL usage
//!
//! This module provides types for representing the results of agent task execution,
//! enabling consistent behavior across different execution contexts (CLI and REPL modes).

use std::time::Duration;

/// Result of executing a task through the agent
///
/// This type encapsulates all information about a task execution,
/// including success status, output, duration, and iteration count.
/// It is designed to be context-agnostic, usable in both CLI and REPL modes.
#[derive(Debug, Clone)]
pub struct TaskExecutionResult {
    /// Whether the task completed successfully
    pub success: bool,
    
    /// The final output or result message from the task
    pub output: String,
    
    /// Total time taken to execute the task
    pub duration: Duration,
    
    /// Number of agent iterations performed
    pub iterations: u32,
    
    /// Whether early success was detected (convergence)
    pub early_success: bool,
    
    /// List of files created or modified during execution
    pub files_touched: Vec<String>,
    
    /// Final validation score (0.0 - 1.0)
    pub validation_score: f64,
}

impl TaskExecutionResult {
    /// Create a new successful execution result
    pub fn success(
        output: String,
        duration: Duration,
        iterations: u32,
        files_touched: Vec<String>,
        validation_score: f64,
    ) -> Self {
        Self {
            success: true,
            output,
            duration,
            iterations,
            early_success: false,
            files_touched,
            validation_score,
        }
    }

    /// Create a new failed execution result
    pub fn failure(output: String, duration: Duration, iterations: u32) -> Self {
        Self {
            success: false,
            output,
            duration,
            iterations,
            early_success: false,
            files_touched: Vec::new(),
            validation_score: 0.0,
        }
    }

    /// Mark this result as an early success (convergence detected)
    pub fn with_early_success(mut self) -> Self {
        self.early_success = true;
        self
    }

    /// Get a human-readable summary of the execution
    pub fn summary(&self) -> String {
        let status = if self.success { "Success" } else { "Failed" };
        let early = if self.early_success { " (early)" } else { "" };
        format!(
            "{}{} in {:.2}s ({} iterations, score: {:.2})",
            status,
            early,
            self.duration.as_secs_f64(),
            self.iterations,
            self.validation_score
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_creation() {
        let result = TaskExecutionResult::success(
            "Task completed".to_string(),
            Duration::from_secs(5),
            10,
            vec!["file.txt".to_string()],
            0.95,
        );

        assert!(result.success);
        assert_eq!(result.output, "Task completed");
        assert_eq!(result.duration, Duration::from_secs(5));
        assert_eq!(result.iterations, 10);
        assert!(!result.early_success);
        assert_eq!(result.files_touched.len(), 1);
        assert_eq!(result.validation_score, 0.95);
    }

    #[test]
    fn test_failure_creation() {
        let result = TaskExecutionResult::failure(
            "Task failed".to_string(),
            Duration::from_secs(3),
            5,
        );

        assert!(!result.success);
        assert_eq!(result.output, "Task failed");
        assert_eq!(result.duration, Duration::from_secs(3));
        assert_eq!(result.iterations, 5);
        assert!(!result.early_success);
        assert!(result.files_touched.is_empty());
        assert_eq!(result.validation_score, 0.0);
    }

    #[test]
    fn test_early_success_marking() {
        let result = TaskExecutionResult::success(
            "Quick win".to_string(),
            Duration::from_secs(1),
            3,
            vec![],
            1.0,
        )
        .with_early_success();

        assert!(result.success);
        assert!(result.early_success);
    }

    #[test]
    fn test_summary_success() {
        let result = TaskExecutionResult::success(
            "Done".to_string(),
            Duration::from_millis(2500),
            8,
            vec![],
            0.88,
        );

        let summary = result.summary();
        assert!(summary.contains("Success"));
        assert!(summary.contains("2.50s"));
        assert!(summary.contains("8 iterations"));
        assert!(summary.contains("0.88"));
    }

    #[test]
    fn test_summary_early_success() {
        let result = TaskExecutionResult::success(
            "Done".to_string(),
            Duration::from_secs(1),
            3,
            vec![],
            0.95,
        )
        .with_early_success();

        let summary = result.summary();
        assert!(summary.contains("Success"));
        assert!(summary.contains("(early)"));
    }

    #[test]
    fn test_summary_failure() {
        let result = TaskExecutionResult::failure(
            "Error".to_string(),
            Duration::from_secs(2),
            5,
        );

        let summary = result.summary();
        assert!(summary.contains("Failed"));
        assert!(!summary.contains("(early)"));
    }
}
