//! Convergence detection and progress analysis
//! Provides velocity calculation and stagnation detection with mathematical guarantees

use crate::analysis::types::{
    ProgressMetrics, VelocityMetric, StagnationResult, ConvergencePrediction, TerminationCondition,
};
use std::collections::VecDeque;

/// Convergence detector configuration
#[derive(Debug, Clone)]
pub struct ConvergenceConfig {
    /// Velocity threshold for stagnation detection
    pub velocity_threshold: f64,
    
    /// Minimum iterations before stagnation check
    pub min_iterations: usize,
    
    /// Progress threshold for success (0.0 to 1.0)
    pub success_threshold: f64,
    
    /// Validation score threshold for success
    pub validation_threshold: f64,
    
    /// Window size for velocity calculation
    pub velocity_window: usize,
    
    /// Maximum history size
    pub max_history: usize,
}

impl Default for ConvergenceConfig {
    fn default() -> Self {
        Self {
            velocity_threshold: 0.05,
            min_iterations: 3,
            success_threshold: 0.95,
            validation_threshold: 0.85,
            velocity_window: 3,
            max_history: 50,
        }
    }
}

/// Convergence detector for progress analysis
pub struct ConvergenceDetector {
    /// Configuration
    config: ConvergenceConfig,
    
    /// Progress history
    history: VecDeque<ProgressMetrics>,
    
    /// Latest velocity metric
    last_velocity: Option<VelocityMetric>,
    
    /// Stagnation counter
    stagnation_count: usize,
}

impl ConvergenceDetector {
    /// Create new convergence detector with default configuration
    pub fn new() -> Self {
        Self::with_config(ConvergenceConfig::default())
    }
    
    /// Create convergence detector with custom configuration
    pub fn with_config(config: ConvergenceConfig) -> Self {
        Self {
            config,
            history: VecDeque::new(),
            last_velocity: None,
            stagnation_count: 0,
        }
    }
    
    /// Record progress update
    pub fn record_progress(&mut self, progress: f64, iteration: usize) {
        let metrics = ProgressMetrics::new(progress, iteration);
        
        // Add to history
        self.history.push_back(metrics);
        
        // Maintain bounded history
        if self.history.len() > self.config.max_history {
            self.history.pop_front();
        }
        
        // Calculate velocity if enough data
        if self.history.len() >= 2 {
            self.calculate_velocity();
        }
    }
    
    /// Calculate current velocity
    fn calculate_velocity(&mut self) {
        if self.history.len() < 2 {
            return;
        }
        
        // Get window for velocity calculation
        let window_size = self.config.velocity_window.min(self.history.len());
        let start_idx = self.history.len() - window_size;
        
        let start_metrics = &self.history[start_idx];
        let end_metrics = self.history.back().unwrap();
        
        let velocity = VelocityMetric::calculate(
            start_metrics.progress,
            end_metrics.progress,
            start_metrics.iteration,
            end_metrics.iteration,
        );
        
        self.last_velocity = Some(velocity);
    }
    
    /// Get current velocity
    pub fn get_velocity(&self) -> Option<&VelocityMetric> {
        self.last_velocity.as_ref()
    }
    
    /// Detect stagnation
    pub fn detect_stagnation(&mut self) -> StagnationResult {
        // Check minimum iterations
        if self.history.len() < self.config.min_iterations {
            return StagnationResult::InsufficientData {
                iterations_needed: self.config.min_iterations - self.history.len(),
            };
        }
        
        // Get velocity
        let velocity = match &self.last_velocity {
            Some(v) => v,
            None => {
                return StagnationResult::InsufficientData {
                    iterations_needed: 1,
                };
            }
        };
        
        // Check if stagnant
        if velocity.is_stagnant(self.config.velocity_threshold) {
            self.stagnation_count += 1;
            
            StagnationResult::Stagnant {
                velocity: velocity.velocity,
                iterations_stagnant: self.stagnation_count,
                threshold: self.config.velocity_threshold,
            }
        } else {
            self.stagnation_count = 0;
            
            StagnationResult::Active {
                velocity: velocity.velocity,
                iterations_observed: self.history.len(),
            }
        }
    }
    
    /// Predict convergence
    pub fn predict_convergence(&self) -> Option<ConvergencePrediction> {
        if self.history.is_empty() {
            return None;
        }
        
        let current = self.history.back().unwrap();
        
        // Calculate average velocity
        let avg_velocity = if self.history.len() >= 2 {
            let total_delta: f64 = self.history
                .iter()
                .zip(self.history.iter().skip(1))
                .map(|(prev, curr)| curr.progress - prev.progress)
                .sum();
            
            let iterations = (self.history.len() - 1) as f64;
            total_delta / iterations
        } else {
            0.0
        };
        
        Some(ConvergencePrediction::new(
            current.progress,
            avg_velocity,
            self.history.len(),
        ))
    }
    
