//! Semantic Knowledge Graph: Entity and relationship extraction
//!
//! Extracts entities and relationships from tool results to build
//! a semantic understanding of the workspace and operations.

use crate::tools::types::ToolResult;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Node types in the knowledge graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeType {
    File { path: String, size: Option<usize> },
    Directory { path: String },
    Command { name: String, args: Vec<String> },
    Concept { name: String },
    Error { error_type: String, message: String },
}

impl NodeType {
    /// Get node identifier
    pub fn id(&self) -> String {
        match self {
            NodeType::File { path, .. } => format!("file:{}", path),
            NodeType::Directory { path } => format!("dir:{}", path),
            NodeType::Command { name, .. } => format!("cmd:{}", name),
            NodeType::Concept { name } => format!("concept:{}", name),
            NodeType::Error { error_type, .. } => format!("error:{}", error_type),
        }
    }

    /// Get human-readable label
    pub fn label(&self) -> String {
        match self {
            NodeType::File { path, .. } => path.clone(),
            NodeType::Directory { path } => path.clone(),
            NodeType::Command { name, .. } => name.clone(),
            NodeType::Concept { name } => name.clone(),
            NodeType::Error { error_type, .. } => error_type.clone(),
        }
    }
}

/// Edge types representing relationships
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    Contains,
    DependsOn,
    ProducedBy,
    CausedBy,
    SimilarTo(f64),
    MentionedIn,
}

/// Edge in the knowledge graph
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

/// Knowledge graph structure
pub struct KnowledgeGraph {
    /// Nodes indexed by ID
    nodes: HashMap<String, NodeType>,
    /// Edges stored as adjacency list
    edges: Vec<Edge>,
    /// Adjacency index for fast neighbor lookup
    adjacency: HashMap<String, Vec<usize>>,
}

