// PRD 11 Phase 3: Learning System - Coordinating cross-session intelligence
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::session::persistence::{PersistenceConfig, SessionPersistence};
use crate::session::recording::{SessionData, SessionRecorder, TaskRecord};
use crate::session::statistics::{CumulativeStats, StatisticsTracker, ToolStats};

/// Learning system configuration
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Persistence configuration
    pub persistence: PersistenceConfig,
    /// Enable automatic session saving
    pub auto_save: bool,
    /// Enable statistics tracking
    pub track_statistics: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            persistence: PersistenceConfig::default(),
            auto_save: true,
            track_statistics: true,
        }
    }
}

/// Comprehensive learning system
pub struct LearningSystem {
    recorder: Arc<RwLock<SessionRecorder>>,
    tracker: Arc<RwLock<StatisticsTracker>>,
    persistence: Arc<SessionPersistence>,
    config: LearningConfig,
}

impl LearningSystem {
    /// Create new learning system
    pub fn new(config: LearningConfig) -> Result<Self> {
        let persistence = Arc::new(
            SessionPersistence::new(config.persistence.clone())
                .context("Failed to create session persistence")?
        );

        // Load existing cumulative stats
        let cumulative = persistence.load_stats().unwrap_or_default();
        let tracker = Arc::new(RwLock::new(StatisticsTracker::from_stats(cumulative)));

        Ok(Self {
            recorder: Arc::new(RwLock::new(SessionRecorder::new())),
            tracker,
            persistence,
            config,
        })
    }

    /// Create with default configuration
    pub fn default_config() -> Result<Self> {
        Self::new(LearningConfig::default())
    }

    /// Record a completed task
    pub async fn record_task(&self, task: TaskRecord) {
        let mut recorder = self.recorder.write().await;
        recorder.record_task(task);
    }

    /// Get current session ID
    pub async fn current_session_id(&self) -> String {
        let recorder = self.recorder.read().await;
        recorder.session_id().to_string()
    }

    /// Get current session statistics
    pub async fn current_session_stats(&self) -> crate::session::recording::SessionStats {
        let recorder = self.recorder.read().await;
        recorder.session_stats()
    }

    /// End current session and start new one
    pub async fn end_session(&self) -> Result<()> {
        let mut recorder = self.recorder.write().await;
        let completed_session = recorder.new_session();

        // Update statistics
        if self.config.track_statistics {
            let mut tracker = self.tracker.write().await;
            tracker.update_with_session(&completed_session);

            // Save updated statistics
            if self.config.auto_save {
                self.persistence.save_stats(tracker.cumulative())
                    .context("Failed to save cumulative statistics")?;
            }
        }

        // Save session to disk
        if self.config.auto_save {
            self.persistence.save_session(&completed_session)
                .context("Failed to save session")?;
        }

        Ok(())
    }

    /// Get cumulative statistics
    pub async fn cumulative_stats(&self) -> CumulativeStats {
        let tracker = self.tracker.read().await;
        tracker.cumulative().clone()
    }

    /// Get tool statistics
    pub async fn tool_stats(&self) -> Vec<ToolStats> {
        let tracker = self.tracker.read().await;
        tracker.tool_stats().values().cloned().collect()
    }

    /// Get most used tools
    pub async fn most_used_tools(&self, limit: usize) -> Vec<ToolStats> {
        let tracker = self.tracker.read().await;
        tracker.most_used_tools(limit)
    }

    /// Get best performing tools
    pub async fn best_performing_tools(&self, limit: usize, min_usage: usize) -> Vec<ToolStats> {
        let tracker = self.tracker.read().await;
        tracker.best_performing_tools(limit, min_usage)
    }

    /// Get success rate trend
    pub async fn success_rate_trend(&self, window: usize) -> f64 {
        let tracker = self.tracker.read().await;
        tracker.success_rate_trend(window)
    }

