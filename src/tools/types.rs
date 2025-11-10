//! Tool execution types and structures
//! 
//! Core types for tool execution, results, and error handling.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool name that was executed
    pub tool: String,
    
    /// Execution output (stdout or result data)
    pub output: String,
    
    /// Whether execution was successful
    pub success: bool,
    
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    
    /// Optional error message if failed
    pub error: Option<String>,
    
    /// Exit code (for commands)
    pub exit_code: Option<i32>,
}

impl ToolResult {
    /// Create successful result
    pub fn success(tool: String, output: String, duration: Duration) -> Self {
        Self {
            tool,
            output,
            success: true,
            duration_ms: duration.as_millis() as u64,
            error: None,
            exit_code: Some(0),
        }
    }

    /// Create failed result
    pub fn failure(tool: String, error: String, duration: Duration) -> Self {
        Self {
            tool,
            output: String::new(),
            success: false,
            duration_ms: duration.as_millis() as u64,
            error: Some(error),
            exit_code: None,
        }
    }

    /// Create result with exit code
    pub fn with_exit_code(
        tool: String,
        output: String,
        exit_code: i32,
        duration: Duration,
    ) -> Self {
        Self {
            tool,
            output,
            success: exit_code == 0,
            duration_ms: duration.as_millis() as u64,
            error: if exit_code != 0 {
                Some(format!("Command exited with code {}", exit_code))
            } else {
                None
            },
            exit_code: Some(exit_code),
        }
    }
}

/// Tool execution context with security and resource bounds
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Working directory (jail root)
    pub working_dir: std::path::PathBuf,
    
    /// Maximum execution timeout
    pub timeout: Duration,
    
    /// Maximum output size (bytes)
    pub max_output_size: usize,
    
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| "/tmp".into()),
            timeout: Duration::from_secs(60),
            max_output_size: 2_097_152, // 2MB
            verbose: false,
        }
    }
}

impl ToolContext {
    /// Create new tool context with working directory
    pub fn new(working_dir: std::path::PathBuf) -> Self {
        Self {
            working_dir,
            ..Default::default()
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set max output size
    pub fn with_max_output_size(mut self, size: usize) -> Self {
        self.max_output_size = size;
        self
    }

    /// Enable verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// Tool schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Parameter schema (JSON Schema)
    pub parameters: serde_json::Value,
    
    /// Whether tool is read-only (safe for parallel execution)
    pub read_only: bool,
}

impl ToolSchema {
    /// Create new tool schema
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
        read_only: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            read_only,
        }
    }
}

/// Tool execution statistics
#[derive(Debug, Clone, Default)]
pub struct ToolStats {
    /// Total executions
    pub total_executions: u64,
    
    /// Successful executions
    pub successful_executions: u64,
    
    /// Failed executions
    pub failed_executions: u64,
    
    /// Total execution time (ms)
    pub total_duration_ms: u64,
    
    /// Retry attempts
    pub retry_attempts: u64,
}

impl ToolStats {
    /// Record successful execution
    pub fn record_success(&mut self, duration_ms: u64) {
        self.total_executions += 1;
        self.successful_executions += 1;
        self.total_duration_ms += duration_ms;
    }

    /// Record failed execution
    pub fn record_failure(&mut self, duration_ms: u64) {
        self.total_executions += 1;
        self.failed_executions += 1;
        self.total_duration_ms += duration_ms;
    }

    /// Record retry attempt
    pub fn record_retry(&mut self) {
        self.retry_attempts += 1;
    }

    /// Calculate average duration
    pub fn average_duration_ms(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.total_executions as f64
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            self.successful_executions as f64 / self.total_executions as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(
            "test".to_string(),
            "output".to_string(),
            Duration::from_millis(100),
        );

        assert!(result.success);
        assert_eq!(result.tool, "test");
        assert_eq!(result.output, "output");
        assert_eq!(result.duration_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult::failure(
            "test".to_string(),
            "error".to_string(),
            Duration::from_millis(50),
        );

        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "error");
        assert_eq!(result.duration_ms, 50);
    }

    #[test]
    fn test_tool_result_with_exit_code() {
        let result = ToolResult::with_exit_code(
            "test".to_string(),
            "output".to_string(),
            1,
            Duration::from_millis(200),
        );

        assert!(!result.success);
        assert_eq!(result.exit_code.unwrap(), 1);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_tool_context_default() {
        let ctx = ToolContext::default();
        assert_eq!(ctx.timeout, Duration::from_secs(60));
        assert_eq!(ctx.max_output_size, 2_097_152);
        assert!(!ctx.verbose);
    }

    #[test]
    fn test_tool_context_builder() {
        let ctx = ToolContext::default()
            .with_timeout(Duration::from_secs(30))
            .with_max_output_size(1024)
            .with_verbose(true);

        assert_eq!(ctx.timeout, Duration::from_secs(30));
        assert_eq!(ctx.max_output_size, 1024);
        assert!(ctx.verbose);
    }

    #[test]
    fn test_tool_stats_tracking() {
        let mut stats = ToolStats::default();

        stats.record_success(100);
        stats.record_success(200);
        stats.record_failure(150);
        stats.record_retry();

        assert_eq!(stats.total_executions, 3);
        assert_eq!(stats.successful_executions, 2);
        assert_eq!(stats.failed_executions, 1);
        assert_eq!(stats.retry_attempts, 1);
        assert_eq!(stats.average_duration_ms(), 150.0);
        assert!((stats.success_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_tool_schema_creation() {
        let schema = ToolSchema::new(
            "test_tool",
            "A test tool",
            serde_json::json!({"type": "object"}),
            true,
        );

        assert_eq!(schema.name, "test_tool");
        assert_eq!(schema.description, "A test tool");
        assert!(schema.read_only);
    }
}
