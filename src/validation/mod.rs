//! Task validation and quality assurance system
//! Provides multi-stage validation with scoring and reporting

pub mod types;
pub mod validator;
pub mod orchestrator;

pub use types::{ValidationCheck, ValidationResult, ValidationScore, ValidationState};
pub use validator::TaskValidator;
pub use orchestrator::ValidationOrchestrator;
