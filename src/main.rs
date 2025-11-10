//! OllamaBuddy v0.2 - Main CLI Entry Point

use anyhow::Result;
use clap::Parser;
use ollamabuddy::{
    cli::{Args, Commands},
    core::{Bootstrap, Doctor},
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
            println!("OllamaBuddy v0.2.0");
            println!("
PRD 1 & 2 Complete - Full agent coming in v0.2.1");
            println!("
Available commands:");
            println!("  doctor  - Run system health checks");
            println!("  models  - List available Ollama models");
            println!("  config  - Show current configuration");
            println!("  clean   - Clear state and logs");
            println!("
For help: ollamabuddy --help");
        }
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
