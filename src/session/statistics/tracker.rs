// PRD 11 Phase 3: Statistics Tracker for cumulative metrics
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::session::recording::{SessionData, TaskOutcome};

/// Cumulative statistics across all sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CumulativeStats {
    /// Total number of sessions
    pub total_sessions: usize,
    /// Total tasks across all sessions
    pub total_tasks: usize,
    /// Total successful tasks
    pub successful_tasks: usize,
    /// Total failed tasks
    pub failed_tasks: usize,
    /// Overall success rate
    pub success_rate: f64,
    /// Average tasks per session
    pub avg_tasks_per_session: f64,
    /// Total execution time (seconds)
    pub total_execution_time_secs: f64,
    /// Average task duration (seconds)
    pub avg_task_duration_secs: f64,
    /// First session timestamp
    pub first_session: Option<DateTime<Utc>>,
    /// Last session timestamp
    pub last_session: Option<DateTime<Utc>>,
}

impl Default for CumulativeStats {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            success_rate: 0.0,
            avg_tasks_per_session: 0.0,
            total_execution_time_secs: 0.0,
            avg_task_duration_secs: 0.0,
            first_session: None,
            last_session: None,
        }
    }
}

/// Tool usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    /// Tool name
    pub tool: String,
    /// Times used
    pub usage_count: usize,
    /// Success count
    pub success_count: usize,
    /// Failure count
    pub failure_count: usize,
    /// Success rate
    pub success_rate: f64,
}

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Statistics tracker for performance metrics
pub struct StatisticsTracker {
    cumulative: CumulativeStats,
    tool_stats: HashMap<String, ToolStats>,
    success_rate_history: Vec<TimeSeriesPoint>,
}

impl StatisticsTracker {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self {
            cumulative: CumulativeStats::default(),
            tool_stats: HashMap::new(),
            success_rate_history: Vec::new(),
        }
    }

    /// Load from existing cumulative stats
    pub fn from_stats(stats: CumulativeStats) -> Self {
        Self {
            cumulative: stats,
            tool_stats: HashMap::new(),
            success_rate_history: Vec::new(),
        }
    }

    /// Update statistics with a completed session
    pub fn update_with_session(&mut self, session: &SessionData) {
        // Update session count
        self.cumulative.total_sessions += 1;
        
        // Update task counts
        self.cumulative.total_tasks += session.total_tasks;
        self.cumulative.successful_tasks += session.successful_tasks;
        self.cumulative.failed_tasks += session.failed_tasks;
        
        // Update execution time
        for task in &session.tasks {
            self.cumulative.total_execution_time_secs += task.duration_secs;
            
            // Update tool statistics
            for tool in &task.tools_used {
                let stats = self.tool_stats.entry(tool.clone()).or_insert(ToolStats {
                    tool: tool.clone(),
                    usage_count: 0,
                    success_count: 0,
                    failure_count: 0,
                    success_rate: 0.0,
                });
                
                stats.usage_count += 1;
                match task.outcome {
                    TaskOutcome::Success => stats.success_count += 1,
                    TaskOutcome::Failure => stats.failure_count += 1,
                    _ => {}
                }
                stats.success_rate = if stats.usage_count > 0 {
                    stats.success_count as f64 / stats.usage_count as f64
                } else {
                    0.0
                };
            }
        }
        
        // Recalculate derived statistics
        self.cumulative.success_rate = if self.cumulative.total_tasks > 0 {
            self.cumulative.successful_tasks as f64 / self.cumulative.total_tasks as f64
        } else {
            0.0
        };
        
        self.cumulative.avg_tasks_per_session = if self.cumulative.total_sessions > 0 {
            self.cumulative.total_tasks as f64 / self.cumulative.total_sessions as f64
        } else {
            0.0
        };
        
        self.cumulative.avg_task_duration_secs = if self.cumulative.total_tasks > 0 {
            self.cumulative.total_execution_time_secs / self.cumulative.total_tasks as f64
        } else {
            0.0
        };
        
        // Update timestamps
        if self.cumulative.first_session.is_none() {
            self.cumulative.first_session = Some(session.start_time);
        }
        self.cumulative.last_session = Some(session.start_time);
        
        // Add to success rate history
        self.success_rate_history.push(TimeSeriesPoint {
            timestamp: session.start_time,
            value: session.success_rate(),
        });
    }

    /// Get cumulative statistics
    pub fn cumulative(&self) -> &CumulativeStats {
        &self.cumulative
    }

    /// Get tool statistics
    pub fn tool_stats(&self) -> &HashMap<String, ToolStats> {
        &self.tool_stats
    }

    /// Get success rate history
    pub fn success_rate_history(&self) -> &[TimeSeriesPoint] {
        &self.success_rate_history
    }

    /// Get most used tools (sorted by usage count)
    pub fn most_used_tools(&self, limit: usize) -> Vec<ToolStats> {
        let mut tools: Vec<_> = self.tool_stats.values().cloned().collect();
        tools.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        tools.truncate(limit);
        tools
    }

    /// Get best performing tools (sorted by success rate, min 5 uses)
    pub fn best_performing_tools(&self, limit: usize, min_usage: usize) -> Vec<ToolStats> {
        let mut tools: Vec<_> = self.tool_stats
            .values()
            .filter(|t| t.usage_count >= min_usage)
            .cloned()
            .collect();
        tools.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());
        tools.truncate(limit);
        tools
    }

    /// Calculate trend (positive = improving, negative = declining)
    pub fn success_rate_trend(&self, window: usize) -> f64 {
        if self.success_rate_history.len() < 2 {
            return 0.0;
        }

        let recent = self.success_rate_history
            .iter()
            .rev()
            .take(window)
            .map(|p| p.value)
            .collect::<Vec<_>>();

        if recent.len() < 2 {
            return 0.0;
        }

        let avg_recent: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
        let overall = self.cumulative.success_rate;

        avg_recent - overall
    }
}

