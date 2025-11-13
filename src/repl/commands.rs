//! Command handler for REPL built-in commands
//! 
//! Provides 9 built-in commands for session management and introspection
//! Performance target: <100ms command execution

use anyhow::Result;
use colored::*;
use crate::repl::session::SessionManager;

/// REPL command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    History { limit: Option<usize> },
    Status,
    Context,
    Reset,
    Exit,
    Verbose { enable: bool },
    Clear,
    Files,
    Unknown { input: String },
}

/// Command handler for parsing and executing REPL commands
pub struct CommandHandler {
    verbose: bool,
}

impl CommandHandler {
    /// Create new command handler
    pub fn new() -> Self {
        CommandHandler { verbose: false }
    }
    
    /// Parse input string into a command
    /// 
    /// Complexity: O(1) string matching
    pub fn parse(&self, input: &str) -> Command {
        let trimmed = input.trim();
        
        // Not a command if doesn't start with /
        if !trimmed.starts_with('/') {
            return Command::Unknown { input: input.to_string() };
        }
        
        let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();
        if parts.is_empty() {
            return Command::Unknown { input: input.to_string() };
        }
        
        match parts[0].to_lowercase().as_str() {
            "help" | "h" => Command::Help,
            "exit" | "quit" | "q" => Command::Exit,
            "history" => {
                let limit = parts.get(1).and_then(|s| s.parse().ok());
                Command::History { limit }
            }
            "status" => Command::Status,
            "context" | "ctx" => Command::Context,
            "reset" => Command::Reset,
            "verbose" => {
                let enable = parts.get(1)
                    .map(|s| s.to_lowercase() == "on" || s == &"1" || s == &"true")
                    .unwrap_or(true);
                Command::Verbose { enable }
            }
            "clear" | "cls" => Command::Clear,
            "files" => Command::Files,
            _ => Command::Unknown { input: input.to_string() },
        }
    }
    
    /// Execute a command
    /// 
    /// Returns true if REPL should continue, false if should exit
    pub fn execute(&mut self, command: Command, session: &mut SessionManager) -> Result<bool> {
        match command {
            Command::Help => {
                self.show_help();
                Ok(true)
            }
            Command::Exit => {
                println!("{}", "Goodbye!".green());
                Ok(false)
            }
            Command::History { limit } => {
                self.show_history(session, limit.unwrap_or(10));
                Ok(true)
            }
            Command::Status => {
                self.show_status(session);
                Ok(true)
            }
            Command::Context => {
                self.show_context(session);
                Ok(true)
            }
            Command::Reset => {
                session.reset();
                println!("{}", "Session reset. Context cleared.".yellow());
                Ok(true)
            }
            Command::Verbose { enable } => {
                self.verbose = enable;
                let status = if enable { "enabled" } else { "disabled" };
                println!("{}", format!("Verbose mode {}", status).cyan());
                Ok(true)
            }
            Command::Clear => {
                print!("\x1B[2J\x1B[1;1H"); // ANSI escape codes to clear screen
                Ok(true)
            }
            Command::Files => {
                self.show_files(session);
                Ok(true)
            }
            Command::Unknown { input } => {
                println!("{}", format!("Unknown command: {}", input).red());
                println!("Type {} for available commands", "/help".cyan());
                Ok(true)
            }
        }
    }
    
    /// Display help information
    fn show_help(&self) {
        println!("\n{}", "Available Commands:".bold().cyan());
        println!("{}", "=".repeat(60).cyan());
        
        let commands = vec![
            ("/help, /h", "Show this help message"),
            ("/history [n]", "Show last n tasks (default: 10)"),
            ("/status", "Show session status and statistics"),
            ("/context, /ctx", "Show current context summary"),
            ("/files", "Show tracked files in session"),
            ("/reset", "Clear session context and history"),
            ("/verbose [on|off]", "Toggle verbose output"),
            ("/clear, /cls", "Clear screen"),
            ("/exit, /quit, /q", "Exit REPL"),
        ];
        
        for (cmd, desc) in commands {
            println!("  {:<20} {}", cmd.green(), desc);
        }
        
        println!("\n{}", "Usage:".bold());
        println!("  - Type your task request directly (no / prefix)");
        println!("  - Use {} for command history", "UP/DOWN arrows".cyan());
        println!("  - Press {} or {} to exit", "Ctrl-D".cyan(), "/exit".cyan());
        println!();
    }
    
    /// Display task history
    fn show_history(&self, session: &SessionManager, limit: usize) {
        let history = session.get_history(limit);
        
        if history.is_empty() {
            println!("{}", "No tasks in history yet.".yellow());
            return;
        }
        
        println!("\n{}", format!("Task History (last {}):", history.len()).bold().cyan());
        println!("{}", "=".repeat(60).cyan());
        
        for (i, record) in history.iter().enumerate() {
            let index = history.len() - i;
            let status_icon = if record.success { "✓".green() } else { "✗".red() };
            let duration = format!("({}ms)", record.duration_ms).dimmed();
            
            println!("  {}. {} {} {}", 
                index.to_string().cyan(),
                status_icon,
                record.task,
                duration
            );
            
            if self.verbose && !record.result.is_empty() {
                println!("     Result: {}", record.result.dimmed());
            }
        }
        println!();
    }
    
    /// Display session status
    fn show_status(&self, session: &SessionManager) {
        println!("\n{}", "Session Status:".bold().cyan());
        println!("{}", "=".repeat(60).cyan());
        
        let duration = session.session_duration();
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;
        
        let duration_str = if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        };
        
