//! Integration tests for HTTP client functionality.

use http_traffic_sim::client::HttpClient;
use http_traffic_sim::config::TargetConfig;
use std::collections::HashMap;
use std::time::Duration;

fn create_test_target(url: &str, method: &str) -> TargetConfig {
    TargetConfig {
        id: "test-target".to_string(),
        url: url.to_string(),
        method: method.to_string(),
        headers: HashMap::new(),
        body: None,
        discovery: None,
    }
}

#[test]
fn test_client_creation() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let result = HttpClient::new(target, Duration::from_secs(30), 128);

    assert!(result.is_ok());
}

#[test]
fn test_client_creation_with_custom_timeout() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let timeout = Duration::from_secs(60);
    let result = HttpClient::new(target, timeout, 128);

    assert!(result.is_ok());
}

#[test]
fn test_client_creation_with_custom_pool_size() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let result = HttpClient::new(target, Duration::from_secs(30), 256);

    assert!(result.is_ok());
}

#[test]
fn test_target_config_with_custom_method() {
    let target = create_test_target("https://httpbin.org/post", "POST");

    assert_eq!(target.method, "POST");
    assert_eq!(target.url, "https://httpbin.org/post");
}

#[test]
fn test_target_config_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), "http-traffic-sim/1.0".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let target = TargetConfig {
        id: "test".to_string(),
        url: "https://httpbin.org/get".to_string(),
        method: "GET".to_string(),
        headers: headers.clone(),
        body: None,
        discovery: None,
    };

    assert_eq!(target.headers.len(), 2);
    assert_eq!(target.headers.get("User-Agent").unwrap(), "http-traffic-sim/1.0");
    assert_eq!(target.headers.get("Accept").unwrap(), "application/json");
}

#[test]
fn test_target_config_with_body() {
    let body = r#"{"key": "value", "number": 42}"#.to_string();

    let target = TargetConfig {
        id: "test".to_string(),
        url: "https://httpbin.org/post".to_string(),
        method: "POST".to_string(),
        headers: HashMap::new(),
        body: Some(body.clone()),
        discovery: None,
    };

    assert!(target.body.is_some());
    assert_eq!(target.body.unwrap(), body);
}

#[test]
fn test_client_supports_https() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let result = HttpClient::new(target, Duration::from_secs(30), 128);

    assert!(result.is_ok());
}

#[test]
fn test_client_supports_http() {
    let target = create_test_target("http://httpbin.org/get", "GET");
    let result = HttpClient::new(target, Duration::from_secs(30), 128);

    assert!(result.is_ok());
}

#[test]
fn test_multiple_clients_with_different_configs() {
    let target1 = create_test_target("https://httpbin.org/get", "GET");
    let target2 = create_test_target("https://httpbin.org/post", "POST");

    let client1 = HttpClient::new(target1, Duration::from_secs(30), 128);
    let client2 = HttpClient::new(target2, Duration::from_secs(60), 256);

    assert!(client1.is_ok());
    assert!(client2.is_ok());
}

#[test]
fn test_client_with_minimal_pool() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let result = HttpClient::new(target, Duration::from_secs(30), 1);

    assert!(result.is_ok());
}

#[test]
fn test_client_with_large_pool() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let result = HttpClient::new(target, Duration::from_secs(30), 1000);

    assert!(result.is_ok());
}

#[test]
fn test_target_config_cloning() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let cloned = target.clone();

    assert_eq!(target.id, cloned.id);
    assert_eq!(target.url, cloned.url);
    assert_eq!(target.method, cloned.method);
}

#[test]
fn test_target_config_different_http_methods() {
    let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

    for method in methods {
        let target = create_test_target("https://httpbin.org/anything", method);
        assert_eq!(target.method, method);
    }
}

#[test]
fn test_target_config_with_query_parameters() {
    let target = create_test_target(
        "https://httpbin.org/get?param1=value1&param2=value2",
        "GET"
    );

    assert!(target.url.contains("param1=value1"));
    assert!(target.url.contains("param2=value2"));
}

#[test]
fn test_target_config_with_path_parameters() {
    let target = create_test_target("https://httpbin.org/status/200", "GET");

    assert!(target.url.contains("/status/200"));
}

#[test]
fn test_target_config_with_fragment() {
    let target = create_test_target("https://httpbin.org/get#section", "GET");

    assert!(target.url.contains("#section"));
}

#[test]
fn test_client_timeout_configuration() {
    let target = create_test_target("https://httpbin.org/get", "GET");

    // Test various timeout values
    let timeouts = vec![
        Duration::from_secs(1),
        Duration::from_secs(10),
        Duration::from_secs(30),
        Duration::from_secs(60),
        Duration::from_secs(120),
    ];

    for timeout in timeouts {
        let result = HttpClient::new(target.clone(), timeout, 128);
        assert!(result.is_ok(), "Failed for timeout: {:?}", timeout);
    }
}

#[test]
fn test_target_id_uniqueness() {
    let target1 = TargetConfig {
        id: "target-1".to_string(),
        url: "https://httpbin.org/get".to_string(),
        method: "GET".to_string(),
        headers: HashMap::new(),
        body: None,
        discovery: None,
    };

    let target2 = TargetConfig {
        id: "target-2".to_string(),
        url: "https://httpbin.org/get".to_string(),
        method: "GET".to_string(),
        headers: HashMap::new(),
        body: None,
        discovery: None,
    };

    assert_ne!(target1.id, target2.id);
}

#[test]
fn test_target_config_with_complex_json_body() {
    let body = r#"{
        "user": {
            "name": "Test User",
            "email": "test@example.com",
            "age": 30
        },
        "items": [1, 2, 3, 4, 5],
        "metadata": {
            "timestamp": "2024-01-01T00:00:00Z",
            "version": "1.0"
        }
    }"#.to_string();

    let target = TargetConfig {
        id: "test".to_string(),
        url: "https://httpbin.org/post".to_string(),
        method: "POST".to_string(),
        headers: HashMap::new(),
        body: Some(body.clone()),
        discovery: None,
    };

    assert!(target.body.is_some());
    let body_content = target.body.unwrap();
    assert!(body_content.contains("Test User"));
    assert!(body_content.contains("test@example.com"));
    assert!(body_content.contains("items"));
}

#[test]
fn test_target_config_debug_format() {
    let target = create_test_target("https://httpbin.org/get", "GET");
    let debug_str = format!("{:?}", target);

    assert!(debug_str.contains("TargetConfig"));
    assert!(debug_str.contains("httpbin.org"));
}

// Note: Actual HTTP execution tests would require a running test server
// or mock server. These tests focus on client construction and configuration.
