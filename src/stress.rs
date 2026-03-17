//! Stress testing pattern execution.
//!
//! This module implements various stress testing patterns designed to test
//! server resilience under extreme conditions. **Requires explicit authorization.**
//!
//! # Stress Testing Patterns
//!
//! - **Connection Flood**: Opens many connections rapidly and holds them
//! - **Slowloris**: Sends partial HTTP headers slowly to exhaust connection pools
//! - **Slow POST**: Sends request body data very slowly
//! - **Request Flood**: Sends requests at extremely high rates
//! - **Large Payload**: Sends very large payloads to test bandwidth handling
//! - **Pipeline Abuse**: Abuses HTTP pipelining with many requests per connection
//! - **Slow Read**: Reads response data very slowly to tie up server resources
//!
//! # Authorization Required
//!
//! All stress testing patterns require:
//! - Explicit authorization configuration (`authorization.confirmed: true`)
//! - Authorization details (owner, notes)
//! - Safety limits (optional but recommended)
//!
//! See the `authorization` module for validation details.
//!
//! # Legal and Ethical Considerations
//!
//! **WARNING**: Stress testing can:
//! - Impact service availability
//! - Trigger security alerts
//! - Violate terms of service
//! - Be illegal without authorization
//!
//! Only use against systems you own or have explicit written permission to test.
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::stress::StressExecutor;
//! use http_traffic_sim::client::HttpClient;
//! use http_traffic_sim::config::{TargetConfig, StressPattern};
//! use http_traffic_sim::metrics::MetricsCollector;
//! use tokio_util::sync::CancellationToken;
//! use std::time::Duration;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let target = TargetConfig::default();
//! let client = HttpClient::new(target, Duration::from_secs(30), 128)?;
//! let metrics = MetricsCollector::new();
//!
//! let pattern = StressPattern::RequestFlood {
//!     target_rps: 1000,
//!     duration_secs: 60,
//! };
//!
//! // IMPORTANT: Requires authorization validation first!
//! let executor = StressExecutor::new(client, metrics, pattern);
//! executor.execute(CancellationToken::new()).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use std::time::Duration;
use tokio::time::{interval, sleep, Instant};
use tokio_util::sync::CancellationToken;

use crate::client::HttpClient;
use crate::config::StressPattern;
use crate::metrics::MetricsCollector;

/// Executor for stress testing patterns.
///
/// Implements various aggressive load patterns designed to test server
/// resilience and identify breaking points. **Requires authorization.**
///
/// # Patterns Supported
///
/// - Connection flood: Rapid connection opening with hold
/// - Slowloris: Partial header attacks
/// - Slow POST: Slow request body transmission
/// - Request flood: High-rate request generation
/// - Large payload: Bandwidth exhaustion testing
/// - Pipeline abuse: HTTP pipelining exploitation
/// - Slow read: Slow response consumption
///
/// # Safety
///
/// All patterns require authorization validation via the `authorization` module
/// before execution. See `authorization::validate_and_warn()`.
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::stress::StressExecutor;
/// use http_traffic_sim::client::HttpClient;
/// use http_traffic_sim::config::{TargetConfig, StressPattern};
/// use http_traffic_sim::metrics::MetricsCollector;
/// use tokio_util::sync::CancellationToken;
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// # let target = TargetConfig::default();
/// let client = HttpClient::new(target, Duration::from_secs(30), 128)?;
/// let metrics = MetricsCollector::new();
///
/// let pattern = StressPattern::ConnectionFlood {
///     connections_per_second: 100,
///     hold_time_ms: 5000,
///     duration_secs: 60,
/// };
///
/// let executor = StressExecutor::new(client, metrics, pattern);
/// executor.execute(CancellationToken::new()).await?;
/// # Ok(())
/// # }
/// ```
pub struct StressExecutor {
    client: HttpClient,
    metrics: MetricsCollector,
    pattern: StressPattern,
}

