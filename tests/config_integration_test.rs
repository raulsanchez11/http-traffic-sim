//! Integration tests for configuration loading and validation.

use http_traffic_sim::config::*;
use std::collections::HashMap;

#[test]
fn test_default_config_creation() {
    let config = ConfigFile::default();

    assert_eq!(config.target.method, "GET");
    assert!(config.target.url.is_empty());
    assert_eq!(config.client.timeout_secs, 30);
    assert_eq!(config.client.pool_max_idle_per_host, 128);
}

#[test]
fn test_traffic_pattern_defaults() {
    let pattern = TrafficPattern::default();

    match pattern {
        TrafficPattern::Fixed {
            concurrent,
            duration_secs,
            total_requests,
        } => {
            assert_eq!(concurrent, 10);
            assert_eq!(duration_secs, Some(30));
            assert_eq!(total_requests, None);
        }
        _ => panic!("Default pattern should be Fixed"),
    }
}

#[test]
fn test_target_config_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());

    let target = TargetConfig {
        id: "test-target".to_string(),
        url: "https://example.com/api".to_string(),
        method: "POST".to_string(),
        headers: headers.clone(),
        body: Some(r#"{"test": "data"}"#.to_string()),
        discovery: None,
    };

    assert_eq!(target.id, "test-target");
    assert_eq!(target.url, "https://example.com/api");
    assert_eq!(target.method, "POST");
    assert_eq!(target.headers.len(), 2);
    assert_eq!(
        target.headers.get("Content-Type").unwrap(),
        "application/json"
    );
    assert!(target.body.is_some());
}

#[test]
fn test_safety_limits_validation() {
    let limits = SafetyLimits {
        max_connections_per_second: Some(1000),
        max_requests_per_second: Some(5000),
        max_payload_size_mb: Some(100),
        max_concurrent_connections: Some(500),
    };

    assert_eq!(limits.max_connections_per_second, Some(1000));
    assert_eq!(limits.max_requests_per_second, Some(5000));
    assert_eq!(limits.max_payload_size_mb, Some(100));
    assert_eq!(limits.max_concurrent_connections, Some(500));
}

#[test]
fn test_traffic_patterns_all_variants() {
    // Fixed pattern
    let fixed = TrafficPattern::Fixed {
        concurrent: 50,
        duration_secs: Some(60),
        total_requests: None,
    };
    match fixed {
        TrafficPattern::Fixed { concurrent, .. } => assert_eq!(concurrent, 50),
        _ => panic!("Should be Fixed"),
    }

    // Rate limit pattern
    let rate_limit = TrafficPattern::RateLimit {
        rate: 100,
        duration_secs: Some(30),
        total_requests: None,
    };
    match rate_limit {
        TrafficPattern::RateLimit { rate, .. } => assert_eq!(rate, 100),
        _ => panic!("Should be RateLimit"),
    }

    // Ramp pattern
    let ramp = TrafficPattern::Ramp {
        from: 10,
        to: 100,
        ramp_duration_secs: 60,
        hold_duration_secs: Some(30),
    };
    match ramp {
        TrafficPattern::Ramp { from, to, .. } => {
            assert_eq!(from, 10);
            assert_eq!(to, 100);
        }
        _ => panic!("Should be Ramp"),
    }

    // Burst pattern
    let burst = TrafficPattern::Burst {
        size: 100,
        interval_secs: 5,
        duration_secs: Some(60),
        total_bursts: None,
    };
    match burst {
        TrafficPattern::Burst {
            size,
            interval_secs,
            ..
        } => {
            assert_eq!(size, 100);
            assert_eq!(interval_secs, 5);
        }
        _ => panic!("Should be Burst"),
    }
}

#[test]
fn test_load_distribution_variants() {
    // Round-robin
    let rr = LoadDistribution::RoundRobin;
    match rr {
        LoadDistribution::RoundRobin => (),
        _ => panic!("Should be RoundRobin"),
    }

    // Weighted
    let weighted = LoadDistribution::Weighted {
        weights: vec![0.7, 0.3],
    };
    match weighted {
        LoadDistribution::Weighted { weights } => {
            assert_eq!(weights.len(), 2);
            assert_eq!(weights[0], 0.7);
        }
        _ => panic!("Should be Weighted"),
    }

    // Random
    let random = LoadDistribution::Random;
    match random {
        LoadDistribution::Random => (),
        _ => panic!("Should be Random"),
    }

    // Hash
    let hash = LoadDistribution::Hash {
        field: HashField::SourceIp,
    };
    match hash {
        LoadDistribution::Hash { field } => match field {
            HashField::SourceIp => (),
            _ => panic!("Should be SourceIp"),
        },
        _ => panic!("Should be Hash"),
    }
}

