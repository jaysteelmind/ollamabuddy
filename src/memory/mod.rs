//! Memory and Learning System (PRD 6)
//!
//! This module implements episodic memory, semantic knowledge graphs,
//! pattern matching, and experience-based learning for the OllamaBuddy agent.
//!
//! Components:
//! - Episodic Memory: Session-scoped experience tracking
//! - Knowledge Graph: Semantic entity and relationship extraction
//! - Pattern Matcher: LSH-based similar problem detection
//! - Experience Tracker: Bayesian success/failure tracking
//! - Working Memory: Active context cache

pub mod episodic;
pub mod knowledge;
pub mod patterns;
pub mod experience;
pub mod working;
pub mod types;

pub use episodic::EpisodicMemory;
pub use knowledge::KnowledgeGraph;
pub use patterns::PatternMatcher;
pub use experience::ExperienceTracker;
pub use working::WorkingMemory;
pub use types::{Episode, EpisodeOutcome, PatternMatch, Recommendation};
