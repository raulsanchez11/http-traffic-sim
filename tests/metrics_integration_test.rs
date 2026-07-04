//! Integration tests for metrics collection and statistics.

use http_traffic_sim::metrics::*;
use http_traffic_sim::stats::Statistics;
use std::time::{Duration, Instant};

#[test]
fn test_metrics_collector_creation() {
    let metrics = MetricsCollector::new();
    let snapshot = metrics.get_snapshot();

    assert_eq!(snapshot.total_requests, 0);
    assert_eq!(snapshot.successful_requests, 0);
    assert_eq!(snapshot.failed_requests, 0);
}

#[test]
fn test_single_successful_request() {
    let metrics = MetricsCollector::new();

    let result = RequestResult {
        start_time: Instant::now(),
        duration: Duration::from_millis(50),
        status_code: Some(200),
        success: true,
        error: None,
        target_id: "test".to_string(),
    };

    metrics.record(result);

    let snapshot = metrics.get_snapshot();
    assert_eq!(snapshot.total_requests, 1);
    assert_eq!(snapshot.successful_requests, 1);
    assert_eq!(snapshot.failed_requests, 0);
    assert_eq!(snapshot.latency_hist.len(), 1);
}

#[test]
fn test_single_failed_request() {
    let metrics = MetricsCollector::new();

    let result = RequestResult {
        start_time: Instant::now(),
        duration: Duration::from_millis(100),
        status_code: None,
        success: false,
        error: Some("Connection refused".to_string()),
        target_id: "test".to_string(),
    };

    metrics.record(result);

    let snapshot = metrics.get_snapshot();
    assert_eq!(snapshot.total_requests, 1);
    assert_eq!(snapshot.successful_requests, 0);
    assert_eq!(snapshot.failed_requests, 1);
}

