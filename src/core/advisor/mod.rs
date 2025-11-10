//! Model Advisor - Statistical decision theory for model recommendations
//! 
//! Implements utility-based decision making for model upgrades

use serde::{Deserialize, Serialize};

/// Model size classifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelSize {
    Small7B,
    Medium14B,
    Large32B,
}

impl ModelSize {
    /// VRAM requirement in GB
    pub fn vram_gb(&self) -> u32 {
        match self {
            Self::Small7B => 4,
            Self::Medium14B => 8,
            Self::Large32B => 16,
        }
    }
    
    /// Base success probability (empirical)
    pub fn base_success_prob(&self) -> f64 {
        match self {
            Self::Small7B => 0.70,
            Self::Medium14B => 0.85,
            Self::Large32B => 0.95,
        }
    }
    
    /// Model tag for Ollama
    pub fn tag(&self) -> &str {
        match self {
            Self::Small7B => "qwen2.5:7b-instruct",
            Self::Medium14B => "qwen2.5:14b-instruct",
            Self::Large32B => "qwen2.5:32b-instruct",
        }
    }
    
    /// Parse from tag string
    pub fn from_tag(tag: &str) -> Self {
        if tag.contains("14b") {
            Self::Medium14B
        } else if tag.contains("32b") {
            Self::Large32B
        } else {
            Self::Small7B
        }
    }
}

/// Task metrics for decision making
#[derive(Debug, Clone, Default)]
pub struct TaskMetrics {
    pub complexity_score: f64,
    pub json_failures: u32,
    pub tool_failures: u32,
    pub plan_steps: usize,
    pub json_success_rate: f64,
}

impl TaskMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn total_failures(&self) -> u32 {
        self.json_failures + self.tool_failures
    }
}

/// Model upgrade recommendation
#[derive(Debug, Clone)]
pub struct ModelUpgrade {
    pub from: ModelSize,
    pub to: ModelSize,
    pub reason: String,
    pub utility_improvement: f64,
    pub confidence: f64,
}

impl ModelUpgrade {
    pub fn format_message(&self) -> String {
        format!(
            "⚠ Model Upgrade Recommended
             
             Current:   {} ({} GB VRAM)
             Suggested: {} ({} GB VRAM)
             
             Reason: {}
             Expected improvement: {:.0}%
             Confidence: {:.0}%
             
             Use: --model {}",
            self.from.tag(),
            self.from.vram_gb(),
            self.to.tag(),
            self.to.vram_gb(),
            self.reason,
            self.utility_improvement * 100.0,
            self.confidence * 100.0,
            self.to.tag()
        )
    }
}

/// Model advisor using statistical decision theory
pub struct ModelAdvisor {
    current_model: ModelSize,
    cost_sensitivity: f64,
    upgrade_threshold: f64,
}

impl ModelAdvisor {
    pub fn new(model_tag: String) -> Self {
        Self {
            current_model: ModelSize::from_tag(&model_tag),
            cost_sensitivity: 0.3,
            upgrade_threshold: 0.05,
        }
    }
    
    /// Calculate utility: U(m, τ) = P(success | m, τ) × (1 - α × Cost(m))
    fn calculate_utility(&self, model: ModelSize, metrics: &TaskMetrics) -> f64 {
        let base_prob = model.base_success_prob();
        
        // Task-specific multipliers
        let complexity_mult = 1.0 - (metrics.complexity_score * 0.3);
        let failure_mult = (1.0 - (metrics.total_failures() as f64 * 0.1)).max(0.5);
        let json_mult = metrics.json_success_rate.max(0.5);
        
        let adjusted_prob = base_prob * complexity_mult * failure_mult * json_mult;
        
        // Cost factor
        let cost = model.vram_gb() as f64 / 16.0;
        
        adjusted_prob * (1.0 - self.cost_sensitivity * cost)
    }
    
