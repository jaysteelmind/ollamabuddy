//! Bounded memory manager for agent conversation history
//! 
//! Maintains a fixed-size circular buffer of memory entries with:
//! - Maximum 100 entries (bounded storage)
//! - FIFO eviction when full
//! - Fast access to recent entries
//! - Token tracking for context management

use crate::errors::{AgentError, Result};
use crate::types::MemoryEntry;
use std::collections::VecDeque;

/// Maximum number of memory entries (bounded storage guarantee)
pub const MAX_MEMORY_ENTRIES: usize = 100;

/// Memory manager with bounded storage
#[derive(Debug, Clone)]
pub struct MemoryManager {
    /// Circular buffer of memory entries (bounded by MAX_MEMORY_ENTRIES)
    entries: VecDeque<MemoryEntry>,
    
    /// Maximum allowed entries
    max_entries: usize,
}

impl MemoryManager {
    /// Create new memory manager with default capacity
    pub fn new() -> Self {
        Self::with_capacity(MAX_MEMORY_ENTRIES)
    }

    /// Create memory manager with custom capacity
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Add entry to memory, evicting oldest if at capacity
    /// 
    /// # Complexity
    /// - O(1) amortized - VecDeque push_back
    /// - O(1) eviction - VecDeque pop_front
    pub fn add(&mut self, entry: MemoryEntry) {
        // Evict oldest if at capacity
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }

