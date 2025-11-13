//! PRD 10 Integration Tests - REPL Mode
//! 
//! Comprehensive tests for interactive terminal experience

use ollamabuddy::repl::{
    ReplSession, ReplConfig,
    commands::{Command, CommandHandler, is_command},
    session::{SessionManager, TaskRecord},
    display::DisplayManager,
    events::{EventBus, AgentEvent, MessageLevel},
    input::InputHandler,
};
use std::path::PathBuf;
use tempfile::TempDir;

// Session Manager Tests

#[test]
fn test_session_manager_initialization() {
    let session = SessionManager::new();
    assert_eq!(session.task_count(), 0);
    assert_eq!(session.history_len(), 0);
    assert!(!session.has_context());
}

#[test]
fn test_session_task_recording() {
    let mut session = SessionManager::new();
    
    let record = TaskRecord {
        task: "create test file".to_string(),
        result: "file created".to_string(),
        success: true,
        duration_ms: 150,
        timestamp: 1700000000,
        files_modified: vec![PathBuf::from("test.txt")],
    };
    
    session.record_task(record);
    
    assert_eq!(session.task_count(), 1);
    assert_eq!(session.history_len(), 1);
    assert!(session.has_context());
    assert_eq!(session.get_tracked_files().len(), 1);
}

#[test]
fn test_session_context_building() {
    let mut session = SessionManager::new();
    
    for i in 0..3 {
        let record = TaskRecord {
            task: format!("task {}", i),
            result: "success".to_string(),
            success: true,
            duration_ms: 100,
            timestamp: 1700000000 + i,
            files_modified: vec![],
        };
        session.record_task(record);
    }
    
    let context = session.build_context();
    assert!(!context.is_empty());
    assert!(context.contains("task 0"));
    assert!(context.contains("task 1"));
    assert!(context.contains("task 2"));
}

#[test]
fn test_session_history_bounded() {
    let mut session = SessionManager::new();
    
    // Add 1100 tasks (exceeds max of 1000)
    for i in 0..1100 {
        let record = TaskRecord {
            task: format!("task {}", i),
            result: "success".to_string(),
            success: true,
            duration_ms: 100,
            timestamp: 1700000000,
            files_modified: vec![],
        };
        session.record_task(record);
    }
    
    assert_eq!(session.history_len(), 1000); // Bounded
    assert_eq!(session.task_count(), 1100);  // Count is accurate
}

#[test]
fn test_session_reset() {
    let mut session = SessionManager::new();
    
    let record = TaskRecord {
        task: "test".to_string(),
        result: "success".to_string(),
        success: true,
        duration_ms: 100,
        timestamp: 1700000000,
        files_modified: vec![],
    };
    
    session.record_task(record);
    assert!(session.has_context());
    
    session.reset();
    assert!(!session.has_context());
    assert_eq!(session.task_count(), 0);
}

// Command Handler Tests

#[test]
fn test_command_parsing_help() {
    let handler = CommandHandler::new();
    assert_eq!(handler.parse("/help"), Command::Help);
    assert_eq!(handler.parse("/h"), Command::Help);
}

#[test]
fn test_command_parsing_all_commands() {
    let handler = CommandHandler::new();
    
    assert_eq!(handler.parse("/exit"), Command::Exit);
    assert_eq!(handler.parse("/quit"), Command::Exit);
    assert_eq!(handler.parse("/q"), Command::Exit);
    assert_eq!(handler.parse("/status"), Command::Status);
    assert_eq!(handler.parse("/context"), Command::Context);
    assert_eq!(handler.parse("/ctx"), Command::Context);
    assert_eq!(handler.parse("/reset"), Command::Reset);
    assert_eq!(handler.parse("/clear"), Command::Clear);
    assert_eq!(handler.parse("/cls"), Command::Clear);
    assert_eq!(handler.parse("/files"), Command::Files);
}

#[test]
fn test_command_parsing_with_args() {
    let handler = CommandHandler::new();
    
    match handler.parse("/history 5") {
        Command::History { limit: Some(5) } => {},
        _ => panic!("Expected History command with limit 5"),
    }
    
    match handler.parse("/verbose on") {
        Command::Verbose { enable: true } => {},
        _ => panic!("Expected Verbose command with enable true"),
    }
}

