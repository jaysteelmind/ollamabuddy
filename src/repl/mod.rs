//! REPL (Read-Eval-Print Loop) module for interactive terminal experience
//! 
//! Provides interactive session management with context preservation,
//! real-time progress updates, and built-in command system.
//! 
//! Performance targets:
//! - Startup: <1s
//! - Input response: <50ms
//! - Progress updates: 10 FPS
//! - Context building: <20ms

pub mod commands;
pub mod display;
pub mod events;
pub mod input;
pub mod session;

use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;

use crate::repl::commands::{is_command, Command, CommandHandler};
use crate::integration::agent::RAGAgent;
pub use crate::repl::display::DisplayManager;
pub use crate::repl::events::{AgentEvent, EventBus};
use crate::repl::input::InputHandler;
pub use crate::repl::session::{SessionManager, TaskRecord};

/// REPL session coordinator
/// 
/// Manages the interactive read-eval-print loop with:
/// - Input handling (rustyline)
/// - Command processing
/// - Session state management
/// - Display coordination
/// - Event handling
pub struct ReplSession {
    input_handler: InputHandler,
    command_handler: CommandHandler,
    session_manager: SessionManager,
    display_manager: DisplayManager,
    event_bus: EventBus,
}

impl ReplSession {
    /// Create new REPL session
    /// 
    /// Complexity: O(1) initialization
    /// Performance target: <1s startup time
    pub fn new() -> Result<Self> {
        let input_handler = InputHandler::new()?;
        let command_handler = CommandHandler::new();
        let session_manager = SessionManager::new();
        let display_manager = DisplayManager::new();
        let (event_bus, _receiver) = EventBus::new();
        
        Ok(ReplSession {
            input_handler,
            command_handler,
            session_manager,
            display_manager,
            event_bus,
        })
    }
    
    /// Create REPL session with persistent history
    pub fn with_history(history_path: PathBuf) -> Result<Self> {
        let input_handler = InputHandler::with_history(history_path)?;
        let command_handler = CommandHandler::new();
        let session_manager = SessionManager::new();
        let display_manager = DisplayManager::new();
        let (event_bus, _receiver) = EventBus::new();
        
        Ok(ReplSession {
            input_handler,
            command_handler,
            session_manager,
            display_manager,
            event_bus,
        })
    }
    
    /// Show welcome banner
    pub fn show_welcome(&self, version: &str, model: &str) {
        self.display_manager.show_banner(version, model);
    }
    
    /// Read a line of input from user
    /// 
    /// Performance target: <50ms response time
    /// 
    /// Returns:
    /// - Ok(Some(input)) for normal input
    /// - Ok(None) for EOF/exit
    /// - Err for interrupt

    /// Set RAG agent for memory commands
    pub fn set_rag_agent(&mut self, rag_agent: std::sync::Arc<RAGAgent>) {
        self.command_handler = CommandHandler::new().with_rag_agent(rag_agent);
    }

    pub fn read_input(&mut self) -> Result<Option<String>> {
        self.display_manager.show_prompt()?;
        self.input_handler.read_line()
    }
    
    /// Handle user input (command or task request)
    /// 
    /// Returns true if session should continue, false to exit
    pub fn handle_input(&mut self, input: &str) -> Result<bool> {
        // Skip empty input
        if input.trim().is_empty() {
            return Ok(true);
        }
        
        // Check if input is a command
        if is_command(input) {
            let command = self.command_handler.parse(input);
            return self.command_handler.execute(command, &mut self.session_manager);
        }
        
        // Otherwise, it's a task request
        // This will be handled by the main agent orchestrator
        Ok(true)
    }
    
    /// Record a completed task
    pub fn record_task(&mut self, record: TaskRecord) {
        self.session_manager.record_task(record);
    }
    
    /// Get current session context
    pub fn get_context(&self) -> String {
        self.session_manager.build_context()
    }
    
    /// Get session manager (immutable)
    pub fn session(&self) -> &SessionManager {
        &self.session_manager
    }
    
