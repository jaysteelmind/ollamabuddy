//! Model Advisor - Statistical Decision Theory for Model Upgrades
//! 
//! Provides intelligent model upgrade recommendations based on task complexity,
//! failure rates, and utility optimization.

use std::collections::HashMap;

/// Available Ollama models with their characteristics
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModelTier {
    Small,   // 7B - qwen2.5:7b-instruct
    Medium,  // 14B - qwen2.5:14b-instruct
    Large,   // 32B - qwen2.5:32b-instruct
}

/// Model characteristics
#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub tier: ModelTier,
    pub tag: String,
    pub size_gb: f64,
    pub vram_gb: u32,
    pub base_success_rate: f64,
    pub speed: String,
}

/// Task metrics for decision making
#[derive(Debug, Clone, Default)]
pub struct TaskMetrics {
    pub complexity_score: f64,
    pub json_failures: u32,
    pub tool_failures: u32,
    pub plan_steps: u32,
    pub total_iterations: u32,
}

/// Upgrade recommendation
#[derive(Debug, Clone)]
pub struct UpgradeRecommendation {
    pub from_model: String,
    pub to_model: String,
    pub confidence: f64,
    pub reason: String,
    pub utility_gain: f64,
}

/// Model Advisor system
pub struct ModelAdvisor {
    current_model: String,
    models: HashMap<ModelTier, ModelSpec>,
    cost_sensitivity: f64,
    upgrade_threshold: f64,
}

impl ModelTier {
    pub fn from_tag(tag: &str) -> Option<Self> {
        if tag.contains("7b") {
            Some(ModelTier::Small)
        } else if tag.contains("14b") {
            Some(ModelTier::Medium)
        } else if tag.contains("32b") {
            Some(ModelTier::Large)
        } else {
            None
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            ModelTier::Small => Some(ModelTier::Medium),
            ModelTier::Medium => Some(ModelTier::Large),
            ModelTier::Large => None,
        }
    }
}

impl ModelAdvisor {
    /// Create a new model advisor
    pub fn new(current_model: String) -> Self {
        let mut models = HashMap::new();

        models.insert(
            ModelTier::Small,
            ModelSpec {
                tier: ModelTier::Small,
                tag: "qwen2.5:7b-instruct".to_string(),
                size_gb: 3.8,
                vram_gb: 4,
                base_success_rate: 0.70,
                speed: "Fast".to_string(),
            },
        );

        models.insert(
            ModelTier::Medium,
            ModelSpec {
                tier: ModelTier::Medium,
                tag: "qwen2.5:14b-instruct".to_string(),
                size_gb: 7.6,
                vram_gb: 8,
                base_success_rate: 0.85,
                speed: "Medium".to_string(),
            },
        );

        models.insert(
            ModelTier::Large,
            ModelSpec {
                tier: ModelTier::Large,
                tag: "qwen2.5:32b-instruct".to_string(),
                size_gb: 18.0,
                vram_gb: 16,
                base_success_rate: 0.95,
                speed: "Slow".to_string(),
            },
        );

        Self {
            current_model,
            models,
            cost_sensitivity: 0.3,
            upgrade_threshold: 0.15,
        }
    }

    /// Set cost sensitivity factor (0.0 to 1.0)
    pub fn with_cost_sensitivity(mut self, sensitivity: f64) -> Self {
        self.cost_sensitivity = sensitivity.clamp(0.0, 1.0);
        self
    }

