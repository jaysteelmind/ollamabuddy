//! Experience Tracker: Bayesian success/failure tracking
//!
//! Tracks tool and strategy effectiveness using Bayesian estimation.

use crate::memory::types::Recommendation;
use crate::tools::types::ToolResult;
use std::collections::HashMap;

/// Context signature for grouping similar situations
pub type ContextSignature = u64;

/// Tool execution record
#[derive(Debug, Clone)]
struct ToolRecord {
    /// Number of successful executions
    successes: usize,
    /// Number of failed executions
    failures: usize,
    /// Total duration in milliseconds
    total_duration_ms: u64,
}

impl ToolRecord {
    fn new() -> Self {
        Self {
            successes: 0,
            failures: 0,
            total_duration_ms: 0,
        }
    }

    /// Compute Bayesian success rate
    /// Prior: Beta(1, 1) uniform distribution
    /// Posterior: Beta(1 + successes, 1 + failures)
    fn success_rate(&self) -> f64 {
        let alpha = 1.0 + self.successes as f64;
        let beta = 1.0 + self.failures as f64;
        alpha / (alpha + beta)
    }

    /// Get sample size
    fn sample_size(&self) -> usize {
        self.successes + self.failures
    }

    /// Compute confidence score based on sample size
    fn confidence(&self) -> f64 {
        let n = self.sample_size() as f64;
        // Confidence increases with sample size, asymptotically approaches 1.0
        // Formula: 1 - exp(-n/10)
        1.0 - (-n / 10.0).exp()
    }
}

/// Experience tracker for learning
pub struct ExperienceTracker {
    /// Tool records per context
    /// Key: (tool_name, context_signature)
    tool_records: HashMap<(String, ContextSignature), ToolRecord>,
    
    /// Strategy effectiveness records
    /// Key: (strategy_name, complexity_bucket)
    strategy_records: HashMap<(String, u8), ToolRecord>,
}

impl ExperienceTracker {
    /// Create a new experience tracker
    pub fn new() -> Self {
        Self {
            tool_records: HashMap::new(),
            strategy_records: HashMap::new(),
        }
    }

    /// Record a tool execution result
    pub fn record_tool_execution(
        &mut self,
        tool: &str,
        context: ContextSignature,
        result: &ToolResult,
    ) {
        let key = (tool.to_string(), context);
        let record = self.tool_records.entry(key).or_insert_with(ToolRecord::new);

        if result.success {
            record.successes += 1;
        } else {
            record.failures += 1;
        }

        record.total_duration_ms += result.duration_ms;
    }

    /// Record strategy effectiveness
    pub fn record_strategy(
        &mut self,
        strategy: &str,
        complexity: f64,
        success: bool,
        duration_ms: u64,
    ) {
        let complexity_bucket = (complexity * 10.0).floor() as u8;
        let key = (strategy.to_string(), complexity_bucket);
        let record = self.strategy_records.entry(key).or_insert_with(ToolRecord::new);

        if success {
            record.successes += 1;
        } else {
            record.failures += 1;
        }

        record.total_duration_ms += duration_ms;
    }

    /// Recommend tools based on experience
    pub fn recommend_tools(
        &self,
        _goal: &str,
        context: ContextSignature,
        available_tools: &[String],
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        for tool in available_tools {
            let key = (tool.clone(), context);
            if let Some(record) = self.tool_records.get(&key) {
                // Only recommend if we have enough data
                if record.sample_size() >= 3 {
                    recommendations.push(Recommendation {
                        tool: tool.clone(),
                        confidence: record.confidence(),
                        success_rate: record.success_rate(),
                        sample_size: record.sample_size(),
                    });
                }
            }
        }

        // Sort by success rate * confidence
        recommendations.sort_by(|a, b| {
            let score_a = a.success_rate * a.confidence;
            let score_b = b.success_rate * b.confidence;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        recommendations
    }

    /// Get strategy effectiveness
    pub fn get_strategy_effectiveness(&self, strategy: &str, complexity: f64) -> Option<f64> {
        let complexity_bucket = (complexity * 10.0).floor() as u8;
        let key = (strategy.to_string(), complexity_bucket);
        self.strategy_records.get(&key).map(|r| r.success_rate())
    }

    /// Get total experience count
    pub fn total_experiences(&self) -> usize {
        self.tool_records.values().map(|r| r.sample_size()).sum()
    }
}

impl Default for ExperienceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayesian_success_rate() {
        let mut record = ToolRecord::new();
        
        // Initial: Beta(1, 1) -> E[p] = 0.5
        assert!((record.success_rate() - 0.5).abs() < 0.01);

        // After 1 success: Beta(2, 1) -> E[p] = 2/3
        record.successes = 1;
        assert!((record.success_rate() - 0.667).abs() < 0.01);

        // After 9 successes, 1 failure: Beta(10, 2) -> E[p] = 10/12
        record.successes = 9;
        record.failures = 1;
        assert!((record.success_rate() - 0.833).abs() < 0.01);
    }

    #[test]
    fn test_confidence_increases_with_samples() {
        let mut record = ToolRecord::new();
        
        let conf_0 = record.confidence();
        record.successes = 5;
        let conf_5 = record.confidence();
        record.successes = 10;
        let conf_10 = record.confidence();

        assert!(conf_5 > conf_0);
        assert!(conf_10 > conf_5);
        assert!(conf_10 < 1.0); // Never quite reaches 1.0
    }
}
