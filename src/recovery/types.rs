//! Recovery system type definitions

use std::time::SystemTime;

/// Failure symptoms detected during execution
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FailureSymptom {
    /// Tool execution failed repeatedly
    ToolExecutionFailure {
        tool_name: String,
        consecutive_failures: usize,
    },
    
    /// Validation failed
    ValidationFailure {
        score: u8,
        threshold: u8,
    },
    
    /// Progress stagnated
    StagnationFailure {
        iterations_stagnant: usize,
    },
    
    /// Budget exhausted without completion
    BudgetExhaustion {
        used: usize,
        allocated: usize,
    },
    
    /// Timeout occurred
    Timeout {
        operation: String,
    },
    
    /// Unknown failure
    Unknown,
}

impl FailureSymptom {
    /// Get symptom severity (0-10)
    pub fn severity(&self) -> u8 {
        match self {
            FailureSymptom::BudgetExhaustion { .. } => 9,
            FailureSymptom::ValidationFailure { .. } => 7,
            FailureSymptom::StagnationFailure { .. } => 6,
            FailureSymptom::ToolExecutionFailure { consecutive_failures, .. } => {
                (*consecutive_failures as u8).min(8)
            }
            FailureSymptom::Timeout { .. } => 5,
            FailureSymptom::Unknown => 3,
        }
    }
    
    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            FailureSymptom::ToolExecutionFailure { tool_name, consecutive_failures } => {
                format!("Tool '{}' failed {} times consecutively", tool_name, consecutive_failures)
            }
            FailureSymptom::ValidationFailure { score, threshold } => {
                format!("Validation score {}% below threshold {}%", score, threshold)
            }
            FailureSymptom::StagnationFailure { iterations_stagnant } => {
                format!("No progress for {} iterations", iterations_stagnant)
            }
            FailureSymptom::BudgetExhaustion { used, allocated } => {
                format!("Budget exhausted: {}/{} iterations used", used, allocated)
            }
            FailureSymptom::Timeout { operation } => {
                format!("Timeout during: {}", operation)
            }
            FailureSymptom::Unknown => "Unknown failure".to_string(),
        }
    }
}

/// Failure pattern with tracking
#[derive(Debug, Clone)]
pub struct FailurePattern {
    /// Symptom detected
    pub symptom: FailureSymptom,
    
    /// Number of times this pattern occurred
    pub frequency: usize,
    
    /// Last time this pattern was seen
    pub last_seen: SystemTime,
    
    /// First time this pattern was seen
    pub first_seen: SystemTime,
}

impl FailurePattern {
    /// Create new failure pattern
    pub fn new(symptom: FailureSymptom) -> Self {
        let now = SystemTime::now();
        Self {
            symptom,
            frequency: 1,
            last_seen: now,
            first_seen: now,
        }
    }
    
    /// Update pattern occurrence
    pub fn update(&mut self) {
        self.frequency += 1;
        self.last_seen = SystemTime::now();
    }
    
    /// Check if pattern is recent (within last 5 minutes)
    pub fn is_recent(&self) -> bool {
        if let Ok(duration) = self.last_seen.elapsed() {
            duration.as_secs() < 300
        } else {
            false
        }
    }
}

/// Recovery actions available
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Rotate to next strategy
    RotateStrategy,
    
    /// Reduce parallelism level
    ReduceParallelism {
        from: usize,
        to: usize,
    },
    
    /// Increase validation threshold temporarily
    RelaxValidation {
        new_threshold: u8,
    },
    
    /// Request complexity reassessment
    ReassessComplexity,
    
    /// Retry with exponential backoff
    RetryWithBackoff {
        attempt: usize,
        delay_ms: u64,
    },
    
    /// Switch to simpler approach
    SimplifyApproach,
    
    /// Abort execution
    Abort {
        reason: String,
    },
}

impl RecoveryAction {
    /// Get action priority (higher = more urgent)
    pub fn priority(&self) -> u8 {
        match self {
            RecoveryAction::Abort { .. } => 10,
            RecoveryAction::ReassessComplexity => 8,
            RecoveryAction::RotateStrategy => 7,
            RecoveryAction::ReduceParallelism { .. } => 6,
            RecoveryAction::SimplifyApproach => 5,
            RecoveryAction::RelaxValidation { .. } => 4,
            RecoveryAction::RetryWithBackoff { .. } => 3,
        }
    }
}

/// Recovery strategy for execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryStrategy {
    /// Direct approach (original from planning)
    Direct,
    
    /// Exploratory approach
    Exploratory,
    
    /// Systematic approach
    Systematic,
}

impl RecoveryStrategy {
    /// Get all strategies in rotation order
    pub fn rotation_order() -> Vec<RecoveryStrategy> {
        vec![
            RecoveryStrategy::Direct,
            RecoveryStrategy::Exploratory,
            RecoveryStrategy::Systematic,
        ]
    }
    
    /// Get next strategy in rotation
    pub fn next(&self) -> RecoveryStrategy {
        match self {
            RecoveryStrategy::Direct => RecoveryStrategy::Exploratory,
            RecoveryStrategy::Exploratory => RecoveryStrategy::Systematic,
            RecoveryStrategy::Systematic => RecoveryStrategy::Direct,
        }
    }
    
    /// Get strategy name
    pub fn name(&self) -> &str {
        match self {
            RecoveryStrategy::Direct => "Direct",
            RecoveryStrategy::Exploratory => "Exploratory",
            RecoveryStrategy::Systematic => "Systematic",
        }
    }
}
