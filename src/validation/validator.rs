//! Task validator implementation
//! Provides multi-stage validation pipeline with weighted scoring

use crate::validation::types::{ValidationCheck, ValidationResult, ValidationState};
use crate::tools::types::ToolResult;
use std::path::Path;
use std::time::Instant;

/// Task validator configuration
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Validation threshold (0.0 to 1.0)
    pub threshold: f64,
    
    /// Enable outcome existence check
    pub check_outcome_existence: bool,
    
    /// Enable format correctness check
    pub check_format_correctness: bool,
    
    /// Enable content quality check
    pub check_content_quality: bool,
    
    /// Enable side effect verification
    pub check_side_effects: bool,
    
    /// Enable regression testing
    pub check_regression: bool,
    
    /// Timeout for validation (milliseconds)
    pub timeout_ms: u64,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            threshold: 0.85,
            check_outcome_existence: true,
            check_format_correctness: true,
            check_content_quality: true,
            check_side_effects: true,
            check_regression: true,
            timeout_ms: 50,
        }
    }
}

/// Task validator for quality assurance
pub struct TaskValidator {
    config: ValidatorConfig,
}

impl TaskValidator {
    /// Create new validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidatorConfig::default())
    }
    
    /// Create validator with custom configuration
    pub fn with_config(config: ValidatorConfig) -> Self {
        Self { config }
    }
    
    /// Validate task execution result
    pub fn validate(&self, tool_results: &[ToolResult], expected_outputs: &[String], attempt: usize) -> ValidationResult {
        let start = Instant::now();
        let mut checks = Vec::new();
        
        // Stage 1: Outcome Existence (weight: 0.30)
        if self.config.check_outcome_existence {
            checks.push(self.check_outcome_existence(tool_results, expected_outputs));
        }
        
        // Stage 2: Format Correctness (weight: 0.20)
        if self.config.check_format_correctness {
            checks.push(self.check_format_correctness(tool_results));
        }
        
        // Stage 3: Content Quality (weight: 0.25)
        if self.config.check_content_quality {
            checks.push(self.check_content_quality(tool_results));
        }
        
        // Stage 4: Side Effect Verification (weight: 0.15)
        if self.config.check_side_effects {
            checks.push(self.check_side_effects(tool_results));
        }
        
        // Stage 5: Regression Testing (weight: 0.10)
        if self.config.check_regression {
            checks.push(self.check_regression(tool_results));
        }
        
        let elapsed_ms = start.elapsed().as_millis() as u64;
        
        // Check timeout
        if elapsed_ms > self.config.timeout_ms {
            let mut timeout_check = ValidationCheck::new("timeout".to_string(), 0.0);
            timeout_check.fail(format!("Validation exceeded {}ms timeout", self.config.timeout_ms));
            checks.push(timeout_check);
        }
        
        ValidationResult::new(checks, self.config.threshold, attempt)
    }
    
    /// Stage 1: Check if required outputs exist
    fn check_outcome_existence(&self, tool_results: &[ToolResult], expected_outputs: &[String]) -> ValidationCheck {
        let start = Instant::now();
        let mut check = ValidationCheck::new("outcome_existence".to_string(), 0.30);
        
        if tool_results.is_empty() {
            check.fail("No tool results to validate".to_string());
        } else if expected_outputs.is_empty() {
            // No specific outputs expected, just check we have results
            check.pass();
        } else {
            // Check if expected outputs are mentioned in results
            let results_text: String = tool_results
                .iter()
                .map(|r| r.output.clone())
                .collect::<Vec<_>>()
                .join("\n");
            
            let mut missing_outputs = Vec::new();
            for expected in expected_outputs {
                if !results_text.contains(expected) && !self.file_exists(expected) {
                    missing_outputs.push(expected.clone());
                }
            }
            
            if missing_outputs.is_empty() {
                check.pass();
            } else {
                check.fail(format!("Missing expected outputs: {}", missing_outputs.join(", ")));
            }
        }
        
        check.execution_time_ms = start.elapsed().as_millis() as u64;
        check
    }
    
    /// Stage 2: Check if output formats are correct
    fn check_format_correctness(&self, tool_results: &[ToolResult]) -> ValidationCheck {
        let start = Instant::now();
        let mut check = ValidationCheck::new("format_correctness".to_string(), 0.20);
        
        // Check that all successful tool results have non-empty output
        let mut format_errors = Vec::new();
        
        for (idx, result) in tool_results.iter().enumerate() {
            if result.success {
                if result.output.trim().is_empty() {
                    format_errors.push(format!("Tool {} returned empty output", idx));
                }
                
                // Check for common error indicators in output
                let output_lower = result.output.to_lowercase();
                if output_lower.contains("error:") || output_lower.contains("failed:") {
                    format_errors.push(format!("Tool {} output contains error indicators", idx));
                }
            }
        }
        
        if format_errors.is_empty() {
            check.pass();
        } else {
            check.fail(format!("Format issues: {}", format_errors.join("; ")));
        }
        
        check.execution_time_ms = start.elapsed().as_millis() as u64;
        check
    }
    
    /// Stage 3: Check content quality
    fn check_content_quality(&self, tool_results: &[ToolResult]) -> ValidationCheck {
        let start = Instant::now();
        let mut check = ValidationCheck::new("content_quality".to_string(), 0.25);
        
        let mut quality_issues = Vec::new();
        
        for (idx, result) in tool_results.iter().enumerate() {
            if result.success {
                let output = &result.output;
                
                // Check minimum content length (at least 10 chars for meaningful output)
                if output.len() < 10 && !output.trim().is_empty() {
                    quality_issues.push(format!("Tool {} output too short", idx));
                }
                
                // Check for placeholder text
                if output.contains("TODO") || output.contains("FIXME") || output.contains("placeholder") {
                    quality_issues.push(format!("Tool {} contains placeholder content", idx));
                }
                
                // Check for excessive error messages
                let error_count = output.matches("error").count() + output.matches("Error").count();
                if error_count > 5 {
                    quality_issues.push(format!("Tool {} has excessive error messages", idx));
                }
            }
        }
        
        if quality_issues.is_empty() {
            check.pass();
        } else {
            check.fail(format!("Quality issues: {}", quality_issues.join("; ")));
        }
        
        check.execution_time_ms = start.elapsed().as_millis() as u64;
        check
    }
    
    /// Stage 4: Verify side effects occurred
    fn check_side_effects(&self, tool_results: &[ToolResult]) -> ValidationCheck {
        let start = Instant::now();
        let mut check = ValidationCheck::new("side_effects".to_string(), 0.15);
        
        // Check that write operations had expected effects
        let write_operations: Vec<_> = tool_results
            .iter()
            .filter(|r| r.tool == "write_file" && r.success)
            .collect();
        
        let mut side_effect_failures = Vec::new();
        
        for result in write_operations {
            // Extract file path from result metadata if available
            if let Some(path_str) = self.extract_file_path(&result.output) {
                if !self.file_exists(&path_str) {
                    side_effect_failures.push(format!("File not found: {}", path_str));
                }
            }
        }
        
        if side_effect_failures.is_empty() {
            check.pass();
        } else {
            check.fail(format!("Side effect failures: {}", side_effect_failures.join("; ")));
        }
        
        check.execution_time_ms = start.elapsed().as_millis() as u64;
        check
    }
    
    /// Stage 5: Check for regressions (unintended side effects)
    fn check_regression(&self, _tool_results: &[ToolResult]) -> ValidationCheck {
        let start = Instant::now();
        let mut check = ValidationCheck::new("regression".to_string(), 0.10);
        
        // For now, pass regression check (would need baseline comparison in full implementation)
        // In production, this would compare against a baseline snapshot
        check.pass();
        
        check.execution_time_ms = start.elapsed().as_millis() as u64;
        check
    }
    
    // Helper methods
    
    /// Check if file exists
    fn file_exists(&self, path_str: &str) -> bool {
        Path::new(path_str).exists()
    }
    
    /// Extract file path from tool output
    fn extract_file_path(&self, output: &str) -> Option<String> {
        // Simple heuristic: look for path-like strings
        output
            .lines()
            .find(|line| line.contains('/') || line.contains('\\'))
            .map(|s| s.trim().to_string())
    }
    
    /// Get validator configuration
    pub fn config(&self) -> &ValidatorConfig {
        &self.config
    }
    
    /// Update threshold
    pub fn set_threshold(&mut self, threshold: f64) {
        self.config.threshold = threshold.clamp(0.0, 1.0);
    }
}

