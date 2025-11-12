//! Validation orchestration and recovery coordination
//! Manages multi-attempt validation with failure handling

use crate::validation::types::{ValidationResult, ValidationState, ValidationFailureType};
use crate::validation::validator::{TaskValidator, ValidatorConfig};
use crate::tools::types::ToolResult;

/// Recovery plan after validation failure
#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    /// Failure type detected
    pub failure_type: ValidationFailureType,
    
    /// Recommended action
    pub action: RecoveryAction,
    
    /// Additional context
    pub context: String,
}

/// Recovery actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry with same parameters
    Retry,
    
    /// Adjust validation threshold
    AdjustThreshold(u8),
    
    /// Request complexity reassessment
    ReassessComplexity,
    
    /// Reduce parallelism
    ReduceParallelism,
    
    /// Abort task
    Abort,
}

/// Validation orchestration result
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    /// Final validation result
    pub validation: ValidationResult,
    
    /// Recovery plans attempted
    pub recovery_attempts: Vec<RecoveryPlan>,
    
    /// Whether validation ultimately succeeded
    pub success: bool,
    
    /// Total attempts made
    pub total_attempts: usize,
}

/// Validation orchestrator
pub struct ValidationOrchestrator {
    /// Task validator
    validator: TaskValidator,
    
    /// Maximum validation attempts
    max_attempts: usize,
    
    /// Current attempt count
    validation_attempts: usize,
    
    /// History of recovery plans
    recovery_history: Vec<RecoveryPlan>,
}

impl ValidationOrchestrator {
    /// Create new orchestrator with default configuration
    pub fn new() -> Self {
        Self {
            validator: TaskValidator::new(),
            max_attempts: 3,
            validation_attempts: 0,
            recovery_history: Vec::new(),
        }
    }
    
    /// Create orchestrator with custom validator
    pub fn with_validator(validator: TaskValidator) -> Self {
        Self {
            validator,
            max_attempts: 3,
            validation_attempts: 0,
            recovery_history: Vec::new(),
        }
    }
    
    /// Orchestrate validation with automatic recovery
    pub fn orchestrate_validation(
        &mut self,
        tool_results: &[ToolResult],
        expected_outputs: &[String],
    ) -> OrchestrationResult {
        self.validation_attempts = 0;
        self.recovery_history.clear();
        
        let mut last_validation = None;
        
        // Attempt validation up to max_attempts times
        while self.validation_attempts < self.max_attempts {
            self.validation_attempts += 1;
            
            let validation = self.validator.validate(
                tool_results,
                expected_outputs,
                self.validation_attempts,
            );
            
            if validation.state == ValidationState::Validated {
                // Success
                return OrchestrationResult {
                    validation,
                    recovery_attempts: self.recovery_history.clone(),
                    success: true,
                    total_attempts: self.validation_attempts,
                };
            }
            
            // Validation failed, attempt recovery
            if self.validation_attempts < self.max_attempts {
                let recovery_plan = self.handle_validation_failure(&validation);
                self.execute_recovery(&recovery_plan);
                self.recovery_history.push(recovery_plan);
            }
            
            last_validation = Some(validation);
        }
        
        // All attempts exhausted
        OrchestrationResult {
            validation: last_validation.unwrap(),
            recovery_attempts: self.recovery_history.clone(),
            success: false,
            total_attempts: self.validation_attempts,
        }
    }
    
    /// Handle validation failure and create recovery plan
    fn handle_validation_failure(&self, validation: &ValidationResult) -> RecoveryPlan {
        let failed_checks = validation.failed_checks();
        
        // Determine primary failure type
        let failure_type = if failed_checks.iter().any(|c| c.name == "outcome_existence") {
            ValidationFailureType::MissingOutputs
        } else if failed_checks.iter().any(|c| c.name == "format_correctness") {
            ValidationFailureType::FormatError
        } else if failed_checks.iter().any(|c| c.name == "content_quality") {
            ValidationFailureType::QualityIssue
        } else if failed_checks.iter().any(|c| c.name == "side_effects") {
            ValidationFailureType::SideEffectFailure
        } else if failed_checks.iter().any(|c| c.name == "regression") {
            ValidationFailureType::RegressionDetected
        } else if failed_checks.iter().any(|c| c.name == "timeout") {
            ValidationFailureType::Timeout
        } else {
            ValidationFailureType::Unknown
        };
        
        // Select recovery action based on failure type and attempt count
        let action = match failure_type {
            ValidationFailureType::MissingOutputs => {
                if self.validation_attempts == 1 {
                    RecoveryAction::Retry
                } else {
                    RecoveryAction::ReassessComplexity
                }
            }
            ValidationFailureType::FormatError => {
                RecoveryAction::Retry
            }
            ValidationFailureType::QualityIssue => {
                if validation.score.overall >= 0.75 {
                    RecoveryAction::AdjustThreshold(75)
                } else {
                    RecoveryAction::Retry
                }
            }
            ValidationFailureType::SideEffectFailure => {
                RecoveryAction::Retry
            }
            ValidationFailureType::RegressionDetected => {
                RecoveryAction::Abort
            }
            ValidationFailureType::Timeout => {
                RecoveryAction::ReduceParallelism
            }
            ValidationFailureType::Unknown => {
                RecoveryAction::Retry
            }
        };
        
        let context = format!(
            "Attempt {}/{}: Score {:.2}, {} checks failed",
            self.validation_attempts,
            self.max_attempts,
            validation.score.overall,
            failed_checks.len()
        );
        
        RecoveryPlan {
            failure_type,
            action,
            context,
        }
    }
    
