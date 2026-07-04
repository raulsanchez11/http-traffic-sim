# End-to-End Benchmarks

This document provides guidance for running comprehensive end-to-end performance benchmarks.

## Overview

End-to-end benchmarks measure real-world performance with actual HTTP requests, unlike micro-benchmarks which test individual functions in isolation.

## Benchmark Scenarios

### 1. Baseline Performance

**Purpose**: Establish performance baseline with simple requests

**Configuration**:
```yaml
target:
  url: "https://httpbin.org/get"
  method: "GET"

pattern:
  type: fixed
  concurrent: 50
  duration_secs: 60

client:
  timeout_secs: 30
  pool_max_idle_per_host: 128
```

**Expected Results** (httpbin.org):
- **Throughput**: 200-500 RPS (depends on network)
- **P50 Latency**: 100-200ms
- **P99 Latency**: 300-500ms
- **Success Rate**: > 99%

**Command**:
```bash
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 50 \
  --duration 60 \
  --output baseline.json
```

### 2. Connection Pool Efficiency

**Purpose**: Test connection reuse and pool performance

**Test Matrix**:
```yaml
# Test 1: Pool too small
client:
  pool_max_idle_per_host: 10

# Test 2: Pool matched
client:
  pool_max_idle_per_host: 50

# Test 3: Pool oversized
client:
  pool_max_idle_per_host: 200
```

**Expected Results**:
- Pool too small: Higher latency variance
- Pool matched: Optimal performance
- Pool oversized: Minimal benefit, higher memory

**Commands**:
```bash
# Small pool
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 50 \
  --duration 60 \
  --timeout 30 \
  --output pool-small.json

# Note: Pool size configured in code or config file
```

### 3. Rate Limiting Accuracy

**Purpose**: Verify request rate accuracy

**Configuration**:
```yaml
pattern:
  type: ratelimit
  rate: 100
  duration_secs: 60
```

**Expected Results**:
- **Total Requests**: ~6000 (100 RPS × 60 seconds)
- **Actual RPS**: 95-105 (±5% tolerance)
- **Success Rate**: > 99%

**Verification**:
```bash
cargo run --release -- \
  --url https://httpbin.org/get \
  --rate 100 \
  --duration 60 \
  --output rate-limit.json

# Check actual rate
cat rate-limit.json | jq '.requests_per_second'
# Should be close to 100
```

### 4. High Concurrency

**Purpose**: Test scalability with many concurrent connections

**Configuration**:
```yaml
pattern:
  type: fixed
  concurrent: 500
  duration_secs: 60

client:
  pool_max_idle_per_host: 750
```

**System Requirements**:
```bash
# Increase file descriptor limit
ulimit -n 5000
```

**Expected Results**:
- **Throughput**: 2000-5000 RPS (httpbin.org)
- **Memory Usage**: 50-100 MB
- **CPU Usage**: 50-80%

**Command**:
```bash
ulimit -n 5000
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 500 \
  --duration 60 \
  --output high-concurrency.json
```

### 5. Latency Distribution

**Purpose**: Measure latency under various loads

**Test Matrix**:
```bash
# Low load
--concurrent 10

# Medium load
--concurrent 50

# High load
--concurrent 200

# Very high load
--concurrent 500
```

**Expected Pattern**:
- Low load: Minimal latency, low variance
- Medium load: Slight increase, stable
- High load: Higher P99, more variance
- Very high load: P99 >> P50, possible timeouts

**Commands**:
```bash
for concurrency in 10 50 200 500; do
  cargo run --release -- \
    --url https://httpbin.org/get \
    --concurrent $concurrency \
    --duration 60 \
    --output latency-${concurrency}.json
done

# Compare results
for f in latency-*.json; do
  echo "File: $f"
  cat $f | jq '{concurrent: .total_requests, p50: .latency.p50_ms, p99: .latency.p99_ms}'
done
```

### 6. Multi-Target Load Distribution

**Purpose**: Test load balancing across multiple targets

**Configuration**:
```yaml
targets:
  distribution:
    strategy: roundrobin
  targets:
    - id: "target1"
      url: "https://httpbin.org/get"
    - id: "target2"
      url: "https://postman-echo.com/get"
    - id: "target3"
      url: "https://reqres.in/api/users"

pattern:
  type: fixed
  concurrent: 100
  duration_secs: 60
```

