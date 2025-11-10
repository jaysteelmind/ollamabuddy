//! Core functionality module (PRD 3)

pub mod advisor;
pub mod bootstrap;
pub mod doctor;

// Re-export for convenience
pub use advisor::{ModelAdvisor, ModelSize, TaskMetrics, ModelUpgrade};
pub use bootstrap::Bootstrap;
pub use doctor::{Doctor, HealthReport, HealthCheck, CheckStatus};
