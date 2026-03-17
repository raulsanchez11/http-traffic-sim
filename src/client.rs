//! HTTP client module for executing requests and stress testing patterns.
//!
//! This module provides HTTP/HTTPS client functionality with support for:
//!
//! - Single-target and multi-target load testing
//! - Standard HTTP request execution with metrics
//! - Connection pooling and keep-alive
//! - Stress testing patterns (slowloris, slow read, connection hold)
//! - Error categorization and detailed reporting
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::client::HttpClient;
//! use http_traffic_sim::config::TargetConfig;
//! use std::time::Duration;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a single-target client
//! let target = TargetConfig::default();
//! let client = HttpClient::new(
//!     target,
//!     Duration::from_secs(30),
//!     128
//! )?;
//!
//! // Execute a request
//! let result = client.execute().await;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use reqwest::{Client, Method, Request};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::config::TargetConfig;
use crate::metrics::RequestResult;
use crate::target_selector::TargetSelector;

/// HTTP client for executing load tests and stress tests.
///
/// The client supports both single-target and multi-target modes,
/// connection pooling, and various stress testing patterns.
///
/// # Features
///
/// - Connection pooling with configurable limits
/// - TCP keep-alive for long-running tests
/// - Multi-target load distribution
/// - Stress testing capabilities (slowloris, slow read, etc.)
/// - Detailed metrics collection
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::client::HttpClient;
/// use http_traffic_sim::config::TargetConfig;
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// let target = TargetConfig::default();
/// let client = HttpClient::new(
///     target,
///     Duration::from_secs(30),
///     128
/// )?;
///
/// let result = client.execute().await;
/// println!("Status: {:?}", result.status_code);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    mode: ClientMode,
}

#[derive(Clone)]
enum ClientMode {
    SingleTarget { target: Arc<TargetConfig> },
    MultiTarget { selector: Arc<TargetSelector> },
}

impl HttpClient {
    /// Creates a new HTTP client for single-target testing.
    ///
    /// # Arguments
    ///
    /// * `target` - Target configuration (URL, method, headers, body)
    /// * `timeout` - Request timeout duration
    /// * `pool_max_idle` - Maximum idle connections per host in pool
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::client::HttpClient;
    /// use http_traffic_sim::config::TargetConfig;
    /// use std::time::Duration;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let target = TargetConfig::default();
    /// let client = HttpClient::new(
    ///     target,
    ///     Duration::from_secs(30),
    ///     128
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(target: TargetConfig, timeout: Duration, pool_max_idle: usize) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(pool_max_idle)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .build()?;

