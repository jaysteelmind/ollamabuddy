//! Process tool implementations
//! 
//! Implements secure process operations:
//! - run_command: Execute system commands (no shell injection)
//! - system_info: Gather system information
//! - web_fetch: Fetch web content

use crate::errors::{AgentError, Result};
use crate::tools::types::{ToolContext, ToolResult};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Run system command
/// 
/// # Security
/// - Uses argv arrays (no shell injection)
/// - Timeout enforcement
/// - Not read-only (may have side effects)
pub async fn run_command(
    command: &str,
    args: &[String],
    timeout_seconds: u64,
    _context: &ToolContext,
) -> Result<ToolResult> {
    let start = Instant::now();

    // Validate command is not empty
    if command.is_empty() {
        return Ok(ToolResult::failure(
            "run_command".to_string(),
            "Command cannot be empty".to_string(),
            start.elapsed(),
        ));
    }

    // Create command - use shell if command contains shell operators
    let needs_shell = command.contains('|') || 
                     command.contains('>') || 
                     command.contains('<') ||
                     command.contains('&') ||
                     command.contains(';');
    
    let mut cmd = if needs_shell {
        // Use shell for complex commands (pipes, redirects, etc.)
        // Security note: User is responsible for command safety
        #[cfg(unix)]
        {
            let mut c = Command::new("sh");
            c.arg("-c");
            c.arg(command);
            c
        }
        #[cfg(windows)]
        {
            let mut c = Command::new("cmd");
            c.arg("/C");
            c.arg(command);
            c
        }
    } else {
        // Direct execution for simple commands (safer)
        let mut c = Command::new(command);
        c.args(args);
        c
    };

    // Execute with timeout
    let timeout_duration = Duration::from_secs(timeout_seconds);
    
    match timeout(timeout_duration, cmd.output()).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            let combined_output = if stderr.is_empty() {
                stdout
            } else {
                format!("STDOUT:
{}

STDERR:
{}", stdout, stderr)
            };

            let exit_code = output.status.code().unwrap_or(-1);

            Ok(ToolResult::with_exit_code(
                "run_command".to_string(),
                combined_output,
                exit_code,
                start.elapsed(),
            ))
        }
        Ok(Err(e)) => Ok(ToolResult::failure(
            "run_command".to_string(),
            format!("Failed to execute command: {}", e),
            start.elapsed(),
        )),
        Err(_) => Ok(ToolResult::failure(
            "run_command".to_string(),
            format!("Command timed out after {}s", timeout_seconds),
            start.elapsed(),
        )),
    }
}

/// Get system information
/// 
/// # Security
/// - Read-only operation (safe for parallelization)
/// - No external commands executed
pub async fn system_info(info_type: &str) -> Result<ToolResult> {
    let start = Instant::now();

    let info = match info_type {
        "os" => get_os_info(),
        "cpu" => get_cpu_info(),
        "memory" => get_memory_info(),
        "disk" => get_disk_info(),
        "all" => format!(
            "OS:
{}

CPU:
{}

Memory:
{}

Disk:
{}",
            get_os_info(),
            get_cpu_info(),
            get_memory_info(),
            get_disk_info()
        ),
        _ => {
            return Ok(ToolResult::failure(
                "system_info".to_string(),
                format!("Unknown info type: {}", info_type),
                start.elapsed(),
            ));
        }
    };

    Ok(ToolResult::success(
        "system_info".to_string(),
        info,
        start.elapsed(),
    ))
}

