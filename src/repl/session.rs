//! Session manager for REPL context and history tracking
//! 
//! Maintains conversation context, task history, and file tracking
//! Performance target: <20ms context building

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of tasks to keep in history
const MAX_HISTORY_SIZE: usize = 1000;

/// Maximum number of recent tasks to include in context
const MAX_CONTEXT_TASKS: usize = 5;

/// Record of a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task: String,
    pub result: String,
    pub success: bool,
    pub duration_ms: u64,
    pub timestamp: u64,
    pub files_modified: Vec<PathBuf>,
}

/// Session manager maintaining REPL state
/// 
/// Tracks:
/// - Task history (bounded to MAX_HISTORY_SIZE)
/// - Context from recent tasks (k=5 most recent)
/// - Files touched during session
/// - Session metadata
pub struct SessionManager {
    /// Task history (FIFO queue, max 1000 entries)
    history: VecDeque<TaskRecord>,
    
    /// Files accessed or modified in this session
    tracked_files: HashSet<PathBuf>,
    
    /// Session start time
    session_start: u64,
    
    /// Total tasks executed
    task_count: usize,
}

impl SessionManager {
    /// Create new session manager
    /// 
    /// Complexity: O(1) initialization
    pub fn new() -> Self {
        let session_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        SessionManager {
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            tracked_files: HashSet::new(),
            session_start,
            task_count: 0,
        }
    }
    
    /// Record a completed task
    /// 
    /// Complexity: O(1) append, O(1) eviction if at capacity
    pub fn record_task(&mut self, record: TaskRecord) {
        // Track any files mentioned in the task
        for file in &record.files_modified {
            self.tracked_files.insert(file.clone());
        }
        
        // Add to history (bounded queue)
        if self.history.len() >= MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(record);
        
        self.task_count += 1;
    }
    
    /// Build context string from recent tasks
    /// 
    /// Complexity: O(k) where k = MAX_CONTEXT_TASKS (typically 5)
    /// Performance target: <20ms
    pub fn build_context(&self) -> String {
        if self.history.is_empty() {
            return String::new();
        }
        
        let mut context = String::from("Recent context:\n");
        
        // Get last k tasks (or all if fewer than k)
        let recent_count = self.history.len().min(MAX_CONTEXT_TASKS);
        let recent_tasks = self.history.iter().rev().take(recent_count);
        
        for (i, record) in recent_tasks.enumerate() {
            let status = if record.success { "completed" } else { "failed" };
            context.push_str(&format!(
                "{}. Task: {} - Status: {} ({}ms)\n",
                recent_count - i,
                record.task,
                status,
                record.duration_ms
            ));
        }
        
        // Add tracked files if any
        if !self.tracked_files.is_empty() {
            context.push_str("\nFiles in context:\n");
            for (i, file) in self.tracked_files.iter().enumerate().take(10) {
                context.push_str(&format!("  {}. {}\n", i + 1, file.display()));
            }
            if self.tracked_files.len() > 10 {
                context.push_str(&format!("  ... and {} more\n", self.tracked_files.len() - 10));
            }
        }
        
        context
    }
    
    /// Get task history (newest first)
    /// 
    /// Returns up to `limit` most recent tasks
    pub fn get_history(&self, limit: usize) -> Vec<&TaskRecord> {
        self.history.iter().rev().take(limit).collect()
    }
    
    /// Get all tracked files
    pub fn get_tracked_files(&self) -> Vec<PathBuf> {
        self.tracked_files.iter().cloned().collect()
    }
    
    /// Add a file to tracked files
    pub fn track_file(&mut self, path: PathBuf) {
        self.tracked_files.insert(path);
    }
    
