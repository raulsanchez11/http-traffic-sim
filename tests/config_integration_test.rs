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
    assert_eq!(target.headers.get("Content-Type").unwrap(), "application/json");
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
fn test_authorization_config() {
    let auth = AuthorizationConfig {
        confirmed: true,
        target_owner: Some("Security Team".to_string()),
        authorization_notes: Some("Load test approved".to_string()),
    };

    assert!(auth.confirmed);
    assert!(auth.target_owner.is_some());
    assert!(auth.authorization_notes.is_some());
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
        TrafficPattern::Burst { size, interval_secs, .. } => {
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
        StressPattern::ConnectionFlood { connections_per_second, .. } => {
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
