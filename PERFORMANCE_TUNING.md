# Performance Tuning Guide

This guide helps you optimize `http-traffic-sim` for maximum performance and efficiency.

## Table of Contents

- [Quick Start](#quick-start)
- [Understanding Bottlenecks](#understanding-bottlenecks)
- [Tuning Parameters](#tuning-parameters)
- [System-Level Optimization](#system-level-optimization)
- [Network Optimization](#network-optimization)
- [Memory Optimization](#memory-optimization)
- [Profiling Guide](#profiling-guide)
- [Common Scenarios](#common-scenarios)

---

## Quick Start

### Baseline Performance Test

```bash
# Run a baseline test
cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 50 \
  --duration 30 \
  --output baseline.json

# Review results
cat baseline.json | jq '.requests_per_second, .latency.p99_ms'
```

### Performance Checklist

- [ ] Using `--release` build (critical!)
- [ ] Connection pooling enabled (default: 128)
- [ ] Appropriate timeout configured
- [ ] System file descriptor limit increased
- [ ] Network latency measured
- [ ] Target has capacity for load

---

## Understanding Bottlenecks

### Performance Hierarchy

```
1. Network Latency         (highest impact, 10-500ms per request)
   └─> Geographic distance, routing, bandwidth

2. Target Capacity         (high impact, varies)
   └─> Server CPU, memory, database, rate limits

3. Connection Setup        (medium impact, 1-50ms per new connection)
   └─> TCP handshake, TLS negotiation

4. Client Resources        (low impact with proper config)
   └─> CPU, memory, file descriptors

5. Metrics Collection      (very low impact, <1μs per request)
   └─> Lock contention, memory allocation
```

### Identifying Bottlenecks

**High Latency (P99 > 500ms)**:
- **Cause**: Network or target overload
- **Solution**: Reduce load, increase timeout, or fix target

**Low Throughput (RPS < expected)**:
- **Cause**: Insufficient concurrency, connection limits, or target capacity
- **Solution**: Increase concurrency, tune connection pool

**High Error Rate (> 5%)**:
- **Cause**: Target overload, rate limiting, or misconfiguration
- **Solution**: Reduce load, check target capacity, review error types

**High Memory Usage**:
- **Cause**: Long test duration, large response bodies, memory leak (rare)
- **Solution**: Shorter tests, investigate with profiler

---

## Tuning Parameters

### 1. Concurrency Level

**Impact**: Directly affects throughput and target load

```yaml
pattern:
  type: fixed
  concurrent: 50  # Start here, adjust based on results
```

**Guidelines**:
- **Low latency targets** (< 50ms): 50-100 concurrent
- **Medium latency targets** (50-200ms): 100-500 concurrent
- **High latency targets** (> 200ms): 500-2000 concurrent
- **Stress testing**: 1000+ concurrent

**Formula**: `Optimal Concurrency ≈ Target RPS × Average Latency (seconds)`

**Example**:
```
Target: 1000 RPS
Latency: 100ms (0.1s)
Optimal: 1000 × 0.1 = 100 concurrent
```

### 2. Connection Pool Size

**Impact**: Reduces connection setup overhead

```yaml
client:
  pool_max_idle_per_host: 128  # Default
```

**Guidelines**:
- **Minimum**: Equal to max concurrency
- **Recommended**: 1.5-2× max concurrency
- **Maximum**: 500-1000 (diminishing returns)

**When to increase**:
- High connection setup latency (> 10ms)
- Many short requests
- Multiple targets

**When to decrease**:
- Memory constraints
- Single long-running request pattern

### 3. Timeout Configuration

**Impact**: Prevents hanging connections, affects error rates

```yaml
client:
  timeout_secs: 30  # Default
```

**Guidelines**:
- **Fast APIs**: 5-10 seconds
- **Standard APIs**: 30 seconds (default)
- **Slow operations**: 60-120 seconds
- **Streaming/downloads**: 300+ seconds

**Formula**: `Timeout = P99 latency + (2 × standard deviation) + safety margin`

### 4. Request Rate (Rate-Limited Pattern)

**Impact**: Precise control over load

```yaml
pattern:
  type: ratelimit
  rate: 100  # Requests per second
```

**Guidelines**:
- **Starting point**: 50% of target's known capacity
- **Stress testing**: 80-100% of capacity
- **Sustained load**: 60-70% of capacity

**Calculation**:
```
Target capacity: 1000 RPS
Starting rate: 500 RPS (50%)
Stress rate: 900 RPS (90%)
Sustained: 700 RPS (70%)
```

---

## System-Level Optimization

### File Descriptor Limits

**Problem**: "Too many open files" error with high concurrency

**Check current limit**:
```bash
ulimit -n
```

**Increase temporarily**:
```bash
# Linux/macOS
ulimit -n 10000
```

**Increase permanently** (Linux):
```bash
# Edit /etc/security/limits.conf
* soft nofile 10000
* hard nofile 10000

# Logout and login, or reboot
```

**Increase permanently** (macOS):
```bash
# Create /Library/LaunchDaemons/limit.maxfiles.plist
sudo launchctl limit maxfiles 10000 10000
```

**Required limit calculation**:
```
Minimum = Concurrency + Connection Pool + System Overhead
Example: 500 + 1000 + 100 = 1600
```

### CPU Optimization

**Use Release Build** (critical!):
```bash
# ❌ Debug build (10-100× slower)
cargo run -- [args]

# ✅ Release build (optimized)
cargo run --release -- [args]
```

**Performance difference**:
- Debug: 5,000 RPS
- Release: 50,000 RPS (10× faster)

**Monitor CPU usage**:
```bash
# During test
top
htop

# Expected: 50-90% CPU utilization
# If < 30%: Increase concurrency
# If > 95%: Target or network is bottleneck
```

### Memory Configuration

**Monitor memory usage**:
```bash
# macOS
top -l 1 | grep PhysMem

# Linux
free -h

# During test
watch -n 1 'ps aux | grep http-traffic-sim'
```

**Expected memory usage**:
```
Base: 15-20 MB
Per 1000 concurrent: +5-10 MB
Per 10,000 requests stored: +10 MB
Total typical: 30-100 MB
```

**Memory issues** (> 500 MB):
- Very long test durations
- Extremely large response bodies
- Memory leak (unlikely, report bug)

---

## Network Optimization

### Measure Network Latency

```bash
# Ping test
ping -c 10 target.example.com

# TCP handshake time
time curl -w "@-" -o /dev/null -s "https://target.example.com" <<< '
time_namelookup:  %{time_namelookup}
time_connect:     %{time_connect}
time_pretransfer: %{time_pretransfer}
time_starttransfer: %{time_starttransfer}
time_total:       %{time_total}
'
```

### Network Tuning (Linux)

```bash
# Increase TCP buffer sizes
sudo sysctl -w net.core.rmem_max=16777216
sudo sysctl -w net.core.wmem_max=16777216
sudo sysctl -w net.ipv4.tcp_rmem="4096 87380 16777216"
sudo sysctl -w net.ipv4.tcp_wmem="4096 65536 16777216"

# Enable TCP window scaling
sudo sysctl -w net.ipv4.tcp_window_scaling=1

# Increase max connections
sudo sysctl -w net.core.somaxconn=4096
sudo sysctl -w net.ipv4.ip_local_port_range="1024 65535"

# Make permanent: add to /etc/sysctl.conf
```

### Network Tuning (macOS)

```bash
# Increase socket buffers
sudo sysctl -w net.inet.tcp.sendspace=262144
sudo sysctl -w net.inet.tcp.recvspace=262144

# Increase connection backlog
sudo sysctl -w kern.ipc.somaxconn=4096

# Make permanent: add to /etc/sysctl.conf
```

### HTTP/2 Configuration

```yaml
client:
  http2_prior_knowledge: true  # If server supports HTTP/2
```

**Benefits**:
- Connection multiplexing
- Header compression
- Reduced latency

**When to use**:
- Server supports HTTP/2
- Many small requests
- High latency networks

---

## Memory Optimization

### Pre-allocation Strategy

The tool pre-allocates memory for typical workloads:
- Latencies: 10,000 requests
- Status codes: 10 common codes
- Errors: 20 error types

**For larger tests**, memory grows automatically but may cause brief pauses.

### Reducing Memory Usage

**1. Shorter test durations**:
```yaml
pattern:
  type: fixed
  concurrent: 100
  duration_secs: 60  # Instead of 3600
```

**2. Multiple shorter runs** instead of one long run:
```bash
for i in {1..10}; do
  cargo run --release -- \
    --url https://target.example.com \
    --concurrent 100 \
    --duration 60 \
    --output results-$i.json
done

# Aggregate results
./scripts/aggregate-results.sh results-*.json
```

**3. Periodic resets** (future feature):
- Currently not available
- Workaround: Multiple test runs

---

## Profiling Guide

### CPU Profiling with Flamegraph

**Install flamegraph**:
```bash
cargo install flamegraph
```

**Generate flamegraph**:
```bash
# Run with profiling
sudo cargo flamegraph -- \
  --url https://httpbin.org/get \
  --concurrent 100 \
  --duration 30

# Open result
open flamegraph.svg
```

**Or use the script**:
```bash
./scripts/profile.sh --url https://httpbin.org/get --concurrent 100 --duration 30
```

**Interpreting flamegraph**:
- **Width**: Time spent in function
- **Height**: Call stack depth
- **Color**: Different functions (no significance to color)

**Look for**:
- Wide bars at top: Hot functions
- Deep stacks: Complex call chains
- System calls: I/O waiting

### Benchmark Comparison

**Save baseline**:
```bash
./scripts/bench.sh --baseline main
```

**Make changes, then compare**:
```bash
./scripts/bench.sh --compare main
```

**Expected output**:
```
metrics_record_with_error
  time:   [82.341 ns 82.892 ns 83.476 ns]
  change: [-56.2% -55.8% -55.4%] (p < 0.01)
  Performance improved! 🎉
```

### Memory Profiling (Linux)

**Install heaptrack**:
```bash
# Ubuntu/Debian
sudo apt-get install heaptrack

# Fedora
sudo dnf install heaptrack
```

**Profile memory usage**:
```bash
heaptrack cargo run --release -- \
  --url https://httpbin.org/get \
  --concurrent 100 \
  --duration 30

# Analyze results
heaptrack_gui heaptrack.http-traffic-sim.*.gz
```

---

## Common Scenarios

### Scenario 1: Maximum Throughput

**Goal**: Highest RPS possible

**Configuration**:
```yaml
pattern:
  type: fixed
  concurrent: 1000  # High concurrency
  duration_secs: 60

client:
  timeout_secs: 10  # Short timeout
  pool_max_idle_per_host: 1500  # Large pool
```

**System tuning**:
```bash
ulimit -n 10000  # Increase file descriptors
```

**Tips**:
- Start at 100 concurrent, increase gradually
- Monitor target CPU/memory
- Watch for connection errors
- Use local or nearby target to minimize latency

**Expected results**:
- Local target: 50,000-100,000 RPS
- Remote target: 5,000-50,000 RPS
- Depends heavily on target capacity and network

### Scenario 2: Realistic Load Testing

**Goal**: Simulate production traffic

**Configuration**:
```yaml
pattern:
  type: ratelimit
  rate: 500  # Match production RPS
  duration_secs: 300  # 5 minutes

client:
  timeout_secs: 30
  pool_max_idle_per_host: 200

# Use real production endpoints
target:
  url: "https://api.example.com/v1/users"
  method: "GET"
  headers:
    Authorization: "Bearer ${API_TOKEN}"
```

**Tips**:
- Use production-like request patterns
- Include headers, authentication
- Vary request paths if possible
- Run during low-traffic periods initially

**Expected results**:
- Success rate > 99.9%
- P99 latency < 2× normal
- No errors or rate limiting

### Scenario 3: Stress Testing

**Goal**: Find breaking point

**Configuration**:
```yaml
pattern:
  type: ramp
  from: 10
  to: 1000
  ramp_duration_secs: 300  # 5 minutes
  hold_duration_secs: 60   # Hold at peak

client:
  timeout_secs: 60
  pool_max_idle_per_host: 1500
```

**Process**:
1. Start with low load
2. Gradually increase
3. Monitor target health
4. Identify degradation point
5. Back off before failure

**Tips**:
- Monitor target metrics (CPU, memory, errors)
- Watch for latency increases
- Note when errors start appearing
- Document breaking point for capacity planning

**Expected results**:
- Clear breaking point (RPS where errors spike)
- Gradual latency increase before failure
- Target recovery after test ends

### Scenario 4: Spike Testing

**Goal**: Test burst handling

**Configuration**:
```yaml
pattern:
  type: burst
  size: 500      # Large burst
  interval_secs: 10  # Every 10 seconds
  duration_secs: 300  # 5 minutes
```

**Tips**:
- Verify target can handle bursts
- Check for queueing/buffering
- Monitor recovery time between bursts
- Test autoscaling triggers

**Expected results**:
- Latency spike during burst
- Recovery between bursts
- No cascading failures

### Scenario 5: Endurance Testing

**Goal**: Test stability over time

**Configuration**:
```yaml
pattern:
  type: fixed
  concurrent: 50  # Moderate load
  duration_secs: 3600  # 1 hour (or longer)

client:
  timeout_secs: 30
  pool_max_idle_per_host: 200
```

**Tips**:
- Run for 1+ hours
- Monitor for memory leaks
- Watch for connection pool exhaustion
- Check for gradual performance degradation

**Expected results**:
- Stable performance over time
- No memory growth
- Consistent latency
- No connection errors

---

## Performance Optimization Checklist

### Before Testing

- [ ] Build with `--release` flag
- [ ] Measure baseline network latency
- [ ] Verify target health and capacity
- [ ] Check system file descriptor limits
- [ ] Close unnecessary applications
- [ ] Use wired connection (not WiFi) if possible

### During Testing

- [ ] Monitor client CPU (should be < 90%)
- [ ] Monitor client memory (should be stable)
- [ ] Watch for error spikes
- [ ] Check target health metrics
- [ ] Verify expected throughput

### After Testing

- [ ] Review latency percentiles (P50, P90, P99)
- [ ] Analyze error distribution
- [ ] Check for connection errors
- [ ] Document findings
- [ ] Compare against baseline

---

## Troubleshooting Performance Issues

### Low RPS Despite High Concurrency

**Possible causes**:
1. Target is bottleneck (most common)
2. Network latency too high
3. Connection pool too small
4. System resource limits

**Diagnosis**:
```bash
# Check if target is bottleneck
curl -w "Time: %{time_total}s\n" https://target.example.com

# If > 100ms, target is likely the bottleneck
```

**Solutions**:
- Increase target capacity
- Optimize target application
- Use CDN or caching
- Scale horizontally

### High Latency Variance (P99 >> P50)

**Possible causes**:
1. Occasional slow requests (cold cache, GC pauses)
2. Network congestion
3. Connection pool contention
4. Target resource contention

**Diagnosis**:
```bash
# Look at latency distribution
cat results.json | jq '.latency'

# Check for outliers
cat results.json | jq '.latency | .p50_ms, .p90_ms, .p99_ms, .max_ms'
```

**Solutions**:
- Increase connection pool
- Implement retries with timeout
- Tune target application (reduce GC, optimize queries)
- Use warm-up period before measurement

### Memory Growth During Test

**Possible causes**:
1. Very long test duration (normal)
2. Large response bodies (normal)
3. Memory leak (rare, report bug)

**Diagnosis**:
```bash
# Monitor memory over time
watch -n 5 'ps aux | grep http-traffic-sim | grep -v grep'
```

**Solutions**:
- Use shorter test durations
- Multiple shorter runs instead of one long run
- If suspected leak, report with reproduction steps

---

## Advanced Techniques

### Distributed Load Testing

For very high load, run multiple instances:

```bash
# Terminal 1
cargo run --release -- --config config1.yaml &

# Terminal 2
cargo run --release -- --config config2.yaml &

# Terminal 3
cargo run --release -- --config config3.yaml &

# Aggregate results
./scripts/aggregate-results.sh results-*.json
```

**Or use multiple machines**:
```bash
# Machine 1
ssh host1 "cd ~/http-traffic-sim && cargo run --release -- --config config.yaml"

# Machine 2
ssh host2 "cd ~/http-traffic-sim && cargo run --release -- --config config.yaml"

# Aggregate
scp host1:~/results.json results1.json
scp host2:~/results.json results2.json
./scripts/aggregate-results.sh results*.json
```

### Custom Metrics Collection

For specialized analysis, export JSON and analyze:

```bash
# Run test
cargo run --release -- \
  --url https://target.example.com \
  --concurrent 100 \
  --duration 60 \
  --output results.json

# Analyze with jq
cat results.json | jq '
  {
    rps: .requests_per_second,
    p50: .latency.p50_ms,
    p99: .latency.p99_ms,
    errors: (.failed_requests / .total_requests * 100)
  }
'

# Or with Python
python3 << EOF
import json

with open('results.json') as f:
    data = json.load(f)

print(f"RPS: {data['requests_per_second']:.2f}")
print(f"P99: {data['latency']['p99_ms']:.2f}ms")
print(f"Error rate: {data['error_rate']:.2f}%")
EOF
```

---

## Best Practices Summary

1. **Always use `--release` builds**
2. **Start with low load, increase gradually**
3. **Monitor both client and target**
4. **Tune connection pool to match concurrency**
5. **Set appropriate timeouts (P99 + buffer)**
6. **Increase file descriptor limits**
7. **Use network tuning for high throughput**
8. **Profile before optimizing**
9. **Run multiple short tests vs one long test**
10. **Document findings and baselines**

---

## References

- **Benchmarking**: `./scripts/bench.sh`
- **Profiling**: `./scripts/profile.sh`
- **Troubleshooting**: `TROUBLESHOOTING.md`
- **Architecture**: `ARCHITECTURE.md`

---

**Last Updated**: March 17, 2026
**For Questions**: Open an issue on GitHub