#[test]
fn test_stress_patterns() {
    // Connection flood
    let conn_flood = StressPattern::ConnectionFlood {
        connections_per_second: 100,
        hold_time_ms: 5000,
        duration_secs: 60,
    };
    match conn_flood {
        StressPattern::ConnectionFlood {
            connections_per_second,
            ..
        } => {
            assert_eq!(connections_per_second, 100);
        }
        _ => panic!("Should be ConnectionFlood"),
    }

    // Request flood
    let req_flood = StressPattern::RequestFlood {
        target_rps: 1000,
        duration_secs: 60,
    };
    match req_flood {
        StressPattern::RequestFlood { target_rps, .. } => {
            assert_eq!(target_rps, 1000);
        }
        _ => panic!("Should be RequestFlood"),
    }

    // Slowloris
    let slowloris = StressPattern::Slowloris {
        connections: 100,
        headers_per_second: 0.5,
        duration_secs: 300,
    };
    match slowloris {
        StressPattern::Slowloris { connections, .. } => {
            assert_eq!(connections, 100);
        }
        _ => panic!("Should be Slowloris"),
    }
}

#[test]
fn test_client_config_defaults() {
    let client = ClientConfig::default();

    assert_eq!(client.timeout_secs, 30);
    assert_eq!(client.pool_max_idle_per_host, 128);
    assert!(!client.http2_prior_knowledge);
}

#[test]
fn test_output_config() {
    let output = OutputConfig {
        file: Some(std::path::PathBuf::from("results.json")),
        console: true,
        realtime_updates: true,
    };

    assert!(output.file.is_some());
    assert!(output.console);
    assert!(output.realtime_updates);
}

// Dedicated unit tests for TrafficPattern::describe() and validate (PR 1)
#[test]
fn test_traffic_pattern_describe() {
    let fixed = TrafficPattern::Fixed {
        concurrent: 50,
        duration_secs: Some(60),
        total_requests: None,
    };
    let d = fixed.describe();
    assert!(d.contains("Pattern:               Fixed Concurrency"));
    assert!(d.contains("Concurrent Clients:    50"));
    assert!(d.contains("Duration:              60s"));

    let rate = TrafficPattern::RateLimit {
        rate: 100,
        duration_secs: Some(30),
        total_requests: None,
    };
    let d = rate.describe();
    assert!(d.contains("Pattern:               Rate Limited"));
    assert!(d.contains("Rate:                  100 req/s"));

    let ramp = TrafficPattern::Ramp {
        from: 10,
        to: 100,
        ramp_duration_secs: 60,
        hold_duration_secs: Some(30),
    };
    let d = ramp.describe();
    assert!(d.contains("Pattern:               Ramp-up"));
    assert!(d.contains("From:                  10 clients"));
    assert!(d.contains("To:                    100 clients"));
    assert!(d.contains("Ramp Duration:         60s"));
    assert!(d.contains("Hold Duration:         30s"));

    let burst = TrafficPattern::Burst {
        size: 100,
        interval_secs: 5,
        duration_secs: Some(60),
        total_bursts: None,
    };
    let d = burst.describe();
    assert!(d.contains("Pattern:               Burst"));
    assert!(d.contains("Burst Size:            100 requests"));
}

#[test]
fn test_traffic_pattern_validate() {
    let good_rate = TrafficPattern::RateLimit {
        rate: 10,
        duration_secs: None,
        total_requests: None,
    };
    assert!(good_rate.validate().is_ok());

    let bad_rate = TrafficPattern::RateLimit {
        rate: 0,
        duration_secs: None,
        total_requests: None,
    };
    let err = bad_rate.validate().unwrap_err();
    assert!(err
        .to_string()
        .contains("Rate limit must be at least 1 request per second"));

    let good_ramp = TrafficPattern::Ramp {
        from: 5,
        to: 10,
        ramp_duration_secs: 30,
        hold_duration_secs: None,
    };
    assert!(good_ramp.validate().is_ok());

    let bad_ramp = TrafficPattern::Ramp {
        from: 10,
        to: 5,
        ramp_duration_secs: 30,
        hold_duration_secs: None,
    };
    let err = bad_ramp.validate().unwrap_err();
    assert!(err.to_string().contains("must be <= 'to'"));
}

