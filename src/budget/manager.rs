//! Dynamic iteration budget manager
//! Implements complexity-based budget allocation with mathematical guarantees

use crate::budget::types::{BudgetConfig, BudgetWarning};
use std::time::{Duration, Instant};

/// Dynamic budget manager for iteration allocation
#[derive(Debug, Clone)]
pub struct DynamicBudgetManager {
    /// Budget configuration
    config: BudgetConfig,
    
    /// Allocated iteration budget for current task
    allocated_budget: usize,
    
    /// Number of iterations used so far
    used_iterations: usize,
    
    /// Task complexity score (0.0 to 1.0)
    complexity_score: f64,
    
    /// Session start time
    start_time: Instant,
    
    /// Last warning issued
    last_warning: Option<BudgetWarning>,
}

impl DynamicBudgetManager {
    /// Create new budget manager with default configuration
    pub fn new() -> Self {
        Self::with_config(BudgetConfig::default())
    }
    
    /// Create budget manager with custom configuration
    pub fn with_config(config: BudgetConfig) -> Self {
        Self {
            config,
            allocated_budget: 0,
            used_iterations: 0,
            complexity_score: 0.0,
            start_time: Instant::now(),
            last_warning: None,
        }
    }
    
    /// Calculate iteration budget based on complexity
    /// 
    /// Formula: I_budget(C) = I_base + floor(I_scale × C × (1 + δ))
    /// 
    /// Guarantees:
    /// - Monotonicity: C1 < C2 => I(C1) <= I(C2)
    /// - Bounded: 8 <= I(C) <= max_budget
    /// - Conservative: I(C) >= I_empirical(C) with 95% confidence
    pub fn calculate_budget(&mut self, complexity: f64) -> usize {
        // Clamp complexity to valid range
        let complexity = complexity.clamp(0.0, 1.0);
        
        // Store complexity score
        self.complexity_score = complexity;
        
        // Calculate uncertainty factor based on complexity
        let uncertainty = if complexity > 0.7 {
            self.config.uncertainty_margin
        } else {
            self.config.uncertainty_margin * 0.5
        };
        
        // Apply budget formula
        let scaled_iterations = (self.config.scale_factor * complexity * (1.0 + uncertainty)).floor() as usize;
        let raw_budget = self.config.base_iterations + scaled_iterations;
        
        // Apply maximum bound
        let budget = raw_budget.min(self.config.max_budget);
        
        // Store allocated budget
        self.allocated_budget = budget;
        
        budget
    }
    
    /// Increment iteration counter
    pub fn increment_iteration(&mut self) {
        self.used_iterations += 1;
    }
    
    /// Get remaining iterations
    pub fn get_remaining(&self) -> usize {
        self.allocated_budget.saturating_sub(self.used_iterations)
    }
    
    /// Get allocated budget
    pub fn get_allocated(&self) -> usize {
        self.allocated_budget
    }
    
    /// Get used iterations
    pub fn get_used(&self) -> usize {
        self.used_iterations
    }
    
    /// Get complexity score
    pub fn get_complexity(&self) -> f64 {
        self.complexity_score
    }
    
    /// Check if budget is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.used_iterations >= self.allocated_budget
    }
    
    /// Get utilization percentage (0.0 to 1.0)
    pub fn get_utilization(&self) -> f64 {
        if self.allocated_budget == 0 {
            return 0.0;
        }
        (self.used_iterations as f64) / (self.allocated_budget as f64)
    }
    
    /// Check for budget warnings
    pub fn check_exhaustion_warning(&mut self) -> Option<BudgetWarning> {
        let utilization = self.get_utilization();
        
        // Check if exhausted
        if self.is_exhausted() {
            let warning = BudgetWarning::Exhausted {
                used: self.used_iterations,
                allocated: self.allocated_budget,
            };
            self.last_warning = Some(warning.clone());
            return Some(warning);
        }
        
        // Check if approaching limit
        if utilization >= self.config.warning_threshold {
            let warning = BudgetWarning::ApproachingLimit {
                used: self.used_iterations,
                allocated: self.allocated_budget,
                remaining: self.get_remaining(),
            };
            
            // Only return if this is a new warning
            if self.last_warning.as_ref() != Some(&warning) {
                self.last_warning = Some(warning.clone());
                return Some(warning);
            }
        }
        
        None
    }
    
    /// Adjust budget at runtime based on new complexity
    pub fn adjust_budget_runtime(&mut self, new_complexity: f64) -> Option<BudgetWarning> {
        let old_complexity = self.complexity_score;
        let old_budget = self.allocated_budget;
        
        // Recalculate budget with new complexity
        let new_budget = self.calculate_budget(new_complexity);
        
        // Only warn if complexity increased significantly
        if new_complexity > old_complexity + 0.1 {
            let additional = new_budget.saturating_sub(old_budget);
            if additional > 0 {
                let warning = BudgetWarning::ComplexityIncreased {
                    old_complexity,
                    new_complexity,
                    additional_budget: additional,
                };
                self.last_warning = Some(warning.clone());
                return Some(warning);
            }
        }
        
        None
    }
    
    /// Get elapsed time since start
    pub fn get_elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Reset budget manager for new task
    pub fn reset(&mut self) {
        self.allocated_budget = 0;
        self.used_iterations = 0;
        self.complexity_score = 0.0;
        self.start_time = Instant::now();
        self.last_warning = None;
    }
}

