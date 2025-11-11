//! Core data types for the Memory & Learning System

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Unique identifier for episodes
pub type EpisodeId = uuid::Uuid;

/// Episode outcome classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EpisodeOutcome {
    /// Episode completed successfully
    Success,
    /// Episode failed with error reason
    Failure(String),
}

/// Episode: Record of a goal-solving experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique episode identifier
    pub id: EpisodeId,
    /// Natural language goal description
    pub goal: String,
    /// Context state at episode start
    pub context: String,
    /// Sequence of (tool, args, result) triples
    pub actions: Vec<ActionRecord>,
    /// Final outcome of the episode
    pub outcome: EpisodeOutcome,
    /// Episode metadata
    pub metadata: EpisodeMetadata,
}

/// Record of a single action within an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Tool name
    pub tool: String,
    /// Tool arguments (JSON)
    pub args: serde_json::Value,
    /// Tool result
    pub result: String,
    /// Action success status
    pub success: bool,
}

/// Episode metadata for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeMetadata {
    /// Complexity score (0.0-1.0)
    pub complexity_score: f64,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of tool calls
    pub tool_count: usize,
    /// Strategy used
    pub strategy_used: String,
    /// Timestamp
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
    /// Similarity hash for fast lookup
    pub similarity_hash: u64,
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Matched episode
    pub episode: Episode,
    /// Similarity score (0.0-1.0)
    pub similarity: f64,
}

/// Tool recommendation with confidence
#[derive(Debug, Clone)]
pub struct Recommendation {
    /// Recommended tool name
    pub tool: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Success rate from experience
    pub success_rate: f64,
    /// Number of observations
    pub sample_size: usize,
}

impl Episode {
    /// Create a new episode
    pub fn new(goal: String, context: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            goal,
            context,
            actions: Vec::new(),
            outcome: EpisodeOutcome::Success,
            metadata: EpisodeMetadata {
                complexity_score: 0.0,
                duration_ms: 0,
                tool_count: 0,
                strategy_used: String::new(),
                timestamp: Instant::now(),
                similarity_hash: 0,
            },
        }
    }
}
