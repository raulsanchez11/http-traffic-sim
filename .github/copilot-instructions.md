# Copilot instructions for `http-traffic-sim`

## Big picture architecture
- Entry point is `src/main.rs`; execution is routed by `Config::get_execution_mode()` into `SingleTarget`, `MultiTarget`, or `StressTest`.
- Config parsing/validation lives in `src/config.rs` (`CliArgs` + file config merge + mode and safety checks).
- Request execution is split by responsibility:
  - `src/client.rs`: builds/sends HTTP requests (`reqwest`), plus stress helpers (`execute_and_hold`, `send_partial_request`, `slow_read`).
  - `src/patterns.rs`: normal traffic patterns (`Fixed`, `RateLimit`, `Ramp`, `Burst`).
  - `src/stress.rs`: stress patterns (`ConnectionFlood`, `RequestFlood`, `Slowloris`, etc.).
- Metrics flow: request result → `src/metrics.rs` collectors (`MetricsCollector` / `MultiTargetMetrics`) → `src/stats.rs` percentile/stat computation → `src/reporter.rs` realtime/final output + JSON export.
- Multi-target routing boundary is `src/target_selector.rs` (distribution strategy only); transport remains in `HttpClient`.

## Project-specific behavior to preserve
- Stress mode requires explicit authorization (`authorization.confirmed: true`) and shows a 5-second warning (`src/authorization.rs`).
- Safety limits are intentionally user-configurable and optional (`SafetyLimits` defaults to `None`); do not introduce hard-coded caps (see `NO_HARDCODED_LIMITS.md`).
- CLI args override config-file values in `Config::load()`; preserve this precedence.
- `ExecutionMode` semantics are intentional: `stress_pattern` takes precedence, then `targets`, then single target.
- Per-target stats in multi-target mode are first-class output, not just global aggregates.

## Developer workflows
- Build debug: `cargo build`
- Build release: `cargo build --release`
- Run with config: `cargo run --release -- --config config.multi-target.example.yaml`
- Run single target quickly: `cargo run --release -- --url https://httpbin.org/get --concurrent 10 --duration 10`
- Format before finalizing edits: `cargo fmt`
- Lint before finalizing edits: `cargo clippy --all-targets --all-features`

## Code patterns and conventions
- Shared state is mostly `Arc` + `Mutex`/`AtomicUsize`; match existing concurrency style (avoid introducing channels/actors unless necessary).
- Cancellation uses `tokio_util::sync::CancellationToken` propagated from `main.rs`; new long-running loops should check cancellation early.
- Error accounting depends on string categorization in `ConnectionStats::categorize_and_increment`; keep error text meaningful when changing client/stress code.
- Stats rely on microsecond latency storage (`latencies_us`) and `hdrhistogram`; keep units consistent when adding metrics.
- Keep config examples aligned with enum serde tags:
  - `TrafficPattern`: `type: fixed|ratelimit|ramp|burst`
  - `StressPattern`: `category: connectionflood|requestflood|slowloris|slowpost|largepayload|pipelineabuse|slowread`

## Integration points
- External crates central to behavior: `reqwest` (HTTP), `tokio` (runtime), `clap` + `serde_*`/`toml` (config), `hdrhistogram` (latency stats), `tracing` (logs).
- Config reference files in repo root are executable documentation; prefer updating those when adding/changing options.