//! Integration tests for port discovery functionality.

use http_traffic_sim::discovery::*;

#[test]
fn test_port_spec_single() {
    let spec = PortSpec::Single(8080);
    let ports = spec.to_vec().unwrap();

    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0], 8080);
}

#[test]
fn test_port_spec_list() {
    let spec = PortSpec::List(vec![80, 443, 8080, 8443]);
    let ports = spec.to_vec().unwrap();

    assert_eq!(ports.len(), 4);
    assert_eq!(ports, vec![80, 443, 8080, 8443]);
}

#[test]
fn test_port_spec_range_small() {
    let spec = PortSpec::Range {
        start: 8000,
        end: 8005,
    };
    let ports = spec.to_vec().unwrap();

    assert_eq!(ports.len(), 6);
    assert_eq!(ports, vec![8000, 8001, 8002, 8003, 8004, 8005]);
}

#[test]
fn test_port_spec_range_large() {
    let spec = PortSpec::Range {
        start: 8000,
        end: 8100,
    };
    let ports = spec.to_vec().unwrap();

    assert_eq!(ports.len(), 101);
    assert_eq!(ports.first().unwrap(), &8000);
    assert_eq!(ports.last().unwrap(), &8100);
}

#[test]
fn test_extract_host_from_url() {
    let test_cases = vec![
        ("https://example.com/path", "example.com"),
        ("http://api.example.com:8080/", "api.example.com"),
        (
            "https://subdomain.example.com/api/v1",
            "subdomain.example.com",
        ),
        ("http://192.168.1.100:3000", "192.168.1.100"),
        ("https://[::1]:8080/test", "[::1]"), // IPv6 includes brackets
    ];

    for (url, expected_host) in test_cases {
        let result = extract_host_from_url(url);
        assert!(result.is_ok(), "Failed to parse URL: {}", url);
        assert_eq!(result.unwrap(), expected_host);
    }
}

#[test]
fn test_extract_host_from_invalid_url() {
    let invalid_urls = vec!["not-a-url", "://missing-scheme", "http://", ""];

    for url in invalid_urls {
        let result = extract_host_from_url(url);
        assert!(result.is_err(), "Should fail for invalid URL: {}", url);
    }
}

#[test]
fn test_discovery_mode_variants() {
    let validate = DiscoveryMode::Validate;
    let scan = DiscoveryMode::Scan;
    let both = DiscoveryMode::Both;

    assert_eq!(validate, DiscoveryMode::Validate);
    assert_eq!(scan, DiscoveryMode::Scan);
    assert_eq!(both, DiscoveryMode::Both);
}

#[test]
fn test_failure_action_variants() {
    let fail = FailureAction::Fail;
    let skip = FailureAction::Skip;
    let warn = FailureAction::Warn;

    assert_eq!(fail, FailureAction::Fail);
    assert_eq!(skip, FailureAction::Skip);
    assert_eq!(warn, FailureAction::Warn);
}

#[test]
fn test_port_status_variants() {
    let open = PortStatus::Open;
    let closed = PortStatus::Closed;
    let filtered = PortStatus::Filtered;

    assert_eq!(open, PortStatus::Open);
    assert_eq!(closed, PortStatus::Closed);
    assert_eq!(filtered, PortStatus::Filtered);
}

#[test]
fn test_service_type_variants() {
    let http = ServiceType::Http;
    let https = ServiceType::Https;
    let unknown = ServiceType::Unknown;

    assert_eq!(http, ServiceType::Http);
    assert_eq!(https, ServiceType::Https);
    assert_eq!(unknown, ServiceType::Unknown);
}

#[test]
fn test_discovery_config_defaults() {
    let config = PortDiscoveryConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.mode, DiscoveryMode::Validate);
    assert_eq!(config.timeout_ms, 2000);
    assert_eq!(config.retries, 2);
    assert_eq!(config.on_failure, FailureAction::Fail);
    assert!(config.detect_service);
    assert!(config.validate_http);
}

#[test]
fn test_discovery_config_custom() {
    let config = PortDiscoveryConfig {
        enabled: true,
        mode: DiscoveryMode::Both,
        ports: PortSpec::Range {
            start: 8000,
            end: 9000,
        },
        timeout_ms: 5000,
        retries: 3,
        on_failure: FailureAction::Skip,
        detect_service: true,
        validate_http: false,
    };

    assert!(config.enabled);
    assert_eq!(config.mode, DiscoveryMode::Both);
    assert_eq!(config.timeout_ms, 5000);
    assert_eq!(config.retries, 3);
    assert_eq!(config.on_failure, FailureAction::Skip);
    assert!(config.detect_service);
    assert!(!config.validate_http);
}

