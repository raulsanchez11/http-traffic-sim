# Architecture Documentation

## Overview

`http-traffic-sim` is a high-performance HTTP/HTTPS load testing and stress testing tool built in Rust. The architecture emphasizes modularity, performance, and safety with clear separation of concerns across modules.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         User Input                           │
│                    (CLI args / Config file)                  │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    Configuration Module                      │
│  - Load from YAML/TOML/CLI                                  │
│  - Validate settings                                         │
│  - Merge sources (file + CLI)                               │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  (Authorization removed - assumed)                           │
│  - Safety limit validation (if configured)                   │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│             Port Discovery (Optional)                        │
│  - TCP connectivity checks                                   │
│  - HTTP/HTTPS service detection                              │
│  - Concurrent port scanning                                  │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
        ┌───────────────┴───────────────┐
        │                               │
        ▼                               ▼
┌──────────────────┐          ┌──────────────────┐
│  Pattern Exec    │          │  Stress Exec     │
│  (Normal Load)   │          │  (Stress Tests)  │
└────────┬─────────┘          └────────┬─────────┘
         │                              │
         │  ┌───────────────────────┐  │
         └──► HTTP Client Module    ◄──┘
            │  - Request execution  │
            │  - Connection pooling │
            │  - Error handling     │
            └───────────┬───────────┘
                        │
                        ▼
            ┌───────────────────────┐
            │  Metrics Collection   │
            │  - Request results    │
            │  - Latency tracking   │
            │  - Error categorization│
            └───────────┬───────────┘
                        │
                        ▼
            ┌───────────────────────┐
            │  Statistics Module    │
            │  - HDR histogram      │
            │  - Percentile calc    │
            │  - Aggregation        │
            └───────────┬───────────┘
                        │
                        ▼
            ┌───────────────────────┐
            │  Reporter Module      │
            │  - Console output     │
            │  - Real-time updates  │
            │  - JSON export        │
            └───────────────────────┘
```

## Module Breakdown

### 1. Configuration Module (`src/config.rs`)

**Responsibility**: Load, validate, and merge configuration from multiple sources.

**Key Components**:
- `Config`: Main configuration struct
- `TargetConfig`: Per-target settings (URL, method, headers, body). Includes `effective_id(index)` helper for consistent ID generation.
- `TrafficPattern`: Load pattern definitions (Fixed, RateLimit, Ramp, Burst)
- `StressPattern`: Stress testing patterns (ConnectionFlood, Slowloris, etc.)
- `LoadDistribution`: Multi-target distribution strategies

**Data Flow**:
```
Config File (YAML/TOML) ──┐
                          ├──► Config::load() ──► Validated Config
CLI Arguments ────────────┘
```

**Key Design Decisions**:
- CLI arguments override config file settings
- Validation happens during load to fail fast
- Separate stress and normal traffic patterns
- Safety limits optional for stress patterns (authorization removed)

### Safety Limits (for Stress Testing)

Safety limits are optional and enforced when `safety_limits` are configured for stress patterns. Authorization has been removed (assumed).

**Key Design Decisions**:
- Safety limits are user-configurable
- Enforced at config load for stress patterns
- No hard-coded limits (see NO_HARDCODED_LIMITS.md for historical verification)

### 3. Discovery Module (`src/discovery.rs`)

**Responsibility**: Validate connectivity and discover services before load testing.

**Key Components**:
- `PortDiscoveryConfig`: Discovery configuration per target
- `discover_targets()`: Multi-target parallel discovery
- `check_tcp_port()`: TCP connectivity check with retries
- `detect_http_service()`: HTTP/HTTPS service detection

**Architecture**:
```
Multiple Targets
    │
    ▼
discover_targets() ──► Spawn parallel tasks
    │
    ├──► Target 1: scan_ports() ──┐
    ├──► Target 2: scan_ports() ──┤
    └──► Target 3: scan_ports() ──┤
                                   │
                        Semaphore (max 10 concurrent/target)
                                   │
                        ┌──────────┴───────────┐
                        │                      │
                   check_tcp_port()    detect_http_service()
                        │                      │
                        └──────────┬───────────┘
                                   │
                              PortInfo / PortFailure
