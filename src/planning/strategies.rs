//! Multi-strategy planning with confidence-based selection
//!
//! Generates multiple strategies (Direct, Exploratory, Systematic) and selects
//! the optimal one using utility function with confidence, cost, and applicability.

use crate::planning::types::{Strategy, StrategyType, PlanStep, GoalTree, GoalNode};
use crate::planning::complexity::{ComplexityEstimator, ComplexityLevel};
use crate::errors::Result;

/// Multi-strategy generator
pub struct StrategyGenerator {
    /// Complexity estimator for strategy selection
    estimator: ComplexityEstimator,
    
    /// Weight for confidence in utility function (0.50)
    weight_confidence: f64,
    
    /// Weight for cost in utility function (0.30)
    weight_cost: f64,
    
    /// Weight for applicability in utility function (0.20)
    weight_applicability: f64,
}

impl StrategyGenerator {
    /// Create new strategy generator
    pub fn new() -> Self {
        Self {
            estimator: ComplexityEstimator::new(),
            weight_confidence: 0.50,
            weight_cost: 0.30,
            weight_applicability: 0.20,
        }
    }
    
    /// Generate all applicable strategies for a goal
    pub fn generate_strategies(&self, goal_tree: &GoalTree, context: &[String]) -> Result<Vec<Strategy>> {
        let root = goal_tree.nodes.get(&goal_tree.root)
            .ok_or_else(|| crate::errors::AgentError::Generic("No root node".to_string()))?;
        
        let complexity = root.complexity;
        let complexity_level = self.estimator.classify(complexity);
        
        let mut strategies = Vec::new();
        
        // Generate Direct strategy
        strategies.push(self.generate_direct_strategy(root, &complexity_level, context)?);
        
        // Generate Exploratory strategy
        strategies.push(self.generate_exploratory_strategy(root, &complexity_level, context)?);
        
        // Generate Systematic strategy
        strategies.push(self.generate_systematic_strategy(root, goal_tree, &complexity_level)?);
        
        Ok(strategies)
    }
    
    /// Select best strategy using utility function
    ///
    /// Utility = 0.50 × confidence + 0.30 × (1 - cost) + 0.20 × applicability
    pub fn select_strategy(&self, strategies: &[Strategy]) -> Option<Strategy> {
        if strategies.is_empty() {
            return None;
        }
        
        let mut best_strategy = strategies[0].clone();
        let mut best_utility = self.calculate_utility(&best_strategy);
        
        for strategy in strategies.iter().skip(1) {
            let utility = self.calculate_utility(strategy);
            if utility > best_utility {
                best_utility = utility;
                best_strategy = strategy.clone();
            }
        }
        
        Some(best_strategy)
    }
    
    /// Calculate utility score for a strategy
    fn calculate_utility(&self, strategy: &Strategy) -> f64 {
        self.weight_confidence * strategy.confidence +
        self.weight_cost * (1.0 - strategy.cost) +
        self.weight_applicability * strategy.applicability
    }
    
    /// Generate Direct strategy (for simple, clear goals)
    fn generate_direct_strategy(
        &self,
        goal: &GoalNode,
        level: &ComplexityLevel,
        _context: &[String],
    ) -> Result<Strategy> {
        let goal_lower = goal.description.to_lowercase();
        
        // Confidence: high if goal is unambiguous and simple
        let confidence = match level {
            ComplexityLevel::Simple => 0.9,
            ComplexityLevel::Medium => 0.6,
            ComplexityLevel::Complex => 0.3,
        };
        
        // Adjust confidence based on clarity
        let mut final_confidence: f64 = confidence;
        if goal_lower.contains("exactly") || goal_lower.contains("specifically") {
            final_confidence = (final_confidence + 0.1).min(1.0);
        }
        if goal_lower.contains("maybe") || goal_lower.contains("somehow") {
            final_confidence = (final_confidence - 0.2).max(0.0);
        }
        
        // Cost: low for simple goals
        let cost = match level {
            ComplexityLevel::Simple => 0.2,
            ComplexityLevel::Medium => 0.4,
            ComplexityLevel::Complex => 0.7,
        };
        
        // Applicability: high if goal is clear and specific
        let applicability = if goal_lower.contains("/") || goal_lower.contains(".") {
            0.8 // Has specific paths/files
        } else if goal_lower.contains("read") || goal_lower.contains("write") 
            || goal_lower.contains("list") {
            0.7 // Clear action verbs
        } else {
            0.5 // Generic
        };
        
        // Generate minimal plan steps
        let steps = self.generate_direct_steps(goal);
        
        Ok(Strategy {
            name: "Direct".to_string(),
            strategy_type: StrategyType::Direct,
            confidence: final_confidence,
            cost,
            applicability,
            steps,
        })
    }
    
