use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RequestResult {
    pub start_time: Instant,
    pub duration: Duration,
    pub status_code: Option<u16>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StatusCodeDistribution {
    pub counts: HashMap<u16, usize>,
}

impl StatusCodeDistribution {
    pub fn add(&mut self, code: u16) {
        *self.counts.entry(code).or_insert(0) += 1;
    }

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

        // Record error
        if let Some(error) = result.error {
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
