//! Tool runtime coordinator
//! 
//! Main coordinator implementing ToolRuntime trait for PRD 1 integration.
//! Orchestrates: executor, retry manager, security, and registry.

use crate::errors::Result;
use crate::tools::executor::ParallelExecutor;
use crate::tools::registry::ToolRegistry;
use crate::tools::security::PathJail;
use crate::tools::types::{ToolContext, ToolResult};
use std::sync::Arc;

/// Tool runtime coordinator
pub struct ToolRuntime {
    /// Parallel executor
    executor: Arc<ParallelExecutor>,
}

impl ToolRuntime {
    /// Create new tool runtime with working directory
    pub fn new(working_dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let jail = PathJail::new(working_dir.as_ref())?;
        let context = ToolContext::new(working_dir.as_ref().to_path_buf());
        let executor = ParallelExecutor::new(jail, context);

        Ok(Self {
            executor: Arc::new(executor),
        })
    }

    /// Create tool runtime with custom context
    pub fn with_context(jail: PathJail, context: ToolContext) -> Self {
        let executor = ParallelExecutor::new(jail, context);

        Self {
            executor: Arc::new(executor),
        }
    }

    /// Execute tool asynchronously
    /// 
    /// This is the main entry point for tool execution from PRD 1.
    /// 
    /// # Flow
    /// 1. Validate tool exists in registry
    /// 2. Route to parallel executor
    /// 3. Executor handles: retry, security, execution
    /// 4. Return result to agent orchestrator
    pub async fn execute(
        &self,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<ToolResult> {
        self.executor.execute(tool, args).await
    }

    /// Get tool registry
    pub fn get_registry(&self) -> &ToolRegistry {
        self.executor.registry()
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.get_registry().tool_names()
    }

    /// Check if tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.get_registry().contains(name)
    }

    /// Get read-only tools (safe for parallel execution)
    pub fn read_only_tools(&self) -> Vec<String> {
        self.get_registry().read_only_tools()
    }

    /// Get write tools (require sequential execution)
    pub fn write_tools(&self) -> Vec<String> {
        self.get_registry().write_tools()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_runtime() -> (ToolRuntime, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let runtime = ToolRuntime::new(temp_dir.path()).unwrap();
        (runtime, temp_dir)
    }

    #[test]
    fn test_runtime_creation() {
        let (runtime, _temp) = setup_runtime();
        assert_eq!(runtime.tool_names().len(), 6);
    }

    #[test]
    fn test_registry_access() {
        let (runtime, _temp) = setup_runtime();
        
        let registry = runtime.get_registry();
        assert_eq!(registry.len(), 6);
    }

    #[test]
    fn test_has_tool() {
        let (runtime, _temp) = setup_runtime();
        
        assert!(runtime.has_tool("read_file"));
        assert!(runtime.has_tool("write_file"));
        assert!(!runtime.has_tool("nonexistent"));
    }

    #[test]
    fn test_tool_classification() {
        let (runtime, _temp) = setup_runtime();
        
        let read_only = runtime.read_only_tools();
        let write = runtime.write_tools();
        
        assert_eq!(read_only.len(), 4);
        assert_eq!(write.len(), 2);
        
        assert!(read_only.contains(&"list_dir".to_string()));
        assert!(write.contains(&"write_file".to_string()));
    }

    #[tokio::test]
    async fn test_execute_list_dir() {
        let (runtime, _temp) = setup_runtime();
        
        let args = serde_json::json!({
            "path": ".",
            "recursive": false
        });

        let result = runtime.execute("list_dir", &args).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_system_info() {
        let (runtime, _temp) = setup_runtime();
        
        let args = serde_json::json!({
            "info_type": "os"
        });

        let result = runtime.execute("system_info", &args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("OS:"));
    }

    #[tokio::test]
    async fn test_execute_write_file() {
        let (runtime, temp) = setup_runtime();
        
        let args = serde_json::json!({
            "path": "test.txt",
            "content": "Hello, World!",
            "append": false
        });

        let result = runtime.execute("write_file", &args).await.unwrap();
        assert!(result.success);

        // Verify file was written
        let content = std::fs::read_to_string(temp.path().join("test.txt")).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_execute_read_file() {
        let (runtime, temp) = setup_runtime();
        
        // Create test file
        std::fs::write(temp.path().join("test.txt"), "test content").unwrap();

        let args = serde_json::json!({
            "path": "test.txt"
        });

        let result = runtime.execute("read_file", &args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output, "test content");
    }

    #[tokio::test]
    async fn test_execute_run_command() {
        let (runtime, _temp) = setup_runtime();
        
        let args = serde_json::json!({
            "command": "echo",
            "args": ["hello"],
            "timeout_seconds": 5
        });

        let result = runtime.execute("run_command", &args).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_security_path_jail() {
        let (runtime, _temp) = setup_runtime();
        
        // Attempt to read outside jail
        let args = serde_json::json!({
            "path": "../../../etc/passwd"
        });

        let result = runtime.execute("read_file", &args).await;
        
        // Should fail due to path jail
        assert!(result.is_err() || !result.unwrap().success);
    }

    #[tokio::test]
    async fn test_parallel_execution_multiple_reads() {
        let (runtime, temp) = setup_runtime();
        let runtime = Arc::new(runtime);
        
        // Create test files
        for i in 0..4 {
            std::fs::write(
                temp.path().join(format!("file{}.txt", i)),
                format!("content{}", i),
            )
            .unwrap();
        }

        // Execute multiple reads in parallel
        let mut handles = vec![];

        for i in 0..4 {
            let rt = runtime.clone();
            let handle = tokio::spawn(async move {
                let args = serde_json::json!({
                    "path": format!("file{}.txt", i)
                });
                rt.execute("read_file", &args).await
            });
            handles.push(handle);
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result.success);
        }
    }
}
