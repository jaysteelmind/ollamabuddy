use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub models: ModelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelsConfig {
    pub default: Option<String>,
}

impl Config {
    /// Load configuration from file, creating default if it doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            // Create default config
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }
        
        let contents = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&contents)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        let toml_string = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(&config_path, toml_string)
            .context("Failed to write config file")?;
        
        Ok(())
    }
    
    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Could not determine home directory")?;
        
        Ok(home.join(".ollamabuddy").join("config.toml"))
    }
    
    /// Set the default model
    pub fn set_default_model(&mut self, name: String) {
        self.models.default = Some(name);
    }
    
    /// Get the default model
    pub fn get_default_model(&self) -> Option<&str> {
        self.models.default.as_deref()
    }
    
    /// Clear the default model
    pub fn clear_default_model(&mut self) {
        self.models.default = None;
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            models: ModelsConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.models.default.is_none());
    }
    
    #[test]
    fn test_set_default_model() {
        let mut config = Config::default();
        config.set_default_model("qwen2.5:7b-instruct".to_string());
        assert_eq!(config.get_default_model(), Some("qwen2.5:7b-instruct"));
    }
    
    #[test]
    fn test_clear_default_model() {
        let mut config = Config::default();
        config.set_default_model("qwen2.5:7b-instruct".to_string());
        config.clear_default_model();
        assert!(config.get_default_model().is_none());
    }
    
    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.set_default_model("qwen2.5:7b-instruct".to_string());
        
        let toml_string = toml::to_string(&config).unwrap();
        assert!(toml_string.contains("qwen2.5:7b-instruct"));
        
        let deserialized: Config = toml::from_str(&toml_string).unwrap();
        assert_eq!(deserialized.get_default_model(), Some("qwen2.5:7b-instruct"));
    }
}
