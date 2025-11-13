//! Display manager for REPL terminal UI
//! 
//! Manages progress bars, formatted output, and real-time updates
//! Performance target: 10 FPS progress updates

use colored::*;
use crossterm::{
    cursor,
    execute,
    terminal::{Clear, ClearType},
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::time::Duration;

/// Display manager for REPL UI
/// 
/// Features:
/// - Multi-line progress bars
/// - Stage-based progress tracking
/// - In-place terminal updates
/// - Color-coded output
pub struct DisplayManager {
    multi_progress: MultiProgress,
    current_bar: Option<ProgressBar>,
    update_interval: Duration,
}

impl DisplayManager {
    /// Create new display manager
    /// 
    /// Update frequency: 10 FPS (100ms interval)
    pub fn new() -> Self {
        DisplayManager {
            multi_progress: MultiProgress::new(),
            current_bar: None,
            update_interval: Duration::from_millis(100), // 10 FPS
        }
    }
    
    /// Show welcome banner
    pub fn show_banner(&self, version: &str, model: &str) {
        let width = 64;
        let top = format!("{}", "=".repeat(width).cyan());
        let title = format!("  OllamaBuddy {} - Interactive Terminal Agent", version);
        let info = format!("  Model: {} | Memory: Enabled | Mode: REPL", model);
        let bottom = format!("{}", "=".repeat(width).cyan());
        
        println!("\n{}", top);
        println!("{}", title.bold().cyan());
        println!("{}", info.dimmed());
        println!("{}\n", bottom);
        println!("Type your request (or {} for commands, {} to quit)\n", 
            "/help".green(), "/exit".green());
    }
    
    /// Create progress bar for planning stage
    pub fn start_planning(&mut self, task: &str) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} Planning... [{bar:40.cyan/blue}] {pos}% | {msg}")
                .unwrap()
                .progress_chars("=>-")
        );
        pb.set_message(format!("Task: {}", task));
        pb.enable_steady_tick(self.update_interval);
        
        self.current_bar = Some(pb.clone());
        pb
    }
    
    /// Create progress bar for execution stage
    pub fn start_execution(&mut self, tool: &str) -> ProgressBar {
        // Finish previous bar if exists
        self.finish_current();
        
        let pb = self.multi_progress.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} Executing... [{bar:40.green/blue}] {pos}% | Tool: {msg}")
                .unwrap()
                .progress_chars("=>-")
        );
        pb.set_message(tool.to_string());
        pb.enable_steady_tick(self.update_interval);
        
        self.current_bar = Some(pb.clone());
        pb
    }
    
    /// Create progress bar for validation stage
    pub fn start_validation(&mut self) -> ProgressBar {
        self.finish_current();
        
        let pb = self.multi_progress.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.yellow} Validation... [{bar:40.yellow/blue}] {pos}% | {msg}")
                .unwrap()
                .progress_chars("=>-")
        );
        pb.set_message("Checking quality...");
        pb.enable_steady_tick(self.update_interval);
        
        self.current_bar = Some(pb.clone());
        pb
    }
    
    /// Create progress bar for iteration tracking
    pub fn create_iteration_bar(&mut self, max_iterations: usize) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(max_iterations as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("Iteration [{bar:40.cyan/blue}] {pos}/{len} | ETA: {eta}")
                .unwrap()
                .progress_chars("=>-")
        );
        pb
    }
    
    /// Update progress bar
    pub fn update_progress(&self, pb: &ProgressBar, progress: f64, message: Option<&str>) {
        let pos = (progress * 100.0).round() as u64;
        pb.set_position(pos);
        if let Some(msg) = message {
            pb.set_message(msg.to_string());
        }
    }
    
    /// Finish current progress bar
    pub fn finish_current(&mut self) {
        if let Some(pb) = self.current_bar.take() {
            pb.finish_and_clear();
        }
    }
    
    /// Finish progress bar with success message
    pub fn finish_with_success(&mut self, message: &str, duration_ms: u64) {
        if let Some(pb) = self.current_bar.take() {
            pb.finish_and_clear();
        }
        println!("{} {} {}", 
            "✓".green(),
            message,
            format!("({}ms)", duration_ms).dimmed()
        );
    }
    
    /// Finish progress bar with error message
    pub fn finish_with_error(&mut self, message: &str) {
        if let Some(pb) = self.current_bar.take() {
            pb.finish_and_clear();
        }
        println!("{} {}", "✗".red(), message.red());
    }
    
    /// Display streaming tokens
    pub fn stream_token(&self, token: &str) {
        print!("{}", token);
        io::stdout().flush().unwrap();
    }
    
    /// Display task result
    pub fn show_result(&self, result: &str, success: bool) {
        println!();
        if success {
            println!("{} {}", "✓".green().bold(), "Task Complete!".green().bold());
        } else {
            println!("{} {}", "✗".red().bold(), "Task Failed".red().bold());
        }
        
        if !result.is_empty() {
            println!("\n{}", result);
        }
        println!();
    }
    
    /// Display validation summary
    pub fn show_validation(&self, success: bool, score: f64) {
        let status = if success {
            format!("PASSED (score: {:.2})", score).green()
        } else {
            format!("FAILED (score: {:.2})", score).red()
        };
        
        println!("{} Validation: {}", 
            if success { "✓" } else { "✗" },
            status
        );
    }
    
    /// Display error message
    pub fn show_error(&self, error: &str) {
        println!("{} {}", "Error:".red().bold(), error.red());
    }
    
    /// Display warning message
    pub fn show_warning(&self, warning: &str) {
        println!("{} {}", "Warning:".yellow().bold(), warning.yellow());
    }
    
    /// Display info message
    pub fn show_info(&self, info: &str) {
        println!("{} {}", "Info:".cyan(), info);
    }
    
    /// Display debug message (only if verbose)
    pub fn show_debug(&self, debug: &str, verbose: bool) {
        if verbose {
            println!("{} {}", "Debug:".dimmed(), debug.dimmed());
        }
    }
    
    /// Display prompt for user input
    pub fn show_prompt(&self) -> io::Result<()> {
        print!("{}", ">ollamabuddy: ".green().bold());
        io::stdout().flush()
    }
    
    /// Clear screen
    pub fn clear_screen(&self) -> io::Result<()> {
        execute!(
            io::stdout(),
            Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
    }
    
    /// Show thinking indicator
    pub fn show_thinking(&self, stage: &str) {
        println!("{} {}...", "→".cyan(), stage.dimmed());
    }
    
    /// Show completion with stats
    pub fn show_completion_stats(&self, duration_ms: u64, iterations: usize, success: bool) {
        let status = if success { "Success".green() } else { "Failed".red() };
        let duration_str = if duration_ms > 1000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else {
            format!("{}ms", duration_ms)
        };
        
        println!("\n{} {} | Time: {} | Iterations: {}",
            if success { "✓" } else { "✗" },
            status.bold(),
            duration_str.dimmed(),
            iterations.to_string().dimmed()
        );
        println!();
    }
    
    /// Show section header
    pub fn show_section(&self, title: &str) {
        println!("\n{}", title.bold().cyan());
        println!("{}", "-".repeat(60).cyan());
    }
    
    /// Show bullet point
    pub fn show_bullet(&self, text: &str) {
        println!("  {} {}", "•".cyan(), text);
    }
    
    /// Show numbered item
    pub fn show_numbered(&self, index: usize, text: &str) {
        println!("  {}. {}", index.to_string().cyan(), text);
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_manager_creation() {
        let manager = DisplayManager::new();
        assert!(manager.current_bar.is_none());
    }

    #[test]
    fn test_start_planning() {
        let mut manager = DisplayManager::new();
        let pb = manager.start_planning("test task");
        assert!(manager.current_bar.is_some());
        pb.finish_and_clear();
    }

    #[test]
    fn test_start_execution() {
        let mut manager = DisplayManager::new();
        let pb = manager.start_execution("test_tool");
        assert!(manager.current_bar.is_some());
        pb.finish_and_clear();
    }

    #[test]
    fn test_start_validation() {
        let mut manager = DisplayManager::new();
        let pb = manager.start_validation();
        assert!(manager.current_bar.is_some());
        pb.finish_and_clear();
    }

    #[test]
    fn test_finish_current() {
        let mut manager = DisplayManager::new();
        let _pb = manager.start_planning("test");
        assert!(manager.current_bar.is_some());
        
        manager.finish_current();
        assert!(manager.current_bar.is_none());
    }

    #[test]
    fn test_update_progress() {
        let mut manager = DisplayManager::new();
        let pb = manager.start_planning("test");
        
        manager.update_progress(&pb, 0.5, Some("halfway"));
        assert_eq!(pb.position(), 50);
        
        pb.finish_and_clear();
    }

    #[test]
    fn test_finish_with_success() {
        let mut manager = DisplayManager::new();
        let _pb = manager.start_planning("test");
        
        manager.finish_with_success("Task completed", 1234);
        assert!(manager.current_bar.is_none());
    }

    #[test]
    fn test_finish_with_error() {
        let mut manager = DisplayManager::new();
        let _pb = manager.start_planning("test");
        
        manager.finish_with_error("Task failed");
        assert!(manager.current_bar.is_none());
    }

    #[test]
    fn test_create_iteration_bar() {
        let mut manager = DisplayManager::new();
        let pb = manager.create_iteration_bar(10);
        assert_eq!(pb.length().unwrap(), 10);
        pb.finish_and_clear();
    }

    #[test]
    fn test_update_interval() {
        let manager = DisplayManager::new();
        assert_eq!(manager.update_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_multiple_stage_transitions() {
        let mut manager = DisplayManager::new();
        
        // Planning
        let _pb1 = manager.start_planning("test");
        assert!(manager.current_bar.is_some());
        
        // Execution (should finish planning)
        let _pb2 = manager.start_execution("tool");
        assert!(manager.current_bar.is_some());
        
        // Validation (should finish execution)
        let _pb3 = manager.start_validation();
        assert!(manager.current_bar.is_some());
        
        manager.finish_current();
    }

    #[test]
    fn test_show_result_success() {
        let manager = DisplayManager::new();
        manager.show_result("Result text", true);
    }

    #[test]
    fn test_show_result_failure() {
        let manager = DisplayManager::new();
        manager.show_result("Error text", false);
    }

    #[test]
    fn test_show_validation() {
        let manager = DisplayManager::new();
        manager.show_validation(true, 0.95);
        manager.show_validation(false, 0.65);
    }

    #[test]
    fn test_message_display() {
        let manager = DisplayManager::new();
        manager.show_error("Test error");
        manager.show_warning("Test warning");
        manager.show_info("Test info");
        manager.show_debug("Test debug", true);
        manager.show_debug("Hidden debug", false);
    }

    #[test]
    fn test_completion_stats() {
        let manager = DisplayManager::new();
        manager.show_completion_stats(1234, 5, true);
        manager.show_completion_stats(567, 3, false);
    }
}