    /// Generate Exploratory strategy (for ambiguous goals)
    fn generate_exploratory_strategy(
        &self,
        goal: &GoalNode,
        level: &ComplexityLevel,
        _context: &[String],
    ) -> Result<Strategy> {
        let goal_lower = goal.description.to_lowercase();
        
        // Confidence: medium, increases with exploration
        let confidence = match level {
            ComplexityLevel::Simple => 0.6,
            ComplexityLevel::Medium => 0.7,
            ComplexityLevel::Complex => 0.8,
        };
        
        // Cost: medium (exploration overhead)
        let cost = match level {
            ComplexityLevel::Simple => 0.4,
            ComplexityLevel::Medium => 0.5,
            ComplexityLevel::Complex => 0.6,
        };
        
        // Applicability: high if goal is ambiguous
        let applicability = if goal_lower.contains("what") || goal_lower.contains("how") 
            || goal_lower.contains("which") {
            0.9 // Question words
        } else if goal_lower.contains("find") || goal_lower.contains("search") {
            0.8 // Search operations
        } else {
            0.6 // Default
        };
        
        // Generate exploration steps
        let steps = self.generate_exploratory_steps(goal);
        
        Ok(Strategy {
            name: "Exploratory".to_string(),
            strategy_type: StrategyType::Exploratory,
            confidence,
            cost,
            applicability,
            steps,
        })
    }
    
    /// Generate Systematic strategy (for complex, structured goals)
    fn generate_systematic_strategy(
        &self,
        goal: &GoalNode,
        tree: &GoalTree,
        level: &ComplexityLevel,
    ) -> Result<Strategy> {
        // Confidence: high for well-structured complex tasks
        let confidence = match level {
            ComplexityLevel::Simple => 0.5,
            ComplexityLevel::Medium => 0.8,
            ComplexityLevel::Complex => 0.9,
        };
        
        // Cost: high (thorough but slow)
        let cost = match level {
            ComplexityLevel::Simple => 0.6,
            ComplexityLevel::Medium => 0.7,
            ComplexityLevel::Complex => 0.8,
        };
        
        // Applicability: high if goal has sub-tasks
        let has_children = tree.edges.get(&tree.root).map(|c| !c.is_empty()).unwrap_or(false);
        let applicability = if has_children {
            0.9
        } else if level == &ComplexityLevel::Complex {
            0.8
        } else {
            0.5
        };
        
        // Generate systematic steps from goal tree
        let steps = self.generate_systematic_steps(goal, tree);
        
        Ok(Strategy {
            name: "Systematic".to_string(),
            strategy_type: StrategyType::Systematic,
            confidence,
            cost,
            applicability,
            steps,
        })
    }
    
    /// Generate minimal steps for direct approach
    fn generate_direct_steps(&self, goal: &GoalNode) -> Vec<PlanStep> {
        vec![
            PlanStep {
                description: format!("Execute: {}", goal.description),
                expected_tool: self.infer_tool(&goal.description),
                completed: false,
            },
        ]
    }
    
    /// Generate exploration steps
    fn generate_exploratory_steps(&self, goal: &GoalNode) -> Vec<PlanStep> {
        vec![
            PlanStep {
                description: "Gather information about the task".to_string(),
                expected_tool: Some("list_dir".to_string()),
                completed: false,
            },
            PlanStep {
                description: format!("Analyze findings: {}", goal.description),
                expected_tool: None,
                completed: false,
            },
            PlanStep {
                description: "Execute based on analysis".to_string(),
                expected_tool: self.infer_tool(&goal.description),
                completed: false,
            },
        ]
    }
    
    /// Generate systematic steps from goal tree
    fn generate_systematic_steps(&self, _goal: &GoalNode, tree: &GoalTree) -> Vec<PlanStep> {
        let mut steps = Vec::new();
        
        // Add decomposition step
        steps.push(PlanStep {
            description: "Break down task into sub-goals".to_string(),
            expected_tool: None,
            completed: false,
        });
        
        // Add steps for each child (up to 5)
        if let Some(children) = tree.edges.get(&tree.root) {
            for child_id in children.iter().take(5) {
                if let Some(child) = tree.nodes.get(child_id) {
                    steps.push(PlanStep {
                        description: format!("Complete: {}", child.description),
                        expected_tool: self.infer_tool(&child.description),
                        completed: false,
                    });
                }
            }
        }
        
        // Add verification step
        steps.push(PlanStep {
            description: "Verify all sub-goals completed".to_string(),
            expected_tool: None,
            completed: false,
        });
        
        steps
    }
    
