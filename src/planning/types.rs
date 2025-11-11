//! Core data structures for the planning system
//!
//! Defines goal trees, nodes, strategies, and failure patterns
//! with mathematical guarantees for bounded complexity.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for goal nodes
pub type NodeId = usize;

/// Goal tree structure representing hierarchical task decomposition
///
/// Mathematical properties:
/// - Acyclic (DAG)
/// - Bounded depth: max 5 levels
/// - Bounded fanout: max 7 children per node
/// - Single root node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTree {
    /// Root node identifier
    pub root: NodeId,
    
    /// All nodes in the tree
    pub nodes: HashMap<NodeId, GoalNode>,
    
    /// Adjacency list: node_id -> children
    pub edges: HashMap<NodeId, Vec<NodeId>>,
    
    /// Maximum depth allowed (5)
    pub max_depth: usize,
    
    /// Maximum children per node (7)
    pub max_fanout: usize,
    
    /// Next available node ID
    next_id: NodeId,
}

impl GoalTree {
    /// Create a new goal tree with a root node
    pub fn new(root_description: String, root_complexity: f64) -> Self {
        let root_node = GoalNode {
            id: 0,
            description: root_description,
            node_type: NodeType::Composite,
            status: GoalStatus::Pending,
            confidence: 1.0,
            dependencies: Vec::new(),
            complexity: root_complexity,
            depth: 0,
        };
        
        let mut nodes = HashMap::new();
        nodes.insert(0, root_node);
        
        Self {
            root: 0,
            nodes,
            edges: HashMap::new(),
            max_depth: 5,
            max_fanout: 7,
            next_id: 1,
        }
    }
    
    /// Add a child node to a parent
    ///
    /// Returns: child node ID or error if constraints violated
    pub fn add_child(
        &mut self,
        parent_id: NodeId,
        description: String,
        node_type: NodeType,
        complexity: f64,
    ) -> Result<NodeId, String> {
        // Verify parent exists
        let parent = self.nodes.get(&parent_id)
            .ok_or_else(|| format!("Parent node {} not found", parent_id))?;
        
        // Check depth constraint
        let child_depth = parent.depth + 1;
        if child_depth > self.max_depth {
            return Err(format!("Max depth {} exceeded", self.max_depth));
        }
        
        // Check fanout constraint
        let current_children = self.edges.get(&parent_id).map(|v| v.len()).unwrap_or(0);
        if current_children >= self.max_fanout {
            return Err(format!("Max fanout {} exceeded for node {}", self.max_fanout, parent_id));
        }
        
        // Create child node
        let child_id = self.next_id;
        self.next_id += 1;
        
        let child_node = GoalNode {
            id: child_id,
            description,
            node_type,
            status: GoalStatus::Pending,
            confidence: 0.8, // Default confidence
            dependencies: vec![parent_id],
            complexity,
            depth: child_depth,
        };
        
        self.nodes.insert(child_id, child_node);
        self.edges.entry(parent_id).or_insert_with(Vec::new).push(child_id);
        
        Ok(child_id)
    }
    
    /// Get all leaf nodes (atomic goals)
    pub fn get_leaf_nodes(&self) -> Vec<&GoalNode> {
        self.nodes.values()
            .filter(|node| !self.edges.contains_key(&node.id))
            .collect()
    }
    
    /// Update node status
    pub fn update_status(&mut self, node_id: NodeId, status: GoalStatus) -> Result<(), String> {
        self.nodes.get_mut(&node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?
            .status = status;
        Ok(())
    }
    
    /// Check if all children of a node are completed
    pub fn all_children_completed(&self, node_id: NodeId) -> bool {
        if let Some(children) = self.edges.get(&node_id) {
            children.iter().all(|child_id| {
                self.nodes.get(child_id)
                    .map(|node| node.status == GoalStatus::Completed)
                    .unwrap_or(false)
            })
        } else {
            true // No children means all completed
        }
    }
}

/// Individual goal node in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalNode {
    /// Unique identifier
    pub id: NodeId,
    
    /// Natural language description
    pub description: String,
    
    /// Node type (atomic or composite)
    pub node_type: NodeType,
    
    /// Current status
    pub status: GoalStatus,
    
    /// Confidence score [0.0, 1.0]
    pub confidence: f64,
    
    /// Parent node IDs (dependencies)
    pub dependencies: Vec<NodeId>,
    
    /// Complexity score [0.0, 1.0]
    pub complexity: f64,
    
    /// Depth in tree (0 = root)
    pub depth: usize,
}

/// Node type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// Leaf node, directly executable
    Atomic,
    
    /// Non-leaf node with sub-goals
    Composite,
}

/// Goal status tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    /// Not yet started
    Pending,
    
    /// Currently being worked on
    InProgress,
    
    /// Successfully completed
    Completed,
    
    /// Failed to complete
    Failed,
}