#[test]
fn test_port_info_structure() {
    let port_info = PortInfo {
        port: 8080,
        status: PortStatus::Open,
        service_type: Some(ServiceType::Http),
        response_time_ms: 45.23,
    };

    assert_eq!(port_info.port, 8080);
    assert_eq!(port_info.status, PortStatus::Open);
    assert_eq!(port_info.service_type, Some(ServiceType::Http));
    assert_eq!(port_info.response_time_ms, 45.23);
}

#[test]
fn test_port_failure_structure() {
    let failure = PortFailure {
        port: 8443,
        error: "Connection refused".to_string(),
    };

    assert_eq!(failure.port, 8443);
    assert_eq!(failure.error, "Connection refused");
}

#[test]
fn test_discovery_result_structure() {
    use std::time::Duration;

    let discovered_ports = vec![
        PortInfo {
            port: 80,
            status: PortStatus::Open,
            service_type: Some(ServiceType::Http),
            response_time_ms: 35.5,
        },
        PortInfo {
            port: 443,
            status: PortStatus::Open,
            service_type: Some(ServiceType::Https),
            response_time_ms: 42.3,
        },
    ];

    let failed_ports = vec![PortFailure {
        port: 8080,
        error: "Connection timeout".to_string(),
    }];

    let result = DiscoveryResult {
        target_id: "test-target".to_string(),
        host: "example.com".to_string(),
        discovered_ports: discovered_ports.clone(),
        failed_ports: failed_ports.clone(),
        duration: Duration::from_secs(5),
    };

    assert_eq!(result.target_id, "test-target");
    assert_eq!(result.host, "example.com");
    assert_eq!(result.discovered_ports.len(), 2);
    assert_eq!(result.failed_ports.len(), 1);
    assert_eq!(result.duration, Duration::from_secs(5));
}

#[test]
fn test_port_spec_range_edge_cases() {
    // Single port range
    let spec = PortSpec::Range {
        start: 8080,
        end: 8080,
    };
    let ports = spec.to_vec().unwrap();
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0], 8080);

    // Two port range
    let spec = PortSpec::Range { start: 80, end: 81 };
    let ports = spec.to_vec().unwrap();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports, vec![80, 81]);
}

#[test]
fn test_port_spec_list_deduplication_not_automatic() {
    // Port lists don't automatically deduplicate
    let spec = PortSpec::List(vec![80, 80, 443, 443]);
    let ports = spec.to_vec().unwrap();
    assert_eq!(ports.len(), 4); // Duplicates preserved
}

#[test]
fn test_extract_host_with_ports() {
    let test_cases = vec![
        ("http://example.com:80/path", "example.com"),
        ("https://example.com:443/path", "example.com"),
        ("http://example.com:8080/path", "example.com"),
        ("https://example.com:8443/path", "example.com"),
    ];

    for (url, expected_host) in test_cases {
        let result = extract_host_from_url(url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_host);
    }
}

#[test]
fn test_extract_host_with_ipv6() {
    let test_cases = vec![
        ("http://[2001:db8::1]/path", "[2001:db8::1]"), // IPv6 includes brackets
        ("https://[::1]:8080/path", "[::1]"),
        ("http://[fe80::1]/path", "[fe80::1]"), // IPv6 link-local without zone ID
    ];

    for (url, expected_host) in test_cases {
        let result = extract_host_from_url(url);
        assert!(result.is_ok(), "Failed to parse IPv6 URL: {}", url);
        assert_eq!(result.unwrap(), expected_host);
    }
}

#[test]
fn test_discovery_config_serialization_structure() {
    // Test that the config can be constructed with all fields
    let _config = PortDiscoveryConfig {
        enabled: true,
        mode: DiscoveryMode::Validate,
        ports: PortSpec::Single(8080),
        timeout_ms: 3000,
        retries: 2,
        on_failure: FailureAction::Warn,
        detect_service: true,
        validate_http: true,
    };

    // If this compiles and runs, the structure is valid
}
