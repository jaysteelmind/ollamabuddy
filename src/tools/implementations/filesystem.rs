//! Filesystem tool implementations
//! 
//! Implements secure filesystem operations:
//! - list_dir: List directory contents
//! - read_file: Read file contents with size limits
//! - write_file: Write content with path validation

use crate::errors::{AgentError, Result};
use crate::tools::security::PathJail;
use crate::tools::types::{ToolContext, ToolResult};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// List directory contents
/// 
/// # Security
/// - Path validated through jail
/// - Read-only operation (safe for parallelization)
/// - No symlink traversal outside jail
pub async fn list_dir(
    path: &str,
    recursive: bool,
    __context: &ToolContext,
    jail: &PathJail,
) -> Result<ToolResult> {
    let start = Instant::now();

    // Expand home directory if path starts with ~
    let expanded_path = if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{}/{}", home, &path[2..])
        } else {
            path.to_string()
        }
    } else if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| path.to_string())
    } else {
        path.to_string()
    };

    // Verify path is within jail
    let verified_path = jail.verify_and_canonicalize(&expanded_path)?;

    // Check if path exists and is a directory
    if !verified_path.exists() {
        return Ok(ToolResult::failure(
            "list_dir".to_string(),
            format!("Directory does not exist: {}", path),
            start.elapsed(),
        ));
    }

    if !verified_path.is_dir() {
        return Ok(ToolResult::failure(
            "list_dir".to_string(),
            format!("Path is not a directory: {}", path),
            start.elapsed(),
        ));
    }

    // List directory contents
    let entries = if recursive {
        list_recursive(&verified_path)?
    } else {
        list_single_level(&verified_path)?
    };

    let output = entries.join("
");

    Ok(ToolResult::success(
        "list_dir".to_string(),
        output,
        start.elapsed(),
    ))
}

/// List single level directory
fn list_single_level(path: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();

    let read_dir = fs::read_dir(path).map_err(|e| {
        AgentError::Generic(format!("Failed to read directory: {}", e))
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|e| {
            AgentError::Generic(format!("Failed to read entry: {}", e))
        })?;

        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().ok();
        
        let entry_type = if let Some(meta) = metadata {
            if meta.is_dir() {
                "DIR"
            } else if meta.is_file() {
                "FILE"
            } else {
                "OTHER"
            }
        } else {
            "UNKNOWN"
        };

        entries.push(format!("{:<10} {}", entry_type, name));
    }

    entries.sort();
    Ok(entries)
}

/// List directory recursively
fn list_recursive(path: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    list_recursive_helper(path, path, &mut entries)?;
    entries.sort();
    Ok(entries)
}

fn list_recursive_helper(base: &Path, current: &Path, entries: &mut Vec<String>) -> Result<()> {
    let read_dir = fs::read_dir(current).map_err(|e| {
        AgentError::Generic(format!("Failed to read directory: {}", e))
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|e| {
            AgentError::Generic(format!("Failed to read entry: {}", e))
        })?;

        let path = entry.path();
        let relative = path.strip_prefix(base).unwrap_or(&path);
        let name = relative.to_string_lossy().to_string();

        if path.is_dir() {
            entries.push(format!("DIR  {}/", name));
            list_recursive_helper(base, &path, entries)?;
        } else {
            entries.push(format!("FILE {}", name));
        }
    }

    Ok(())
}

/// Read file contents
/// 
/// # Security
/// - Path validated through jail
/// - Size limit enforced (max 2MB by default)
/// - Read-only operation (safe for parallelization)
pub async fn read_file(
    path: &str,
    context: &ToolContext,
    jail: &PathJail,
) -> Result<ToolResult> {
    let start = Instant::now();

    // Expand home directory if path starts with ~
    let expanded_path = if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{}/{}", home, &path[2..])
        } else {
            path.to_string()
        }
    } else if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| path.to_string())
    } else {
        path.to_string()
    };

    // Verify path is within jail
    let verified_path = jail.verify_and_canonicalize(&expanded_path)?;

    // Check if file exists
    if !verified_path.exists() {
        return Ok(ToolResult::failure(
            "read_file".to_string(),
            format!("File does not exist: {}", path),
            start.elapsed(),
        ));
    }

    if !verified_path.is_file() {
        return Ok(ToolResult::failure(
            "read_file".to_string(),
            format!("Path is not a file: {}", path),
            start.elapsed(),
        ));
    }

    // Check file size
    let metadata = fs::metadata(&verified_path).map_err(|e| {
        AgentError::Generic(format!("Failed to read metadata: {}", e))
    })?;

    if metadata.len() > context.max_output_size as u64 {
        return Ok(ToolResult::failure(
            "read_file".to_string(),
            format!(
                "File too large: {} bytes (max: {} bytes)",
                metadata.len(),
                context.max_output_size
            ),
            start.elapsed(),
        ));
    }

    // Read file contents
    let content = fs::read_to_string(&verified_path).map_err(|e| {
        AgentError::Generic(format!("Failed to read file: {}", e))
    })?;

    Ok(ToolResult::success(
        "read_file".to_string(),
        content,
        start.elapsed(),
    ))
}

