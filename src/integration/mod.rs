// PRD 11 Phase 4: Agent Integration Module
//
// This module integrates long-term memory, RAG pipeline, and
// cross-session learning into the agent orchestrator and REPL.
//
// Components:
// - Agent Integration: RAG-enhanced agent execution
// - REPL Commands: Knowledge and session management commands

pub mod agent;
pub mod commands;

// Re-export key types
pub use agent::RAGConfig;
pub use commands::KnowledgeCommands;