```

**Concurrency Model**:
- **Outer level**: Unlimited parallel target discovery
- **Inner level**: Max 10 concurrent port scans per target (semaphore-controlled)
- **Retry logic**: Exponential backoff for TCP connections

**Key Design Decisions**:
- Parallel discovery for multiple targets
- Bounded concurrency per target to avoid overwhelming network
- Graceful degradation with FailureAction (Fail/Skip/Warn)
- Service detection optional for performance

### 4. HTTP Client Module (`src/client.rs`)

**Responsibility**: Execute HTTP/HTTPS requests with connection pooling.

**Key Components**:
- `HttpClient`: Main client struct
- `ClientMode`: Single-target vs multi-target mode
- Request execution methods (execute, execute_and_hold, send_partial_request, slow_read)

**Architecture**:
```
HttpClient
    │
    ├─ Single-Target Mode
    │   └─ Arc<TargetConfig>
    │
    └─ Multi-Target Mode
        └─ Arc<TargetSelector> ──► Select target per request
```

**Connection Pooling**:
- Uses `reqwest` client with built-in connection pooling
- Default: 128 idle connections per host
- TCP keep-alive: 60 seconds
- Configurable timeout per request

**Key Design Decisions**:
- Single client instance shared across all tasks (Arc)
- Mode enum avoids runtime checks
- Connection pool managed by reqwest (battle-tested)
- Stress testing methods (execute_and_hold, send_partial_request) for specialized patterns

### 5. Target Selector Module (`src/target_selector.rs`)

**Responsibility**: Distribute load across multiple targets.

**Distribution Strategies**:

1. **RoundRobin**:
   ```
   Counter (AtomicUsize)
   ├─ fetch_add(1) ──► index = counter % targets.len()
   └─ Return targets[index]
   ```
   - Lock-free (atomic operations)
   - Even distribution guaranteed

2. **Weighted**:
   ```
   Random(0..1) ──► Cumulative weight distribution
   ├─ 0.0 - 0.7 ──► Target 0 (70%)
   ├─ 0.7 - 0.9 ──► Target 1 (20%)
   └─ 0.9 - 1.0 ──► Target 2 (10%)
   ```
   - Proportional distribution
   - Handles non-normalized weights

3. **Random**:
   ```
   RNG.gen_range(0..targets.len()) ──► Random index
   ```
   - Uniform distribution
   - Simple and fast

4. **Hash** (planned):
   - Currently falls back to RoundRobin
   - Future: Hash request fields for sticky routing

**Thread Safety**:
- Shared via `Arc` across async tasks
- AtomicUsize for counter (lock-free)
- No mutexes needed

### 6. Pattern Executor Module (`src/patterns.rs`)

**Responsibility**: Execute different load testing patterns.

**Traffic Patterns**:

1. **Fixed Concurrency**:
   ```
   Semaphore(N) ──► Max N concurrent requests
       │
       └─ Loop: spawn tasks until limit/duration reached
   ```
   - Constant load level
   - Bounded by semaphore

2. **Rate-Limited**:
   ```
   Interval(1s / rate) ──► Ticker fires every interval
       │
       └─ Spawn request on each tick
   ```
   - Precise rate control
   - Uses tokio interval for accurate timing

3. **Ramp-Up**:
   ```
   Steps (10+) ──► Gradual increase
       │
       ├─ Step 1: from concurrency ──► run for step_duration
       ├─ Step 2: intermediate      ──► run for step_duration
       └─ Step N: to concurrency    ──► hold for hold_duration
   ```
   - Smooth load increase
   - Configurable ramp duration

4. **Burst**:
   ```
   Interval(burst_interval) ──► Ticker fires periodically
       │
       └─ Spawn burst_size concurrent requests
           │
           └─ Wait for all to complete before next burst
   ```
   - Periodic load spikes
   - Tests burst handling

**Execution Model**:
```
PatternExecutor::execute()
    │
    ├─ Fixed ──► execute_fixed()
    ├─ RateLimit ──► execute_rate_limit()
    ├─ Ramp ──► execute_ramp()
    └─ Burst ──► execute_burst()
        │
        └──► spawn(async move {
                client.execute()
                metrics.record(result)
             })
