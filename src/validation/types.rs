//! Validation system type definitions

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Validation state machine states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationState {
    /// Not yet validated
    Unvalidated,
    
    /// Currently validating
    Validating,
    
    /// Validation passed
    Validated,
    
    /// Validation failed
    Failed,
}

/// Individual validation check
#[derive(Debug, Clone)]
pub struct ValidationCheck {
    /// Check name
    pub name: String,
    
    /// Check weight in scoring (0.0 to 1.0)
    pub weight: f64,
    
    /// Check passed
    pub passed: bool,
    
    /// Optional failure reason
    pub failure_reason: Option<String>,
    
    /// Check execution time
    pub execution_time_ms: u64,
}

impl ValidationCheck {
    /// Create new validation check
    pub fn new(name: String, weight: f64) -> Self {
        Self {
            name,
            weight,
            passed: false,
            failure_reason: None,
            execution_time_ms: 0,
        }
    }
    
    /// Mark check as passed
    pub fn pass(&mut self) {
        self.passed = true;
        self.failure_reason = None;
    }
    
    /// Mark check as failed with reason
    pub fn fail(&mut self, reason: String) {
        self.passed = false;
        self.failure_reason = Some(reason);
    }
}

/// Validation score calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationScore {
    /// Overall score (0.0 to 1.0)
    pub overall: f64,
    
    /// Number of checks passed
    pub checks_passed: usize,
    
    /// Total number of checks
    pub total_checks: usize,
    
    /// Pass threshold
    pub threshold: f64,
    
    /// Whether validation passed
    pub passed: bool,
}

impl ValidationScore {
    /// Create validation score from checks
    pub fn from_checks(checks: &[ValidationCheck], threshold: f64) -> Self {
        let total_checks = checks.len();
        let checks_passed = checks.iter().filter(|c| c.passed).count();
        
        // Calculate weighted score
        let overall = if total_checks > 0 {
            let total_weight: f64 = checks.iter().map(|c| c.weight).sum();
            let passed_weight: f64 = checks.iter()
                .filter(|c| c.passed)
                .map(|c| c.weight)
                .sum();
            
            if total_weight > 0.0 {
                passed_weight / total_weight
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        let passed = overall >= threshold;
        
        Self {
            overall,
            checks_passed,
            total_checks,
            threshold,
            passed,
        }
    }
    
    /// Get pass percentage
    pub fn pass_percentage(&self) -> f64 {
        if self.total_checks > 0 {
            (self.checks_passed as f64 / self.total_checks as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Complete validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Validation state
    pub state: ValidationState,
    
    /// Validation score
    pub score: ValidationScore,
    
    /// Individual checks
    pub checks: Vec<ValidationCheck>,
    
    /// Total validation time
    pub total_time_ms: u64,
    
    /// Timestamp
    pub timestamp: SystemTime,
    
    /// Validation attempt number
    pub attempt: usize,
}

impl ValidationResult {
    /// Create new validation result
    pub fn new(checks: Vec<ValidationCheck>, threshold: f64, attempt: usize) -> Self {
        let score = ValidationScore::from_checks(&checks, threshold);
        let state = if score.passed {
            ValidationState::Validated
        } else {
            ValidationState::Failed
        };
        
        let total_time_ms = checks.iter().map(|c| c.execution_time_ms).sum();
        
        Self {
            state,
            score,
            checks,
            total_time_ms,
            timestamp: SystemTime::now(),
            attempt,
        }
    }
    
    /// Get failed checks
    pub fn failed_checks(&self) -> Vec<&ValidationCheck> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }
    
    /// Get failure reasons
    pub fn failure_reasons(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter_map(|c| {
                if !c.passed {
                    c.failure_reason.clone()
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Validation failure types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationFailureType {
    /// Required outputs missing
    MissingOutputs,
    
    /// Output format incorrect
    FormatError,
    
    /// Content quality issues
    QualityIssue,
    
    /// Side effects not verified
    SideEffectFailure,
    
    /// Regression detected
    RegressionDetected,
    
    /// Timeout during validation
    Timeout,
    
    /// Unknown failure
    Unknown,
}

impl ValidationFailureType {
    /// Get human-readable description
    pub fn description(&self) -> &str {
        match self {
            Self::MissingOutputs => "Required outputs are missing",
            Self::FormatError => "Output format is incorrect",
            Self::QualityIssue => "Content quality does not meet standards",
            Self::SideEffectFailure => "Expected side effects not verified",
            Self::RegressionDetected => "Unintended side effects detected",
            Self::Timeout => "Validation exceeded time limit",
            Self::Unknown => "Unknown validation failure",
        }
    }
}