#[test]
fn test_is_command_function() {
    assert!(is_command("/help"));
    assert!(is_command("  /exit  "));
    assert!(!is_command("help"));
    assert!(!is_command("create a file"));
}

#[test]
fn test_command_execution() {
    let mut handler = CommandHandler::new();
    let mut session = SessionManager::new();
    
    // Test exit returns false
    let result = handler.execute(Command::Exit, &mut session).unwrap();
    assert!(!result);
    
    // Test help returns true
    let result = handler.execute(Command::Help, &mut session).unwrap();
    assert!(result);
    
    // Test status returns true
    let result = handler.execute(Command::Status, &mut session).unwrap();
    assert!(result);
}

#[test]
fn test_verbose_toggle() {
    let mut handler = CommandHandler::new();
    let mut session = SessionManager::new();
    
    assert!(!handler.is_verbose());
    
    handler.execute(Command::Verbose { enable: true }, &mut session).unwrap();
    assert!(handler.is_verbose());
    
    handler.execute(Command::Verbose { enable: false }, &mut session).unwrap();
    assert!(!handler.is_verbose());
}

// Event Bus Tests

#[tokio::test]
async fn test_event_bus_creation() {
    let (bus, _receiver) = EventBus::new();
    assert!(bus.clone_sender().capacity() > 0);
}

