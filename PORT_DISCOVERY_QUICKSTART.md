# Port Discovery Quick Start Guide

## What is Port Discovery?

Port discovery validates that your target endpoints are reachable before running expensive load tests. It can also discover available HTTP/HTTPS services on non-standard ports.

## Why Use It?

- ✅ Catch configuration errors early (wrong port, wrong protocol)
- ✅ Discover services on non-standard ports automatically
- ✅ Validate connectivity across multiple environments
- ✅ Auto-detect whether to use HTTP or HTTPS

## Quick Examples

### 1. Validate a Single Port

Check that port 8443 is reachable before testing:

```yaml
target:
  url: "https://api.example.com:8443/health"
  method: "GET"

  discovery:
    enabled: true
    mode: validate
    ports: 8443
    on_failure: fail

pattern:
  type: fixed
  concurrent: 50
  duration_secs: 60
```

Run:
```bash
./target/release/http-traffic-sim --config config.yaml
```

Output:
```
================================================================================
                    PORT DISCOVERY PHASE
================================================================================

Target: target (api.example.com)
Discovery Duration: 0.50s

  Open Ports:
    - Port 8443 [HTTPS] - 26.99ms response

================================================================================
```

### 2. Check Multiple Ports

Validate that any of several ports work:

```yaml
discovery:
  enabled: true
  mode: validate
  ports: [80, 443, 8080, 8443]
  on_failure: warn
```

The tool will:
1. Check all ports in parallel
2. Auto-select HTTPS if available (prefers secure)
3. Continue if at least one port works

### 3. Scan Port Range

Discover what services are available:

```yaml
discovery:
  enabled: true
  mode: scan
  ports:
    start: 8000
    end: 9000
  on_failure: skip
```

The tool will:
1. Scan all ports 8000-9000
2. Identify HTTP vs HTTPS services
3. Use the first HTTPS service found

### 4. Multi-Target Discovery

Different discovery per target:

```yaml
targets:
  distribution:
    strategy: roundrobin
  targets:
    - id: "production"
      url: "https://api.prod.example.com/health"
      discovery:
        enabled: true
        mode: validate
        ports: 443
        on_failure: fail

    - id: "staging"
      url: "http://api.staging.example.com/health"
      discovery:
        enabled: true
        mode: scan
        ports:
          start: 8000
          end: 8010
        on_failure: skip

    - id: "dev"
      url: "http://localhost:3000/health"
      # No discovery for localhost
```

## Discovery Modes

| Mode | Use Case | Example |
|------|----------|---------|
| `validate` | Check explicit ports | Production validation |
| `scan` | Find available services | Development/staging |
| `both` | Validate + scan | Multi-environment testing |

## Failure Handling

| Mode | Behavior | Use Case |
|------|----------|----------|
| `fail` | Stop on error | Production (must be reachable) |
| `skip` | Continue without failed targets | Multi-target failover |
| `warn` | Log warning, continue | Best-effort validation |

## Common Patterns

### Pattern 1: Production Pre-flight Check

Always validate production endpoints before load testing:

```yaml
discovery:
  enabled: true
  mode: validate
  ports: 443
  timeout_ms: 3000
  retries: 2
  on_failure: fail
```

### Pattern 2: Development Auto-Discovery

Find the right port in development:

```yaml
discovery:
  enabled: true
  mode: scan
  ports:
    start: 3000
    end: 9000
  timeout_ms: 1000
  on_failure: skip
```

### Pattern 3: Multi-Environment Testing

Validate each environment before testing:

```yaml
targets:
  targets:
    - id: "prod"
      url: "https://api.prod.com/health"
      discovery:
        enabled: true
        mode: validate
        ports: 443
        on_failure: fail

    - id: "staging"
      url: "https://api.staging.com/health"
      discovery:
        enabled: true
        mode: both
        ports: [443, 8443]
        on_failure: warn

    - id: "dev"
      url: "http://api.dev.com/health"
      discovery:
        enabled: true
        mode: scan
        ports:
          start: 8000
          end: 8100
        on_failure: skip
```

## Configuration Reference

```yaml
discovery:
  # Required
  enabled: true              # Enable discovery

  # Optional (with defaults)
  mode: validate             # validate/scan/both
  ports: 443                 # int, [int], or {start, end}
  timeout_ms: 2000           # Timeout per port
  retries: 2                 # Retry attempts
  on_failure: fail           # fail/skip/warn
  detect_service: true       # Detect HTTP vs HTTPS
  validate_http: true        # Validate HTTP responses
```

## Troubleshooting

### "Port discovery failed"

**Problem**: All ports failed to connect

**Solutions**:
1. Check the host is reachable: `ping api.example.com`
2. Check the port is open: `telnet api.example.com 443`
3. Check firewall rules allow connections
4. Try increasing `timeout_ms` or `retries`
5. Use `on_failure: warn` to continue anyway

### "Connection timeout"

**Problem**: Port check times out

**Solutions**:
1. Increase `timeout_ms` (default: 2000ms)
2. Increase `retries` (default: 2)
3. Check network latency
4. Verify no firewall blocking connection

### "No service detected"

**Problem**: Port is open but service type unknown

**Solutions**:
1. Set `detect_service: false` to skip detection
2. Set `validate_http: false` to skip HTTP validation
3. Manually specify protocol in URL

## Performance Tips

- **Parallel Discovery**: All targets discovered simultaneously
- **Concurrent Scanning**: Up to 10 ports per target at once
- **Fast Validation**: Single port checks complete in <100ms
- **Port Range Scanning**: ~1-5 seconds for 100 ports

## Security Notes

- Discovery uses relaxed TLS validation for detection only
- Actual load tests use normal TLS validation
- Discovery does not generate significant load
- Safe for production environments

## Examples in This Repo

Try these example configurations:

```bash
# Basic validation
./target/release/http-traffic-sim --config config.discovery-validate.example.yaml

# Port scanning
./target/release/http-traffic-sim --config config.discovery-scan.example.yaml

# Multi-target
./target/release/http-traffic-sim --config config.multi-target-discovery.example.yaml

# Auto-detection
./target/release/http-traffic-sim --config config.discovery-auto-detect.example.yaml
```

## Integration with CI/CD

Use discovery in CI/CD pipelines to validate deployments:

```bash
# Validate deployment before load test
./http-traffic-sim --config prod-validation.yaml

# Exit code 0 = success, 1 = failure
if [ $? -eq 0 ]; then
  echo "Validation passed, running load test..."
  ./http-traffic-sim --config prod-load-test.yaml
else
  echo "Validation failed, skipping load test"
  exit 1
fi
```

## Need Help?

- See `README.md` for full documentation
- See `ARCHITECTURE.md` or `docs/DOCUMENTATION.md` for technical details
- Check example configs in `config.discovery-*.yaml`
- Report issues on GitHub

## Summary

Port discovery helps you:
- ✅ Validate endpoints before expensive tests
- ✅ Discover services automatically
- ✅ Catch configuration errors early
- ✅ Test across multiple environments safely

Enable it by adding `discovery.enabled: true` to any target in your config file!
