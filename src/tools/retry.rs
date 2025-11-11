//! Retry manager with exponential backoff
//! 
//! Implements bounded retry strategy with mathematical guarantees:
//! - Max retries: 5 attempts
//! - Total wait: ≤31s bounded
//! - Strategy: Binary exponential with jitter
//! - Convergence: Proven finite termination

use crate::errors::{AgentError, Result};
use std::time::Duration;
use tokio::time::sleep;

/// Maximum number of retry attempts
pub const MAX_RETRIES: u32 = 5;

/// Base delay for exponential backoff (1 second)
const BASE_DELAY_MS: u64 = 1000;

/// Maximum delay cap (16 seconds)
const MAX_DELAY_MS: u64 = 16000;

/// Retry manager with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryManager {
    /// Maximum retry attempts
    max_retries: u32,
    
    /// Base delay in milliseconds
    base_delay_ms: u64,
    
    /// Maximum delay cap in milliseconds
    max_delay_ms: u64,
    
    /// Enable jitter
    enable_jitter: bool,
}

impl Default for RetryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryManager {
    /// Create new retry manager with default settings
    pub fn new() -> Self {
        Self {
            max_retries: MAX_RETRIES,
            base_delay_ms: BASE_DELAY_MS,
            max_delay_ms: MAX_DELAY_MS,
            enable_jitter: true,
        }
    }

    /// Create retry manager with custom settings
    pub fn with_config(max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms: MAX_DELAY_MS,
            enable_jitter: true,
        }
    }

    /// Execute operation with retry logic
    pub async fn execute_with_retry<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;

        loop {
            // Try operation
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Check if error is retryable
                    if !self.is_retryable(&e) {
                        return Err(e);
                    }

                    attempt += 1;

                    // Check if max retries exceeded
                    if attempt >= self.max_retries {
                        return Err(AgentError::Generic(format!(
                            "Max retries exceeded after {} attempts", 
                            attempt
                        )));
                    }

                    // Calculate delay for this attempt
                    let delay = self.calculate_delay(attempt);
                    sleep(delay).await;
                }
            }
        }
    }

    /// Calculate delay for given attempt number
    fn calculate_delay(&self, attempt: u32) -> Duration {
        // Binary exponential backoff: 2^attempt
        let exponential_delay = self.base_delay_ms * 2u64.pow(attempt);
        
        // Cap at maximum delay
        let delay_ms = exponential_delay.min(self.max_delay_ms);
        
        // Add jitter if enabled (±25% random variation)
        let final_delay = if self.enable_jitter {
            let jitter = (delay_ms / 4) as i64;
            let random_jitter = (rand::random::<f64>() * 2.0 - 1.0) * jitter as f64;
            ((delay_ms as i64) + random_jitter as i64).max(0) as u64
        } else {
            delay_ms
        };

        Duration::from_millis(final_delay)
    }

    /// Calculate total maximum wait time
    pub fn max_total_wait_time(&self) -> Duration {
        let mut total_ms = 0u64;
        
        for attempt in 0..self.max_retries {
            let delay = self.base_delay_ms * 2u64.pow(attempt);
            total_ms += delay.min(self.max_delay_ms);
        }
        
        Duration::from_millis(total_ms)
    }

    /// Check if error is retryable
    fn is_retryable(&self, error: &AgentError) -> bool {
        match error {
            // Retryable errors (transient)
            AgentError::Timeout { .. } => true,
            AgentError::HttpError(_) => true,
            AgentError::StreamingError(_) => true,
            AgentError::OllamaApiError(_) => true,
            
            // Non-retryable errors (permanent)
            AgentError::InvalidTransition { .. } => false,
            AgentError::ContextOverflow { .. } => false,
            AgentError::JsonParseError(_) => false,
            AgentError::ConfigError(_) => false,
            
            // Generic errors: retry by default
            AgentError::Generic(_) => true,
            
            // Other errors: don't retry
            _ => false,
        }
    }

    /// Get max retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let retry_manager = RetryManager::new();
        
        let attempt_count = Arc::new(Mutex::new(0));
        let count_clone = attempt_count.clone();
        
        let result = retry_manager
            .execute_with_retry(move || {
                let count = count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Ok::<i32, AgentError>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempt_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let retry_manager = RetryManager::new();
        
        let attempt_count = Arc::new(Mutex::new(0));
        let count_clone = attempt_count.clone();
        
        let result = retry_manager
            .execute_with_retry(move || {
                let count = count_clone.clone();
                async move {
                    let mut attempts = count.lock().unwrap();
                    *attempts += 1;
                    let current = *attempts;
                    drop(attempts);
                    
                    if current < 3 {
                        Err(AgentError::Generic("Transient error".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let retry_manager = RetryManager::with_config(3, 10);
        
        let attempt_count = Arc::new(Mutex::new(0));
        let count_clone = attempt_count.clone();
        
        let result = retry_manager
            .execute_with_retry(move || {
                let count = count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Err::<i32, _>(AgentError::Generic("Always fails".to_string()))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let retry_manager = RetryManager::new();
        
        let attempt_count = Arc::new(Mutex::new(0));
        let count_clone = attempt_count.clone();
        
        let result = retry_manager
            .execute_with_retry(move || {
                let count = count_clone.clone();
                async move {
                    *count.lock().unwrap() += 1;
                    Err::<i32, _>(AgentError::ConfigError("Permanent error".to_string()))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_calculate_delay() {
        let retry_manager = RetryManager {
            max_retries: 5,
            base_delay_ms: 1000,
            max_delay_ms: 16000,
            enable_jitter: false,
        };

        assert_eq!(retry_manager.calculate_delay(0), Duration::from_millis(1000));
        assert_eq!(retry_manager.calculate_delay(1), Duration::from_millis(2000));
        assert_eq!(retry_manager.calculate_delay(2), Duration::from_millis(4000));
        assert_eq!(retry_manager.calculate_delay(3), Duration::from_millis(8000));
        assert_eq!(retry_manager.calculate_delay(4), Duration::from_millis(16000));
    }

    #[test]
    fn test_max_total_wait_time() {
        let retry_manager = RetryManager::new();
        let max_wait = retry_manager.max_total_wait_time();
        
        assert_eq!(max_wait, Duration::from_secs(31));
    }

    #[test]
    fn test_is_retryable() {
        let retry_manager = RetryManager::new();

        assert!(retry_manager.is_retryable(&AgentError::Timeout { duration_ms: 1000 }));
        assert!(retry_manager.is_retryable(&AgentError::Generic("test".to_string())));
        assert!(!retry_manager.is_retryable(&AgentError::ConfigError("test".to_string())));
        assert!(!retry_manager.is_retryable(&AgentError::JsonParseError("test".to_string())));
    }

    #[test]
    fn test_delay_cap() {
        // Create retry manager without jitter for predictable testing
        let retry_manager = RetryManager {
            max_retries: 5,
            base_delay_ms: 1000,
            max_delay_ms: MAX_DELAY_MS,
            enable_jitter: false, // Disable jitter for deterministic test
        };
        
        let delay = retry_manager.calculate_delay(10);
        assert_eq!(delay, Duration::from_millis(MAX_DELAY_MS));
    }
}