#[tokio::test]
async fn test_event_emission_and_reception() {
    let (bus, mut receiver) = EventBus::new();
    
    bus.emit(AgentEvent::PlanningStarted {
        task: "test task".to_string()
    }).await;
    
    let event = receiver.recv().await.unwrap();
    match event {
        AgentEvent::PlanningStarted { task } => {
            assert_eq!(task, "test task");
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_multiple_event_types() {
    let (bus, mut receiver) = EventBus::new();
    
    bus.emit(AgentEvent::PlanningStarted { task: "t".into() }).await;
    bus.emit(AgentEvent::ExecutionStarted { tool: "tool".into() }).await;
    bus.emit(AgentEvent::ValidationStarted).await;
    
    let _e1 = receiver.recv().await.unwrap();
    let _e2 = receiver.recv().await.unwrap();
    let _e3 = receiver.recv().await.unwrap();
}

#[tokio::test]
async fn test_event_bus_clone() {
    let (bus1, mut receiver) = EventBus::new();
    let bus2 = bus1.clone();
    
    bus1.emit(AgentEvent::PlanningStarted { task: "1".into() }).await;
    bus2.emit(AgentEvent::PlanningStarted { task: "2".into() }).await;
    
    let _e1 = receiver.recv().await.unwrap();
    let _e2 = receiver.recv().await.unwrap();
}

#[test]
fn test_message_level_display() {
    assert_eq!(format!("{}", MessageLevel::Info), "INFO");
    assert_eq!(format!("{}", MessageLevel::Warning), "WARN");
    assert_eq!(format!("{}", MessageLevel::Error), "ERROR");
    assert_eq!(format!("{}", MessageLevel::Debug), "DEBUG");
}

// Display Manager Tests

#[test]
fn test_display_manager_creation() {
    let manager = DisplayManager::new();
    // Just verify it creates without panic
    drop(manager);
}

#[test]
fn test_display_manager_progress_bars() {
    let mut manager = DisplayManager::new();
    
    let pb = manager.start_planning("test task");
    manager.update_progress(&pb, 0.5, Some("halfway"));
    assert_eq!(pb.position(), 50);
    pb.finish_and_clear();
    
    let pb = manager.start_execution("test_tool");
    manager.update_progress(&pb, 0.8, None);
    assert_eq!(pb.position(), 80);
    pb.finish_and_clear();
}

#[test]
fn test_display_manager_stage_transitions() {
    let mut manager = DisplayManager::new();
    
    let _pb1 = manager.start_planning("task");
    let _pb2 = manager.start_execution("tool");
    let _pb3 = manager.start_validation();
    
    manager.finish_current();
}

// Input Handler Tests

#[test]
fn test_input_handler_creation() {
    let handler = InputHandler::new();
    assert!(handler.is_ok());
}

#[test]
fn test_input_handler_with_history_file() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history");
    
    let handler = InputHandler::with_history(history_path);
    assert!(handler.is_ok());
}

#[test]
fn test_input_handler_prompt_customization() {
    let mut handler = InputHandler::new().unwrap();
    
    handler.set_prompt("custom> ".to_string());
    // Prompt is private, so we just verify no panic
}

// REPL Session Tests

#[test]
fn test_repl_session_creation() {
    let session = ReplSession::new();
    assert!(session.is_ok());
}

#[test]
fn test_repl_session_with_history() {
    let temp_dir = TempDir::new().unwrap();
    let history_path = temp_dir.path().join("history");
    
    let session = ReplSession::with_history(history_path);
    assert!(session.is_ok());
}

#[test]
fn test_repl_session_task_recording() {
    let mut session = ReplSession::new().unwrap();
    
    let record = TaskRecord {
        task: "test".to_string(),
        result: "success".to_string(),
        success: true,
        duration_ms: 100,
        timestamp: 1700000000,
        files_modified: vec![],
    };
    
    session.record_task(record);
    assert_eq!(session.task_count(), 1);
}

#[test]
fn test_repl_session_context() {
    let mut session = ReplSession::new().unwrap();
    
    assert!(!session.has_context());
    assert_eq!(session.get_context(), "");
    
    let record = TaskRecord {
        task: "test".to_string(),
        result: "success".to_string(),
        success: true,
        duration_ms: 100,
        timestamp: 1700000000,
        files_modified: vec![],
    };
    
    session.record_task(record);
    assert!(session.has_context());
    assert!(!session.get_context().is_empty());
}

#[test]
fn test_repl_session_command_handling() {
    let mut session = ReplSession::new().unwrap();
    
    // Test help command
    let result = session.handle_input("/help").unwrap();
    assert!(result); // Should continue
    
    // Test exit command
    let result = session.handle_input("/exit").unwrap();
    assert!(!result); // Should exit
}

#[test]
fn test_repl_session_verbose_mode() {
    let mut session = ReplSession::new().unwrap();
    
    assert!(!session.is_verbose());
    session.set_verbose(true);
    assert!(session.is_verbose());
}

// REPL Config Tests

#[test]
fn test_repl_config_default() {
    let config = ReplConfig::default();
    
    assert!(config.enabled);
    assert!(config.show_progress);
    assert!(config.auto_save);
    assert!(config.history_file.is_none());
}

#[test]
fn test_repl_config_with_history() {
    let path = PathBuf::from("/tmp/history");
    let config = ReplConfig::with_history(path.clone());
    
    assert_eq!(config.history_file, Some(path));
    assert!(config.enabled);
}

// Performance Tests

#[test]
fn test_session_startup_performance() {
    let start = std::time::Instant::now();
    let _session = ReplSession::new().unwrap();
    let elapsed = start.elapsed();
    
    assert!(elapsed.as_millis() < 1000, "Startup too slow: {:?}", elapsed);
}

#[test]
fn test_context_building_performance() {
    let mut session = SessionManager::new();
    
    for i in 0..5 {
        let record = TaskRecord {
            task: format!("task {}", i),
            result: "success".to_string(),
            success: true,
            duration_ms: 100,
            timestamp: 1700000000,
            files_modified: vec![],
        };
        session.record_task(record);
    }
    
    let start = std::time::Instant::now();
    let _context = session.build_context();
    let elapsed = start.elapsed();
    
    assert!(elapsed.as_millis() < 20, "Context building too slow: {:?}", elapsed);
}

#[test]
fn test_command_execution_performance() {
    let mut handler = CommandHandler::new();
    let mut session = SessionManager::new();
    
    let start = std::time::Instant::now();
    handler.execute(Command::Status, &mut session).unwrap();
    let elapsed = start.elapsed();
    
    assert!(elapsed.as_millis() < 100, "Command execution too slow: {:?}", elapsed);
}