        Ok(Self {
            client,
            mode: ClientMode::SingleTarget {
                target: Arc::new(target),
            },
        })
    }

    /// Creates a new HTTP client for multi-target testing.
    ///
    /// Uses a target selector to distribute load across multiple targets
    /// according to the configured distribution strategy.
    ///
    /// # Arguments
    ///
    /// * `selector` - Target selector for load distribution
    /// * `timeout` - Request timeout duration
    /// * `pool_max_idle` - Maximum idle connections per host in pool
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::client::HttpClient;
    /// use http_traffic_sim::target_selector::TargetSelector;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// # let targets = vec![];
    /// # let distribution = http_traffic_sim::config::LoadDistribution::RoundRobin;
    /// let selector = Arc::new(TargetSelector::new(targets, distribution));
    /// let client = HttpClient::new_multi_target(
    ///     selector,
    ///     Duration::from_secs(30),
    ///     128
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_multi_target(
        selector: Arc<TargetSelector>,
        timeout: Duration,
        pool_max_idle: usize,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(pool_max_idle)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .build()?;

        Ok(Self {
            client,
            mode: ClientMode::MultiTarget { selector },
        })
    }

    /// Executes a single HTTP request and returns timing metrics.
    ///
    /// This is the main method for standard load testing. It:
    /// - Selects a target (single or from multi-target pool)
    /// - Builds the HTTP request with method, headers, and body
    /// - Executes the request and measures response time
    /// - Categorizes errors for detailed reporting
    ///
    /// # Returns
    ///
    /// Returns a `RequestResult` containing:
    /// - Duration of the request
    /// - HTTP status code (if successful)
    /// - Success/failure indication
    /// - Error message (if failed)
    /// - Target ID for multi-target tracking
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::client::HttpClient;
    /// use http_traffic_sim::config::TargetConfig;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let target = TargetConfig::default();
    /// let client = HttpClient::new(target, Duration::from_secs(30), 128)?;
    /// let result = client.execute().await;
    ///
    /// if result.success {
    ///     println!("Request succeeded in {:?}", result.duration);
    /// } else {
    ///     println!("Request failed: {:?}", result.error);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self) -> RequestResult {
        let start = Instant::now();

        // Select target based on mode
        let target = match &self.mode {
            ClientMode::SingleTarget { target } => target.clone(),
            ClientMode::MultiTarget { selector } => selector.select(),
        };

        // Build request
        let method = Method::from_bytes(target.method.as_bytes()).unwrap_or(Method::GET);

        let mut request_builder = self.client.request(method, &target.url);

        // Add headers
        for (key, value) in &target.headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = &target.body {
            request_builder = request_builder.body(body.clone());
        }

        // Execute request and measure
        let result = match request_builder.build() {
            Ok(request) => self.send_and_measure(request, start, &target.id).await,
            Err(e) => RequestResult {
                start_time: start,
                duration: start.elapsed(),
                status_code: None,
                success: false,
                error: Some(format!("Failed to build request: {}", e)),
                target_id: target.id.clone(),
            },
        };

        result
    }

    async fn send_and_measure(
        &self,
        request: Request,
        start: Instant,
        target_id: &str,
    ) -> RequestResult {
        match self.client.execute(request).await {
            Ok(response) => {
                let duration = start.elapsed();
                let status = response.status();
                let success = status.is_success();

                RequestResult {
                    start_time: start,
                    duration,
                    status_code: Some(status.as_u16()),
                    success,
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", status.as_u16()))
                    },
                    target_id: target_id.to_string(),
                }
            }
            Err(e) => {
                let duration = start.elapsed();
                let error_msg = if e.is_timeout() {
                    "Request timeout".to_string()
                } else if e.is_connect() {
                    format!("Connection failed: {}", e)
                } else if e.is_request() {
                    format!("Request error: {}", e)
                } else {
                    format!("Error: {}", e)
                };

                RequestResult {
                    start_time: start,
                    duration,
                    status_code: None,
                    success: false,
                    error: Some(error_msg),
                    target_id: target_id.to_string(),
                }
            }
        }
    }

    /// Executes a request and holds the connection open for a specified duration.
    ///
    /// Used for connection flood stress testing patterns. Opens a connection,
    /// makes a request, then holds the connection open to consume server resources.
    ///
    /// # Arguments
    ///
    /// * `hold_duration` - How long to keep the connection open after the request
    ///
    /// # Returns
    ///
    /// Returns the request result from the initial execution.
    ///
    /// # Note
    ///
    /// This is a stress testing feature and requires proper authorization.
    pub async fn execute_and_hold(&self, hold_duration: Duration) -> RequestResult {
        let result = self.execute().await;

        // Hold for the specified duration
        tokio::time::sleep(hold_duration).await;

        result
    }

    /// Opens a raw TCP connection and sends partial HTTP headers (slowloris attack).
    ///
    /// Used for slowloris stress testing patterns. Opens a TCP connection and sends
    /// incomplete HTTP headers, holding the connection open without completing the request.
    ///
    /// # Arguments
    ///
    /// * `url` - Target URL to connect to
    /// * `partial_headers` - Incomplete HTTP headers to send
    ///
    /// # Behavior
    ///
    /// - Parses URL to extract host and port
    /// - Opens raw TCP connection
    /// - Sends partial headers without completing request
    /// - Holds connection open for 5 minutes
    ///
    /// # Note
    ///
    /// This is a stress testing feature and requires proper authorization.
    /// Only use against systems you own or have explicit permission to test.
    pub async fn send_partial_request(&self, url: &str, partial_headers: &str) -> Result<()> {
        // Parse URL to extract host and port
        let parsed_url = url::Url::parse(url)?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid host"))?;
        let port = parsed_url
            .port()
            .unwrap_or(if parsed_url.scheme() == "https" {
                443
            } else {
                80
            });

        // Open TCP connection
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr).await?;

        // Send partial headers
        stream.write_all(partial_headers.as_bytes()).await?;
        stream.flush().await?;

        // Keep connection open but don't complete the request
        tokio::time::sleep(Duration::from_secs(300)).await; // Hold for 5 minutes

        Ok(())
    }

    /// Executes a request and reads the response very slowly (slow read attack).
    ///
    /// Used for slow read stress testing patterns. Makes a request and then
    /// deliberately reads the response data very slowly to tie up server resources.
    ///
    /// # Arguments
    ///
    /// * `_read_rate_bps` - Target read rate in bytes per second (currently unused)
    ///
    /// # Behavior
    ///
    /// - Executes HTTP request
    /// - Reads response chunks with 100ms delays between reads
    /// - Holds connection open while slowly consuming response
    ///
    /// # Note
    ///
    /// This is a stress testing feature and requires proper authorization.
    /// Only use against systems you own or have explicit permission to test.
    pub async fn slow_read(&self, _read_rate_bps: usize) -> Result<()> {
        let target = match &self.mode {
            ClientMode::SingleTarget { target } => target.clone(),
            ClientMode::MultiTarget { selector } => selector.select(),
        };

        let method = Method::from_bytes(target.method.as_bytes()).unwrap_or(Method::GET);

        let request = self.client.request(method, &target.url).build()?;

        if let Ok(mut response) = self.client.execute(request).await {
            loop {
                match response.chunk().await {
                    Ok(Some(_chunk)) => {
                        // Simulate slow reading
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Ok(None) => break, // End of stream
                    Err(_) => break,
                }
            }
        }

        Ok(())
    }
}
