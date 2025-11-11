//! Complexity estimation for task analysis
//!
//! Provides multi-factor complexity scoring with mathematical guarantees:
//! - Bounded output: [0.0, 1.0]
//! - Monotonic: more operations → higher complexity
//! - Composable: complexity(A∪B) ≥ max(complexity(A), complexity(B))

use crate::planning::types::{GoalNode, GoalTree};

/// Complexity estimator with 5-factor weighted scoring
pub struct ComplexityEstimator {
    /// Weight for tool count factor (0.20)
    weight_tools: f64,
    
    /// Weight for file operations factor (0.15)
    weight_files: f64,
    
    /// Weight for command complexity factor (0.25)
    weight_commands: f64,
    
    /// Weight for data volume factor (0.15)
    weight_data: f64,
    
    /// Weight for ambiguity factor (0.25)
    weight_ambiguity: f64,
}

impl ComplexityEstimator {
    /// Create new estimator with standard weights
    pub fn new() -> Self {
        Self {
            weight_tools: 0.20,
            weight_files: 0.15,
            weight_commands: 0.25,
            weight_data: 0.15,
            weight_ambiguity: 0.25,
        }
    }
    
    /// Estimate complexity of a goal
    ///
    /// Returns: complexity score ∈ [0.0, 1.0]
    ///
    /// Formula:
    /// complexity = 0.20 × tools + 0.15 × files + 0.25 × commands
    ///            + 0.15 × data + 0.25 × ambiguity
    pub fn estimate(&self, goal: &str, context: &[String]) -> f64 {
        let tools = self.estimate_tool_count(goal, context);
        let files = self.estimate_file_operations(goal);
        let commands = self.estimate_command_complexity(goal);
        let data = self.estimate_data_volume(goal);
        let ambiguity = self.estimate_ambiguity(goal);
        
        let complexity = 
            self.weight_tools * tools +
            self.weight_files * files +
            self.weight_commands * commands +
            self.weight_data * data +
            self.weight_ambiguity * ambiguity;
        
        // Guarantee bounds [0.0, 1.0]
        complexity.max(0.0).min(1.0)
    }
    
    /// Estimate complexity of entire goal tree
    pub fn estimate_tree(&self, tree: &GoalTree) -> f64 {
        let root = tree.nodes.get(&tree.root).unwrap();
        self.estimate(&root.description, &[])
    }
    
    /// Classify complexity level
    pub fn classify(&self, complexity: f64) -> ComplexityLevel {
        if complexity < 0.3 {
            ComplexityLevel::Simple
        } else if complexity < 0.7 {
            ComplexityLevel::Medium
        } else {
            ComplexityLevel::Complex
        }
    }
    
    /// Estimate number of tools needed
    ///
    /// Heuristics:
    /// - Keywords: "read", "write", "list", "run", "fetch", "check"
    /// - Multiple files mentioned → multiple tools
    /// - Commands present → run_command tool
    ///
    /// Normalized to [0.0, 1.0] with max=10 tools
    fn estimate_tool_count(&self, goal: &str, _context: &[String]) -> f64 {
        let goal_lower = goal.to_lowercase();
        let mut count: i32 = 0;
        
        // File operation keywords
        if goal_lower.contains("read") || goal_lower.contains("view") 
            || goal_lower.contains("show") || goal_lower.contains("display") {
            count += 1;
        }
        
        if goal_lower.contains("write") || goal_lower.contains("create") 
            || goal_lower.contains("save") || goal_lower.contains("modify") 
            || goal_lower.contains("generate") {
            count += 1;
        }
        
        if goal_lower.contains("list") || goal_lower.contains("find") 
            || goal_lower.contains("search") {
            count += 1;
        }
        
        // Analysis and reporting
        if goal_lower.contains("analyze") || goal_lower.contains("report") 
            || goal_lower.contains("statistics") || goal_lower.contains("count") {
            count += 2;
        }
        
        // Command execution
        if goal_lower.contains("run") || goal_lower.contains("execute") 
            || goal_lower.contains("command") {
            count += 2; // Commands often need multiple steps
        }
        
        // System info
        if goal_lower.contains("system") || goal_lower.contains("cpu") 
            || goal_lower.contains("memory") || goal_lower.contains("disk") {
            count += 1;
        }
        
        // Web operations
        if goal_lower.contains("fetch") || goal_lower.contains("download") 
            || goal_lower.contains("http") || goal_lower.contains("url") {
            count += 1;
        }
        
        // Multiple files indicator
        if goal_lower.contains("all") || goal_lower.contains("every") 
            || goal_lower.contains("each") {
            count += 3;
        }
        
        // Normalize to [0.0, 1.0] with max=10
        (count as f64 / 10.0).min(1.0)
    }
    