impl Default for StatisticsTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::recording::TaskRecord;

    fn create_test_session(tasks: usize, success: usize) -> SessionData {
        let mut session = SessionData::new();
        for i in 0..tasks {
            let task = if i < success {
                TaskRecord::new(format!("task_{}", i)).success()
            } else {
                TaskRecord::new(format!("task_{}", i)).failure("error".to_string())
            };
            session.add_task(task);
        }
        session
    }

    #[test]
    fn test_tracker_creation() {
        let tracker = StatisticsTracker::new();
        assert_eq!(tracker.cumulative().total_sessions, 0);
        assert_eq!(tracker.cumulative().total_tasks, 0);
    }

    #[test]
    fn test_update_with_session() {
        let mut tracker = StatisticsTracker::new();
        let session = create_test_session(10, 8);
        
        tracker.update_with_session(&session);
        
        assert_eq!(tracker.cumulative().total_sessions, 1);
        assert_eq!(tracker.cumulative().total_tasks, 10);
        assert_eq!(tracker.cumulative().successful_tasks, 8);
        assert_eq!(tracker.cumulative().success_rate, 0.8);
    }

    #[test]
    fn test_multiple_sessions() {
        let mut tracker = StatisticsTracker::new();
        
        tracker.update_with_session(&create_test_session(10, 8));
        tracker.update_with_session(&create_test_session(5, 5));
        
        assert_eq!(tracker.cumulative().total_sessions, 2);
        assert_eq!(tracker.cumulative().total_tasks, 15);
        assert_eq!(tracker.cumulative().successful_tasks, 13);
    }

    #[test]
    fn test_avg_tasks_per_session() {
        let mut tracker = StatisticsTracker::new();
        
        tracker.update_with_session(&create_test_session(10, 8));
        tracker.update_with_session(&create_test_session(20, 15));
        
        assert_eq!(tracker.cumulative().avg_tasks_per_session, 15.0);
    }

    #[test]
    fn test_success_rate_history() {
        let mut tracker = StatisticsTracker::new();
        
        tracker.update_with_session(&create_test_session(10, 8));
        tracker.update_with_session(&create_test_session(10, 9));
        
        assert_eq!(tracker.success_rate_history().len(), 2);
        assert_eq!(tracker.success_rate_history()[0].value, 0.8);
        assert_eq!(tracker.success_rate_history()[1].value, 0.9);
    }
}
