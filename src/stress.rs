//! Stress testing pattern execution.

use anyhow::{bail, Result};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::{interval, sleep, Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

use crate::client::HttpClient;
use crate::config::StressPattern;
use crate::metrics::MetricsCollector;
use crate::patterns::drain_join_set;

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
        if connections_per_second == 0 {
            bail!("connections_per_second must be at least 1");
        }

        tracing::info!(
            "Starting connection flood: {} conn/s, hold {}ms, duration {}s",
            connections_per_second,
            hold_time_ms,
            duration_secs
        );

        let interval_micros = 1_000_000 / connections_per_second as u64;
        let mut ticker = interval(Duration::from_micros(interval_micros));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let start = Instant::now();
        let hold_duration = Duration::from_millis(hold_time_ms);
        let mut join_set = JoinSet::new();

        loop {
            ticker.tick().await;

            if !should_continue(&cancel_token, start, duration_secs) {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();
            join_set.spawn(async move {
                let result = client.execute_and_hold(hold_duration).await;
                metrics.record(result);
            });
        }

        drain_join_set(&mut join_set, Duration::from_secs(60)).await;
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

        let target = self.client.selected_target();
        let parsed = url::Url::parse(&target.url)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Target URL has no host"))?;
        let partial = format!("GET {} HTTP/1.1\r\nHost: {}\r\nX-", parsed.path(), host);

        let client = self.client.clone();
        let mut join_set = JoinSet::new();

        for _ in 0..connections.max(1) {
            let client = client.clone();
            let cancel = cancel_token.clone();
            let url = target.url.clone();
            let partial = partial.clone();

            join_set.spawn(async move {
                let interval_secs = 1.0 / headers_per_second.max(0.01);
                let mut ticker = interval(Duration::from_secs_f64(interval_secs));
                let start = Instant::now();
                loop {
                    ticker.tick().await;
                    if cancel.is_cancelled() || start.elapsed().as_secs() >= duration_secs {
                        break;
                    }
                    let _ = client.send_partial_request(&url, &partial).await;
                }
            });
        }

        drain_join_set(&mut join_set, Duration::from_secs(duration_secs + 30)).await;
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

        let mut join_set = JoinSet::new();
        for _ in 0..connections.max(1) {
            if cancel_token.is_cancelled() {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();
            join_set.spawn(async move {
                let result = client
                    .execute_slow_post(bytes_per_second, payload_size)
                    .await;
                metrics.record(result);
            });
        }

        drain_join_set(&mut join_set, Duration::from_secs(300)).await;
        Ok(())
    }

    async fn execute_request_flood(
        &self,
        target_rps: usize,
        duration_secs: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        if target_rps == 0 {
            bail!("target_rps must be at least 1");
        }

        tracing::info!(
            "Starting request flood: {} req/s, duration {}s",
            target_rps,
            duration_secs
        );

        let interval_micros = 1_000_000 / target_rps as u64;
        let mut ticker = interval(Duration::from_micros(interval_micros));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let start = Instant::now();
        let mut join_set = JoinSet::new();

        loop {
            ticker.tick().await;

            if !should_continue(&cancel_token, start, duration_secs) {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();
            join_set.spawn(async move {
                let result = client.execute().await;
                metrics.record(result);
            });
        }

        drain_join_set(&mut join_set, Duration::from_secs(60)).await;
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

        while !cancel_token.is_cancelled() && start.elapsed().as_secs() < duration_secs {
            let mut join_set = JoinSet::new();
            for _ in 0..concurrent.max(1) {
                let client = self.client.clone();
                let metrics = self.metrics.clone();
                join_set.spawn(async move {
                    let result = client.execute_large_payload(size_mb).await;
                    metrics.record(result);
                });
            }
            drain_join_set(&mut join_set, Duration::from_secs(600)).await;
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

        let mut join_set = JoinSet::new();

        for _ in 0..concurrent_connections.max(1) {
            if cancel_token.is_cancelled() {
                break;
            }

            let client = self.client.clone();
            let metrics = self.metrics.clone();
            let reqs = requests_per_connection;
            join_set.spawn(async move {
                let result = client.execute_pipelined(reqs).await;
                metrics.record(result);
            });
        }

        drain_join_set(&mut join_set, Duration::from_secs(300)).await;
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

        let mut join_set = JoinSet::new();

        for _ in 0..connections.max(1) {
            if cancel_token.is_cancelled() {
                break;
            }

            let client = self.client.clone();
            let rate = read_rate_bps;
            join_set.spawn(async move {
                let _ = client.slow_read(rate).await;
            });
        }

        let start = Instant::now();
        while should_continue(&cancel_token, start, duration_secs) {
            sleep(Duration::from_secs(1)).await;
        }

        join_set.shutdown().await;
        Ok(())
    }
}

/// Shared helper for common cancel + elapsed duration checks (PR 8 cleanup).
fn should_continue(cancel_token: &CancellationToken, start: Instant, duration_secs: u64) -> bool {
    if cancel_token.is_cancelled() {
        return false;
    }
    if start.elapsed().as_secs() >= duration_secs {
        return false;
    }
    true
}
