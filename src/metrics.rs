use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RequestResult {
    #[allow(dead_code)]
    pub start_time: Instant,
    pub duration: Duration,
    pub status_code: Option<u16>,
    pub success: bool,
    pub error: Option<String>,
    pub target_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub refused_count: Arc<AtomicUsize>,
    pub timeout_count: Arc<AtomicUsize>,
    pub reset_by_peer_count: Arc<AtomicUsize>,
    pub tls_handshake_errors: Arc<AtomicUsize>,
    pub dns_errors: Arc<AtomicUsize>,
    pub other_errors: Arc<AtomicUsize>,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self {
            refused_count: Arc::new(AtomicUsize::new(0)),
            timeout_count: Arc::new(AtomicUsize::new(0)),
            reset_by_peer_count: Arc::new(AtomicUsize::new(0)),
            tls_handshake_errors: Arc::new(AtomicUsize::new(0)),
            dns_errors: Arc::new(AtomicUsize::new(0)),
            other_errors: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn categorize_and_increment(&self, error: &str) {
        let error_lower = error.to_lowercase();

        if error_lower.contains("connection refused") || error_lower.contains("econnrefused") {
            self.refused_count.fetch_add(1, Ordering::Relaxed);
        } else if error_lower.contains("timeout") || error_lower.contains("etimedout") {
            self.timeout_count.fetch_add(1, Ordering::Relaxed);
        } else if error_lower.contains("connection reset") || error_lower.contains("econnreset") {
            self.reset_by_peer_count.fetch_add(1, Ordering::Relaxed);
        } else if error_lower.contains("tls")
            || error_lower.contains("ssl")
            || error_lower.contains("certificate")
        {
            self.tls_handshake_errors.fetch_add(1, Ordering::Relaxed);
        } else if error_lower.contains("dns") || error_lower.contains("resolve") {
            self.dns_errors.fetch_add(1, Ordering::Relaxed);
        } else {
            self.other_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get_snapshot(&self) -> ConnectionStatsSnapshot {
        ConnectionStatsSnapshot {
            refused_count: self.refused_count.load(Ordering::Relaxed),
            timeout_count: self.timeout_count.load(Ordering::Relaxed),
            reset_by_peer_count: self.reset_by_peer_count.load(Ordering::Relaxed),
            tls_handshake_errors: self.tls_handshake_errors.load(Ordering::Relaxed),
            dns_errors: self.dns_errors.load(Ordering::Relaxed),
            other_errors: self.other_errors.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionStatsSnapshot {
    pub refused_count: usize,
    pub timeout_count: usize,
    pub reset_by_peer_count: usize,
    pub tls_handshake_errors: usize,
    pub dns_errors: usize,
    pub other_errors: usize,
}

#[derive(Debug, Clone, Default)]
pub struct StatusCodeDistribution {
    pub counts: HashMap<u16, usize>,
}

impl StatusCodeDistribution {
    pub fn add(&mut self, code: u16) {
        *self.counts.entry(code).or_insert(0) += 1;
    }

    #[allow(dead_code)]
    pub fn merge(&mut self, other: &StatusCodeDistribution) {
        for (code, count) in &other.counts {
            *self.counts.entry(*code).or_insert(0) += count;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ErrorDistribution {
    pub counts: HashMap<String, usize>,
}

impl ErrorDistribution {
    pub fn add(&mut self, error: String) {
        *self.counts.entry(error).or_insert(0) += 1;
    }

    #[allow(dead_code)]
    pub fn merge(&mut self, other: &ErrorDistribution) {
        for (error, count) in &other.counts {
            *self.counts.entry(error.clone()).or_insert(0) += count;
        }
    }
}

#[derive(Clone)]
pub struct MetricsCollector {
    inner: Arc<Mutex<MetricsInner>>,
}

struct MetricsInner {
    start_time: Instant,
    latencies_us: Vec<u64>, // Store latencies in microseconds
    status_codes: StatusCodeDistribution,
    errors: ErrorDistribution,
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    connection_stats: ConnectionStats,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MetricsInner {
                start_time: Instant::now(),
                latencies_us: Vec::new(),
                status_codes: StatusCodeDistribution::default(),
                errors: ErrorDistribution::default(),
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                connection_stats: ConnectionStats::new(),
            })),
        }
    }

    pub fn record(&self, result: RequestResult) {
        let mut inner = self.inner.lock().unwrap();

        inner.total_requests += 1;

        if result.success {
            inner.successful_requests += 1;
        } else {
            inner.failed_requests += 1;
        }

        // Record latency in microseconds
        let latency_us = result.duration.as_micros() as u64;
        inner.latencies_us.push(latency_us);

        // Record status code
        if let Some(code) = result.status_code {
            inner.status_codes.add(code);
        }

        // Record error and categorize connection errors
        if let Some(error) = result.error {
            if !result.success {
                inner.connection_stats.categorize_and_increment(&error);
            }
            inner.errors.add(error);
        }
    }

    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let inner = self.inner.lock().unwrap();

        let elapsed = inner.start_time.elapsed();

        MetricsSnapshot {
            elapsed,
            total_requests: inner.total_requests,
            successful_requests: inner.successful_requests,
            failed_requests: inner.failed_requests,
            latencies_us: inner.latencies_us.clone(),
            status_codes: inner.status_codes.clone(),
            errors: inner.errors.clone(),
            connection_stats: inner.connection_stats.get_snapshot(),
        }
    }

    pub fn reset_start_time(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.start_time = Instant::now();
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub elapsed: Duration,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub latencies_us: Vec<u64>,
    pub status_codes: StatusCodeDistribution,
    pub errors: ErrorDistribution,
    pub connection_stats: ConnectionStatsSnapshot,
}

/// Per-target metrics tracker
#[derive(Clone)]
pub struct TargetMetrics {
    #[allow(dead_code)]
    pub target_id: String,
    pub collector: MetricsCollector,
}

impl TargetMetrics {
    pub fn new(target_id: String) -> Self {
        Self {
            target_id,
            collector: MetricsCollector::new(),
        }
    }
}

/// Multi-target metrics aggregator
#[derive(Clone)]
pub struct MultiTargetMetrics {
    targets: Arc<Mutex<HashMap<String, Arc<TargetMetrics>>>>,
    global: MetricsCollector,
}

impl MultiTargetMetrics {
    pub fn new() -> Self {
        Self {
            targets: Arc::new(Mutex::new(HashMap::new())),
            global: MetricsCollector::new(),
        }
    }

    pub fn record(&self, result: RequestResult) {
        // Record to global metrics
        self.global.record(result.clone());

        // Record to per-target metrics
        let mut targets = self.targets.lock().unwrap();
        let target_metrics = targets
            .entry(result.target_id.clone())
            .or_insert_with(|| Arc::new(TargetMetrics::new(result.target_id.clone())));

        target_metrics.collector.record(result);
    }

    pub fn get_global_snapshot(&self) -> MetricsSnapshot {
        self.global.get_snapshot()
    }

    pub fn get_per_target_snapshots(&self) -> HashMap<String, MetricsSnapshot> {
        let targets = self.targets.lock().unwrap();
        targets
            .iter()
            .map(|(id, metrics)| (id.clone(), metrics.collector.get_snapshot()))
            .collect()
    }

    pub fn reset_start_time(&self) {
        self.global.reset_start_time();
        let targets = self.targets.lock().unwrap();
        for metrics in targets.values() {
            metrics.collector.reset_start_time();
        }
    }
}

impl MetricsSnapshot {
    pub fn requests_per_second(&self) -> f64 {
        let secs = self.elapsed.as_secs_f64();
        if secs > 0.0 {
            self.total_requests as f64 / secs
        } else {
            0.0
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.failed_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }
}
