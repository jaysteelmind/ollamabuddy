//! Telemetry system for OllamaBuddy
//! 
//! Provides real-time monitoring, event collection, and terminal display.

use std::time::Instant;
use std::sync::{Arc, Mutex};

/// Telemetry event types
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    // Agent events
    StateTransition {
        from: String,
        to: String,
        timestamp: Instant,
    },
    TokenReceived {
        token: String,
        timestamp: Instant,
    },
    ContextCompression {
        before_tokens: usize,
        after_tokens: usize,
        timestamp: Instant,
    },
    
    // Tool events
    ToolStarted {
        tool: String,
        timestamp: Instant,
    },
    ToolCompleted {
        tool: String,
        duration_ms: u64,
        success: bool,
        timestamp: Instant,
    },
    RetryAttempt {
        tool: String,
        attempt: u32,
        timestamp: Instant,
    },
    ParallelDispatch {
        tool_count: usize,
        timestamp: Instant,
    },
}

/// Telemetry statistics
#[derive(Debug, Clone, Default)]
pub struct TelemetryStats {
    pub tokens_processed: usize,
    pub tools_executed: usize,
    pub tools_succeeded: usize,
    pub tools_failed: usize,
    pub retry_attempts: usize,
    pub compression_events: usize,
    pub state_transitions: usize,
    pub parallel_dispatches: usize,
}

/// Telemetry collector
#[derive(Clone)]
pub struct TelemetryCollector {
    events: Arc<Mutex<Vec<TelemetryEvent>>>,
    stats: Arc<Mutex<TelemetryStats>>,
    start_time: Instant,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(TelemetryStats::default())),
            start_time: Instant::now(),
        }
    }

    /// Record an event
    pub fn record(&self, event: TelemetryEvent) {
        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            match &event {
                TelemetryEvent::StateTransition { .. } => {
                    stats.state_transitions += 1;
                }
                TelemetryEvent::TokenReceived { .. } => {
                    stats.tokens_processed += 1;
                }
                TelemetryEvent::ContextCompression { .. } => {
                    stats.compression_events += 1;
                }
                TelemetryEvent::ToolStarted { .. } => {
                    stats.tools_executed += 1;
                }
                TelemetryEvent::ToolCompleted { success, .. } => {
                    if *success {
                        stats.tools_succeeded += 1;
                    } else {
                        stats.tools_failed += 1;
                    }
                }
                TelemetryEvent::RetryAttempt { .. } => {
                    stats.retry_attempts += 1;
                }
                TelemetryEvent::ParallelDispatch { .. } => {
                    stats.parallel_dispatches += 1;
                }
            }
        }

        // Store event
        let mut events = self.events.lock().unwrap();
        events.push(event);
    }

    /// Get current statistics
    pub fn get_stats(&self) -> TelemetryStats {
        self.stats.lock().unwrap().clone()
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Get recent events (last n)
    pub fn recent_events(&self, n: usize) -> Vec<TelemetryEvent> {
        let events = self.events.lock().unwrap();
        let start = events.len().saturating_sub(n);
        events[start..].to_vec()
    }

    /// Calculate tool success rate
    pub fn tool_success_rate(&self) -> f64 {
        let stats = self.stats.lock().unwrap();
        let total = stats.tools_succeeded + stats.tools_failed;
        if total == 0 {
            1.0
        } else {
            stats.tools_succeeded as f64 / total as f64
        }
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple telemetry display
pub struct TelemetryDisplay {
    collector: TelemetryCollector,
    verbosity: crate::cli::Verbosity,
}

impl TelemetryDisplay {
    /// Create a new display
    pub fn new(collector: TelemetryCollector, verbosity: crate::cli::Verbosity) -> Self {
        Self {
            collector,
            verbosity,
        }
    }

    /// Display summary statistics
    pub fn display_summary(&self) {
        let stats = self.collector.get_stats();
        let elapsed = self.collector.elapsed();

        println!("
📊 Session Summary");
        println!("─────────────────────────────────────");
        println!("Duration:          {:?}", elapsed);
        println!("Tokens processed:  {}", stats.tokens_processed);
        println!("Tools executed:    {}", stats.tools_executed);
        println!("Success rate:      {:.1}%", self.collector.tool_success_rate() * 100.0);
        println!("Retries:           {}", stats.retry_attempts);
        println!("Compressions:      {}", stats.compression_events);
        println!();
    }

    /// Check if should show detailed output
    pub fn should_show_details(&self) -> bool {
        self.verbosity.show_events()
    }

    /// Check if should show tokens
    pub fn should_show_tokens(&self) -> bool {
        self.verbosity.show_tokens()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_creation() {
        let collector = TelemetryCollector::new();
        assert_eq!(collector.event_count(), 0);
        let stats = collector.get_stats();
        assert_eq!(stats.tokens_processed, 0);
    }

    #[test]
    fn test_record_token_event() {
        let collector = TelemetryCollector::new();
        collector.record(TelemetryEvent::TokenReceived {
            token: "test".to_string(),
            timestamp: Instant::now(),
        });
        
        let stats = collector.get_stats();
        assert_eq!(stats.tokens_processed, 1);
        assert_eq!(collector.event_count(), 1);
    }

    #[test]
    fn test_record_tool_events() {
        let collector = TelemetryCollector::new();
        
        collector.record(TelemetryEvent::ToolStarted {
            tool: "test".to_string(),
            timestamp: Instant::now(),
        });
        
        collector.record(TelemetryEvent::ToolCompleted {
            tool: "test".to_string(),
            duration_ms: 100,
            success: true,
            timestamp: Instant::now(),
        });
        
        let stats = collector.get_stats();
        assert_eq!(stats.tools_executed, 1);
        assert_eq!(stats.tools_succeeded, 1);
        assert_eq!(stats.tools_failed, 0);
    }

    #[test]
    fn test_tool_success_rate() {
        let collector = TelemetryCollector::new();
        
        // 2 successes
        collector.record(TelemetryEvent::ToolCompleted {
            tool: "test1".to_string(),
            duration_ms: 100,
            success: true,
            timestamp: Instant::now(),
        });
        collector.record(TelemetryEvent::ToolCompleted {
            tool: "test2".to_string(),
            duration_ms: 100,
            success: true,
            timestamp: Instant::now(),
        });
        
        // 1 failure
        collector.record(TelemetryEvent::ToolCompleted {
            tool: "test3".to_string(),
            duration_ms: 100,
            success: false,
            timestamp: Instant::now(),
        });
        
        let rate = collector.tool_success_rate();
        assert!((rate - 0.666).abs() < 0.01); // 2/3 = 0.666...
    }

    #[test]
    fn test_recent_events() {
        let collector = TelemetryCollector::new();
        
        for i in 0..10 {
            collector.record(TelemetryEvent::TokenReceived {
                token: format!("token{}", i),
                timestamp: Instant::now(),
            });
        }
        
        let recent = collector.recent_events(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_elapsed_time() {
        let collector = TelemetryCollector::new();
        let elapsed = collector.elapsed();
        assert!(elapsed.as_millis() < 100); // Should be very quick
    }

    #[test]
    fn test_compression_event() {
        let collector = TelemetryCollector::new();
        collector.record(TelemetryEvent::ContextCompression {
            before_tokens: 6000,
            after_tokens: 4000,
            timestamp: Instant::now(),
        });
        
        let stats = collector.get_stats();
        assert_eq!(stats.compression_events, 1);
    }
}
