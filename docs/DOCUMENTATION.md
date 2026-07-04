# HTTP/HTTPS Traffic Simulator — Complete Documentation

A high-performance Rust tool for HTTP/HTTPS load testing, multi-target benchmarking, port discovery, and authorized stress testing.

**Version:** 0.1.0  
**Binary name:** `http-traffic-sim`  
**Library crate:** `http_traffic_sim`

---

## Table of Contents

1. [Overview](#overview)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Execution Modes](#execution-modes)
5. [Command-Line Interface](#command-line-interface)
6. [Configuration Reference](#configuration-reference)
7. [Traffic Patterns](#traffic-patterns)
8. [Multi-Target Load Testing](#multi-target-load-testing)
9. [Port Discovery](#port-discovery)
10. [Stress Testing](#stress-testing)
11. [Metrics and Reporting](#metrics-and-reporting)
12. [Architecture](#architecture)
13. [Library API](#library-api)
14. [Development and Testing](#development-and-testing)
15. [Example Configuration Files](#example-configuration-files)
16. [Related Guides](#related-guides)
17. [Legal and Safety](#legal-and-safety)

---

## Overview

The HTTP/HTTPS Traffic Simulator generates realistic client traffic against one or more HTTP endpoints. It is built for:

- **Load testing** — sustained or ramped traffic with latency percentiles and throughput metrics
- **Multi-target benchmarking** — distribute requests across backends with per-target breakdowns
- **Pre-flight port discovery** — validate connectivity and detect HTTP/HTTPS services before tests run
- **Authorized stress testing** — connection floods, slow attacks, large payloads, and pipeline abuse (requires explicit authorization)

### Key capabilities

| Area | Features |
|------|----------|
| Traffic patterns | Fixed concurrency, rate limit, ramp-up, burst |
| Distribution | Round-robin, weighted, random, hash-based routing |
| Discovery | TCP validation, HTTP/HTTPS detection, port ranges (max 1024 ports) |
| Stress patterns | Connection flood, request flood, slowloris, slow POST/read, large payload, pipeline abuse |
| Metrics | HDR histogram latencies, status codes, error distribution, connection error categories |
| Output | Real-time terminal updates, console summary, JSON export |
| Config | YAML/TOML files + CLI overrides |

### Technology stack

- **Language:** Rust 2021 edition
- **Async runtime:** Tokio
- **HTTP client:** reqwest (rustls TLS, optional HTTP/2 prior knowledge)
- **Statistics:** hdrhistogram for percentile accuracy
- **CLI parsing:** clap

---

## Installation

### Requirements

- Rust 1.70 or later (stable recommended)
- Network access to target hosts
- For HTTPS targets: valid TLS (standard certificate validation; discovery uses relaxed certs only during port probing)

### Build from source

```bash
git clone <repository-url>
cd nh
cargo build --release
```

The binary is at `target/release/http-traffic-sim`.

### Verify installation

```bash
cargo test
./target/release/http-traffic-sim --help
```

---

## Quick Start

### CLI — fixed concurrency

```bash
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 50 \
  --duration 60
```

### CLI — rate limited

```bash
cargo run --release -- \
  --url https://httpbin.org/get \
  --rate 100 \
  --duration 30
```

### Config file

```bash
cargo run --release -- --config config.example.yaml
```

### Override config with CLI

CLI arguments override values from the config file:

```bash
cargo run --release -- --config config.example.yaml --concurrent 100 --duration 120
```

### Graceful shutdown

Press **Ctrl+C** at any time. The tool cancels in-flight work and prints final metrics when possible.

---

## Execution Modes

The application selects a mode automatically based on configuration:

| Mode | Trigger | Description |
|------|---------|-------------|
| **Single target** | `target.url` set, no `targets`, no `stress_pattern` | One endpoint, standard load patterns |
| **Multi-target** | `targets` section present, no `stress_pattern` | Multiple endpoints with load distribution |
| **Stress test** | `stress_pattern` present | Aggressive patterns; requires authorization |

### Mode selection logic

```
if stress_pattern is set     → StressTest
else if targets is set       → MultiTarget
else                         → SingleTarget
```

Stress tests require a single `target.url`. Multi-target and single-target modes use `pattern` (not `stress_pattern`).

### Startup sequence

1. Load and validate configuration
2. If stress mode: display legal warning + 5-second async countdown
3. If discovery enabled on any target: run port discovery phase
4. Print startup summary (mode, targets, pattern)
5. Execute traffic or stress pattern
6. Print results and optionally write JSON

---

## Command-Line Interface

```
http-traffic-sim [OPTIONS]
```

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--config` | `-c` | Path to YAML or TOML config | — |
| `--url` | `-u` | Target URL | — |
| `--concurrent` | | Fixed concurrency (workers) | — |
| `--duration` | `-d` | Duration in seconds | — |
| `--requests` | `-n` | Total request cap | — |
| `--rate` | | Requests per second (≥ 1) | — |
| `--ramp-from` | | Ramp start concurrency | — |
| `--ramp-to` | | Ramp end concurrency | — |
| `--ramp-duration` | | Ramp duration in seconds | — |
| `--burst-size` | | Requests per burst | — |
| `--burst-interval` | | Seconds between bursts | — |
| `--output` | `-o` | JSON results file path | — |
| `--method` | `-m` | HTTP method | `GET` |
| `--timeout` | | Request timeout (seconds) | `30` |
| `--verbose` | `-v` | Log level 0–4 | `1` |
| `--help` | `-h` | Print help | — |

### Verbosity levels

| Level | Logs |
|-------|------|
| 0 | Error only |
| 1 | Warnings |
| 2 | Info |
| 3 | Debug |
| 4 | Trace |

Override with environment variable `RUST_LOG` (tracing subscriber).

### Pattern selection via CLI

The first matching pattern wins when parsing CLI args:

1. `--burst-size` or `--burst-interval` → burst mode
2. `--ramp-from`, `--ramp-to`, or `--ramp-duration` → ramp mode
3. `--rate` → rate limit mode
4. `--concurrent` → fixed mode
5. Otherwise → use config file pattern (default: fixed 10 concurrent, 30s)

---

## Configuration Reference

Configuration files use YAML (`.yaml`, `.yml`) or TOML (`.toml`). All fields below are file-based unless noted as CLI-only.

### Top-level structure

```yaml
target:          # Single-target configuration
targets:         # Multi-target group (optional)
pattern:         # Load pattern (single/multi modes)
stress_pattern:  # Stress pattern (stress mode only)
authorization:   # Required for stress_pattern
safety_limits:   # Optional caps for stress tests
client:          # HTTP client settings
output:          # Reporting settings
# verbose is CLI-only
```

### `target`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | No | Target identifier (default: `target`) |
| `url` | string | Yes* | Full HTTP/HTTPS URL |
| `method` | string | No | HTTP verb (default: `GET`) |
| `headers` | map | No | Request headers |
| `body` | string | No | Request body (POST/PUT) |
| `discovery` | object | No | Port discovery settings |

\* Required unless using `targets` or stress mode with URL in config.

### `pattern`

Tagged by `type`: `fixed`, `ratelimit`, `ramp`, `burst`.

When using these types from Rust code, prefer the methods on the enum:

- `.validate()` — performs validation
- `.describe()` — returns the same multi-line description shown at startup (also implements `Display`)

#### Fixed

```yaml
pattern:
  type: fixed
  concurrent: 50
  duration_secs: 60      # optional
  total_requests: 1000     # optional (alternative to duration)
```

#### Rate limit

```yaml
pattern:
  type: ratelimit
  rate: 100              # must be ≥ 1
  duration_secs: 30
  total_requests: 5000
```

#### Ramp

```yaml
pattern:
  type: ramp
  from: 10
  to: 100                # must be ≥ from
  ramp_duration_secs: 60
  hold_duration_secs: 30 # optional hold at max concurrency
```

#### Burst

```yaml
pattern:
  type: burst
  size: 100
  interval_secs: 10
  duration_secs: 120
  total_bursts: 12       # optional cap
```

### `targets` (multi-target)

```yaml
targets:
  distribution:
    strategy: roundrobin   # roundrobin | weighted | random | hash
  targets:
    - id: api1
      url: https://api1.example.com/health
      method: GET
      headers: {}
      discovery: { ... }   # optional per target
```

#### Distribution strategies

| Strategy | Config | Behavior |
|----------|--------|----------|
| `roundrobin` | `strategy: roundrobin` | Cycle through targets in order |
| `weighted` | `strategy: weighted` + `weights: [0.6, 0.3, 0.1]` | Probabilistic by weight |
| `random` | `strategy: random` | Uniform random selection |
| `hash` | `strategy: hash` + `field: sourceip \| sessionid` | Deterministic hash of request counter |

### `client`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout_secs` | u64 | 30 | Per-request timeout |
| `pool_max_idle_per_host` | usize | 128 | Connection pool size per host |
| `http2_prior_knowledge` | bool | false | Use HTTP/2 prior knowledge (h2c) |

### `output`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file` | path | — | JSON export path |
| `console` | bool | true | Print final summary |
| `realtime_updates` | bool | false | Live in-place terminal metrics (single-target) |

Multi-target mode supports `file` and `console`; JSON export includes global + per-target statistics.

### `safety_limits` (stress only)

All fields are optional (`null` = unlimited):

| Field | Applies to |
|-------|------------|
| `max_connections_per_second` | Connection flood |
| `max_requests_per_second` | Request flood |
| `max_payload_size_mb` | Large payload |
| `max_concurrent_connections` | Slowloris, slow POST/read, pipeline abuse |

Limits are enforced at config load time when `stress_pattern` is set.

### `authorization` (stress only)

```yaml
authorization:
  confirmed: true                              # must be true
  target_owner: "Team Name - Ticket #12345"
  authorization_notes: "Approved capacity test"
```

---

## Traffic Patterns

### Fixed concurrency

Maintains N concurrent worker tasks. Each worker loops until duration, request count, or cancellation.

- **Use when:** simulating steady concurrent users
- **Limits:** `duration_secs` and/or `total_requests` (either or both)

### Rate limited

Spawns one request per tick at `rate` requests/second using precise microsecond intervals.

- **Use when:** testing API rate limits or steady RPS
- **Validation:** `rate` must be ≥ 1

### Ramp-up

Steps through increasing concurrency levels over `ramp_duration_secs`, then optionally holds at `to` for `hold_duration_secs`.

- **Use when:** finding breaking points gradually
- **Validation:** `from` must be ≤ `to`

### Burst

Sends `size` concurrent requests every `interval_secs` until duration or burst count is reached.

- **Use when:** simulating traffic spikes (batch jobs, flash sales)

---

## Multi-Target Load Testing

### Example: round-robin

See `config.multi-target.example.yaml`.

```bash
cargo run --release -- --config config.multi-target.example.yaml
```

### Example: weighted

See `config.weighted.example.yaml`. Weights need not sum to 1.0 — they are normalized automatically.

### Output format

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
  Avg Latency:        45.20ms
  P99 Latency:        156.30ms
```

### JSON export (multi-target)

When `output.file` is set, JSON contains:

```json
{
  "global": { /* Statistics */ },
  "per_target": {
    "api1": { /* Statistics */ },
    "api2": { /* Statistics */ }
  }
}
```

---

## Port Discovery

Discovery runs **before** load tests when `discovery.enabled: true` on any target.

### Discovery modes

| Mode | TCP check | HTTP/HTTPS probe | Notes |
|------|-----------|------------------|-------|
| `validate` | Yes | No | Fast reachability check only |
| `scan` | Yes | Yes (if `detect_service` + `validate_http`) | Full service detection |
| `both` | Yes | Yes | Also probes ports 80 and 443 if not in spec |

### Port specification

```yaml
ports: 8080                    # single
ports: [80, 443, 8080]         # list
ports: { start: 8000, end: 8100 }  # range (max 1024 ports)
```

### Failure handling

| Action | Behavior |
|--------|----------|
| `fail` | Abort if discovery fails (default) |
| `skip` | Remove targets with no open ports; continue with survivors |
| `warn` | Log warning; keep original URL and continue |

### Auto URL update

When discovery finds open ports, the tool picks the best port (HTTPS > HTTP > first open) and updates the target URL before the load test.

### Discovery configuration

| Field | Default | Description |
|-------|---------|-------------|
| `enabled` | false | Enable for this target |
| `mode` | validate | validate / scan / both |
| `ports` | 80 | Port spec (required) |
| `timeout_ms` | 2000 | Per-port timeout |
| `retries` | 2 | TCP retry count with backoff |
| `on_failure` | fail | fail / skip / warn |
| `detect_service` | true | HTTP vs HTTPS detection |
| `validate_http` | true | Send HTTP probe during scan/both |

### Example configs

- `config.discovery-validate.example.yaml`
- `config.discovery-scan.example.yaml`
- `config.discovery-auto-detect.example.yaml`
- `config.multi-target-discovery.example.yaml`

---

## Stress Testing

> **WARNING:** Stress testing can impact service availability and may be illegal without authorization. Only test systems you own or have explicit written permission to test.

### Requirements

1. `stress_pattern` in config
2. `authorization.confirmed: true`
3. Optional `safety_limits` (recommended)
4. Single `target.url`

### Startup behavior

1. Config validation (auth + safety limits) at load time
2. Prominent legal warning printed to console
3. 5-second countdown (async, non-blocking; Ctrl+C to cancel)
4. Pattern execution

### Stress patterns

| Category | YAML key | Parameters | Implementation |
|----------|----------|------------|--------------|
| Connection flood | `connectionflood` | `connections_per_second`, `hold_time_ms`, `duration_secs` | Raw TCP partial requests held open |
| Request flood | `requestflood` | `target_rps`, `duration_secs` | High-rate spawned requests |
| Slowloris | `slowloris` | `connections`, `headers_per_second`, `duration_secs` | Partial headers to **configured target** |
| Slow POST | `slowpost` | `connections`, `bytes_per_second`, `payload_size` | Slow body drip over TCP |
| Large payload | `largepayload` | `size_mb`, `concurrent`, `duration_secs` | POST with large body |
| Pipeline abuse | `pipelineabuse` | `requests_per_connection`, `concurrent_connections` | Multiple HTTP/1.1 requests per connection |
| Slow read | `slowread` | `connections`, `read_rate_bps`, `duration_secs` | Slow response consumption |

When using `StressPattern` from Rust, call:

- `.describe()` — human-readable string used in warnings and startup output
- `.validate_against(&safety_limits)` — safety enforcement

### Example

```yaml
target:
  url: "https://test.mycompany.com"

stress_pattern:
  category: slowloris
  connections: 200
  headers_per_second: 0.1
  duration_secs: 300

authorization:
  confirmed: true
  target_owner: "Security Team - Pen Test #2024-Q1"
  authorization_notes: "Testing slowloris defenses"

safety_limits:
  max_concurrent_connections: 500

client:
  timeout_secs: 300
  pool_max_idle_per_host: 500

output:
  file: slowloris-results.json
  console: true
```

See `config.stress-*.example.yaml` for more examples.

### Connection error statistics

Stress and load tests categorize transport-level failures:

- Connection refused
- Timeouts
- Connection reset
- TLS/SSL errors
- DNS resolution errors
- Other

HTTP status errors (4xx/5xx) are tracked separately and are **not** counted as connection errors.

---

## Metrics and Reporting

### Collected metrics

| Metric | Description |
|--------|-------------|
| Total / successful / failed requests | Counts |
| Requests per second | Throughput |
| Success / error rate | Percentages |
| Latency histogram | Recorded at ingest (microseconds) |
| Percentiles | p50, p90, p95, p99, p99.9 (milliseconds) |
| Status code distribution | Sorted by frequency |
| Error distribution | Top errors by message |
| Connection stats | Transport failure categories |

### Real-time updates

Enable with:

```yaml
output:
  realtime_updates: true
```

Updates every second on a single line (single-target mode):

```
Elapsed: 12.0s | Requests: 1200 | RPS: 100.0 | Success: 99.5% | ...
```

### Final summary

Printed when `output.console: true`. Includes latency min/max/mean/stddev, percentiles, status codes, and top 10 errors.

### JSON export

```yaml
output:
  file: results.json
```

Single-target: `Statistics` object. Multi-target: `{ global, per_target }`.

---

## Architecture

### Project layout

```
src/
  main.rs           # Thin entry: calls http_traffic_sim::run()
  lib.rs            # Library exports
  run.rs            # Orchestration (modes, discovery phase, reporting)
  config.rs         # CLI + file config, validation
  client.rs         # HTTP/TCP client, stress primitives
  patterns.rs       # Load pattern executor (worker pool)
  stress.rs         # Stress pattern executor
  discovery.rs      # Port scanning and validation
  metrics.rs        # HDR histogram metrics, RequestRecorder trait
  stats.rs          # Percentile calculation from snapshots
  reporter.rs       # Console + JSON output
  target_selector.rs# Multi-target distribution
  authorization.rs  # Stress warnings and countdown
```

### Data flow

```
Config::load()
    → authorization (stress) / discovery phase
    → HttpClient + PatternExecutor | StressExecutor
    → RequestResult stream
    → MetricsCollector (HDR histogram)
    → Statistics::from_snapshot()
    → Reporter (console / JSON)
```

### Concurrency model

- **Tokio** async runtime with `#[tokio::main]`
- **Fixed pattern:** N worker tasks (no unbounded handle accumulation)
- **Rate/burst:** `JoinSet` for in-flight request tracking
- **Metrics:** Mutex-protected histogram (bounded memory vs. raw latency vectors)
- **Cancellation:** `CancellationToken` + Ctrl+C handler

### Design principles

- Binary uses library crate (no duplicate module tree)
- Transport errors separated from HTTP status errors
- Discovery mode affects probing behavior (not just config decoration)
- Stress patterns use raw TCP where connection holding matters

For deeper design detail, see [ARCHITECTURE.md](../ARCHITECTURE.md).

---

## Library API

Use as a Rust library for integration tests, custom runners, or embedding.

### Run the full application

```rust
use http_traffic_sim::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
```

### Load configuration programmatically

```rust
use http_traffic_sim::config::Config;

let config = Config::load()?;
```

Note: `Config::load()` parses CLI args via clap — intended for binary use.

### Core types

```rust
use http_traffic_sim::{
    Config, TargetConfig, TrafficPattern,
    MetricsCollector, MultiTargetMetrics, RequestRecorder,
    Statistics, run,
};
```

### Pattern and Target helpers (recommended for library use)

`TrafficPattern` and `StressPattern` provide canonical behavior:

```rust
let pattern = TrafficPattern::RateLimit { rate: 100, .. };
pattern.validate()?;                    // central validation
println!("{}", pattern.describe());     // or format!("{}", pattern)
```

`StressPattern` also has safety-limit validation:

```rust
stress_pattern.validate_against(&safety_limits)?;
```

`TargetConfig` centralizes ID handling:

```rust
let id = target.effective_id(Some(index));  // "target" or "target-N" or custom id
target.id = target.effective_id(Some(i));
```

### Example: custom metrics loop

```rust,no_run
use http_traffic_sim::client::HttpClient;
use http_traffic_sim::config::{ClientConfig, TargetConfig};
use http_traffic_sim::metrics::MetricsCollector;
use http_traffic_sim::stats::Statistics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let target = TargetConfig {
        url: "https://httpbin.org/get".into(),
        ..Default::default()
    };
    let client = HttpClient::new(target, &ClientConfig::default())?;
    let metrics = MetricsCollector::new();

    for _ in 0..100 {
        metrics.record(client.execute().await);
    }

    let stats = Statistics::from_snapshot(&metrics.get_snapshot());
    println!("RPS: {:.1}", stats.requests_per_second);
    Ok(())
}
```

---

## Development and Testing

### Run tests

```bash
cargo test              # unit + integration + doctests
cargo test --release    # CI uses release builds
```

### Lint and format

```bash
cargo fmt
cargo clippy -- -D warnings
```

### Benchmarks

```bash
cargo bench
./scripts/bench.sh
```

### Profiling

```bash
./scripts/profile.sh --url https://httpbin.org/get --concurrent 100 --duration 30
```

See [docs/PROFILING_QUICKSTART.md](PROFILING_QUICKSTART.md).

### CI/CD

GitHub Actions (`.github/workflows/ci.yml`):

- Multi-platform tests (Ubuntu, macOS, Windows)
- Clippy linting
- Release builds

### Helper scripts

| Script | Purpose |
|--------|---------|
| `scripts/setup-hooks.sh` | Install pre-commit hooks |
| `scripts/bench.sh` | Run benchmark suite |
| `scripts/profile.sh` | Flamegraph profiling |
| `scripts/quick_wins.sh` | Quick optimization checks |

---

## Example Configuration Files

| File | Purpose |
|------|---------|
| `config.example.yaml` | Basic single-target load test |
| `config.multi-target.example.yaml` | Round-robin multi-target |
| `config.weighted.example.yaml` | Weighted distribution |
| `config.discovery-validate.example.yaml` | Validate specific port |
| `config.discovery-scan.example.yaml` | Scan port range |
| `config.discovery-auto-detect.example.yaml` | Auto-detect service |
| `config.multi-target-discovery.example.yaml` | Mixed discovery per target |
| `config.stress-flood.example.yaml` | Connection flood |
| `config.stress-requestflood.example.yaml` | Request flood |
| `config.stress-slowloris.example.yaml` | Slowloris |
| `config.stress-largepayload.example.yaml` | Large payload |

Test configs (discovery failure modes):

- `config.discovery-fail-test.yaml`
- `config.discovery-warn-test.yaml`
- `config.discovery-multi-port-test.yaml`

---

## Related Guides

| Document | Contents |
|----------|----------|
| [README.md](../README.md) | Quick reference and feature overview |
| [ARCHITECTURE.md](../ARCHITECTURE.md) | Detailed system design |
| [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) | Common problems and fixes |
| [PERFORMANCE_TUNING.md](../PERFORMANCE_TUNING.md) | Throughput optimization |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Development workflow |
| [NO_HARDCODED_LIMITS.md](../NO_HARDCODED_LIMITS.md) | Safety limit philosophy |
| [PORT_DISCOVERY_QUICKSTART.md](../PORT_DISCOVERY_QUICKSTART.md) | Discovery quick reference |
| [docs/END_TO_END_BENCHMARKS.md](END_TO_END_BENCHMARKS.md) | Benchmark scenarios |
| [docs/PROFILING_QUICKSTART.md](PROFILING_QUICKSTART.md) | Profiling guide |

---

## Legal and Safety

- Only test infrastructure you **own** or have **explicit written authorization** to test.
- Unauthorized testing may violate laws such as the CFAA (US) or equivalent regulations.
- Stress tests require `authorization.confirmed: true` — this is enforced, not optional.
- Configure `safety_limits` to reduce risk of accidental overload.
- Users are solely responsible for obtaining proper authorization.
- The authors provide this tool for legitimate performance and security testing only.

---

## License

MIT — see repository LICENSE file.