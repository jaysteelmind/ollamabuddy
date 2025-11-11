//! Hierarchical task decomposition with goal tree generation
//!
//! Provides top-down decomposition of complex goals into executable sub-goals
//! with formal guarantees: DAG structure, bounded depth, bounded fanout.

use crate::planning::types::{GoalTree, GoalNode, NodeType, NodeId};
use crate::planning::complexity::ComplexityEstimator;
use crate::errors::Result;

/// Hierarchical task planner
pub struct HierarchicalPlanner {
    /// Complexity estimator for goal analysis
    estimator: ComplexityEstimator,
    
    /// Maximum decomposition depth (5)
    max_depth: usize,
    
    /// Maximum children per node (7)
    max_fanout: usize,
    
    /// Complexity threshold for atomic goals (0.2)
    atomic_threshold: f64,
}

impl HierarchicalPlanner {
    /// Create new planner with default settings
    pub fn new() -> Self {
        Self {
            estimator: ComplexityEstimator::new(),
            max_depth: 5,
            max_fanout: 7,
            atomic_threshold: 0.2,
        }
    }
    
    /// Decompose a goal into hierarchical sub-goals
    ///
    /// Algorithm: Top-down recursive decomposition
    /// Complexity: O(n log n) where n = number of sub-goals
    ///
    /// Returns: GoalTree with DAG structure
    pub fn decompose(&self, goal: &str, context: &[String]) -> Result<GoalTree> {
        // Estimate overall complexity
        let complexity = self.estimator.estimate(goal, context);
        
        // Create root node
        let mut tree = GoalTree::new(goal.to_string(), complexity);
        
        // Recursively decompose if needed
        if complexity >= self.atomic_threshold {
            let root_id = tree.root;
            self.decompose_recursive(&mut tree, root_id, 0, context)?;
        } else {
            // Already atomic, update node type
            let root_id = tree.root;
            if let Some(root) = tree.nodes.get_mut(&root_id) {
                root.node_type = NodeType::Atomic;
            }
        }
        
        Ok(tree)
    }
    
    /// Recursive decomposition implementation
    fn decompose_recursive(
        &self,
        tree: &mut GoalTree,
        parent_id: NodeId,
        depth: usize,
        context: &[String],
    ) -> Result<()> {
        // Check depth constraint
        if depth >= self.max_depth {
            return Ok(());
        }
        
        // Get parent goal (clone to avoid borrow issues)
        let parent_goal = {
            let node = tree.nodes.get(&parent_id)
                .ok_or_else(|| crate::errors::AgentError::Generic(
                    format!("Parent node {} not found", parent_id)
                ))?;
            node.description.clone()
        };
        
        // Generate sub-goals
        let sub_goals = self.generate_subgoals(&parent_goal, context)?;
        
        // If no sub-goals or goal is already simple, mark as atomic
        if sub_goals.is_empty() {
            if let Some(node) = tree.nodes.get_mut(&parent_id) {
                node.node_type = NodeType::Atomic;
            }
            return Ok(());
        }
        
        // Add sub-goals as children
        for sub_goal in sub_goals.iter().take(self.max_fanout) {
            let complexity = self.estimator.estimate(sub_goal, context);
            
            let node_type = if complexity < self.atomic_threshold {
                NodeType::Atomic
            } else {
                NodeType::Composite
            };
            
            let child_id = tree.add_child(
                parent_id,
                sub_goal.clone(),
                node_type,
                complexity,
            ).map_err(|e| crate::errors::AgentError::Generic(e))?;
            
            // Recursively decompose composite children
            if node_type == NodeType::Composite {
                self.decompose_recursive(tree, child_id, depth + 1, context)?;
            }
        }
        
        Ok(())
    }
    
    /// Generate sub-goals for a given goal
    ///
    /// Uses heuristic-based decomposition strategies
    fn generate_subgoals(&self, goal: &str, _context: &[String]) -> Result<Vec<String>> {
        let goal_lower = goal.to_lowercase();
        let mut sub_goals = Vec::new();
        
        // Strategy 1: Sequential operations (indicated by "and", "then")
        if goal_lower.contains(" and ") || goal_lower.contains(" then ") {
            sub_goals.extend(self.decompose_sequential(goal));
        }
        
        // Strategy 2: File operations on multiple targets
        else if (goal_lower.contains("all") || goal_lower.contains("each")) 
            && (goal_lower.contains("file") || goal_lower.contains("directory")) {
            sub_goals.extend(self.decompose_batch_operation(goal));
        }
        
        // Strategy 3: Analysis and reporting pattern
        else if goal_lower.contains("analyze") && goal_lower.contains("report") {
            sub_goals.extend(self.decompose_analysis_report(goal));
        }
        
        // Strategy 4: Search and process pattern
        else if goal_lower.contains("find") && (goal_lower.contains("count") 
            || goal_lower.contains("list") || goal_lower.contains("show")) {
            sub_goals.extend(self.decompose_search_process(goal));
        }
        
        // Strategy 5: Complex command patterns
        else if goal_lower.contains("pipe") || goal_lower.contains("|") 
            || goal_lower.contains("grep") || goal_lower.contains("sed") {
            sub_goals.extend(self.decompose_pipeline(goal));
        }
        
        // Strategy 6: Generic decomposition for unmatched patterns
        else if self.estimator.estimate(goal, &[]) >= 0.3 {
            sub_goals.extend(self.decompose_generic(goal));
        }
        
        Ok(sub_goals)
    }
    
