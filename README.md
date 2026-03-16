# HTTP/HTTPS Traffic Simulator

A high-performance Rust-based HTTP/HTTPS benchmarking tool that simulates client traffic with configurable patterns. Supports single-target, multi-target load distribution, and comprehensive stress testing patterns for authorized infrastructure testing.

## Features

### Phase 1: Core Load Testing

- **Multiple Traffic Patterns**:
  - Fixed Concurrency - maintain constant concurrent requests
  - Rate Limited - maintain specific requests per second
  - Ramp-up - gradually increase load
  - Burst - send periodic bursts of requests

- **Comprehensive Metrics**:
  - Latency percentiles (p50, p90, p95, p99, p99.9)
  - Requests per second
  - Success/failure rates
  - Status code distribution
  - Error analysis

### Phase 2: Multi-Target & Stress Testing

- **Multi-Target Load Distribution**:
  - Round-robin distribution
  - Weighted distribution
  - Random distribution
  - Hash-based routing (session affinity)
  - Per-target metrics tracking

- **Stress Testing Patterns** (Authorized Use Only):
  - Connection Flood - rapid connection establishment
  - Request Flood - extreme request rate testing
  - Slowloris - slow header attack testing
  - Slow POST - slow body upload testing
  - Slow Read - slow response consumption
  - Large Payload - memory exhaustion testing
  - Pipeline Abuse - HTTP pipelining stress

- **Enhanced Metrics**:
  - Per-target breakdown (requests, latency, errors)
  - Connection-level error categorization
  - Refused connections, timeouts, resets
  - TLS handshake failures
  - DNS errors

- **Safety & Authorization**:
  - Mandatory authorization for stress tests
  - User-configurable safety limits
  - 5-second warning before stress tests
  - Clear legal disclaimers

- **Flexible Configuration**:
  - YAML/TOML config files
  - CLI arguments (override config)
  - Custom headers and request bodies
  - Optional safety limit enforcement

- **Real-time Reporting**:
  - Live updates during test execution
  - Detailed final summary
  - Per-target metrics breakdown
  - JSON export for further analysis

## Installation

```bash
cargo build --release
```

## Quick Start

### CLI Usage

```bash
# Fixed concurrency - 50 concurrent clients for 60 seconds
cargo run --release -- --url https://httpbin.org/get --concurrent 50 --duration 60

# Rate limited - 100 requests/second for 30 seconds
cargo run --release -- --url https://httpbin.org/get --rate 100 --duration 30

# Ramp-up - from 5 to 50 clients over 20 seconds
cargo run --release -- --url https://httpbin.org/get --ramp-from 5 --ramp-to 50 --ramp-duration 20

# Burst mode - 100 requests every 10 seconds
cargo run --release -- --url https://httpbin.org/get --burst-size 100 --burst-interval 10 --duration 60

# With output file
cargo run --release -- --url https://httpbin.org/get --concurrent 10 --duration 30 --output results.json
```

### Config File Usage

```bash
# Use config file
cargo run --release -- --config config.yaml

# Override config with CLI args
cargo run --release -- --config config.yaml --concurrent 100
```

See `config.example.yaml` for configuration file format.

## CLI Options

```
Options:
  -c, --config <PATH>              Path to configuration file (YAML or TOML)
  -u, --url <URL>                  Target URL to test
      --concurrent <N>             Number of concurrent requests
  -d, --duration <SECS>            Test duration in seconds
  -n, --requests <N>               Total number of requests to send
      --rate <N>                   Requests per second (rate-limited mode)
      --ramp-from <N>              Ramp-up: starting concurrent clients
      --ramp-to <N>                Ramp-up: ending concurrent clients
      --ramp-duration <SECS>       Ramp-up duration in seconds
      --burst-size <N>             Burst mode: requests per burst
      --burst-interval <SECS>      Burst mode: interval between bursts
  -o, --output <PATH>              Output file for results (JSON format)
  -m, --method <METHOD>            HTTP method [default: GET]
      --timeout <SECS>             Request timeout [default: 30]
  -v, --verbose <LEVEL>            Verbosity level 0-4 [default: 1]
  -h, --help                       Print help
```

