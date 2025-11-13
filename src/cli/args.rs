//! Command-line argument parsing for OllamaBuddy
//! 
//! Provides clap-based CLI with subcommands and verbosity control.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// OllamaBuddy - Transform local Ollama models into capable terminal agents
#[derive(Parser, Debug)]
#[command(name = "ollamabuddy")]
#[command(author = "Jerome (Kubashen) Naidoo")]
#[command(version = "0.2.0")]
#[command(about = "Turn any local Ollama model into a capable terminal agent", long_about = None)]
pub struct Args {
    /// Task description or goal for the agent
    #[arg(value_name = "TASK")]
    pub task: Option<String>,

    /// Ollama model to use
    #[arg(short, long, default_value = "qwen2.5:7b-instruct")]
    pub model: String,

    /// Ollama host
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Ollama port
    #[arg(long, default_value_t = 11434)]
    pub port: u16,

    /// Working directory (current directory by default)
    #[arg(long)]
    pub cwd: Option<PathBuf>,


    /// Enable online mode (web_fetch tool)
    #[arg(long)]
    pub online: bool,

    /// Auto-upgrade model when recommended
    #[arg(long)]
    pub auto_upgrade: bool,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Verbosity level: -q (quiet), default (normal), -v (verbose), -vv (very verbose)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Quiet mode (suppress all output except final result)
    #[arg(short, long)]
    pub quiet: bool,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start interactive REPL mode
    Start,
    /// Run system diagnostics and health checks
    Doctor,

    /// List available Ollama models
    Models,

    /// Clean agent state and temporary files
    Clean {
        /// Also remove logs
        #[arg(long)]
        logs: bool,
    },

    /// Display current configuration
    Config,
}

/// Verbosity level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    VeryVerbose,
}

impl Args {
    /// Get verbosity level based on flags
    pub fn verbosity(&self) -> Verbosity {
        if self.quiet {
            Verbosity::Quiet
        } else {
            match self.verbose {
                0 => Verbosity::Normal,
                1 => Verbosity::Verbose,
                _ => Verbosity::VeryVerbose,
            }
        }
    }

    /// Get working directory (current dir if not specified)
    pub fn working_dir(&self) -> PathBuf {
        self.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        })
    }

    /// Check if task is required and provided
    pub fn validate(&self) -> Result<(), String> {
        // Task required if no subcommand
        if self.command.is_none() && self.task.is_none() {
            return Err("Task description required. Use 'ollamabuddy <TASK>' or run a subcommand.".to_string());
        }

        // Task not allowed with subcommands
        if self.command.is_some() && self.task.is_some() {
            return Err("Cannot specify task with subcommand.".to_string());
        }

        Ok(())
    }

    /// Get Ollama base URL
    pub fn ollama_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

impl Verbosity {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Verbosity::Quiet => "quiet",
            Verbosity::Normal => "normal",
            Verbosity::Verbose => "verbose",
            Verbosity::VeryVerbose => "very_verbose",
        }
    }

    /// Check if should show progress bars
    pub fn show_progress(&self) -> bool {
        !matches!(self, Verbosity::Quiet)
    }

    /// Check if should show detailed events
    pub fn show_events(&self) -> bool {
        matches!(self, Verbosity::Verbose | Verbosity::VeryVerbose)
    }

    /// Check if should show token streaming
    pub fn show_tokens(&self) -> bool {
        matches!(self, Verbosity::VeryVerbose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_quiet() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: true,
            command: None,
        };
        assert_eq!(args.verbosity(), Verbosity::Quiet);
    }

    #[test]
    fn test_verbosity_normal() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: false,
            command: None,
        };
        assert_eq!(args.verbosity(), Verbosity::Normal);
    }

    #[test]
    fn test_verbosity_verbose() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 1,
            quiet: false,
            command: None,
        };
        assert_eq!(args.verbosity(), Verbosity::Verbose);
    }

    #[test]
    fn test_verbosity_very_verbose() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 2,
            quiet: false,
            command: None,
        };
        assert_eq!(args.verbosity(), Verbosity::VeryVerbose);
    }

    #[test]
    fn test_validate_success_with_task() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: false,
            command: None,
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_success_with_subcommand() {
        let args = Args {
            task: None,
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: false,
            command: Some(Commands::Doctor),
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_fail_no_task_or_command() {
        let args = Args {
            task: None,
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: false,
            command: None,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_fail_both_task_and_command() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 11434,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: false,
            command: Some(Commands::Doctor),
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_ollama_url() {
        let args = Args {
            task: Some("test".to_string()),
            model: "test".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            cwd: None,
            online: false,
            auto_upgrade: false,
            config: None,
            verbose: 0,
            quiet: false,
            command: None,
        };
        assert_eq!(args.ollama_url(), "http://localhost:8080");
    }

    #[test]
    fn test_verbosity_methods() {
        assert!(!Verbosity::Quiet.show_progress());
        assert!(Verbosity::Normal.show_progress());
        
        assert!(!Verbosity::Normal.show_events());
        assert!(Verbosity::Verbose.show_events());
        
        assert!(!Verbosity::Verbose.show_tokens());
        assert!(Verbosity::VeryVerbose.show_tokens());
    }
}
