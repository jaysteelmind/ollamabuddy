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
}

impl AgentOrchestrator {
    /// Create new agent orchestrator
    pub fn new(config: AgentConfig) -> Result<Self> {
        let client = OllamaClient::with_config(&config.ollama_url, &config.model)?;
        
        Ok(Self {
            state: AgentState::Init,
            memory: MemoryManager::new(),
            compressor: ContextCompressor::new(),
            client,
            parser: JsonParser::new(),
            config,
            iterations: 0,
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
}

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