    /// Load historical session
    pub async fn load_session(&self, session_id: &str) -> Result<SessionData> {
        self.persistence.load_session(session_id)
            .context("Failed to load session")
    }

    /// List all saved sessions
    pub async fn list_sessions(&self) -> Result<Vec<String>> {
        self.persistence.list_sessions()
            .context("Failed to list sessions")
    }

    /// Load all historical sessions
    pub async fn load_all_sessions(&self) -> Result<Vec<SessionData>> {
        self.persistence.load_all_sessions()
            .context("Failed to load all sessions")
    }

    /// Rebuild statistics from all historical sessions
    pub async fn rebuild_statistics(&self) -> Result<()> {
        let sessions = self.load_all_sessions().await?;
        
        let mut tracker = StatisticsTracker::new();
        for session in sessions {
            tracker.update_with_session(&session);
        }

        // Replace current tracker
        let mut current_tracker = self.tracker.write().await;
        *current_tracker = tracker;

        // Save rebuilt stats
        if self.config.auto_save {
            self.persistence.save_stats(current_tracker.cumulative())
                .context("Failed to save rebuilt statistics")?;
        }

        Ok(())
    }

    /// Get persistence storage directory
    pub fn storage_dir(&self) -> &std::path::PathBuf {
        self.persistence.storage_dir()
    }

    /// Get configuration
    pub fn config(&self) -> &LearningConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_system() -> (LearningSystem, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = LearningConfig {
            persistence: PersistenceConfig {
                storage_dir: temp_dir.path().to_path_buf(),
                max_sessions: 10,
                auto_save: true,
            },
            auto_save: true,
            track_statistics: true,
        };
        let system = LearningSystem::new(config).unwrap();
        (system, temp_dir)
    }

    #[tokio::test]
    async fn test_system_creation() {
        let (system, _temp) = create_test_system().await;
        let session_id = system.current_session_id().await;
        assert!(!session_id.is_empty());
    }

    #[tokio::test]
    async fn test_record_task() {
        let (system, _temp) = create_test_system().await;
        
        let task = TaskRecord::new("test task".to_string())
            .success()
            .with_metrics(5, 10.0);
        
        system.record_task(task).await;
        
        let stats = system.current_session_stats().await;
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.successful_tasks, 1);
    }

    #[tokio::test]
    async fn test_end_session() {
        let (system, _temp) = create_test_system().await;
        
        let task = TaskRecord::new("test".to_string()).success();
        system.record_task(task).await;
        
        let old_id = system.current_session_id().await;
        system.end_session().await.unwrap();
        let new_id = system.current_session_id().await;
        
        assert_ne!(old_id, new_id);
    }

    #[tokio::test]
    async fn test_cumulative_stats() {
        let (system, _temp) = create_test_system().await;
        
        system.record_task(TaskRecord::new("1".to_string()).success()).await;
        system.end_session().await.unwrap();
        
        system.record_task(TaskRecord::new("2".to_string()).success()).await;
        system.end_session().await.unwrap();
        
        let stats = system.cumulative_stats().await;
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_tasks, 2);
    }

    #[tokio::test]
    async fn test_load_sessions() {
        let (system, _temp) = create_test_system().await;
        
        system.record_task(TaskRecord::new("test".to_string()).success()).await;
        system.end_session().await.unwrap();
        
        let sessions = system.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
        
        let loaded = system.load_all_sessions().await.unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[tokio::test]
    async fn test_rebuild_statistics() {
        let (system, _temp) = create_test_system().await;
        
        // Create some sessions
        for i in 0..3 {
            system.record_task(
                TaskRecord::new(format!("task_{}", i)).success()
            ).await;
            system.end_session().await.unwrap();
        }
        
        // Rebuild statistics
        system.rebuild_statistics().await.unwrap();
        
        let stats = system.cumulative_stats().await;
        assert_eq!(stats.total_sessions, 3);
        assert_eq!(stats.total_tasks, 3);
    }
}