        println!("  Total Tasks:      {}", session.task_count().to_string().green());
        println!("  History Size:     {}", session.history_len().to_string().green());
        println!("  Tracked Files:    {}", session.get_tracked_files().len().to_string().green());
        println!("  Session Duration: {}", duration_str.green());
        println!("  Verbose Mode:     {}", if self.verbose { "On".green() } else { "Off".red() });
        println!("  Has Context:      {}", if session.has_context() { "Yes".green() } else { "No".red() });
        println!();
    }
    
    /// Display current context
    fn show_context(&self, session: &SessionManager) {
        let context = session.build_context();
        
        if context.is_empty() {
            println!("{}", "No context available yet.".yellow());
            return;
        }
        
        println!("\n{}", "Current Context:".bold().cyan());
        println!("{}", "=".repeat(60).cyan());
        println!("{}", context);
    }
    
    /// Display tracked files
    fn show_files(&self, session: &SessionManager) {
        let files = session.get_tracked_files();
        
        if files.is_empty() {
            println!("{}", "No files tracked yet.".yellow());
            return;
        }
        
        println!("\n{}", format!("Tracked Files ({}):", files.len()).bold().cyan());
        println!("{}", "=".repeat(60).cyan());
        
        for (i, file) in files.iter().enumerate() {
            println!("  {}. {}", (i + 1).to_string().cyan(), file.display());
        }
        println!();
    }
    
    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
    
    /// Set verbose mode
    pub fn set_verbose(&mut self, enable: bool) {
        self.verbose = enable;
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if input is a command (starts with /)
pub fn is_command(input: &str) -> bool {
    input.trim().starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repl::session::TaskRecord;
    use std::path::PathBuf;

    #[test]
    fn test_is_command() {
        assert!(is_command("/help"));
        assert!(is_command(" /help"));
        assert!(!is_command("help"));
        assert!(!is_command("create a file"));
    }

    #[test]
    fn test_parse_help() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/help"), Command::Help);
        assert_eq!(handler.parse("/h"), Command::Help);
    }

    #[test]
    fn test_parse_exit() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/exit"), Command::Exit);
        assert_eq!(handler.parse("/quit"), Command::Exit);
        assert_eq!(handler.parse("/q"), Command::Exit);
    }

    #[test]
    fn test_parse_history() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/history"), Command::History { limit: None });
        assert_eq!(handler.parse("/history 5"), Command::History { limit: Some(5) });
    }

    #[test]
    fn test_parse_status() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/status"), Command::Status);
    }

    #[test]
    fn test_parse_context() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/context"), Command::Context);
        assert_eq!(handler.parse("/ctx"), Command::Context);
    }

    #[test]
    fn test_parse_reset() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/reset"), Command::Reset);
    }

    #[test]
    fn test_parse_verbose() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/verbose"), Command::Verbose { enable: true });
        assert_eq!(handler.parse("/verbose on"), Command::Verbose { enable: true });
        assert_eq!(handler.parse("/verbose off"), Command::Verbose { enable: false });
    }

    #[test]
    fn test_parse_clear() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/clear"), Command::Clear);
        assert_eq!(handler.parse("/cls"), Command::Clear);
    }

    #[test]
    fn test_parse_files() {
        let handler = CommandHandler::new();
        assert_eq!(handler.parse("/files"), Command::Files);
    }

    #[test]
    fn test_parse_unknown() {
        let handler = CommandHandler::new();
        match handler.parse("/unknown") {
            Command::Unknown { input } => assert!(input.contains("unknown")),
            _ => panic!("Expected Unknown command"),
        }
    }

    #[test]
    fn test_parse_non_command() {
        let handler = CommandHandler::new();
        match handler.parse("create a file") {
            Command::Unknown { .. } => {},
            _ => panic!("Expected Unknown command for non-command input"),
        }
    }

    #[test]
    fn test_execute_exit() {
        let mut handler = CommandHandler::new();
        let mut session = SessionManager::new();
        
        let result = handler.execute(Command::Exit, &mut session).unwrap();
        assert!(!result); // Should return false to exit REPL
    }

    #[test]
    fn test_execute_help() {
        let mut handler = CommandHandler::new();
        let mut session = SessionManager::new();
        
        let result = handler.execute(Command::Help, &mut session).unwrap();
        assert!(result); // Should continue
    }

    #[test]
    fn test_execute_reset() {
        let mut handler = CommandHandler::new();
        let mut session = SessionManager::new();
        
        // Add some data
        session.record_task(TaskRecord {
            task: "test".to_string(),
            result: "result".to_string(),
            success: true,
            duration_ms: 100,
            timestamp: 1234567890,
            files_modified: vec![],
        });
        
        assert_eq!(session.task_count(), 1);
        
        handler.execute(Command::Reset, &mut session).unwrap();
        
        assert_eq!(session.task_count(), 0);
    }

    #[test]
    fn test_execute_verbose() {
        let mut handler = CommandHandler::new();
        let mut session = SessionManager::new();
        
        assert!(!handler.is_verbose());
        
        handler.execute(Command::Verbose { enable: true }, &mut session).unwrap();
        assert!(handler.is_verbose());
        
        handler.execute(Command::Verbose { enable: false }, &mut session).unwrap();
        assert!(!handler.is_verbose());
    }

    #[test]
    fn test_verbose_mode() {
        let mut handler = CommandHandler::new();
        
        assert!(!handler.is_verbose());
        handler.set_verbose(true);
        assert!(handler.is_verbose());
        handler.set_verbose(false);
        assert!(!handler.is_verbose());
    }

    #[test]
    fn test_command_execution_speed() {
        let mut handler = CommandHandler::new();
        let mut session = SessionManager::new();
        
        let start = std::time::Instant::now();
        handler.execute(Command::Status, &mut session).unwrap();
        let elapsed = start.elapsed();
        
        // Should be well under 100ms target
        assert!(elapsed.as_millis() < 100, "Command execution too slow: {:?}", elapsed);
    }
}
