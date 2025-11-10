//! Configuration management for OllamaBuddy
//! 
//! Provides TOML-based configuration with defaults and validation.
//! Location: ~/.ollamabuddy/config.toml

// External crates

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::errors::{AgentError, Result};

/// Complete configuration for OllamaBuddy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ollama: OllamaConfig,
    pub agent: AgentConfig,
    pub tools: ToolsConfig,
    pub advisor: AdvisorConfig,
    pub telemetry: TelemetryConfig,
    pub paths: PathsConfig,
}

/// Ollama connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: String,
    pub port: u16,
    pub default_model: String,
}

/// Agent behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_context_tokens: usize,
    pub compress_threshold: usize,
    pub max_memory_entries: usize,
    pub max_iterations: usize,
    pub timeout_minutes: u64,
}

/// Tool execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub default_timeout_sec: u64,
    pub max_output_bytes: usize,
    pub online_enabled: bool,
    pub max_parallel: usize,
}

/// Model advisor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorConfig {
    pub auto_upgrade: bool,
    pub cost_sensitivity: f64,
    pub upgrade_threshold: f64,
}

/// Telemetry display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub default_verbosity: String,
    pub show_progress_bars: bool,
    pub color_output: bool,
}

/// File system paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub state_dir: String,
    pub log_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama: OllamaConfig::default(),
            agent: AgentConfig::default(),
            tools: ToolsConfig::default(),
            advisor: AdvisorConfig::default(),
            telemetry: TelemetryConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 11434,
            default_model: "qwen2.5:7b-instruct".to_string(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8000,
            compress_threshold: 6000,
            max_memory_entries: 100,
            max_iterations: 10,
            timeout_minutes: 30,
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            default_timeout_sec: 30,
            max_output_bytes: 2_000_000,
            online_enabled: false,
            max_parallel: 4,
        }
    }
}

impl Default for AdvisorConfig {
    fn default() -> Self {
        Self {
            auto_upgrade: false,
            cost_sensitivity: 0.3,
            upgrade_threshold: 0.15,
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            default_verbosity: "normal".to_string(),
            show_progress_bars: true,
            color_output: true,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            state_dir: "~/.ollamabuddy".to_string(),
            log_dir: "~/.ollamabuddy/logs".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from file or use defaults
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        if let Some(config_path) = path {
            Self::load_from_file(&config_path)
        } else {
            Self::load_default()
        }
    }

    /// Load configuration from specific file
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| AgentError::ConfigError(format!("Failed to read config: {}", e)))?;
        
        let config: Config = toml::from_str(&contents)
            .map_err(|e| AgentError::ConfigError(format!("Failed to parse config: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Load default configuration from standard location or use built-in defaults
    pub fn load_default() -> Result<Self> {
        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".ollamabuddy").join("config.toml");
            if config_path.exists() {
                return Self::load_from_file(&config_path);
            }
        }
        
        Ok(Config::default())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.agent.max_context_tokens == 0 {
            return Err(AgentError::ConfigError(
                "max_context_tokens must be greater than 0".to_string()
            ));
        }

        if self.agent.compress_threshold >= self.agent.max_context_tokens {
            return Err(AgentError::ConfigError(
                "compress_threshold must be less than max_context_tokens".to_string()
            ));
        }

        if self.advisor.cost_sensitivity < 0.0 || self.advisor.cost_sensitivity > 1.0 {
            return Err(AgentError::ConfigError(
                "cost_sensitivity must be between 0.0 and 1.0".to_string()
            ));
        }

        if self.advisor.upgrade_threshold < 0.0 || self.advisor.upgrade_threshold > 1.0 {
            return Err(AgentError::ConfigError(
                "upgrade_threshold must be between 0.0 and 1.0".to_string()
            ));
        }

        match self.telemetry.default_verbosity.as_str() {
            "quiet" | "normal" | "verbose" | "very_verbose" => {}
            _ => return Err(AgentError::ConfigError(
                format!("Invalid verbosity level: {}", self.telemetry.default_verbosity)
            )),
        }

        Ok(())
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| AgentError::ConfigError(format!("Failed to serialize config: {}", e)))?;
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AgentError::ConfigError(format!("Failed to create config dir: {}", e)))?;
        }

        std::fs::write(path, contents)
            .map_err(|e| AgentError::ConfigError(format!("Failed to write config: {}", e)))?;
        
        Ok(())
    }

    /// Get Ollama base URL
    pub fn ollama_url(&self) -> String {
        format!("http://{}:{}", self.ollama.host, self.ollama.port)
    }

    /// Expand tilde in paths
    pub fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }

    /// Get state directory path
    pub fn state_dir(&self) -> PathBuf {
        Self::expand_path(&self.paths.state_dir)
    }

    /// Get log directory path
    pub fn log_dir(&self) -> PathBuf {
        Self::expand_path(&self.paths.log_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.ollama.host, "127.0.0.1");
        assert_eq!(config.ollama.port, 11434);
        assert_eq!(config.agent.max_context_tokens, 8000);
    }

    #[test]
    fn test_config_validation_success() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_zero_tokens() {
        let mut config = Config::default();
        config.agent.max_context_tokens = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_threshold() {
        let mut config = Config::default();
        config.agent.compress_threshold = 9000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_cost_sensitivity() {
        let mut config = Config::default();
        config.advisor.cost_sensitivity = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_verbosity() {
        let mut config = Config::default();
        config.telemetry.default_verbosity = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_url() {
        let config = Config::default();
        assert_eq!(config.ollama_url(), "http://127.0.0.1:11434");
    }

    #[test]
    fn test_expand_path_with_tilde() {
        let path = "~/.ollamabuddy";
        let expanded = Config::expand_path(path);
        assert!(!expanded.to_string_lossy().contains("~"));
    }

    #[test]
    fn test_expand_path_without_tilde() {
        let path = "/absolute/path";
        let expanded = Config::expand_path(path);
        assert_eq!(expanded.to_string_lossy(), path);
    }
}