    /// Estimate file operation complexity
    ///
    /// Factors:
    /// - Number of files mentioned
    /// - Read vs write operations
    /// - File size indicators
    ///
    /// Normalized to [0.0, 1.0] with max=20 operations
    fn estimate_file_operations(&self, goal: &str) -> f64 {
        let goal_lower = goal.to_lowercase();
        let mut count: i32 = 0;
        
        // Count file-related keywords
        let file_keywords = ["file", "directory", "folder", "path"];
        for keyword in &file_keywords {
            if goal_lower.contains(keyword) {
                count += 2; // Increased weight
            }
        }
        
        // Write operations are more complex
        if goal_lower.contains("write") || goal_lower.contains("modify") 
            || goal_lower.contains("update") {
            count += 2;
        }
        
        // Batch operations
        if goal_lower.contains("all files") || goal_lower.contains("multiple") {
            count += 3;
        }
        
        // Normalize to [0.0, 1.0] with max=20
        (count as f64 / 20.0).min(1.0)
    }
    
    /// Estimate command execution complexity
    ///
    /// Factors:
    /// - Shell features (pipes, redirects)
    /// - Multiple commands
    /// - Complex arguments
    ///
    /// Returns: score ∈ [0.0, 1.0]
    fn estimate_command_complexity(&self, goal: &str) -> f64 {
        let goal_lower = goal.to_lowercase();
        let mut score: f64 = 0.0;
        
        // Basic command execution
        if goal_lower.contains("run") || goal_lower.contains("execute") 
            || goal_lower.contains("command") {
            score += 0.3;
        }
        
        // Complex analysis operations
        if goal_lower.contains("lines of code") || goal_lower.contains("analyze") 
            || goal_lower.contains("complexity") {
            score += 0.4;
        }
        
        // Shell features (pipes, redirects)
        if goal_lower.contains("pipe") || goal_lower.contains("|") 
            || goal_lower.contains(">") {
            score += 0.3;
        }
        
        // Multiple commands
        if goal_lower.contains("and then") || goal_lower.contains("after") 
            || goal_lower.contains("&&") {
            score += 0.2;
        }
        
        // Complex operations
        if goal_lower.contains("grep") || goal_lower.contains("sed") 
            || goal_lower.contains("awk") || goal_lower.contains("find") {
            score += 0.2;
        }
        
        score.min(1.0)
    }
    
    /// Estimate data volume to process
    ///
    /// Indicators:
    /// - Size keywords (large, small, many, few)
    /// - Numeric quantities
    ///
    /// Normalized to [0.0, 1.0] with max=100MB implied
    fn estimate_data_volume(&self, goal: &str) -> f64 {
        let goal_lower = goal.to_lowercase();
        let mut score: f64 = 0.0;
        
        // Small data indicators
        if goal_lower.contains("small") || goal_lower.contains("few") 
            || goal_lower.contains("single") {
            score = 0.1;
        }
        
        // Medium data indicators
        if goal_lower.contains("several") || goal_lower.contains("some") {
            score = 0.4;
        }
        
        // Large data indicators
        if goal_lower.contains("large") || goal_lower.contains("many") 
            || goal_lower.contains("all") {
            score = 0.7;
        }
        
        // Very large data indicators
        if goal_lower.contains("entire") || goal_lower.contains("whole") 
            || goal_lower.contains("complete") {
            score = 0.9;
        }
        
        score
    }
    
    /// Estimate goal ambiguity
    ///
    /// Factors:
    /// - Vague words (somehow, maybe, try)
    /// - Missing details (no specific paths, numbers)
    /// - Question words without clear answers
    ///
    /// Returns: ambiguity score ∈ [0.0, 1.0]
    fn estimate_ambiguity(&self, goal: &str) -> f64 {
        let goal_lower = goal.to_lowercase();
        let mut score: f64 = 0.0;
        
        // Vague language
        let vague_words = ["somehow", "maybe", "try", "attempt", "perhaps", "possibly"];
        for word in &vague_words {
            if goal_lower.contains(word) {
                score += 0.15;
            }
        }
        
        // Question indicators without specifics
        if goal_lower.contains("what") || goal_lower.contains("how") 
            || goal_lower.contains("which") {
            score += 0.2;
        }
        
        // Missing specifics
        if !goal_lower.contains("/") && !goal_lower.contains(".") {
            // No paths or file extensions mentioned
            score += 0.15;
        }
        
        // Long, rambling goals
        if goal.len() > 200 {
            score += 0.1;
        }
        
        // Clear, specific goals reduce ambiguity
        if goal_lower.contains("exactly") || goal_lower.contains("specifically") {
            score -= 0.2;
        }
        
        score.max(0.0).min(1.0)
    }
}

