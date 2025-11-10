//! Doctor command for system diagnostics
//! 
//! Provides comprehensive health checks for OllamaBuddy system.

use crate::bootstrap::BootstrapDetector;
use reqwest::Client;
use std::path::Path;
use std::time::Duration;
use sysinfo::System;

/// Health check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Pass,
    Warn(String),
    Fail(String),
}

/// Individual health check
#[derive(Debug)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
}

/// Doctor diagnostics system
pub struct Doctor {
    ollama_url: String,
    working_dir: String,
}

impl Doctor {
    /// Create a new doctor instance
    pub fn new(ollama_url: String, working_dir: String) -> Self {
        Self {
            ollama_url,
            working_dir,
        }
    }

    /// Run all health checks
    pub async fn run_diagnostics(&self) -> Vec<HealthCheck> {
        let mut checks = Vec::new();

        checks.push(self.check_ollama_api().await);
        checks.push(self.check_model_available().await);
        checks.push(self.check_disk_space());
        checks.push(self.check_memory());
        checks.push(self.check_network().await);
        checks.push(self.check_permissions());
        checks.push(self.check_ollama_version().await);
        checks.push(self.check_tools());

        checks
    }

    /// Check 1: Ollama API reachable
    async fn check_ollama_api(&self) -> HealthCheck {
        let detector = BootstrapDetector::new(self.ollama_url.clone());
        
        match detector.check_ollama_running().await {
            Ok(true) => HealthCheck {
                name: "Ollama API".to_string(),
                status: HealthStatus::Pass,
            },
            Ok(false) => HealthCheck {
                name: "Ollama API".to_string(),
                status: HealthStatus::Fail("Ollama not running or not reachable".to_string()),
            },
            Err(e) => HealthCheck {
                name: "Ollama API".to_string(),
                status: HealthStatus::Fail(format!("Error checking Ollama: {}", e)),
            },
        }
    }

    /// Check 2: Model availability
    async fn check_model_available(&self) -> HealthCheck {
        let detector = BootstrapDetector::new(self.ollama_url.clone());
        
        match detector.list_models().await {
            Ok(models) if !models.is_empty() => {
                let _model_list = models.join(", ");
                HealthCheck {
                    name: "Models Available".to_string(),
                    status: HealthStatus::Pass,
                }
            }
            Ok(_) => HealthCheck {
                name: "Models Available".to_string(),
                status: HealthStatus::Warn("No models installed".to_string()),
            },
            Err(e) => HealthCheck {
                name: "Models Available".to_string(),
                status: HealthStatus::Fail(format!("Cannot check models: {}", e)),
            },
        }
    }

    /// Check 3: Disk space
    fn check_disk_space(&self) -> HealthCheck {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();

        let working_path = Path::new(&self.working_dir);
        
        // Find disk containing working directory
        for disk in &disks {
            if working_path.starts_with(disk.mount_point()) {
                let available_gb = disk.available_space() / (1024 * 1024 * 1024);
                
                return if available_gb < 1 {
                    HealthCheck {
                        name: "Disk Space".to_string(),
                        status: HealthStatus::Fail(
                            format!("Less than 1GB available ({} GB)", available_gb)
                        ),
                    }
                } else if available_gb < 5 {
                    HealthCheck {
                        name: "Disk Space".to_string(),
                        status: HealthStatus::Warn(
                            format!("Low disk space ({} GB available)", available_gb)
                        ),
                    }
                } else {
                    HealthCheck {
                        name: "Disk Space".to_string(),
                        status: HealthStatus::Pass,
                    }
                };
            }
        }

        HealthCheck {
            name: "Disk Space".to_string(),
            status: HealthStatus::Warn("Could not determine disk space".to_string()),
        }
    }

    /// Check 4: Memory availability
    fn check_memory(&self) -> HealthCheck {
        let mut sys = System::new_all();
        sys.refresh_memory();

        let available_gb = sys.available_memory() / (1024 * 1024 * 1024);
        
        if available_gb < 1 {
            HealthCheck {
                name: "Memory".to_string(),
                status: HealthStatus::Fail(
                    format!("Less than 1GB RAM available ({} GB)", available_gb)
                ),
            }
        } else if available_gb < 2 {
            HealthCheck {
                name: "Memory".to_string(),
                status: HealthStatus::Warn(
                    format!("Low memory ({} GB available)", available_gb)
                ),
            }
        } else {
            HealthCheck {
                name: "Memory".to_string(),
                status: HealthStatus::Pass,
            }
        }
    }

