// PRD 11 Phase 3: Cross-Session Learning & Persistence
//
// This module implements session recording, statistics tracking,
// and cumulative learning across multiple agent sessions.
//
// Components:
// - Session Recorder: Captures session data and outcomes
// - Statistics Tracker: Tracks performance metrics over time
// - Persistence: Saves/loads session data to disk
// - Learning System: Cumulative intelligence from past sessions

pub mod recording;
pub mod statistics;
pub mod persistence;
pub mod learning;

// Re-export key types
pub use recording::SessionRecorder;
pub use statistics::StatisticsTracker;
pub use persistence::SessionPersistence;
pub use learning::LearningSystem;
