use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep, Instant};
use tokio_util::sync::CancellationToken;

use crate::client::HttpClient;
use crate::config::StressPattern;
use crate::metrics::MetricsCollector;

pub struct StressExecutor {
    client: HttpClient,
    metrics: MetricsCollector,
    pattern: StressPattern,
}

impl StressExecutor {
    pub fn new(client: HttpClient, metrics: MetricsCollector, pattern: StressPattern) -> Self {
        Self {
            client,
            metrics,
            pattern,
        }
    }

    pub async fn execute(&self, cancel_token: CancellationToken) -> Result<()> {
        match &self.pattern {
            StressPattern::ConnectionFlood {
                connections_per_second,
                hold_time_ms,
                duration_secs,
            } => {
                self.execute_connection_flood(*connections_per_second, *hold_time_ms, *duration_secs, cancel_token)
                    .await
            }
            StressPattern::Slowloris {
                connections,
                headers_per_second,
                duration_secs,
            } => {
                self.execute_slowloris(*connections, *headers_per_second, *duration_secs, cancel_token)
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
                self.execute_pipeline_abuse(*requests_per_connection, *concurrent_connections, cancel_token)
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
                    let _ = client.send_partial_request("http://example.com", partial).await;
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