    /// Set upgrade threshold
    pub fn with_upgrade_threshold(mut self, threshold: f64) -> Self {
        self.upgrade_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Calculate complexity score from task metrics
    fn calculate_complexity(&self, metrics: &TaskMetrics) -> f64 {
        let mut complexity = 0.0;

        // JSON failures contribute to complexity
        if metrics.json_failures > 0 {
            complexity += (metrics.json_failures as f64 * 0.2).min(0.4);
        }

        // Tool failures indicate task difficulty
        if metrics.tool_failures > 0 {
            complexity += (metrics.tool_failures as f64 * 0.15).min(0.3);
        }

        // Number of plan steps indicates complexity
        if metrics.plan_steps > 5 {
            complexity += 0.2;
        }

        // High iteration count suggests struggle
        if metrics.total_iterations > 5 {
            complexity += 0.2;
        }

        complexity.min(1.0)
    }

    /// Calculate task multiplier based on metrics
    fn calculate_task_multiplier(&self, metrics: &TaskMetrics) -> f64 {
        let complexity = self.calculate_complexity(metrics);
        
        // Complexity penalty
        let complexity_multiplier = 1.0 - (complexity * 0.3);

        // Failure penalty
        let total_failures = metrics.json_failures + metrics.tool_failures;
        let failure_multiplier = (1.0 - (total_failures as f64 * 0.1)).max(0.5);

        complexity_multiplier * failure_multiplier
    }

    /// Calculate utility for a given model and task
    fn calculate_utility(&self, model: &ModelSpec, metrics: &TaskMetrics) -> f64 {
        // Base success probability
        let base_prob = model.base_success_rate;

        // Task-specific multiplier
        let task_multiplier = self.calculate_task_multiplier(metrics);

        // Adjusted success probability
        let success_prob = base_prob * task_multiplier;

        // Cost factor (normalized by VRAM)
        let cost = model.vram_gb as f64 / 16.0;

        // Utility = Success probability - (cost_sensitivity * cost)
        success_prob * (1.0 - self.cost_sensitivity * cost)
    }

    /// Check if upgrade should be recommended
    pub fn recommend_upgrade(&self, metrics: &TaskMetrics) -> Option<UpgradeRecommendation> {
        // Get current model tier
        let current_tier = ModelTier::from_tag(&self.current_model)?;
        let current_spec = self.models.get(&current_tier)?;

        // Get next tier
        let next_tier = current_tier.next()?;
        let next_spec = self.models.get(&next_tier)?;

        // Calculate utilities
        let current_utility = self.calculate_utility(current_spec, metrics);
        let next_utility = self.calculate_utility(next_spec, metrics);

        // Calculate utility gain
        let utility_gain = next_utility - current_utility;

        // Check if upgrade is worthwhile
        if utility_gain > self.upgrade_threshold {
            // Determine reason
            let reason = self.generate_reason(metrics);

            // Calculate confidence
            let confidence = (utility_gain / 0.5).min(1.0);

            Some(UpgradeRecommendation {
                from_model: current_spec.tag.clone(),
                to_model: next_spec.tag.clone(),
                confidence,
                reason,
                utility_gain,
            })
        } else {
            None
        }
    }

    /// Generate human-readable reason for upgrade
    fn generate_reason(&self, metrics: &TaskMetrics) -> String {
        let mut reasons = Vec::new();

        if metrics.json_failures > 2 {
            reasons.push(format!("High JSON parsing failures ({})", metrics.json_failures));
        }

        if metrics.tool_failures > 3 {
            reasons.push(format!("Multiple tool execution failures ({})", metrics.tool_failures));
        }

        if metrics.plan_steps > 5 {
            reasons.push("Complex multi-step task".to_string());
        }

        if metrics.total_iterations > 5 {
            reasons.push("Task requires many iterations".to_string());
        }

        if reasons.is_empty() {
            "Task complexity exceeds current model capabilities".to_string()
        } else {
            reasons.join("; ")
        }
    }

    /// Check if specific trigger conditions are met
    pub fn check_triggers(&self, metrics: &TaskMetrics) -> bool {
        metrics.json_failures > 2
            || metrics.tool_failures > 3
            || metrics.plan_steps > 5
            || self.calculate_complexity(metrics) > 0.6
    }

    /// Get model specifications
    pub fn get_model_spec(&self, tier: &ModelTier) -> Option<&ModelSpec> {
        self.models.get(tier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advisor_creation() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        assert_eq!(advisor.current_model, "qwen2.5:7b-instruct");
        assert_eq!(advisor.cost_sensitivity, 0.3);
        assert_eq!(advisor.upgrade_threshold, 0.15);
    }

    #[test]
    fn test_model_tier_from_tag() {
        assert_eq!(ModelTier::from_tag("qwen2.5:7b-instruct"), Some(ModelTier::Small));
        assert_eq!(ModelTier::from_tag("qwen2.5:14b-instruct"), Some(ModelTier::Medium));
        assert_eq!(ModelTier::from_tag("qwen2.5:32b-instruct"), Some(ModelTier::Large));
        assert_eq!(ModelTier::from_tag("unknown"), None);
    }

    #[test]
    fn test_model_tier_next() {
        assert_eq!(ModelTier::Small.next(), Some(ModelTier::Medium));
        assert_eq!(ModelTier::Medium.next(), Some(ModelTier::Large));
        assert_eq!(ModelTier::Large.next(), None);
    }

    #[test]
    fn test_calculate_complexity_low() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        let metrics = TaskMetrics {
            complexity_score: 0.0,
            json_failures: 0,
            tool_failures: 0,
            plan_steps: 2,
            total_iterations: 2,
        };
        let complexity = advisor.calculate_complexity(&metrics);
        assert!(complexity < 0.2);
    }

    #[test]
    fn test_calculate_complexity_high() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        let metrics = TaskMetrics {
            complexity_score: 0.8,
            json_failures: 5,
            tool_failures: 4,
            plan_steps: 8,
            total_iterations: 10,
        };
        let complexity = advisor.calculate_complexity(&metrics);
        assert!(complexity > 0.5);
    }

