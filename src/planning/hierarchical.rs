//! Hierarchical task decomposition with goal tree generation
//!
//! Provides top-down decomposition of complex goals into executable sub-goals
//! using LLM-based reasoning for intelligent planning.

use crate::planning::types::{GoalTree, GoalNode, NodeType, NodeId};
use crate::planning::complexity::ComplexityEstimator;
use crate::streaming::OllamaClient;
use crate::errors::Result;
use futures_util::StreamExt;
use serde_json::json;

/// Hierarchical task planner with LLM-based reasoning
pub struct HierarchicalPlanner {
    /// Complexity estimator for goal analysis
    estimator: ComplexityEstimator,

    /// Maximum decomposition depth (5)
    max_depth: usize,

    /// Maximum children per node (7)
    max_fanout: usize,

    /// Complexity threshold for atomic goals (0.2)
    atomic_threshold: f64,

    /// Ollama client for LLM-based planning
    client: Option<OllamaClient>,
}

impl HierarchicalPlanner {
    /// Create new planner with default settings
    pub fn new() -> Self {
        Self {
            estimator: ComplexityEstimator::new(),
            max_depth: 5,
            max_fanout: 7,
            atomic_threshold: 0.2,
            client: None,
        }
    }

    /// Set the Ollama client for LLM-based planning
    pub fn set_client(&mut self, client: OllamaClient) {
        self.client = Some(client);
    }
    
    /// Decompose a goal into hierarchical sub-goals
    ///
    /// Algorithm: Top-down recursive decomposition using LLM-based reasoning
    /// Complexity: O(n log n) where n = number of sub-goals
    ///
    /// Returns: GoalTree with DAG structure
    pub async fn decompose(&self, goal: &str, context: &[String]) -> Result<GoalTree> {
        // Estimate overall complexity
        let complexity = self.estimator.estimate(goal, context);

        // Create root node
        let mut tree = GoalTree::new(goal.to_string(), complexity);

        // Always use LLM-based planning - let the LLM decide if task is atomic
        // The LLM will return empty array if task cannot be meaningfully decomposed
        let root_id = tree.root;
        self.decompose_recursive(&mut tree, root_id, 0, context).await?;

        Ok(tree)
    }

    /// Recursive decomposition implementation
    async fn decompose_recursive(
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

        // Generate sub-goals using LLM
        let sub_goals = self.generate_subgoals(&parent_goal, context).await?;

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
                Box::pin(self.decompose_recursive(tree, child_id, depth + 1, context)).await?;
            }
        }

        Ok(())
    }
    
    /// Generate sub-goals for a given goal using LLM-based reasoning
    ///
    /// This method uses the LLM to actually think about task decomposition,
    /// considering potential failure points, edge cases, and proper sequencing.
    async fn generate_subgoals(&self, goal: &str, context: &[String]) -> Result<Vec<String>> {
        // If no client available, return empty (will mark as atomic)
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        // Build deep planning prompt
        let context_str = if context.is_empty() {
            String::new()
        } else {
            format!("\n\nContext from previous steps:\n{}", context.join("\n"))
        };

        let planning_prompt = format!(r#"You are an intelligent task planning system. Break down the following task into concrete, executable steps.

TASK: {}{}

PLANNING GUIDELINES:
1. Think deeply about what steps are actually needed
2. Consider potential failure points (e.g., files not existing, permissions, syntax errors)
3. Plan for verification of each step
4. Consider edge cases like:
   - Quote escaping in code generation
   - Path handling (~/  vs absolute paths)
   - File/directory existence checks
5. Keep steps atomic and testable
6. List steps in proper execution order

OUTPUT FORMAT:
Return ONLY a JSON array of step descriptions. Each step should be a clear, actionable task.
Example: ["Step 1 description", "Step 2 description", "Step 3 description"]

If the task is already atomic (cannot be meaningfully broken down), return an empty array: []

STEPS:"#, goal, context_str);

        // Call LLM for planning
        let request = json!({
            "model": "qwen2.5:14b-instruct",
            "prompt": planning_prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 500
            }
        });

        let mut stream = client.generate_stream(serde_json::to_string(&request)
            .map_err(|e| crate::errors::AgentError::Generic(format!("JSON error: {}", e)))?
        ).await
            .map_err(|e| crate::errors::AgentError::Generic(format!("LLM error: {}", e)))?;

        // Collect full response
        let mut response_text = String::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result
                .map_err(|e| crate::errors::AgentError::Generic(format!("Stream error: {}", e)))?;

            if let Ok(ollama_response) = serde_json::from_slice::<serde_json::Value>(&chunk_bytes) {
                if let Some(token) = ollama_response.get("response").and_then(|r| r.as_str()) {
                    response_text.push_str(token);
                }
            }
        }

        // Parse JSON response
        let response_text = response_text.trim();

        // Try to extract JSON array from response
        let steps: Vec<String> = if let Some(start) = response_text.find('[') {
            if let Some(end) = response_text.rfind(']') {
                let json_str = &response_text[start..=end];
                serde_json::from_str(json_str).unwrap_or_else(|_| Vec::new())
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(steps)
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
