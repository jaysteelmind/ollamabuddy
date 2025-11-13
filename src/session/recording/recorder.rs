// PRD 11 Phase 3: Session Recorder for task execution tracking
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Task execution outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskOutcome {
    Success,
    Failure,
    Partial,
    Timeout,
}

/// Recorded task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Unique task ID
    pub id: String,
    /// Task description/goal
    pub goal: String,
    /// Outcome of execution
    pub outcome: TaskOutcome,
    /// Number of iterations used
    pub iterations: usize,
    /// Execution duration in seconds
    pub duration_secs: f64,
    /// Files created/modified
    pub files_touched: Vec<String>,
    /// Tools used
    pub tools_used: Vec<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Timestamp of execution
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl TaskRecord {
    /// Create new task record
    pub fn new(goal: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            goal,
            outcome: TaskOutcome::Success,
            iterations: 0,
            duration_secs: 0.0,
            files_touched: Vec::new(),
            tools_used: Vec::new(),
            error: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Mark task as successful
    pub fn success(mut self) -> Self {
        self.outcome = TaskOutcome::Success;
        self
    }

    /// Mark task as failed with error
    pub fn failure(mut self, error: String) -> Self {
        self.outcome = TaskOutcome::Failure;
        self.error = Some(error);
        self
    }

    /// Set execution metrics
    pub fn with_metrics(mut self, iterations: usize, duration_secs: f64) -> Self {
        self.iterations = iterations;
        self.duration_secs = duration_secs;
        self
    }

    /// Add file that was touched
    pub fn add_file(&mut self, file: String) {
        if !self.files_touched.contains(&file) {
            self.files_touched.push(file);
        }
    }

    /// Add tool that was used
    pub fn add_tool(&mut self, tool: String) {
        if !self.tools_used.contains(&tool) {
            self.tools_used.push(tool);
        }
    }

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

/// Session recording container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Unique session ID
    pub id: String,
    /// Session start time
    pub start_time: DateTime<Utc>,
    /// Session end time
    pub end_time: Option<DateTime<Utc>>,
    /// All task records in this session
    pub tasks: Vec<TaskRecord>,
    /// Total tasks attempted
    pub total_tasks: usize,
    /// Successful tasks count
    pub successful_tasks: usize,
    /// Failed tasks count
    pub failed_tasks: usize,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

impl SessionData {
    /// Create new session
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            start_time: Utc::now(),
            end_time: None,
            tasks: Vec::new(),
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            metadata: HashMap::new(),
        }
    }

    /// Add task record to session
    pub fn add_task(&mut self, task: TaskRecord) {
        self.total_tasks += 1;
        match task.outcome {
            TaskOutcome::Success => self.successful_tasks += 1,
            TaskOutcome::Failure => self.failed_tasks += 1,
            _ => {}
        }
        self.tasks.push(task);
    }

    /// Mark session as ended
    pub fn end_session(&mut self) {
        self.end_time = Some(Utc::now());
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            0.0
        } else {
            self.successful_tasks as f64 / self.total_tasks as f64
        }
    }

    /// Get session duration in seconds
    pub fn duration_secs(&self) -> f64 {
        let end = self.end_time.unwrap_or_else(Utc::now);
        (end - self.start_time).num_seconds() as f64
    }
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new()
    }
}

/// Session recorder for tracking task execution
pub struct SessionRecorder {
    current_session: SessionData,
}

impl SessionRecorder {
    /// Create new session recorder
    pub fn new() -> Self {
        Self {
            current_session: SessionData::new(),
        }
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.current_session.id
    }

    /// Record a completed task
    pub fn record_task(&mut self, task: TaskRecord) {
        self.current_session.add_task(task);
    }

    /// Get current session data
    pub fn current_session(&self) -> &SessionData {
        &self.current_session
    }

    /// Get mutable reference to current session
    pub fn current_session_mut(&mut self) -> &mut SessionData {
        &mut self.current_session
    }

    /// End current session and start new one
    pub fn new_session(&mut self) -> SessionData {
        let mut old_session = SessionData::new();
        std::mem::swap(&mut self.current_session, &mut old_session);
        old_session.end_session();
        old_session
    }

    /// Get statistics for current session
    pub fn session_stats(&self) -> SessionStats {
        SessionStats {
            total_tasks: self.current_session.total_tasks,
            successful_tasks: self.current_session.successful_tasks,
            failed_tasks: self.current_session.failed_tasks,
            success_rate: self.current_session.success_rate(),
            duration_secs: self.current_session.duration_secs(),
        }
    }
}

impl Default for SessionRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Session statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub success_rate: f64,
    pub duration_secs: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_record_creation() {
        let task = TaskRecord::new("test goal".to_string());
        assert_eq!(task.goal, "test goal");
        assert_eq!(task.outcome, TaskOutcome::Success);
        assert_eq!(task.iterations, 0);
    }

    #[test]
    fn test_task_record_builder() {
        let task = TaskRecord::new("test".to_string())
            .success()
            .with_metrics(5, 10.5);
        
        assert_eq!(task.outcome, TaskOutcome::Success);
        assert_eq!(task.iterations, 5);
        assert_eq!(task.duration_secs, 10.5);
    }

    #[test]
    fn test_task_record_failure() {
        let task = TaskRecord::new("test".to_string())
            .failure("error message".to_string());
        
        assert_eq!(task.outcome, TaskOutcome::Failure);
        assert_eq!(task.error, Some("error message".to_string()));
    }

    #[test]
    fn test_session_data_creation() {
        let session = SessionData::new();
        assert_eq!(session.total_tasks, 0);
        assert_eq!(session.successful_tasks, 0);
        assert!(session.end_time.is_none());
    }

    #[test]
    fn test_session_add_task() {
        let mut session = SessionData::new();
        let task = TaskRecord::new("test".to_string()).success();
        
        session.add_task(task);
        
        assert_eq!(session.total_tasks, 1);
        assert_eq!(session.successful_tasks, 1);
        assert_eq!(session.tasks.len(), 1);
    }

    #[test]
    fn test_session_success_rate() {
        let mut session = SessionData::new();
        
        session.add_task(TaskRecord::new("1".to_string()).success());
        session.add_task(TaskRecord::new("2".to_string()).failure("err".to_string()));
        session.add_task(TaskRecord::new("3".to_string()).success());
        
        assert_eq!(session.success_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_recorder_creation() {
        let recorder = SessionRecorder::new();
        assert!(!recorder.session_id().is_empty());
    }

    #[test]
    fn test_recorder_record_task() {
        let mut recorder = SessionRecorder::new();
        let task = TaskRecord::new("test".to_string()).success();
        
        recorder.record_task(task);
        
        let stats = recorder.session_stats();
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.successful_tasks, 1);
    }

    #[test]
    fn test_recorder_new_session() {
        let mut recorder = SessionRecorder::new();
        let old_id = recorder.session_id().to_string();
        
        recorder.record_task(TaskRecord::new("test".to_string()).success());
        
        let old_session = recorder.new_session();
        assert_eq!(old_session.total_tasks, 1);
        assert!(old_session.end_time.is_some());
        assert_ne!(recorder.session_id(), old_id);
    }
}