impl KnowledgeGraph {
    /// Create a new knowledge graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Extract knowledge from tool result
    pub fn extract_from_result(&mut self, result: &ToolResult) -> Result<()> {
        match result.tool.as_str() {
            "list_dir" => self.extract_from_list_dir(result),
            "read_file" => self.extract_from_read_file(result),
            "write_file" => self.extract_from_write_file(result),
            "run_command" => self.extract_from_command(result),
            "system_info" => self.extract_from_system_info(result),
            _ => Ok(()),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: NodeType) -> String {
        let id = node.id();
        self.nodes.insert(id.clone(), node);
        id
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, from: String, to: String, edge_type: EdgeType) {
        let edge_idx = self.edges.len();
        self.edges.push(Edge {
            from: from.clone(),
            to,
            edge_type,
        });
        
        // Update adjacency index
        self.adjacency
            .entry(from)
            .or_insert_with(Vec::new)
            .push(edge_idx);
    }

    /// Find a node by identifier
    pub fn find_node(&self, id: &str) -> Option<&NodeType> {
        self.nodes.get(id)
    }

    /// Get neighbors of a node
    pub fn get_neighbors(&self, node_id: &str) -> Vec<&NodeType> {
        if let Some(edge_indices) = self.adjacency.get(node_id) {
            edge_indices
                .iter()
                .filter_map(|&idx| self.edges.get(idx))
                .filter_map(|edge| self.nodes.get(&edge.to))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get edges from a node
    pub fn get_edges(&self, node_id: &str) -> Vec<&Edge> {
        if let Some(edge_indices) = self.adjacency.get(node_id) {
            edge_indices
                .iter()
                .filter_map(|&idx| self.edges.get(idx))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find nodes by type
    pub fn find_nodes_by_type(&self, type_filter: impl Fn(&NodeType) -> bool) -> Vec<&NodeType> {
        self.nodes.values().filter(|n| type_filter(n)).collect()
    }

    /// Get all file nodes
    pub fn get_files(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter_map(|node| match node {
                NodeType::File { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get all directory nodes
    pub fn get_directories(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter_map(|node| match node {
                NodeType::Directory { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.adjacency.clear();
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    // Extraction methods

    fn extract_from_list_dir(&mut self, result: &ToolResult) -> Result<()> {
        // Note: We don't have access to original args in ToolResult
        // Parse directory path from output context if available
        // For now, use a generic "workspace" directory
        let parent_path = "workspace";

        let parent_id = self.add_node(NodeType::Directory {
            path: parent_path.to_string(),
        });

        // Parse output for files and directories
        for line in result.output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Simple heuristic: lines ending with / are directories
            if line.ends_with('/') {
                let dir_path = format!("{}/{}", parent_path, line.trim_end_matches('/'));
                let dir_id = self.add_node(NodeType::Directory { path: dir_path });
                self.add_edge(parent_id.clone(), dir_id, EdgeType::Contains);
            } else {
                let file_path = format!("{}/{}", parent_path, line);
                let file_id = self.add_node(NodeType::File {
                    path: file_path,
                    size: None,
                });
                self.add_edge(parent_id.clone(), file_id, EdgeType::Contains);
            }
        }

        Ok(())
    }

    fn extract_from_read_file(&mut self, result: &ToolResult) -> Result<()> {
        // Extract file path from tool name or use generic identifier
        let file_path = format!("file_{}", self.node_count());

        let file_id = self.add_node(NodeType::File {
            path: file_path,
            size: Some(result.output.len()),
        });

        // Extract concepts from content (simple keyword extraction)
        let concepts = self.extract_concepts(&result.output);
        for concept in concepts {
            let concept_id = self.add_node(NodeType::Concept {
                name: concept.clone(),
            });
            self.add_edge(concept_id, file_id.clone(), EdgeType::MentionedIn);
        }

        Ok(())
    }

    fn extract_from_write_file(&mut self, result: &ToolResult) -> Result<()> {
        // Use generic file identifier
        let file_path = format!("written_file_{}", self.node_count());

        self.add_node(NodeType::File {
            path: file_path,
            size: Some(result.output.len()),
        });

        Ok(())
    }

    fn extract_from_command(&mut self, result: &ToolResult) -> Result<()> {
        // Extract command name from tool identifier
        let cmd = &result.tool;

        self.add_node(NodeType::Command {
            name: cmd.to_string(),
            args: vec![],
        });

        Ok(())
    }

    fn extract_from_system_info(&mut self, _result: &ToolResult) -> Result<()> {
        // System info doesn't add nodes to the graph currently
        Ok(())
    }

    /// Extract concepts from text (keywords that might be important)
    fn extract_concepts(&self, text: &str) -> Vec<String> {
        // Simple concept extraction: find capitalized words and technical terms
        let mut concepts = HashSet::new();

        for word in text.split_whitespace() {
            let word = word.trim_matches(|c: char| !c.is_alphanumeric());
            
            // Extract capitalized words (potential proper nouns)
            if word.len() > 3 && word.chars().next().map_or(false, |c| c.is_uppercase()) {
                concepts.insert(word.to_string());
            }

            // Extract technical terms (contains underscore or camelCase)
            if word.len() > 4 && (word.contains('_') || Self::is_camel_case(word)) {
                concepts.insert(word.to_string());
            }
        }

        concepts.into_iter().take(10).collect() // Limit to 10 concepts per file
    }

    /// Check if string is camelCase
    fn is_camel_case(s: &str) -> bool {
        s.chars().any(|c| c.is_lowercase()) && s.chars().any(|c| c.is_uppercase())
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_graph_creation() {
        let graph = KnowledgeGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = KnowledgeGraph::new();
        let node = NodeType::File {
            path: "test.txt".to_string(),
            size: Some(100),
        };
        
        let id = graph.add_node(node);
        assert_eq!(id, "file:test.txt");
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = KnowledgeGraph::new();
        
        let dir_id = graph.add_node(NodeType::Directory {
            path: "src".to_string(),
        });
        let file_id = graph.add_node(NodeType::File {
            path: "src/main.rs".to_string(),
            size: None,
        });
        
        graph.add_edge(dir_id.clone(), file_id, EdgeType::Contains);
        
        assert_eq!(graph.edge_count(), 1);
        let neighbors = graph.get_neighbors(&dir_id);
        assert_eq!(neighbors.len(), 1);
    }

    #[test]
    fn test_find_node() {
        let mut graph = KnowledgeGraph::new();
        let node = NodeType::File {
            path: "test.txt".to_string(),
            size: Some(100),
        };
        
        graph.add_node(node);
        
        let found = graph.find_node("file:test.txt");
        assert!(found.is_some());
        
        let not_found = graph.find_node("file:nonexistent.txt");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_files() {
        let mut graph = KnowledgeGraph::new();
        
        graph.add_node(NodeType::File {
            path: "file1.txt".to_string(),
            size: None,
        });
        graph.add_node(NodeType::File {
            path: "file2.txt".to_string(),
            size: None,
        });
        graph.add_node(NodeType::Directory {
            path: "dir1".to_string(),
        });
        
        let files = graph.get_files();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_concept_extraction() {
        let graph = KnowledgeGraph::new();
        let text = "The DatabaseConnection class handles ConnectionPool initialization";
        
        let concepts = graph.extract_concepts(text);
        
        assert!(concepts.iter().any(|c| c.contains("Database")));
        assert!(concepts.iter().any(|c| c.contains("Connection")));
    }

    #[test]
    fn test_is_camel_case() {
        assert!(KnowledgeGraph::is_camel_case("camelCase"));
        assert!(KnowledgeGraph::is_camel_case("PascalCase"));
        assert!(!KnowledgeGraph::is_camel_case("lowercase"));
        assert!(!KnowledgeGraph::is_camel_case("UPPERCASE"));
    }
}