    #[test]
    fn test_no_upgrade_for_simple_task() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        let metrics = TaskMetrics {
            complexity_score: 0.0,
            json_failures: 0,
            tool_failures: 0,
            plan_steps: 2,
            total_iterations: 2,
        };
        assert!(advisor.recommend_upgrade(&metrics).is_none());
    }

    #[test]

    fn test_upgrade_for_complex_task() {
        // Use zero cost sensitivity and lower threshold
        // This simulates prioritizing success over resource cost
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string())
            .with_cost_sensitivity(0.0)
            .with_upgrade_threshold(0.05);
        let metrics = TaskMetrics {
            complexity_score: 0.8,
            json_failures: 5,
            tool_failures: 4,
            plan_steps: 8,
            total_iterations: 10,
        };
        let recommendation = advisor.recommend_upgrade(&metrics);
        assert!(recommendation.is_some());
        
        let rec = recommendation.unwrap();
        assert_eq!(rec.from_model, "qwen2.5:7b-instruct");
        assert_eq!(rec.to_model, "qwen2.5:14b-instruct");
        assert!(rec.confidence > 0.0);
        assert!(rec.utility_gain > 0.0);
    }

    #[test]
    fn test_trigger_conditions() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        
        let metrics_trigger = TaskMetrics {
            complexity_score: 0.0,
            json_failures: 5,
            tool_failures: 0,
            plan_steps: 2,
            total_iterations: 2,
        };
        assert!(advisor.check_triggers(&metrics_trigger));

        let metrics_no_trigger = TaskMetrics {
            complexity_score: 0.0,
            json_failures: 0,
            tool_failures: 0,
            plan_steps: 2,
            total_iterations: 2,
        };
        assert!(!advisor.check_triggers(&metrics_no_trigger));
    }

    #[test]
    fn test_custom_cost_sensitivity() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string())
            .with_cost_sensitivity(0.5);
        assert_eq!(advisor.cost_sensitivity, 0.5);
    }

    #[test]
    fn test_custom_upgrade_threshold() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string())
            .with_upgrade_threshold(0.2);
        assert_eq!(advisor.upgrade_threshold, 0.2);
    }

    #[test]
    fn test_get_model_spec() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        let spec = advisor.get_model_spec(&ModelTier::Small);
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().base_success_rate, 0.70);
    }
}
