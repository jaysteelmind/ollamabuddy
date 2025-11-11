//! OllamaBuddy v0.2 - Main CLI Entry Point

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use ollamabuddy::{
    cli::{Args, Commands, Verbosity},
    bootstrap::Bootstrap,
    doctor::Doctor,
    agent::AgentOrchestrator,
    agent::orchestrator::AgentConfig,
    tools::ToolRuntime,
    telemetry::{TelemetryCollector, TelemetryEvent, TelemetryDisplay},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Some(Commands::Doctor) => {
            run_doctor(&args).await?;
        }
        Some(Commands::Models) => {
            list_models(&args).await?;
        }
        Some(Commands::Clean { logs }) => {
            clean_state(&args, *logs).await?;
        }
        Some(Commands::Config) => {
            show_config(&args)?;
        }
        None => {
            // No subcommand - run main agent with task from args
            if let Some(task) = &args.task {
                run_agent(&args, task).await?;
            } else {
                println!("OllamaBuddy v0.2.1 - Terminal Agent");
                println!("
Usage:");
                println!("  ollamabuddy \"<task>\"          Run agent with task");
                println!("  ollamabuddy doctor            System health checks");
                println!("  ollamabuddy models            List Ollama models");
                println!("  ollamabuddy config            Show configuration");
                println!("  ollamabuddy clean             Clear state/logs");
                println!("
Example:");
                println!("  ollamabuddy \"List all .rs files and count lines of code\"");
                println!();
            }
        }
    }

    Ok(())
}


async fn run_agent(args: &Args, task: &str) -> Result<()> {
    use std::path::PathBuf;
    
    // 1. Bootstrap check
    let bootstrap = Bootstrap::new(
        args.host.clone(),
        args.port,
        args.model.clone(),
    );
    
    if !bootstrap.check_ollama_running().await? {
        eprintln!("❌ Ollama is not running!");
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
        max_iterations: 10,
        verbose: matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose),
    };
    
    let mut orchestrator = AgentOrchestrator::new(config)?;
    let tool_runtime = ToolRuntime::new(&working_dir)?;
    
    // Set system prompt with tool instructions
    let system_prompt = format!(
        "You are an autonomous AI agent that helps users complete tasks by using available tools.

CRITICAL INSTRUCTIONS:
- You MUST respond ONLY with valid JSON
- Use tools by outputting JSON in this exact format: {{"type": "tool_call", "tool": "tool_name", "args": {{...}}}}
- When task is complete, output: {{"type": "final", "result": "description of what was accomplished"}}
- NEVER output plain text explanations or commands
- NEVER use markdown code blocks

Available tools: {}

Example tool call:
{{"type": "tool_call", "tool": "list_dir", "args": {{"path": "src"}}}}

Example completion:
{{"type": "final", "result": "Found 29 .rs files in src directory"}}

Remember: OUTPUT ONLY JSON, NO OTHER TEXT.",
        tool_runtime.tool_names().join(", ")
    );
    
    orchestrator.add_system_prompt(system_prompt);
    
    // Initialize telemetry
    let telemetry = TelemetryCollector::new();
    let display = TelemetryDisplay::new(telemetry.clone(), args.verbosity());
    
    // 3. Set up agent with task
    orchestrator.add_user_goal(task.to_string());
    
    println!("🤖 OllamaBuddy Agent Starting...");
    println!("📋 Task: {}", task);
    println!("📁 Working Directory: {:?}", working_dir);
    println!("🔧 Available Tools: {}", tool_runtime.tool_names().join(", "));
    println!();
    
    // 4. Start state machine
    use ollamabuddy::agent::StateEvent;
    orchestrator.transition(StateEvent::StartSession)?;
    telemetry.record(TelemetryEvent::StateTransition {
        from: "Init".to_string(),
        to: "Planning".to_string(),
        timestamp: std::time::Instant::now(),
    });
    
    // 5. Main agent loop
    let max_iterations = 10;
    let mut iteration = 0;
    
    while iteration < max_iterations && !matches!(
        orchestrator.state(), 
        ollamabuddy::agent::AgentState::Final | ollamabuddy::agent::AgentState::Error
    ) {
        iteration += 1;
        println!("
\n=== Iteration {} ===", iteration);
        
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
            println!("🗜️  Compressed context: {} → {} tokens", tokens_before, tokens_after);
        }
        
        // Build prompt
        let prompt = orchestrator.build_prompt();
        
        if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
            println!("📝 Prompt ({} tokens)", orchestrator.token_count());
        }
        
        // Stream response from Ollama
        let client = orchestrator.client();
        let mut stream = client.generate_stream(prompt).await?;
        
        let thinking_pb = ProgressBar::new_spinner();
        thinking_pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        thinking_pb.set_message("Model thinking...");
        thinking_pb.enable_steady_tick(std::time::Duration::from_millis(100));
        
        let mut response_text = String::new();
        let mut token_count = 0;
        
        use futures_util::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result?;
            
            // Parse streaming JSON
            let parser = orchestrator.parser_mut();
            if let Some(json_str) = parser.add_bytes(&chunk_bytes)? {
                // Try to parse as agent message
                if let Ok(agent_msg) = parser.parse_agent_msg(&json_str) {
                    use ollamabuddy::types::AgentMsg;
                    
                    match agent_msg {
                        AgentMsg::ToolCall { tool, args } => {
                            println!("🔧 Tool call: {} with args: {:?}", tool, args);
                            
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
                                    println!("✅ Tool result ({}ms): {}", duration, tool_output.output);
                                    
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
                                    
                                    // Transition to planning for next iteration
                                    orchestrator.transition(StateEvent::ToolComplete)?;
                                }
                                Err(e) => {
                                    eprintln!("❌ Tool execution failed: {}", e);
                                    orchestrator.transition(StateEvent::ToolFailure)?;
                                }
                            }
                        }
                        AgentMsg::Final { result, summary } => {
                            println!("\n✅ Task Complete!");
                            println!("{}", result);
                            if let Some(sum) = summary {
                                println!("Summary: {}", sum);
                            }
                            orchestrator.transition(StateEvent::GoalAchieved)?;
                            break;
                        }
                        AgentMsg::Plan { steps, reasoning } => {
                            println!("📋 Plan created:");
                            for (i, step) in steps.iter().enumerate() {
                                println!("   {}. {}", i + 1, step);
                            }
                            if let Some(reason) = reasoning {
                                println!("💭 Reasoning: {}", reason);
                            }
                            orchestrator.transition(StateEvent::PlanComplete)?;
                        }
                        AgentMsg::Ask { question } => {
                            println!("❓ Model asks: {}", question);
                            // For now, just continue
                        }
                        AgentMsg::Error { message, recoverable } => {
                            eprintln!("❌ Model error: {}", message);
                            if recoverable {
                                println!("⚠️  Error is recoverable, continuing...");
                            } else {
                                orchestrator.transition(StateEvent::UnrecoverableError)?;
                            }
                        }
                    }
                } else {
                    // Plain text response
                    response_text.push_str(&json_str);
                    if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
                        print!("{}", json_str);
                    }
                }
            }
        }
        
        thinking_pb.finish_and_clear();
        
        if !response_text.is_empty() && matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
            println!();
        }
    }
    
    if iteration >= max_iterations {
        println!("\n⚠️  Maximum iterations reached");
    }
    
    println!("\n🏁 Agent finished");
    
    // Display telemetry summary
    println!();
    display.display_summary();
    
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

async fn list_models(args: &Args) -> Result<()> {
    let bootstrap = Bootstrap::new(
        args.host.clone(),
        args.port,
        args.model.clone(),
    );

    println!("
Checking Ollama models...
");

    match bootstrap.list_models().await {
        Ok(models) => {
            if models.is_empty() {
                println!("No models installed.");
                println!("
Pull a model with:");
                println!("  ollama pull qwen2.5:7b-instruct");
            } else {
                println!("Available models:");
                for model in models {
                    println!("  • {}", model);
                }
            }
            println!();
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("
Is Ollama running? Start with: ollama serve");
            std::process::exit(1);
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
        println!("✓ Cleaned state directory: {:?}", state_dir);
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