/// Strategy for achieving a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    /// Strategy name
    pub name: String,
    
    /// Strategy type
    pub strategy_type: StrategyType,
    
    /// Confidence score [0.0, 1.0]
    pub confidence: f64,
    
    /// Estimated cost (iterations) [0.0, 1.0]
    pub cost: f64,
    
    /// Applicability to current goal [0.0, 1.0]
    pub applicability: f64,
    
    /// Planned steps
    pub steps: Vec<PlanStep>,
}

/// Type of strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyType {
    /// Direct approach for simple goals
    Direct,
    
    /// Exploratory approach for ambiguous goals
    Exploratory,
    
    /// Systematic approach for complex goals
    Systematic,
}

/// Individual step in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step description
    pub description: String,
    
    /// Expected tool to use
    pub expected_tool: Option<String>,
    
    /// Step completed flag
    pub completed: bool,
}

/// Failure pattern detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailurePattern {
    /// Same tool called repeatedly with identical arguments
    RepeatedCall {
        tool: String,
        count: usize,
    },
    
    /// Tool returns empty/null results repeatedly
    EmptyResults {
        tool: String,
        count: usize,
    },
    
    /// Same error occurs multiple times
    ErrorStreak {
        error_type: String,
        count: usize,
    },
    
    /// No progress for N iterations
    StuckProgress {
        iterations: usize,
    },
}

/// Re-planning action to take
#[derive(Debug, Clone)]
pub enum ReplanningAction {
    /// Switch to a different strategy
    SwitchStrategy {
        reason: String,
        new_strategy: Option<Strategy>,
    },
    
    /// Modify current approach
    ModifyApproach {
        suggestion: String,
        modified_plan: Vec<String>,
    },
    
    /// Terminate (task impossible)
    Terminate {
        reason: String,
    },
}

/// Progress tracking metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMetrics {
    /// Completed goals / total goals [0.0, 1.0]
    pub goal_completion: f64,
    
    /// Successful tools / expected tools [0.0, 1.0]
    pub tool_success_rate: f64,
    
    /// Milestones reached / total milestones [0.0, 1.0]
    pub milestone_progress: f64,
    
    /// Overall progress [0.0, 1.0]
    pub overall_progress: f64,
    
    /// Iterations with no progress
    pub stagnant_iterations: usize,
}

impl ProgressMetrics {
    /// Create new metrics with all zeros
    pub fn new() -> Self {
        Self {
            goal_completion: 0.0,
            tool_success_rate: 0.0,
            milestone_progress: 0.0,
            overall_progress: 0.0,
            stagnant_iterations: 0,
        }
    }
    
    /// Calculate overall progress from components
    pub fn calculate_overall(&mut self) {
        self.overall_progress = 
            0.40 * self.goal_completion +
            0.30 * self.tool_success_rate +
            0.30 * self.milestone_progress;
    }
}

impl Default for ProgressMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_goal_tree_creation() {
        let tree = GoalTree::new("Root goal".to_string(), 0.5);
        assert_eq!(tree.root, 0);
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.max_depth, 5);
        assert_eq!(tree.max_fanout, 7);
    }
    
    #[test]
    fn test_add_child_node() {
        let mut tree = GoalTree::new("Root".to_string(), 0.5);
        let child_id = tree.add_child(0, "Child 1".to_string(), NodeType::Atomic, 0.3).unwrap();
        
        assert_eq!(child_id, 1);
        assert_eq!(tree.nodes.len(), 2);
        assert!(tree.edges.get(&0).unwrap().contains(&1));
    }
    
    #[test]
    fn test_depth_constraint() {
        let mut tree = GoalTree::new("Root".to_string(), 0.5);
        let mut current_id = 0;
        
        // Add nodes up to max depth
        for depth in 0..5 {
            let child_id = tree.add_child(
                current_id,
                format!("Node at depth {}", depth + 1),
                NodeType::Composite,
                0.3
            ).unwrap();
            current_id = child_id;
        }
        
        // Try to exceed max depth
        let result = tree.add_child(current_id, "Too deep".to_string(), NodeType::Atomic, 0.3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Max depth"));
    }
    
    #[test]
    fn test_fanout_constraint() {
        let mut tree = GoalTree::new("Root".to_string(), 0.5);
        
        // Add max_fanout children
        for i in 0..7 {
            tree.add_child(0, format!("Child {}", i), NodeType::Atomic, 0.2).unwrap();
        }
        
        // Try to exceed fanout
        let result = tree.add_child(0, "Too many".to_string(), NodeType::Atomic, 0.2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Max fanout"));
    }
    
    #[test]
    fn test_progress_metrics_calculation() {
        let mut metrics = ProgressMetrics::new();
        metrics.goal_completion = 0.5;
        metrics.tool_success_rate = 0.8;
        metrics.milestone_progress = 0.6;
        metrics.calculate_overall();
        
        let expected = 0.40 * 0.5 + 0.30 * 0.8 + 0.30 * 0.6;
        assert!((metrics.overall_progress - expected).abs() < 0.001);
    }
}
