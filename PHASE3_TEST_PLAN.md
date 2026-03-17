# Phase 3: Port Discovery - Comprehensive Test Plan

## Test Plan Overview

**Purpose**: Validate all port discovery functionality including connectivity checks, service detection, failure handling, and integration with load testing.

**Scope**: All Phase 3 features including discovery modes, failure actions, multi-target support, and backward compatibility.

**Test Environment**:
- Local machine (macOS/Linux/Windows)
- Public endpoints (google.com, httpbin.org)
- Local test servers (HTTP/HTTPS)
- Docker containers (optional)

## Test Categories

1. [Unit Tests](#1-unit-tests)
2. [Integration Tests](#2-integration-tests)
3. [Functional Tests](#3-functional-tests)
4. [Performance Tests](#4-performance-tests)
5. [Security Tests](#5-security-tests)
6. [Edge Case Tests](#6-edge-case-tests)
7. [Regression Tests](#7-regression-tests)
8. [Multi-Target Tests](#8-multi-target-tests)
9. [Failure Scenario Tests](#9-failure-scenario-tests)
10. [Real-World Scenario Tests](#10-real-world-scenario-tests)

---

## 1. Unit Tests

### 1.1 Port Specification Parsing

**Test Case**: `test_port_spec_single`
- **Input**: `PortSpec::Single(8080)`
- **Expected**: `vec![8080]`
- **Status**: ✅ Passing

**Test Case**: `test_port_spec_list`
- **Input**: `PortSpec::List(vec![80, 443, 8080])`
- **Expected**: `vec![80, 443, 8080]`
- **Status**: ✅ Passing

**Test Case**: `test_port_spec_range`
- **Input**: `PortSpec::Range { start: 8000, end: 8003 }`
- **Expected**: `vec![8000, 8001, 8002, 8003]`
- **Status**: ✅ Passing

**Test Case**: `test_port_spec_large_range`
- **Input**: `PortSpec::Range { start: 8000, end: 9000 }`
- **Expected**: 1001 ports in sequence
- **Status**: TODO

### 1.2 URL Parsing

**Test Case**: `test_extract_host`
- **Input**: `"https://example.com/path"`
- **Expected**: `"example.com"`
- **Status**: ✅ Passing

**Test Case**: `test_extract_host_with_port`
- **Input**: `"http://api.example.com:8080/"`
- **Expected**: `"api.example.com"`
- **Status**: ✅ Passing

**Test Case**: `test_extract_port_https_default`
- **Input**: `"https://example.com/path"`
- **Expected**: `443`
- **Status**: ✅ Passing

**Test Case**: `test_extract_port_http_default`
- **Input**: `"http://example.com/path"`
- **Expected**: `80`
- **Status**: ✅ Passing

**Test Case**: `test_extract_port_explicit`
- **Input**: `"https://example.com:8443/path"`
- **Expected**: `8443`
- **Status**: ✅ Passing

**Test Case**: `test_extract_host_invalid_url`
- **Input**: `"not-a-url"`
- **Expected**: Error
- **Status**: TODO

### 1.3 Configuration Parsing

**Test Case**: `test_parse_discovery_config_minimal`
- **Input**: YAML with only `enabled: true`
- **Expected**: Uses all defaults
- **Status**: TODO

**Test Case**: `test_parse_discovery_config_full`
- **Input**: YAML with all fields
- **Expected**: All fields correctly parsed
- **Status**: TODO

**Test Case**: `test_parse_discovery_modes`
- **Input**: Each mode (validate/scan/both)
- **Expected**: Correct enum variant
- **Status**: TODO

**Test Case**: `test_parse_failure_actions`
- **Input**: Each action (fail/skip/warn)
- **Expected**: Correct enum variant
- **Status**: TODO

---

## 2. Integration Tests

### 2.1 TCP Connectivity

**Test Case**: `test_tcp_connect_open_port`
- **Setup**: Public HTTPS endpoint (google.com:443)
- **Action**: Check TCP connectivity
- **Expected**: PortStatus::Open, response_time < 1000ms
- **Status**: ✅ Passing (manual)

**Test Case**: `test_tcp_connect_closed_port`
- **Setup**: Known closed port (google.com:9999)
- **Action**: Check TCP connectivity
- **Expected**: PortStatus::Closed or timeout
- **Status**: ✅ Passing (manual)

**Test Case**: `test_tcp_connect_filtered_port`
- **Setup**: Filtered port (firewalled)
- **Action**: Check TCP connectivity
- **Expected**: Timeout or filtered status
- **Status**: TODO

**Test Case**: `test_tcp_connect_with_retries`
- **Setup**: Intermittent endpoint
- **Action**: Check with retries enabled
- **Expected**: Eventually succeeds or all retries exhausted
- **Status**: TODO

### 2.2 Service Detection

**Test Case**: `test_detect_https_service`
- **Setup**: Known HTTPS endpoint (google.com:443)
- **Action**: Detect service type
- **Expected**: ServiceType::Https
- **Status**: ✅ Passing (manual)

**Test Case**: `test_detect_http_service`
- **Setup**: Known HTTP endpoint (google.com:80)
- **Action**: Detect service type
- **Expected**: ServiceType::Http
- **Status**: ✅ Passing (manual)

**Test Case**: `test_detect_https_with_invalid_cert`
- **Setup**: HTTPS endpoint with self-signed cert
- **Action**: Detect service type
- **Expected**: ServiceType::Https (cert validation bypassed)
- **Status**: TODO

**Test Case**: `test_detect_non_http_service`
- **Setup**: SSH port (22)
- **Action**: Detect service type
- **Expected**: ServiceType::Unknown or None
- **Status**: TODO

### 2.3 Port Scanning

**Test Case**: `test_scan_single_port`
- **Setup**: Single port config
- **Action**: Scan port
- **Expected**: Result for that port only
- **Status**: ✅ Passing (manual)

**Test Case**: `test_scan_multiple_ports`
- **Setup**: Array of ports [80, 443, 8080]
- **Action**: Scan all ports
- **Expected**: Results for all ports, concurrent execution
- **Status**: ✅ Passing (manual)

**Test Case**: `test_scan_port_range_small`
- **Setup**: Range 8080-8085
- **Action**: Scan range
- **Expected**: 6 results, all tested
- **Status**: TODO

**Test Case**: `test_scan_port_range_large`
- **Setup**: Range 8000-9000
- **Action**: Scan range
- **Expected**: 1001 results, reasonable duration (<10s)
- **Status**: TODO

**Test Case**: `test_scan_concurrency_limit`
- **Setup**: 100 ports
- **Action**: Monitor concurrent connections
- **Expected**: Never exceeds 10 concurrent
- **Status**: TODO

---

## 3. Functional Tests

### 3.1 Discovery Modes

**Test Case**: `test_validate_mode_success`
- **Config**: mode: validate, ports: 443
- **Target**: google.com
- **Expected**: Port validated, test continues
- **Status**: ✅ Passing (manual)

**Test Case**: `test_validate_mode_failure`
- **Config**: mode: validate, ports: 9999, on_failure: fail
- **Target**: google.com
- **Expected**: Discovery fails, test stops
- **Status**: ✅ Passing (manual)

**Test Case**: `test_scan_mode_finds_services`
- **Config**: mode: scan, ports: {start: 8000, end: 8100}
- **Target**: Server with HTTP on 8080
- **Expected**: Discovers port 8080
- **Status**: TODO

**Test Case**: `test_both_mode_validates_and_scans`
- **Config**: mode: both, ports: [443, {start: 8000, end: 8100}]
- **Target**: Server with services on 443 and 8080
- **Expected**: Validates 443, discovers 8080
- **Status**: TODO

### 3.2 Failure Handling

**Test Case**: `test_on_failure_fail`
- **Config**: on_failure: fail, unreachable port
- **Expected**: Exit code 1, clear error message
- **Status**: ✅ Passing (manual)

**Test Case**: `test_on_failure_skip`
- **Config**: on_failure: skip, some ports fail
- **Expected**: Warning logged, continues with working ports
- **Status**: TODO

**Test Case**: `test_on_failure_warn`
- **Config**: on_failure: warn, some ports fail
- **Expected**: Warning logged, continues with all ports
- **Status**: ✅ Passing (manual)

**Test Case**: `test_all_ports_fail_skip_mode`
- **Config**: on_failure: skip, all ports fail
- **Expected**: Warning logged, skips target
- **Status**: TODO

### 3.3 URL Auto-Update

**Test Case**: `test_url_update_prefers_https`
- **Setup**: Discovers both HTTP (80) and HTTPS (443)
- **Expected**: URL updated to use HTTPS (443)
- **Status**: TODO

**Test Case**: `test_url_update_http_only`
- **Setup**: Discovers only HTTP (80)
- **Expected**: URL updated to use HTTP (80)
- **Status**: TODO

**Test Case**: `test_url_update_preserves_path`
- **Setup**: Original URL has path /api/health
- **Expected**: Updated URL preserves /api/health
- **Status**: TODO

**Test Case**: `test_url_update_preserves_query`
- **Setup**: Original URL has query ?key=value
- **Expected**: Updated URL preserves ?key=value
- **Status**: TODO

---

## 4. Performance Tests

### 4.1 Discovery Speed

**Test Case**: `test_single_port_check_speed`
- **Setup**: Single port validation
- **Expected**: < 100ms for open port, < timeout for closed
- **Status**: ✅ Passing (26-60ms observed)

**Test Case**: `test_multi_port_parallel_speed`
- **Setup**: 10 ports
- **Expected**: ~same time as single port (parallel)
- **Status**: TODO

**Test Case**: `test_port_range_scan_speed`
- **Setup**: 100 ports
- **Expected**: < 5 seconds
- **Status**: TODO

**Test Case**: `test_multi_target_discovery_speed`
- **Setup**: 10 targets, each with 5 ports
- **Expected**: All discovered in < 5 seconds (parallel)
- **Status**: TODO

### 4.2 Resource Usage

**Test Case**: `test_memory_usage_large_scan`
- **Setup**: Scan 1000 ports
- **Expected**: Memory usage < 100MB
- **Status**: TODO

**Test Case**: `test_connection_cleanup`
- **Setup**: Scan many ports
- **Expected**: All connections properly closed
- **Status**: TODO

**Test Case**: `test_concurrent_connection_limit`
- **Setup**: Scan 100 ports
- **Expected**: Never exceeds 10 concurrent TCP connections
- **Status**: TODO

---

## 5. Security Tests

### 5.1 TLS Validation

**Test Case**: `test_discovery_accepts_invalid_certs`
- **Setup**: HTTPS with self-signed cert
- **Expected**: Discovery succeeds (relaxed validation)
- **Status**: TODO

**Test Case**: `test_load_test_validates_certs`
- **Setup**: After discovery, run load test
- **Expected**: Load test uses normal validation (strict)
- **Status**: TODO

### 5.2 Input Validation

**Test Case**: `test_port_range_validation`
- **Setup**: Invalid range (start > end)
- **Expected**: Error during config parsing
- **Status**: TODO

**Test Case**: `test_port_number_validation`
- **Setup**: Port > 65535
- **Expected**: Error during config parsing
- **Status**: TODO

**Test Case**: `test_timeout_validation`
- **Setup**: Negative timeout
- **Expected**: Error during config parsing
- **Status**: TODO

**Test Case**: `test_invalid_url_handling`
- **Setup**: Malformed URL
- **Expected**: Clear error message
- **Status**: TODO

---

## 6. Edge Case Tests

### 6.1 Network Conditions

**Test Case**: `test_dns_resolution_failure`
- **Setup**: Non-existent domain
- **Expected**: DNS error reported, handled gracefully
- **Status**: TODO

**Test Case**: `test_network_timeout`
- **Setup**: Very slow endpoint
- **Expected**: Timeout respected, retries work
- **Status**: TODO

**Test Case**: `test_connection_refused`
- **Setup**: Closed port
- **Expected**: Connection refused reported
- **Status**: TODO

**Test Case**: `test_connection_reset`
- **Setup**: Port that resets connections
- **Expected**: Reset reported, handled gracefully
- **Status**: TODO

### 6.2 Configuration Edge Cases

**Test Case**: `test_empty_port_list`
- **Setup**: ports: []
- **Expected**: Error or no-op
- **Status**: TODO

**Test Case**: `test_zero_timeout`
- **Setup**: timeout_ms: 0
- **Expected**: Error or immediate timeout
- **Status**: TODO

**Test Case**: `test_zero_retries`
- **Setup**: retries: 0
- **Expected**: Single attempt only
- **Status**: TODO

**Test Case**: `test_discovery_disabled`
- **Setup**: enabled: false
- **Expected**: Discovery skipped, no impact
- **Status**: TODO

### 6.3 Service Detection Edge Cases

**Test Case**: `test_http_on_non_standard_port`
- **Setup**: HTTP service on port 8888
- **Expected**: Correctly detected as HTTP
- **Status**: TODO

**Test Case**: `test_https_on_non_standard_port`
- **Setup**: HTTPS service on port 8443
- **Expected**: Correctly detected as HTTPS
- **Status**: TODO

**Test Case**: `test_service_redirects`
- **Setup**: Service that redirects
- **Expected**: Follows redirect, detects service
- **Status**: TODO

**Test Case**: `test_service_404_response`
- **Setup**: Service returns 404
- **Expected**: Still detected as HTTP/HTTPS (service exists)
- **Status**: TODO

---

## 7. Regression Tests

### 7.1 Backward Compatibility

**Test Case**: `test_config_without_discovery`
- **Setup**: Existing config without discovery field
- **Expected**: Works exactly as before
- **Status**: TODO

**Test Case**: `test_single_target_without_discovery`
- **Setup**: Single target, no discovery
- **Expected**: No discovery phase, direct to load test
- **Status**: TODO

**Test Case**: `test_multi_target_without_discovery`
- **Setup**: Multi-target, no discovery
- **Expected**: No discovery phase, direct to load test
- **Status**: TODO

**Test Case**: `test_stress_test_without_discovery`
- **Setup**: Stress pattern, no discovery
- **Expected**: No discovery phase, authorization then test
- **Status**: TODO

### 7.2 Existing Features

**Test Case**: `test_fixed_pattern_still_works`
- **Setup**: Fixed concurrency pattern with discovery
- **Expected**: Discovery then fixed pattern execution
- **Status**: TODO

**Test Case**: `test_rate_limit_still_works`
- **Setup**: Rate limit pattern with discovery
- **Expected**: Discovery then rate limit execution
- **Status**: TODO

**Test Case**: `test_multi_target_roundrobin_still_works`
- **Setup**: Multi-target roundrobin with discovery
- **Expected**: Discovery then roundrobin distribution
- **Status**: TODO

**Test Case**: `test_json_export_still_works`
- **Setup**: Test with JSON export
- **Expected**: JSON file created with all metrics
- **Status**: TODO

---

## 8. Multi-Target Tests

### 8.1 Independent Discovery

**Test Case**: `test_multi_target_independent_discovery`
- **Setup**: 3 targets, each with different discovery config
- **Expected**: Each discovered independently
- **Status**: TODO

**Test Case**: `test_multi_target_mixed_discovery`
- **Setup**: 3 targets: 1 with discovery, 2 without
- **Expected**: Only 1 target discovered, others unchanged
- **Status**: TODO

**Test Case**: `test_multi_target_parallel_discovery`
- **Setup**: 5 targets, all with discovery
- **Expected**: All discovered in parallel (not sequential)
- **Status**: TODO

### 8.2 Failure Handling

**Test Case**: `test_multi_target_one_fails_skip`
- **Setup**: 3 targets, 1 fails, on_failure: skip
- **Expected**: Failed target skipped, others continue
- **Status**: TODO

**Test Case**: `test_multi_target_one_fails_fail`
- **Setup**: 3 targets, 1 fails, on_failure: fail
- **Expected**: Entire test stops with error
- **Status**: TODO

**Test Case**: `test_multi_target_one_fails_warn`
- **Setup**: 3 targets, 1 fails, on_failure: warn
- **Expected**: Warning logged, all targets continue
- **Status**: TODO

**Test Case**: `test_multi_target_all_fail_skip`
- **Setup**: 3 targets, all fail, on_failure: skip
- **Expected**: All skipped, test exits gracefully
- **Status**: TODO

---

## 9. Failure Scenario Tests

### 9.1 Network Failures

**Test Case**: `test_network_unreachable`
- **Setup**: Endpoint with no route
- **Expected**: Network unreachable error, handled gracefully
- **Status**: TODO

**Test Case**: `test_host_unreachable`
- **Setup**: Host that doesn't exist
- **Expected**: Host unreachable error, retries work
- **Status**: TODO

**Test Case**: `test_partial_network_failure`
- **Setup**: Some ports work, others timeout
- **Expected**: Working ports succeed, failed ports reported
- **Status**: TODO

### 9.2 Service Failures

**Test Case**: `test_service_starts_after_discovery`
- **Setup**: Port closed during discovery, opens after
- **Expected**: Discovery fails, load test handles gracefully
- **Status**: TODO

**Test Case**: `test_service_stops_during_discovery`
- **Setup**: Port open initially, closes during scan
- **Expected**: Port reported as failed, handled gracefully
- **Status**: TODO

**Test Case**: `test_service_very_slow_response`
- **Setup**: Service takes > timeout to respond
- **Expected**: Timeout error, retries exhausted
- **Status**: TODO

### 9.3 Configuration Errors

**Test Case**: `test_invalid_mode`
- **Setup**: mode: "invalid"
- **Expected**: Config parsing error
- **Status**: TODO

**Test Case**: `test_invalid_port_spec`
- **Setup**: ports: "invalid"
- **Expected**: Config parsing error
- **Status**: TODO

**Test Case**: `test_conflicting_config`
- **Setup**: mode: validate but no ports specified
- **Expected**: Config validation error
- **Status**: TODO

---

## 10. Real-World Scenario Tests

### 10.1 Production Validation

**Test Case**: `test_production_pre_flight_check`
- **Scenario**: Validate production API before load test
- **Config**: mode: validate, ports: 443, on_failure: fail
- **Expected**: Fails fast if production down
- **Status**: TODO

**Test Case**: `test_production_with_fallback`
- **Scenario**: Check primary and backup endpoints
- **Config**: Multi-target with skip on failure
- **Expected**: Uses working endpoints only
- **Status**: TODO

### 10.2 Development Discovery

**Test Case**: `test_find_dev_server_port`
- **Scenario**: Discover dev server in range 3000-9000
- **Config**: mode: scan, large range
- **Expected**: Finds first HTTP/HTTPS service
- **Status**: TODO

**Test Case**: `test_dev_env_multiple_services`
- **Scenario**: Multiple microservices on different ports
- **Config**: Multi-target with scan mode
- **Expected**: Discovers all services
- **Status**: TODO

### 10.3 Load Balancer Testing

**Test Case**: `test_load_balancer_backend_validation`
- **Scenario**: Validate all backend servers
- **Config**: Multi-target with all backends
- **Expected**: All backends validated before test
- **Status**: TODO

**Test Case**: `test_load_balancer_with_unhealthy_backend`
- **Scenario**: One backend unhealthy
- **Config**: on_failure: skip
- **Expected**: Test continues with healthy backends
- **Status**: TODO

### 10.4 CDN/Edge Testing

**Test Case**: `test_cdn_edge_location_discovery`
- **Scenario**: Discover CDN edge on various ports
- **Config**: mode: both, multiple ports
- **Expected**: Finds best edge location
- **Status**: TODO

**Test Case**: `test_cdn_failover`
- **Scenario**: Primary edge fails, test secondary
- **Config**: Multi-target with primary/secondary
- **Expected**: Fails over to secondary
- **Status**: TODO

### 10.5 Kubernetes/Docker Testing

**Test Case**: `test_k8s_service_discovery`
- **Scenario**: Discover services in K8s cluster
- **Config**: Service hostnames with port scanning
- **Expected**: Discovers all exposed services
- **Status**: TODO

**Test Case**: `test_docker_compose_discovery`
- **Scenario**: Discover services in docker-compose stack
- **Config**: Container names with port ranges
- **Expected**: All containers discovered
- **Status**: TODO

---

## Test Execution Plan

### Phase 1: Core Functionality (Priority 1)
**Estimated Time**: 2-3 hours

1. Run existing unit tests
2. Execute basic integration tests
3. Test all three discovery modes
4. Test all three failure modes
5. Verify URL auto-update

**Success Criteria**:
- All unit tests pass
- Basic discovery works end-to-end
- All modes and failure actions work

### Phase 2: Multi-Target & Edge Cases (Priority 2)
**Estimated Time**: 3-4 hours

1. Multi-target independent discovery
2. Multi-target failure handling
3. Network failure scenarios
4. Configuration edge cases
5. Service detection edge cases

**Success Criteria**:
- Multi-target discovery works correctly
- Edge cases handled gracefully
- No crashes or hangs

### Phase 3: Performance & Security (Priority 2)
**Estimated Time**: 2-3 hours

1. Performance benchmarks
2. Resource usage monitoring
3. Concurrency validation
4. Security validation
5. TLS handling

**Success Criteria**:
- Performance meets targets
- Resource usage acceptable
- Security requirements met

### Phase 4: Real-World Scenarios (Priority 3)
**Estimated Time**: 3-4 hours

1. Production validation scenarios
2. Development discovery scenarios
3. Load balancer testing
4. CDN testing
5. Container orchestration testing

**Success Criteria**:
- Real-world scenarios work
- Integration with existing tools
- User workflows validated

### Phase 5: Regression Testing (Priority 1)
**Estimated Time**: 1-2 hours

1. Test all existing configs without discovery
2. Verify no performance degradation
3. Verify backward compatibility
4. Test all existing features

**Success Criteria**:
- No regressions in existing functionality
- Backward compatibility maintained
- No breaking changes

---

## Test Automation

### Automated Test Suite

Create `tests/discovery_tests.rs`:

```rust
#[cfg(test)]
mod discovery_integration_tests {
    use http_traffic_sim::discovery::*;

    #[tokio::test]
    async fn test_tcp_connect_google_https() {
        // Test against known-good endpoint
    }

    #[tokio::test]
    async fn test_service_detection_http() {
        // Test HTTP detection
    }

    #[tokio::test]
    async fn test_service_detection_https() {
        // Test HTTPS detection
    }

    // ... more tests
}
```

### CI/CD Integration

Add to `.github/workflows/test.yml`:

```yaml
- name: Run discovery tests
  run: cargo test discovery

- name: Run integration tests
  run: |
    cargo build --release
    ./target/release/http-traffic-sim --config config.discovery-test.yaml
```

---

## Test Data & Fixtures

### Test Servers

**HTTP Test Server** (Python):
```python
python3 -m http.server 8080
```

**HTTPS Test Server** (Python):
```python
# Create self-signed cert
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
# Start server
python3 -m http.server 8443 --bind 0.0.0.0 --cert cert.pem
```

**Multiple Port Server** (Docker):
```yaml
version: '3'
services:
  http:
    image: nginx
    ports:
      - "8080:80"
  https:
    image: nginx
    ports:
      - "8443:443"
```

### Test Configurations

Store in `tests/fixtures/`:
- `discovery_validate.yaml`
- `discovery_scan.yaml`
- `discovery_both.yaml`
- `multi_target_discovery.yaml`
- `discovery_fail_modes.yaml`

---

## Success Metrics

### Coverage Targets
- **Unit Test Coverage**: > 80%
- **Integration Test Coverage**: > 70%
- **Edge Case Coverage**: > 60%
- **Real-World Scenarios**: > 5 scenarios

### Performance Targets
- Single port check: < 100ms
- 10 ports (parallel): < 500ms
- 100 ports: < 5 seconds
- Multi-target (5): < 5 seconds

### Quality Targets
- Zero crashes
- Zero hangs
- All errors have clear messages
- All failures handled gracefully

---

## Test Environment Setup

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build project
cargo build --release

# Install testing tools
cargo install cargo-tarpaulin  # Coverage
cargo install cargo-watch      # Auto-testing
```

### Local Test Setup

```bash
# Start test HTTP server
python3 -m http.server 8080 &

# Start test HTTPS server (if needed)
# openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 1 -nodes -subj "/CN=localhost"
# python3 tests/https_server.py &

# Run tests
cargo test

# Run with coverage
cargo tarpaulin --out Html
```

---

## Bug Tracking Template

When issues are found:

```markdown
### Bug Report

**Test Case**: [Test case name]
**Priority**: [High/Medium/Low]
**Status**: [Open/In Progress/Fixed]

**Description**:
[Clear description of the issue]

**Steps to Reproduce**:
1. [Step 1]
2. [Step 2]
3. [Step 3]

**Expected Behavior**:
[What should happen]

**Actual Behavior**:
[What actually happens]

**Configuration**:
```yaml
[Config that triggers the bug]
```

**Logs**:
```
[Relevant log output]
```

**Fix**:
[Description of fix, if known]
```

---

## Test Completion Checklist

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] All discovery modes tested
- [ ] All failure modes tested
- [ ] Multi-target scenarios tested
- [ ] Edge cases handled
- [ ] Performance targets met
- [ ] Security validated
- [ ] Backward compatibility verified
- [ ] Real-world scenarios tested
- [ ] Documentation updated
- [ ] Example configs validated
- [ ] CI/CD pipeline updated
- [ ] Test report generated

---

## Test Report Template

```markdown
# Phase 3 Discovery Test Report

**Date**: [Date]
**Tester**: [Name]
**Version**: [Git commit hash]

## Summary
- Total Tests: [X]
- Passed: [X]
- Failed: [X]
- Skipped: [X]
- Pass Rate: [X%]

## Test Categories
1. Unit Tests: [X/Y passed]
2. Integration Tests: [X/Y passed]
3. Functional Tests: [X/Y passed]
4. Performance Tests: [X/Y passed]
5. Security Tests: [X/Y passed]
6. Edge Cases: [X/Y passed]
7. Regression Tests: [X/Y passed]

## Failed Tests
[List of failed tests with details]

## Performance Results
- Single port: [X]ms
- Multi-port: [X]ms
- Port range: [X]s
- Multi-target: [X]s

## Issues Found
[List of issues with bug IDs]

## Recommendations
[Testing recommendations]

## Sign-off
- [ ] All critical tests passed
- [ ] All blockers resolved
- [ ] Ready for release
```

---

## Next Steps

1. **Implement Missing Unit Tests**: Add tests for edge cases
2. **Setup Test Infrastructure**: Docker containers, test servers
3. **Create Automated Test Suite**: Integration with CI/CD
4. **Performance Benchmarking**: Establish baselines
5. **Security Audit**: Third-party security review
6. **User Acceptance Testing**: Real-world validation
7. **Documentation**: Update with test results

---

## Appendix: Quick Test Commands

```bash
# Run all tests
cargo test

# Run discovery tests only
cargo test discovery

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_port_spec_single

# Run integration tests
cargo test --test '*'

# Run with coverage
cargo tarpaulin

# Watch mode (auto-run on changes)
cargo watch -x test

# Release build tests
cargo test --release

# Benchmark tests
cargo bench
```