**Expected Results**:
- Even distribution (~33% each for round-robin)
- Combined throughput higher than single target
- Per-target metrics available

### 7. Ramp-Up Pattern

**Purpose**: Test gradual load increase

**Configuration**:
```yaml
pattern:
  type: ramp
  from: 10
  to: 200
  ramp_duration_secs: 120
  hold_duration_secs: 60
```

**Expected Results**:
- Smooth increase in RPS
- Gradual latency increase
- No sudden spikes or failures

**Command**:
```bash
cargo run --release -- \
  --url https://httpbin.org/get \
  --ramp-from 10 \
  --ramp-to 200 \
  --ramp-duration 120 \
  --duration 60 \
  --output ramp-up.json
```

### 8. Burst Pattern

**Purpose**: Test spike handling

**Configuration**:
```yaml
pattern:
  type: burst
  size: 100
  interval_secs: 10
  duration_secs: 120
```

**Expected Results**:
- Periodic latency spikes
- Recovery between bursts
- ~12 bursts in 120 seconds

**Command**:
```bash
cargo run --release -- \
  --url https://httpbin.org/get \
  --burst-size 100 \
  --burst-interval 10 \
  --duration 120 \
  --output burst.json
```

## Test Targets

### Public Test APIs

1. **httpbin.org** (Recommended)
   - URL: `https://httpbin.org/get`
   - Advantages: Stable, reliable, good performance
   - Limitations: Rate limiting possible at very high load

2. **postman-echo.com**
   - URL: `https://postman-echo.com/get`
   - Advantages: Good for multi-target tests
   - Limitations: Slower than httpbin

3. **reqres.in**
   - URL: `https://reqres.in/api/users`
   - Advantages: REST API patterns
   - Limitations: Rate limiting

### Local Test Server

For maximum performance testing, run a local server:

```bash
# Simple HTTP server with Python
python3 -m http.server 8080 &

# Test against localhost
cargo run --release -- \
  --url http://localhost:8080 \
  --concurrent 1000 \
  --duration 60 \
  --output local-server.json
```

**Expected results (localhost)**:
- **Throughput**: 50,000-100,000 RPS
- **Latency**: < 1ms P50, < 5ms P99
- **Bottleneck**: Client CPU or network stack

### Production-like Server

For realistic testing, use a production-like environment:

```bash
# Deploy test server (example with Docker)
docker run -d -p 8080:80 nginx

# Run comprehensive test suite
cargo run --release -- \
  --url http://localhost:8080 \
  --concurrent 100 \
  --duration 300 \
  --output production-like.json
```

## Benchmark Workflow

### 1. Pre-Benchmark Checklist

```bash
# Build release version
cargo build --release

# Check system limits
ulimit -n
# Should be > 5000

# Verify target availability
curl -I https://httpbin.org/get

# Close unnecessary applications
# Disable WiFi (use wired connection)
```

### 2. Run Benchmark Suite

```bash
#!/bin/bash
# benchmark-suite.sh

OUTPUT_DIR="benchmarks/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$OUTPUT_DIR"

echo "Running benchmark suite..."
echo "Output directory: $OUTPUT_DIR"

# 1. Baseline
echo "1/8: Baseline..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 50 \
  --duration 60 \
  --output "$OUTPUT_DIR/01-baseline.json"

# 2. Low concurrency
echo "2/8: Low concurrency..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 10 \
  --duration 60 \
  --output "$OUTPUT_DIR/02-low-concurrency.json"

# 3. Medium concurrency
echo "3/8: Medium concurrency..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 100 \
  --duration 60 \
  --output "$OUTPUT_DIR/03-medium-concurrency.json"

# 4. High concurrency
echo "4/8: High concurrency..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 500 \
  --duration 60 \
  --output "$OUTPUT_DIR/04-high-concurrency.json"

# 5. Rate limit
echo "5/8: Rate limiting..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --rate 100 \
  --duration 60 \
  --output "$OUTPUT_DIR/05-rate-limit.json"

# 6. Ramp up
echo "6/8: Ramp up..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --ramp-from 10 \
  --ramp-to 200 \
  --ramp-duration 120 \
  --duration 60 \
  --output "$OUTPUT_DIR/06-ramp-up.json"

# 7. Burst
echo "7/8: Burst pattern..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --burst-size 100 \
  --burst-interval 10 \
  --duration 120 \
  --output "$OUTPUT_DIR/07-burst.json"

# 8. Endurance
echo "8/8: Endurance test..."
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 50 \
  --duration 300 \
  --output "$OUTPUT_DIR/08-endurance.json"

echo "Benchmark suite complete!"
echo "Results in: $OUTPUT_DIR"
```