// Dedicated unit tests for StressPattern::describe() and validate_against (PR 2)
#[test]
fn test_stress_pattern_describe() {
    let flood = StressPattern::ConnectionFlood {
        connections_per_second: 100,
        hold_time_ms: 5000,
        duration_secs: 60,
    };
    assert_eq!(
        flood.describe(),
        "Connection Flood - 100 conn/s, hold 5000ms, duration 60s"
    );

    let slowloris = StressPattern::Slowloris {
        connections: 50,
        headers_per_second: 0.5,
        duration_secs: 300,
    };
    assert_eq!(
        slowloris.describe(),
        "Slowloris - 50 connections, 0.50 headers/s, duration 300s"
    );

    let slowpost = StressPattern::SlowPost {
        connections: 20,
        bytes_per_second: 1024,
        payload_size: 4096,
    };
    assert_eq!(
        slowpost.describe(),
        "Slow POST - 20 connections, 1024 bytes/s, payload 4096 bytes"
    );

    let reqflood = StressPattern::RequestFlood {
        target_rps: 1000,
        duration_secs: 60,
    };
    assert_eq!(
        reqflood.describe(),
        "Request Flood - 1000 req/s, duration 60s"
    );

    let large = StressPattern::LargePayload {
        size_mb: 100,
        concurrent: 10,
        duration_secs: 30,
    };
    assert_eq!(
        large.describe(),
        "Large Payload - 100 MB, 10 concurrent, duration 30s"
    );

    let pipeline = StressPattern::PipelineAbuse {
        requests_per_connection: 100,
        concurrent_connections: 5,
    };
    assert_eq!(
        pipeline.describe(),
        "Pipeline Abuse - 100 req/conn, 5 connections"
    );

    let slowread = StressPattern::SlowRead {
        connections: 10,
        read_rate_bps: 1024,
        duration_secs: 60,
    };
    assert_eq!(
        slowread.describe(),
        "Slow Read - 10 connections, 1024 bytes/s, duration 60s"
    );
}

#[test]
fn test_stress_pattern_validate_against() {
    let limits = SafetyLimits {
        max_connections_per_second: Some(50),
        max_requests_per_second: Some(500),
        max_payload_size_mb: Some(10),
        max_concurrent_connections: Some(20),
    };

    let bad_flood = StressPattern::ConnectionFlood {
        connections_per_second: 100,
        hold_time_ms: 1000,
        duration_secs: 10,
    };
    let err = bad_flood.validate_against(&limits).unwrap_err();
    assert!(err
        .to_string()
        .contains("exceeds safety limit of 50 conn/s"));

    let good_flood = StressPattern::ConnectionFlood {
        connections_per_second: 40,
        hold_time_ms: 1000,
        duration_secs: 10,
    };
    assert!(good_flood.validate_against(&limits).is_ok());

    let bad_large = StressPattern::LargePayload {
        size_mb: 50,
        concurrent: 5,
        duration_secs: 10,
    };
    let err = bad_large.validate_against(&limits).unwrap_err();
    assert!(err.to_string().contains("exceeds safety limit of 10 MB"));

    let bad_conns = StressPattern::Slowloris {
        connections: 30,
        headers_per_second: 1.0,
        duration_secs: 10,
    };
    let err = bad_conns.validate_against(&limits).unwrap_err();
    assert!(err.to_string().contains("exceeds safety limit of 20"));
}

// Regression test for default target ID assignment via effective_id (PR 3)
#[test]
fn test_target_config_effective_id_regression() {
    let empty = TargetConfig {
        id: String::new(),
        ..Default::default()
    };
    assert_eq!(empty.effective_id(None), "target");
    assert_eq!(empty.effective_id(Some(3)), "target-3");

    let named = TargetConfig {
        id: "api1".to_string(),
        ..Default::default()
    };
    assert_eq!(named.effective_id(Some(0)), "api1");
    assert_eq!(named.effective_id(None), "api1"); // named takes precedence

    // Simulate the multi-target mutation site behavior (as in execute_multi_target)
    let mut targets = [
        TargetConfig {
            id: String::new(),
            ..Default::default()
        },
        TargetConfig {
            id: "custom".to_string(),
            ..Default::default()
        },
        TargetConfig {
            id: String::new(),
            ..Default::default()
        },
    ];
    for (i, t) in targets.iter_mut().enumerate() {
        t.id = t.effective_id(Some(i));
    }
    assert_eq!(targets[0].id, "target-0");
    assert_eq!(targets[1].id, "custom");
    assert_eq!(targets[2].id, "target-2");
}