    /// Clear session state (reset)
    pub fn reset(&mut self) {
        self.history.clear();
        self.tracked_files.clear();
        self.task_count = 0;
        self.session_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// Get total task count
    pub fn task_count(&self) -> usize {
        self.task_count
    }
    
    /// Get session duration in seconds
    pub fn session_duration(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.session_start
    }
    
    /// Get history size
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
    
    /// Check if session has context
    pub fn has_context(&self) -> bool {
        !self.history.is_empty()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(task: &str, success: bool) -> TaskRecord {
        TaskRecord {
            task: task.to_string(),
            result: "test result".to_string(),
            success,
            duration_ms: 100,
            timestamp: 1234567890,
            files_modified: vec![],
        }
    }

    #[test]
    fn test_session_creation() {
        let session = SessionManager::new();
        assert_eq!(session.task_count(), 0);
        assert_eq!(session.history_len(), 0);
        assert!(!session.has_context());
    }

    #[test]
    fn test_record_task() {
        let mut session = SessionManager::new();
        let record = create_test_record("test task", true);
        
        session.record_task(record);
        
        assert_eq!(session.task_count(), 1);
        assert_eq!(session.history_len(), 1);
        assert!(session.has_context());
    }

    #[test]
    fn test_build_context_empty() {
        let session = SessionManager::new();
        let context = session.build_context();
        assert_eq!(context, "");
    }

    #[test]
    fn test_build_context_with_tasks() {
        let mut session = SessionManager::new();
        session.record_task(create_test_record("task 1", true));
        session.record_task(create_test_record("task 2", false));
        
        let context = session.build_context();
        assert!(context.contains("task 1"));
        assert!(context.contains("task 2"));
        assert!(context.contains("completed"));
        assert!(context.contains("failed"));
    }

    #[test]
    fn test_context_limited_to_recent_tasks() {
        let mut session = SessionManager::new();
        
        // Add 10 tasks
        for i in 0..10 {
            session.record_task(create_test_record(&format!("task {}", i), true));
        }
        
        let context = session.build_context();
        
        // Should only contain last 5 tasks
        assert!(context.contains("task 9"));
        assert!(context.contains("task 5"));
        assert!(!context.contains("task 4"));
    }

    #[test]
    fn test_history_bounded() {
        let mut session = SessionManager::new();
        
        // Add more than MAX_HISTORY_SIZE tasks
        for i in 0..1100 {
            session.record_task(create_test_record(&format!("task {}", i), true));
        }
        
        // Should be capped at MAX_HISTORY_SIZE
        assert_eq!(session.history_len(), MAX_HISTORY_SIZE);
        assert_eq!(session.task_count(), 1100); // But count is accurate
    }

    #[test]
    fn test_file_tracking() {
        let mut session = SessionManager::new();
        let path = PathBuf::from("/test/file.txt");
        
        session.track_file(path.clone());
        
        let tracked = session.get_tracked_files();
        assert_eq!(tracked.len(), 1);
        assert!(tracked.contains(&path));
    }

    #[test]
    fn test_file_tracking_in_record() {
        let mut session = SessionManager::new();
        let mut record = create_test_record("test", true);
        record.files_modified = vec![PathBuf::from("/test/file.txt")];
        
        session.record_task(record);
        
        let tracked = session.get_tracked_files();
        assert_eq!(tracked.len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut session = SessionManager::new();
        session.record_task(create_test_record("task", true));
        session.track_file(PathBuf::from("/test/file.txt"));
        
        assert_eq!(session.task_count(), 1);
        assert_eq!(session.history_len(), 1);
        
        session.reset();
        
        assert_eq!(session.task_count(), 0);
        assert_eq!(session.history_len(), 0);
        assert_eq!(session.get_tracked_files().len(), 0);
    }

    #[test]
    fn test_get_history_limit() {
        let mut session = SessionManager::new();
        
        for i in 0..10 {
            session.record_task(create_test_record(&format!("task {}", i), true));
        }
        
        let history = session.get_history(3);
        assert_eq!(history.len(), 3);
        
        // Should be newest first
        assert!(history[0].task.contains("task 9"));
        assert!(history[1].task.contains("task 8"));
        assert!(history[2].task.contains("task 7"));
    }

    #[test]
    fn test_session_duration() {
        let session = SessionManager::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = session.session_duration();
        assert!(duration >= 0);
    }

    #[test]
    fn test_context_build_performance() {
        let mut session = SessionManager::new();
        
        // Add maximum context tasks
        for i in 0..MAX_CONTEXT_TASKS {
            session.record_task(create_test_record(&format!("task {}", i), true));
        }
        
        let start = std::time::Instant::now();
        let _context = session.build_context();
        let elapsed = start.elapsed();
        
        // Should be well under 20ms target
        assert!(elapsed.as_millis() < 20, "Context building too slow: {:?}", elapsed);
    }
}
