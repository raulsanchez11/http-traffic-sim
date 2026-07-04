//! Port discovery module for validating connectivity and discovering services.
//!
//! This module provides functionality to discover and validate ports before
//! running load tests. It supports:
//!
//! - TCP connectivity validation
//! - HTTP/HTTPS service detection
//! - Port range scanning
//! - Concurrent multi-target discovery
//! - Configurable failure handling
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::discovery::{
//!     PortDiscoveryConfig, DiscoveryMode, PortSpec, FailureAction
//! };
//!
//! // Create a discovery configuration
//! let config = PortDiscoveryConfig {
//!     enabled: true,
//!     mode: DiscoveryMode::Validate,
//!     ports: PortSpec::Single(443),
//!     timeout_ms: 2000,
//!     retries: 2,
//!     on_failure: FailureAction::Fail,
//!     detect_service: true,
//!     validate_http: true,
//! };
//! ```

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Configuration for port discovery on a target.
///
/// Port discovery validates connectivity and discovers available services before
/// running load tests. This helps catch configuration errors early and enables
/// automatic service detection.
///
/// # Examples
///
/// ```
/// use http_traffic_sim::discovery::{PortDiscoveryConfig, DiscoveryMode, PortSpec, FailureAction};
///
/// // Validate a specific port
/// let config = PortDiscoveryConfig {
///     enabled: true,
///     mode: DiscoveryMode::Validate,
///     ports: PortSpec::Single(443),
///     timeout_ms: 2000,
///     retries: 2,
///     on_failure: FailureAction::Fail,
///     detect_service: true,
///     validate_http: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDiscoveryConfig {
    /// Enable port discovery for this target
    #[serde(default)]
    pub enabled: bool,

    /// Discovery mode: validate, scan, or both
    #[serde(default)]
    pub mode: DiscoveryMode,

    /// Port specification: single port, list, or range
    pub ports: PortSpec,

    /// Timeout per port check in milliseconds (default: 2000)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Number of retry attempts for failed checks (default: 2)
    #[serde(default = "default_retries")]
    pub retries: u8,

    /// Action to take on discovery failure
    #[serde(default)]
    pub on_failure: FailureAction,

    /// Enable HTTP/HTTPS service type detection (default: true)
    #[serde(default = "default_true")]
    pub detect_service: bool,

    /// Validate HTTP responses during detection (default: true)
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

/// Port discovery mode.
///
/// Determines how ports are discovered for a target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DiscoveryMode {
    /// Only validate explicitly specified ports
    #[default]
    Validate,
    /// Scan port ranges to discover available services
    Scan,
    /// Both validate explicit ports and scan ranges
    Both,
}

/// Port specification for discovery.
///
/// Specifies which ports to check during discovery.
///
/// # Examples
///
/// ```
/// use http_traffic_sim::discovery::PortSpec;
///
/// // Single port
/// let single = PortSpec::Single(8080);
///
/// // Multiple ports
/// let list = PortSpec::List(vec![80, 443, 8080]);
///
/// // Port range
/// let range = PortSpec::Range { start: 8000, end: 9000 };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PortSpec {
    /// Single port to check
    Single(u16),
    /// List of specific ports
    List(Vec<u16>),
    /// Range of ports (inclusive)
    Range {
        /// Starting port (inclusive)
        start: u16,
        /// Ending port (inclusive)
        end: u16,
    },
}

/// Maximum inclusive port range width allowed during discovery.
pub const MAX_PORT_RANGE_SPAN: u16 = 1024;

impl PortSpec {
    pub fn to_vec(&self) -> Result<Vec<u16>> {
        match self {
            PortSpec::Single(port) => Ok(vec![*port]),
            PortSpec::List(ports) => Ok(ports.clone()),
            PortSpec::Range { start, end } => {
                if end < start {
                    bail!("Port range start ({start}) must be <= end ({end})");
                }
                let span = (*end as u32).saturating_sub(*start as u32) + 1;
                if span > MAX_PORT_RANGE_SPAN as u32 {
                    bail!(
                        "Port range spans {span} ports; maximum allowed is {MAX_PORT_RANGE_SPAN}"
                    );
                }
                Ok((*start..=*end).collect())
            }
        }
    }
}

fn resolve_ports(config: &PortDiscoveryConfig) -> Result<Vec<u16>> {
    let mut ports = config.ports.to_vec()?;
    if config.mode == DiscoveryMode::Both {
        for well_known in [80u16, 443] {
            if !ports.contains(&well_known) {
                ports.push(well_known);
            }
        }
        ports.sort_unstable();
        ports.dedup();
    }
    Ok(ports)
}

fn should_probe_http(config: &PortDiscoveryConfig) -> bool {
    matches!(config.mode, DiscoveryMode::Scan | DiscoveryMode::Both)
        && config.detect_service
        && config.validate_http
}

/// Action to take when port discovery fails.
///
/// Determines how the load tester should proceed when discovery
/// encounters unreachable ports or services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum FailureAction {
    /// Stop execution with an error (default)
    #[default]
    Fail,
    /// Continue with only reachable targets
    Skip,
    /// Log warning but continue with all targets
    Warn,
}

