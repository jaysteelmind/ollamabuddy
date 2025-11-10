//! Message types for agent communication
//! 
//! Defines the structured messages exchanged between the agent,
//! model, and tool execution system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent messages parsed from model output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMsg {
    /// Model has created a plan with steps
    Plan {
        steps: Vec<String>,
        reasoning: Option<String>,
    },

    /// Model requests tool execution
    ToolCall {
        tool: String,
        args: HashMap<String, serde_json::Value>,
    },

    /// Model asks user for clarification
    Ask {
        question: String,
    },

    /// Task completed successfully
    Final {
        result: String,
        summary: Option<String>,
    },

    /// Error occurred during execution
    Error {
        message: String,
        recoverable: bool,
    },
}

/// Memory entry types stored in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "entry_type", rename_all = "snake_case")]
pub enum MemoryEntry {
    /// User's original goal/task
    UserGoal {
        goal: String,
        timestamp: u64,
    },

    /// System prompt (always preserved)
    SystemPrompt {
        content: String,
    },

    /// Model's plan
    Plan {
        steps: Vec<String>,
        reasoning: Option<String>,
        timestamp: u64,
    },

    /// Tool execution request
    ToolCall {
        tool: String,
        args: HashMap<String, serde_json::Value>,
        timestamp: u64,
    },

    /// Tool execution result
    ToolResult {
        tool: String,
        output: String,
        success: bool,
        duration_ms: u64,
        timestamp: u64,
    },

    /// Model asking for clarification
    Question {
        question: String,
        timestamp: u64,
    },

    /// User's response to question
    UserResponse {
        response: String,
        timestamp: u64,
    },

    /// Final result
    FinalResult {
        result: String,
        summary: Option<String>,
        timestamp: u64,
    },

    /// Error entry
    ErrorEntry {
        message: String,
        recoverable: bool,
        timestamp: u64,
    },
}

impl MemoryEntry {
    /// Estimate token count for this memory entry
    pub fn estimate_tokens(&self) -> usize {
        let text = match self {
            MemoryEntry::UserGoal { goal, .. } => goal.as_str(),
            MemoryEntry::SystemPrompt { content } => content.as_str(),
            MemoryEntry::Plan { steps, reasoning, .. } => {
                let steps_text = steps.join(" ");
                let reasoning_text = reasoning.as_deref().unwrap_or("");
                return (steps_text.len() + reasoning_text.len()) / 4;
            }
            MemoryEntry::ToolCall { tool, args, .. } => {
                let args_str = serde_json::to_string(args).unwrap_or_default();
                return (tool.len() + args_str.len()) / 4;
            }
            MemoryEntry::ToolResult { output, .. } => output.as_str(),
            MemoryEntry::Question { question, .. } => question.as_str(),
            MemoryEntry::UserResponse { response, .. } => response.as_str(),
            MemoryEntry::FinalResult { result, summary, .. } => {
                let summary_text = summary.as_deref().unwrap_or("");
                return (result.len() + summary_text.len()) / 4;
            }
            MemoryEntry::ErrorEntry { message, .. } => message.as_str(),
        };
        
        // Heuristic: 1 token ≈ 4 characters
        text.chars().count() / 4
    }

    /// Get timestamp of this entry
    pub fn timestamp(&self) -> u64 {
        match self {
            MemoryEntry::UserGoal { timestamp, .. }
            | MemoryEntry::Plan { timestamp, .. }
            | MemoryEntry::ToolCall { timestamp, .. }
            | MemoryEntry::ToolResult { timestamp, .. }
            | MemoryEntry::Question { timestamp, .. }
            | MemoryEntry::UserResponse { timestamp, .. }
            | MemoryEntry::FinalResult { timestamp, .. }
            | MemoryEntry::ErrorEntry { timestamp, .. } => *timestamp,
            MemoryEntry::SystemPrompt { .. } => 0, // System prompt is timeless
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_msg_serialization() {
        let msg = AgentMsg::Plan {
            steps: vec!["Step 1".to_string(), "Step 2".to_string()],
            reasoning: Some("Because...".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMsg = serde_json::from_str(&json).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_memory_entry_token_estimation() {
        let entry = MemoryEntry::UserGoal {
            goal: "Write a 400 character test string".to_string(), // 100 chars ≈ 25 tokens
            timestamp: 1234567890,
        };

        let tokens = entry.estimate_tokens();
        assert!(tokens >= 7 && tokens <= 12); // Rough estimate check
    }

    #[test]
    fn test_memory_entry_timestamp() {
        let entry = MemoryEntry::Plan {
            steps: vec![],
            reasoning: None,
            timestamp: 9999,
        };

        assert_eq!(entry.timestamp(), 9999);
    }
}