#[test]
fn test_multiple_requests_with_status_codes() {
    let metrics = MetricsCollector::new();

    // Record various status codes
    for (status, success) in &[
        (200, true),
        (201, true),
        (400, false),
        (500, false),
        (200, true),
    ] {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(50),
            status_code: Some(*status),
            success: *success,
            error: if *success {
                None
            } else {
                Some(format!("HTTP {}", status))
            },
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    let snapshot = metrics.get_snapshot();
    assert_eq!(snapshot.total_requests, 5);
    assert_eq!(snapshot.successful_requests, 3);
    assert_eq!(snapshot.failed_requests, 2);
}

#[test]
fn test_connection_stats_categorization() {
    let conn_stats = ConnectionStats::new();

    // Test each error category
    conn_stats.categorize_and_increment("Connection refused");
    conn_stats.categorize_and_increment("ECONNREFUSED");
    conn_stats.categorize_and_increment("Request timeout");
    conn_stats.categorize_and_increment("ETIMEDOUT");
    conn_stats.categorize_and_increment("Connection reset by peer");
    conn_stats.categorize_and_increment("ECONNRESET");
    conn_stats.categorize_and_increment("TLS handshake failed");
    conn_stats.categorize_and_increment("SSL certificate error");
    conn_stats.categorize_and_increment("DNS resolution failed");
    conn_stats.categorize_and_increment("Unknown error");

    let snapshot = conn_stats.get_snapshot();
    assert_eq!(snapshot.refused_count, 2); // Connection refused + ECONNREFUSED
    assert_eq!(snapshot.timeout_count, 2); // timeout + ETIMEDOUT
    assert_eq!(snapshot.reset_by_peer_count, 2); // reset + ECONNRESET
    assert_eq!(snapshot.tls_handshake_errors, 2); // TLS + SSL
    assert_eq!(snapshot.dns_errors, 1);
    assert_eq!(snapshot.other_errors, 1);
}

#[test]
fn test_metrics_success_and_error_rates() {
    let metrics = MetricsCollector::new();

    // Record 7 successful and 3 failed requests
    for i in 0..10 {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(50),
            status_code: if i < 7 { Some(200) } else { Some(500) },
            success: i < 7,
            error: if i < 7 {
                None
            } else {
                Some("Server error".to_string())
            },
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    let snapshot = metrics.get_snapshot();
    assert_eq!(snapshot.success_rate(), 70.0);
    assert_eq!(snapshot.error_rate(), 30.0);
}

#[test]
fn test_requests_per_second_calculation() {
    let metrics = MetricsCollector::new();

    // Record 100 requests
    for _ in 0..100 {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(10),
            status_code: Some(200),
            success: true,
            error: None,
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    // Sleep briefly to ensure elapsed time > 0
    std::thread::sleep(Duration::from_millis(100));

    let snapshot = metrics.get_snapshot();
    let rps = snapshot.requests_per_second();

    assert!(rps > 0.0);
    assert!(rps < 10_000.0); // Sanity check
    assert_eq!(snapshot.total_requests, 100);
}

#[test]
fn test_multi_target_metrics() {
    let metrics = MultiTargetMetrics::new();

    // Record requests for different targets
    for (target_id, success) in &[
        ("target1", true),
        ("target1", true),
        ("target2", true),
        ("target2", false),
        ("target3", true),
    ] {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(50),
            status_code: if *success { Some(200) } else { Some(500) },
            success: *success,
            error: if *success {
                None
            } else {
                Some("Error".to_string())
            },
            target_id: target_id.to_string(),
        };
        metrics.record(result);
    }

    let global = metrics.get_global_snapshot();
    assert_eq!(global.total_requests, 5);
    assert_eq!(global.successful_requests, 4);

    let per_target = metrics.get_per_target_snapshots();
    assert_eq!(per_target.len(), 3);
    assert_eq!(per_target.get("target1").unwrap().total_requests, 2);
    assert_eq!(per_target.get("target2").unwrap().total_requests, 2);
    assert_eq!(per_target.get("target3").unwrap().total_requests, 1);
}

#[test]
fn test_statistics_from_snapshot() {
    let metrics = MetricsCollector::new();

    // Record some requests with varying latencies
    for ms in &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(*ms),
            status_code: Some(200),
            success: true,
            error: None,
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    std::thread::sleep(Duration::from_millis(100));

    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);

    assert_eq!(stats.total_requests, 10);
    assert_eq!(stats.successful_requests, 10);
    assert_eq!(stats.failed_requests, 0);
    assert_eq!(stats.success_rate, 100.0);

    // Check latency stats are reasonable
    assert!(stats.latency.min_ms >= 9.0 && stats.latency.min_ms <= 11.0);
    assert!(stats.latency.max_ms >= 99.0 && stats.latency.max_ms <= 101.0);
    assert!(stats.latency.mean_ms >= 45.0 && stats.latency.mean_ms <= 65.0);
    assert!(stats.latency.p50_ms >= 40.0 && stats.latency.p50_ms <= 60.0);
}

#[test]
fn test_status_code_distribution() {
    let metrics = MetricsCollector::new();

    // Record various status codes
    let status_codes = vec![200, 200, 200, 201, 201, 400, 404, 500, 500, 503];

    for status in status_codes {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(50),
            status_code: Some(status),
            success: status < 400,
            error: if status < 400 {
                None
            } else {
                Some(format!("HTTP {}", status))
            },
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);

    // Check status code distribution
    let code_200 = stats.status_codes.iter().find(|(code, _)| *code == 200);
    let code_500 = stats.status_codes.iter().find(|(code, _)| *code == 500);

    assert_eq!(code_200.unwrap().1, 3);
    assert_eq!(code_500.unwrap().1, 2);
}

#[test]
fn test_error_distribution() {
    let metrics = MetricsCollector::new();

    // Record various errors
    let errors = vec![
        "Connection refused",
        "Connection refused",
        "Connection refused",
        "Timeout",
        "Timeout",
        "DNS error",
    ];

    for error in errors {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(50),
            status_code: None,
            success: false,
            error: Some(error.to_string()),
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);

    // Check error distribution is sorted by frequency
    assert_eq!(stats.errors.len(), 3);
    assert_eq!(stats.errors[0].0, "Connection refused");
    assert_eq!(stats.errors[0].1, 3);
    assert_eq!(stats.errors[1].0, "Timeout");
    assert_eq!(stats.errors[1].1, 2);
}

#[test]
fn test_latency_percentiles_with_outliers() {
    let metrics = MetricsCollector::new();

    // Most requests are fast (10-20ms), with a few outliers
    for _ in 0..90 {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(15),
            status_code: Some(200),
            success: true,
            error: None,
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    // Add outliers
    for ms in &[500, 1000, 2000] {
        let result = RequestResult {
            start_time: Instant::now(),
            duration: Duration::from_millis(*ms),
            status_code: Some(200),
            success: true,
            error: None,
            target_id: "test".to_string(),
        };
        metrics.record(result);
    }

    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);

    // P50 should be around 15ms (most requests)
    assert!(stats.latency.p50_ms < 20.0);

    // P90 should still be low (90% of requests are fast)
    assert!(stats.latency.p90_ms < 50.0);

    // P99 should show outlier impact
    assert!(stats.latency.p99_ms > 100.0);

    // Max should be the largest outlier
    assert!(stats.latency.max_ms >= 2000.0);
}