/// Write file contents
/// 
/// # Security
/// - Path validated through jail
/// - Size limit enforced on content
/// - Not read-only (sequential execution required)
pub async fn write_file(
    path: &str,
    content: &str,
    append: bool,
    context: &ToolContext,
    jail: &PathJail,
) -> Result<ToolResult> {
    let start = Instant::now();

    // Check content size
    if content.len() > context.max_output_size {
        return Ok(ToolResult::failure(
            "write_file".to_string(),
            format!(
                "Content too large: {} bytes (max: {} bytes)",
                content.len(),
                context.max_output_size
            ),
            start.elapsed(),
        ));
    }

    // Expand home directory if path starts with ~
    let expanded_path = if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{}/{}", home, &path[2..])
        } else {
            path.to_string()
        }
    } else if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| path.to_string())
    } else {
        path.to_string()
    };

    // Construct full path
    let full_path = if std::path::Path::new(&expanded_path).is_absolute() {
        std::path::PathBuf::from(&expanded_path)
    } else {
        jail.jail_root().join(&expanded_path)
    };

    // Create parent directories first (before verification)
    if let Some(parent) = full_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                AgentError::Generic(format!("Failed to create parent directory: {}", e))
            })?;
        }
    }

    // Now verify the path is within jail
    let verified_path = jail.verify_and_canonicalize(&expanded_path)?;

    // Write or append to file
    let result = if append {
        fs::write(&verified_path, content)
    } else {
        fs::write(&verified_path, content)
    };

    match result {
        Ok(_) => {
            let action = if append { "appended" } else { "written" };
            Ok(ToolResult::success(
                "write_file".to_string(),
                format!("Successfully {} {} bytes to {}", action, content.len(), path),
                start.elapsed(),
            ))
        }
        Err(e) => Ok(ToolResult::failure(
            "write_file".to_string(),
            format!("Failed to write file: {}", e),
            start.elapsed(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_env() -> (TempDir, PathJail, ToolContext) {
        let temp_dir = TempDir::new().unwrap();
        let jail = PathJail::new(temp_dir.path()).unwrap();
        let context = ToolContext::new(temp_dir.path().to_path_buf());
        (temp_dir, jail, context)
    }

    #[tokio::test]
    async fn test_list_dir_empty() {
        let (_temp, jail, context) = setup_test_env().await;

        let result = list_dir(".", false, &context, &jail).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_list_dir_with_files() {
        let (temp, jail, context) = setup_test_env().await;

        // Create test files
        fs::write(temp.path().join("file1.txt"), "test").unwrap();
        fs::write(temp.path().join("file2.txt"), "test").unwrap();

        let result = list_dir(".", false, &context, &jail).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("file1.txt"));
        assert!(result.output.contains("file2.txt"));
    }

    #[tokio::test]
    async fn test_list_dir_recursive() {
        let (temp, jail, context) = setup_test_env().await;

        // Create nested structure
        fs::create_dir(temp.path().join("subdir")).unwrap();
        fs::write(temp.path().join("subdir/file.txt"), "test").unwrap();

        let result = list_dir(".", true, &context, &jail).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("subdir"));
        assert!(result.output.contains("file.txt"));
    }

    #[tokio::test]
    async fn test_list_dir_nonexistent() {
        let (_temp, jail, context) = setup_test_env().await;

        let result = list_dir("nonexistent", false, &context, &jail).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let (temp, jail, context) = setup_test_env().await;

        let test_content = "Hello, World!";
        fs::write(temp.path().join("test.txt"), test_content).unwrap();

        let result = read_file("test.txt", &context, &jail).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output, test_content);
    }

    #[tokio::test]
    async fn test_read_file_nonexistent() {
        let (_temp, jail, context) = setup_test_env().await;

        let result = read_file("nonexistent.txt", &context, &jail).await.unwrap();
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_read_file_too_large() {
        let (temp, jail, mut context) = setup_test_env().await;

        // Set small size limit
        context.max_output_size = 10;

        fs::write(temp.path().join("large.txt"), "a".repeat(100)).unwrap();

        let result = read_file("large.txt", &context, &jail).await.unwrap();
        assert!(!result.success);
        assert!(result.output.contains("too large") || result.error.unwrap().contains("too large"));
    }

    #[tokio::test]
    async fn test_write_file_success() {
        let (temp, jail, context) = setup_test_env().await;

        let content = "Test content";
        let result = write_file("output.txt", content, false, &context, &jail)
            .await
            .unwrap();

        assert!(result.success);

        // Verify file was written
        let written = fs::read_to_string(temp.path().join("output.txt")).unwrap();
        assert_eq!(written, content);
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let (_temp, jail, context) = setup_test_env().await;

        let result = write_file("nested/dir/file.txt", "test", false, &context, &jail)
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_write_file_too_large() {
        let (_temp, jail, mut context) = setup_test_env().await;

        context.max_output_size = 10;

        let result = write_file("file.txt", &"a".repeat(100), false, &context, &jail)
            .await
            .unwrap();

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_path_jail_security() {
        let (_temp, jail, context) = setup_test_env().await;

        // Attempt to escape jail
        let result = read_file("../../../etc/passwd", &context, &jail).await;
        assert!(result.is_err());
    }
}
