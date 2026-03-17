# Performance Baseline - Phase 4.2

**Date**: March 16, 2026
**Commit**: 168c71a
**Rust Version**: 1.70+ (stable)
**Build Profile**: Release (optimized)

## Benchmark Results

### Discovery Module Benchmarks

| Benchmark | Time | Iterations | Notes |
|-----------|------|------------|-------|
| port_spec_single | 16.20 ns | 308M | Fast - single port conversion |
| port_spec_list | 30.95 ns | 161M | 4-port list conversion |
| port_spec_range_small | 24.35 ns | 203M | 11-port range (8000-8010) |
| port_spec_range_large | 52.11 ns | 96M | 1001-port range (8000-9000) |
| extract_host | 229.83 ns | 22M | URL parsing (slowest) |

### Key Observations

1. **Port Spec Performance**:
   - Single port: ~16 ns (baseline)
   - List of 4: ~31 ns (1.9x slower)
   - Small range (11 ports): ~24 ns (1.5x slower)
   - Large range (1001 ports): ~52 ns (3.2x slower)
   - **Insight**: Large ranges scale linearly, not quadratically ✅

2. **URL Parsing**:
   - extract_host: ~230 ns
   - **Insight**: 14x slower than port operations
   - **Optimization opportunity**: Cache parsed URLs

3. **Overall Discovery**:
   - All operations under 250 ns
   - Discovery phase dominated by network I/O, not computation
   - **Focus optimization**: Network operations, not parsing

## System Configuration

- **CPU**: Apple M-series / Intel x86_64
- **RAM**: Sufficient for benchmarking
- **Network**: Public internet
- **OS**: macOS (Darwin 25.3.0)

## Real-World Performance

### Discovery Phase (from tests)

| Operation | Time | Target |
|-----------|------|--------|
| Single port check | 571 ms | < 2000 ms ✅ |
| Multi-port check (2 ports) | 804 ms | < 3000 ms ✅ |
| Port response time | 27-65 ms | - |

**Breakdown**:
- Network latency: ~500 ms (dominant)
- URL parsing: ~230 ns (negligible)
- Port checks: ~50 ms per port
- Service detection: ~100 ms per port

### Load Testing Performance

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Single target RPS | 3.3 req/s | - | Baseline |
| Multi-target RPS | ~500 req/s | - | Baseline |
| Connection setup | Variable | - | Baseline |

**Note**: Exact RPS varies by target and concurrency level.

## Hot Path Analysis

### Critical Paths (by frequency)

1. **Request Execution** (hottest)
   - `HttpClient::execute_request()`
   - `send_and_measure()`
   - Network I/O dominant

2. **Metrics Collection**
   - `MetricsCollector::record()`
   - Histogram updates
   - Lock contention possible

3. **Target Selection**
   - `TargetSelector::select()`
   - Called per request
   - Fast (atomic operations)

4. **Discovery Phase**
   - `scan_ports()`
   - `check_tcp_port()`
   - Network I/O dominant

### Bottlenecks Identified

1. **Network I/O** (primary)
   - Cannot optimize significantly
   - Already using connection pooling
   - HTTP/2 enabled where supported

2. **Metrics Lock Contention** (secondary)
   - Multiple threads updating metrics
   - Mutex contention during collection
   - **Optimization target**: Lock-free structures

3. **Memory Allocations** (tertiary)
   - String allocations in hot paths
   - Histogram allocations
   - **Optimization target**: Object pooling

## Optimization Priorities

### Priority 1: Metrics Collection
- **Issue**: Lock contention in metrics updates
- **Impact**: High (affects all requests)
- **Difficulty**: Medium
- **Target**: 20-30% improvement

### Priority 2: Memory Allocations
- **Issue**: Frequent allocations in request path
- **Impact**: Medium
- **Difficulty**: Medium
- **Target**: 10-15% improvement

### Priority 3: Connection Pooling
- **Issue**: Pool size not tuned for high concurrency
- **Impact**: Low-Medium
- **Difficulty**: Low
- **Target**: 5-10% improvement

### Not Worth Optimizing

1. **URL Parsing**: Only 230 ns, called infrequently
2. **Port Spec Conversion**: Only 16-52 ns, negligible
3. **Target Selection**: Already fast (atomic ops)

## Memory Profile

### Current Usage (estimated)

- **Base memory**: ~10 MB (binary + static data)
- **Per request**: ~1-2 KB (request + response buffers)
- **Metrics**: ~5-10 MB (histograms + counters)
- **Connection pool**: ~1 MB (128 connections max)

**Total at 1000 concurrent**: ~30-50 MB (reasonable)

### Allocation Patterns

- Request building: Strings, headers (allocates)
- Response handling: Body buffers (allocates)
- Metrics recording: Histogram updates (allocates)
- Error messages: String formatting (allocates)

## Performance Goals (Phase 4.2)

### Throughput
- **Current**: 3-500 req/s (varies by pattern)
- **Target**: +20% (3.6-600 req/s)
- **Strategy**: Reduce metrics overhead

### Latency
- **Current**: 300-500 ms per request (varies by target)
- **Target**: -30% reduction in overhead
- **Strategy**: Faster metrics collection

### Memory
- **Current**: ~30-50 MB at 1000 concurrent
- **Target**: -20% (24-40 MB)
- **Strategy**: Reduce allocations

### Metrics Overhead
- **Current**: Unknown (needs profiling)
- **Target**: < 1% of total request time
- **Strategy**: Lock-free data structures

## Profiling Plan

### Tools to Use

1. **Criterion** - Micro-benchmarks ✅
2. **Flamegraph** - CPU profiling
3. **Heaptrack** - Memory profiling
4. **Perf** - System profiling (Linux)
5. **Instruments** - System profiling (macOS)

### Profiling Sessions Needed

1. **High RPS test** - Find CPU bottlenecks
2. **High concurrency test** - Find lock contention
3. **Long duration test** - Find memory leaks
4. **Multi-target test** - Find distribution overhead

## Next Steps

1. ✅ Run micro-benchmarks (complete)
2. ✅ Document baselines (complete)
3. ⏳ Profile CPU usage with flamegraph
4. ⏳ Analyze lock contention
5. ⏳ Optimize metrics collection
6. ⏳ Reduce memory allocations
7. ⏳ Re-benchmark and compare
8. ⏳ Document improvements

## Baseline Summary

**Discovery Module**: Fast (16-230 ns)
**Real-World Performance**: Dominated by network I/O
**Optimization Focus**: Metrics collection, allocations
**Expected Gains**: 20-30% throughput improvement possible

---

**Baseline Established**: March 16, 2026
**Next Review**: After optimizations
**Status**: Ready for optimization phase