    /// Infer likely tool from goal description
    fn infer_tool(&self, goal: &str) -> Option<String> {
        let goal_lower = goal.to_lowercase();
        
        if goal_lower.contains("read") || goal_lower.contains("view") 
            || goal_lower.contains("show") {
            Some("read_file".to_string())
        } else if goal_lower.contains("write") || goal_lower.contains("create") 
            || goal_lower.contains("save") {
            Some("write_file".to_string())
        } else if goal_lower.contains("list") || goal_lower.contains("directory") {
            Some("list_dir".to_string())
        } else if goal_lower.contains("run") || goal_lower.contains("execute") 
            || goal_lower.contains("command") {
            Some("run_command".to_string())
        } else if goal_lower.contains("system") || goal_lower.contains("cpu") 
            || goal_lower.contains("memory") {
            Some("system_info".to_string())
        } else if goal_lower.contains("fetch") || goal_lower.contains("http") 
            || goal_lower.contains("url") {
            Some("web_fetch".to_string())
        } else {
            None
        }
    }
}

impl Default for StrategyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::types::GoalTree;
    
    #[test]
    fn test_generator_creation() {
        let generator = StrategyGenerator::new();
        assert!((generator.weight_confidence - 0.50).abs() < 0.001);
        assert!((generator.weight_cost - 0.30).abs() < 0.001);
        assert!((generator.weight_applicability - 0.20).abs() < 0.001);
    }
    
    #[test]
    fn test_generate_strategies() {
        let generator = StrategyGenerator::new();
        let tree = GoalTree::new("Read file.txt".to_string(), 0.3);
        let strategies = generator.generate_strategies(&tree, &[]).unwrap();
        
        assert_eq!(strategies.len(), 3);
        assert_eq!(strategies[0].strategy_type, StrategyType::Direct);
        assert_eq!(strategies[1].strategy_type, StrategyType::Exploratory);
        assert_eq!(strategies[2].strategy_type, StrategyType::Systematic);
    }
    
    #[test]
    fn test_select_best_strategy() {
        let generator = StrategyGenerator::new();
        let tree = GoalTree::new("Read file.txt".to_string(), 0.2);
        let strategies = generator.generate_strategies(&tree, &[]).unwrap();
        
        let best = generator.select_strategy(&strategies).unwrap();
        assert!(best.confidence > 0.0);
        assert!(best.cost >= 0.0 && best.cost <= 1.0);
        assert!(best.applicability >= 0.0 && best.applicability <= 1.0);
    }
    
    #[test]
    fn test_utility_calculation() {
        let generator = StrategyGenerator::new();
        let strategy = Strategy {
            name: "Test".to_string(),
            strategy_type: StrategyType::Direct,
            confidence: 0.8,
            cost: 0.3,
            applicability: 0.7,
            steps: vec![],
        };
        
        let utility = generator.calculate_utility(&strategy);
        let expected = 0.50 * 0.8 + 0.30 * (1.0 - 0.3) + 0.20 * 0.7;
        assert!((utility - expected).abs() < 0.001);
    }
    
    #[test]
    fn test_direct_strategy_simple_goal() {
        let generator = StrategyGenerator::new();
        let tree = GoalTree::new("Read /home/user/file.txt".to_string(), 0.15);
        let strategies = generator.generate_strategies(&tree, &[]).unwrap();
        
        let direct = &strategies[0];
        assert!(direct.confidence > 0.8); // High confidence for simple clear goal
        assert!(direct.cost < 0.3); // Low cost
    }
    
    #[test]
    fn test_exploratory_strategy_ambiguous_goal() {
        let generator = StrategyGenerator::new();
        let tree = GoalTree::new("Find what files are related to authentication".to_string(), 0.5);
        let strategies = generator.generate_strategies(&tree, &[]).unwrap();
        
        let exploratory = &strategies[1];
        assert!(exploratory.applicability > 0.7); // High applicability for search
        assert!(exploratory.steps.len() >= 3); // Exploration requires multiple steps
    }
    
    #[test]
    fn test_systematic_strategy_complex_goal() {
        let generator = StrategyGenerator::new();
        let tree = GoalTree::new("Analyze all Python files and generate report".to_string(), 0.8);
        let strategies = generator.generate_strategies(&tree, &[]).unwrap();
        
        let systematic = &strategies[2];
        assert!(systematic.confidence > 0.7); // High confidence for structured task
        assert!(systematic.steps.len() >= 2); // Multiple systematic steps
    }
    
    #[test]
    fn test_tool_inference() {
        let generator = StrategyGenerator::new();
        
        assert_eq!(generator.infer_tool("Read the config file"), Some("read_file".to_string()));
        assert_eq!(generator.infer_tool("Write output to log"), Some("write_file".to_string()));
        assert_eq!(generator.infer_tool("List directory contents"), Some("list_dir".to_string()));
        assert_eq!(generator.infer_tool("Run command ls -la"), Some("run_command".to_string()));
        assert_eq!(generator.infer_tool("Check system memory"), Some("system_info".to_string()));
        assert_eq!(generator.infer_tool("Fetch URL content"), Some("web_fetch".to_string()));
    }
}