        self.entries.push_back(entry);
    }

    /// Get reference to all entries
    pub fn entries(&self) -> &VecDeque<MemoryEntry> {
        &self.entries
    }

    /// Get mutable reference to all entries
    pub fn entries_mut(&mut self) -> &mut VecDeque<MemoryEntry> {
        &mut self.entries
    }

    /// Get the last N entries
    /// 
    /// # Complexity
    /// O(N) - creates slice reference
    pub fn last_n(&self, n: usize) -> Vec<&MemoryEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries.range(start..).collect()
    }

    /// Get the most recent entry
    pub fn last(&self) -> Option<&MemoryEntry> {
        self.entries.back()
    }

    /// Get the most recent entry of a specific type
    pub fn last_of_type<F>(&self, predicate: F) -> Option<&MemoryEntry>
    where
        F: Fn(&MemoryEntry) -> bool,
    {
        self.entries.iter().rev().find(|entry| predicate(entry))
    }

    /// Count total entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Calculate total token count for all entries
    /// 
    /// # Complexity
    /// O(n) - single pass through all entries
    pub fn total_tokens(&self) -> usize {
        self.entries.iter().map(|e| e.estimate_tokens()).sum()
    }

    /// Get entries as a vector (for compression/serialization)
    pub fn to_vec(&self) -> Vec<MemoryEntry> {
        self.entries.iter().cloned().collect()
    }

    /// Replace all entries with new set
    pub fn replace_all(&mut self, new_entries: Vec<MemoryEntry>) -> Result<()> {
        if new_entries.len() > self.max_entries {
            return Err(AgentError::MemoryOverflow {
                current: new_entries.len(),
                max: self.max_entries,
            });
        }

        self.entries.clear();
        self.entries.extend(new_entries);
        Ok(())
    }

    /// Get the last tool call entry if it exists
    pub fn last_tool_call(&self) -> Option<(String, std::collections::HashMap<String, serde_json::Value>)> {
        self.entries.iter().rev().find_map(|entry| {
            if let MemoryEntry::ToolCall { tool, args, .. } = entry {
                Some((tool.clone(), args.clone()))
            } else {
                None
            }
        })
    }

    /// Get the system prompt entry if it exists
    pub fn system_prompt(&self) -> Option<&str> {
        self.entries.iter().find_map(|entry| {
            if let MemoryEntry::SystemPrompt { content } = entry {
                Some(content.as_str())
            } else {
                None
            }
        })
    }

    /// Get the user goal entry if it exists
    pub fn user_goal(&self) -> Option<&str> {
        self.entries.iter().find_map(|entry| {
            if let MemoryEntry::UserGoal { goal, .. } = entry {
                Some(goal.as_str())
            } else {
                None
            }
        })
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_entry(id: u64) -> MemoryEntry {
        MemoryEntry::UserGoal {
            goal: format!("Test goal {}", id),
            timestamp: id,
        }
    }

    #[test]
    fn test_bounded_capacity() {
        let mut memory = MemoryManager::with_capacity(5);

        // Add 10 entries, only last 5 should remain
        for i in 0..10 {
            memory.add(create_test_entry(i));
        }

        assert_eq!(memory.len(), 5);
        assert_eq!(memory.last().unwrap().timestamp(), 9);
    }

    #[test]
    fn test_fifo_eviction() {
        let mut memory = MemoryManager::with_capacity(3);

        memory.add(create_test_entry(1));
        memory.add(create_test_entry(2));
        memory.add(create_test_entry(3));
        memory.add(create_test_entry(4)); // Should evict entry 1

        assert_eq!(memory.len(), 3);
        assert_eq!(memory.entries()[0].timestamp(), 2);
        assert_eq!(memory.entries()[2].timestamp(), 4);
    }

    #[test]
    fn test_last_n() {
        let mut memory = MemoryManager::new();

        for i in 0..10 {
            memory.add(create_test_entry(i));
        }

        let last_3 = memory.last_n(3);
        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].timestamp(), 7);
        assert_eq!(last_3[2].timestamp(), 9);
    }

    #[test]
    fn test_total_tokens() {
        let mut memory = MemoryManager::new();

        memory.add(MemoryEntry::UserGoal {
            goal: "a".repeat(400), // ~100 tokens
            timestamp: 1,
        });

        memory.add(MemoryEntry::UserGoal {
            goal: "b".repeat(400), // ~100 tokens
            timestamp: 2,
        });

        let tokens = memory.total_tokens();
        assert!(tokens >= 150 && tokens <= 250); // Rough estimate
    }

    #[test]
    fn test_clear() {
        let mut memory = MemoryManager::new();
        memory.add(create_test_entry(1));
        memory.add(create_test_entry(2));

        assert_eq!(memory.len(), 2);

        memory.clear();
        assert_eq!(memory.len(), 0);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_last_tool_call() {
        let mut memory = MemoryManager::new();

        let mut args = HashMap::new();
        args.insert("path".to_string(), serde_json::json!("/test"));

        memory.add(MemoryEntry::ToolCall {
            tool: "read_file".to_string(),
            args: args.clone(),
            timestamp: 1,
        });

        let (tool, retrieved_args) = memory.last_tool_call().unwrap();
        assert_eq!(tool, "read_file");
        assert_eq!(retrieved_args.get("path").unwrap(), &serde_json::json!("/test"));
    }

    #[test]
    fn test_system_prompt_retrieval() {
        let mut memory = MemoryManager::new();

        memory.add(MemoryEntry::SystemPrompt {
            content: "You are a helpful assistant".to_string(),
        });

        memory.add(create_test_entry(1));

        assert_eq!(
            memory.system_prompt().unwrap(),
            "You are a helpful assistant"
        );
    }

    #[test]
    fn test_replace_all() {
        let mut memory = MemoryManager::with_capacity(5);

        memory.add(create_test_entry(1));
        memory.add(create_test_entry(2));

        let new_entries = vec![create_test_entry(10), create_test_entry(11)];
        memory.replace_all(new_entries).unwrap();

        assert_eq!(memory.len(), 2);
        assert_eq!(memory.entries()[0].timestamp(), 10);
    }

    #[test]
    fn test_replace_all_overflow() {
        let mut memory = MemoryManager::with_capacity(2);

        let too_many = vec![
            create_test_entry(1),
            create_test_entry(2),
            create_test_entry(3),
        ];

        let result = memory.replace_all(too_many);
        assert!(result.is_err());
    }
}
