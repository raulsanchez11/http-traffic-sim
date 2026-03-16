use crate::metrics::MetricsSnapshot;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub duration: Duration,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub requests_per_second: f64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub latency: LatencyStats,
    pub status_codes: Vec<(u16, usize)>,
    pub errors: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub stddev_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub p99_9_ms: f64,
}

impl Statistics {
    pub fn from_snapshot(snapshot: &MetricsSnapshot) -> Self {
        let latency = Self::calculate_latency_stats(&snapshot.latencies_us);

        let mut status_codes: Vec<(u16, usize)> = snapshot
            .status_codes
            .counts
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        status_codes.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

        let mut errors: Vec<(String, usize)> = snapshot
            .errors
            .counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        errors.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

        Statistics {
            duration: snapshot.elapsed,
            total_requests: snapshot.total_requests,
            successful_requests: snapshot.successful_requests,
            failed_requests: snapshot.failed_requests,
            requests_per_second: snapshot.requests_per_second(),
            success_rate: snapshot.success_rate(),
            error_rate: snapshot.error_rate(),
            latency,
            status_codes,
            errors,
        }
    }

    fn calculate_latency_stats(latencies_us: &[u64]) -> LatencyStats {
        if latencies_us.is_empty() {
            return LatencyStats {
                min_ms: 0.0,
                max_ms: 0.0,
                mean_ms: 0.0,
                stddev_ms: 0.0,
                p50_ms: 0.0,
                p90_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                p99_9_ms: 0.0,
            };
        }

        // Create histogram with auto-resizing (1 microsecond to 1 hour)
        let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000_000, 3)
            .expect("Failed to create histogram");

        // Record all latencies
        for &latency in latencies_us {
            let _ = hist.record(latency);
        }

        // Convert microseconds to milliseconds
        let us_to_ms = |us: u64| us as f64 / 1000.0;

        LatencyStats {
            min_ms: us_to_ms(hist.min()),
            max_ms: us_to_ms(hist.max()),
            mean_ms: us_to_ms(hist.mean() as u64),
            stddev_ms: us_to_ms(hist.stdev() as u64),
            p50_ms: us_to_ms(hist.value_at_quantile(0.5)),
            p90_ms: us_to_ms(hist.value_at_quantile(0.9)),
            p95_ms: us_to_ms(hist.value_at_quantile(0.95)),
            p99_ms: us_to_ms(hist.value_at_quantile(0.99)),
            p99_9_ms: us_to_ms(hist.value_at_quantile(0.999)),
        }
    }
}
