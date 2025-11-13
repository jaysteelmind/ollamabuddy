// PRD 11 Phase 2: RAG (Retrieval-Augmented Generation) Pipeline
//
// This module implements semantic retrieval, re-ranking, and context
// augmentation for enhancing agent responses with relevant knowledge.
//
// Components:
// - Retrieval Engine: Semantic search across knowledge base
// - Re-ranking: Score and prioritize retrieved documents
// - Context Builder: Assemble augmented context for prompts
// - Pipeline: End-to-end RAG orchestration

pub mod retrieval;
pub mod reranking;
pub mod context;
pub mod pipeline;

// Re-export key types
pub use retrieval::RetrievalEngine;
pub use reranking::ReRanker;
pub use context::ContextBuilder;
pub use pipeline::RAGPipeline;
