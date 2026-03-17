use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{interval, sleep, Instant};
use tokio_util::sync::CancellationToken;

use crate::client::HttpClient;
use crate::config::TrafficPattern;
use crate::metrics::{MetricsCollector, MultiTargetMetrics};

pub struct PatternExecutor {
    client: HttpClient,
    metrics: MetricsMode,
    pattern: TrafficPattern,
}

#[derive(Clone)]
enum MetricsMode {
    Single(MetricsCollector),
    Multi(MultiTargetMetrics),
}

impl PatternExecutor {
    pub fn new(client: HttpClient, metrics: MetricsCollector, pattern: TrafficPattern) -> Self {
        Self {
            client,
            metrics: MetricsMode::Single(metrics),
            pattern,
        }
    }

    pub fn new_multi_target(
        client: HttpClient,
        metrics: MultiTargetMetrics,
        pattern: TrafficPattern,
    ) -> Self {
        Self {
            client,
            metrics: MetricsMode::Multi(metrics),
            pattern,
        }
    }

    pub async fn execute(&self, cancel_token: CancellationToken) -> Result<()> {
        match &self.pattern {
            TrafficPattern::Fixed {
                concurrent,
                duration_secs,
                total_requests,
            } => {
                self.execute_fixed(*concurrent, *duration_secs, *total_requests, cancel_token)
                    .await
            }
            TrafficPattern::RateLimit {
                rate,
                duration_secs,
                total_requests,
            } => {
                self.execute_rate_limit(*rate, *duration_secs, *total_requests, cancel_token)
                    .await
            }
            TrafficPattern::Ramp {
                from,
                to,
                ramp_duration_secs,
                hold_duration_secs,
            } => {
                self.execute_ramp(
                    *from,
                    *to,
                    *ramp_duration_secs,
                    *hold_duration_secs,
                    cancel_token,
                )
                .await
            }
            TrafficPattern::Burst {
                size,
                interval_secs,
                duration_secs,
                total_bursts,
            } => {
                self.execute_burst(
                    *size,
                    *interval_secs,
                    *duration_secs,
                    *total_bursts,
                    cancel_token,
                )
                .await
            }
        }
    }

    async fn execute_fixed(
        &self,
        concurrent: usize,
        duration_secs: Option<u64>,
        total_requests: Option<usize>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(concurrent));
        let start = Instant::now();

        let mut request_count = 0;
        let mut handles = Vec::new();

        loop {
            // Check cancellation
            if cancel_token.is_cancelled() {
                break;
            }

            // Check duration limit
            if let Some(duration) = duration_secs {
                if start.elapsed().as_secs() >= duration {
                    break;
                }
            }

            // Check request count limit
            if let Some(total) = total_requests {
                if request_count >= total {
                    break;
                }
            }

            let permit = semaphore.clone().acquire_owned().await?;
            let client = self.client.clone();
            let metrics_mode = match &self.metrics {
                MetricsMode::Single(m) => MetricsMode::Single(m.clone()),
                MetricsMode::Multi(m) => MetricsMode::Multi(m.clone()),
            };

            let handle = tokio::spawn(async move {
                let result = client.execute().await;
                match metrics_mode {
                    MetricsMode::Single(m) => m.record(result),
                    MetricsMode::Multi(m) => m.record(result),
                }
                drop(permit);
            });

            handles.push(handle);
            request_count += 1;
        }

        // Wait for all requests to complete
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    async fn execute_rate_limit(
        &self,
        rate: usize,
        duration_secs: Option<u64>,
        total_requests: Option<usize>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let interval_micros = 1_000_000 / rate as u64;
        let mut ticker = interval(Duration::from_micros(interval_micros));
        let start = Instant::now();

        let mut request_count = 0;

        loop {
            ticker.tick().await;

            // Check cancellation
            if cancel_token.is_cancelled() {
                break;
            }

            // Check duration limit
            if let Some(duration) = duration_secs {
                if start.elapsed().as_secs() >= duration {
                    break;
                }
            }

            // Check request count limit
            if let Some(total) = total_requests {
                if request_count >= total {
                    break;
                }
            }

            let client = self.client.clone();
            let metrics_mode = match &self.metrics {
                MetricsMode::Single(m) => MetricsMode::Single(m.clone()),
                MetricsMode::Multi(m) => MetricsMode::Multi(m.clone()),
            };

            tokio::spawn(async move {
                let result = client.execute().await;
                match metrics_mode {
                    MetricsMode::Single(m) => m.record(result),
                    MetricsMode::Multi(m) => m.record(result),
                }
            });

            request_count += 1;
        }

        // Give some time for in-flight requests to complete
        sleep(Duration::from_secs(2)).await;

        Ok(())
    }

    async fn execute_ramp(
        &self,
        from: usize,
        to: usize,
        ramp_duration_secs: u64,
        hold_duration_secs: Option<u64>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let ramp_duration = Duration::from_secs(ramp_duration_secs);

        // Calculate step size and interval
        let steps = 10.max((to - from) / 5); // At least 10 steps
        let step_duration = ramp_duration / steps as u32;
        let concurrency_step = (to as f64 - from as f64) / steps as f64;

        // Ramp-up phase
        for step in 0..steps {
            if cancel_token.is_cancelled() {
                break;
            }

            let target_concurrency = from + ((step as f64 + 1.0) * concurrency_step) as usize;
            let current_concurrency = target_concurrency.min(to);

            tracing::debug!("Ramping to {} concurrent clients", current_concurrency);

            // Run at this concurrency level for step_duration
            self.execute_fixed(
                current_concurrency,
                Some(step_duration.as_secs()),
                None,
                cancel_token.clone(),
            )
            .await?;
        }

        // Hold phase at maximum concurrency
        if let Some(hold_duration) = hold_duration_secs {
            if !cancel_token.is_cancelled() {
                tracing::debug!("Holding at {} concurrent clients", to);
                self.execute_fixed(to, Some(hold_duration), None, cancel_token)
                    .await?;
            }
        }

        Ok(())
    }

    async fn execute_burst(
        &self,
        size: usize,
        interval_secs: u64,
        duration_secs: Option<u64>,
        total_bursts: Option<usize>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let mut burst_ticker = interval(Duration::from_secs(interval_secs));
        let start = Instant::now();
        let mut burst_count = 0;

        loop {
            burst_ticker.tick().await;

            // Check cancellation
            if cancel_token.is_cancelled() {
                break;
            }

            // Check duration limit
            if let Some(duration) = duration_secs {
                if start.elapsed().as_secs() >= duration {
                    break;
                }
            }

            // Check burst count limit
            if let Some(total) = total_bursts {
                if burst_count >= total {
                    break;
                }
            }

            tracing::debug!("Sending burst {} of {} requests", burst_count + 1, size);

            // Send burst
            let mut handles = Vec::new();
            for _ in 0..size {
                let client = self.client.clone();
                let metrics_mode = match &self.metrics {
                    MetricsMode::Single(m) => MetricsMode::Single(m.clone()),
                    MetricsMode::Multi(m) => MetricsMode::Multi(m.clone()),
                };

                let handle = tokio::spawn(async move {
                    let result = client.execute().await;
                    match metrics_mode {
                        MetricsMode::Single(m) => m.record(result),
                        MetricsMode::Multi(m) => m.record(result),
                    }
                });

                handles.push(handle);
            }

            // Wait for burst to complete
            for handle in handles {
                let _ = handle.await;
            }

            burst_count += 1;
        }

        Ok(())
    }
}
