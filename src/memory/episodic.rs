//! Episodic Memory: Session-scoped experience tracking
//!
//! Implements a bounded circular buffer of episodes with O(1) operations.
//! Uses similarity hashing for fast lookup of related episodes.

use crate::memory::types::{Episode, EpisodeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Maximum number of episodes to store
const MAX_EPISODES: usize = 100;

/// Episodic memory storage
pub struct EpisodicMemory {
    /// Circular buffer of episodes (FIFO eviction)
    episodes: VecDeque<Episode>,
    /// Hash-based index for fast lookup
    index: HashMap<u64, Vec<EpisodeId>>,
    /// Episode ID to position mapping for O(1) access
    id_map: HashMap<EpisodeId, usize>,
}

impl EpisodicMemory {
    /// Create a new episodic memory
    pub fn new() -> Self {
        Self {
            episodes: VecDeque::with_capacity(MAX_EPISODES),
            index: HashMap::new(),
            id_map: HashMap::new(),
        }
    }

    /// Add an episode to memory
    /// Complexity: O(1) amortized
    pub fn add_episode(&mut self, mut episode: Episode) {
        // Compute similarity hash if not already set
        if episode.metadata.similarity_hash == 0 {
            episode.metadata.similarity_hash = self.compute_similarity_hash(&episode);
        }

        // Evict oldest if at capacity
        if self.episodes.len() >= MAX_EPISODES {
            if let Some(old_episode) = self.episodes.pop_front() {
                self.remove_from_index(&old_episode);
                self.id_map.remove(&old_episode.id);
            }
            // Rebuild position map after eviction
            self.rebuild_id_map();
        }

        // Add to index
        let hash = episode.metadata.similarity_hash;
        self.index
            .entry(hash)
            .or_insert_with(Vec::new)
            .push(episode.id);

        // Add position mapping
        self.id_map.insert(episode.id, self.episodes.len());

        // Add to buffer
        self.episodes.push_back(episode);
    }

    /// Find episodes by similarity hash
    /// Complexity: O(k) where k = number of collisions
    pub fn find_by_hash(&self, hash: u64) -> Vec<&Episode> {
        if let Some(ids) = self.index.get(&hash) {
            ids.iter()
                .filter_map(|id| self.get_by_id(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get episode by ID
    /// Complexity: O(1)
    pub fn get_by_id(&self, id: &EpisodeId) -> Option<&Episode> {
        self.id_map.get(id).and_then(|&pos| self.episodes.get(pos))
    }

    /// Find episodes with similar goals
    /// Uses fuzzy matching on goal keywords
    pub fn find_similar(&self, goal: &str, threshold: f64) -> Vec<(&Episode, f64)> {
        let goal_keywords = Self::extract_keywords(goal);
        let mut results = Vec::new();

        for episode in &self.episodes {
            let ep_keywords = Self::extract_keywords(&episode.goal);
            let similarity = Self::compute_keyword_similarity(&goal_keywords, &ep_keywords);
            
            if similarity >= threshold {
                results.push((episode, similarity));
            }
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Get all episodes
    pub fn get_all(&self) -> &VecDeque<Episode> {
        &self.episodes
    }

    /// Get recent episodes (last N)
    pub fn get_recent(&self, n: usize) -> Vec<&Episode> {
        self.episodes.iter().rev().take(n).collect()
    }

    /// Remove episode from index
    fn remove_from_index(&mut self, episode: &Episode) {
        let hash = episode.metadata.similarity_hash;
        if let Some(ids) = self.index.get_mut(&hash) {
            ids.retain(|id| *id != episode.id);
            if ids.is_empty() {
                self.index.remove(&hash);
            }
        }
    }

    /// Rebuild ID to position mapping
    fn rebuild_id_map(&mut self) {
        self.id_map.clear();
        for (pos, episode) in self.episodes.iter().enumerate() {
            self.id_map.insert(episode.id, pos);
        }
    }

    /// Compute similarity hash for an episode
    /// Hash combines: goal keywords + tool sequence + complexity bucket
    fn compute_similarity_hash(&self, episode: &Episode) -> u64 {
        let mut hasher = DefaultHasher::new();

        // Hash goal keywords
        let keywords = Self::extract_keywords(&episode.goal);
        for keyword in keywords {
            keyword.hash(&mut hasher);
        }

        // Hash tool sequence
        for action in &episode.actions {
            action.tool.hash(&mut hasher);
        }

        // Hash complexity bucket (0-10)
        let complexity_bucket = (episode.metadata.complexity_score * 10.0).floor() as u8;
        complexity_bucket.hash(&mut hasher);

        hasher.finish()
    }

    /// Extract keywords from text (simple tokenization)
    fn extract_keywords(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 3) // Filter short words
            .filter(|word| !Self::is_stopword(word))
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if word is a stopword
    fn is_stopword(word: &str) -> bool {
        matches!(
            word,
            "this" | "that" | "these" | "those" | "with" | "from" | "have" | "been" | "were" | "what" | "when" | "where" | "which"
        )
    }

    /// Compute keyword similarity using Jaccard index
    fn compute_keyword_similarity(keywords1: &[String], keywords2: &[String]) -> f64 {
        if keywords1.is_empty() || keywords2.is_empty() {
            return 0.0;
        }

        let set1: HashSet<_> = keywords1.iter().collect();
        let set2: HashSet<_> = keywords2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Get memory size
    pub fn len(&self) -> usize {
        self.episodes.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }

    /// Clear all episodes
    pub fn clear(&mut self) {
        self.episodes.clear();
        self.index.clear();
        self.id_map.clear();
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{ActionRecord, EpisodeMetadata, EpisodeOutcome};

    fn create_test_episode(goal: &str, tools: &[&str]) -> Episode {
        let mut episode = Episode::new(goal.to_string(), "test context".to_string());
        
        for tool in tools {
            episode.actions.push(ActionRecord {
                tool: tool.to_string(),
                args: serde_json::json!({}),
                result: "success".to_string(),
                success: true,
            });
        }

        episode.metadata.tool_count = tools.len();
        episode.metadata.complexity_score = 0.5;
        episode.outcome = EpisodeOutcome::Success;
        
        episode
    }

    #[test]
    fn test_episodic_memory_creation() {
        let memory = EpisodicMemory::new();
        assert_eq!(memory.len(), 0);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_add_and_retrieve_episode() {
        let mut memory = EpisodicMemory::new();
        let episode = create_test_episode("test goal", &["read_file"]);
        let id = episode.id;
        
        memory.add_episode(episode);
        
        assert_eq!(memory.len(), 1);
        assert!(memory.get_by_id(&id).is_some());
    }

    #[test]
    fn test_bounded_capacity() {
        let mut memory = EpisodicMemory::new();
        
        for i in 0..150 {
            let episode = create_test_episode(&format!("Goal {}", i), &["read_file"]);
            memory.add_episode(episode);
        }
        
        assert_eq!(memory.len(), 100);
    }

    #[test]
    fn test_similarity_hash_computation() {
        let memory = EpisodicMemory::new();
        let ep1 = create_test_episode("read the file", &["read_file"]);
        let ep2 = create_test_episode("read the file", &["read_file"]);
        
        let hash1 = memory.compute_similarity_hash(&ep1);
        let hash2 = memory.compute_similarity_hash(&ep2);
        
        assert_eq!(hash1, hash2, "Similar episodes should have same hash");
    }

    #[test]
    fn test_find_by_hash() {
        let mut memory = EpisodicMemory::new();
        let episode = create_test_episode("test goal", &["read_file"]);
        let hash = memory.compute_similarity_hash(&episode);
        
        memory.add_episode(episode);
        
        let found = memory.find_by_hash(hash);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_find_similar_episodes() {
        let mut memory = EpisodicMemory::new();
        
        memory.add_episode(create_test_episode("read the configuration file", &["read_file"]));
        memory.add_episode(create_test_episode("write the output file", &["write_file"]));
        memory.add_episode(create_test_episode("read the input file", &["read_file"]));
        
        let similar = memory.find_similar("read the data file", 0.3);
        
        assert!(similar.len() >= 2, "Should find at least 2 similar episodes");
        assert!(similar[0].1 > similar[1].1 || (similar[0].1 - similar[1].1).abs() < 0.01, 
                "Results should be sorted by similarity");
    }

    #[test]
    fn test_keyword_extraction() {
        let keywords = EpisodicMemory::extract_keywords("Read the configuration file from disk");
        assert!(keywords.contains(&"read".to_string()));
        assert!(keywords.contains(&"configuration".to_string()));
        assert!(keywords.contains(&"file".to_string()));
        assert!(keywords.contains(&"disk".to_string()));
        assert!(!keywords.contains(&"the".to_string()), "Should filter stopwords");
    }

    #[test]
    fn test_jaccard_similarity() {
        let kw1 = vec!["read".to_string(), "file".to_string(), "data".to_string()];
        let kw2 = vec!["read".to_string(), "file".to_string(), "config".to_string()];
        
        let sim = EpisodicMemory::compute_keyword_similarity(&kw1, &kw2);
        
        // Intersection: {read, file} = 2
        // Union: {read, file, data, config} = 4
        // Jaccard: 2/4 = 0.5
        assert!((sim - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_get_recent() {
        let mut memory = EpisodicMemory::new();
        
        for i in 0..10 {
            memory.add_episode(create_test_episode(&format!("Goal {}", i), &["read_file"]));
        }
        
        let recent = memory.get_recent(3);
        assert_eq!(recent.len(), 3);
        assert!(recent[0].goal.contains("Goal 9"));
    }

    #[test]
    fn test_clear() {
        let mut memory = EpisodicMemory::new();
        memory.add_episode(create_test_episode("test", &["read_file"]));
        
        assert_eq!(memory.len(), 1);
        memory.clear();
        assert_eq!(memory.len(), 0);
        assert!(memory.is_empty());
    }
}
