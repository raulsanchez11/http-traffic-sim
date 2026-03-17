//! Statistics calculation and formatting.
//!
//! This module converts raw metrics into formatted statistics with:
//!
//! - Latency percentile calculations using HDR Histogram
//! - Success/error rate computation
//! - Throughput (requests per second) calculation
//! - Status code distribution
//! - Error distribution
//!
//! # Percentile Calculation
//!
//! Latency percentiles are calculated using the HDR (High Dynamic Range) Histogram
//! algorithm, which provides:
//! - Accurate percentile values even with large datasets
//! - Efficient memory usage
//! - Support for wide value ranges (1μs to 1 hour)
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::stats::Statistics;
//! use http_traffic_sim::metrics::MetricsSnapshot;
//!
//! # fn example(snapshot: MetricsSnapshot) {
//! // Convert metrics snapshot to statistics
//! let stats = Statistics::from_snapshot(&snapshot);
//!
//! println!("Success rate: {:.1}%", stats.success_rate);
//! println!("P99 latency: {:.2}ms", stats.latency.p99_ms);
//! println!("Throughput: {:.2} req/s", stats.requests_per_second);
//! # }
//! ```

use crate::metrics::MetricsSnapshot;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Complete statistics for a load test.
///
/// Contains aggregated metrics including throughput, latency distribution,
/// success rates, status codes, and error types.
///
/// # Fields
///
/// - `duration` - Total duration of the test
/// - `total_requests` - Total number of requests sent
/// - `successful_requests` - Number of successful requests (2xx/3xx status codes)
/// - `failed_requests` - Number of failed requests (errors or 4xx/5xx codes)
/// - `requests_per_second` - Throughput (total requests / duration)
/// - `success_rate` - Percentage of successful requests
/// - `error_rate` - Percentage of failed requests
/// - `latency` - Detailed latency statistics with percentiles
/// - `status_codes` - HTTP status code distribution (sorted by frequency)
/// - `errors` - Error message distribution (sorted by frequency)
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::stats::Statistics;
/// use http_traffic_sim::metrics::MetricsSnapshot;
///
/// # fn example(snapshot: MetricsSnapshot) {
/// let stats = Statistics::from_snapshot(&snapshot);
///
/// println!("Duration: {:.2}s", stats.duration.as_secs_f64());
/// println!("Total requests: {}", stats.total_requests);
/// println!("Success rate: {:.1}%", stats.success_rate);
/// println!("P50 latency: {:.2}ms", stats.latency.p50_ms);
/// # }
/// ```
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

/// Latency statistics with percentiles.
///
/// Contains comprehensive latency measurements:
/// - Min/max values
/// - Mean and standard deviation
/// - Percentiles: P50 (median), P90, P95, P99, P99.9
///
/// All values are in milliseconds.
///
/// # Percentile Interpretation
///
/// - **P50 (median)**: Half of requests completed faster than this
/// - **P90**: 90% of requests completed faster than this
/// - **P95**: 95% of requests completed faster than this
/// - **P99**: 99% of requests completed faster than this
/// - **P99.9**: 99.9% of requests completed faster than this (tail latency)
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::stats::LatencyStats;
///
/// # fn example(latency: LatencyStats) {
/// println!("Median latency: {:.2}ms", latency.p50_ms);
/// println!("P99 latency: {:.2}ms", latency.p99_ms);
/// println!("Min/Max: {:.2}ms / {:.2}ms", latency.min_ms, latency.max_ms);
/// # }
/// ```
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
    /// Converts a metrics snapshot into formatted statistics.
    ///
    /// Processes raw metrics data to compute:
    /// - Throughput (requests per second)
    /// - Success/error rates
    /// - Latency percentiles using HDR histogram
    /// - Sorted status code and error distributions
    ///
    /// # Arguments
    ///
    /// * `snapshot` - Raw metrics snapshot from test execution
    ///
    /// # Returns
    ///
    /// Formatted statistics ready for display or export.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::stats::Statistics;
    /// use http_traffic_sim::metrics::{MetricsCollector, RequestResult};
    /// use std::time::{Duration, Instant};
    ///
    /// # fn example() {
    /// let metrics = MetricsCollector::new();
    ///
    /// // Record some results...
    /// // metrics.record(result);
    ///
    /// let snapshot = metrics.get_snapshot();
    /// let stats = Statistics::from_snapshot(&snapshot);
    ///
    /// println!("RPS: {:.2}", stats.requests_per_second);
    /// # }
    /// ```
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
