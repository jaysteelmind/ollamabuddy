//! OllamaBuddy v0.2 - Main CLI Entry Point

use anyhow::Result;
use clap::Parser;
use ollamabuddy::{
    cli::{Args, Commands, Verbosity},
    bootstrap::Bootstrap,
    doctor::Doctor,
    agent::AgentOrchestrator,
    agent::orchestrator::AgentConfig,
    tools::ToolRuntime,
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
        orchestrator.maybe_compress()?;
        
        // Build prompt
        let prompt = orchestrator.build_prompt();
        
        if matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
            println!("📝 Prompt ({} tokens)", orchestrator.token_count());
        }
        
        // Stream response from Ollama
        let client = orchestrator.client();
        let mut stream = client.generate_stream(prompt).await?;
        
        println!("🤔 Model thinking...");
        
        let mut response_text = String::new();
        
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
                            
                            // Transition to executing
                            orchestrator.transition(StateEvent::ToolCall)?;
                            
                            // Execute tool
                            let result = tool_runtime.execute(&tool, &serde_json::to_value(&args)?).await;
                            
                            match result {
                                Ok(tool_output) => {
                                    println!("✅ Tool result: {}", tool_output.output);
                                    
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
        
        if !response_text.is_empty() && matches!(args.verbosity(), Verbosity::Verbose | Verbosity::VeryVerbose) {
            println!("\n");
        }
    }
    
    if iteration >= max_iterations {
        println!("\n⚠️  Maximum iterations reached");
    }
    
    println!("\n🏁 Agent finished");
    
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
