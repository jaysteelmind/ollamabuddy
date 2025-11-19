//! OllamaBuddy v0.2 - Main CLI Entry Point

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use colored::Colorize;
use ollamabuddy::budget::DynamicBudgetManager;
use ollamabuddy::integration::agent::RAGAgent;
use ollamabuddy::validation::ValidationOrchestrator;
use ollamabuddy::analysis::ConvergenceDetector;
use ollamabuddy::analysis::types::TerminationCondition;
use ollamabuddy::recovery::AdaptiveRecovery;
use ollamabuddy::repl::{ReplSession, ReplConfig};
use ollamabuddy::{
    models::ModelManager,
    cli::{Args, Commands, Verbosity},
    bootstrap::Bootstrap,
    doctor::Doctor,
    agent::AgentOrchestrator,
    agent::orchestrator::AgentConfig,
    tools::ToolRuntime,
    telemetry::{TelemetryCollector, TelemetryEvent, TelemetryDisplay},
};


/// Run agent in interactive REPL mode
/// Execute a task within REPL context with event emission
async fn execute_task_in_repl(
    args: &Args,
    task: &str,
    repl_session: &mut ReplSession,
) -> Result<()> {
    use std::path::PathBuf;
    use std::time::Instant;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let start_time = Instant::now();
    let verbose = repl_session.is_verbose();
    
    // Emit planning started event
    repl_session.event_bus().emit(
        ollamabuddy::repl::events::AgentEvent::PlanningStarted {
            task: task.to_string()
        }
    ).await;
    
    // Show planning progress
    let pb = repl_session.display_mut().start_planning(task);
    
    // Bootstrap check (silent in REPL)
    let bootstrap = Bootstrap::new(
        args.host.clone(),
        args.port,
        args.model.clone(),
    );
    
    if !bootstrap.check_ollama_running().await? {
        repl_session.display().show_error("Ollama is not running! Start with: ollama serve");
        return Err(anyhow::anyhow!("Ollama not running"));
    }
    
    // Initialize components
    let working_dir = args.cwd.clone().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });
    
    let ollama_url = format!("http://{}:{}", args.host, args.port);
    
    let config = AgentConfig {
        ollama_url,
        model: args.model.clone(),
        max_iterations: 50,
        verbose,
    };
    
    let mut orchestrator = AgentOrchestrator::new(config)?;

    // Use home directory as jail root for REPL mode to allow writes to ~/
    let jail_root = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| working_dir.clone());
    let tool_runtime = ToolRuntime::new(&jail_root)?;

    // Update progress
    repl_session.display().update_progress(&pb, 0.3, Some("Initializing agent"));
    
    // Initialize planning (async - LLM-based reasoning)
    orchestrator.initialize_planning(task).await?;

    // Update progress
    repl_session.display().update_progress(&pb, 0.6, Some("Creating execution plan"));
    
    // Get context from session
    let context = repl_session.get_context();
    
    // Build system prompt with full tool descriptions
    let tool_descriptions = vec![
        "list_dir: List files and directories. Args: path (string, required), recursive (bool, optional, default false)",
        "read_file: Read contents of a text file. Args: path (string, required)",
        "write_file: Write or append content to a file. Args: path (string, required), content (string, required), append (bool, optional, default false)",
        "run_command: Execute a system command (supports shell pipes/redirects). Args: command (string, required), args (array of strings, optional), timeout_seconds (number, optional, default 60)",
        "system_info: Get system information. Args: info_type (string, optional: 'os', 'cpu', 'memory', 'disk', 'all', default 'all')",
        "web_fetch: Fetch content from a URL. Args: url (string, required), method (string, optional: 'GET' or 'POST', default 'GET'), timeout_seconds (number, optional, default 30)",
    ];
    
    let tools_formatted = tool_descriptions.join("\n  ");
    
    let system_prompt = format!(r#"You are an autonomous AI agent that helps users complete tasks using available tools.

RESPONSE FORMAT - Think out loud, then provide JSON:

1. First, explain your reasoning (what you're trying to accomplish and why)
2. Then, on a new line, output valid JSON for your action

Tool call format:
{{"type": "tool_call", "tool": "tool_name", "args": {{"key": "value"}}}}

Completion format:
{{"type": "final", "result": "description of what was accomplished"}}

AVAILABLE TOOLS:
  {}

CRITICAL RULES:
1. Always explain your thinking before outputting JSON
2. End your response with valid JSON on its own line
3. Use exact tool names from the list above
4. Provide all required arguments as specified
5. When writing code files:
   - Use proper error handling (try/except in Python, proper error checks)
   - NEVER use shell=True in subprocess - use list arguments instead
   - Add input validation and sanitization
   - Include type hints and docstrings
   - Use meaningful variable names
   - Add comments for complex logic
   - Ensure code is production-ready, not just syntactically valid
   - Handle edge cases (empty inputs, missing files, permission errors)
   - Add logging for debugging
6. Before marking task complete, verify:
   - Code runs without errors
   - All edge cases are handled
   - No security vulnerabilities
   - Code follows best practices

CODE QUALITY EXAMPLES:

BAD (Don't do this):
```python
import subprocess
result = subprocess.check_output("df -h", shell=True)  # Security risk!
data = result.decode().split()[1]  # Will crash if format changes
```

GOOD (Do this instead):
```python
import subprocess
from typing import Optional

def check_disk_usage() -> Optional[str]:
    \"\"\"Check disk usage safely.\"\"\"
    try:
        result = subprocess.run(['df', '-h'], capture_output=True, text=True, check=True)
        lines = result.stdout.strip().split('\\n')
        if len(lines) < 2:
            return None
        return lines[1]
    except (subprocess.CalledProcessError, FileNotFoundError) as e:
        print(f"Error checking disk usage: {{e}}")
        return None
```

{}

Now begin!"#,
        tools_formatted,
        if !context.is_empty() { format!("Previous context:\n{}", context) } else { String::new() }
    );
    
    orchestrator.add_system_prompt(system_prompt);
    orchestrator.add_user_goal(task.to_string());
    orchestrator.set_goal(task.to_string());
    
    // Transition state machine
    use ollamabuddy::agent::StateEvent;
    orchestrator.transition(StateEvent::StartSession)?;
    
    // Complete planning phase
    repl_session.display().update_progress(&pb, 1.0, Some("Planning complete"));
    pb.finish_and_clear();
    
    let planning_duration = start_time.elapsed().as_millis() as u64;
    repl_session.event_bus().emit(
        ollamabuddy::repl::events::AgentEvent::PlanningComplete {
            duration_ms: planning_duration
        }
    ).await;
    
    repl_session.display().show_info(&format!("Planning complete ({}ms)", planning_duration));
    
    // Initialize telemetry
    let telemetry = TelemetryCollector::new();
    
    // Calculate dynamic budget
    let task_complexity = {
        let base_complexity = (task.len() as f64 / 200.0).min(0.5);
        let keyword_boost = if task.to_lowercase().contains("analyze") ||
                                task.to_lowercase().contains("complex") ||
                                task.to_lowercase().contains("multiple") {
            0.3
        } else {
            0.0
        };
        (base_complexity + keyword_boost).min(1.0)
    };
    
    let mut budget_manager = DynamicBudgetManager::new();
    let max_iterations = budget_manager.calculate_budget(task_complexity);
    
    if verbose {
        repl_session.display().show_info(&format!(
            "Task complexity: {:.2}, Allocated iterations: {}",
            task_complexity, max_iterations
        ));
    }
    
    // Create display mode for REPL (use CLI mode for now as DisplayManager is not Clone)
    let display_mode = ollamabuddy::DisplayMode::cli();
    
    // Emit execution started event
    repl_session.event_bus().emit(
        ollamabuddy::repl::events::AgentEvent::ExecutionStarted {
            tool: "agent".to_string()
        }
    ).await;
    
    // Execute task using shared function
    let execution_result = ollamabuddy::execution::execute_agent_task(
        &mut orchestrator,
        &tool_runtime,
        &telemetry,
        max_iterations,
        task,
        verbose,
        &display_mode,
    ).await?;
    
    // Emit completion event
    repl_session.event_bus().emit(
        ollamabuddy::repl::events::AgentEvent::TaskComplete {
            result: execution_result.output.clone(),
            duration_ms: execution_result.duration.as_millis() as u64,
        }
    ).await;
    
    // Record task in session
    let record = ollamabuddy::repl::session::TaskRecord {
        task: task.to_string(),
        result: execution_result.output.clone(),
        success: execution_result.success,
        duration_ms: execution_result.duration.as_millis() as u64,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        files_modified: execution_result.files_touched.iter()
            .map(|s| PathBuf::from(s))
            .collect(),
    };
    
    repl_session.record_task(record);
    
    // Show summary
    if execution_result.success {
        let duration_ms = execution_result.duration.as_millis() as u64;
        repl_session.display_mut().finish_with_success(
            &format!(
                "Task completed ({} iterations, score: {:.2})",
                execution_result.iterations,
                execution_result.validation_score
            ),
            duration_ms
        );
    } else {
        repl_session.display_mut().finish_with_error(&format!(
            "Task incomplete after {:.2}s ({} iterations)",
            execution_result.duration.as_secs_f64(),
            execution_result.iterations
        ));
    }
    
    Ok(())
}

async fn run_repl(args: &Args) -> Result<()> {
    // Initialize REPL session with history
    let history_path = std::env::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ollamabuddy_history");
    
    let mut repl_session = ReplSession::with_history(history_path)?;
    
    // Initialize RAG agent for memory commands (best effort - don't fail REPL if it errors)
    match RAGAgent::default_config().await {
        Ok(rag_agent) => {
            let rag_agent = std::sync::Arc::new(rag_agent);
            repl_session.set_rag_agent(rag_agent);
            println!("{}", "  [OK] Memory system initialized".green());
        }
        Err(e) => {
            eprintln!("{}: Could not initialize memory system: {}", "Warning".yellow(), e);
            eprintln!("  Memory commands (/memory, /stats) will not be available.");
        }
    }
    
    // Show welcome banner
    repl_session.show_welcome("v0.5.0", &args.model);
    
    // Main REPL loop
    loop {
        // Read user input
        match repl_session.read_input() {
            Ok(Some(input)) => {
                if input.is_empty() {
                    continue;
                }
                
                // Handle input (command or task)
                match repl_session.handle_input(&input) {
                    Ok(should_continue) => {
                        if !should_continue {
                            // User requested exit
                            break;
                        }
                        
                        // Check if it was a command (already handled)
                        if ollamabuddy::repl::commands::is_command(&input) {
                            continue;
                        }
                        
                        // Execute the task with full agent integration
                        match execute_task_in_repl(&args, &input, &mut repl_session).await {
                            Ok(()) => {
                                // Task executed successfully
                            }
                            Err(e) => {
                                repl_session.display_mut().finish_with_error(&format!("Task execution failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        repl_session.display_mut().finish_with_error(&format!("Error: {}", e));
                    }
                }
            }
            Ok(None) => {
                // EOF (Ctrl-D) - exit gracefully
                break;
            }
            Err(e) => {
                // Interrupted (Ctrl-C) or other error
                if e.to_string().contains("Interrupted") {
                    println!("\nUse /exit to quit gracefully");
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }
    
    // Save session history
    repl_session.save()?;
    
    Ok(())
}

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

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = Args::parse();
    
    // Load config and apply default model if set
    if let Ok(config) = ollamabuddy::config::Config::load() {
        if let Some(default_model) = config.get_default_model() {
            // Only override if user didn't specify --model flag
            // Check if model is still the default value
            if args.model == "qwen2.5:7b-instruct" {
                args.model = default_model.to_string();
            }
        }
    }

    match &args.command {
        Some(Commands::Start) => {
            // Start interactive REPL mode
            run_repl(&args).await?;
        }
        Some(Commands::Doctor) => {
            run_doctor(&args).await?;
        }
        Some(Commands::Models(models_cmd)) => {
            use ollamabuddy::cli::ModelsCommand;
            match models_cmd {
                ModelsCommand::List => {
                    handle_models_list(&args).await?;
                }
                ModelsCommand::Pull { name } => {
                    handle_models_pull(&args, name).await?;
                }
                ModelsCommand::Delete { name, force } => {
                    handle_models_delete(&args, name, *force).await?;
                }
                ModelsCommand::Info { name } => {
                    handle_models_info(&args, name).await?;
                }
                ModelsCommand::Use { name } => {
                    handle_models_use(&args, name).await?;
                }
                ModelsCommand::Current => {
                    handle_models_current(&args).await?;
                }
            }
        }
        Some(Commands::Clean { logs }) => {
            clean_state(&args, *logs).await?;
        }
        Some(Commands::Config) => {
            show_config(&args)?;
        }
        None => {
            // No subcommand - run single task or show help
            if let Some(task) = &args.task {
                // Run single task (traditional CLI mode)
                run_agent(&args, task).await?;
            } else {
                // No task and no REPL - show usage
                println!("OllamaBuddy v0.5.0 - Terminal Agent");
                println!("\nUsage:");
                println!("  ollamabuddy <task>            Run agent with task");
                println!("  ollamabuddy start             Interactive REPL mode");
                println!("  ollamabuddy doctor            System health checks");
                println!("  ollamabuddy models            List Ollama models");
                println!("  ollamabuddy config            Show configuration");
                println!("  ollamabuddy clean             Clear state/logs");
                println!("\nExample:");
                println!("  ollamabuddy \"List all .rs files and count lines of code\"");
                println!();
            }
        }
    }

    Ok(())
}


async fn run_agent(args: &Args, task: &str) -> Result<()> {
    // TODO PRD 10a Phase 3: Refactor to use ollamabuddy::execution::execute_agent_task()
    // Current implementation works, but could be simplified by using shared execution logic
    use std::path::PathBuf;
    
    // 1. Bootstrap check
    let bootstrap = Bootstrap::new(
        args.host.clone(),
        args.port,
        args.model.clone(),
    );
    
    if !bootstrap.check_ollama_running().await? {
        eprintln!("[ERROR] Ollama is not running!");
        eprintln!("
Start Ollama with: ollama serve");
        std::process::exit(2);
    }
    
    // 2. Initialize components
    let working_dir = args.cwd.clone().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });
    
    let ollama_url = format!("http://{}:{}", args.host, args.port);
    
    let config = AgentConfig {
        ollama_url,
        model: args.model.clone(),
        max_iterations: 50,
        verbose: matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose),
    };

    let mut orchestrator = AgentOrchestrator::new(config)?;

    // Use home directory as jail root for CLI mode to allow writes to ~/
    let jail_root = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| working_dir.clone());
    let tool_runtime = ToolRuntime::new(&jail_root)?;
    
    // Initialize advanced planning system (PRD 5) - uses LLM for actual reasoning
    if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
        println!("🧠 Initializing advanced planning system...");
    }
    orchestrator.initialize_planning(task).await?;

    if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
        if let Some(progress) = orchestrator.planning_progress() {
            println!("📊 Initial planning complete. Progress: {:.1}%", progress * 100.0);
        }
    }
    
    // Set system prompt with tool instructions
    // Build detailed tool descriptions for better model understanding
    let tool_descriptions = vec![
        "list_dir: List files and directories. Args: path (string, required), recursive (bool, optional, default false)",
        "read_file: Read contents of a text file. Args: path (string, required)",
        "write_file: Write or append content to a file. Args: path (string, required), content (string, required), append (bool, optional, default false)",
        "run_command: Execute a system command (supports shell pipes/redirects). Args: command (string, required), args (array of strings, optional), timeout_seconds (number, optional, default 60)",
        "system_info: Get system information. Args: info_type (string, optional: 'os', 'cpu', 'memory', 'disk', 'all', default 'all')",
        "web_fetch: Fetch content from a URL. Args: url (string, required), method (string, optional: 'GET' or 'POST', default 'GET'), timeout_seconds (number, optional, default 30)",
    ];
    
    let tools_formatted = tool_descriptions.join("\n  ");
    
    let system_prompt = format!(r#"You are an autonomous AI agent that helps users complete tasks using available tools.

RESPONSE FORMAT - Think out loud, then provide JSON:

1. First, explain your reasoning (what you're trying to accomplish and why)
2. Then, on a new line, output valid JSON for your action

Tool call format:
{{\"type\": \"tool_call\", \"tool\": \"tool_name\", \"args\": {{\"key\": \"value\"}}}}

Completion format:
{{\"type\": \"final\", \"result\": \"description of what was accomplished\"}}

AVAILABLE TOOLS:
  {}

TOOL SELECTION GUIDELINES:
- list_dir: Use to explore directories and find files
- read_file: Use to read file contents (text files only)
- write_file: Use to create or modify files
- run_command: Use for system commands, file operations, shell pipes (find, grep, wc, etc.)
- system_info: Use to check OS, CPU, memory, disk space
- web_fetch: Use to download web content

CRITICAL RULES:
1. Always explain your thinking before outputting JSON
2. End your response with valid JSON on its own line
3. Use exact tool names from the list above
4. Provide all required arguments as specified
5. When writing code files:
   - Use proper error handling (try/except in Python, proper error checks)
   - NEVER use shell=True in subprocess - use list arguments instead
   - Add input validation and sanitization
   - Include type hints and docstrings
   - Use meaningful variable names
   - Add comments for complex logic
   - Ensure code is production-ready, not just syntactically valid
   - Handle edge cases (empty inputs, missing files, permission errors)
   - Add logging for debugging
6. Before marking task complete, verify:
   - Code runs without errors
   - All edge cases are handled
   - No security vulnerabilities
   - Code follows best practices
7. For shell commands with pipes/redirects, use run_command with full command string

CODE QUALITY EXAMPLES:

BAD (Don't do this):
```python
import subprocess
result = subprocess.check_output("df -h", shell=True)  # Security risk!
data = result.decode().split()[1]  # Will crash if format changes
```

GOOD (Do this instead):
```python
import subprocess
from typing import Optional

def check_disk_usage() -> Optional[str]:
    \"\"\"Check disk usage safely.\"\"\"
    try:
        result = subprocess.run(['df', '-h'], capture_output=True, text=True, check=True)
        lines = result.stdout.strip().split('\\n')
        if len(lines) < 2:
            return None
        return lines[1]
    except (subprocess.CalledProcessError, FileNotFoundError) as e:
        print(f"Error checking disk usage: {{e}}")
        return None
```

TOOL USAGE EXAMPLES:

Example 1:
I need to see what files are in the src directory to understand the project structure.
{{\"type\": \"tool_call\", \"tool\": \"list_dir\", \"args\": {{\"path\": \"src\"}}}}

Example 2:
To count all Rust files, I'll use the find command with a pipe to wc.
{{\"type\": \"tool_call\", \"tool\": \"run_command\", \"args\": {{\"command\": \"find src -name '*.rs' | wc -l\"}}}}

Example 3:
I've successfully counted all the Rust files. The task is complete.
{{\"type\": \"final\", \"result\": \"Found 32 .rs files in src directory\"}}

Now begin!"#, tools_formatted);
    
    orchestrator.add_system_prompt(system_prompt);
    
    // Initialize telemetry
    let telemetry = TelemetryCollector::new();
    let display = TelemetryDisplay::new(telemetry.clone(), args.verbosity());
    
    // 3. Set up agent with task
    orchestrator.add_user_goal(task.to_string());
    
    // PRD 7: Initialize working memory with goal
    orchestrator.set_goal(task.to_string());
    
    println!("OllamaBuddy Agent Starting...");
    println!("Task: {}", task);
    println!("Working Directory: {:?}", working_dir);
    println!("Available Tools: {}", tool_runtime.tool_names().join(", "));
    println!();
    
    // 4. Start state machine
    use ollamabuddy::agent::StateEvent;
    orchestrator.transition(StateEvent::StartSession)?;
    telemetry.record(TelemetryEvent::StateTransition {
        from: "Init".to_string(),
        to: "Planning".to_string(),
        timestamp: std::time::Instant::now(),
    });
    
    // PRD 7: Query memory before planning
    let verbose = matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose);
    if verbose {
        eprintln!("[MEMORY] Checking for similar past episodes...");
    }
    let similar_patterns = orchestrator.find_similar_patterns(task, 0.7);
    if !similar_patterns.is_empty() && verbose {
        eprintln!("[MEMORY] Found {} similar episodes", similar_patterns.len());
    }
    
    // Get tool recommendations from experience
    let available_tools = vec![
        "list_dir".to_string(),
        "read_file".to_string(),
        "write_file".to_string(),
        "run_command".to_string(),
        "system_info".to_string(),
        "web_fetch".to_string(),
    ];
    let tool_recommendations = orchestrator.get_tool_recommendations(task, &available_tools);
    if !tool_recommendations.is_empty() && verbose {
        eprintln!("[MEMORY] Got {} tool recommendations from experience", tool_recommendations.len());
    }
    
    // 5. Main agent loop
    // PRD 8: Initialize dynamic budget manager
    let mut budget_manager = DynamicBudgetManager::new();
    
    // PRD 9: Initialize validation system
    let mut validation_orchestrator = ValidationOrchestrator::new();
    let mut convergence_detector = ConvergenceDetector::new();
    let mut adaptive_recovery = AdaptiveRecovery::new();
    let mut tool_results_log: Vec<ollamabuddy::tools::types::ToolResult> = Vec::new();
    
    // Estimate initial complexity (simple heuristic based on task length and keywords)
    let task_complexity = {
        let base_complexity = (task.len() as f64 / 200.0).min(0.5);
        let keyword_boost = if task.to_lowercase().contains("analyze") ||
                                task.to_lowercase().contains("complex") ||
                                task.to_lowercase().contains("multiple") {
            0.3
        } else {
            0.0
        };
        (base_complexity + keyword_boost).min(1.0)
    };
    
    // Calculate dynamic budget based on complexity
    let max_iterations = budget_manager.calculate_budget(task_complexity);
    
    if verbose {
        eprintln!("[BUDGET] Task complexity: {:.2}, Allocated iterations: {}", task_complexity, max_iterations);
    }
    let mut iteration = 0;
    
    while iteration < max_iterations && !matches!(
        orchestrator.state(), 
        ollamabuddy::agent::AgentState::Final | ollamabuddy::agent::AgentState::Error
    ) {
        iteration += 1;

        // Check context and compress if needed
        let tokens_before = orchestrator.token_count();
        orchestrator.maybe_compress()?;
        let tokens_after = orchestrator.token_count();
        if tokens_before != tokens_after {
            telemetry.record(TelemetryEvent::ContextCompression {
                before_tokens: tokens_before,
                after_tokens: tokens_after,
                timestamp: std::time::Instant::now(),
            });
            println!("Compressed context: {} -> {} tokens", tokens_before, tokens_after);
        }
        
        // Build prompt
        let prompt = orchestrator.build_prompt();
        
        if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
            println!("Prompt ({} tokens)", orchestrator.token_count());
        }
        
        // Stream response from Ollama
        let client = orchestrator.client();
        let mut stream = client.generate_stream(prompt).await?;
        
        // Stream thinking in real-time (no progress bar needed)
        println!("\nAgent:");

        let mut response_text_accumulator = String::new();
        let mut token_count = 0;

        use futures_util::StreamExt;
        use std::io::Write;

        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result?;

            // Extract "response" field from Ollama API format
            if let Ok(ollama_response) = serde_json::from_slice::<serde_json::Value>(&chunk_bytes) {
                if let Some(token) = ollama_response.get("response").and_then(|r| r.as_str()) {
                    response_text_accumulator.push_str(token);
                    token_count += 1;

                    // Report token to telemetry
                    telemetry.record(TelemetryEvent::TokenReceived {
                        token: token.to_string(),
                        timestamp: std::time::Instant::now(),
                    });

                    // Stream thinking text in real-time (always, not just verbose)
                    print!("{}", token);
                    std::io::stdout().flush().ok();
                }
            }
        }

        println!(); // New line after streaming
        
        // Parse accumulated response as AgentMsg
        if !response_text_accumulator.is_empty() {
            let trimmed = response_text_accumulator.trim();

            // Unescape JSON first (model outputs escaped quotes)
            let unescaped = trimmed.replace(r#"\""#, r#"""#);

            // Extract JSON from the response (thinking comes before)
            // Find complete JSON object by matching braces
            let json_str = extract_json_object(&unescaped).unwrap_or(&unescaped);

            if matches!(args.verbosity(), Verbosity::VeryVerbose) {
                eprintln!("\n[DEBUG] Extracted JSON: {}", json_str);
            }

            match serde_json::from_str::<ollamabuddy::types::AgentMsg>(json_str) {
                Ok(agent_msg) => {
                    use ollamabuddy::types::AgentMsg;
                    
                    match agent_msg {
                        AgentMsg::ToolCall { tool, args } => {
                            println!("Tool call: {} with args: {:?}", tool, args);
                            
                            let tool_start = std::time::Instant::now();
                            telemetry.record(TelemetryEvent::ToolStarted {
                                tool: tool.clone(),
                                timestamp: tool_start,
                            });
                            
                            // Transition to executing
                            orchestrator.transition(StateEvent::ToolCall)?;
                            
                            // Show progress bar for tool execution
                            let pb = ProgressBar::new_spinner();
                            pb.set_style(
                                ProgressStyle::default_spinner()
                                    .template("{spinner:.green} {msg}")
                                    .unwrap()
                            );
                            pb.set_message(format!("Executing: {}", tool));
                            pb.enable_steady_tick(std::time::Duration::from_millis(100));
                            
                            // Execute tool
                            let result = tool_runtime.execute(&tool, &serde_json::to_value(&args)?).await;
                            pb.finish_and_clear();
                            
                            match result {
                                Ok(tool_output) => {
                                    let duration = tool_start.elapsed().as_millis() as u64;
                                    telemetry.record(TelemetryEvent::ToolCompleted {
                                        tool: tool.clone(),
                                        duration_ms: duration,
                                        success: true,
                                        timestamp: std::time::Instant::now(),
                                    });
                                    println!("[OK] Tool result ({}ms): {}", duration, tool_output.output);
                                    
                                    // PRD 9: Collect tool result for validation
                                    tool_results_log.push(tool_output.clone());
                                    
                                    // Add tool result to memory
                                    use ollamabuddy::types::MemoryEntry;
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

                                    // Transition: Executing -> Verifying
                                    orchestrator.transition(StateEvent::ToolComplete)?;

                                    // Immediately transition: Verifying -> Planning for next iteration
                                    orchestrator.transition(StateEvent::ContinueIteration)?;
                                }
                                Err(e) => {
                                    // PRD 9 Phase 3: Adaptive recovery on tool failure
                                    eprintln!("[ERROR] Tool execution failed: {}", e);
                                    
                                    // Detect failure pattern
                                    use ollamabuddy::recovery::types::FailureSymptom;
                                    let symptom = FailureSymptom::ToolExecutionFailure {
                                        tool_name: tool.clone(),
                                        consecutive_failures: tool_results_log.iter()
                                            .rev()
                                            .take_while(|r| !r.success && r.tool == tool)
                                            .count() + 1,
                                    };
                                    
                                    if let Some(pattern) = adaptive_recovery.detect_pattern(symptom) {
                                        let action = adaptive_recovery.select_recovery_action(&pattern);
                                        
                                        if verbose {
                                            eprintln!("[RECOVERY] Detected pattern: {:?}", pattern.symptom);
                                            eprintln!("[RECOVERY] Action: {:?}", action);
                                        }
                                        
                                        // Apply recovery action (basic implementation)
                                        match action {
                                            ollamabuddy::recovery::types::RecoveryAction::Abort { reason } => {
                                                eprintln!("[RECOVERY] Aborting: {}", reason);
                                                orchestrator.transition(StateEvent::UnrecoverableError)?;
                                            }
                                            _ => {
                                                // For other actions, transition to ToolFailure and continue
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
                            // PRD 9: Run validation on task completion
                            if !tool_results_log.is_empty() {
                                let expected_outputs = vec![task.to_string()];
                                let validation_result = validation_orchestrator.orchestrate_validation(
                                    &tool_results_log,
                                    &expected_outputs,
                                );
                                
                                if validation_result.success {
                                    if verbose {
                                        eprintln!("[VALIDATION] Task validated successfully (score: {:.2})",
                                            validation_result.validation.score.overall);
                                    }
                                }
                                // Don't show validation warnings - internal metric
                            }
                            
                            println!("\n[SUCCESS] Task Complete!");
                            println!("{}", result);
                            if let Some(sum) = summary {
                                println!("Summary: {}", sum);
                            }
                            orchestrator.transition(StateEvent::GoalAchieved)?;
                            break;
                        }
                        AgentMsg::Plan { steps, reasoning } => {
                            println!("Plan created:");
                            for (i, step) in steps.iter().enumerate() {
                                println!("   {}. {}", i + 1, step);
                            }
                            if let Some(reason) = reasoning {
                                println!("Reasoning: {}", reason);
                            }
                            orchestrator.transition(StateEvent::PlanComplete)?;
                        }
                        AgentMsg::Ask { question } => {
                            println!("Model asks: {}", question);
                            // For now, just continue
                        }
                        AgentMsg::Error { message, recoverable } => {
                            eprintln!("[ERROR] Model error: {}", message);
                            if recoverable {
                                println!("[WARNING] Error is recoverable, continuing...");
                            } else {
                                orchestrator.transition(StateEvent::UnrecoverableError)?;
                            }
                        }
                    }
                }
                Err(e) => {
                    if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
                        eprintln!("\n[WARNING] Parse failed: {}", e);
                        eprintln!("   Text: {}", unescaped);
                    }
                }
            }
        }

        // PRD 9 Phase 2: Track progress and check convergence
        let current_progress = match orchestrator.state() {
            ollamabuddy::agent::AgentState::Final => 1.0,
            ollamabuddy::agent::AgentState::Executing => {
                // Estimate progress based on successful tool executions
                (tool_results_log.iter().filter(|r| r.success).count() as f64 * 0.15).min(0.9)
            },
            _ => iteration as f64 / max_iterations as f64 * 0.5,
        };
        
        convergence_detector.record_progress(current_progress, iteration);
        
        if verbose {
            if let Some(velocity) = convergence_detector.get_velocity() {
                eprintln!("[CONVERGENCE] Progress: {:.2}, Velocity: {:.3}", 
                    current_progress, velocity.velocity);
            }
        }

        // Check for early termination conditions
        let validation_score = if let Some(last_result) = tool_results_log.last() {
            if last_result.success { 0.9 } else { 0.5 }
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
            match termination {
                ollamabuddy::analysis::types::TerminationCondition::Success => {
                    if verbose {
                        eprintln!("[CONVERGENCE] Early success detected at iteration {}", iteration);
                    }
                    break;
                }
                ollamabuddy::analysis::types::TerminationCondition::Stagnation => {
                    eprintln!("[CONVERGENCE] Stagnation detected at iteration {}", iteration);
                    break;
                }
                _ => {}
            }
        }
    }
    
    if iteration >= max_iterations {
        println!("\n[WARNING] Maximum iterations reached");
    }

    println!("\nAgent finished");
    
    // Display telemetry summary
    println!();
    display.display_summary();
    
    
    // PRD 7: Record episode at session end
    let session_success = !matches!(
        orchestrator.state(),
        ollamabuddy::agent::AgentState::Error
    );
    
    orchestrator.record_episode(
        task.to_string(),
        session_success,
        if session_success { None } else { Some("Task incomplete or failed".to_string()) }
    );
    
    if verbose {
        eprintln!("[MEMORY] Session episode recorded");
    }
    
        Ok(())
}

async fn run_doctor(args: &Args) -> Result<()> {
    let doctor = Doctor::new(
        args.host.clone(),
        args.port,
        args.model.clone(),
    );

    let report = doctor.run_checks().await?;
    report.print();

    std::process::exit(if report.is_healthy() { 0 } else { 1 });
}


/// Handle 'models list' command
async fn handle_models_list(_args: &Args) -> Result<()> {
    use colored::Colorize;
    use ollamabuddy::models::ModelOperation;
    use ollamabuddy::config::Config;
    
    let manager = ModelManager::new(None);
    
    // Check if Ollama is available
    if !manager.is_ollama_available().await {
        eprintln!("{}", "Error: Cannot connect to Ollama server".red());
        eprintln!("Make sure Ollama is running: ollama serve");
        return Ok(());
    }
    
    // Load config to get default model
    let config = Config::load().ok();
    let default_model = config.as_ref().and_then(|c| c.get_default_model());
    
    match manager.list_models().await {
        ModelOperation::List(models) => {
            if models.is_empty() {
                println!("{}", "No models installed".yellow());
                println!("
Install a model with:");
                println!("  ollamabuddy models pull <model-name>");
                return Ok(());
            }
            
            println!("{}", "Installed Ollama Models:".bright_blue().bold());
            println!();
            
            for model in models {
                let description = model.description();
                let is_default = default_model.map_or(false, |d| d == model.name);
                let marker = if is_default { " *".green().bold() } else { "".normal() };
                
                println!("  - {}{}", model.name.bright_white().bold(), marker);
                println!("    Size: {} | {}", model.formatted_size(), description.dimmed());
                
                if let Some(details) = &model.details {
                    if let Some(family) = &details.family {
                        println!("    Family: {}", family.dimmed());
                    }
                }
                println!();
            }
            
            if default_model.is_some() {
                println!("{}", "  * = default model".dimmed());
            }
        }
        ModelOperation::Error(e) => {
            eprintln!("{} {}", "Error:".red(), e);
        }
        _ => unreachable!(),
    }
    
    Ok(())
}

/// Handle 'models pull' command
async fn handle_models_pull(_args: &Args, name: &str) -> Result<()> {
    use colored::Colorize;
    use indicatif::{ProgressBar, ProgressStyle};
    use ollamabuddy::models::{ModelOperation, PullProgress};
    
    let manager = ModelManager::new(None);
    
    // Check if Ollama is available
    if !manager.is_ollama_available().await {
        eprintln!("{}", "Error: Cannot connect to Ollama server".red());
        eprintln!("Make sure Ollama is running: ollama serve");
        return Ok(());
    }
    
    println!("{} {}", "Pulling model:".bright_blue().bold(), name.bright_white());
    println!();
    
    // Create progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}% {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    
    // Clone pb for use in closure
    let pb_clone = pb.clone();
    let progress_callback = Box::new(move |progress: &PullProgress| {
        if let (Some(total), Some(completed)) = (progress.total, progress.completed) {
            let percentage = ((completed as f64 / total as f64) * 100.0) as u64;
            pb_clone.set_position(percentage);
            pb_clone.set_message(progress.status.clone());
        } else {
            pb_clone.set_message(progress.status.clone());
        }
    });
    
    match manager.pull_model(name, Some(progress_callback)).await {
        ModelOperation::Pulled(_) => {
            pb.finish_with_message("Complete");
            println!();
            println!("{} {}", "Successfully pulled:".bright_green().bold(), name.bright_white());
        }
        ModelOperation::Error(e) => {
            pb.finish_and_clear();
            eprintln!("{} {}", "Error:".red(), e);
        }
        _ => unreachable!(),
    }
    
    Ok(())
}

/// Handle 'models delete' command
async fn handle_models_delete(_args: &Args, name: &str, force: bool) -> Result<()> {
    use colored::Colorize;
    use std::io::{self, Write};
    use ollamabuddy::models::ModelOperation;
    
    let manager = ModelManager::new(None);
    
    // Check if Ollama is available
    if !manager.is_ollama_available().await {
        eprintln!("{}", "Error: Cannot connect to Ollama server".red());
        eprintln!("Make sure Ollama is running: ollama serve");
        return Ok(());
    }
    
    // Check if model exists
    if !manager.model_exists(name).await {
        eprintln!("{} Model '{}' not found", "Error:".red(), name);
        return Ok(());
    }
    
    // Get confirmation unless --force
    if !force {
        print!("{} Delete model '{}'? This cannot be undone. [y/N]: ", 
               "Warning:".yellow().bold(), name.bright_white());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }
    
    println!("Deleting model: {}", name);
    
    match manager.delete_model(name, force).await {
        ModelOperation::Deleted(_) => {
            println!("{} {}", "Successfully deleted:".bright_green().bold(), name.bright_white());
        }
        ModelOperation::Error(e) => {
            eprintln!("{} {}", "Error:".red(), e);
        }
        _ => unreachable!(),
    }
    
    Ok(())
}

/// Handle 'models info' command
async fn handle_models_info(_args: &Args, name: &str) -> Result<()> {
    use colored::Colorize;
    use ollamabuddy::models::ModelOperation;
    
    let manager = ModelManager::new(None);
    
    // Check if Ollama is available
    if !manager.is_ollama_available().await {
        eprintln!("{}", "Error: Cannot connect to Ollama server".red());
        eprintln!("Make sure Ollama is running: ollama serve");
        return Ok(());
    }
    
    match manager.show_model(name).await {
        ModelOperation::Info(info) => {
            println!("{}", "Model Information:".bright_blue().bold());
            println!();
            println!("  {} {}", "Name:".bold(), info.name.bright_white());
            println!("  {} {}", "Size:".bold(), info.formatted_size());
            println!("  {} {}", "Modified:".bold(), info.modified_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  {} {}", "Digest:".bold(), &info.digest[..16].dimmed());
            
            if let Some(details) = &info.details {
                println!();
                println!("  {}", "Details:".bold());
                
                if let Some(format) = &details.format {
                    println!("    {} {}", "Format:".dimmed(), format);
                }
                if let Some(family) = &details.family {
                    println!("    {} {}", "Family:".dimmed(), family);
                }
                if let Some(params) = &details.parameter_size {
                    println!("    {} {}", "Parameters:".dimmed(), params);
                }
                if let Some(quant) = &details.quantization_level {
                    println!("    {} {}", "Quantization:".dimmed(), quant);
                }
            }
            
            println!();
        }
        ModelOperation::Error(e) => {
            eprintln!("{} {}", "Error:".red(), e);
        }
        _ => unreachable!(),
    }
    
    Ok(())
}



async fn handle_models_use(_args: &Args, name: &str) -> Result<()> {
    use colored::Colorize;
    use ollamabuddy::models::{ModelManager, ModelOperation};
    use ollamabuddy::config::Config;
    
    let manager = ModelManager::new(None);
    
    // Check if Ollama is available
    if !manager.is_ollama_available().await {
        eprintln!("{}", "Error: Ollama is not running".red());
        eprintln!("Start Ollama first: {}", "ollama serve".dimmed());
        return Ok(());
    }
    
    // Verify model exists
    if manager.model_exists(name).await {
        // Model exists, set as default
        let mut config = Config::load()?;
        config.set_default_model(name.to_string());
        config.save()?;
        
        println!("{} {}", "[OK] Default model set to:".green(), name.bold());
        println!("");
        println!("{}", "The new model will be used for all future sessions.".dimmed());
    } else {
        eprintln!("{} '{}'", "Error: Model not found".red(), name);
        eprintln!("");
        eprintln!("Available models:");
        
        // List available models
        match manager.list_models().await {
            ModelOperation::List(models) => {
                for model in models {
                    eprintln!("  - {}", model.name);
                }
            }
            _ => {
                eprintln!("  (Could not retrieve model list)");
            }
        }
        
        eprintln!("");
        eprintln!("Pull a model first: {}", format!("ollamabuddy models pull {}", name).dimmed());
    }
    
    Ok(())
}

async fn handle_models_current(_args: &Args) -> Result<()> {
    use colored::Colorize;
    use ollamabuddy::config::Config;
    
    let config = Config::load()?;
    
    match config.get_default_model() {
        Some(model) => {
            println!("{} {}", "Current default model:".blue().bold(), model.green());
        }
        None => {
            println!("{}", "No default model set".yellow());
            println!("");
            println!("{}", "Using Ollama's default model selection.".dimmed());
            println!("Set a default with: {}", "ollamabuddy models use <name>".dimmed());
        }
    }
    
    Ok(())
}

async fn clean_state(_args: &Args, _logs: bool) -> Result<()> {
    use tokio::fs;

    let state_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ollamabuddy");

    if state_dir.exists() {
        fs::remove_dir_all(&state_dir).await?;
        println!("[OK] Cleaned state directory: {:?}", state_dir);
    } else {
        println!("No state directory found.");
    }

    Ok(())
}

fn show_config(args: &Args) -> Result<()> {
    println!("
╔═══════════════════════════════════════════════════════╗");
    println!("║ OllamaBuddy Configuration                             ║");
    println!("╚═══════════════════════════════════════════════════════╝
");

    println!("Ollama:");
    println!("  Host:  {}", args.host);
    println!("  Port:  {}", args.port);
    println!("  Model: {}", args.model);
    println!();

    if let Some(cwd) = &args.cwd {
        println!("Working Directory:");
        println!("  {:?}", cwd);
        println!();
    }

    println!("Features:");
    println!("  Online mode:    {}", if args.online { "enabled" } else { "disabled" });
    println!("  Auto-upgrade:   {}", if args.auto_upgrade { "enabled" } else { "disabled" });
    println!("  Verbosity:      {:?}", args.verbosity());
    println!();

    Ok(())
}