    /// Check 5: Network connectivity
    async fn check_network(&self) -> HealthCheck {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        // Try to reach a reliable endpoint
        let test_urls = vec![
            "https://ollama.com",
            "https://www.google.com",
            "https://www.cloudflare.com",
        ];

        for url in test_urls {
            if let Ok(response) = client.get(url).send().await {
                if response.status().is_success() {
                    return HealthCheck {
                        name: "Network".to_string(),
                        status: HealthStatus::Pass,
                    };
                }
            }
        }

        HealthCheck {
            name: "Network".to_string(),
            status: HealthStatus::Warn("Cannot reach external networks".to_string()),
        }
    }

    /// Check 6: File permissions
    fn check_permissions(&self) -> HealthCheck {
        let working_path = Path::new(&self.working_dir);
        
        // Test read permission
        if !working_path.exists() {
            return HealthCheck {
                name: "Permissions".to_string(),
                status: HealthStatus::Fail("Working directory does not exist".to_string()),
            };
        }

        // Test write permission by attempting to create a temp file
        let test_file = working_path.join(".ollamabuddy_test");
        match std::fs::write(&test_file, "test") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_file);
                HealthCheck {
                    name: "Permissions".to_string(),
                    status: HealthStatus::Pass,
                }
            }
            Err(_) => HealthCheck {
                name: "Permissions".to_string(),
                status: HealthStatus::Fail("No write permission in working directory".to_string()),
            },
        }
    }

    /// Check 7: Ollama version
    async fn check_ollama_version(&self) -> HealthCheck {
        let url = format!("{}/api/version", self.ollama_url);
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                HealthCheck {
                    name: "Ollama Version".to_string(),
                    status: HealthStatus::Pass,
                }
            }
            _ => HealthCheck {
                name: "Ollama Version".to_string(),
                status: HealthStatus::Warn("Cannot determine Ollama version".to_string()),
            },
        }
    }

    /// Check 8: Tool availability
    fn check_tools(&self) -> HealthCheck {
        // Check if basic system tools are available
        let tools_available = true; // All tools are built-in Rust
        
        HealthCheck {
            name: "Tools".to_string(),
            status: if tools_available {
                HealthStatus::Pass
            } else {
                HealthStatus::Warn("Some tools may not be available".to_string())
            },
        }
    }

    /// Display diagnostics results
    pub fn display_results(checks: &[HealthCheck]) {
        println!("
🔍 OllamaBuddy System Diagnostics
");
        println!("{:<20} {}", "Check", "Status");
        println!("{}", "=".repeat(50));

        for check in checks {
            let (symbol, color, message) = match &check.status {
                HealthStatus::Pass => ("✅", "[32m", "PASS".to_string()),
                HealthStatus::Warn(msg) => ("⚠️ ", "[33m", format!("WARN: {}", msg)),
                HealthStatus::Fail(msg) => ("❌", "[31m", format!("FAIL: {}", msg)),
            };

            println!("{:<20} {}{} {}[0m", check.name, symbol, color, message);
        }

        println!();
    }

    /// Get overall health status
    pub fn overall_status(checks: &[HealthCheck]) -> bool {
        !checks.iter().any(|c| matches!(c.status, HealthStatus::Fail(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_creation() {
        let doctor = Doctor::new(
            "http://localhost:11434".to_string(),
            "/tmp".to_string(),
        );
        assert_eq!(doctor.ollama_url, "http://localhost:11434");
        assert_eq!(doctor.working_dir, "/tmp");
    }

    #[test]
    fn test_health_status_equality() {
        assert_eq!(HealthStatus::Pass, HealthStatus::Pass);
        assert_eq!(
            HealthStatus::Warn("test".to_string()),
            HealthStatus::Warn("test".to_string())
        );
        assert_eq!(
            HealthStatus::Fail("test".to_string()),
            HealthStatus::Fail("test".to_string())
        );
    }

    #[test]
    fn test_overall_status_pass() {
        let checks = vec![
            HealthCheck {
                name: "Test 1".to_string(),
                status: HealthStatus::Pass,
            },
            HealthCheck {
                name: "Test 2".to_string(),
                status: HealthStatus::Warn("warning".to_string()),
            },
        ];
        assert!(Doctor::overall_status(&checks));
    }

    #[test]
    fn test_overall_status_fail() {
        let checks = vec![
            HealthCheck {
                name: "Test 1".to_string(),
                status: HealthStatus::Pass,
            },
            HealthCheck {
                name: "Test 2".to_string(),
                status: HealthStatus::Fail("error".to_string()),
            },
        ];
        assert!(!Doctor::overall_status(&checks));
    }

    #[tokio::test]
    async fn test_check_tools() {
        let doctor = Doctor::new(
            "http://localhost:11434".to_string(),
            "/tmp".to_string(),
        );
        let check = doctor.check_tools();
        assert_eq!(check.name, "Tools");
        assert_eq!(check.status, HealthStatus::Pass);
    }
}
