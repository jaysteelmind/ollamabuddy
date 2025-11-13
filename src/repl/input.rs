//! Input handler for REPL using rustyline
//! 
//! Provides readline functionality with history, editing, and command completion
//! Performance target: <50ms input responsiveness

use rustyline::history::History;
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Editor};
use std::path::PathBuf;

/// Input handler managing readline interface and command history
/// 
/// Features:
/// - Command line editing (Emacs-style by default)
/// - Persistent history (saved to disk)
/// - Input validation
/// - Graceful interrupt handling
pub struct InputHandler {
    editor: DefaultEditor,
    history_path: Option<PathBuf>,
    prompt: String,
}

impl InputHandler {
    /// Create new input handler
    /// 
    /// Complexity: O(1) initialization
    pub fn new() -> Result<Self> {
        let editor = DefaultEditor::new()?;
        
        Ok(InputHandler {
            editor,
            history_path: None,
            prompt: ">ollamabuddy: ".to_string(),
        })
    }
    
    /// Create input handler with persistent history
    /// 
    /// History file: ~/.ollamabuddy_history
    /// Max entries: 1000 (configurable)
    pub fn with_history(history_file: PathBuf) -> Result<Self> {
        let mut editor = DefaultEditor::new()?;
        
        // Load existing history if file exists
        if history_file.exists() {
            let _ = editor.load_history(&history_file);
        }
        
        Ok(InputHandler {
            editor,
            history_path: Some(history_file),
            prompt: ">ollamabuddy: ".to_string(),
        })
    }
    
    /// Set custom prompt
    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }
    
    /// Read a line of input from user
    /// 
    /// Returns:
    /// - Ok(Some(input)) for normal input
    /// - Ok(None) for EOF (Ctrl-D)
    /// - Err on interrupt (Ctrl-C) or other errors
    pub fn read_line(&mut self) -> Result<Option<String>> {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                
                // Skip empty lines
                if trimmed.is_empty() {
                    return Ok(Some(String::new()));
                }
                
                // Add to history
                let _ = self.editor.add_history_entry(trimmed);
                
                Ok(Some(trimmed.to_string()))
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C pressed
                Err(anyhow::anyhow!("Interrupted"))
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D pressed
                Ok(None)
            }
            Err(err) => {
                Err(anyhow::anyhow!("Readline error: {}", err))
            }
        }
    }
    
    /// Save history to disk
    /// 
    /// Called on graceful shutdown
    pub fn save_history(&mut self) -> Result<()> {
        if let Some(ref path) = self.history_path {
            self.editor.save_history(path)?;
        }
        Ok(())
    }
    
    /// Clear command history
    pub fn clear_history(&mut self) {
        let _ = self.editor.history_mut().clear();
    }
    
    /// Get history size
    pub fn history_len(&self) -> usize {
        self.editor.history().len()
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new().expect("Failed to create input handler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new();
        assert!(handler.is_ok());
    }

    #[test]
    fn test_input_handler_with_history() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("test_history");
        
        let handler = InputHandler::with_history(history_path);
        assert!(handler.is_ok());
    }

    #[test]
    fn test_custom_prompt() {
        let mut handler = InputHandler::new().unwrap();
        handler.set_prompt("test> ".to_string());
        assert_eq!(handler.prompt, "test> ");
    }

    #[test]
    fn test_history_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");
        
        // Create handler and add history
        {
            let mut handler = InputHandler::with_history(history_path.clone()).unwrap();
            let _ = handler.editor.add_history_entry("command1");
            let _ = handler.editor.add_history_entry("command2");
            handler.save_history().unwrap();
        }
        
        // Verify history was saved
        assert!(history_path.exists());
        
        // Load history in new handler
        {
            let handler = InputHandler::with_history(history_path).unwrap();
            assert_eq!(handler.history_len(), 2);
        }
    }

    #[test]
    fn test_clear_history() {
        let mut handler = InputHandler::new().unwrap();
        let _ = handler.editor.add_history_entry("test");
        assert_eq!(handler.history_len(), 1);
        
        handler.clear_history();
        assert_eq!(handler.history_len(), 0);
    }

    #[test]
    fn test_default_prompt() {
        let handler = InputHandler::new().unwrap();
        assert_eq!(handler.prompt, ">ollamabuddy: ");
    }

    #[test]
    fn test_history_path_none() {
        let handler = InputHandler::new().unwrap();
        assert!(handler.history_path.is_none());
    }

    #[test]
    fn test_history_path_some() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");
        
        let handler = InputHandler::with_history(history_path.clone()).unwrap();
        assert_eq!(handler.history_path, Some(history_path));
    }
}
