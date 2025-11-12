//! Progress analysis and convergence detection system
//! Provides velocity calculation and stagnation detection

pub mod convergence;
pub mod types;

pub use convergence::ConvergenceDetector;
pub use types::{ProgressMetrics, VelocityMetric, StagnationResult};