```

**Key Design Decisions**:
- Each pattern spawns independent async tasks
- Cancellation token for graceful shutdown
- Metrics recorded per request
- No shared mutable state (Arc/Clone pattern)

### 7. Stress Executor Module (`src/stress.rs`)

**Responsibility**: Execute aggressive stress testing patterns.

**Stress Patterns**:

1. **Connection Flood**:
   - Opens connections at high rate
   - Holds connections open (configurable duration)
   - Tests connection pool exhaustion

2. **Slowloris**:
   - Opens connections
   - Sends partial HTTP headers slowly
   - Never completes request
   - Tests timeout handling

3. **Slow POST**:
   - Sends POST body very slowly
   - Ties up server resources
   - Tests request timeout policies

4. **Request Flood**:
   - Sends requests at extreme rates
   - Tests throughput limits
   - Similar to rate-limited but no limits

5. **Large Payload**:
   - Sends very large request bodies
   - Tests bandwidth and memory limits
   - Concurrent large requests

6. **Pipeline Abuse**:
   - Sends many requests per connection
   - Abuses HTTP pipelining
   - Tests connection handling

7. **Slow Read**:
   - Reads response data very slowly
   - Ties up server connections
   - Tests server timeout policies

Stress tests run directly (authorization is assumed).

### 8. Metrics Module (`src/metrics.rs`)

**Responsibility**: Collect and aggregate request metrics.

**Architecture**:
```
MetricsCollector (single-target)
    │
    └─ Arc<Mutex<MetricsInner>>
        ├─ Vec<u64>: latencies_us (pre-allocated 10,000)
        ├─ HashMap<u16, usize>: status_codes (pre-allocated 10)
        ├─ HashMap<String, usize>: errors (pre-allocated 20)
        └─ ConnectionStats: error categorization

MultiTargetMetrics
    │
    ├─ MetricsCollector: global
    └─ HashMap<String, TargetMetrics>: per-target
```

**Performance Optimizations**:
1. **Pre-allocation**:
   ```rust
   Vec::with_capacity(10_000)      // Typical workload size
   HashMap::with_capacity(10)       // Common status codes
   HashMap::with_capacity(20)       // Error types
   ```

2. **Error Categorization** (56% faster):
   ```rust
   // Before: String allocation
   error.to_lowercase().contains("timeout")  // ❌ Slow

   // After: Zero-allocation byte comparison
   contains_ignore_case(error.as_bytes(), "timeout")  // ✅ 2.3x faster
   ```

3. **Lock Minimization**:
   - ConnectionStats uses AtomicUsize (lock-free)
   - Mutex only for aggregate metrics
   - Short critical sections

**Key Design Decisions**:
- Immutable snapshots for reading
- Pre-allocation for common workload sizes
- Zero-allocation error categorization
- Per-target metrics for multi-target tests

### 9. Statistics Module (`src/stats.rs`)

**Responsibility**: Calculate percentiles and statistics from raw metrics.

**HDR Histogram**:
```
Raw Latencies (microseconds)
    │
    ▼
HDR Histogram (1μs - 1 hour range, 3 significant digits)
    │
    ├─ record() for each latency
    │
    └─ Percentile queries:
        ├─ P50 (median)
        ├─ P90
        ├─ P95
        ├─ P99
        └─ P99.9
```

**Why HDR Histogram**:
- Accurate percentiles even with large datasets
- Constant memory usage (configurable precision)
- Wide value range support
- Industry standard (used by Netflix, Google, etc.)

**Statistics Calculation**:
```
MetricsSnapshot
    │
    ▼
Statistics::from_snapshot()
    │
    ├─ Latency stats (HDR histogram)
    ├─ Throughput (requests / duration)
    ├─ Success/error rates
    ├─ Status code distribution (sorted by frequency)
    └─ Error distribution (sorted by frequency)
```

### 10. Reporter Module (`src/reporter.rs`)

**Responsibility**: Display and export test results.

**Output Modes**:

1. **Real-time Updates**:
   ```
   Terminal control sequences (ANSI)
       │
       ├─ Move cursor up (previous output)
       ├─ Clear line
       └─ Print new stats
   ```
   - Non-blocking updates
   - Overwrite previous output

2. **Final Summary**:
   ```
   ================================================================================
                               FINAL RESULTS
   ================================================================================
   Duration:              60.00s
   Total Requests:        10,000
   Successful:            9,950 (99.5%)
   Failed:                50 (0.5%)
   Requests/sec:          166.67

   --------------------------------------------------------------------------------
   LATENCY STATISTICS (milliseconds)
   --------------------------------------------------------------------------------
   Min:                   10.23
   Max:                   245.67
   Mean:                  45.32
   P50 (median):          42.15
   P90:                   78.91
   P99:                   156.34
   ```

3. **JSON Export**:
   ```json
   {
     "duration": {"secs": 60, "nanos": 0},
     "total_requests": 10000,
     "latency": {
       "p50_ms": 42.15,
       "p99_ms": 156.34
     }
   }
   ```

**Key Design Decisions**:
- Separate concerns: formatting vs calculation
- JSON export for automation/CI
- Real-time updates optional (can be noisy)

## Concurrency Model

### Async Runtime: Tokio

All I/O operations use Tokio's async runtime:
```
tokio::main
    │
    └─ Multi-threaded work-stealing scheduler
        ├─ Pattern executor tasks
        ├─ HTTP client requests
        ├─ Discovery tasks
        └─ Metrics collection
