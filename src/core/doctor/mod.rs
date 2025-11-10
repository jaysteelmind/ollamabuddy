//! Doctor command - System health checks

use crate::core::bootstrap::Bootstrap;
use crate::errors::Result;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
}

impl CheckStatus {
    fn symbol(&self) -> &str {
        match self {
            Self::Pass => "✓",
            Self::Warning => "⚠",
            Self::Fail => "✗",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub checks: Vec<HealthCheck>,
}

impl HealthReport {
    pub fn is_healthy(&self) -> bool {
        !self.checks.iter().any(|c| c.status == CheckStatus::Fail)
    }
    
    pub fn print(&self) {
        println!("
╔═══════════════════════════════════════════════════════╗");
        println!("║ OllamaBuddy System Health Check                       ║");
        println!("╚═══════════════════════════════════════════════════════╝
");
        
        for check in &self.checks {
            let symbol = check.status.symbol();
            let latency = check.latency_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            
            println!("  {} {:<20} {}{}", 
                symbol, 
                format!("{}:", check.name),
                check.message,
                latency
            );
        }
        
        println!();
        
        if self.is_healthy() {
            println!("  ✓ All checks passed - System is healthy
");
        } else {
            println!("  ✗ Some checks failed - Run install script or fix manually
");
        }
    }
}

pub struct Doctor {
    bootstrap: Bootstrap,
}

impl Doctor {
    pub fn new(host: String, port: u16, model: String) -> Self {
        Self {
            bootstrap: Bootstrap::new(host, port, model),
        }
    }

    pub async fn run_checks(&self) -> Result<HealthReport> {
        let mut checks = Vec::new();

        checks.push(self.check_ollama_api().await);
        checks.push(self.check_model().await);
        checks.push(self.check_disk_space());
        checks.push(self.check_cwd_writable());
        checks.push(self.check_network().await);

        Ok(HealthReport { checks })
    }

    async fn check_ollama_api(&self) -> HealthCheck {
        let start = std::time::Instant::now();
        
        match self.bootstrap.is_ollama_running().await {
            Ok(true) => {
                let latency = start.elapsed().as_millis() as u64;
                
                let version = self.bootstrap.get_version().await
                    .unwrap_or_else(|_| "unknown".to_string());
                
                HealthCheck {
                    name: "Ollama API".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("Running (v{})", version),
                    latency_ms: Some(latency),
                }
            }
            _ => {
                HealthCheck {
                    name: "Ollama API".to_string(),
                    status: CheckStatus::Fail,
                    message: "Not reachable - Start with: ollama serve".to_string(),
                    latency_ms: None,
                }
            }
        }
    }

    async fn check_model(&self) -> HealthCheck {
        match self.bootstrap.is_model_available().await {
            Ok(true) => {
                HealthCheck {
                    name: "Model".to_string(),
                    status: CheckStatus::Pass,
                    message: "Available".to_string(),
                    latency_ms: None,
                }
            }
            Ok(false) => {
                HealthCheck {
                    name: "Model".to_string(),
                    status: CheckStatus::Warning,
                    message: "Not found - Will auto-pull on first run".to_string(),
                    latency_ms: None,
                }
            }
            Err(_) => {
                HealthCheck {
                    name: "Model".to_string(),
                    status: CheckStatus::Fail,
                    message: "Could not check".to_string(),
                    latency_ms: None,
                }
            }
        }
    }

    fn check_disk_space(&self) -> HealthCheck {
        use sysinfo::Disks;
        
        let disks = Disks::new_with_refreshed_list();

        if let Some(disk) = disks.iter().next() {
            let available_gb = disk.available_space() / 1_000_000_000;
            
            if available_gb >= 5 {
                HealthCheck {
                    name: "Disk Space".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("{} GB available", available_gb),
                    latency_ms: None,
                }
            } else {
                HealthCheck {
                    name: "Disk Space".to_string(),
                    status: CheckStatus::Warning,
                    message: format!("Low: {} GB (recommend 5GB+)", available_gb),
                    latency_ms: None,
                }
            }
        } else {
            HealthCheck {
                name: "Disk Space".to_string(),
                status: CheckStatus::Warning,
                message: "Could not determine".to_string(),
                latency_ms: None,
            }
        }
    }

    fn check_cwd_writable(&self) -> HealthCheck {
        let test_file = Path::new(".ollamabuddy_test");
        
        match std::fs::write(test_file, "test") {
            Ok(_) => {
                std::fs::remove_file(test_file).ok();
                HealthCheck {
                    name: "Working Directory".to_string(),
                    status: CheckStatus::Pass,
                    message: "Writable".to_string(),
                    latency_ms: None,
                }
            }
            Err(e) => {
                HealthCheck {
                    name: "Working Directory".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("Not writable: {}", e),
                    latency_ms: None,
                }
            }
        }
    }

    async fn check_network(&self) -> HealthCheck {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap();
        
        match client.get("https://ollama.com").send().await {
            Ok(_) => {
                HealthCheck {
                    name: "Network".to_string(),
                    status: CheckStatus::Pass,
                    message: "Online".to_string(),
                    latency_ms: None,
                }
            }
            Err(_) => {
                HealthCheck {
                    name: "Network".to_string(),
                    status: CheckStatus::Warning,
                    message: "Offline (web_fetch unavailable)".to_string(),
                    latency_ms: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_symbols() {
        assert_eq!(CheckStatus::Pass.symbol(), "✓");
        assert_eq!(CheckStatus::Warning.symbol(), "⚠");
        assert_eq!(CheckStatus::Fail.symbol(), "✗");
    }

    #[test]
    fn test_health_report_healthy() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "Test".to_string(),
                    status: CheckStatus::Pass,
                    message: "OK".to_string(),
                    latency_ms: None,
                },
            ],
        };
        
        assert!(report.is_healthy());
    }

    #[test]
    fn test_health_report_unhealthy() {
        let report = HealthReport {
            checks: vec![
                HealthCheck {
                    name: "Test".to_string(),
                    status: CheckStatus::Fail,
                    message: "Failed".to_string(),
                    latency_ms: None,
                },
            ],
        };
        
        assert!(!report.is_healthy());
    }
}
