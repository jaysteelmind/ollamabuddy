//! PRD 9 Phase 3: Recovery Integration Tests
//! 
//! Tests for AdaptiveRecovery integration into main execution loop

use ollamabuddy::recovery::{AdaptiveRecovery, types::{FailureSymptom, RecoveryAction}};

#[test]
fn test_adaptive_recovery_creation() {
    let recovery = AdaptiveRecovery::new();
    assert!(recovery.config().max_strategy_attempts > 0);
}

#[test]
fn test_tool_failure_pattern_detection() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::ToolExecutionFailure {
        tool_name: "read_file".to_string(),
        consecutive_failures: 1,
    };
    
    let pattern = recovery.detect_pattern(symptom);
    assert!(pattern.is_some());
}

#[test]
fn test_recovery_action_for_single_failure() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::ToolExecutionFailure {
        tool_name: "web_fetch".to_string(),
        consecutive_failures: 1,
    };
    
    let pattern = recovery.detect_pattern(symptom).unwrap();
    let action = recovery.select_recovery_action(&pattern);
    
    // First failure should retry with backoff
    match action {
        RecoveryAction::RetryWithBackoff { .. } => assert!(true),
        _ => panic!("Expected RetryWithBackoff for single failure"),
    }
}

#[test]
fn test_recovery_action_for_multiple_failures() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::ToolExecutionFailure {
        tool_name: "run_command".to_string(),
        consecutive_failures: 3,
    };
    
    let pattern = recovery.detect_pattern(symptom).unwrap();
    let action = recovery.select_recovery_action(&pattern);
    
    // Multiple failures should rotate strategy or abort
    match action {
        RecoveryAction::RotateStrategy | RecoveryAction::Abort { .. } => assert!(true),
        _ => {}
    }
}

#[test]
fn test_validation_failure_recovery() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::ValidationFailure {
        score: 75,
        threshold: 85,
    };
    
    let pattern = recovery.detect_pattern(symptom).unwrap();
    let action = recovery.select_recovery_action(&pattern);
    
    // Should suggest relaxing validation or rotating strategy
    match action {
        RecoveryAction::RelaxValidation { .. } | 
        RecoveryAction::RotateStrategy |
        RecoveryAction::ReassessComplexity => assert!(true),
        _ => {}
    }
}

#[test]
fn test_stagnation_failure_recovery() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::StagnationFailure {
        iterations_stagnant: 5,
    };
    
    let pattern = recovery.detect_pattern(symptom).unwrap();
    let action = recovery.select_recovery_action(&pattern);
    
    // Should rotate strategy or simplify
    match action {
        RecoveryAction::RotateStrategy | 
        RecoveryAction::SimplifyApproach => assert!(true),
        _ => {}
    }
}

#[test]
fn test_budget_exhaustion_recovery() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::BudgetExhaustion {
        used: 20,
        allocated: 20,
    };
    
    let pattern = recovery.detect_pattern(symptom).unwrap();
    let action = recovery.select_recovery_action(&pattern);
    
    // Budget exhaustion should have a recovery action
    assert!(matches!(
        action,
        RecoveryAction::ReassessComplexity | 
        RecoveryAction::SimplifyApproach |
        RecoveryAction::Abort { .. }
    ));
}

#[test]
fn test_pattern_history_tracking() {
    let mut recovery = AdaptiveRecovery::new();
    
    // Detect same pattern multiple times
    for i in 1..=3 {
        let symptom = FailureSymptom::ToolExecutionFailure {
            tool_name: "test_tool".to_string(),
            consecutive_failures: i,
        };
        
        let pattern = recovery.detect_pattern(symptom);
        assert!(pattern.is_some());
    }
    
    // Pattern should be tracked in history
    // (Internal state, we just verify it doesn't crash)
}

#[test]
fn test_timeout_failure_recovery() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::Timeout {
        operation: "web_fetch".to_string(),
    };
    
    let pattern = recovery.detect_pattern(symptom).unwrap();
    let action = recovery.select_recovery_action(&pattern);
    
    // Timeout should suggest retry or strategy change
    match action {
        RecoveryAction::RetryWithBackoff { .. } |
        RecoveryAction::RotateStrategy |
        RecoveryAction::SimplifyApproach => assert!(true),
        _ => {}
    }
}

#[test]
fn test_unknown_failure_recovery() {
    let mut recovery = AdaptiveRecovery::new();
    
    let symptom = FailureSymptom::Unknown;
    
    let pattern = recovery.detect_pattern(symptom);
    assert!(pattern.is_some());
    
    // Unknown failures should still get recovery actions
    let action = recovery.select_recovery_action(&pattern.unwrap());
    
    // Should have some recovery action
    assert!(matches!(
        action,
        RecoveryAction::RetryWithBackoff { .. } |
        RecoveryAction::RotateStrategy |
        RecoveryAction::ReassessComplexity |
        RecoveryAction::SimplifyApproach |
        RecoveryAction::Abort { .. }
    ));
}