### 3. Analyze Results

```bash
#!/bin/bash
# analyze-benchmarks.sh

BENCHMARK_DIR=$1

echo "=== Benchmark Analysis ==="
echo ""

for file in "$BENCHMARK_DIR"/*.json; do
  name=$(basename "$file" .json)
  echo "--- $name ---"

  cat "$file" | jq '{
    rps: .requests_per_second,
    total: .total_requests,
    success_rate: .success_rate,
    latency: {
      p50: .latency.p50_ms,
      p90: .latency.p90_ms,
      p99: .latency.p99_ms
    },
    errors: .failed_requests
  }'

  echo ""
done
```

### 4. Compare Baselines

```bash
# Save current results as baseline
cp results.json baseline.json

# Make changes...

# Compare new results to baseline
./scripts/compare-benchmarks.sh baseline.json results.json
```

## Performance Baselines

### Micro-benchmarks

(Note: Detailed baseline from 2026 is in git history. Current focus is on end-to-end.)

```
Port spec parsing:
- Single: 16.20 ns
- List: 30.95 ns
- Range (small): 24.35 ns
- Range (large): 52.11 ns

URL parsing:
- extract_host: 229.83 ns

Metrics:
- Record: 42.5 ns
- Record with error: 82 ns (56% improvement!)
- Snapshot: 141 ns
- Concurrent record: 51 ns
```

### End-to-End (httpbin.org, typical)

```
Baseline (50 concurrent):
- RPS: 300-500
- P50: 100-150ms
- P99: 300-500ms
- Success: > 99%

High concurrency (500 concurrent):
- RPS: 2000-5000
- P50: 100-200ms
- P99: 500-1000ms
- Success: > 95%

Rate limited (100 RPS):
- RPS: 95-105 (accurate)
- P50: 100-150ms
- P99: 200-400ms
- Success: > 99%
```

### Local Server (nginx, localhost)

```
High throughput (1000 concurrent):
- RPS: 50,000-100,000
- P50: < 1ms
- P99: < 5ms
- Success: > 99.9%

Memory usage:
- Baseline: 15-20 MB
- High load: 50-100 MB
- Growth: Stable over time
```

## Continuous Benchmarking

### CI/CD Integration

Add benchmark job to `.github/workflows/ci.yml`:

```yaml
benchmark:
  name: Performance Benchmarks
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable

    - name: Run benchmarks
      run: cargo bench

    - name: Store benchmark results
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: target/criterion/output.json
        github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Automated Regression Detection

```bash
# Save baseline
cargo bench -- --save-baseline main

# After changes
cargo bench -- --baseline main

# If regression detected, fail
if grep -q "Performance regressed" target/criterion/reports; then
  echo "Performance regression detected!"
  exit 1
fi
```

## Best Practices

1. **Always use `--release` builds**
2. **Run multiple iterations** (3-5 times, use median)
3. **Control external factors** (network, other processes)
4. **Use consistent hardware** for comparisons
5. **Document test environment** (specs, OS, network)
6. **Track results over time** (baseline evolution)
7. **Test realistic scenarios** (production-like patterns)
8. **Monitor system resources** (CPU, memory, network)

## Troubleshooting

### Inconsistent Results

- Run multiple times, use median
- Close other applications
- Use wired network (not WiFi)
- Check for system updates/background tasks

### Lower Than Expected Performance

- Verify `--release` build used
- Check network latency to target
- Verify target isn't rate limiting
- Increase concurrency if target has capacity

### High Variance (P99 >> P50)

- Increase connection pool size
- Check for network congestion
- Verify target isn't overloaded
- Consider longer warmup period

---

**See Also**:
- (Historical baselines in git history)
- `PERFORMANCE_TUNING.md` - Optimization guide
- `./scripts/bench.sh` - Benchmark runner script
- `./scripts/profile.sh` - Profiling script

**Last Updated**: March 17, 2026