## Traffic Patterns

### Fixed Concurrency
Maintains a constant number of concurrent requests throughout the test duration.

```bash
cargo run --release -- --url <URL> --concurrent 50 --duration 60
```

### Rate Limited
Maintains a specific requests per second rate.

```bash
cargo run --release -- --url <URL> --rate 100 --duration 30
```

### Ramp-up
Gradually increases load from one concurrency level to another.

```bash
cargo run --release -- --url <URL> --ramp-from 10 --ramp-to 100 --ramp-duration 60
```

### Burst
Sends periodic bursts of requests at specified intervals.

```bash
cargo run --release -- --url <URL> --burst-size 100 --burst-interval 10 --duration 60
```

## Output

### Real-time Updates
During execution, the tool displays live metrics:
- Elapsed time
- Total requests
- Requests per second
- Success rate
- Average latency
- Percentiles (p50, p90, p99)

### Final Summary
After completion, a detailed report includes:
- Total duration
- Request counts (total, successful, failed)
- Success/error rates
- Latency statistics (min, max, mean, std dev)
- Percentiles (p50, p90, p95, p99, p99.9)
- Status code distribution
- Error distribution

### JSON Export
Export detailed results for further analysis:

```bash
cargo run --release -- --url <URL> --concurrent 10 --duration 30 --output results.json
```

## Examples

### Test a Local API
```bash
cargo run --release -- \
  --url http://localhost:8080/api/health \
  --concurrent 20 \
  --duration 60 \
  --output local-api-test.json
```

### Load Test with Ramp-up
```bash
cargo run --release -- \
  --url https://example.com/api \
  --ramp-from 5 \
  --ramp-to 100 \
  --ramp-duration 120 \
  --output load-test.json
```

### POST Request with Config File
Create a config file with custom headers and body:

```yaml
target:
  url: "https://api.example.com/data"
  method: "POST"
  headers:
    Content-Type: "application/json"
    Authorization: "Bearer token123"
  body: '{"data": "test"}'

pattern:
  type: fixed
  concurrent: 50
  duration_secs: 60

output:
  file: "post-test.json"
```

Run with:
```bash
cargo run --release -- --config my-test.yaml
```

## Multi-Target Load Testing

Test multiple endpoints simultaneously with configurable load distribution.

### Example: Round-Robin Distribution

```yaml
targets:
  distribution:
    strategy: roundrobin
  targets:
    - id: "api1"
      url: "https://api1.example.com/health"
      method: "GET"
    - id: "api2"
      url: "https://api2.example.com/health"
      method: "GET"
    - id: "api3"
      url: "https://api3.example.com/health"
      method: "GET"

pattern:
  type: fixed
  concurrent: 50
  duration_secs: 60
```

Run with:
```bash
cargo run --release -- --config config.multi-target.example.yaml
```

### Example: Weighted Distribution

Distribute load according to specified weights (60%/30%/10%):

```yaml
targets:
  distribution:
    strategy: weighted
    weights: [0.6, 0.3, 0.1]
  targets:
    - id: "primary"
      url: "https://primary.example.com/api"
    - id: "secondary"
      url: "https://secondary.example.com/api"
    - id: "tertiary"
      url: "https://tertiary.example.com/api"
```

See `config.weighted.example.yaml` for complete example.

### Per-Target Metrics

Multi-target tests provide detailed breakdown per target:

```
GLOBAL SUMMARY:
Duration:              60.00s
Total Requests:        30000
Successful:            29500 (98.3%)
Requests/sec:          500.00

PER-TARGET BREAKDOWN:
Target: api1 (33.3% of traffic)
  Total Requests:     10000
  Success Rate:       98.5%
  Avg Latency:        45.2ms
  P99 Latency:        156.3ms
  Connection Errors:
    - Timeout:        15
```

## Stress Testing (Authorized Use Only)

