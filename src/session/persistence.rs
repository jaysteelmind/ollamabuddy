// PRD 11 Phase 3: Session Persistence for disk storage
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::session::recording::{SessionData, TaskRecord};
use crate::session::statistics::CumulativeStats;

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Base directory for session storage
    pub storage_dir: PathBuf,
    /// Maximum sessions to keep
    pub max_sessions: usize,
    /// Auto-save enabled
    pub auto_save: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        let storage_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ollamabuddy")
            .join("sessions");

        Self {
            storage_dir,
            max_sessions: 100,
            auto_save: true,
        }
    }
}

/// Session persistence manager
pub struct SessionPersistence {
    config: PersistenceConfig,
}

impl SessionPersistence {
    /// Create new persistence manager
    pub fn new(config: PersistenceConfig) -> Result<Self> {
        // Create storage directory if it doesn't exist
        if !config.storage_dir.exists() {
            fs::create_dir_all(&config.storage_dir)
                .context("Failed to create session storage directory")?;
        }

        Ok(Self { config })
    }

    /// Create with default configuration
    pub fn default_config() -> Result<Self> {
        Self::new(PersistenceConfig::default())
    }

    /// Save session to disk
    pub fn save_session(&self, session: &SessionData) -> Result<PathBuf> {
        let filename = format!("session_{}.json", session.id);
        let path = self.config.storage_dir.join(&filename);

        let json = serde_json::to_string_pretty(session)
            .context("Failed to serialize session")?;

        fs::write(&path, json)
            .context("Failed to write session file")?;

        // Cleanup old sessions if needed
        self.cleanup_old_sessions()?;

        Ok(path)
    }

    /// Load session from disk
    pub fn load_session(&self, session_id: &str) -> Result<SessionData> {
        let filename = format!("session_{}.json", session_id);
        let path = self.config.storage_dir.join(&filename);

        let json = fs::read_to_string(&path)
            .context("Failed to read session file")?;

        let session: SessionData = serde_json::from_str(&json)
            .context("Failed to deserialize session")?;

        Ok(session)
    }

    /// List all saved session IDs
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        if !self.config.storage_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.config.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with("session_") && filename.ends_with(".json") {
                        let id = filename
                            .trim_start_matches("session_")
                            .trim_end_matches(".json");
                        sessions.push(id.to_string());
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Load all sessions
    pub fn load_all_sessions(&self) -> Result<Vec<SessionData>> {
        let ids = self.list_sessions()?;
        let mut sessions = Vec::new();

        for id in ids {
            match self.load_session(&id) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    eprintln!("Warning: Failed to load session {}: {}", id, e);
                }
            }
        }

        // Sort by start time
        sessions.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        Ok(sessions)
    }

    /// Delete session
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let filename = format!("session_{}.json", session_id);
        let path = self.config.storage_dir.join(&filename);

        if path.exists() {
            fs::remove_file(&path)
                .context("Failed to delete session file")?;
        }

        Ok(())
    }

    /// Cleanup old sessions (keep only max_sessions most recent)
    fn cleanup_old_sessions(&self) -> Result<()> {
        let mut sessions = self.load_all_sessions()?;

        if sessions.len() > self.config.max_sessions {
            // Sort by start time descending (newest first)
            sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

            // Delete oldest sessions
            for session in sessions.iter().skip(self.config.max_sessions) {
                let _ = self.delete_session(&session.id);
            }
        }

        Ok(())
    }

    /// Save cumulative statistics
    pub fn save_stats(&self, stats: &CumulativeStats) -> Result<PathBuf> {
        let path = self.config.storage_dir.join("cumulative_stats.json");

        let json = serde_json::to_string_pretty(stats)
            .context("Failed to serialize stats")?;

        fs::write(&path, json)
            .context("Failed to write stats file")?;

        Ok(path)
    }

    /// Load cumulative statistics
    pub fn load_stats(&self) -> Result<CumulativeStats> {
        let path = self.config.storage_dir.join("cumulative_stats.json");

        if !path.exists() {
            return Ok(CumulativeStats::default());
        }

        let json = fs::read_to_string(&path)
            .context("Failed to read stats file")?;

        let stats: CumulativeStats = serde_json::from_str(&json)
            .context("Failed to deserialize stats")?;

        Ok(stats)
    }

    /// Get storage directory
    pub fn storage_dir(&self) -> &PathBuf {
        &self.config.storage_dir
    }

    /// Get configuration
    pub fn config(&self) -> &PersistenceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::recording::TaskRecord;
    use tempfile::TempDir;

    fn create_test_persistence() -> (SessionPersistence, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistenceConfig {
            storage_dir: temp_dir.path().to_path_buf(),
            max_sessions: 10,
            auto_save: true,
        };
        let persistence = SessionPersistence::new(config).unwrap();
        (persistence, temp_dir)
    }

    fn create_test_session() -> SessionData {
        let mut session = SessionData::new();
        session.add_task(TaskRecord::new("test task".to_string()).success());
        session
    }

    #[test]
    fn test_persistence_creation() {
        let (persistence, _temp) = create_test_persistence();
        assert!(persistence.storage_dir().exists());
    }

    #[test]
    fn test_save_and_load_session() {
        let (persistence, _temp) = create_test_persistence();
        let session = create_test_session();
        let session_id = session.id.clone();

        persistence.save_session(&session).unwrap();
        let loaded = persistence.load_session(&session_id).unwrap();

        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.total_tasks, 1);
    }

    #[test]
    fn test_list_sessions() {
        let (persistence, _temp) = create_test_persistence();

        persistence.save_session(&create_test_session()).unwrap();
        persistence.save_session(&create_test_session()).unwrap();

        let sessions = persistence.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let (persistence, _temp) = create_test_persistence();
        let session = create_test_session();
        let session_id = session.id.clone();

        persistence.save_session(&session).unwrap();
        assert_eq!(persistence.list_sessions().unwrap().len(), 1);

        persistence.delete_session(&session_id).unwrap();
        assert_eq!(persistence.list_sessions().unwrap().len(), 0);
    }

    #[test]
    fn test_save_and_load_stats() {
        let (persistence, _temp) = create_test_persistence();
        
        let stats = CumulativeStats {
            total_sessions: 5,
            total_tasks: 20,
            successful_tasks: 18,
            ..Default::default()
        };

        persistence.save_stats(&stats).unwrap();
        let loaded = persistence.load_stats().unwrap();

        assert_eq!(loaded.total_sessions, 5);
        assert_eq!(loaded.total_tasks, 20);
    }
}
