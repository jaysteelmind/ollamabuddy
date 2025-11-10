//! Tool registry with JSON schemas
//! 
//! Maintains registry of available tools with validation schemas.
//! 
//! Tools:
//! - list_dir: List directory contents
//! - read_file: Read file contents
//! - write_file: Write content to file
//! - run_command: Execute system command
//! - system_info: Get system information
//! - web_fetch: Fetch web content

use crate::tools::types::ToolSchema;
use serde_json::json;
use std::collections::HashMap;

/// Tool registry
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    /// Map of tool name to schema
    tools: HashMap<String, ToolSchema>,
}

impl ToolRegistry {
    /// Create new tool registry with all tools
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // Register all tools
        registry.register_list_dir();
        registry.register_read_file();
        registry.register_write_file();
        registry.register_run_command();
        registry.register_system_info();
        registry.register_web_fetch();

        registry
    }

    /// Register list_dir tool
    fn register_list_dir(&mut self) {
        let schema = ToolSchema::new(
            "list_dir",
            "List contents of a directory",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list (relative to working directory)"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list recursively",
                        "default": false
                    }
                },
                "required": ["path"]
            }),
            true, // Read-only
        );
        self.tools.insert("list_dir".to_string(), schema);
    }

    /// Register read_file tool
    fn register_read_file(&mut self) {
        let schema = ToolSchema::new(
            "read_file",
            "Read contents of a file",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to read (relative to working directory)"
                    }
                },
                "required": ["path"]
            }),
            true, // Read-only
        );
        self.tools.insert("read_file".to_string(), schema);
    }

    /// Register write_file tool
    fn register_write_file(&mut self) {
        let schema = ToolSchema::new(
            "write_file",
            "Write content to a file",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to write (relative to working directory)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "append": {
                        "type": "boolean",
                        "description": "Whether to append to file (default: overwrite)",
                        "default": false
                    }
                },
                "required": ["path", "content"]
            }),
            false, // Not read-only (writes to filesystem)
        );
        self.tools.insert("write_file".to_string(), schema);
    }

    /// Register run_command tool
    fn register_run_command(&mut self) {
        let schema = ToolSchema::new(
            "run_command",
            "Execute a system command",
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Command arguments",
                        "default": []
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "description": "Timeout in seconds",
                        "default": 60,
                        "minimum": 1,
                        "maximum": 300
                    }
                },
                "required": ["command"]
            }),
            false, // Not read-only (may have side effects)
        );
        self.tools.insert("run_command".to_string(), schema);
    }

    /// Register system_info tool
    fn register_system_info(&mut self) {
        let schema = ToolSchema::new(
            "system_info",
            "Get system information",
            json!({
                "type": "object",
                "properties": {
                    "info_type": {
                        "type": "string",
                        "enum": ["os", "cpu", "memory", "disk", "all"],
                        "description": "Type of system information to retrieve",
                        "default": "all"
                    }
                }
            }),
            true, // Read-only
        );
        self.tools.insert("system_info".to_string(), schema);
    }

    /// Register web_fetch tool
    fn register_web_fetch(&mut self) {
        let schema = ToolSchema::new(
            "web_fetch",
            "Fetch content from a URL",
            json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to fetch",
                        "format": "uri"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST"],
                        "description": "HTTP method",
                        "default": "GET"
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "description": "Timeout in seconds",
                        "default": 30,
                        "minimum": 1,
                        "maximum": 120
                    }
                },
                "required": ["url"]
            }),
            true, // Read-only (HTTP GET)
        );
        self.tools.insert("web_fetch".to_string(), schema);
    }

    /// Get tool schema by name
    pub fn get(&self, name: &str) -> Option<&ToolSchema> {
        self.tools.get(name)
    }

    /// Check if tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all tool schemas
    pub fn schemas(&self) -> Vec<&ToolSchema> {
        self.tools.values().collect()
    }

    /// Get read-only tool names
    pub fn read_only_tools(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter(|(_, schema)| schema.read_only)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get write tool names
    pub fn write_tools(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter(|(_, schema)| !schema.read_only)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get total number of tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.len(), 6);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_all_tools_registered() {
        let registry = ToolRegistry::new();
        
        assert!(registry.contains("list_dir"));
        assert!(registry.contains("read_file"));
        assert!(registry.contains("write_file"));
        assert!(registry.contains("run_command"));
        assert!(registry.contains("system_info"));
        assert!(registry.contains("web_fetch"));
    }

    #[test]
    fn test_get_tool_schema() {
        let registry = ToolRegistry::new();
        
        let schema = registry.get("read_file");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().name, "read_file");
    }

    #[test]
    fn test_read_only_tools() {
        let registry = ToolRegistry::new();
        let read_only = registry.read_only_tools();
        
        assert_eq!(read_only.len(), 4);
        assert!(read_only.contains(&"list_dir".to_string()));
        assert!(read_only.contains(&"read_file".to_string()));
        assert!(read_only.contains(&"system_info".to_string()));
        assert!(read_only.contains(&"web_fetch".to_string()));
    }

    #[test]
    fn test_write_tools() {
        let registry = ToolRegistry::new();
        let write_tools = registry.write_tools();
        
        assert_eq!(write_tools.len(), 2);
        assert!(write_tools.contains(&"write_file".to_string()));
        assert!(write_tools.contains(&"run_command".to_string()));
    }

    #[test]
    fn test_tool_names() {
        let registry = ToolRegistry::new();
        let names = registry.tool_names();
        
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn test_schemas() {
        let registry = ToolRegistry::new();
        let schemas = registry.schemas();
        
        assert_eq!(schemas.len(), 6);
        
        for schema in schemas {
            assert!(!schema.name.is_empty());
            assert!(!schema.description.is_empty());
        }
    }

    #[test]
    fn test_nonexistent_tool() {
        let registry = ToolRegistry::new();
        
        assert!(!registry.contains("nonexistent_tool"));
        assert!(registry.get("nonexistent_tool").is_none());
    }

    #[test]
    fn test_tool_schema_structure() {
        let registry = ToolRegistry::new();
        
        let read_file_schema = registry.get("read_file").unwrap();
        assert!(read_file_schema.read_only);
        assert_eq!(read_file_schema.name, "read_file");
        
        let write_file_schema = registry.get("write_file").unwrap();
        assert!(!write_file_schema.read_only);
        assert_eq!(write_file_schema.name, "write_file");
    }
}
