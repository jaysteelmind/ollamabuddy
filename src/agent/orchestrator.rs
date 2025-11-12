//! Agent orchestrator - main coordinator
//! 
//! Orchestrates the agent execution loop, coordinating:
//! - State machine transitions
//! - Memory management
//! - Context compression
//! - Streaming communication
//! - Tool execution (interface for PRD 2)

use crate::agent::{AgentState, StateEvent, MemoryManager};
use crate::context::ContextCompressor;
use crate::errors::Result;
use crate::streaming::{OllamaClient, JsonParser};
use crate::types::MemoryEntry;
use crate::planning::AdvancedPlanner;

/// Agent orchestrator configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Ollama base URL
    pub ollama_url: String,
    
    /// Model name
    pub model: String,
    
    /// Maximum iterations before forcing completion
    pub max_iterations: usize,
    
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            model: "qwen2.5:7b-instruct".to_string(),
            max_iterations: 50,
            verbose: false,
        }
    }
}

/// Main agent orchestrator
pub struct AgentOrchestrator {
    /// Current state
    state: AgentState,
    
    /// Memory manager
    memory: MemoryManager,
    
    /// Context compressor
    compressor: ContextCompressor,
    
    /// Ollama client
    client: OllamaClient,
    
    /// JSON parser
    parser: JsonParser,
    
    /// Configuration
    config: AgentConfig,
    
    /// Iteration counter
    iterations: usize,
    
    /// Advanced planning system (PRD 5)
    planner: Option<AdvancedPlanner>,
    
    /// PRD 6: Memory & Learning System
    /// Episodic memory for experience tracking
    episodic_memory: crate::memory::EpisodicMemory,
    
    /// Knowledge graph for semantic understanding
    knowledge_graph: std::sync::Arc<std::sync::RwLock<crate::memory::KnowledgeGraph>>,
    
    /// Pattern matcher for similar problem detection
    pattern_matcher: std::sync::Arc<std::sync::RwLock<crate::memory::PatternMatcher>>,
    
    /// Experience tracker for learning
    experience_tracker: std::sync::Arc<std::sync::RwLock<crate::memory::ExperienceTracker>>,
    
    /// Working memory for active context
    working_memory: crate::memory::WorkingMemory,
}

impl AgentOrchestrator {
    /// Create new agent orchestrator
    pub fn new(config: AgentConfig) -> Result<Self> {
        let client = OllamaClient::with_config(&config.ollama_url, &config.model)?;
        
        
        // Initialize memory system (PRD 6)
        let episodic_memory = crate::memory::EpisodicMemory::new();
        let knowledge_graph = std::sync::Arc::new(std::sync::RwLock::new(
            crate::memory::KnowledgeGraph::new()
        ));
        let pattern_matcher = std::sync::Arc::new(std::sync::RwLock::new(
            crate::memory::PatternMatcher::new(5)
        ));
        let experience_tracker = std::sync::Arc::new(std::sync::RwLock::new(
            crate::memory::ExperienceTracker::new()
        ));
        let working_memory = crate::memory::WorkingMemory::new();
        Ok(Self {
            state: AgentState::Init,
            memory: MemoryManager::new(),
            compressor: ContextCompressor::new(),
            client,
            parser: JsonParser::new(),
            config,
            iterations: 0,
            episodic_memory,
            knowledge_graph,
            pattern_matcher,
            experience_tracker,
            working_memory,
            planner: None,
        })
    }

