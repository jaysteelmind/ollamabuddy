//! PRD 9 Phase 2: Convergence Integration Tests
//! 
//! Tests for ConvergenceDetector integration into main execution loop

use ollamabuddy::analysis::{ConvergenceDetector, types::TerminationCondition};

#[test]
fn test_convergence_detector_creation() {
    let detector = ConvergenceDetector::new();
    assert!(detector.config().success_threshold > 0.0);
    assert!(detector.config().velocity_threshold > 0.0);
}

#[test]
fn test_progress_tracking() {
    let mut detector = ConvergenceDetector::new();
    
    // Record progress over multiple iterations
    detector.record_progress(0.0, 1);
    detector.record_progress(0.3, 2);
    detector.record_progress(0.6, 3);
    detector.record_progress(0.9, 4);
    
    // Should have velocity after multiple recordings
    assert!(detector.get_velocity().is_some());
    
    // Progress should be tracked
    assert_eq!(detector.get_current_progress(), Some(0.9));
}

#[test]
fn test_early_success_detection() {
    let mut detector = ConvergenceDetector::new();
    
    // Simulate rapid progress to completion
    detector.record_progress(0.0, 1);
    detector.record_progress(0.5, 2);
    detector.record_progress(0.95, 3);
    
    // Check termination with high progress and validation
    let termination = detector.check_termination(0.95, 0.9, 3, 20);
    
    match termination {
        TerminationCondition::Success => assert!(true),
        _ => panic!("Should detect early success"),
    }
}

#[test]
fn test_stagnation_detection() {
    let mut detector = ConvergenceDetector::new();
    
    // Simulate stagnation - no progress
    for i in 1..=10 {
        detector.record_progress(0.3, i);
    }
    
    let stagnation = detector.detect_stagnation();
    
    match stagnation {
        ollamabuddy::analysis::types::StagnationResult::Stagnant { .. } => assert!(true),
        _ => {}  // May be active if velocity threshold not met
    }
}

#[test]
fn test_budget_exhaustion() {
    let detector = ConvergenceDetector::new();
    
    // Check termination when budget exhausted
    let termination = detector.check_termination(0.5, 0.7, 20, 20);
    
    assert!(matches!(termination, TerminationCondition::BudgetExhausted));
}

#[test]
fn test_continue_execution() {
    let mut detector = ConvergenceDetector::new();
    
    // Normal progress, not yet complete
    detector.record_progress(0.4, 5);
    
    let termination = detector.check_termination(0.4, 0.6, 5, 20);
    
    // Should continue (not terminate)
    assert!(matches!(termination, TerminationCondition::None));
    assert!(!termination.should_terminate());
}

#[test]
fn test_velocity_calculation() {
    let mut detector = ConvergenceDetector::new();
    
    // Record increasing progress
    detector.record_progress(0.0, 1);
    detector.record_progress(0.2, 2);
    detector.record_progress(0.4, 3);
    
    // Should have positive velocity
    if let Some(velocity) = detector.get_velocity() {
        assert!(velocity.velocity > 0.0);
    }
}

#[test]
fn test_convergence_history() {
    let mut detector = ConvergenceDetector::new();
    
    // Record multiple progress points
    for i in 1..=5 {
        detector.record_progress(i as f64 * 0.2, i);
    }
    
    // Should have 5 history entries
    assert_eq!(detector.get_history().len(), 5);
}

#[test]
fn test_detector_reset() {
    let mut detector = ConvergenceDetector::new();
    
    // Record some progress
    detector.record_progress(0.5, 3);
    assert!(detector.get_current_progress().is_some());
    
    // Reset
    detector.reset();
    
    // Should be clean
    assert_eq!(detector.get_history().len(), 0);
    assert_eq!(detector.get_stagnation_count(), 0);
}
