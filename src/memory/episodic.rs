//! Episodic Memory: Session-scoped experience tracking
//!
//! Implements a bounded circular buffer of episodes with O(1) operations.

use crate::memory::types::{Episode, EpisodeId};
use std::collections::{HashMap, VecDeque};

/// Maximum number of episodes to store
const MAX_EPISODES: usize = 100;

/// Episodic memory storage
pub struct EpisodicMemory {
    /// Circular buffer of episodes (FIFO eviction)
    episodes: VecDeque<Episode>,
    /// Hash-based index for fast lookup
    index: HashMap<u64, Vec<EpisodeId>>,
}

impl EpisodicMemory {
    /// Create a new episodic memory
    pub fn new() -> Self {
        Self {
            episodes: VecDeque::with_capacity(MAX_EPISODES),
            index: HashMap::new(),
        }
    }

    /// Add an episode to memory
    /// Complexity: O(1)
    pub fn add_episode(&mut self, episode: Episode) {
        // Evict oldest if at capacity
        if self.episodes.len() >= MAX_EPISODES {
            if let Some(old_episode) = self.episodes.pop_front() {
                self.remove_from_index(&old_episode);
            }
        }

        // Add to index
        let hash = episode.metadata.similarity_hash;
        self.index
            .entry(hash)
            .or_insert_with(Vec::new)
            .push(episode.id);

        // Add to buffer
        self.episodes.push_back(episode);
    }

    /// Find episodes by similarity hash
    /// Complexity: O(k) where k = number of collisions
    pub fn find_by_hash(&self, hash: u64) -> Vec<&Episode> {
        if let Some(ids) = self.index.get(&hash) {
            self.episodes
                .iter()
                .filter(|e| ids.contains(&e.id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all episodes
    pub fn get_all(&self) -> &VecDeque<Episode> {
        &self.episodes
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

    /// Get memory size
    pub fn len(&self) -> usize {
        self.episodes.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new()
    }
}
