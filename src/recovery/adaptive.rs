//! Adaptive recovery system with failure pattern recognition
//! Implements intelligent strategy rotation and recovery action selection

use crate::recovery::types::{FailurePattern, FailureSymptom, RecoveryAction, RecoveryStrategy};
use std::collections::HashMap;

/// Adaptive recovery configuration
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum strategy attempts before giving up
    pub max_strategy_attempts: usize,
    
    /// Maximum failure history size
    pub max_history_size: usize,
    
    /// Parallelism levels to try
    pub parallelism_levels: Vec<usize>,
    
    /// Enable aggressive recovery
    pub aggressive_recovery: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_strategy_attempts: 3,
            max_history_size: 50,
            parallelism_levels: vec![4, 2, 1],
            aggressive_recovery: false,
        }
    }
}

/// Adaptive recovery system
pub struct AdaptiveRecovery {
    /// Configuration
    config: RecoveryConfig,
    
    /// Failure pattern history
    failure_history: HashMap<String, FailurePattern>,
    
    /// Strategy rotation queue
    strategy_rotation: Vec<RecoveryStrategy>,
    
    /// Current strategy index
    current_strategy_index: usize,
    
    /// Current strategy attempts
    strategy_attempts: HashMap<RecoveryStrategy, usize>,
    
    /// Current parallelism level index
    parallelism_index: usize,
}

impl AdaptiveRecovery {
    /// Create new adaptive recovery with default configuration
    pub fn new() -> Self {
        Self::with_config(RecoveryConfig::default())
    }
    
    /// Create adaptive recovery with custom configuration
    pub fn with_config(config: RecoveryConfig) -> Self {
        let strategy_rotation = RecoveryStrategy::rotation_order();
        let mut strategy_attempts = HashMap::new();
        
        for strategy in &strategy_rotation {
            strategy_attempts.insert(*strategy, 0);
        }
        
        Self {
            config,
            failure_history: HashMap::new(),
            strategy_rotation,
            current_strategy_index: 0,
            strategy_attempts,
            parallelism_index: 0,
        }
    }
    
    /// Detect failure pattern from symptom
    pub fn detect_pattern(&mut self, symptom: FailureSymptom) -> Option<FailurePattern> {
        let key = format!("{:?}", symptom);
        
        if let Some(pattern) = self.failure_history.get_mut(&key) {
            pattern.update();
            return Some(pattern.clone());
        }
        
        // New pattern
        let pattern = FailurePattern::new(symptom);
        self.failure_history.insert(key, pattern.clone());
        
        // Maintain bounded history
        if self.failure_history.len() > self.config.max_history_size {
            self.prune_old_patterns();
        }
        
        Some(pattern)
    }
    
