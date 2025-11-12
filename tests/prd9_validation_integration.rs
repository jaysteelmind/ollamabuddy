//! PRD 9 Phase 1: Validation Integration Tests
//! 
//! Tests for ValidationOrchestrator integration into main execution loop

use ollamabuddy::validation::ValidationOrchestrator;
use ollamabuddy::tools::types::ToolResult;
use std::time::Duration;

#[test]
fn test_validation_orchestrator_creation() {
    let orchestrator = ValidationOrchestrator::new();
    assert!(orchestrator.validator().config().threshold > 0.0);
}

#[test]
fn test_validation_with_successful_tools() {
    let mut orchestrator = ValidationOrchestrator::new();
    
    let tool_results = vec![
        ToolResult::success(
            "list_dir".to_string(),
            "file1.txt\nfile2.txt".to_string(),
            Duration::from_millis(50),
        ),
        ToolResult::success(
            "read_file".to_string(),
            "File contents here".to_string(),
            Duration::from_millis(30),
        ),
    ];
    
    let expected_outputs = vec!["list files".to_string()];
    
    let result = orchestrator.orchestrate_validation(&tool_results, &expected_outputs);
    
    // Should succeed with successful tool results
    assert!(result.validation.score.overall >= 0.0);
    assert!(result.validation.score.overall <= 1.0);
}

#[test]
fn test_validation_with_failed_tools() {
    let mut orchestrator = ValidationOrchestrator::new();
    
    let tool_results = vec![
        ToolResult::failure(
            "read_file".to_string(),
            "File not found".to_string(),
            Duration::from_millis(10),
        ),
    ];
    
    let expected_outputs = vec!["read file contents".to_string()];
    
    let result = orchestrator.orchestrate_validation(&tool_results, &expected_outputs);
    
    // Failed tools should result in lower score
    assert!(result.validation.score.overall >= 0.0);
    assert!(result.validation.score.overall <= 1.0);
    
    // Should have some failed checks
    assert!(result.validation.failed_checks().len() > 0);
}

#[test]
fn test_validation_with_empty_results() {
    let mut orchestrator = ValidationOrchestrator::new();
    
    let tool_results: Vec<ToolResult> = vec![];
    let expected_outputs = vec!["do something".to_string()];
    
    let result = orchestrator.orchestrate_validation(&tool_results, &expected_outputs);
    
    // Empty results should still produce valid score
    assert!(result.validation.score.overall >= 0.0);
    assert!(result.validation.score.overall <= 1.0);
}

#[test]
fn test_validation_multiple_attempts() {
    let mut orchestrator = ValidationOrchestrator::new();
    
    let tool_results = vec![
        ToolResult::success(
            "test_tool".to_string(),
            "result".to_string(),
            Duration::from_millis(20),
        ),
    ];
    
    let expected_outputs = vec!["expected".to_string()];
    
    // First validation
    let result1 = orchestrator.orchestrate_validation(&tool_results, &expected_outputs);
    assert!(result1.total_attempts >= 1);
    
    // Second validation should work independently
    orchestrator.reset();
    let result2 = orchestrator.orchestrate_validation(&tool_results, &expected_outputs);
    assert!(result2.total_attempts >= 1);
}

#[test]
fn test_validation_score_bounds() {
    let mut orchestrator = ValidationOrchestrator::new();
    
    let successful_results = vec![
        ToolResult::success(
            "tool1".to_string(),
            "output1".to_string(),
            Duration::from_millis(10),
        ),
        ToolResult::success(
            "tool2".to_string(),
            "output2".to_string(),
            Duration::from_millis(15),
        ),
    ];
    
    let expected = vec!["task complete".to_string()];
    
    let result = orchestrator.orchestrate_validation(&successful_results, &expected);
    
    // Score must be between 0.0 and 1.0
    assert!(result.validation.score.overall >= 0.0);
    assert!(result.validation.score.overall <= 1.0);
    
    // Should have check counts
    assert!(result.validation.score.total_checks > 0);
}
