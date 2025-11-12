//! Budget system type definitions

use serde::{Deserialize, Serialize};

/// Configuration for dynamic budget calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Base iterations for simple tasks (default: 8)
    pub base_iterations: usize,
    
    /// Scaling factor for complexity (default: 25.0)
    pub scale_factor: f64,
    
    /// Uncertainty margin multiplier (default: 0.2)
    pub uncertainty_margin: f64,
    
    /// Absolute maximum budget (default: 50)
    pub max_budget: usize,
    
    /// Warning threshold percentage (default: 0.8)
    pub warning_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            base_iterations: 8,
            scale_factor: 25.0,
            uncertainty_margin: 0.2,
            max_budget: 50,
            warning_threshold: 0.8,
        }
    }
}

/// Budget warning types
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetWarning {
    /// Approaching budget limit
    ApproachingLimit {
        used: usize,
        allocated: usize,
        remaining: usize,
    },
    
    /// Budget exhausted
    Exhausted {
        used: usize,
        allocated: usize,
    },
    
    /// Complexity increased during execution
    ComplexityIncreased {
        old_complexity: f64,
        new_complexity: f64,
        additional_budget: usize,
    },
}