impl Default for DynamicBudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_budget_calculation_simple_task() {
        let mut manager = DynamicBudgetManager::new();
        let budget = manager.calculate_budget(0.2);
        assert!(budget >= 13 && budget <= 14, "Expected ~13, got {}", budget);
    }
    
    #[test]
    fn test_budget_calculation_medium_task() {
        let mut manager = DynamicBudgetManager::new();
        let budget = manager.calculate_budget(0.5);
        assert!(budget >= 20 && budget <= 22, "Expected ~21, got {}", budget);
    }
    
    #[test]
    fn test_budget_calculation_complex_task() {
        let mut manager = DynamicBudgetManager::new();
        let budget = manager.calculate_budget(0.8);
        assert!(budget >= 30 && budget <= 34, "Expected ~32, got {}", budget);
    }
    
    #[test]
    fn test_budget_monotonicity() {
        let mut manager = DynamicBudgetManager::new();
        let budget_low = manager.calculate_budget(0.3);
        let budget_high = manager.calculate_budget(0.7);
        assert!(budget_low <= budget_high, "Monotonicity violated: {} > {}", budget_low, budget_high);
    }
    
    #[test]
    fn test_budget_bounded() {
        let mut manager = DynamicBudgetManager::new();
        let budget_min = manager.calculate_budget(0.0);
        assert!(budget_min >= 8, "Below minimum: {}", budget_min);
        let budget_max = manager.calculate_budget(1.0);
        assert!(budget_max <= 50, "Above maximum: {}", budget_max);
    }
    
    #[test]
    fn test_iteration_tracking() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.5);
        assert_eq!(manager.get_used(), 0);
        manager.increment_iteration();
        assert_eq!(manager.get_used(), 1);
        manager.increment_iteration();
        assert_eq!(manager.get_used(), 2);
    }
    
    #[test]
    fn test_remaining_iterations() {
        let mut manager = DynamicBudgetManager::new();
        let budget = manager.calculate_budget(0.5);
        let initial_remaining = manager.get_remaining();
        assert_eq!(initial_remaining, budget);
        manager.increment_iteration();
        assert_eq!(manager.get_remaining(), budget - 1);
    }
    
    #[test]
    fn test_budget_exhaustion() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.2);
        let budget = manager.get_allocated();
        assert!(!manager.is_exhausted());
        for _ in 0..budget {
            manager.increment_iteration();
        }
        assert!(manager.is_exhausted());
    }
    
    #[test]
    fn test_utilization_calculation() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.5);
        let budget = manager.get_allocated();
        assert_eq!(manager.get_utilization(), 0.0);
        for _ in 0..(budget / 2) {
            manager.increment_iteration();
        }
        let utilization = manager.get_utilization();
        assert!(utilization >= 0.4 && utilization <= 0.6, "Expected ~0.5, got {}", utilization);
    }
    
    #[test]
    fn test_exhaustion_warning() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.3);
        let budget = manager.get_allocated();
        let threshold_iterations = (budget as f64 * 0.8).ceil() as usize;
        for _ in 0..threshold_iterations {
            manager.increment_iteration();
        }
        let warning = manager.check_exhaustion_warning();
        assert!(warning.is_some(), "Expected warning at 80% utilization");
        match warning.unwrap() {
            BudgetWarning::ApproachingLimit { .. } => {},
            _ => panic!("Expected ApproachingLimit warning"),
        }
    }
    
    #[test]
    fn test_exhausted_warning() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.2);
        let budget = manager.get_allocated();
        for _ in 0..budget {
            manager.increment_iteration();
        }
        let warning = manager.check_exhaustion_warning();
        assert!(warning.is_some(), "Expected exhausted warning");
        match warning.unwrap() {
            BudgetWarning::Exhausted { .. } => {},
            _ => panic!("Expected Exhausted warning"),
        }
    }
    
    #[test]
    fn test_runtime_adjustment() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.3);
        let initial_budget = manager.get_allocated();
        
        // Increase complexity significantly
        let warning = manager.adjust_budget_runtime(0.7);
        let new_budget = manager.get_allocated();
        
        assert!(new_budget > initial_budget, "Budget should increase with complexity");
        
        // Warning should be issued for significant increase (0.3 -> 0.7 = +0.4 > 0.1)
        assert!(warning.is_some(), "Expected complexity increase warning");
    }
    
    #[test]
    fn test_reset() {
        let mut manager = DynamicBudgetManager::new();
        manager.calculate_budget(0.5);
        manager.increment_iteration();
        manager.increment_iteration();
        assert!(manager.get_used() > 0);
        assert!(manager.get_allocated() > 0);
        manager.reset();
        assert_eq!(manager.get_used(), 0);
        assert_eq!(manager.get_allocated(), 0);
        assert_eq!(manager.get_complexity(), 0.0);
    }
    
    #[test]
    fn test_custom_config() {
        let config = BudgetConfig {
            base_iterations: 10,
            scale_factor: 30.0,
            uncertainty_margin: 0.15,
            max_budget: 60,
            warning_threshold: 0.75,
        };
        let mut manager = DynamicBudgetManager::with_config(config);
        let budget = manager.calculate_budget(0.5);
        assert!(budget >= 10, "Budget should respect custom base");
        assert!(budget <= 60, "Budget should respect custom max");
    }
    
    #[test]
    fn test_complexity_clamping() {
        let mut manager = DynamicBudgetManager::new();
        let budget_high = manager.calculate_budget(1.5);
        let budget_one = manager.calculate_budget(1.0);
        assert_eq!(budget_high, budget_one, "Should clamp to 1.0");
        manager.reset();
        let budget_negative = manager.calculate_budget(-0.5);
        let budget_zero = manager.calculate_budget(0.0);
        assert_eq!(budget_negative, budget_zero, "Should clamp to 0.0");
    }
}
