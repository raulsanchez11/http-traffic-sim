use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Per-target port discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDiscoveryConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub mode: DiscoveryMode,

    pub ports: PortSpec,

    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    #[serde(default = "default_retries")]
    pub retries: u8,

    #[serde(default)]
    pub on_failure: FailureAction,

    #[serde(default = "default_true")]
    pub detect_service: bool,

    #[serde(default = "default_true")]
    pub validate_http: bool,
}

impl Default for PortDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DiscoveryMode::Validate,
            ports: PortSpec::Single(80),
            timeout_ms: default_timeout_ms(),
            retries: default_retries(),
            on_failure: FailureAction::Fail,
            detect_service: true,
            validate_http: true,
        }
    }
}

fn default_timeout_ms() -> u64 {
    2000
}

fn default_retries() -> u8 {
    2
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DiscoveryMode {
    #[default]
    Validate, // Only validate explicit ports
    Scan,     // Scan port ranges
    Both,     // Validate + scan
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PortSpec {
    Single(u16),
    List(Vec<u16>),
    Range { start: u16, end: u16 },
}

impl PortSpec {
    pub fn to_vec(&self) -> Vec<u16> {
        match self {
            PortSpec::Single(port) => vec![*port],
            PortSpec::List(ports) => ports.clone(),
            PortSpec::Range { start, end } => (*start..=*end).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum FailureAction {
    #[default]
    Fail, // Stop execution with error
    Skip, // Continue with reachable targets only
    Warn, // Log warning but continue with all
}


/// Discovery results for a target
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryResult {
    pub target_id: String,
    pub host: String,
    pub discovered_ports: Vec<PortInfo>,
    pub failed_ports: Vec<PortFailure>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub status: PortStatus,
    pub service_type: Option<ServiceType>,
    pub response_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    Open,
    Closed,
    #[allow(dead_code)]
    Filtered,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Http,
    Https,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortFailure {
    pub port: u16,
    pub error: String,
}

/// Check TCP connectivity to a port
async fn check_tcp_port(
    host: &str,
    port: u16,
    timeout_duration: Duration,
    retries: u8,
) -> Result<PortCheckResult> {
    let mut last_error = None;

    for attempt in 0..=retries {
        if attempt > 0 {
            // Exponential backoff
            tokio::time::sleep(Duration::from_millis(100 * (1 << (attempt - 1)))).await;
        }

        let start = std::time::Instant::now();

        // Resolve address
        let addr = match format!("{}:{}", host, port).to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => addr,
                None => {
                    last_error = Some(anyhow::anyhow!("No addresses resolved"));
                    continue;
                }
            },
            Err(e) => {
                last_error = Some(anyhow::anyhow!("DNS resolution failed: {}", e));
                continue;
            }
        };

        // Try to connect
        match timeout(timeout_duration, TcpStream::connect(addr)).await {
            Ok(Ok(_stream)) => {
                let elapsed = start.elapsed();
                return Ok(PortCheckResult {
                    status: PortStatus::Open,
                    response_time: elapsed,
                });
            }
            Ok(Err(e)) => {
                if e.kind() == std::io::ErrorKind::ConnectionRefused {
                    return Ok(PortCheckResult {
                        status: PortStatus::Closed,
                        response_time: start.elapsed(),
                    });
                }
                last_error = Some(anyhow::anyhow!("Connection error: {}", e));
            }
            Err(_) => {
                last_error = Some(anyhow::anyhow!("Connection timeout"));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
}

struct PortCheckResult {
    status: PortStatus,
    response_time: Duration,
}

/// Detect HTTP/HTTPS service
async fn detect_http_service(
    host: &str,
    port: u16,
    timeout_duration: Duration,
) -> Option<ServiceType> {
    // Build HTTP client for discovery with relaxed security
    let client = match reqwest::Client::builder()
        .timeout(timeout_duration)
        .danger_accept_invalid_certs(true) // For discovery only
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    // Try HTTPS first
    let https_url = format!("https://{}:{}/", host, port);
    if let Ok(resp) = client.get(&https_url).send().await {
        if resp.status().is_success()
            || resp.status().is_redirection()
            || resp.status().is_client_error()
        {
            return Some(ServiceType::Https);
        }
    }

    // Try HTTP
    let http_url = format!("http://{}:{}/", host, port);
    if let Ok(resp) = client.get(&http_url).send().await {
        if resp.status().is_success()
            || resp.status().is_redirection()
            || resp.status().is_client_error()
        {
            return Some(ServiceType::Http);
        }
    }

    Some(ServiceType::Unknown)
}

/// Scan ports with concurrency control
async fn scan_ports(host: &str, ports: &[u16], config: &PortDiscoveryConfig) -> Result<ScanResult> {
    let timeout_duration = Duration::from_millis(config.timeout_ms);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(10)); // Max 10 concurrent scans

    let mut tasks = Vec::new();

    for &port in ports {
        let host = host.to_string();
        let config = config.clone();
        let sem = semaphore.clone();

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // Check TCP connectivity
            let check_result =
                match check_tcp_port(&host, port, timeout_duration, config.retries).await {
                    Ok(result) => result,
                    Err(e) => {
                        return ScanPortResult::Failed(PortFailure {
                            port,
                            error: e.to_string(),
                        });
                    }
                };

            if check_result.status != PortStatus::Open {
                return ScanPortResult::Failed(PortFailure {
                    port,
                    error: "Port closed or filtered".to_string(),
                });
            }

            // Detect service type if enabled
            let service_type = if config.detect_service && config.validate_http {
                detect_http_service(&host, port, timeout_duration).await
            } else {
                None
            };

            ScanPortResult::Success(PortInfo {
                port,
                status: check_result.status,
                service_type,
                response_time_ms: check_result.response_time.as_secs_f64() * 1000.0,
            })
        });

        tasks.push(task);
    }

    let mut discovered = Vec::new();
    let mut failed = Vec::new();

    for task in tasks {
        match task.await {
            Ok(ScanPortResult::Success(info)) => discovered.push(info),
            Ok(ScanPortResult::Failed(failure)) => failed.push(failure),
            Err(_) => {} // Task panicked, ignore
        }
    }

    Ok(ScanResult { discovered, failed })
}

enum ScanPortResult {
    Success(PortInfo),
    Failed(PortFailure),
}

struct ScanResult {
    discovered: Vec<PortInfo>,
    failed: Vec<PortFailure>,
}

/// Discover ports for multiple targets in parallel
pub async fn discover_targets(
    targets: &[(String, String, PortDiscoveryConfig)], // (id, host, config)
) -> Result<Vec<DiscoveryResult>> {
    let mut tasks = Vec::new();

    for (target_id, host, config) in targets {
        let target_id = target_id.clone();
        let host = host.clone();
        let config = config.clone();

        let task =
            tokio::spawn(async move { discover_single_target(&target_id, &host, &config).await });

        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => {
                tracing::warn!("Discovery task failed: {}", e);
            }
            Err(e) => {
                tracing::warn!("Discovery task panicked: {}", e);
            }
        }
    }

    Ok(results)
}

/// Discover ports for a single target
async fn discover_single_target(
    target_id: &str,
    host: &str,
    config: &PortDiscoveryConfig,
) -> Result<DiscoveryResult> {
    let start = std::time::Instant::now();

    let ports = config.ports.to_vec();
    let scan_result = scan_ports(host, &ports, config).await?;

    let duration = start.elapsed();

    Ok(DiscoveryResult {
        target_id: target_id.to_string(),
        host: host.to_string(),
        discovered_ports: scan_result.discovered,
        failed_ports: scan_result.failed,
        duration,
    })
}

/// Extract host from URL
pub fn extract_host_from_url(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url).with_context(|| format!("Invalid URL: {}", url))?;

    parsed
        .host_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("URL has no host: {}", url))
}

/// Extract port from URL or use default
#[allow(dead_code)]
pub fn extract_port_from_url(url: &str) -> Result<u16> {
    let parsed = url::Url::parse(url).with_context(|| format!("Invalid URL: {}", url))?;

    if let Some(port) = parsed.port() {
        return Ok(port);
    }

    // Default ports based on scheme
    match parsed.scheme() {
        "http" => Ok(80),
        "https" => Ok(443),
        _ => Err(anyhow::anyhow!(
            "Cannot determine default port for scheme: {}",
            parsed.scheme()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_spec_single() {
        let spec = PortSpec::Single(8080);
        assert_eq!(spec.to_vec(), vec![8080]);
    }

    #[test]
    fn test_port_spec_list() {
        let spec = PortSpec::List(vec![80, 443, 8080]);
        assert_eq!(spec.to_vec(), vec![80, 443, 8080]);
    }

    #[test]
    fn test_port_spec_range() {
        let spec = PortSpec::Range {
            start: 8000,
            end: 8003,
        };
        assert_eq!(spec.to_vec(), vec![8000, 8001, 8002, 8003]);
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(
            extract_host_from_url("https://example.com/path").unwrap(),
            "example.com"
        );
        assert_eq!(
            extract_host_from_url("http://api.example.com:8080/").unwrap(),
            "api.example.com"
        );
    }

    #[test]
    fn test_extract_port() {
        assert_eq!(
            extract_port_from_url("https://example.com/path").unwrap(),
            443
        );
        assert_eq!(
            extract_port_from_url("http://example.com/path").unwrap(),
            80
        );
        assert_eq!(
            extract_port_from_url("https://example.com:8443/path").unwrap(),
            8443
        );
    }
}
