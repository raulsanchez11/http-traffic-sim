use criterion::{black_box, criterion_group, criterion_main, Criterion};
use http_traffic_sim::metrics::{MetricsCollector, RequestResult};
use std::time::{Duration, Instant};

fn bench_metrics_record(c: &mut Criterion) {
    let metrics = MetricsCollector::new();

    c.bench_function("metrics_record", |b| {
        b.iter(|| {
            let result = RequestResult {
                start_time: Instant::now(),
                duration: Duration::from_millis(100),
                status_code: Some(200),
                success: true,
                error: None,
                target_id: String::from("test"),
            };
            metrics.record(black_box(result))
        })
    });
}

fn bench_metrics_record_with_error(c: &mut Criterion) {
    let metrics = MetricsCollector::new();

    c.bench_function("metrics_record_with_error", |b| {
        b.iter(|| {
            let result = RequestResult {
                start_time: Instant::now(),
                duration: Duration::from_millis(100),
                status_code: None,
                success: false,
                error: Some(String::from("Connection timeout")),
                target_id: String::from("test"),
            };
            metrics.record(black_box(result))
        })
    });
}

fn bench_metrics_snapshot(c: &mut Criterion) {
    let metrics = MetricsCollector::new();

    // Add some data
    for _ in 0..100 {
        metrics.record(RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(100),
            status_code: Some(200),
            success: true,
            error: None,
            target_id: String::from("test"),
        });
    }

    c.bench_function("metrics_snapshot", |b| {
        b.iter(|| black_box(metrics.get_snapshot()))
    });
}

fn bench_metrics_concurrent(c: &mut Criterion) {
    use std::sync::Arc;

    let metrics = Arc::new(MetricsCollector::new());

    c.bench_function("metrics_concurrent_record", |b| {
        b.iter(|| {
            let metrics_clone = metrics.clone();
            let result = RequestResult {
                start_time: Instant::now(),
                duration: Duration::from_millis(100),
                status_code: Some(200),
                success: true,
                error: None,
                target_id: String::from("test"),
            };
            metrics_clone.record(black_box(result))
        })
    });
}

criterion_group!(
    benches,
    bench_metrics_record,
    bench_metrics_record_with_error,
    bench_metrics_snapshot,
    bench_metrics_concurrent
);
criterion_main!(benches);