    /// Create orchestrator with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(AgentConfig::default())
    }

    /// Get current state
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// Get memory manager reference
    pub fn memory(&self) -> &MemoryManager {
        &self.memory
    }

    /// Get mutable memory manager reference
    pub fn memory_mut(&mut self) -> &mut MemoryManager {
        &mut self.memory
    }

    /// Transition to new state
    pub fn transition(&mut self, event: StateEvent) -> Result<()> {
        let new_state = self.state.transition(event)?;
        
        if self.config.verbose {
            eprintln!("[STATE] {:?} -> {:?}", self.state, new_state);
        }
        
        self.state = new_state;
        Ok(())
    }

    /// Add system prompt to memory
    pub fn add_system_prompt(&mut self, prompt: String) {
        self.memory.add(MemoryEntry::SystemPrompt { content: prompt });
    }

    /// Add user goal to memory
    pub fn add_user_goal(&mut self, goal: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.memory.add(MemoryEntry::UserGoal { goal, timestamp });
    }

    /// Check if compression is needed and compress if necessary
    pub fn maybe_compress(&mut self) -> Result<()> {
        let entries = self.memory.to_vec();
        
        if self.compressor.needs_compression(&entries) {
            if self.config.verbose {
                let before_tokens: usize = entries.iter().map(|e| e.estimate_tokens()).sum();
                eprintln!("[COMPRESS] Starting compression: {} tokens", before_tokens);
            }
            
            let compressed = self.compressor.compress(&entries)?;
            
            if self.config.verbose {
                let stats = self.compressor.compression_stats(&entries, &compressed);
                eprintln!(
                    "[COMPRESS] Complete: {} -> {} tokens ({:.1}% reduction)",
                    stats.tokens_before,
                    stats.tokens_after,
                    stats.token_reduction_percent
                );
            }
            
            self.memory.replace_all(compressed)?;
        }
        
        Ok(())
    }

    /// Build prompt from current memory
    pub fn build_prompt(&self) -> String {
        let mut parts = Vec::new();
        
        for entry in self.memory.entries() {
            match entry {
                MemoryEntry::SystemPrompt { content } => {
                    parts.push(format!("SYSTEM: {}", content));
                }
                MemoryEntry::UserGoal { goal, .. } => {
                    parts.push(format!("GOAL: {}", goal));
                }
                MemoryEntry::Plan { steps, reasoning, .. } => {
                    parts.push("PLAN:".to_string());
                    for (i, step) in steps.iter().enumerate() {
                        parts.push(format!("  {}. {}", i + 1, step));
                    }
                    if let Some(r) = reasoning {
                        parts.push(format!("  Reasoning: {}", r));
                    }
                }
                MemoryEntry::ToolCall { tool, args, .. } => {
                    parts.push(format!("TOOL_CALL: {} with {:?}", tool, args));
                }
                MemoryEntry::ToolResult { tool, output, success, .. } => {
                    let status = if *success { "SUCCESS" } else { "FAILED" };
                    parts.push(format!("TOOL_RESULT [{}] {}: {}", status, tool, output));
                }
                MemoryEntry::Question { question, .. } => {
                    parts.push(format!("QUESTION: {}", question));
                }
                MemoryEntry::UserResponse { response, .. } => {
                    parts.push(format!("USER_RESPONSE: {}", response));
                }
                MemoryEntry::FinalResult { result, summary, .. } => {
                    parts.push(format!("FINAL_RESULT: {}", result));
                    if let Some(s) = summary {
                        parts.push(format!("  Summary: {}", s));
                    }
                }
                MemoryEntry::ErrorEntry { message, .. } => {
                    parts.push(format!("ERROR: {}", message));
                }
            }
        }
        
        parts.join("\n\n")
    }

    /// Get total token count in current memory
    pub fn token_count(&self) -> usize {
        self.memory.total_tokens()
    }

    /// Check if max iterations reached
    pub fn max_iterations_reached(&self) -> bool {
        self.iterations >= self.config.max_iterations
    }

    /// Increment iteration counter
    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }

    /// Reset iteration counter
    pub fn reset_iterations(&mut self) {
        self.iterations = 0;
    }

    /// Get Ollama client reference
    pub fn client(&self) -> &OllamaClient {
        &self.client
    }

    /// Get JSON parser reference
    pub fn parser(&self) -> &JsonParser {
        &self.parser
    }

    /// Get mutable JSON parser reference
    pub fn parser_mut(&mut self) -> &mut JsonParser {
        &mut self.parser
    }

    /// Initialize advanced planning for a goal
    ///
    /// PRD 5: Hierarchical decomposition, complexity estimation, strategy generation
    pub fn initialize_planning(&mut self, goal: &str) -> Result<()> {
        let mut planner = AdvancedPlanner::new();
        planner.initialize(goal, &[])?;
        self.planner = Some(planner);
        Ok(())
    }
    
    /// Get reference to the planner
    pub fn planner(&self) -> Option<&AdvancedPlanner> {
        self.planner.as_ref()
    }
    
    /// Get mutable reference to the planner
    pub fn planner_mut(&mut self) -> Option<&mut AdvancedPlanner> {
        self.planner.as_mut()
    }
    
    /// Get current planning progress
    pub fn planning_progress(&self) -> Option<f64> {
        self.planner.as_ref()
            .and_then(|p| p.get_progress())
            .map(|pt| pt.get_metrics().overall_progress)
    }
    
    /// Reset planning for new task
    pub fn reset_planning(&mut self) {
        if let Some(planner) = &mut self.planner {
            planner.reset();
        }
    }

    // ============================================================================
    // PRD 7: Memory Integration Methods
    // ============================================================================
    
    /// Compute context signature for experience tracking
    /// 
    /// Generates a 64-bit hash that uniquely identifies the current execution context
    /// based on goal, recent tools, complexity, and state flags.
    /// 
    /// # Mathematical Properties
    /// - Deterministic: Same context -> same signature
    /// - Collision-resistant: Different contexts -> different signatures (high probability)
    /// - Fast: O(k + t) where k=keywords, t=tools
    /// 
    /// # Performance
    /// Target: < 1ms
    fn compute_context_signature(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash current goal from working memory
        if let Some(goal) = self.working_memory.get_goal() {
            goal.hash(&mut hasher);
        }
        
        // Hash recent tool sequence (last 5 tools)
        for tool_record in self.working_memory.get_recent_tools().iter().take(5) {
            tool_record.tool.hash(&mut hasher);
        }
        
        // Hash complexity bucket (0-10)
        let complexity = self.estimate_current_complexity();
        let complexity_bucket = (complexity * 10.0).floor() as u8;
        complexity_bucket.hash(&mut hasher);
        
        // Hash context flags
        if !self.working_memory.get_known_paths().is_empty() {
            "has_files".hash(&mut hasher);
        }
        
        if !self.working_memory.get_recent_errors().is_empty() {
            "has_errors".hash(&mut hasher);
        }
        
        hasher.finish()
    }
    
    /// Estimate current task complexity based on execution history
    /// 
    /// # Returns
    /// Complexity score in range [0.0, 1.0]
    /// 
    /// # Algorithm
    /// - Base: tool_count / 10 (capped at 1.0)
    /// - Penalty: error_count / 5 (capped at 0.3)
    /// - Total: min(base + penalty, 1.0)
    fn estimate_current_complexity(&self) -> f64 {
        let tool_count = self.working_memory.get_recent_tools().len();
        let error_count = self.working_memory.get_recent_errors().len();
        
        let base_complexity = (tool_count as f64 / 10.0).min(1.0);
        let error_penalty = (error_count as f64 / 5.0).min(0.3);
        
        (base_complexity + error_penalty).min(1.0)
    }
    
    /// Build episode context string for pattern matching
    /// 
    /// Serializes current execution context into a compact string representation
    /// including recent tools and complexity estimate.
    /// 
    /// # Format
    /// "tools:<tool1>,<tool2>,... complexity:<score>"
    /// 
    /// # Performance
    /// O(t) where t = number of recent tools
    fn build_episode_context(&self) -> String {
        let recent_tools: Vec<String> = self.working_memory
            .get_recent_tools()
            .iter()
            .map(|r| r.tool.clone())
            .collect();
        
        format!(
            "tools:{} complexity:{:.2}",
            recent_tools.join(","),
            self.estimate_current_complexity()
        )
    }
    
    /// Serialize current context for episode storage
    /// 
    /// Creates a detailed snapshot of agent state for episodic memory.
    /// 
    /// # Format
    /// "state:<state> tools:<count> errors:<count>"
    fn serialize_context(&self) -> String {
        format!(
            "state:{:?} tools:{} errors:{}",
            self.state,
            self.working_memory.get_recent_tools().len(),
            self.working_memory.get_recent_errors().len()
        )
    }
    
    /// Compute session duration from working memory
    /// 
    /// Estimates total session time based on tool execution history.
    /// 
    /// # Returns
    /// Duration in milliseconds
    fn compute_session_duration(&self) -> u64 {
        // Sum estimated durations from tool results
        // Heuristic: result length / 100 as proxy for execution time
        self.working_memory
            .get_recent_tools()
            .iter()
            .map(|r| {
                // Estimate duration from result length if not tracked
                r.result.len() as u64 / 100
            })
            .sum()
    }
    
    /// Get current planning strategy name
    /// 
    /// Returns the strategy being used by the planner, or "basic" if no planner.
    fn get_current_strategy(&self) -> String {
        if self.planner.is_some() {
            // Advanced planning active
            "advanced".to_string()
        } else {
            "basic".to_string()
        }
    }

    // ============================================================================
    // PRD 7: Public Memory Integration API
    // ============================================================================
    
    /// Set goal in working memory at session start
    /// 
    /// Initializes the working memory with the user's goal for the current session.
    /// This should be called at the beginning of each agent execution.
    /// 
    /// # Arguments
    /// * `goal` - The user's goal or task description
    /// 
    /// # Performance
    /// O(1) - Simple assignment to working memory
    pub fn set_goal(&mut self, goal: String) {
        self.working_memory.set_goal(goal);
        
        if self.config.verbose {
            eprintln!("[MEMORY] Goal set in working memory");
        }
    }

    /// Find similar past episodes using pattern matching
    /// 
    /// Queries the pattern matcher to find episodes similar to the current goal.
    /// Uses LSH-based similarity search with configurable threshold.
    /// 
    /// # Arguments
    /// * `goal` - The current goal to match against
    /// * `threshold` - Minimum similarity score (default: 0.7)
    /// 
    /// # Returns
    /// Vector of pattern matches sorted by similarity (descending)
    /// 
    /// # Performance
    /// O(k × log n) where k=LSH bands, n=episodes
    /// Target: < 20ms for 100 episodes
    pub fn find_similar_patterns(&self, goal: &str, threshold: f64) -> Vec<crate::memory::PatternMatch> {
        let context = self.build_episode_context();
        
        let matcher = match self.pattern_matcher.read() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[MEMORY] Failed to acquire pattern matcher lock: {}", e);
                return Vec::new();
            }
        };
        
        let mut matches = matcher.find_matches(goal, &context, threshold);
        
        // Sort by similarity descending
        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to top 5 matches
        matches.truncate(5);
        
        if self.config.verbose && !matches.is_empty() {
            eprintln!("[MEMORY] Found {} similar patterns:", matches.len());
            for (i, m) in matches.iter().enumerate() {
                eprintln!("  {}. Similarity: {:.2}, Episode: {}", i+1, m.similarity, m.episode.id);
            }
        }
        
        matches
    }

    /// Get experience-based tool recommendations
    /// 
    /// Queries the experience tracker for tool recommendations based on
    /// past successes in similar contexts.
    /// 
    /// # Arguments
    /// * `goal` - The current goal
    /// * `available_tools` - List of available tool names
    /// 
    /// # Returns
    /// Vector of tool recommendations sorted by (success_rate × confidence)
    /// 
    /// # Performance
    /// O(m) where m = number of available tools
    /// Target: < 5ms
    pub fn get_tool_recommendations(
        &self,
        goal: &str,
        available_tools: &[String],
    ) -> Vec<crate::memory::Recommendation> {
        let context_sig = self.compute_context_signature();
        
        let tracker = match self.experience_tracker.read() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[MEMORY] Failed to acquire experience tracker lock: {}", e);
                return Vec::new();
            }
        };
        
        let mut recommendations = tracker.recommend_tools(goal, context_sig, available_tools);
        
        // Filter by confidence threshold (0.5)
        recommendations.retain(|r| r.confidence > 0.5);
        
        // Sort by combined score (success_rate × confidence)
        recommendations.sort_by(|a, b| {
            let score_a = a.success_rate * a.confidence;
            let score_b = b.success_rate * b.confidence;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Limit to top 3
        recommendations.truncate(3);
        
        if self.config.verbose && !recommendations.is_empty() {
            eprintln!("[MEMORY] Tool recommendations:");
            for rec in recommendations.iter() {
                eprintln!(
                    "  - {}: success_rate={:.2}, confidence={:.2} (n={})",
                    rec.tool,
                    rec.success_rate,
                    rec.confidence,
                    rec.sample_size
                );
            }
        }
        
        recommendations
    }

    /// Record tool execution experience for learning
    /// 
    /// Updates the experience tracker with the result of a tool execution,
    /// enabling the system to learn which tools work well in which contexts.
    /// 
    /// # Arguments
    /// * `tool` - Name of the tool that was executed
    /// * `result` - The tool execution result
    /// 
    /// # Performance
    /// O(1) - Direct update to experience tracker
    pub fn record_tool_experience(&mut self, tool: &str, result: &crate::tools::ToolResult) {
        let context_sig = self.compute_context_signature();
        
        let mut tracker = match self.experience_tracker.write() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[MEMORY] Failed to acquire experience tracker lock: {}", e);
                return;
            }
        };
        
        tracker.record_tool_execution(tool, context_sig, result);
        
        if self.config.verbose {
            eprintln!(
                "[MEMORY] Recorded experience: tool={}, success={}",
                tool,
                result.success
            );
        }
    }

    /// Extract knowledge from tool result
    /// 
    /// Analyzes tool execution results and extracts semantic information
    /// (entities, relationships, concepts) into the knowledge graph.
    /// 
    /// # Arguments
    /// * `result` - The tool execution result to analyze
    /// 
    /// # Performance
    /// O(n) where n = size of tool output
    /// Target: < 50ms per result
    pub fn extract_knowledge(&mut self, result: &crate::tools::ToolResult) {
        // Only extract from successful results
        if !result.success {
            return;
        }
        
        let mut graph = match self.knowledge_graph.write() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[MEMORY] Failed to acquire knowledge graph lock: {}", e);
                return;
            }
        };
        
        let nodes_before = graph.node_count();
        let edges_before = graph.edge_count();
        
        // Extract entities and relationships from result
        if let Err(e) = graph.extract_from_result(result) {
            eprintln!("[MEMORY] Knowledge extraction failed: {}", e);
            return;
        }
        
        if self.config.verbose {
            let nodes_added = graph.node_count() - nodes_before;
            let edges_added = graph.edge_count() - edges_before;
            if nodes_added > 0 || edges_added > 0 {
                eprintln!(
                    "[MEMORY] Extracted {} nodes, {} edges (total: {} nodes, {} edges)",
                    nodes_added,
                    edges_added,
                    graph.node_count(),
                    graph.edge_count()
                );
            }
        }
    }

    /// Update working memory with tool execution
    /// 
    /// Records tool execution in working memory and tracks any errors
    /// for context-aware decision making.
    /// 
    /// # Arguments
    /// * `tool` - Name of the executed tool
    /// * `args` - Tool arguments (as JSON value)
    /// * `result` - Tool execution result
    /// 
    /// # Performance
    /// O(1) - Direct updates to working memory
    pub fn update_working_memory(
        &mut self,
        tool: &str,
        args: &serde_json::Value,
        result: &crate::tools::ToolResult,
    ) {
        self.working_memory.record_tool_call(tool, args, result);
        
        // Record errors for failure pattern detection
        if !result.success {
            let error_msg = result.output.clone();
            self.working_memory.record_error(
                error_msg,
                format!("Tool: {}", tool),
                Some(tool.to_string()),
            );
        }
        
        if self.config.verbose {
            eprintln!(
                "[MEMORY] Updated working memory: tool={}, success={}",
                tool,
                result.success
            );
        }
    }

    /// Record complete episode at session end
    /// 
    /// Creates a complete episode record from the execution history and stores it
    /// in episodic memory. Also indexes the episode for pattern matching.
    /// 
    /// # Arguments
    /// * `goal` - The user's original goal
    /// * `success` - Whether the task completed successfully
    /// * `error` - Optional error message if task failed
    /// 
    /// # Performance
    /// O(n) where n = number of actions
    /// Target: < 100ms for typical episodes
    pub fn record_episode(
        &mut self,
        goal: String,
        success: bool,
        error: Option<String>,
    ) {
        use crate::memory::types::{ActionRecord, EpisodeOutcome, EpisodeMetadata};
        
        let context = self.serialize_context();
        let mut episode = crate::memory::Episode::new(goal.clone(), context);
        
        // Collect actions from working memory
        for tool_record in self.working_memory.get_recent_tools() {
            episode.actions.push(ActionRecord {
                tool: tool_record.tool.clone(),
                args: tool_record.args.clone(),
                result: tool_record.result.clone(),
                success: tool_record.success,
            });
        }
        
        // Set outcome
        episode.outcome = if success {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Failure(error.unwrap_or_else(|| "Unknown error".to_string()))
        };
        
        // Compute metadata
        episode.metadata = EpisodeMetadata {
            complexity_score: self.estimate_current_complexity(),
            duration_ms: self.compute_session_duration(),
            tool_count: episode.actions.len(),
            strategy_used: self.get_current_strategy(),
            timestamp: std::time::Instant::now(),
            similarity_hash: 0, // Will be computed by pattern matcher
        };
        
        // Store in episodic memory
        self.episodic_memory.add_episode(episode.clone());
        
        // Index for pattern matching
        let mut matcher = match self.pattern_matcher.write() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[MEMORY] Failed to acquire pattern matcher lock: {}", e);
                return;
            }
        };
        matcher.index_episode(episode.clone());
        
        if self.config.verbose {
            eprintln!(
                "[MEMORY] Episode recorded: {} actions, outcome: {:?}",
                episode.metadata.tool_count,
                episode.outcome
            );
        }
    }

    // ============================================================================
    // PRD 7: Public Accessors for Testing
    // ============================================================================
    
    /// Get episodic memory size (for testing)
    pub fn episodic_memory_size(&self) -> usize {
        self.episodic_memory.len()
    }
    
    /// Get working memory tool count (for testing)
    pub fn working_memory_tool_count(&self) -> usize {
        self.working_memory.get_recent_tools().len()
    }
    
    /// Get working memory error count (for testing)
    pub fn working_memory_error_count(&self) -> usize {
        self.working_memory.get_recent_errors().len()
    }
}

    // ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = AgentOrchestrator::with_defaults();
        assert!(orchestrator.is_ok());
        
        let orch = orchestrator.unwrap();
        assert_eq!(orch.state(), AgentState::Init);
        assert_eq!(orch.memory().len(), 0);
    }

    #[test]
    fn test_state_transition() {
        let mut orch = AgentOrchestrator::with_defaults().unwrap();
        
        assert_eq!(orch.state(), AgentState::Init);
        
        orch.transition(StateEvent::StartSession).unwrap();
        assert_eq!(orch.state(), AgentState::Planning);
    }

    #[test]
    fn test_add_system_prompt() {
        let mut orch = AgentOrchestrator::with_defaults().unwrap();
        
        orch.add_system_prompt("You are a helpful assistant".to_string());
        assert_eq!(orch.memory().len(), 1);
        assert!(orch.memory().system_prompt().is_some());
    }

    #[test]
    fn test_add_user_goal() {
        let mut orch = AgentOrchestrator::with_defaults().unwrap();
        
        orch.add_user_goal("Write a file".to_string());
        assert_eq!(orch.memory().len(), 1);
        assert!(orch.memory().user_goal().is_some());
    }

    #[test]
    fn test_build_prompt() {
        let mut orch = AgentOrchestrator::with_defaults().unwrap();
        
        orch.add_system_prompt("System".to_string());
        orch.add_user_goal("Goal".to_string());
        
        let prompt = orch.build_prompt();
        assert!(prompt.contains("SYSTEM: System"));
        assert!(prompt.contains("GOAL: Goal"));
    }

    #[test]
    fn test_token_counting() {
        let mut orch = AgentOrchestrator::with_defaults().unwrap();
        
        orch.add_system_prompt("a".repeat(400)); // ~100 tokens
        orch.add_user_goal("b".repeat(400));     // ~100 tokens
        
        let count = orch.token_count();
        assert!(count >= 150 && count <= 250); // Rough estimate
    }

    #[test]
    fn test_iteration_tracking() {
        let mut orch = AgentOrchestrator::with_defaults().unwrap();
        
        assert!(!orch.max_iterations_reached());
        
        for _ in 0..50 {
            orch.increment_iteration();
        }
        
        assert!(orch.max_iterations_reached());
        
        orch.reset_iterations();
        assert!(!orch.max_iterations_reached());
    }
}
