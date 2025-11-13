// PRD 11 Phase 4: REPL Commands for knowledge and session management
use anyhow::Result;
use colored::Colorize;

use crate::integration::agent::RAGAgent;
use crate::memory::knowledge_manager::KnowledgeCategory;

/// Knowledge and session management commands
pub struct KnowledgeCommands;

impl KnowledgeCommands {
    /// Display memory system status
    pub async fn memory_status(agent: &RAGAgent) -> Result<()> {
        println!("\n{}", "=== Memory System Status ===".bold().cyan());
        
        // Knowledge counts
        println!("\n{}:", "Knowledge Base".bold());
        let episodes = agent.knowledge_count(KnowledgeCategory::Episode).await?;
        let knowledge = agent.knowledge_count(KnowledgeCategory::Knowledge).await?;
        let code = agent.knowledge_count(KnowledgeCategory::Code).await?;
        let documents = agent.knowledge_count(KnowledgeCategory::Document).await?;
        let total = agent.total_knowledge_count().await?;
        
        println!("  Episodes:  {}", episodes.to_string().green());
        println!("  Knowledge: {}", knowledge.to_string().green());
        println!("  Code:      {}", code.to_string().green());
        println!("  Documents: {}", documents.to_string().green());
        println!("  {}: {}", "Total".bold(), total.to_string().green().bold());
        
        // Session statistics
        println!("\n{}:", "Current Session".bold());
        let session_stats = agent.session_stats().await;
        println!("  Total tasks:      {}", session_stats.total_tasks.to_string().green());
        println!("  Successful:       {}", session_stats.successful_tasks.to_string().green());
        println!("  Failed:           {}", session_stats.failed_tasks.to_string().yellow());
        println!("  Success rate:     {:.1}%", (session_stats.success_rate * 100.0).to_string().green());
        
        // Cumulative statistics
        println!("\n{}:", "All-Time Statistics".bold());
        let cumulative = agent.cumulative_stats().await;
        println!("  Total sessions:   {}", cumulative.total_sessions.to_string().cyan());
        println!("  Total tasks:      {}", cumulative.total_tasks.to_string().cyan());
        println!("  Success rate:     {:.1}%", (cumulative.success_rate * 100.0).to_string().green());
        println!("  Avg tasks/session: {:.1}", cumulative.avg_tasks_per_session.to_string().cyan());
        
        // RAG status
        println!("\n{}:", "RAG Pipeline".bold());
        let rag_status = if agent.is_enabled() { "Enabled" } else { "Disabled" };
        let color = if agent.is_enabled() { rag_status.green() } else { rag_status.red() };
        println!("  Status: {}", color);
        
        println!();
        Ok(())
    }

    /// Search knowledge base
    pub async fn search_knowledge(agent: &RAGAgent, query: &str, category_str: &str) -> Result<()> {
        let category = match category_str.to_lowercase().as_str() {
            "episode" | "episodes" => KnowledgeCategory::Episode,
            "knowledge" => KnowledgeCategory::Knowledge,
            "code" => KnowledgeCategory::Code,
            "document" | "documents" => KnowledgeCategory::Document,
            _ => {
                println!("{}: Unknown category '{}'. Use: episode, knowledge, code, document", 
                    "Error".red().bold(), category_str);
                return Ok(());
            }
        };

        println!("\n{} {} in {}...", "Searching".cyan(), query.bold(), category_str.yellow());
        
        let results = agent.search_knowledge(query, category, 5).await?;
        
        if results.is_empty() {
            println!("{}", "No results found.".yellow());
        } else {
            println!("\n{} {}:", "Found".green().bold(), results.len().to_string().green());
            for (i, result) in results.iter().enumerate() {
                println!("\n{}. {} {:.2}", 
                    (i + 1).to_string().cyan(),
                    "Score:".bold(),
                    result.score.to_string().green()
                );
                println!("   {}", result.document);
            }
        }
        
        println!();
        Ok(())
    }

    /// Display help for memory commands
    pub fn memory_help() {
        println!("\n{}", "=== Memory Commands ===".bold().cyan());
        println!("\n{}:", "Available Commands".bold());
        println!("  {:<20} {}", "/memory status".green(), "Show memory system status");
        println!("  {:<20} {}", "/memory search".green(), "Search knowledge base");
        println!("  {:<20} {}", "/stats".green(), "Show detailed statistics");
        println!("  {:<20} {}", "/knowledge".green(), "List knowledge entries");
        
        println!("\n{}:", "Examples".bold());
        println!("  {} {}", ">".cyan(), "/memory status".yellow());
        println!("  {} {}", ">".cyan(), "/memory search episode \"create API\"".yellow());
        println!("  {} {}", ">".cyan(), "/memory search code \"authentication\"".yellow());
        println!();
    }

    /// Display detailed statistics
    pub async fn show_statistics(agent: &RAGAgent) -> Result<()> {
        println!("\n{}", "=== Detailed Statistics ===".bold().cyan());
        
        let cumulative = agent.cumulative_stats().await;
        
        // Session statistics
        println!("\n{}:", "Sessions".bold());
        println!("  Total sessions:        {}", cumulative.total_sessions.to_string().cyan());
        println!("  First session:         {}", 
            cumulative.first_session
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".to_string())
                .cyan()
        );
        println!("  Last session:          {}", 
            cumulative.last_session
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".to_string())
                .cyan()
        );
        
        // Task statistics
        println!("\n{}:", "Tasks".bold());
        println!("  Total tasks:           {}", cumulative.total_tasks.to_string().cyan());
        println!("  Successful:            {}", cumulative.successful_tasks.to_string().green());
        println!("  Failed:                {}", cumulative.failed_tasks.to_string().yellow());
        println!("  Success rate:          {:.1}%", (cumulative.success_rate * 100.0).to_string().green().bold());
        
        // Performance statistics
        println!("\n{}:", "Performance".bold());
        println!("  Avg tasks per session: {:.1}", cumulative.avg_tasks_per_session.to_string().cyan());
        println!("  Avg task duration:     {:.1}s", cumulative.avg_task_duration_secs.to_string().cyan());
        println!("  Total execution time:  {:.1}s", cumulative.total_execution_time_secs.to_string().cyan());
        
        println!();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_commands_exists() {
        // Just verify the struct exists
        let _ = KnowledgeCommands;
    }
}