    /// Check early termination conditions
    pub fn check_termination(
        &self,
        progress: f64,
        validation_score: f64,
        iterations_used: usize,
        budget: usize,
    ) -> TerminationCondition {
        // Success condition: high progress + high validation score
        if progress >= self.config.success_threshold
            && validation_score >= self.config.validation_threshold
        {
            return TerminationCondition::Success;
        }
        
        // Budget exhausted
        if iterations_used >= budget {
            return TerminationCondition::BudgetExhausted;
        }
        
        // Stagnation condition
        if let Some(velocity) = &self.last_velocity {
            if velocity.velocity < 0.01 && iterations_used > 8 {
                return TerminationCondition::Stagnation;
            }
        }
        
        TerminationCondition::None
    }
    
    /// Get current progress
    pub fn get_current_progress(&self) -> Option<f64> {
        self.history.back().map(|m| m.progress)
    }
    
    /// Get progress history
    pub fn get_history(&self) -> &VecDeque<ProgressMetrics> {
        &self.history
    }
    
    /// Get stagnation count
    pub fn get_stagnation_count(&self) -> usize {
        self.stagnation_count
    }
    
    /// Reset detector state
    pub fn reset(&mut self) {
        self.history.clear();
        self.last_velocity = None;
        self.stagnation_count = 0;
    }
    
    /// Get configuration
    pub fn config(&self) -> &ConvergenceConfig {
        &self.config
    }
}

impl Default for ConvergenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detector_creation() {
        let detector = ConvergenceDetector::new();
        assert_eq!(detector.config().velocity_threshold, 0.05);
        assert_eq!(detector.get_stagnation_count(), 0);
    }
    
    #[test]
    fn test_record_progress() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.0, 1);
        assert_eq!(detector.get_current_progress(), Some(0.0));
        
        detector.record_progress(0.2, 2);
        assert_eq!(detector.get_current_progress(), Some(0.2));
    }
    
    #[test]
    fn test_velocity_calculation() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.0, 1);
        detector.record_progress(0.3, 2);
        detector.record_progress(0.6, 3);
        
        let velocity = detector.get_velocity();
        assert!(velocity.is_some(), "Velocity should be calculated");
        
        let v = velocity.unwrap();
        assert!(v.velocity > 0.0, "Velocity should be positive");
    }
    
    #[test]
    fn test_stagnation_detection_insufficient_data() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.0, 1);
        
        let result = detector.detect_stagnation();
        assert!(matches!(result, StagnationResult::InsufficientData { .. }));
    }
    
    #[test]
    fn test_stagnation_detection_active() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.0, 1);
        detector.record_progress(0.3, 2);
        detector.record_progress(0.6, 3);
        
        let result = detector.detect_stagnation();
        assert!(result.is_active(), "Should detect active progress");
    }
    
    #[test]
    fn test_stagnation_detection_stagnant() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.5, 1);
        detector.record_progress(0.5, 2);
        detector.record_progress(0.5, 3);
        detector.record_progress(0.5, 4);
        
        let result = detector.detect_stagnation();
        assert!(result.is_stagnant(), "Should detect stagnation");
    }
    
    #[test]
    fn test_convergence_prediction() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.2, 1);
        detector.record_progress(0.4, 2);
        detector.record_progress(0.6, 3);
        
        let prediction = detector.predict_convergence();
        assert!(prediction.is_some(), "Should predict convergence");
        
        let pred = prediction.unwrap();
        assert!(pred.average_velocity > 0.0, "Should have positive velocity");
    }
    
    #[test]
    fn test_termination_success() {
        let detector = ConvergenceDetector::new();
        
        let condition = detector.check_termination(0.96, 0.90, 5, 20);
        assert_eq!(condition, TerminationCondition::Success);
    }
    
    #[test]
    fn test_termination_budget_exhausted() {
        let detector = ConvergenceDetector::new();
        
        let condition = detector.check_termination(0.5, 0.7, 20, 20);
        assert_eq!(condition, TerminationCondition::BudgetExhausted);
    }
    
    #[test]
    fn test_termination_none() {
        let detector = ConvergenceDetector::new();
        
        let condition = detector.check_termination(0.5, 0.7, 5, 20);
        assert_eq!(condition, TerminationCondition::None);
    }
    
    #[test]
    fn test_reset() {
        let mut detector = ConvergenceDetector::new();
        
        detector.record_progress(0.3, 1);
        detector.record_progress(0.6, 2);
        
        assert!(!detector.history.is_empty());
        
        detector.reset();
        
        assert!(detector.history.is_empty());
        assert_eq!(detector.get_stagnation_count(), 0);
    }
    
    #[test]
    fn test_bounded_history() {
        let config = ConvergenceConfig {
            max_history: 5,
            ..Default::default()
        };
        
        let mut detector = ConvergenceDetector::with_config(config);
        
        for i in 1..=10 {
            detector.record_progress(i as f64 / 10.0, i);
        }
        
        assert_eq!(detector.history.len(), 5, "History should be bounded to 5");
    }
}
