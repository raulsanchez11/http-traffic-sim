# HTTP/HTTPS Traffic Simulator

A high-performance Rust-based HTTP/HTTPS benchmarking tool that simulates client traffic with configurable patterns. Measure and report performance metrics including latency percentiles, throughput, and error rates.

## Features

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

- **Flexible Configuration**:
  - YAML/TOML config files
  - CLI arguments (override config)
  - Custom headers and request bodies

- **Real-time Reporting**:
  - Live updates during test execution
  - Detailed final summary
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

## Requirements

- Rust 1.70 or later
- Tokio async runtime
- Internet connection for HTTPS targets

## License

MIT
