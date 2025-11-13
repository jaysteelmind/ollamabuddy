//! Display mode abstraction for CLI and REPL contexts
//!
//! This module provides an abstraction layer for handling output differently
//! in CLI mode (direct stdout) versus REPL mode (event bus + display manager).

use crate::repl::DisplayManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Display mode determines how output is rendered
///
/// The agent execution logic needs to work in both CLI and REPL contexts.
/// This enum allows the execution code to be context-agnostic by providing
/// a unified interface for displaying progress, status, and results.
#[derive(Clone)]
pub enum DisplayMode {
    /// CLI mode - direct terminal output
    Cli,
    
    /// REPL mode - use display manager with event bus
    Repl(Arc<Mutex<DisplayManager>>),
}

impl DisplayMode {
    /// Create CLI display mode
    pub fn cli() -> Self {
        Self::Cli
    }

    /// Create REPL display mode with display manager
    pub fn repl(display_manager: Arc<Mutex<DisplayManager>>) -> Self {
        Self::Repl(display_manager)
    }

    /// Show an informational message
    pub async fn show_info(&self, message: &str) {
        match self {
            Self::Cli => {
                println!("{}", message);
            }
            Self::Repl(display) => {
                let display = display.lock().await;
                display.show_info(message);
            }
        }
    }

    /// Show a success message
    pub async fn show_success(&self, message: &str) {
        match self {
            Self::Cli => {
                println!("{}", message);
            }
            Self::Repl(display) => {
                let mut display = display.lock().await;
                display.finish_with_success(message, 0);
            }
        }
    }

    /// Show an error message
    pub async fn show_error(&self, message: &str) {
        match self {
            Self::Cli => {
                eprintln!("Error: {}", message);
            }
            Self::Repl(display) => {
                let display = display.lock().await;
                display.show_error(message);
            }
        }
    }

    /// Show a warning message
    pub async fn show_warning(&self, message: &str) {
        match self {
            Self::Cli => {
                println!("Warning: {}", message);
            }
            Self::Repl(display) => {
                let display = display.lock().await;
                display.show_warning(message);
            }
        }
    }

    /// Check if this is REPL mode
    pub fn is_repl(&self) -> bool {
        matches!(self, Self::Repl(_))
    }

    /// Check if this is CLI mode
    pub fn is_cli(&self) -> bool {
        matches!(self, Self::Cli)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_mode_creation() {
        let mode = DisplayMode::cli();
        assert!(mode.is_cli());
        assert!(!mode.is_repl());
    }

    #[test]
    fn test_mode_type_checking() {
        let cli = DisplayMode::cli();
        assert!(cli.is_cli());
        assert!(!cli.is_repl());
    }

    #[tokio::test]
    async fn test_cli_info_message() {
        let mode = DisplayMode::cli();
        // Should not panic
        mode.show_info("Test message").await;
    }

    #[tokio::test]
    async fn test_cli_success_message() {
        let mode = DisplayMode::cli();
        mode.show_success("Success").await;
    }

    #[tokio::test]
    async fn test_cli_error_message() {
        let mode = DisplayMode::cli();
        mode.show_error("Error").await;
    }

    #[tokio::test]
    async fn test_cli_warning_message() {
        let mode = DisplayMode::cli();
        mode.show_warning("Warning").await;
    }

    #[test]
    fn test_display_mode_clone() {
        let mode = DisplayMode::cli();
        let cloned = mode.clone();
        assert!(cloned.is_cli());
    }
}
