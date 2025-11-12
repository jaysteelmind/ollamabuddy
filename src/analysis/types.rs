//! Analysis system type definitions

use std::time::{Duration, Instant};

/// Progress metrics tracking
#[derive(Debug, Clone)]
pub struct ProgressMetrics {
    /// Current progress (0.0 to 1.0)
    pub progress: f64,
    
    /// Iteration number
    pub iteration: usize,
    
    /// Timestamp
    pub timestamp: Instant,
}

impl ProgressMetrics {
    /// Create new progress metrics
    pub fn new(progress: f64, iteration: usize) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            iteration,
            timestamp: Instant::now(),
        }
    }
    
    /// Get elapsed time since creation
    pub fn elapsed(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Velocity metric for progress rate
#[derive(Debug, Clone)]
pub struct VelocityMetric {
    /// Progress change (delta P)
    pub delta_progress: f64,
    
    /// Iteration change (delta t)
    pub delta_iterations: usize,
    
    /// Velocity (progress per iteration)
    pub velocity: f64,
    
    /// Time window start
    pub window_start: usize,
    
    /// Time window end
    pub window_end: usize,
}

impl VelocityMetric {
    /// Calculate velocity from progress change
    pub fn calculate(
        progress_start: f64,
        progress_end: f64,
        iteration_start: usize,
        iteration_end: usize,
    ) -> Self {
        let delta_progress = progress_end - progress_start;
        let delta_iterations = iteration_end.saturating_sub(iteration_start);
        
        let velocity = if delta_iterations > 0 {
            delta_progress / (delta_iterations as f64)
        } else {
            0.0
        };
        
        Self {
            delta_progress,
            delta_iterations,
            velocity,
            window_start: iteration_start,
            window_end: iteration_end,
        }
    }
    
    /// Check if velocity indicates stagnation
    pub fn is_stagnant(&self, threshold: f64) -> bool {
        self.velocity.abs() < threshold
    }
}

/// Stagnation detection result
#[derive(Debug, Clone, PartialEq)]
pub enum StagnationResult {
    /// Making progress
    Active {
        velocity: f64,
        iterations_observed: usize,
    },
    
    /// Stagnant (no progress)
    Stagnant {
        velocity: f64,
        iterations_stagnant: usize,
        threshold: f64,
    },
    
    /// Insufficient data
    InsufficientData {
        iterations_needed: usize,
    },
}

impl StagnationResult {
    /// Check if stagnant
    pub fn is_stagnant(&self) -> bool {
        matches!(self, StagnationResult::Stagnant { .. })
    }
    
    /// Check if active
    pub fn is_active(&self) -> bool {
        matches!(self, StagnationResult::Active { .. })
    }
}

/// Convergence prediction
#[derive(Debug, Clone)]
pub struct ConvergencePrediction {
    /// Current progress
    pub current_progress: f64,
    
    /// Average velocity
    pub average_velocity: f64,
    
    /// Predicted remaining iterations
    pub estimated_remaining: usize,
    
    /// Confidence in prediction (0.0 to 1.0)
    pub confidence: f64,
}

impl ConvergencePrediction {
    /// Create convergence prediction
    pub fn new(current_progress: f64, average_velocity: f64, observations: usize) -> Self {
        let remaining_progress = (1.0 - current_progress).max(0.0);
        
        let estimated_remaining = if average_velocity > 0.0 {
            (remaining_progress / average_velocity).ceil() as usize
        } else {
            usize::MAX
        };
        
        // Confidence increases with more observations and higher velocity
        let confidence = if observations >= 3 && average_velocity > 0.01 {
            (observations as f64 / 10.0).min(1.0) * (average_velocity * 10.0).min(1.0)
        } else {
            0.0
        };
        
        Self {
            current_progress,
            average_velocity,
            estimated_remaining,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
    
    /// Check if convergence is likely
    pub fn is_likely(&self) -> bool {
        self.confidence >= 0.5 && self.estimated_remaining < 100
    }
}

/// Early termination condition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationCondition {
    /// Task completed successfully
    Success,
    
    /// Progress stagnated
    Stagnation,
    
    /// Budget exhausted
    BudgetExhausted,
    
    /// No termination yet
    None,
}

impl TerminationCondition {
    /// Check if should terminate
    pub fn should_terminate(&self) -> bool {
        !matches!(self, TerminationCondition::None)
    }
}
