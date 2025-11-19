//! Shared agent execution logic for CLI and REPL modes
//!
//! This module extracts the core agent execution loop from main.rs,
//! making it reusable across different execution contexts (CLI and REPL).

/// Extract complete JSON object from text that may contain thinking before JSON
/// Returns the JSON substring, or the original text if no complete JSON found
fn extract_json_object(text: &str) -> Option<&str> {
    // Find all opening braces
    for (i, c) in text.char_indices() {
        if c == '{' {
            // Try to find matching closing brace
            let mut depth = 0;
            let mut in_string = false;
            let mut escape_next = false;

            for (j, ch) in text[i..].char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }

                match ch {
                    '\\' if in_string => escape_next = true,
                    '"' if !in_string => in_string = true,
                    '"' if in_string => in_string = false,
                    '{' if !in_string => depth += 1,
                    '}' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            // Found complete JSON object
                            return Some(&text[i..=i+j]);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

use crate::agent::{AgentOrchestrator, StateEvent};
use crate::analysis::ConvergenceDetector;
use crate::display_mode::DisplayMode;
use crate::recovery::AdaptiveRecovery;
use crate::telemetry::{TelemetryCollector, TelemetryEvent};
use crate::tools::runtime::ToolRuntime;
use crate::types::{AgentMsg, MemoryEntry, TaskExecutionResult};
use crate::validation::ValidationOrchestrator;
use anyhow::Result;
use futures_util::StreamExt;
use std::time::Instant;

/// Execute an agent task with full orchestration
///
/// This function encapsulates the complete agent execution loop,
/// including streaming, tool execution, validation, and convergence detection.
///
/// # Parameters
/// - `orchestrator`: Pre-initialized agent orchestrator with system prompt and task
/// - `tool_runtime`: Tool execution runtime
/// - `telemetry`: Telemetry collector for metrics
/// - `max_iterations`: Maximum number of iterations allowed
/// - `task`: The task description (for validation)
/// - `verbose`: Whether to show verbose output
/// - `display_mode`: Display abstraction for CLI vs REPL output
///
/// # Returns
/// - `TaskExecutionResult` with execution details
pub async fn execute_agent_task(
    orchestrator: &mut AgentOrchestrator,
    tool_runtime: &ToolRuntime,
    telemetry: &TelemetryCollector,
    max_iterations: usize,
    task: &str,
    verbose: bool,
    display_mode: &DisplayMode,
) -> Result<TaskExecutionResult> {
    let start_time = Instant::now();
    
    // Initialize PRD 9 components
    let mut validation_orchestrator = ValidationOrchestrator::new();
    let mut convergence_detector = ConvergenceDetector::new();
    let mut adaptive_recovery = AdaptiveRecovery::new();
    let mut tool_results_log: Vec<crate::tools::types::ToolResult> = Vec::new();
    
    let mut iteration = 0;
    let mut files_touched: Vec<String> = Vec::new();
    let mut final_output = String::new();
    
    // Main execution loop
    while iteration < max_iterations
        && !matches!(
            orchestrator.state(),
            crate::agent::AgentState::Final | crate::agent::AgentState::Error
        )
    {
        iteration += 1;
        
        display_mode
            .show_info(&format!("\n=== Iteration {}/{} ===", iteration, max_iterations))
            .await;
        
        // Check context and compress if needed
        let tokens_before = orchestrator.token_count();
        orchestrator.maybe_compress()?;
        let tokens_after = orchestrator.token_count();
        
        if tokens_before != tokens_after {
            telemetry.record(TelemetryEvent::ContextCompression {
                before_tokens: tokens_before,
                after_tokens: tokens_after,
                timestamp: Instant::now(),
            });
            
            display_mode
                .show_info(&format!(
                    "Compressed context: {} -> {} tokens",
                    tokens_before, tokens_after
                ))
                .await;
        }
        
        // Build prompt
        let prompt = orchestrator.build_prompt();
        
        if verbose {
            display_mode
                .show_info(&format!("Prompt ({} tokens)", orchestrator.token_count()))
                .await;
        }
        
        // Stream response from Ollama
        let client = orchestrator.client();
        let mut stream = client.generate_stream(prompt).await?;
        
        display_mode.show_info("Agent:").await;

        let mut response_text = String::new();

        // Stream thinking in real-time
        use std::io::Write;

        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result?;

            // Extract "response" field from Ollama API format
            if let Ok(ollama_response) = serde_json::from_slice::<serde_json::Value>(&chunk_bytes) {
                if let Some(token) = ollama_response.get("response").and_then(|r| r.as_str()) {
                    response_text.push_str(token);

                    telemetry.record(TelemetryEvent::TokenReceived {
                        token: token.to_string(),
                        timestamp: Instant::now(),
                    });

                    // Stream thinking text in real-time
                    print!("{}", token);
                    std::io::stdout().flush().ok();
                }
            }
        }

        println!(); // New line after streaming

        // Parse accumulated response
        if !response_text.is_empty() {
            let trimmed = response_text.trim();

            // Unescape JSON first (model outputs escaped quotes)
            let unescaped = trimmed.replace(r#"\""#, r#"""#);

            // Extract JSON from the response (thinking comes before)
            let json_str = extract_json_object(&unescaped).unwrap_or(&unescaped);

            if verbose {
                display_mode
                    .show_info(&format!("Extracted JSON: {}", &json_str[..json_str.len().min(100)]))
                    .await;
            }

            match serde_json::from_str::<AgentMsg>(json_str) {
                Ok(agent_msg) => {
                    match agent_msg {
                        AgentMsg::ToolCall { tool, args } => {
                            display_mode
                                .show_info(&format!("Tool call: {} with args: {:?}", tool, args))
                                .await;
                            
                            let tool_start = Instant::now();
                            telemetry.record(TelemetryEvent::ToolStarted {
                                tool: tool.clone(),
                                timestamp: tool_start,
                            });
                            
                            // Transition to executing
                            orchestrator.transition(StateEvent::ToolCall)?;
                            
                            display_mode
                                .show_info(&format!("Executing: {}", tool))
                                .await;
                            
                            // Execute tool
                            let result = tool_runtime
                                .execute(&tool, &serde_json::to_value(&args)?)
                                .await;
                            
                            match result {
                                Ok(tool_output) => {
                                    let duration = tool_start.elapsed().as_millis() as u64;
                                    telemetry.record(TelemetryEvent::ToolCompleted {
                                        tool: tool.clone(),
                                        duration_ms: duration,
                                        success: true,
                                        timestamp: Instant::now(),
                                    });
                                    
                                    display_mode
                                        .show_success(&format!(
                                            "Tool result ({}ms): {}",
                                            duration,
                                            &tool_output.output[..tool_output.output.len().min(100)]
                                        ))
                                        .await;
                                    
                                    // Track files if tool modified filesystem
                                    if tool == "write_file" {
                                        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                                            files_touched.push(path.to_string());
                                        }
                                    }
                                    
                                    // Collect tool result for validation
                                    tool_results_log.push(tool_output.clone());
                                    
                                    // Add to memory
                                    orchestrator.memory_mut().add(MemoryEntry::ToolCall {
                                        tool: tool.clone(),
                                        args: args.clone(),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    });
                                    
                                    orchestrator.memory_mut().add(MemoryEntry::ToolResult {
                                        tool: tool.clone(),
                                        output: tool_output.output.clone(),
                                        success: true,
                                        duration_ms: tool_output.duration_ms,
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    });

                                    // Add reflection prompt after tool execution
                                    let reflection_prompt = format!(
                                        "\n\nREFLECTION: You just executed '{}'. Result: {}\n\n\
                                        Original task: {}\n\n\
                                        Has the task been FULLY completed?\n\
                                        - If YES: Output {{\"type\": \"final\", \"result\": \"description of what you accomplished\"}}\n\
                                        - If NO: Either call another tool OR explain what still needs to be done.",
                                        tool,
                                        &tool_output.output[..tool_output.output.len().min(200)],
                                        task
                                    );

                                    orchestrator.memory_mut().add(MemoryEntry::SystemPrompt {
                                        content: reflection_prompt,
                                    });

                                    // State transitions
                                    orchestrator.transition(StateEvent::ToolComplete)?;
                                    orchestrator.transition(StateEvent::ContinueIteration)?;
                                }
                                Err(e) => {
                                    display_mode
                                        .show_error(&format!("Tool execution failed: {}", e))
                                        .await;
                                    
                                    // Adaptive recovery
                                    use crate::recovery::types::FailureSymptom;
                                    let symptom = FailureSymptom::ToolExecutionFailure {
                                        tool_name: tool.clone(),
                                        consecutive_failures: tool_results_log
                                            .iter()
                                            .rev()
                                            .take_while(|r| !r.success && r.tool == tool)
                                            .count()
                                            + 1,
                                    };
                                    
                                    if let Some(pattern) = adaptive_recovery.detect_pattern(symptom) {
                                        let action = adaptive_recovery.select_recovery_action(&pattern);
                                        
                                        if verbose {
                                            display_mode
                                                .show_warning(&format!("Recovery action: {:?}", action))
                                                .await;
                                        }
                                        
                                        use crate::recovery::types::RecoveryAction;
                                        match action {
                                            RecoveryAction::Abort { reason } => {
                                                display_mode
                                                    .show_error(&format!("Aborting: {}", reason))
                                                    .await;
                                                orchestrator.transition(StateEvent::UnrecoverableError)?;
                                            }
                                            _ => {
                                                orchestrator.transition(StateEvent::ToolFailure)?;
                                            }
                                        }
                                    } else {
                                        orchestrator.transition(StateEvent::ToolFailure)?;
                                    }
                                }
                            }
                        }
                        AgentMsg::Final { result, summary } => {
                            // Run validation on task completion
                            if !tool_results_log.is_empty() {
                                let expected_outputs = vec![task.to_string()];
                                let validation_result = validation_orchestrator
                                    .orchestrate_validation(&tool_results_log, &expected_outputs);
                                
                                if validation_result.success {
                                    if verbose {
                                        display_mode
                                            .show_success(&format!(
                                                "Task validated (score: {:.2})",
                                                validation_result.validation.score.overall
                                            ))
                                            .await;
                                    }
                                } else {
                                    display_mode
                                        .show_warning(&format!(
                                            "Validation score: {:.2}",
                                            validation_result.validation.score.overall
                                        ))
                                        .await;
                                }
                            }
                            
                            display_mode.show_success("Task Complete!").await;
                            display_mode.show_success(&result).await;
                            
                            if let Some(sum) = summary {
                                display_mode.show_info(&format!("Summary: {}", sum)).await;
                            }
                            
                            final_output = result;
                            orchestrator.transition(StateEvent::GoalAchieved)?;
                            break;
                        }
                        AgentMsg::Plan { steps, reasoning } => {
                            display_mode.show_info("Plan created:").await;
                            for (i, step) in steps.iter().enumerate() {
                                display_mode.show_info(&format!("  {}. {}", i + 1, step)).await;
                            }
                            if let Some(reason) = reasoning {
                                display_mode.show_info(&format!("Reasoning: {}", reason)).await;
                            }
                            orchestrator.transition(StateEvent::PlanComplete)?;
                        }
                        AgentMsg::Ask { question } => {
                            display_mode.show_info(&format!("Model asks: {}", question)).await;
                        }
                        AgentMsg::Error { message, recoverable } => {
                            display_mode.show_error(&format!("Model error: {}", message)).await;
                            if recoverable {
                                display_mode.show_warning("Error is recoverable, continuing...").await;
                            } else {
                                orchestrator.transition(StateEvent::UnrecoverableError)?;
                            }
                        }
                    }
                }
                Err(e) => {
                    if verbose {
                        display_mode
                            .show_warning(&format!("Parse failed: {} - Text: {}", e, &unescaped[..unescaped.len().min(100)]))
                            .await;
                    }
                }
            }
        }
        
        // Track progress and check convergence
        let current_progress = match orchestrator.state() {
            crate::agent::AgentState::Final => 1.0,
            crate::agent::AgentState::Executing => {
                (tool_results_log.iter().filter(|r| r.success).count() as f64 * 0.15).min(0.9)
            }
            _ => iteration as f64 / max_iterations as f64 * 0.5,
        };
        
        convergence_detector.record_progress(current_progress, iteration);
        
        if verbose {
            if let Some(velocity) = convergence_detector.get_velocity() {
                display_mode
                    .show_info(&format!(
                        "Progress: {:.2}, Velocity: {:.3}",
                        current_progress, velocity.velocity
                    ))
                    .await;
            }
        }
        
        // Check for early termination
        let validation_score = if let Some(last_result) = tool_results_log.last() {
            if last_result.success {
                0.9
            } else {
                0.5
            }
        } else {
            0.5
        };
        
        let termination = convergence_detector.check_termination(
            current_progress,
            validation_score,
            iteration,
            max_iterations,
        );
        
        if termination.should_terminate() {
            use crate::analysis::types::TerminationCondition;
            match termination {
                TerminationCondition::Success => {
                    if verbose {
                        display_mode
                            .show_success(&format!("Early success detected at iteration {}", iteration))
                            .await;
                    }
                    break;
                }
                TerminationCondition::Stagnation => {
                    display_mode
                        .show_warning(&format!("Stagnation detected at iteration {}", iteration))
                        .await;
                    break;
                }
                _ => {}
            }
        }
    }
    
    // Check if max iterations reached
    if iteration >= max_iterations {
        display_mode.show_warning("Maximum iterations reached").await;
    }
    
    // Build final result
    let duration = start_time.elapsed();
    let success = matches!(orchestrator.state(), crate::agent::AgentState::Final);
    
    let output = if !final_output.is_empty() {
        final_output
    } else if success {
        "Task completed successfully".to_string()
    } else {
        "Task incomplete".to_string()
    };
    
    let validation_score = if !tool_results_log.is_empty() {
        let expected_outputs = vec![task.to_string()];
        let validation_result =
            validation_orchestrator.orchestrate_validation(&tool_results_log, &expected_outputs);
        validation_result.validation.score.overall
    } else {
        0.0
    };
    
    // Record episode to memory
    orchestrator.record_episode(task.to_string(), success, if success { None } else { Some("Task incomplete or failed".to_string()) });
    
    if verbose {
        display_mode.show_info("Session episode recorded to memory").await;
    }
    
    Ok(if success {
        TaskExecutionResult::success(output, duration, iteration as u32, files_touched, validation_score)
    } else {
        TaskExecutionResult::failure(output, duration, iteration as u32)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}