    /// Select recovery action based on failure pattern
    pub fn select_recovery_action(&mut self, pattern: &FailurePattern) -> RecoveryAction {
        let current_strategy = self.get_current_strategy();
        let attempts = self.strategy_attempts.get(&current_strategy).copied().unwrap_or(0);
        
        // Match symptom to appropriate action
        match &pattern.symptom {
            FailureSymptom::ToolExecutionFailure { consecutive_failures, .. } => {
                if *consecutive_failures >= 3 {
                    if attempts < self.config.max_strategy_attempts {
                        RecoveryAction::RotateStrategy
                    } else {
                        RecoveryAction::Abort {
                            reason: "Tool execution failing persistently".to_string(),
                        }
                    }
                } else {
                    RecoveryAction::RetryWithBackoff {
                        attempt: *consecutive_failures,
                        delay_ms: 100 * (2_u64.pow(*consecutive_failures as u32)),
                    }
                }
            }
            
            FailureSymptom::ValidationFailure { score, threshold } => {
                if *score >= 75 && *threshold > 75 {
                    // Close to passing, relax threshold
                    RecoveryAction::RelaxValidation {
                        new_threshold: 75,
                    }
                } else if attempts < self.config.max_strategy_attempts {
                    RecoveryAction::RotateStrategy
                } else {
                    RecoveryAction::ReassessComplexity
                }
            }
            
            FailureSymptom::StagnationFailure { iterations_stagnant } => {
                if *iterations_stagnant >= 5 {
                    if attempts < self.config.max_strategy_attempts {
                        RecoveryAction::RotateStrategy
                    } else {
                        RecoveryAction::SimplifyApproach
                    }
                } else {
                    RecoveryAction::ReassessComplexity
                }
            }
            
            FailureSymptom::BudgetExhaustion { .. } => {
                RecoveryAction::Abort {
                    reason: "Iteration budget exhausted".to_string(),
                }
            }
            
            FailureSymptom::Timeout { .. } => {
                if self.parallelism_index < self.config.parallelism_levels.len() - 1 {
                    let from = self.config.parallelism_levels[self.parallelism_index];
                    self.parallelism_index += 1;
                    let to = self.config.parallelism_levels[self.parallelism_index];
                    
                    RecoveryAction::ReduceParallelism { from, to }
                } else {
                    RecoveryAction::SimplifyApproach
                }
            }
            
            FailureSymptom::Unknown => {
                if attempts < self.config.max_strategy_attempts {
                    RecoveryAction::RotateStrategy
                } else {
                    RecoveryAction::Abort {
                        reason: "Unknown failure persisting".to_string(),
                    }
                }
            }
        }
    }
    
    /// Rotate to next strategy
    pub fn rotate_strategy(&mut self) -> RecoveryStrategy {
        let current = self.get_current_strategy();
        
        // Increment attempts for current strategy
        *self.strategy_attempts.get_mut(&current).unwrap() += 1;
        
        // Move to next strategy
        self.current_strategy_index = (self.current_strategy_index + 1) % self.strategy_rotation.len();
        
        self.get_current_strategy()
    }
    
    /// Get current strategy
    pub fn get_current_strategy(&self) -> RecoveryStrategy {
        self.strategy_rotation[self.current_strategy_index]
    }
    
    /// Get strategy attempts
    pub fn get_strategy_attempts(&self, strategy: RecoveryStrategy) -> usize {
        self.strategy_attempts.get(&strategy).copied().unwrap_or(0)
    }
    
    /// Check if should abort (too many failures)
    pub fn should_abort(&self) -> bool {
        // Check if all strategies have been attempted max times
        self.strategy_attempts
            .values()
            .all(|&attempts| attempts >= self.config.max_strategy_attempts)
    }
    
    /// Get failure history
    pub fn get_failure_history(&self) -> &HashMap<String, FailurePattern> {
        &self.failure_history
    }
    
    /// Get recent failure count
    pub fn get_recent_failure_count(&self) -> usize {
        self.failure_history
            .values()
            .filter(|p| p.is_recent())
            .count()
    }
    
    /// Prune old patterns from history
    fn prune_old_patterns(&mut self) {
        // Remove patterns that are not recent
        self.failure_history.retain(|_, pattern| pattern.is_recent());
        
        // If still too large, remove least frequent
        if self.failure_history.len() > self.config.max_history_size {
            let mut patterns: Vec<_> = self.failure_history
                .iter()
                .map(|(k, p)| (k.clone(), p.frequency))
                .collect();
            patterns.sort_by_key(|(_, freq)| *freq);
            
            let to_remove = patterns.len() - self.config.max_history_size;
            let keys_to_remove: Vec<_> = patterns
                .iter()
                .take(to_remove)
                .map(|(k, _)| k.clone())
                .collect();
            
            for key in keys_to_remove {
                self.failure_history.remove(&key);
            }
        }
    }
    
    /// Reset recovery state
    pub fn reset(&mut self) {
        self.failure_history.clear();
        self.current_strategy_index = 0;
        self.parallelism_index = 0;
        
        for attempts in self.strategy_attempts.values_mut() {
            *attempts = 0;
        }
    }
    
    /// Get configuration
    pub fn config(&self) -> &RecoveryConfig {
        &self.config
    }
}