/// Results from port discovery for a single target.
///
/// Contains information about which ports were successfully discovered,
/// which failed, and timing information.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryResult {
    /// Target identifier
    pub target_id: String,
    /// Target hostname
    pub host: String,
    /// Successfully discovered ports with their information
    pub discovered_ports: Vec<PortInfo>,
    /// Ports that failed discovery with error messages
    pub failed_ports: Vec<PortFailure>,
    /// Total duration of the discovery process
    pub duration: Duration,
}

/// Information about a discovered port.
///
/// Contains the port number, its status, detected service type,
/// and response time from the connectivity check.
#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    /// Port number
    pub port: u16,
    /// Port status (Open, Closed, or Filtered)
    pub status: PortStatus,
    /// Detected service type (HTTP, HTTPS, or Unknown)
    pub service_type: Option<ServiceType>,
    /// Response time in milliseconds
    pub response_time_ms: f64,
}

/// Status of a port during discovery.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    /// Port is open and accepting connections
    Open,
    /// Port is closed or refusing connections
    Closed,
    /// Port is filtered (no response, possibly firewall)
    #[allow(dead_code)] // kept for completeness (not currently constructed)
    Filtered,
}

/// Type of service detected on a port.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    /// HTTP service detected
    Http,
    /// HTTPS service detected
    Https,
    /// Service type could not be determined
    Unknown,
}

/// Information about a port that failed discovery.
#[derive(Debug, Clone, Serialize)]
pub struct PortFailure {
    /// Port number that failed
    pub port: u16,
    /// Error message describing the failure
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

            let service_type = if should_probe_http(&config) {
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

    let ports = resolve_ports(config)?;
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
#[allow(dead_code)] // kept as utility (unused in main code paths; tested in unit tests)
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

pub fn display_results(results: &[DiscoveryResult]) {
    for result in results {
        println!("Target: {} ({})", result.target_id, result.host);
        println!("Discovery Duration: {:.2}s", result.duration.as_secs_f64());

        if !result.discovered_ports.is_empty() {
            println!("\n  Open Ports:");
            for port_info in &result.discovered_ports {
                let service = match &port_info.service_type {
                    Some(ServiceType::Http) => " [HTTP]",
                    Some(ServiceType::Https) => " [HTTPS]",
                    Some(ServiceType::Unknown) => " [Unknown]",
                    None => "",
                };
                println!(
                    "    - Port {}{} - {:.2}ms response",
                    port_info.port, service, port_info.response_time_ms
                );
            }
        }

        if !result.failed_ports.is_empty() {
            println!("\n  Failed Ports:");
            for failure in &result.failed_ports {
                println!("    - Port {}: {}", failure.port, failure.error);
            }
        }

        println!();
    }

    println!("{}\n", "=".repeat(80));
}

pub fn handle_failures(
    results: &[DiscoveryResult],
    targets: &[(String, String, PortDiscoveryConfig)],
) -> Result<()> {
    for (result, (_, _, discovery_config)) in results.iter().zip(targets.iter()) {
        if result.failed_ports.is_empty() && !result.discovered_ports.is_empty() {
            continue;
        }

        match discovery_config.on_failure {
            FailureAction::Fail => {
                bail!(
                    "Port discovery failed for target '{}'. {} port(s) failed, {} succeeded. \
                    Set on_failure to 'skip' or 'warn' to continue anyway.",
                    result.target_id,
                    result.failed_ports.len(),
                    result.discovered_ports.len()
                );
            }
            FailureAction::Skip => {
                tracing::warn!(
                    "Port discovery failed for target '{}'; target will be skipped (on_failure=skip). \
                    {} port(s) failed, {} succeeded.",
                    result.target_id,
                    result.failed_ports.len(),
                    result.discovered_ports.len()
                );
            }
            FailureAction::Warn => {
                tracing::warn!(
                    "Port discovery had failures for target '{}', but continuing with original URL (on_failure=warn). \
                    {} port(s) failed, {} succeeded.",
                    result.target_id,
                    result.failed_ports.len(),
                    result.discovered_ports.len()
                );
            }
        }
    }

    Ok(())
}

pub fn find_best_port(result: &DiscoveryResult) -> Option<u16> {
    for port_info in &result.discovered_ports {
        if port_info.service_type == Some(ServiceType::Https) {
            return Some(port_info.port);
        }
    }
    for port_info in &result.discovered_ports {
        if port_info.service_type == Some(ServiceType::Http) {
            return Some(port_info.port);
        }
    }
    result.discovered_ports.first().map(|p| p.port)
}

pub fn update_url_port(url: &str, port: u16) -> Result<String> {
    let mut parsed = url::Url::parse(url)?;
    parsed
        .set_port(Some(port))
        .map_err(|_| anyhow::anyhow!("Failed to set port {port} on URL {url}"))?;
    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_spec_single() {
        let spec = PortSpec::Single(8080);
        assert_eq!(spec.to_vec().unwrap(), vec![8080]);
    }

    #[test]
    fn test_port_spec_list() {
        let spec = PortSpec::List(vec![80, 443, 8080]);
        assert_eq!(spec.to_vec().unwrap(), vec![80, 443, 8080]);
    }

    #[test]
    fn test_port_spec_range() {
        let spec = PortSpec::Range {
            start: 8000,
            end: 8003,
        };
        assert_eq!(spec.to_vec().unwrap(), vec![8000, 8001, 8002, 8003]);
    }

    #[test]
    fn test_port_spec_range_too_large() {
        let spec = PortSpec::Range {
            start: 1,
            end: 2000,
        };
        assert!(spec.to_vec().is_err());
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
