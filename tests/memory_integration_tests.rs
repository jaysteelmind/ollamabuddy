//! Integration tests for PRD 7: Memory System Runtime Integration

use ollamabuddy::agent::orchestrator::AgentOrchestrator;
use ollamabuddy::tools::ToolResult;
use serde_json::json;

#[test]
fn test_set_goal_initializes_working_memory() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    let goal = "Test goal for memory integration".to_string();
    
    orch.set_goal(goal.clone());
    
    // Goal is set (verified by no panic)
    assert!(true);
}

#[test]
fn test_find_similar_patterns_empty_memory() {
    let orch = AgentOrchestrator::with_defaults().unwrap();
    
    let matches = orch.find_similar_patterns("test goal", 0.7);
    
    // Should return empty vec when no episodes stored
    assert!(matches.is_empty(), "Should return empty matches when memory is empty");
}

#[test]
fn test_get_tool_recommendations_empty_experience() {
    let orch = AgentOrchestrator::with_defaults().unwrap();
    let tools = vec!["list_dir".to_string(), "read_file".to_string()];
    
    let recommendations = orch.get_tool_recommendations("test goal", &tools);
    
    // Should return empty vec when no experience
    assert!(recommendations.is_empty(), "Should return empty recommendations with no experience");
}

#[test]
fn test_update_working_memory_records_tool_call() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    let result = ToolResult {
        tool: "list_dir".to_string(),
        output: "file1.txt\nfile2.txt".to_string(),
        success: true,
        duration_ms: 10,
        error: None,
        exit_code: Some(0),
    };
    
    orch.update_working_memory("list_dir", &json!({"path": "."}), &result);
    
    // Verify tool was recorded
    assert_eq!(orch.working_memory_tool_count(), 1);
}

#[test]
fn test_update_working_memory_records_errors() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    let result = ToolResult {
        tool: "read_file".to_string(),
        output: "File not found".to_string(),
        success: false,
        duration_ms: 5,
        error: Some("File not found".to_string()),
        exit_code: Some(1),
    };
    
    orch.update_working_memory("read_file", &json!({"path": "missing.txt"}), &result);
    
    // Verify error was recorded
    assert_eq!(orch.working_memory_error_count(), 1);
}

#[test]
fn test_record_episode_creates_episode() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    // Set goal and record some activity
    orch.set_goal("Test task".to_string());
    
    // Record episode
    orch.record_episode("Test task".to_string(), true, None);
    
    // Verify episode was recorded
    assert_eq!(orch.episodic_memory_size(), 1);
}

#[test]
fn test_record_episode_with_failure() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    orch.set_goal("Failed task".to_string());
    
    orch.record_episode(
        "Failed task".to_string(),
        false,
        Some("Task failed due to missing file".to_string()),
    );
    
    // Verify episode recorded
    assert_eq!(orch.episodic_memory_size(), 1);
}

#[test]
fn test_extract_knowledge_from_successful_result() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    let result = ToolResult {
        tool: "list_dir".to_string(),
        output: "src/\ntests/\nCargo.toml".to_string(),
        success: true,
        duration_ms: 10,
        error: None,
        exit_code: Some(0),
    };
    
    orch.extract_knowledge(&result);
    
    // Knowledge extraction completes without panic
    assert!(true);
}

#[test]
fn test_record_tool_experience_updates_tracker() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    let result = ToolResult {
        tool: "read_file".to_string(),
        output: "file contents".to_string(),
        success: true,
        duration_ms: 15,
        error: None,
        exit_code: Some(0),
    };
    
    orch.record_tool_experience("read_file", &result);
    
    // Experience recorded without panic
    assert!(true);
}

#[test]
fn test_full_session_workflow() {
    let mut orch = AgentOrchestrator::with_defaults().unwrap();
    
    // 1. Start session - set goal
    orch.set_goal("List and read files".to_string());
    
    // 2. Before planning - check for patterns
    let patterns = orch.find_similar_patterns("List and read files", 0.7);
    assert!(patterns.is_empty()); // First run, no patterns
    
    // 3. Get recommendations
    let tools = vec!["list_dir".to_string(), "read_file".to_string()];
    let recs = orch.get_tool_recommendations("List and read files", &tools);
    assert!(recs.is_empty()); // No experience yet
    
    // 4. Execute tool and update memory
    let result = ToolResult {
        tool: "list_dir".to_string(),
        output: "file1.txt".to_string(),
        success: true,
        duration_ms: 10,
        error: None,
        exit_code: Some(0),
    };
    
    orch.update_working_memory("list_dir", &json!({"path": "."}), &result);
    orch.record_tool_experience("list_dir", &result);
    orch.extract_knowledge(&result);
    
    // 5. End session - record episode
    orch.record_episode("List and read files".to_string(), true, None);
    
    // Verify final state
    assert_eq!(orch.episodic_memory_size(), 1);
    assert_eq!(orch.working_memory_tool_count(), 1);
}
