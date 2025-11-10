//! OllamaBuddy v0.2 - Main Entry Point
//! 
//! This is a stub demonstrating the PRD 3 architecture.
//! Full integration of all components will be completed in subsequent phases.

use clap::Parser;
use ollamabuddy::{
    cli::{Args, Commands, Verbosity},
    bootstrap::{BootstrapDetector, BootstrapStatus, EXIT_CODE_SETUP_NEEDED},
    doctor::Doctor,
    advisor::ModelAdvisor,
    telemetry::TelemetryCollector,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = Args::parse();

    // Validate arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        eprintln!("\nUsage: ollamabuddy <TASK> [OPTIONS]");
        eprintln!("   or: ollamabuddy <COMMAND>");
        eprintln!("\nRun 'ollamabuddy --help' for more information.");
        std::process::exit(1);
    }

    // Handle subcommands
    if let Some(ref command) = args.command {
        return handle_subcommand(command, &args).await;
    }

    // Main agent execution path (stub)
    println!("🤖 OllamaBuddy v0.2");
    println!("Task: {}", args.task.as_ref().unwrap());
    println!("Model: {}", args.model);
    println!("Verbosity: {:?}", args.verbosity());
    println!("\n⚠️  Full agent integration coming in next phase.");
    println!("✅ PRD 3 architecture complete:");
    println!("   - Configuration system");
    println!("   - CLI argument parsing");
    println!("   - Bootstrap detection");
    println!("   - Doctor diagnostics");
    println!("   - Model advisor");
    println!("   - Telemetry system");
    
    Ok(())
}

/// Handle subcommands
async fn handle_subcommand<'a>(
    command: &Commands,
    args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Doctor => {
            println!("🔍 Running system diagnostics...\n");
            let doctor = Doctor::new(
                args.ollama_url(),
                args.working_dir().to_string_lossy().to_string(),
            );
            let checks = doctor.run_diagnostics().await;
            Doctor::display_results(&checks);
            
            if !Doctor::overall_status(&checks) {
                std::process::exit(1);
            }
        }
        
        Commands::Models => {
            println!("📦 Listing available models...\n");
            let detector = BootstrapDetector::new(args.ollama_url());
            
            match detector.check_ollama_running().await {
                Ok(true) => {
                    match detector.list_models().await {
                        Ok(models) => {
                            if models.is_empty() {
                                println!("No models installed.");
                                println!("\nTo install a model:");
                                println!("  ollama pull qwen2.5:7b-instruct");
                            } else {
                                println!("Available models:");
                                for model in models {
                                    println!("  • {}", model);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error listing models: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Ok(false) => {
                    BootstrapDetector::show_ollama_install_instructions();
                    std::process::exit(EXIT_CODE_SETUP_NEEDED);
                }
                Err(e) => {
                    eprintln!("Error checking Ollama: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Clean { logs } => {
            println!("🧹 Cleaning OllamaBuddy state...");
            if *logs {
                println!("   - Removing logs");
            }
            println!("   - Removing temporary files");
            println!("\n✅ Cleanup complete");
        }
        
        Commands::Config => {
            println!("⚙️  Current configuration:");
            println!("\nOllama:");
            println!("  Host: {}", args.host);
            println!("  Port: {}", args.port);
            println!("  Model: {}", args.model);
            println!("\nSettings:");
            println!("  Working directory: {}", args.working_dir().display());
            println!("  Online mode: {}", args.online);
            println!("  Auto-upgrade: {}", args.auto_upgrade);
            println!("  Verbosity: {:?}", args.verbosity());
        }
    }
    
    Ok(())
}