    /// Get session manager (mutable)
    pub fn session_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }
    
    /// Get display manager
    pub fn display(&self) -> &DisplayManager {
        &self.display_manager
    }
    
    /// Get display manager (mutable)
    pub fn display_mut(&mut self) -> &mut DisplayManager {
        &mut self.display_manager
    }
    
    /// Get event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }
    
    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.command_handler.is_verbose()
    }
    
    /// Set verbose mode
    pub fn set_verbose(&mut self, enable: bool) {
        self.command_handler.set_verbose(enable);
    }
    
    /// Save session state
    pub fn save(&mut self) -> Result<()> {
        self.input_handler.save_history()?;
        Ok(())
    }
    
    /// Get task count
    pub fn task_count(&self) -> usize {
        self.session_manager.task_count()
    }
    
    /// Check if session has context
    pub fn has_context(&self) -> bool {
        self.session_manager.has_context()
    }
}

impl Default for ReplSession {
    fn default() -> Self {
        Self::new().expect("Failed to create REPL session")
    }
}

/// Configuration for REPL mode
#[derive(Debug, Clone)]
pub struct ReplConfig {
    pub enabled: bool,
    pub history_file: Option<PathBuf>,
    pub show_progress: bool,
    pub auto_save: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        ReplConfig {
            enabled: true,
            history_file: None,
            show_progress: true,
            auto_save: true,
        }
    }
}

impl ReplConfig {
    /// Create config with history file
    pub fn with_history(path: PathBuf) -> Self {
        ReplConfig {
            history_file: Some(path),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_session_creation() {
        let session = ReplSession::new();
        assert!(session.is_ok());
    }

    #[test]
    fn test_repl_session_default() {
        let session = ReplSession::default();
        assert_eq!(session.task_count(), 0);
        assert!(!session.has_context());
    }

    #[test]
    fn test_handle_command() {
        let mut session = ReplSession::new().unwrap();
        
        // Test help command
        let result = session.handle_input("/help").unwrap();
        assert!(result); // Should continue
    }

    #[test]
    fn test_handle_exit_command() {
        let mut session = ReplSession::new().unwrap();
        
        let result = session.handle_input("/exit").unwrap();
        assert!(!result); // Should exit
    }

    #[test]
    fn test_handle_empty_input() {
        let mut session = ReplSession::new().unwrap();
        
        let result = session.handle_input("").unwrap();
        assert!(result); // Should continue
        
        let result = session.handle_input("   ").unwrap();
        assert!(result); // Should continue
    }

    #[test]
    fn test_record_task() {
        let mut session = ReplSession::new().unwrap();
        
        let record = TaskRecord {
            task: "test task".to_string(),
            result: "success".to_string(),
            success: true,
            duration_ms: 100,
            timestamp: 1234567890,
            files_modified: vec![],
        };
        
        session.record_task(record);
        
        assert_eq!(session.task_count(), 1);
        assert!(session.has_context());
    }

    #[test]
    fn test_get_context() {
        let mut session = ReplSession::new().unwrap();
        
        // Empty context
        let context = session.get_context();
        assert_eq!(context, "");
        
        // Add task
        let record = TaskRecord {
            task: "test task".to_string(),
            result: "success".to_string(),
            success: true,
            duration_ms: 100,
            timestamp: 1234567890,
            files_modified: vec![],
        };
        session.record_task(record);
        
        // Should have context now
        let context = session.get_context();
        assert!(!context.is_empty());
        assert!(context.contains("test task"));
    }

    #[test]
    fn test_verbose_mode() {
        let mut session = ReplSession::new().unwrap();
        
        assert!(!session.is_verbose());
        
        session.set_verbose(true);
        assert!(session.is_verbose());
        
        session.set_verbose(false);
        assert!(!session.is_verbose());
    }

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
    }

    #[test]
    fn test_session_startup_performance() {
        let start = Instant::now();
        let _session = ReplSession::new().unwrap();
        let elapsed = start.elapsed();
        
        // Should be well under 1s target
        assert!(elapsed.as_secs() < 1, "Startup too slow: {:?}", elapsed);
    }

    #[test]
    fn test_session_managers_access() {
        let mut session = ReplSession::new().unwrap();
        
        // Test immutable access
        let _session_ref = session.session();
        let _display_ref = session.display();
        let _event_bus_ref = session.event_bus();
        
        // Test mutable access
        let _session_mut = session.session_mut();
        let _display_mut = session.display_mut();
    }
}