    /// Recommend upgrade if utility improvement exceeds threshold
    pub fn recommend_upgrade(&self, metrics: &TaskMetrics) -> Option<ModelUpgrade> {
        if self.current_model == ModelSize::Large32B {
            return None;
        }
        
        let next_model = match self.current_model {
            ModelSize::Small7B => ModelSize::Medium14B,
            ModelSize::Medium14B => ModelSize::Large32B,
            ModelSize::Large32B => return None,
        };
        
        let current_utility = self.calculate_utility(self.current_model, metrics);
        let next_utility = self.calculate_utility(next_model, metrics);
        
        if next_utility > current_utility + self.upgrade_threshold {
            let improvement = next_utility - current_utility;
            let reason = self.build_reason(metrics);
            let confidence = self.calculate_confidence(metrics);
            
            Some(ModelUpgrade {
                from: self.current_model,
                to: next_model,
                reason,
                utility_improvement: improvement,
                confidence,
            })
        } else {
            None
        }
    }
    
    fn build_reason(&self, metrics: &TaskMetrics) -> String {
        let mut reasons = Vec::new();
        
        if metrics.json_failures > 2 {
            reasons.push(format!("JSON failures: {}", metrics.json_failures));
        }
        if metrics.complexity_score > 0.6 {
            reasons.push(format!("High complexity: {:.0}%", metrics.complexity_score * 100.0));
        }
        if metrics.plan_steps > 5 {
            reasons.push(format!("Complex plan: {} steps", metrics.plan_steps));
        }
        if metrics.tool_failures > 3 {
            reasons.push(format!("Tool failures: {}", metrics.tool_failures));
        }
        
        if reasons.is_empty() {
            "Task characteristics suggest larger model".to_string()
        } else {
            reasons.join(", ")
        }
    }
    
    fn calculate_confidence(&self, metrics: &TaskMetrics) -> f64 {
        let mut confidence: f64 = 0.5;
        
        if metrics.json_failures > 2 { confidence += 0.15; }
        if metrics.complexity_score > 0.7 { confidence += 0.15; }
        if metrics.tool_failures > 3 { confidence += 0.10; }
        if metrics.plan_steps > 7 { confidence += 0.10; }
        
        confidence.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_size_properties() {
        assert_eq!(ModelSize::Small7B.vram_gb(), 4);
        assert_eq!(ModelSize::Medium14B.vram_gb(), 8);
        assert_eq!(ModelSize::Large32B.vram_gb(), 16);
    }

    #[test]
    fn test_model_from_tag() {
        assert_eq!(ModelSize::from_tag("qwen2.5:7b-instruct"), ModelSize::Small7B);
        assert_eq!(ModelSize::from_tag("qwen2.5:14b-instruct"), ModelSize::Medium14B);
        assert_eq!(ModelSize::from_tag("qwen2.5:32b-instruct"), ModelSize::Large32B);
    }

    #[test]
    fn test_advisor_creation() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        assert_eq!(advisor.current_model, ModelSize::Small7B);
    }

    #[test]
    fn test_advisor_recommend_runs() {
        let advisor = ModelAdvisor::new("qwen2.5:7b-instruct".to_string());
        let metrics = TaskMetrics::default();
        
        // Just verify it runs without crashing
        let _ = advisor.recommend_upgrade(&metrics);
    }
    
    #[test]
    fn test_advisor_no_upgrade_at_max() {
        let advisor = ModelAdvisor::new("qwen2.5:32b-instruct".to_string());
        let metrics = TaskMetrics {
            complexity_score: 0.9,
            json_failures: 10,
            tool_failures: 10,
            plan_steps: 20,
            json_success_rate: 0.1,
        };
        
        // Already at largest model, no upgrade possible
        assert!(advisor.recommend_upgrade(&metrics).is_none());
    }
    
    #[test]
    fn test_upgrade_message_format() {
        let upgrade = ModelUpgrade {
            from: ModelSize::Small7B,
            to: ModelSize::Medium14B,
            reason: "Test reason".to_string(),
            utility_improvement: 0.2,
            confidence: 0.8,
        };
        
        let msg = upgrade.format_message();
        assert!(msg.contains("qwen2.5:7b-instruct"));
        assert!(msg.contains("qwen2.5:14b-instruct"));
    }
}
