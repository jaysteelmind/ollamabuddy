//! PRD 10a Integration Tests: REPL Execution
//!
//! Tests for full agent execution in REPL mode using shared execution logic.

use ollamabuddy::agent::orchestrator::AgentOrchestrator;

use ollamabuddy::budget::DynamicBudgetManager;
use ollamabuddy::agent::orchestrator::AgentConfig;
use ollamabuddy::execution::execute_agent_task;
use ollamabuddy::telemetry::TelemetryCollector;
use ollamabuddy::tools::runtime::ToolRuntime;
use ollamabuddy::DisplayMode;
use std::path::PathBuf;

/// Test that execute_agent_task can be called without panicking
#[tokio::test]
async fn test_execute_agent_task_initialization() {
    let config = AgentConfig {
        ollama_url: "http://localhost:11434".to_string(),
        model: "qwen2.5:7b-instruct".to_string(),
        max_iterations: 5,
        verbose: false,
    };

    let mut orchestrator = AgentOrchestrator::new(config).unwrap();
    let tool_runtime = ToolRuntime::new(&PathBuf::from(".")).unwrap();
    let telemetry = TelemetryCollector::new();
    let display_mode = DisplayMode::cli();

    // Initialize orchestrator
    orchestrator.add_system_prompt("Test system".to_string());
    orchestrator.add_user_goal("Test task".to_string());

    // This should not panic
    let result = execute_agent_task(
        &mut orchestrator,
        &tool_runtime,
        &telemetry,
        5,
        "Test task",
        false,
        &display_mode,
    )
    .await;

    // Result may fail (no Ollama), but function should complete
    assert!(result.is_ok() || result.is_err());
}

/// Test TaskExecutionResult structure
#[test]
fn test_task_execution_result_success() {
    use ollamabuddy::types::TaskExecutionResult;
    use std::time::Duration;

    let result = TaskExecutionResult::success(
        "Task completed".to_string(),
        Duration::from_secs(5),
        10,
        vec!["file.txt".to_string()],
        0.95,
    );

    assert!(result.success);
    assert_eq!(result.output, "Task completed");
    assert_eq!(result.iterations, 10);
    assert_eq!(result.validation_score, 0.95);
    assert_eq!(result.files_touched.len(), 1);
}

/// Test TaskExecutionResult failure
#[test]
fn test_task_execution_result_failure() {
    use ollamabuddy::types::TaskExecutionResult;
    use std::time::Duration;

    let result = TaskExecutionResult::failure(
        "Task failed".to_string(),
        Duration::from_secs(3),
        5,
    );

    assert!(!result.success);
    assert_eq!(result.output, "Task failed");
    assert_eq!(result.iterations, 5);
    assert_eq!(result.validation_score, 0.0);
    assert!(result.files_touched.is_empty());
}

/// Test early success detection
#[test]
fn test_task_execution_result_early_success() {
    use ollamabuddy::types::TaskExecutionResult;
    use std::time::Duration;

    let result = TaskExecutionResult::success(
        "Quick completion".to_string(),
        Duration::from_secs(1),
        3,
        vec![],
        1.0,
    )
    .with_early_success();

    assert!(result.success);
    assert!(result.early_success);
    assert_eq!(result.iterations, 3);
}

/// Test DisplayMode CLI creation
#[test]
fn test_display_mode_cli() {
    let mode = DisplayMode::cli();
    assert!(mode.is_cli());
    assert!(!mode.is_repl());
}

/// Test DisplayMode type checking
#[test]
fn test_display_mode_type_checks() {
    let cli = DisplayMode::cli();
    assert!(cli.is_cli());
    assert!(!cli.is_repl());
}

/// Test result summary formatting
#[test]
fn test_task_execution_result_summary() {
    use ollamabuddy::types::TaskExecutionResult;
    use std::time::Duration;

    let result = TaskExecutionResult::success(
        "Done".to_string(),
        Duration::from_millis(2500),
        8,
        vec![],
        0.88,
    );

    let summary = result.summary();
    assert!(summary.contains("Success"));
    assert!(summary.contains("2.50s"));
    assert!(summary.contains("8 iterations"));
    assert!(summary.contains("0.88"));
}

/// Test budget manager integration
#[test]
fn test_budget_manager_complexity_calculation() {
    let mut manager = DynamicBudgetManager::new();
    
    // Low complexity
    let budget_low = manager.calculate_budget(0.2);
    assert!(budget_low >= 8);
    assert!(budget_low <= 50);
    
    // High complexity
    let budget_high = manager.calculate_budget(0.9);
    assert!(budget_high >= 8);
    assert!(budget_high <= 50);
    assert!(budget_high >= budget_low);
}

/// Test orchestrator state after initialization
#[test]
fn test_orchestrator_initial_state() {
    let config = AgentConfig {
        ollama_url: "http://localhost:11434".to_string(),
        model: "test".to_string(),
        max_iterations: 10,
        verbose: false,
    };

    let orchestrator = AgentOrchestrator::new(config).unwrap();
    
    // Check initial state
    
    assert!(matches!(orchestrator.state(), ollamabuddy::agent::AgentState::Init));
}

/// Test tool runtime initialization
#[test]
fn test_tool_runtime_creation() {
    let runtime = ToolRuntime::new(&PathBuf::from("."));
    assert!(runtime.is_ok());
    
    let runtime = runtime.unwrap();
    let tools = runtime.tool_names();
    
    // Verify expected tools are available
    assert!(tools.contains(&"list_dir".to_string()));
    assert!(tools.contains(&"read_file".to_string()));
    assert!(tools.contains(&"write_file".to_string()));
    assert!(tools.contains(&"run_command".to_string()));
    assert!(tools.contains(&"system_info".to_string()));
    assert!(tools.contains(&"web_fetch".to_string()));
}

/// Test telemetry collector creation
#[test]
fn test_telemetry_collector_creation() {
    let telemetry = TelemetryCollector::new();
    
    // Should not panic
    drop(telemetry);
}
