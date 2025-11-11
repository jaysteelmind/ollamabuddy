//! Integration tests for Memory & Learning System (PRD 6)

#[cfg(test)]
mod episodic_tests {
    use ollamabuddy::memory::{EpisodicMemory, Episode};

    #[test]
    fn test_episodic_memory_creation() {
        let memory = EpisodicMemory::new();
        assert_eq!(memory.len(), 0);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_add_episode() {
        let mut memory = EpisodicMemory::new();
        let episode = Episode::new(
            "Test goal".to_string(),
            "Test context".to_string(),
        );
        
        memory.add_episode(episode);
        assert_eq!(memory.len(), 1);
        assert!(!memory.is_empty());
    }

    #[test]
    fn test_bounded_capacity() {
        let mut memory = EpisodicMemory::new();
        
        // Add more than max capacity (100 episodes)
        for i in 0..150 {
            let episode = Episode::new(
                format!("Goal {}", i),
                format!("Context {}", i),
            );
            memory.add_episode(episode);
        }
        
        // Should be capped at 100
        assert_eq!(memory.len(), 100);
    }
}

#[cfg(test)]
mod knowledge_tests {
    use ollamabuddy::memory::KnowledgeGraph;

    #[test]
    fn test_knowledge_graph_creation() {
        let graph = KnowledgeGraph::new();
        assert_eq!(graph.find_node("nonexistent"), None);
    }
}

#[cfg(test)]
mod experience_tests {
    use ollamabuddy::memory::ExperienceTracker;

    #[test]
    fn test_experience_tracker_creation() {
        let tracker = ExperienceTracker::new();
        assert_eq!(tracker.total_experiences(), 0);
    }
}

#[cfg(test)]
mod working_memory_tests {
    use ollamabuddy::memory::WorkingMemory;

    #[test]
    fn test_working_memory_creation() {
        let memory = WorkingMemory::new();
        assert_eq!(memory.get_goal(), None);
        assert_eq!(memory.get_recent_tools().len(), 0);
    }

    #[test]
    fn test_set_goal() {
        let mut memory = WorkingMemory::new();
        memory.set_goal("Test goal".to_string());
        assert_eq!(memory.get_goal(), Some("Test goal"));
    }
}