⚠️ **WARNING**: Stress testing should ONLY be performed against infrastructure you own or have explicit written permission to test. Unauthorized stress testing may be illegal in your jurisdiction.

### Authorization Required

All stress tests require authorization configuration:

```yaml
stress_pattern:
  category: connectionflood
  connections_per_second: 500
  hold_time_ms: 100
  duration_secs: 60

authorization:
  confirmed: true
  target_owner: "Infrastructure Team - Ticket #12345"
  authorization_notes: "Load balancer capacity testing"
```

### Optional Safety Limits

Configure safety limits to prevent accidental overload:

```yaml
safety_limits:
  max_connections_per_second: 1000
  max_requests_per_second: 100000
  max_payload_size_mb: 100
  max_concurrent_connections: 5000
```

If omitted, no limits are enforced (unlimited). Safety limits are validated before test execution.

### Stress Patterns

#### Connection Flood
Rapidly establishes and closes connections:

```yaml
stress_pattern:
  category: connectionflood
  connections_per_second: 500
  hold_time_ms: 100
  duration_secs: 60
```

#### Request Flood
Generates extreme request rates:

```yaml
stress_pattern:
  category: requestflood
  target_rps: 10000
  duration_secs: 120
```

#### Slowloris
Simulates slow header attack:

```yaml
stress_pattern:
  category: slowloris
  connections: 200
  headers_per_second: 0.1
  duration_secs: 300
```

#### Large Payload
Tests handling of large request bodies:

```yaml
stress_pattern:
  category: largepayload
  size_mb: 50
  concurrent: 10
  duration_secs: 300
```

#### Slow POST
Sends request body slowly:

```yaml
stress_pattern:
  category: slowpost
  connections: 100
  bytes_per_second: 100
  payload_size: 10485760
```

#### Slow Read
Consumes response data slowly:

```yaml
stress_pattern:
  category: slowread
  connections: 100
  read_rate_bps: 1024
  duration_secs: 300
```

#### Pipeline Abuse
Sends multiple requests per connection:

```yaml
stress_pattern:
  category: pipelineabuse
  requests_per_connection: 100
  concurrent_connections: 50
```

See `config.stress-*.example.yaml` files for complete examples.

### Stress Test Safety Features

- **Mandatory Authorization**: Tests require `authorization.confirmed: true`
- **5-Second Warning**: Countdown before test starts (press Ctrl+C to cancel)
- **Legal Disclaimer**: Clear warnings displayed before execution
- **Optional Safety Limits**: User-configurable caps on rates and payloads
- **Connection Error Tracking**: Detailed categorization of failures

### Enhanced Error Tracking

Stress tests track connection-level errors:

```
CONNECTION STATISTICS
Refused:               12   (ECONNREFUSED)
Timeout:               43   (ETIMEDOUT)
Reset by peer:         8    (ECONNRESET)
TLS handshake errors:  2
DNS errors:            1
Other errors:          5
```

## Example Workflows

### Load Balancer Testing

Test multiple backend servers with round-robin distribution:

```bash
cargo run --release -- --config config.multi-target.example.yaml
```

### CDN Performance Testing

Test multiple edge locations with weighted distribution:

```bash
cargo run --release -- --config config.weighted.example.yaml
```

### Connection Limit Testing

Test connection handling capacity (authorized):

```bash
cargo run --release -- --config config.stress-flood.example.yaml
```

### API Gateway Stress Testing

Test extreme request rates (authorized):

```bash
cargo run --release -- --config config.stress-requestflood.example.yaml
```

## Requirements

- Rust 1.70 or later
- Tokio async runtime
- Internet connection for HTTPS targets

## Legal & Safety

**IMPORTANT**: This tool is designed for authorized infrastructure testing only.

- Only test infrastructure you own or have explicit written permission to test
- Unauthorized stress testing may violate Computer Fraud and Abuse Act (CFAA) or similar laws
- Users are solely responsible for ensuring they have proper authorization
- Configure appropriate safety limits to prevent unintended impact
- Always obtain written approval before running stress tests

## License

MIT
