//! Traffic pattern execution module.

use anyhow::{bail, Result};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::{interval, Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

use crate::client::HttpClient;
use crate::config::TrafficPattern;
use crate::metrics::{MetricsCollector, MultiTargetMetrics, RequestRecorder};

pub struct PatternExecutor<R: RequestRecorder> {
    client: HttpClient,
    recorder: R,
    pattern: TrafficPattern,
}

impl PatternExecutor<MetricsCollector> {
    pub fn new(client: HttpClient, metrics: MetricsCollector, pattern: TrafficPattern) -> Self {
        Self {
            client,
            recorder: metrics,
            pattern,
        }
    }
}

impl PatternExecutor<MultiTargetMetrics> {
    pub fn new_multi_target(
        client: HttpClient,
        metrics: MultiTargetMetrics,
        pattern: TrafficPattern,
    ) -> Self {
        Self {
            client,
            recorder: metrics,
            pattern,
        }
    }
}

impl<R: RequestRecorder> PatternExecutor<R> {
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
        let workers = concurrent.max(1);
        let start = Instant::now();
        let issued = Arc::new(AtomicUsize::new(0));

        let mut join_set = JoinSet::new();
        for _ in 0..workers {
            let client = self.client.clone();
            let recorder = self.recorder.clone();
            let cancel = cancel_token.clone();
            let issued = issued.clone();

            join_set.spawn(async move {
                loop {
                    if cancel.is_cancelled() {
                        break;
                    }
                    if let Some(duration) = duration_secs {
                        if start.elapsed().as_secs() >= duration {
                            break;
                        }
                    }
                    if let Some(total) = total_requests {
                        if issued.fetch_add(1, Ordering::Relaxed) >= total {
                            break;
                        }
                    } else {
                        issued.fetch_add(1, Ordering::Relaxed);
                    }

                    let result = client.execute().await;
                    recorder.record(result);
                }
            });
        }

        while join_set.join_next().await.transpose()?.is_some() {}
        Ok(())
    }

    async fn execute_rate_limit(
        &self,
        rate: usize,
        duration_secs: Option<u64>,
        total_requests: Option<usize>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        if rate == 0 {
            bail!("Rate limit must be at least 1 request per second");
        }

        let interval_micros = 1_000_000 / rate as u64;
        let mut ticker = interval(Duration::from_micros(interval_micros));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let start = Instant::now();
        let mut join_set = JoinSet::new();
        let mut request_count = 0usize;

        loop {
            ticker.tick().await;

            if !should_continue(&cancel_token, start, duration_secs) {
                break;
            }
            if let Some(total) = total_requests {
                if request_count >= total {
                    break;
                }
            }

            let client = self.client.clone();
            let recorder = self.recorder.clone();
            join_set.spawn(async move {
                let result = client.execute().await;
                recorder.record(result);
            });

            request_count += 1;
        }

        drain_join_set(&mut join_set, Duration::from_secs(30)).await;
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
        if from > to {
            bail!("Ramp 'from' ({from}) must be <= 'to' ({to})");
        }

        let ramp_duration = Duration::from_secs(ramp_duration_secs);
        let steps = 10.max((to.saturating_sub(from)).max(1) / 5);
        let step_duration = ramp_duration / steps as u32;
        let concurrency_step = (to as f64 - from as f64) / steps as f64;

        for step in 0..steps {
            if cancel_token.is_cancelled() {
                break;
            }

            let target_concurrency = from + ((step as f64 + 1.0) * concurrency_step) as usize;
            let current_concurrency = target_concurrency.min(to);

            tracing::debug!("Ramping to {} concurrent clients", current_concurrency);

            self.execute_fixed(
                current_concurrency,
                Some(step_duration.as_secs().max(1)),
                None,
                cancel_token.clone(),
            )
            .await?;
        }

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
        burst_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let start = Instant::now();
        let mut burst_count = 0usize;

        loop {
            burst_ticker.tick().await;

            if !should_continue(&cancel_token, start, duration_secs) {
                break;
            }
            if let Some(total) = total_bursts {
                if burst_count >= total {
                    break;
                }
            }

            tracing::debug!("Sending burst {} of {} requests", burst_count + 1, size);

            let mut join_set = JoinSet::new();
            for _ in 0..size.max(1) {
                let client = self.client.clone();
                let recorder = self.recorder.clone();
                join_set.spawn(async move {
                    let result = client.execute().await;
                    recorder.record(result);
                });
            }
            drain_join_set(&mut join_set, Duration::from_secs(300)).await;

            burst_count += 1;
        }

        Ok(())
    }
}

pub(crate) async fn drain_join_set(join_set: &mut JoinSet<()>, timeout: Duration) {
    let _ = tokio::time::timeout(timeout, async {
        while join_set
            .join_next()
            .await
            .transpose()
            .unwrap_or(None)
            .is_some()
        {}
    })
    .await;
}

/// Shared helper for the common cancel + elapsed duration check pattern.
/// Returns true if we should keep going.
fn should_continue(
    cancel_token: &CancellationToken,
    start: Instant,
    duration_secs: Option<u64>,
) -> bool {
    if cancel_token.is_cancelled() {
        return false;
    }
    if let Some(d) = duration_secs {
        if start.elapsed().as_secs() >= d {
            return false;
        }
    }
    true
}