```

**Task Spawning Strategy**:
- Each request = separate task (`tokio::spawn`)
- Bounded by pattern constraints (semaphore/rate)
- Automatic load balancing by Tokio scheduler

**Thread Safety Patterns**:
1. **Arc + Clone**: Share immutable data
2. **Arc + Mutex**: Share mutable data (minimal lock time)
3. **AtomicUsize**: Lock-free counters
4. **Message Passing**: Avoid shared state where possible

### Synchronization Primitives

1. **Semaphore** (Fixed pattern):
   ```rust
   let semaphore = Arc::new(Semaphore::new(concurrent));
   let permit = semaphore.acquire().await?;
   // ... do work ...
   drop(permit);  // Release
   ```

2. **CancellationToken** (Graceful shutdown):
   ```rust
   if cancel_token.is_cancelled() {
       break;
   }
   ```

3. **Mutex** (Metrics aggregation):
   ```rust
   let mut inner = self.inner.lock().unwrap();
   inner.total_requests += 1;
   // Short critical section
   ```

4. **AtomicUsize** (Connection stats):
   ```rust
   self.refused_count.fetch_add(1, Ordering::Relaxed);
   ```

## Data Flow Examples

### Single-Target Load Test

```
User: cargo run -- --url https://api.example.com --concurrent 50 --duration 60

Config::load()
    │
    └─ CLI args → Config{concurrent: 50, duration: 60}
        │
        └─ HttpClient::new(target, timeout, pool_size)
            │
            └─ PatternExecutor::new(client, metrics, Fixed{50, 60})
                │
                └─ execute(cancel_token)
                    │
                    ├─ Semaphore::new(50)  // Max 50 concurrent
                    │
                    └─ Loop for 60 seconds:
                        │
                        ├─ Acquire permit
                        ├─ spawn(async move {
                        │     client.execute()
                        │     metrics.record(result)
                        │  })
                        └─ Repeat
                            │
                            └─ After 60s:
                                │
                                └─ Reporter::show_final_summary(stats)
```

### Multi-Target Load Test

```
Config{targets: [api1, api2, api3], distribution: RoundRobin}
    │
    └─ TargetSelector::new(targets, RoundRobin)
        │
        └─ HttpClient::new_multi_target(selector, ...)
            │
            └─ PatternExecutor::new_multi_target(client, multi_metrics, ...)
                │
                └─ execute()
                    │
                    └─ spawn(async move {
                           let target = selector.select()  // RoundRobin
                           client.execute()  // Uses selected target
                           multi_metrics.record(result)  // Per-target tracking
                       })
```

### Stress Testing

```
Config{stress_pattern: ConnectionFlood}
    │
    (authorization assumed)
    ├─ Validate safety limits (if configured)
    └─ StressExecutor::new(client, metrics, pattern)
        │
        └─ execute_connection_flood()
            │
            └─ Interval(connections_per_second)
                        │
                        └─ spawn(async move {
                               client.execute_and_hold(hold_duration)
                           })
```

## Performance Characteristics

### Throughput

**Theoretical Maximum**:
- Limited by: Network bandwidth, target capacity, client resources
- Typical: 5,000-50,000 req/s depending on workload

**Factors**:
- Connection pooling (128 idle/host)
- Async I/O (minimal thread overhead)
- Lock-free operations where possible

### Memory Usage

**Per Request**:
- RequestResult: ~128 bytes
- Latency storage: 8 bytes (u64)
- Status code: 2 bytes (u16)
- Error string: ~50 bytes average

**With 10,000 concurrent requests**:
- Base: ~1.5 MB
- Connection pool: ~10 MB
- Tokio runtime: ~5 MB
- **Total**: ~15-20 MB typical

### Latency Overhead

**Metrics Collection**: <1μs per request
- Optimized error categorization: 82ns
- Atomic operations: ~5ns
- Mutex lock: ~50ns (short critical section)

**HTTP Request Overhead**: ~1-5ms
- Connection setup (if not pooled): 1-50ms
- TLS handshake (if not reused): 50-200ms
- Request/response: depends on network

## Error Handling Strategy

### Error Categories

1. **Connection Errors** (categorized):
   - Connection refused (ECONNREFUSED)
   - Connection timeout (ETIMEDOUT)
   - Connection reset (ECONNRESET)
   - TLS/SSL errors
   - DNS errors

2. **HTTP Errors**:
   - 4xx client errors
   - 5xx server errors
   - Timeout errors

3. **Configuration Errors**:
   - Invalid URLs
   - Missing required fields
   - Validation failures

### Error Propagation

```
Low-level error (reqwest)
    │
    ▼