impl StressExecutor {
    /// Creates a new stress testing executor.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client for executing stress patterns
    /// * `metrics` - Metrics collector for tracking results
    /// * `pattern` - Stress testing pattern to execute
    ///
    /// # Note
    ///
    /// This does NOT validate authorization. You must call
    /// `authorization::validate_and_warn()` before executing.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::stress::StressExecutor;
    /// use http_traffic_sim::client::HttpClient;
    /// use http_traffic_sim::config::{TargetConfig, StressPattern};
    /// use http_traffic_sim::metrics::MetricsCollector;
    /// use std::time::Duration;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// # let target = TargetConfig::default();
    /// let client = HttpClient::new(target, Duration::from_secs(30), 128)?;
    /// let metrics = MetricsCollector::new();
    ///
    /// let pattern = StressPattern::RequestFlood {
    ///     target_rps: 1000,
    ///     duration_secs: 60,
    /// };
    ///
    /// let executor = StressExecutor::new(client, metrics, pattern);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: HttpClient, metrics: MetricsCollector, pattern: StressPattern) -> Self {
        Self {
            client,
            metrics,
            pattern,
        }
    }

    /// Executes the configured stress testing pattern.
    ///
    /// Routes to the appropriate stress pattern implementation based on
    /// the configured pattern type. Supports graceful cancellation.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - Token for graceful cancellation
    ///
    /// # Behavior
    ///
    /// Each pattern implements a specific stress testing technique:
    /// - Manages concurrent connections/requests
    /// - Enforces pattern-specific timing (rates, delays, holds)
    /// - Collects metrics for analysis
    /// - Responds to cancellation signals
    ///
    /// # Errors
    ///
    /// Returns an error if the pattern execution encounters critical failures.
    /// Most individual request failures are recorded in metrics rather than
    /// causing execution to stop.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::stress::StressExecutor;
    /// use tokio_util::sync::CancellationToken;
    ///
    /// # async fn example(executor: StressExecutor) -> anyhow::Result<()> {
    /// let cancel_token = CancellationToken::new();
    ///
    /// // Execute stress pattern
    /// executor.execute(cancel_token.clone()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self, cancel_token: CancellationToken) -> Result<()> {
        match &self.pattern {
            StressPattern::ConnectionFlood {
                connections_per_second,
                hold_time_ms,
                duration_secs,
            } => {
                self.execute_connection_flood(
                    *connections_per_second,
                    *hold_time_ms,
                    *duration_secs,
                    cancel_token,
                )
                .await
            }
            StressPattern::Slowloris {
                connections,
                headers_per_second,
                duration_secs,
            } => {
                self.execute_slowloris(
                    *connections,
                    *headers_per_second,
                    *duration_secs,
                    cancel_token,
                )
                .await
            }
            StressPattern::SlowPost {
                connections,
                bytes_per_second,
                payload_size,
            } => {
                self.execute_slow_post(*connections, *bytes_per_second, *payload_size, cancel_token)
                    .await
            }
            StressPattern::RequestFlood {
                target_rps,
                duration_secs,
            } => {
                self.execute_request_flood(*target_rps, *duration_secs, cancel_token)
                    .await
            }
            StressPattern::LargePayload {
                size_mb,
                concurrent,
                duration_secs,
            } => {
                self.execute_large_payload(*size_mb, *concurrent, *duration_secs, cancel_token)
                    .await
            }
            StressPattern::PipelineAbuse {
                requests_per_connection,
                concurrent_connections,
            } => {
                self.execute_pipeline_abuse(
                    *requests_per_connection,
                    *concurrent_connections,
                    cancel_token,
                )
                .await
            }
            StressPattern::SlowRead {
                connections,
                read_rate_bps,
                duration_secs,
            } => {
                self.execute_slow_read(*connections, *read_rate_bps, *duration_secs, cancel_token)
                    .await
            }
        }
    }

    async fn execute_connection_flood(
        &self,
        connections_per_second: usize,
        hold_time_ms: u64,
        duration_secs: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting connection flood: {} conn/s, hold {}ms, duration {}s",
            connections_per_second,
            hold_time_ms,
            duration_secs
        );

        let interval_micros = 1_000_000 / connections_per_second as u64;
        let mut ticker = interval(Duration::from_micros(interval_micros));
        let start = Instant::now();
        let hold_duration = Duration::from_millis(hold_time_ms);

        loop {
            ticker.tick().await;

            if cancel_token.is_cancelled() || start.elapsed().as_secs() >= duration_secs {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                let result = client.execute_and_hold(hold_duration).await;
                metrics.record(result);
            });
        }

        // Wait for in-flight connections to complete
        sleep(Duration::from_secs(2)).await;

        Ok(())
    }

    async fn execute_slowloris(
        &self,
        connections: usize,
        headers_per_second: f64,
        duration_secs: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting slowloris: {} connections, {:.2} headers/s, duration {}s",
            connections,
            headers_per_second,
            duration_secs
        );

        let client = self.client.clone();
        let mut handles = Vec::new();

        // Spawn connection handlers
        for _ in 0..connections {
            let client = client.clone();
            let cancel = cancel_token.clone();

            let handle = tokio::spawn(async move {
                let interval_secs = 1.0 / headers_per_second;
                let mut ticker = interval(Duration::from_secs_f64(interval_secs));

                let start = Instant::now();
                loop {
                    ticker.tick().await;

                    if cancel.is_cancelled() || start.elapsed().as_secs() >= duration_secs {
                        break;
                    }

                    // Send partial headers (this is a simplified version)
                    let partial = "GET / HTTP/1.1\r\nHost: example.com\r\nX-";
                    let _ = client
                        .send_partial_request("http://example.com", partial)
                        .await;
                }
            });

            handles.push(handle);
        }

        // Wait for all handlers to complete
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    async fn execute_slow_post(
        &self,
        connections: usize,
        bytes_per_second: usize,
        payload_size: usize,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting slow POST: {} connections, {} bytes/s, payload {} bytes",
            connections,
            bytes_per_second,
            payload_size
        );

        // This is a simplified implementation
        // A full implementation would need raw socket handling to send body slowly
        for _ in 0..connections {
            if cancel_token.is_cancelled() {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                let result = client.execute().await;
                metrics.record(result);
            });

            // Delay between connection attempts
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    async fn execute_request_flood(
        &self,
        target_rps: usize,
        duration_secs: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting request flood: {} req/s, duration {}s",
            target_rps,
            duration_secs
        );

        let interval_micros = 1_000_000 / target_rps as u64;
        let mut ticker = interval(Duration::from_micros(interval_micros));
        let start = Instant::now();

        loop {
            ticker.tick().await;

            if cancel_token.is_cancelled() || start.elapsed().as_secs() >= duration_secs {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                let result = client.execute().await;
                metrics.record(result);
            });
        }

        // Wait for in-flight requests
        sleep(Duration::from_secs(2)).await;

        Ok(())
    }

    async fn execute_large_payload(
        &self,
        size_mb: usize,
        concurrent: usize,
        duration_secs: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting large payload: {} MB, {} concurrent, duration {}s",
            size_mb,
            concurrent,
            duration_secs
        );

        let start = Instant::now();

        // Note: Large payload generation would happen here in a full implementation
        // For now, we just simulate with standard requests

        while !cancel_token.is_cancelled() && start.elapsed().as_secs() < duration_secs {
            let mut handles = Vec::new();

            for _ in 0..concurrent {
                let client = self.client.clone();
                let metrics = self.metrics.clone();

                let handle = tokio::spawn(async move {
                    let result = client.execute().await;
                    metrics.record(result);
                });

                handles.push(handle);
            }

            // Wait for batch to complete
            for handle in handles {
                let _ = handle.await;
            }
        }

        Ok(())
    }

    async fn execute_pipeline_abuse(
        &self,
        requests_per_connection: usize,
        concurrent_connections: usize,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting pipeline abuse: {} req/conn, {} connections",
            requests_per_connection,
            concurrent_connections
        );

        let mut handles = Vec::new();

        for _ in 0..concurrent_connections {
            if cancel_token.is_cancelled() {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();
            let reqs = requests_per_connection;

            let handle = tokio::spawn(async move {
                // Send multiple requests on the same connection
                for _ in 0..reqs {
                    let result = client.execute().await;
                    metrics.record(result);
                }
            });

            handles.push(handle);
        }

        // Wait for all connections
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    async fn execute_slow_read(
        &self,
        connections: usize,
        read_rate_bps: usize,
        duration_secs: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        tracing::info!(
            "Starting slow read: {} connections, {} bytes/s, duration {}s",
            connections,
            read_rate_bps,
            duration_secs
        );

        let mut handles = Vec::new();

        for _ in 0..connections {
            if cancel_token.is_cancelled() {
                break;
            }

            let client = self.client.clone();
            let rate = read_rate_bps;

            let handle = tokio::spawn(async move {
                let _ = client.slow_read(rate).await;
            });

            handles.push(handle);
        }

        // Wait for duration
        let start = Instant::now();
        while !cancel_token.is_cancelled() && start.elapsed().as_secs() < duration_secs {
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }
}
