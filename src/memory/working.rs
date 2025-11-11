//! Working Memory: Active context cache
//!
//! Maintains recent tool calls, filesystem state, and error history.

use serde_json::Value;
use std::collections::VecDeque;

/// Maximum number of recent tool calls to remember
const MAX_RECENT_TOOLS: usize = 10;

/// Maximum number of recent errors to track
const MAX_RECENT_ERRORS: usize = 20;

/// Tool call record for working memory
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    /// Tool name
    pub tool: String,
    /// Arguments
    pub args: Value,
    /// Result (truncated if too long)
    pub result: String,
    /// Success status
    pub success: bool,
    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// Error record for working memory
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    /// Error message
    pub message: String,
    /// Context when error occurred
    pub context: String,
    /// Tool that caused the error (if applicable)
    pub tool: Option<String>,
    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// Working memory cache
pub struct WorkingMemory {
    /// Current active goal
    current_goal: Option<String>,
    
    /// Recent tool calls (bounded FIFO queue)
    recent_tools: VecDeque<ToolCallRecord>,
    
    /// Known filesystem paths
    known_paths: Vec<String>,
    
    /// Recent errors
    recent_errors: VecDeque<ErrorRecord>,
}

impl WorkingMemory {
    /// Create new working memory
    pub fn new() -> Self {
        Self {
            current_goal: None,
            recent_tools: VecDeque::with_capacity(MAX_RECENT_TOOLS),
            known_paths: Vec::new(),
            recent_errors: VecDeque::with_capacity(MAX_RECENT_ERRORS),
        }
    }

    /// Set the current goal
    pub fn set_goal(&mut self, goal: String) {
        self.current_goal = Some(goal);
    }

    /// Get the current goal
    pub fn get_goal(&self) -> Option<&str> {
        self.current_goal.as_deref()
    }

    /// Record a tool call
    pub fn record_tool_call(
        &mut self,
        tool: &str,
        args: &Value,
        result: &crate::tools::types::ToolResult,
    ) {
        // Evict oldest if at capacity
        if self.recent_tools.len() >= MAX_RECENT_TOOLS {
            self.recent_tools.pop_front();
        }

        // Truncate result if too long
        let result_str = if result.output.len() > 500 {
            format!("{}... (truncated)", &result.output[..500])
        } else {
            result.output.clone()
        };

        self.recent_tools.push_back(ToolCallRecord {
            tool: tool.to_string(),
            args: args.clone(),
            result: result_str,
            success: result.success,
            timestamp: std::time::Instant::now(),
        });

        // Extract filesystem paths if applicable
        if tool == "list_dir" || tool == "read_file" || tool == "write_file" {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                if !self.known_paths.contains(&path.to_string()) {
                    self.known_paths.push(path.to_string());
                }
            }
        }
    }

    /// Record an error
    pub fn record_error(&mut self, message: String, context: String, tool: Option<String>) {
        if self.recent_errors.len() >= MAX_RECENT_ERRORS {
            self.recent_errors.pop_front();
        }

        self.recent_errors.push_back(ErrorRecord {
            message,
            context,
            tool,
            timestamp: std::time::Instant::now(),
        });
    }

    /// Get recent tool calls
    pub fn get_recent_tools(&self) -> &VecDeque<ToolCallRecord> {
        &self.recent_tools
    }

    /// Get known filesystem paths
    pub fn get_known_paths(&self) -> &[String] {
        &self.known_paths
    }

    /// Get recent errors
    pub fn get_recent_errors(&self) -> &VecDeque<ErrorRecord> {
        &self.recent_errors
    }

    /// Check if we've seen similar error recently
    pub fn has_similar_error(&self, error_msg: &str) -> bool {
        self.recent_errors
            .iter()
            .any(|e| e.message.contains(error_msg) || error_msg.contains(&e.message))
    }

    /// Clear working memory
    pub fn clear(&mut self) {
        self.current_goal = None;
        self.recent_tools.clear();
        self.known_paths.clear();
        self.recent_errors.clear();
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}
