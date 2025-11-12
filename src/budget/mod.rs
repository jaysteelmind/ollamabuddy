//! Dynamic iteration budget management system
//! Provides complexity-based iteration allocation with mathematical guarantees

pub mod manager;
pub mod types;

pub use manager::DynamicBudgetManager;
pub use types::{BudgetConfig, BudgetWarning};