impl Default for AdaptiveRecovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_recovery_creation() {
        let recovery = AdaptiveRecovery::new();
        assert_eq!(recovery.get_current_strategy(), RecoveryStrategy::Direct);
        assert_eq!(recovery.get_recent_failure_count(), 0);
    }
    
    #[test]
    fn test_detect_pattern() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::ToolExecutionFailure {
            tool_name: "test_tool".to_string(),
            consecutive_failures: 1,
        };
        
        let pattern = recovery.detect_pattern(symptom.clone());
        assert!(pattern.is_some());
        
        let p = pattern.unwrap();
        assert_eq!(p.frequency, 1);
    }
    
    #[test]
    fn test_pattern_frequency_tracking() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::StagnationFailure {
            iterations_stagnant: 3,
        };
        
        recovery.detect_pattern(symptom.clone());
        recovery.detect_pattern(symptom.clone());
        let pattern = recovery.detect_pattern(symptom);
        
        assert_eq!(pattern.unwrap().frequency, 3);
    }
    
    #[test]
    fn test_strategy_rotation() {
        let mut recovery = AdaptiveRecovery::new();
        
        assert_eq!(recovery.get_current_strategy(), RecoveryStrategy::Direct);
        
        let next = recovery.rotate_strategy();
        assert_eq!(next, RecoveryStrategy::Exploratory);
        
        let next = recovery.rotate_strategy();
        assert_eq!(next, RecoveryStrategy::Systematic);
        
        let next = recovery.rotate_strategy();
        assert_eq!(next, RecoveryStrategy::Direct);
    }
    
    #[test]
    fn test_recovery_action_tool_failure() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::ToolExecutionFailure {
            tool_name: "test".to_string(),
            consecutive_failures: 1,
        };
        
        let pattern = recovery.detect_pattern(symptom).unwrap();
        let action = recovery.select_recovery_action(&pattern);
        
        assert!(matches!(action, RecoveryAction::RetryWithBackoff { .. }));
    }
    
    #[test]
    fn test_recovery_action_validation_failure() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::ValidationFailure {
            score: 80,
            threshold: 85,
        };
        
        let pattern = recovery.detect_pattern(symptom).unwrap();
        let action = recovery.select_recovery_action(&pattern);
        
        assert!(matches!(action, RecoveryAction::RelaxValidation { .. }));
    }
    
    #[test]
    fn test_recovery_action_stagnation() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::StagnationFailure {
            iterations_stagnant: 5,
        };
        
        let pattern = recovery.detect_pattern(symptom).unwrap();
        let action = recovery.select_recovery_action(&pattern);
        
        assert!(matches!(action, RecoveryAction::RotateStrategy));
    }
    
    #[test]
    fn test_recovery_action_budget_exhaustion() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::BudgetExhaustion {
            used: 20,
            allocated: 20,
        };
        
        let pattern = recovery.detect_pattern(symptom).unwrap();
        let action = recovery.select_recovery_action(&pattern);
        
        assert!(matches!(action, RecoveryAction::Abort { .. }));
    }
    
    #[test]
    fn test_recovery_action_timeout() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::Timeout {
            operation: "test_op".to_string(),
        };
        
        let pattern = recovery.detect_pattern(symptom).unwrap();
        let action = recovery.select_recovery_action(&pattern);
        
        assert!(matches!(action, RecoveryAction::ReduceParallelism { .. }));
    }
    
    #[test]
    fn test_strategy_attempts_tracking() {
        let mut recovery = AdaptiveRecovery::new();
        
        assert_eq!(recovery.get_strategy_attempts(RecoveryStrategy::Direct), 0);
        
        recovery.rotate_strategy();
        assert_eq!(recovery.get_strategy_attempts(RecoveryStrategy::Direct), 1);
    }
    
    #[test]
    fn test_should_abort() {
        let mut recovery = AdaptiveRecovery::new();
        
        assert!(!recovery.should_abort());
        
        // Exhaust all strategies
        for _ in 0..3 {
            recovery.rotate_strategy();
            recovery.rotate_strategy();
            recovery.rotate_strategy();
        }
        
        assert!(recovery.should_abort());
    }
    
    #[test]
    fn test_bounded_history() {
        let config = RecoveryConfig {
            max_history_size: 5,
            ..Default::default()
        };
        
        let mut recovery = AdaptiveRecovery::with_config(config);
        
        for i in 0..10 {
            let symptom = FailureSymptom::ToolExecutionFailure {
                tool_name: format!("tool_{}", i),
                consecutive_failures: 1,
            };
            recovery.detect_pattern(symptom);
        }
        
        assert!(recovery.get_failure_history().len() <= 5);
    }
    
    #[test]
    fn test_reset() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::StagnationFailure {
            iterations_stagnant: 3,
        };
        
        recovery.detect_pattern(symptom);
        recovery.rotate_strategy();
        
        assert!(!recovery.get_failure_history().is_empty());
        assert_eq!(recovery.get_current_strategy(), RecoveryStrategy::Exploratory);
        
        recovery.reset();
        
        assert!(recovery.get_failure_history().is_empty());
        assert_eq!(recovery.get_current_strategy(), RecoveryStrategy::Direct);
    }
    
    #[test]
    fn test_parallelism_reduction() {
        let mut recovery = AdaptiveRecovery::new();
        
        let symptom = FailureSymptom::Timeout {
            operation: "test".to_string(),
        };
        
        let pattern1 = recovery.detect_pattern(symptom.clone()).unwrap();
        let action1 = recovery.select_recovery_action(&pattern1);
        
        if let RecoveryAction::ReduceParallelism { from, to } = action1 {
            assert_eq!(from, 4);
            assert_eq!(to, 2);
        } else {
            panic!("Expected ReduceParallelism action");
        }
        
        let pattern2 = recovery.detect_pattern(symptom).unwrap();
        let action2 = recovery.select_recovery_action(&pattern2);
        
        if let RecoveryAction::ReduceParallelism { from, to } = action2 {
            assert_eq!(from, 2);
            assert_eq!(to, 1);
        } else {
            panic!("Expected ReduceParallelism action");
        }
    }
    
    #[test]
    fn test_aggressive_recovery_config() {
        let config = RecoveryConfig {
            aggressive_recovery: true,
            max_strategy_attempts: 5,
            ..Default::default()
        };
        
        let recovery = AdaptiveRecovery::with_config(config);
        assert!(recovery.config().aggressive_recovery);
        assert_eq!(recovery.config().max_strategy_attempts, 5);
    }
    
    #[test]
    fn test_failure_symptom_severity() {
        let budget = FailureSymptom::BudgetExhaustion { used: 20, allocated: 20 };
        assert_eq!(budget.severity(), 9);
        
        let validation = FailureSymptom::ValidationFailure { score: 70, threshold: 85 };
        assert_eq!(validation.severity(), 7);
        
        let stagnation = FailureSymptom::StagnationFailure { iterations_stagnant: 5 };
        assert_eq!(stagnation.severity(), 6);
    }
    
    #[test]
    fn test_recovery_strategy_next() {
        assert_eq!(RecoveryStrategy::Direct.next(), RecoveryStrategy::Exploratory);
        assert_eq!(RecoveryStrategy::Exploratory.next(), RecoveryStrategy::Systematic);
        assert_eq!(RecoveryStrategy::Systematic.next(), RecoveryStrategy::Direct);
    }
}
    
    #[test]
    fn test_recovery_action_priority() {
        let abort = RecoveryAction::Abort { reason: "test".to_string() };
        assert_eq!(abort.priority(), 10);
        
        let rotate = RecoveryAction::RotateStrategy;
        assert_eq!(rotate.priority(), 7);
        
        let retry = RecoveryAction::RetryWithBackoff { attempt: 1, delay_ms: 100 };
        assert_eq!(retry.priority(), 3);
    }
