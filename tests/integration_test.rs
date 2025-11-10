//! Integration tests for OllamaBuddy v0.2.1
//! 
//! Tests the full agent execution flow without requiring Ollama running.

use ollamabuddy::{
    agent::{AgentOrchestrator, orchestrator::AgentConfig},
    tools::ToolRuntime,
    bootstrap::Bootstrap,
};

#[tokio::test]
async fn test_component_initialization() {
    // Test that all components can be initialized
    let config = AgentConfig {
        ollama_url: "http://127.0.0.1:11434".to_string(),
        model: "qwen2.5:7b-instruct".to_string(),
        max_iterations: 10,
        verbose: false,
    };
    
    let orchestrator = AgentOrchestrator::new(config);
    assert!(orchestrator.is_ok());
    
    let tool_runtime = ToolRuntime::new(".");
    assert!(tool_runtime.is_ok());
}

#[tokio::test]
async fn test_bootstrap_creation() {
    let bootstrap = Bootstrap::new(
        "127.0.0.1".to_string(),
        11434,
        "qwen2.5:7b-instruct".to_string(),
    );
    
    // Bootstrap should always succeed in creation
    // (actual Ollama check happens on method calls)
    assert_eq!(bootstrap.model_tag, "qwen2.5:7b-instruct");
}

#[test]
fn test_tool_registry() {
    let runtime = ToolRuntime::new(".").unwrap();
    
    // Check that all 6 tools are registered
    let tools = runtime.tool_names();
    assert!(tools.len() >= 6, "Expected at least 6 tools, got {}", tools.len());
    
    // Check specific tools
    assert!(runtime.has_tool("read_file"));
    assert!(runtime.has_tool("write_file"));
    assert!(runtime.has_tool("list_dir"));
    assert!(runtime.has_tool("run_command"));
    assert!(runtime.has_tool("system_info"));
    assert!(runtime.has_tool("web_fetch"));
}

#[test]
fn test_agent_state_machine() {
    use ollamabuddy::agent::{AgentState, StateEvent};
    
    // Test initial state
    let mut state = AgentState::Init;
    
    // Test valid transitions
    state = state.transition(StateEvent::StartSession).unwrap();
    assert_eq!(state, AgentState::Planning);
}