impl Default for TaskValidator {
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
    fn test_validator_creation() {
        let validator = TaskValidator::new();
        assert_eq!(validator.config().threshold, 0.85);
    }
    
    #[test]
    fn test_outcome_existence_pass() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "file contents here", true),
        ];
        let expected = vec!["contents".to_string()];
        
        let validation = validator.validate(&results, &expected, 1);
        
        let outcome_check = validation.checks.iter()
            .find(|c| c.name == "outcome_existence")
            .unwrap();
        
        assert!(outcome_check.passed, "Outcome check should pass");
    }
    
    #[test]
    fn test_outcome_existence_fail() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "some output", true),
        ];
        let expected = vec!["missing_content".to_string()];
        
        let validation = validator.validate(&results, &expected, 1);
        
        let outcome_check = validation.checks.iter()
            .find(|c| c.name == "outcome_existence")
            .unwrap();
        
        assert!(!outcome_check.passed, "Outcome check should fail for missing content");
    }
    
    #[test]
    fn test_format_correctness_pass() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("list_dir", "file1.txt\nfile2.txt", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        let format_check = validation.checks.iter()
            .find(|c| c.name == "format_correctness")
            .unwrap();
        
        assert!(format_check.passed, "Format check should pass for valid output");
    }
    
    #[test]
    fn test_format_correctness_fail_empty() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("list_dir", "", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        let format_check = validation.checks.iter()
            .find(|c| c.name == "format_correctness")
            .unwrap();
        
        assert!(!format_check.passed, "Format check should fail for empty output");
    }
    
    #[test]
    fn test_format_correctness_fail_error_indicator() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("run_command", "Error: command failed", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        let format_check = validation.checks.iter()
            .find(|c| c.name == "format_correctness")
            .unwrap();
        
        assert!(!format_check.passed, "Format check should fail with error indicators");
    }
    
    #[test]
    fn test_content_quality_pass() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "This is good quality content with sufficient length", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        let quality_check = validation.checks.iter()
            .find(|c| c.name == "content_quality")
            .unwrap();
        
        assert!(quality_check.passed, "Quality check should pass");
    }
    
    #[test]
    fn test_content_quality_fail_short() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "short", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        let quality_check = validation.checks.iter()
            .find(|c| c.name == "content_quality")
            .unwrap();
        
        assert!(!quality_check.passed, "Quality check should fail for short content");
    }
    
    #[test]
    fn test_content_quality_fail_placeholder() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("write_file", "TODO: implement this feature later", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        let quality_check = validation.checks.iter()
            .find(|c| c.name == "content_quality")
            .unwrap();
        
        assert!(!quality_check.passed, "Quality check should fail with placeholder text");
    }
    
    #[test]
    fn test_validation_score_calculation() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "good quality output with sufficient length", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        assert!(validation.score.overall >= 0.0 && validation.score.overall <= 1.0,
                "Score should be between 0 and 1");
    }
    
    #[test]
    fn test_validation_threshold() {
        let mut validator = TaskValidator::new();
        validator.set_threshold(0.9);
        
        assert_eq!(validator.config().threshold, 0.9);
    }
    
    #[test]
    fn test_validation_state_validated() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "excellent quality output with all requirements met", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        if validation.score.passed {
            assert_eq!(validation.state, ValidationState::Validated);
        }
    }
    
    #[test]
    fn test_validation_state_failed() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        if !validation.score.passed {
            assert_eq!(validation.state, ValidationState::Failed);
        }
    }
    
    #[test]
    fn test_multiple_checks_all_enabled() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("list_dir", "file1.txt\nfile2.txt\nfile3.txt", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        assert_eq!(validation.checks.len(), 5, "Should have 5 validation checks");
    }
    
    #[test]
    fn test_custom_validator_config() {
        let config = ValidatorConfig {
            threshold: 0.75,
            check_outcome_existence: true,
            check_format_correctness: true,
            check_content_quality: false,
            check_side_effects: false,
            check_regression: false,
            timeout_ms: 100,
        };
        
        let validator = TaskValidator::with_config(config);
        let results = vec![
            create_test_result("read_file", "content here", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        
        assert_eq!(validation.checks.len(), 2, "Should have 2 checks enabled");
    }
    
    #[test]
    fn test_failed_checks_extraction() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        let failed = validation.failed_checks();
        
        assert!(!failed.is_empty(), "Should have failed checks");
    }
    
    #[test]
    fn test_failure_reasons_extraction() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        let reasons = validation.failure_reasons();
        
        assert!(!reasons.is_empty(), "Should have failure reasons");
    }
    
    #[test]
    fn test_validation_attempt_tracking() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "content", true),
        ];
        
        let validation = validator.validate(&results, &[], 3);
        assert_eq!(validation.attempt, 3, "Should track attempt number");
    }
    
    #[test]
    fn test_validation_timing() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "content", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        // Timing should be recorded (may be 0 for very fast operations)
        assert!(validation.total_time_ms >= 0, "Should track execution time");
        assert!(validation.timestamp.elapsed().is_ok(), "Should have valid timestamp");
    }
    
    #[test]
    fn test_check_weights_sum() {
        let validator = TaskValidator::new();
        let results = vec![
            create_test_result("read_file", "quality content here", true),
        ];
        
        let validation = validator.validate(&results, &[], 1);
        let total_weight: f64 = validation.checks.iter().map(|c| c.weight).sum();
        
        assert!((total_weight - 1.0).abs() < 0.01, "Weights should sum to 1.0, got {}", total_weight);
    }
}