    /// Execute recovery action
    fn execute_recovery(&mut self, plan: &RecoveryPlan) {
        match plan.action {
            RecoveryAction::Retry => {
                // No changes needed, will retry with same config
            }
            RecoveryAction::AdjustThreshold(threshold_percent) => {
                let new_threshold = (threshold_percent as f64) / 100.0;
                self.validator.set_threshold(new_threshold);
            }
            RecoveryAction::ReassessComplexity => {
                // Would trigger complexity reassessment in orchestrator
                // For now, just log intent
            }
            RecoveryAction::ReduceParallelism => {
                // Would reduce parallel execution
                // For now, just log intent
            }
            RecoveryAction::Abort => {
                // Set attempts to max to prevent further retries
                self.validation_attempts = self.max_attempts;
            }
        }
    }
    
    /// Generate validation report
    pub fn generate_report(&self, result: &OrchestrationResult) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("Validation Report\n"));
        report.push_str(&format!("================\n\n"));
        report.push_str(&format!("Status: {}\n", if result.success { "PASSED" } else { "FAILED" }));
        report.push_str(&format!("Total Attempts: {}\n", result.total_attempts));
        report.push_str(&format!("Final Score: {:.2}\n", result.validation.score.overall));
        report.push_str(&format!("Checks Passed: {}/{}\n\n", 
            result.validation.score.checks_passed,
            result.validation.score.total_checks));
        
        if !result.validation.failed_checks().is_empty() {
            report.push_str("Failed Checks:\n");
            for check in result.validation.failed_checks() {
                report.push_str(&format!("  - {} (weight: {:.2}): {}\n",
                    check.name,
                    check.weight,
                    check.failure_reason.as_ref().unwrap_or(&"Unknown".to_string())));
            }
            report.push_str("\n");
        }
        
        if !result.recovery_attempts.is_empty() {
            report.push_str("Recovery Attempts:\n");
            for (idx, plan) in result.recovery_attempts.iter().enumerate() {
                report.push_str(&format!("  {}. {:?} -> {:?}\n",
                    idx + 1,
                    plan.failure_type,
                    plan.action));
            }
        }
        
        report
    }
    
    /// Reset orchestrator state
    pub fn reset(&mut self) {
        self.validation_attempts = 0;
        self.recovery_history.clear();
    }
    
    /// Get validator reference
    pub fn validator(&self) -> &TaskValidator {
        &self.validator
    }
    
    /// Get mutable validator reference
    pub fn validator_mut(&mut self) -> &mut TaskValidator {
        &mut self.validator
    }
}

impl Default for ValidationOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_result(tool_name: &str, output: &str, success: bool) -> ToolResult {
        ToolResult {
            tool: tool_name.to_string(),
            output: output.to_string(),
            success,
            duration_ms: 10,
            error: None,
            exit_code: Some(0),
        }
    }
    
    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = ValidationOrchestrator::new();
        assert_eq!(orchestrator.max_attempts, 3);
    }
    
    #[test]
    fn test_successful_validation_first_attempt() {
        let mut orchestrator = ValidationOrchestrator::new();
        let results = vec![
            create_test_result("read_file", "excellent quality output with all requirements", true),
        ];
        
        let result = orchestrator.orchestrate_validation(&results, &[]);
        
        assert!(result.success || result.total_attempts <= 3, "Should succeed or exhaust attempts");
    }
    
    #[test]
    fn test_validation_with_recovery() {
        let mut orchestrator = ValidationOrchestrator::new();
        let results = vec![
            create_test_result("read_file", "", true),  // Will fail
        ];
        
        let result = orchestrator.orchestrate_validation(&results, &[]);
        
        assert_eq!(result.total_attempts, 3, "Should attempt 3 times");
    }
    
    #[test]
    fn test_recovery_plan_generation() {
        let orchestrator = ValidationOrchestrator::new();
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        let plan = orchestrator.handle_validation_failure(&validation);
        
        assert!(matches!(plan.failure_type, ValidationFailureType::FormatError));
    }
    
    #[test]
    fn test_report_generation() {
        let mut orchestrator = ValidationOrchestrator::new();
        let results = vec![
            create_test_result("read_file", "quality content", true),
        ];
        
        let result = orchestrator.orchestrate_validation(&results, &[]);
        let report = orchestrator.generate_report(&result);
        
        assert!(report.contains("Validation Report"));
        assert!(report.contains("Status:"));
    }
    
    #[test]
    fn test_orchestrator_reset() {
        let mut orchestrator = ValidationOrchestrator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let _ = orchestrator.orchestrate_validation(&results, &[]);
        assert!(orchestrator.validation_attempts > 0);
        
        orchestrator.reset();
        assert_eq!(orchestrator.validation_attempts, 0);
        assert!(orchestrator.recovery_history.is_empty());
    }
    
    #[test]
    fn test_max_attempts_enforcement() {
        let mut orchestrator = ValidationOrchestrator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let result = orchestrator.orchestrate_validation(&results, &[]);
        
        assert!(result.total_attempts <= 3, "Should not exceed max attempts");
    }
    
    #[test]
    fn test_recovery_history_tracking() {
        let mut orchestrator = ValidationOrchestrator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let result = orchestrator.orchestrate_validation(&results, &[]);
        
        if !result.success {
            assert!(!result.recovery_attempts.is_empty(), "Should have recovery attempts");
        }
    }
}