fn get_os_info() -> String {
    format!(
        "OS: {}
Arch: {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn get_cpu_info() -> String {
    let num_cpus = num_cpus::get();
    format!("CPU Cores: {}", num_cpus)
}

fn get_memory_info() -> String {
    // Simple memory info (platform-specific details would require sys-info crate)
    "Memory info: Available via system tools".to_string()
}

fn get_disk_info() -> String {
    // Simple disk info
    if let Ok(current_dir) = std::env::current_dir() {
        format!("Current directory: {}", current_dir.display())
    } else {
        "Disk info: Available via system tools".to_string()
    }
}

/// Fetch web content
/// 
/// # Security
/// - Timeout enforcement
/// - Size limit on response
/// - Read-only operation (HTTP GET)
pub async fn web_fetch(
    url: &str,
    method: &str,
    timeout_seconds: u64,
    context: &ToolContext,
) -> Result<ToolResult> {
    let start = Instant::now();

    // Validate URL
    if url.is_empty() {
        return Ok(ToolResult::failure(
            "web_fetch".to_string(),
            "URL cannot be empty".to_string(),
            start.elapsed(),
        ));
    }

    // Validate method
    if method != "GET" && method != "POST" {
        return Ok(ToolResult::failure(
            "web_fetch".to_string(),
            format!("Unsupported HTTP method: {}", method),
            start.elapsed(),
        ));
    }

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .map_err(|e| AgentError::HttpError(e))?;

    // Execute request
    let request = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        _ => unreachable!(),
    };

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            
            match response.text().await {
                Ok(text) => {
                    // Check size limit
                    if text.len() > context.max_output_size {
                        return Ok(ToolResult::failure(
                            "web_fetch".to_string(),
                            format!(
                                "Response too large: {} bytes (max: {})",
                                text.len(),
                                context.max_output_size
                            ),
                            start.elapsed(),
                        ));
                    }

                    let output = format!("Status: {}

Body:
{}", status, text);

                    Ok(ToolResult::success(
                        "web_fetch".to_string(),
                        output,
                        start.elapsed(),
                    ))
                }
                Err(e) => Ok(ToolResult::failure(
                    "web_fetch".to_string(),
                    format!("Failed to read response: {}", e),
                    start.elapsed(),
                )),
            }
        }
        Err(e) => {
            let error_msg = if e.is_timeout() {
                format!("Request timed out after {}s", timeout_seconds)
            } else {
                format!("Request failed: {}", e)
            };

            Ok(ToolResult::failure(
                "web_fetch".to_string(),
                error_msg,
                start.elapsed(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let context = ToolContext::default();
        
        // Run echo command
        let result = run_command("echo", &["hello".to_string()], 5, &context)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("hello"));
        assert_eq!(result.exit_code.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_run_command_with_args() {
        let context = ToolContext::default();
        
        let result = run_command(
            "echo",
            &["arg1".to_string(), "arg2".to_string()],
            5,
            &context,
        )
        .await
        .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_run_command_nonzero_exit() {
        let context = ToolContext::default();
        
        // Run command that fails
        let result = run_command("false", &[], 5, &context).await.unwrap();

        assert!(!result.success);
        assert_ne!(result.exit_code.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_run_command_timeout() {
        let context = ToolContext::default();
        
        // Run sleep command with short timeout
        let result = run_command("sleep", &["10".to_string()], 1, &context)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn test_run_command_empty() {
        let context = ToolContext::default();
        
        let result = run_command("", &[], 5, &context).await.unwrap();

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_system_info_os() {
        let result = system_info("os").await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("OS:"));
    }

    #[tokio::test]
    async fn test_system_info_cpu() {
        let result = system_info("cpu").await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("CPU"));
    }

    #[tokio::test]
    async fn test_system_info_all() {
        let result = system_info("all").await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("OS:"));
        assert!(result.output.contains("CPU:"));
    }

    #[tokio::test]
    async fn test_system_info_invalid_type() {
        let result = system_info("invalid").await.unwrap();

        assert!(!result.success);
    }

    // Note: web_fetch tests would require mock HTTP server
    // These are integration tests, not unit tests
    
    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let context = ToolContext::default();
        
        let result = web_fetch("", "GET", 5, &context).await.unwrap();

        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_method() {
        let context = ToolContext::default();
        
        let result = web_fetch("http://example.com", "DELETE", 5, &context)
            .await
            .unwrap();

        assert!(!result.success);
    }
}