    /// Decompose sequential operations (A and B and C)
    fn decompose_sequential(&self, goal: &str) -> Vec<String> {
        let mut sub_goals = Vec::new();
        
        // Split on "and", "then", ","
        let separators = [" and ", " then ", ", and ", ", then "];
        let mut parts = vec![goal.to_string()];
        
        for sep in &separators {
            let mut new_parts = Vec::new();
            for part in parts {
                new_parts.extend(
                    part.split(sep).map(|s| s.trim().to_string())
                );
            }
            parts = new_parts;
        }
        
        // Clean up and filter
        for part in parts {
            let cleaned = part.trim().to_string();
            if !cleaned.is_empty() && cleaned.len() > 3 {
                sub_goals.push(cleaned);
            }
        }
        
        sub_goals
    }
    
    /// Decompose batch operations (process all files)
    fn decompose_batch_operation(&self, goal: &str) -> Vec<String> {
        vec![
            "List all target items".to_string(),
            format!("Process each item from: {}", goal),
            "Aggregate results".to_string(),
        ]
    }
    
    /// Decompose analysis and report pattern
    fn decompose_analysis_report(&self, goal: &str) -> Vec<String> {
        vec![
            "Gather data for analysis".to_string(),
            format!("Analyze: {}", goal),
            "Generate report with results".to_string(),
        ]
    }
    
    /// Decompose search and process pattern
    fn decompose_search_process(&self, goal: &str) -> Vec<String> {
        vec![
            format!("Search for items matching: {}", goal),
            "Process found items".to_string(),
            "Display or save results".to_string(),
        ]
    }
    
    /// Decompose pipeline operations
    fn decompose_pipeline(&self, _goal: &str) -> Vec<String> {
        vec![
            "Prepare input data".to_string(),
            "Execute pipeline stages".to_string(),
            "Collect final output".to_string(),
        ]
    }
    
    /// Generic decomposition for complex goals
    fn decompose_generic(&self, goal: &str) -> Vec<String> {
        vec![
            "Understand the requirement".to_string(),
            format!("Execute: {}", goal),
            "Verify and report results".to_string(),
        ]
    }
}

impl Default for HierarchicalPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_planner_creation() {
        let planner = HierarchicalPlanner::new();
        assert_eq!(planner.max_depth, 5);
        assert_eq!(planner.max_fanout, 7);
        assert!((planner.atomic_threshold - 0.2).abs() < 0.001);
    }
    
    #[test]
    fn test_simple_goal_no_decomposition() {
        let planner = HierarchicalPlanner::new();
        let goal = "Read file.txt";
        let tree = planner.decompose(goal, &[]).unwrap();
        
        // Should have only root node
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[&tree.root].node_type, NodeType::Atomic);
    }
    
    #[test]
    fn test_complex_goal_decomposition() {
        let planner = HierarchicalPlanner::new();
        let goal = "Find all Python files and count lines of code";
        let tree = planner.decompose(goal, &[]).unwrap();
        
        // Should have root + children
        assert!(tree.nodes.len() > 1, "Expected decomposition to create children");
        
        // Root should be composite
        assert_eq!(tree.nodes[&tree.root].node_type, NodeType::Composite);
    }
    
    #[test]
    fn test_sequential_decomposition() {
        let planner = HierarchicalPlanner::new();
        let sub_goals = planner.decompose_sequential("Read file and process data and save results");
        
        assert_eq!(sub_goals.len(), 3);
        assert!(sub_goals[0].contains("Read file"));
        assert!(sub_goals[1].contains("process data"));
        assert!(sub_goals[2].contains("save results"));
    }
    
    #[test]
    fn test_depth_limit_respected() {
        let planner = HierarchicalPlanner::new();
        let goal = "Very complex task that should decompose deeply";
        let tree = planner.decompose(goal, &[]).unwrap();
        
        // Check no node exceeds max depth
        for node in tree.nodes.values() {
            assert!(node.depth <= planner.max_depth);
        }
    }
    
    #[test]
    fn test_fanout_limit_respected() {
        let planner = HierarchicalPlanner::new();
        let goal = "Do A and B and C and D and E and F and G and H and I";
        let tree = planner.decompose(goal, &[]).unwrap();
        
        // Check no node exceeds max fanout
        for (node_id, _) in &tree.nodes {
            if let Some(children) = tree.edges.get(node_id) {
                assert!(children.len() <= planner.max_fanout);
            }
        }
    }
    
    #[test]
    fn test_leaf_nodes_are_atomic() {
        let planner = HierarchicalPlanner::new();
        let goal = "Find files and count lines";
        let tree = planner.decompose(goal, &[]).unwrap();
        
        // All leaf nodes should be atomic
        let leaves = tree.get_leaf_nodes();
        for leaf in leaves {
            assert_eq!(leaf.node_type, NodeType::Atomic);
        }
    }
}
