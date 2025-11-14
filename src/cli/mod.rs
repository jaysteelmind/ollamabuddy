//! CLI module for OllamaBuddy
//! 
//! Handles command-line argument parsing and configuration management.

pub mod config;
pub mod args;

pub use config::Config;
pub use args::{Args, Commands, ModelsCommand, Verbosity};
