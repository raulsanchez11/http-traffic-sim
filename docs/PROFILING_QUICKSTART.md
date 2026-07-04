# Profiling Quick Start

Quick reference for profiling `http-traffic-sim`.

## One-Command Profiling

### CPU Profiling (Flamegraph)

```bash
# Using the helper script (recommended)
./scripts/profile.sh --url http://localhost:8080 --concurrent 100 --duration 30

# Manual
sudo cargo flamegraph --release -- \
  --url http://localhost:8080 \
  --concurrent 100 \
  --duration 30
```

**Output**: `flamegraph.svg` (open in browser)

### Micro-benchmarks

```bash
# Run all benchmarks
cargo bench

# Save baseline
./scripts/bench.sh --baseline main

# Compare against baseline
./scripts/bench.sh --compare main
```

## Prerequisites

### macOS
```bash
# Flamegraph (uses DTrace)
cargo install flamegraph

# That's it! DTrace is built-in
```

### Linux
```bash
# Flamegraph
cargo install flamegraph

# perf tools
sudo apt-get install linux-tools-common linux-tools-generic

# Or on Fedora
sudo dnf install perf
```

### Windows
```bash
# Currently not supported
# Use WSL2 with Linux instructions
```

## Quick Checks

### Is My Code the Bottleneck?

```bash
# Profile with local target
python3 -m http.server 8080 &
sudo cargo flamegraph --release -- \
  --url http://localhost:8080 \
  --concurrent 100 \
  --duration 10

# Check flamegraph:
# - Is application code >10%? → Investigate
# - Is network I/O >40%? → Normal
# - Is metrics <5%? → Good (✅ after optimization)
```

### Comparing Performance

```bash
# Before changes
cargo bench -- --save-baseline before

# Make changes
# ...

# After changes
cargo bench -- --baseline before

# Look for regressions (red) or improvements (green)
```

## Interpreting Results

### Flamegraph

**Good Signs** ✅:
- Most time in network I/O (40-70%)
- HTTP operations (20-30%)
- Async runtime overhead (10-15%)
- Application code (<5%)

**Investigate** ⚠️:
- Application code >10%
- Unexpected string operations
- Lock contention
- Repeated allocations

### Benchmarks

**Good Results** ✅:
```
metrics_record_with_error
  time:   [82.341 ns 82.892 ns 83.476 ns]
  change: [-56.2% -55.8% -55.4%] ✅ Improvement!
```

**Regression** ⚠️:
```
metrics_record
  time:   [62.341 ns 65.892 ns 68.476 ns]
  change: [+45.2% +48.8% +52.4%] ⚠️ Regression!
```

## Common Commands

```bash
# Profile current code
sudo cargo flamegraph --release -- --url http://localhost:8080 --concurrent 100 --duration 30

# Benchmark all
cargo bench

# Benchmark specific
cargo bench metrics

# Save baseline
cargo bench -- --save-baseline main

# Compare
cargo bench -- --baseline main

# View benchmark history
ls target/criterion/*/report/index.html
```

## Tips

1. **Always use `--release`** - Debug is 10-100× slower
2. **Use local targets** - Eliminates network variance
3. **Run long enough** - 30+ seconds for good samples
4. **Close other apps** - Reduce system noise
5. **Profile under load** - 100+ concurrent for realistic results

## See Also

- Old profiling sessions are in git history for reference.
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Optimization guide
- [End-to-End Benchmarks](END_TO_END_BENCHMARKS.md) - Full benchmarks

---

**Quick Help**: Run `./scripts/profile.sh` without args for usage