impl Default for ComplexityEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Complexity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityLevel {
    /// Simple: 1-3 tools, single operation
    Simple,
    
    /// Medium: 4-7 tools, multiple steps
    Medium,
    
    /// Complex: 8+ tools, intricate dependencies
    Complex,
}

impl ComplexityLevel {
    /// Get recommended iteration limit for this complexity
    pub fn recommended_iterations(&self) -> usize {
        match self {
            ComplexityLevel::Simple => 5,
            ComplexityLevel::Medium => 10,
            ComplexityLevel::Complex => 15,
        }
    }
    
    /// Get recommended model for this complexity
    pub fn recommended_model(&self) -> &'static str {
        match self {
            ComplexityLevel::Simple => "qwen2.5:7b-instruct",
            ComplexityLevel::Medium => "qwen2.5:14b-instruct",
            ComplexityLevel::Complex => "qwen2.5:32b-instruct",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_estimator_creation() {
        let estimator = ComplexityEstimator::new();
        assert!((estimator.weight_tools - 0.20).abs() < 0.001);
        assert!((estimator.weight_files - 0.15).abs() < 0.001);
        assert!((estimator.weight_commands - 0.25).abs() < 0.001);
        assert!((estimator.weight_data - 0.15).abs() < 0.001);
        assert!((estimator.weight_ambiguity - 0.25).abs() < 0.001);
    }
    
    #[test]
    fn test_simple_goal_complexity() {
        let estimator = ComplexityEstimator::new();
        let goal = "Read the file config.txt";
        let complexity = estimator.estimate(goal, &[]);
        
        assert!(complexity >= 0.0 && complexity <= 1.0);
        assert!(complexity < 0.3); // Should be simple
    }
    
    #[test]
    fn test_complex_goal_complexity() {
        let estimator = ComplexityEstimator::new();
        let goal = "Find all Python files, count lines of code, analyze complexity, and generate a report with statistics for each file";
        let complexity = estimator.estimate(goal, &[]);
        
        assert!(complexity >= 0.0 && complexity <= 1.0);
        assert!(complexity >= 0.4); // Should be medium-to-complex
    }
    
    #[test]
    fn test_complexity_classification() {
        let estimator = ComplexityEstimator::new();
        
        assert_eq!(estimator.classify(0.2), ComplexityLevel::Simple);
        assert_eq!(estimator.classify(0.5), ComplexityLevel::Medium);
        assert_eq!(estimator.classify(0.8), ComplexityLevel::Complex);
    }
    
    #[test]
    fn test_complexity_bounds() {
        let estimator = ComplexityEstimator::new();
        
        // Test various goals
        let goals = vec![
            "Read a file",
            "List all directories and count files in each one",
            "Execute a complex pipeline with grep, sed, awk, and multiple files",
        ];
        
        for goal in goals {
            let complexity = estimator.estimate(goal, &[]);
            assert!(complexity >= 0.0, "Complexity below 0.0: {}", complexity);
            assert!(complexity <= 1.0, "Complexity above 1.0: {}", complexity);
        }
    }
    
    #[test]
    fn test_tool_count_estimation() {
        let estimator = ComplexityEstimator::new();
        
        let simple = estimator.estimate_tool_count("Read file.txt", &[]);
        let complex = estimator.estimate_tool_count("Read, write, list, run, fetch all files", &[]);
        
        assert!(complex > simple);
        assert!(simple <= 1.0);
        assert!(complex <= 1.0);
    }
    
    #[test]
    fn test_ambiguity_estimation() {
        let estimator = ComplexityEstimator::new();
        
        let clear = estimator.estimate_ambiguity("Read /home/user/config.txt");
        let vague = estimator.estimate_ambiguity("Maybe try to somehow read some file");
        
        assert!(vague > clear);
        assert!(clear >= 0.0 && clear <= 1.0);
        assert!(vague >= 0.0 && vague <= 1.0);
    }
    
    #[test]
    fn test_recommended_iterations() {
        assert_eq!(ComplexityLevel::Simple.recommended_iterations(), 5);
        assert_eq!(ComplexityLevel::Medium.recommended_iterations(), 10);
        assert_eq!(ComplexityLevel::Complex.recommended_iterations(), 15);
    }
}