Categorize & record (metrics)
    │
    ▼
Continue execution (don't fail fast)
    │
    ▼
Report in final statistics
```

**Philosophy**: Errors are data, not exceptions
- Record all errors for analysis
- Don't stop test on individual failures
- Provide detailed error distribution

## Security Considerations

### Stress Testing

Authorization requirement has been removed. The tool assumes the operator is authorized.

**Safety**:
- Optional `safety_limits` are still validated and enforced when provided.
- No hard-coded limits.
- Users remain responsible for obtaining proper authorization.

### TLS/HTTPS

**Production Use**:
- Full certificate validation
- Modern TLS versions (1.2+)
- Secure cipher suites

**Discovery Only**:
- `danger_accept_invalid_certs` for service detection
- Not used for actual load testing
- Clearly documented as discovery-only

### Input Validation

- URL parsing with `url` crate
- Configuration validation before execution
- Safety limits for stress testing
- Bounds checking on all user inputs

## Extensibility

The design centralizes behavior on the data types themselves, making common operations (validation, human-readable descriptions, ID handling) available directly on the structs/enums.

### Working with Patterns (Library Use)

- `TrafficPattern` and `StressPattern` implement:
  - `.validate()` — central validation logic
  - `.describe()` (and `Display`) — human-readable description (used for startup info and stress warnings)
- `TargetConfig::effective_id(index: Option<usize>)` — returns configured ID or a sensible default (`"target"` or `"target-N"`)

### Adding New Traffic Patterns

1. Add variant to `TrafficPattern` enum (config.rs)
2. Implement execution logic in `PatternExecutor` (patterns.rs)
3. Implement `.validate()` and `.describe()` on the new variant (in the `impl TrafficPattern` block)
4. Add configuration parsing / tests as needed

### Adding New Stress Patterns

1. Add variant to `StressPattern` enum (config.rs)
2. Implement execution in `StressExecutor` (stress.rs)
3. Implement `.validate_against(&self, limits: &SafetyLimits)` and `.describe()` on the new variant
4. Add tests (safety limits optional)

### Adding New Distribution Strategies

1. Add variant to `LoadDistribution` enum (config.rs)
2. Implement selection logic in `TargetSelector` (target_selector.rs)
3. Add tests for distribution fairness

## Testing Strategy

### Unit Tests (current count in tests + src doctests)
- Per-module functionality
- Edge cases and error conditions
- Located in `src/` files

### Integration Tests (current count)
- Cross-module functionality
- Real-world scenarios
- Located in `tests/` directory

### Benchmarks
- Micro-benchmarks with Criterion
- Hot path performance measurement
- Located in `benches/` directory

**Total: 99 tests, 100% pass rate**

## Dependencies

### Core Dependencies
- `tokio`: Async runtime
- `reqwest`: HTTP client
- `serde`: Serialization
- `clap`: CLI parsing
- `anyhow`: Error handling

### Performance
- `hdrhistogram`: Percentile calculation
- Optimized for zero-allocation hot paths

### Configuration
- `serde_yaml`: YAML parsing
- `toml`: TOML parsing
- `serde_json`: JSON export

## Future Improvements

### Short-term
1. Property-based testing (proptest)
2. HTTP/2 server push testing
3. WebSocket support
4. gRPC support

### Medium-term
1. Distributed load testing (coordinator + workers)
2. Custom protocol support
3. Plugin system for custom metrics
4. Real-time dashboards

### Long-term
1. Record/replay functionality
2. AI-driven load pattern generation
3. Chaos engineering features
4. Cloud provider integrations

## References

- **HDR Histogram**: http://hdrhistogram.org/
- **Tokio**: https://tokio.rs/
- **Reqwest**: https://docs.rs/reqwest/
- **Load Testing Best Practices**: Industry standards for responsible testing
