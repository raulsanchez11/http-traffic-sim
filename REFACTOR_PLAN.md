# Refactoring Design: Eliminating Duplicated Pattern Logic and Orchestration Spaghetti in http-traffic-sim

**Author:** Grok (Systems Architect)  
**Date:** 2026-07-03  
**Status:** Draft  
**Project:** http-traffic-sim (Rust workspace at /Users/chef/dev/nh)  
**Scope:** src/config.rs, src/patterns.rs, src/stress.rs, src/run.rs, src/authorization.rs, src/client.rs, supporting tests (full suite ~140 unit+integration + doc-tests), no public API changes. Hot-path work explicitly excluded (see Non-Goals + separate follow-up).

---

## Overview

This design addresses strict code quality findings from a review of the http-traffic-sim load/stress testing tool. The core problems are duplicated "shape knowledge" of pure-data enums `TrafficPattern` and `StressPattern` (defined at `src/config.rs:160` and `src/config.rs:226`) via large external `match` statements for validation and description (plus structured printing), repeated default-target-id logic (exactly 6 sites), identical `drain_join_set` helpers, and ad-hoc branching in the 517-line orchestration module `src/run.rs`.

The proposed solution moves canonical behavior (validation and human-readable description) onto the types via inherent `impl` blocks in `config.rs`. This deletes (not merely moves) the duplicate matches for *validate + describe/description* (in `src/config.rs:438` inside `validate_traffic_pattern`, `src/config.rs:566` inside `validate_safety_limits`, `src/authorization.rs:200` `pattern_description`, and the detailed printing knowledge in `src/run.rs:319` inside `print_startup_info`). Execution dispatch matches (`src/patterns.rs:47` inside `PatternExecutor::execute`, `src/stress.rs:28` inside `StressExecutor::execute`) and runtime bail checks inside the private execute_* methods remain by design. Additional canonical helpers centralize target ID resolution (`TargetConfig::effective_id`) and join draining. Orchestration boilerplate in `src/run.rs` is incrementally cleaned while preserving 100% identical observable behavior for traffic paths and semantically equivalent (with documented details) for stress (all tests + manual flows).

Result: adding a `TrafficPattern` or `StressPattern` variant touches fewer sites for the high-impact concerns; `run.rs` legibility improves; incidental complexity drops with no structural regression and no new public API surface.

---

## Background & Motivation

Current state (verified via source exploration with read_file + grep on 2026-07-03):

- `TrafficPattern` (4 variants: `Fixed`, `RateLimit`, `Ramp`, `Burst` at `src/config.rs:160`) and `StressPattern` (7 variants at `src/config.rs:226`) are pure data + `Serialize`/`Deserialize` + `Default` (for traffic).
- Behavior for validation/description lives in external matches (shape knowledge duplication):
  - `src/patterns.rs:47` (execute dispatch + private `execute_fixed` etc.; see also rate bail at patterns.rs:150, ramp at:200).
  - `src/stress.rs:28` (execute dispatch + 7 private `execute_*` at ~106+; plus host parsing in `execute_slowloris:164`, `execute_slow_post:189` etc.; bails inside).
  - `src/run.rs:279` (`print_startup_info`: `match ExecutionMode` at 284 containing nested `match &config.pattern` at 319 + Debug `{pattern:?}` for stress at 311).
  - `src/authorization.rs:200` (`pattern_description` match, called from `display_stress_warning:160`).
  - `src/config.rs:438` (`validate_traffic_pattern` inside impl Config), `src/config.rs:566` (`validate_safety_limits` full 7-arm match on stress).
- `async fn drain_join_set(...)` is byte-identical at `src/patterns.rs:289` and `src/stress.rs:375`.
- Default target ID logic (exactly 6 sites, **all inside src/run.rs**; none in discovery.rs):
  - `execute_multi_target:101` (mut assignment in map: `let mut target = t.clone(); if target.id.is_empty() { target.id = format!("target-{i}"); }`)
  - `print_startup_info:298` (in multi targets loop)
  - `perform_discovery:412` (single: `"target"`), `perform_discovery:426` (multi)
  - `apply_discovery_results:461` (single), `apply_discovery_results:489` (multi)
- Reporter construction + emission: `Reporter::new(false)` + summary/export at `execute_stress_test:167/173` (inside ifs), `emit_results:186/195` (single vs multi paths).
- `PatternExecutor<R: RequestRecorder>` uses two `impl` blocks (`new:22`, `new_multi_target:32`) + generic execute (`src/patterns.rs:45`).
- Dozens of manual `if cancel_token.is_cancelled() || elapsed >= duration` checks (e.g. patterns:117,164; stress:134,186,...).
- Ad-hoc byte scanning lives inside `metrics::ConnectionStats::categorize_and_increment:40`.
- Client stress methods duplicate `parse_host_port` + `Url::parse` + path logic (`client.rs:170,189,248,301`).
- Module boundaries per `src/lib.rs:4-14` and `ARCHITECTURE.md` are clean; all `src/*.rs` <750 LOC (discovery 730, config 637, run 517); clean `cargo build`; full test suite passes.

Pain points (from review):
- Adding a variant requires coordinated edits in 4-5 files for validate/describe/print + risk of divergence.
- "Incidental complexity" from duplicated shape knowledge.
- `run.rs` has grown "spaghetti" branches.
- No file-size crisis yet, but continued growth without cleanup will create one.

Motivation aligns with `ARCHITECTURE.md` "Extensibility" section (which currently documents the multi-file touch model) and "Key Design Decisions" (separation of concerns).

---

## Goals & Non-Goals

**Goals**
- Centralize `validate()` and `describe()` (or `Display`) on `TrafficPattern`/`StressPattern` in `src/config.rs`; delete the external duplicate matches for *validate + describe/description/print-description concerns* (execution dispatch matches and inline runtime checks remain by design).
- Introduce canonical helpers: `TargetConfig::effective_id(index: Option<usize>)` (replaces the 6 sites), single `drain_join_set` (pub(crate)).
- Reduce (not eliminate) branching and Reporter construction duplication in `src/run.rs` via extraction and unified emission while preserving exact relative output order for stress (summary / conn stats / export).
- Make adding patterns/targets cheaper and localized (quantified: primary validate/describe now in 1 impl location vs. multiple external matches today).
- 100% identical observable behavior for traffic (console/JSON/startup output byte-identical); for stress, semantically equivalent with exact conn stats + export message content (minor documented reordering avoided by design choice; see reporter section). Full test suite + doc-tests + manual flows (single/multi/stress + discovery) continue to pass.
- Add small dedicated unit tests for `TrafficPattern::describe()` / `StressPattern::describe()` and a regression test covering default target ID assignment via `effective_id`.
- Boring, direct code. Small, focused PRs.
- Preserve module boundaries from `src/lib.rs` and `ARCHITECTURE.md` (no new public exports; no moving `PatternExecutor` into `config`).
- Improve legibility of `run.rs` orchestration and `print_startup_info`.

**Non-Goals**
- Do not introduce trait objects or visitor for patterns (see Alternatives).
- Do not unify single-target vs multi-target execution paths or always use `MultiTargetMetrics` in this effort (consider in future PR).
- Do not change public API (`pub use` in lib.rs, Config struct layout, enum variants, or signatures of `run()`, `PatternExecutor::new*`, `StressExecutor::new`).
- Do not rewrite cancellation handling comprehensively (secondary; targeted helper only if fits small PR).
- Do not touch benches/, docs/ beyond incidental, or add new runtime features/flags.
- Do not create new top-level modules (e.g. no `src/util.rs`) unless a 1-line boundary justification exists; prefer reuse inside existing modules.
- No performance claims or changes to hot paths.
- **Hot-path optimizations are explicitly a separate follow-up**: metrics (categorize_and_increment, record paths), patterns.rs / stress.rs (worker loops, cancel/elapsed checks, tickers, JoinSet usage). Do not optimize or touch for perf in this series.

**Follow-up Refactor (out of scope for this plan)**
A separate follow-up refactor (different branch / PR stack, after this one lands) will target hot-path work:
- `src/metrics.rs`: categorization logic, record paths, snapshot costs.
- `src/patterns.rs` + `src/stress.rs`: repeated cancel/elapsed checks inside loops, ticker setup, JoinSet draining, worker spawn patterns.
This series deliberately leaves those untouched (no perf claims, no loop changes).

---

## Proposed Design

### 1. Centralize Pattern Behavior on the Types (Highest Impact)

Add inherent impls in `src/config.rs` (after the enum definitions and existing `impl Default`).

**Proposed additions (concrete, full verbatim describe):**

```rust
// src/config.rs (after TrafficPattern Default impl ~ line 200)
impl TrafficPattern {
    /// Canonical validation. Replaces (delegates from) Config::validate_traffic_pattern
    /// and the rate==0 check in Config::pattern_from_args.
    pub fn validate(&self) -> Result<()> {
        match self {
            TrafficPattern::RateLimit { rate, .. } if *rate == 0 => {
                anyhow::bail!("Rate limit must be at least 1 request per second");
            }
            TrafficPattern::Ramp { from, to, .. } if from > to => {
                anyhow::bail!("Ramp 'from' ({from}) must be <= 'to' ({to})");
            }
            _ => {}
        }
        Ok(())
    }

    /// Human + structured description.
    /// For TrafficPattern this returns a multi-line block using *exact* text from the
    /// original printlns inside print_startup_info (run.rs:319-378) so that
    ///   println!("{}", pattern.describe());
    /// produces byte-identical output.
    /// For contexts needing compact (logs), callers can use or a future label() helper.
    pub fn describe(&self) -> String {
        match self {
            TrafficPattern::Fixed {
                concurrent,
                duration_secs,
                total_requests,
            } => {
                let mut lines = vec![
                    "Pattern:               Fixed Concurrency".to_string(),
                    format!("Concurrent Clients:    {}", concurrent),
                ];
                if let Some(duration) = duration_secs {
                    lines.push(format!("Duration:              {}s", duration));
                }
                if let Some(total) = total_requests {
                    lines.push(format!("Total Requests:        {}", total));
                }
                lines.join("\n")
            }
            TrafficPattern::RateLimit {
                rate,
                duration_secs,
                total_requests,
            } => {
                let mut lines = vec![
                    "Pattern:               Rate Limited".to_string(),
                    format!("Rate:                  {} req/s", rate),
                ];
                if let Some(duration) = duration_secs {
                    lines.push(format!("Duration:              {}s", duration));
                }
                if let Some(total) = total_requests {
                    lines.push(format!("Total Requests:        {}", total));
                }
                lines.join("\n")
            }
            TrafficPattern::Ramp {
                from,
                to,
                ramp_duration_secs,
                hold_duration_secs,
            } => {
                let mut lines = vec![
                    "Pattern:               Ramp-up".to_string(),
                    format!("From:                  {} clients", from),
                    format!("To:                    {} clients", to),
                    format!("Ramp Duration:         {}s", ramp_duration_secs),
                ];
                if let Some(hold) = hold_duration_secs {
                    lines.push(format!("Hold Duration:         {}s", hold));
                }
                lines.join("\n")
            }
            TrafficPattern::Burst {
                size,
                interval_secs,
                duration_secs,
                total_bursts,
            } => {
                let mut lines = vec![
                    "Pattern:               Burst".to_string(),
                    format!("Burst Size:            {} requests", size),
                    format!("Burst Interval:        {}s", interval_secs),
                ];
                if let Some(duration) = duration_secs {
                    lines.push(format!("Duration:              {}s", duration));
                }
                if let Some(total) = total_bursts {
                    lines.push(format!("Total Bursts:          {}", total));
                }
                lines.join("\n")
            }
        }
    }
}

impl std::fmt::Display for TrafficPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.describe())
    }
}
```

Analogous for `StressPattern` (after line 260; describe body verbatim copy of old pattern_description):

```rust
impl StressPattern {
    pub fn describe(&self) -> String {
        // Exact body moved from authorization.rs:200 (pattern_description)
        match self {
            StressPattern::ConnectionFlood {
                connections_per_second,
                hold_time_ms,
                duration_secs,
            } => {
                format!(
                    "Connection Flood - {} conn/s, hold {}ms, duration {}s",
                    connections_per_second, hold_time_ms, duration_secs
                )
            }
            StressPattern::Slowloris {
                connections,
                headers_per_second,
                duration_secs,
            } => {
                format!(
                    "Slowloris - {} connections, {:.2} headers/s, duration {}s",
                    connections, headers_per_second, duration_secs
                )
            }
            StressPattern::SlowPost {
                connections,
                bytes_per_second,
                payload_size,
            } => {
                format!(
                    "Slow POST - {} connections, {} bytes/s, payload {} bytes",
                    connections, bytes_per_second, payload_size
                )
            }
            StressPattern::RequestFlood {
                target_rps,
                duration_secs,
            } => {
                format!(
                    "Request Flood - {} req/s, duration {}s",
                    target_rps, duration_secs
                )
            }
            StressPattern::LargePayload {
                size_mb,
                concurrent,
                duration_secs,
            } => {
                format!(
                    "Large Payload - {} MB, {} concurrent, duration {}s",
                    size_mb, concurrent, duration_secs
                )
            }
            StressPattern::PipelineAbuse {
                requests_per_connection,
                concurrent_connections,
            } => {
                format!(
                    "Pipeline Abuse - {} req/conn, {} connections",
                    requests_per_connection, concurrent_connections
                )
            }
            StressPattern::SlowRead {
                connections,
                read_rate_bps,
                duration_secs,
            } => {
                format!(
                    "Slow Read - {} connections, {} bytes/s, duration {}s",
                    connections, read_rate_bps, duration_secs
                )
            }
        }
    }

    pub fn validate_against(&self, limits: &SafetyLimits) -> Result<()> {
        // Exact logic + error messages moved from config.rs:566 (validate_safety_limits)
        match self {
            StressPattern::ConnectionFlood {
                connections_per_second,
                ..
            } => {
                if let Some(max) = limits.max_connections_per_second {
                    if *connections_per_second > max {
                        anyhow::bail!(
                            "Connection rate {} exceeds safety limit of {} conn/s. \
                            Adjust your config or increase safety_limits.max_connections_per_second",
                            connections_per_second, max
                        );
                    }
                }
            }
            StressPattern::RequestFlood { target_rps, .. } => {
                if let Some(max) = limits.max_requests_per_second {
                    if *target_rps > max {
                        anyhow::bail!(
                            "Request rate {} exceeds safety limit of {} req/s. \
                            Adjust your config or increase safety_limits.max_requests_per_second",
                            target_rps, max
                        );
                    }
                }
            }
            StressPattern::LargePayload { size_mb, .. } => {
                if let Some(max) = limits.max_payload_size_mb {
                    if *size_mb > max {
                        anyhow::bail!(
                            "Payload size {} MB exceeds safety limit of {} MB. \
                            Adjust your config or increase safety_limits.max_payload_size_mb",
                            size_mb,
                            max
                        );
                    }
                }
            }
            StressPattern::Slowloris { connections, .. }
            | StressPattern::SlowPost { connections, .. }
            | StressPattern::SlowRead { connections, .. } => {
                if let Some(max) = limits.max_concurrent_connections {
                    if *connections > max {
                        anyhow::bail!(
                            "Concurrent connections {} exceeds safety limit of {}. \
                            Adjust your config or increase safety_limits.max_concurrent_connections",
                            connections, max
                        );
                    }
                }
            }
            StressPattern::PipelineAbuse {
                concurrent_connections,
                ..
            } => {
                if let Some(max) = limits.max_concurrent_connections {
                    if *concurrent_connections > max {
                        anyhow::bail!(
                            "Concurrent connections {} exceeds safety limit of {}. \
                            Adjust your config or increase safety_limits.max_concurrent_connections",
                            concurrent_connections, max
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for StressPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.describe())
    }
}
```

**Before/after sketches (key duplication sites for validate/describe/print-description):**

*Before (src/run.rs:319 inside print_startup_info — the nested pattern match):*
```rust
if mode != ExecutionMode::StressTest {
    match &config.pattern {
        TrafficPattern::Fixed { concurrent, duration_secs, total_requests } => {
            println!("Pattern:               Fixed Concurrency");
            println!("Concurrent Clients:    {concurrent}");
            if let Some(duration) = duration_secs { println!("Duration:              {duration}s"); }
            if let Some(total) = total_requests { println!("Total Requests:        {total}"); }
        }
        // RateLimit, Ramp, Burst arms (each ~8-10 lines with exact label strings)
        ...
    }
}
...
// stress
if let Some(ref pattern) = config.stress_pattern {
    println!("Stress Pattern:        {pattern:?}");
}
```

*After (in print_startup_info):*
```rust
if mode != ExecutionMode::StressTest {
    println!("{}", config.pattern.describe());  // reproduces the multi-line block exactly
}
// stress (improves from Debug to human, centralized)
if let Some(ref pattern) = config.stress_pattern {
    println!("Stress Pattern:        {}", pattern.describe());
}
```

*Before (src/authorization.rs:200):*
```rust
fn pattern_description(pattern: &StressPattern) -> String { match ... { exact formats } }
...
println!("Pattern: {}", pattern_description(pattern));
```

*After:*
```rust
println!("Pattern: {}", pattern.describe());
// delete pattern_description entirely
```

*Before/after for Config (validate_traffic_pattern at 438 and validate_safety_limits at 566 become thin or inlined):*
```rust
// before
fn validate_traffic_pattern(&self) -> Result<()> { match &self.pattern { ... } }
fn validate_safety_limits(&self) -> Result<()> { match ... }

// after (in load)
self.pattern.validate()?;
if let Some(p) = &self.stress_pattern {
    p.validate_against(&self.safety_limits)?;
}
// old private fns can delegate or be removed
```

Before/after startup output example (traffic byte-identical; stress "Stress Pattern" line improves from Debug repr to human):

```
... (mode lines)
Timeout:               30s
Pattern:               Fixed Concurrency
Concurrent Clients:    50
Duration:              60s

================================================================================
```
(identical for traffic; for stress the line becomes "Stress Pattern:        Connection Flood - 100 conn/s, hold 5000ms, duration 60s" instead of the Debug form.)

**Target ID helper (src/config.rs, add to `impl TargetConfig` after line 156):**

```rust
impl TargetConfig {
    /// Returns the configured id or a generated default.
    /// - index=None  => "target"   (single-target discovery, perform/apply)
    /// - index=Some(i) => "target-{i}" (multi-target paths)
    pub fn effective_id(&self, index: Option<usize>) -> String {
        if !self.id.is_empty() {
            self.id.clone()
        } else {
            match index {
                Some(i) => format!("target-{}", i),
                None => "target".to_string(),
            }
        }
    }
}
```

**Before/after for execute_multi_target mutation site (run.rs ~99):**
```rust
// before
.map(|(i, t)| {
    let mut target = t.clone();
    if target.id.is_empty() {
        target.id = format!("target-{i}");
    }
    Arc::new(target)
})

// after
.map(|(i, t)| {
    let mut target = t.clone();
    target.id = target.effective_id(Some(i));
    Arc::new(target)
})
```

**Before/after example for one discovery apply site (run.rs ~489 inside apply_discovery_results):**
```rust
// before
let id = if target.id.is_empty() {
    format!("target-{i}")
} else {
    target.id.clone()
};

// after
let id = target.effective_id(Some(i));
```

Replace the other 4 sites (print_startup_info, perform_discovery x2, apply x1) similarly with `effective_id(...)`. No behavior change.

**Drain helper consolidation:**

Keep definition in `src/patterns.rs:289`, change to `pub(crate) async fn drain_join_set(...)`.

In `src/stress.rs`: remove the duplicate fn, `use crate::patterns::drain_join_set;`. All call sites (2 in patterns + 7 in stress) unchanged.

**Duplication removed vs. remaining (after this refactor):**
- Removed: validate matches (config), description matches (auth + print knowledge for labels/fields), ID ifs (6 sites), drain fn def.
- Remaining (intentional): execute dispatch matches (patterns:47, stress:28), runtime bails inside private execute_* (e.g. patterns rate==0 at 150, ramp check at 200; stress various), manual cancel/elapsed checks in loops, client parse dupe (secondary).

### 2. Reporter & Result Emission Unification

In `src/run.rs` (adjust to preserve exact order):
```rust
// in execute_stress_test after stats =
let snapshot = metrics.get_snapshot();
let stats = Statistics::from_snapshot(&snapshot);

let reporter = Reporter::new(false);
if config.output.console {
    reporter.show_final_summary(&stats);
    print_connection_stats(&snapshot.connection_stats);
}
if let Some(output_path) = &config.output.file {
    reporter.export_json(&stats, output_path)?;
}
```

This reduces `Reporter::new(false)` constructions inside stress from 2 to 1, keeps `emit_results` as the canonical point for single/multi normal paths (which handles the show vs show_multi decision), and preserves the *exact* relative output order for console+file stress cases: FINAL RESULTS / stats, then CONNECTION STATISTICS, then "Results exported to: ...".

(emit_results itself is left unchanged; its internal creations are for the normal paths.)

Realtime stays single-target only.

### 3. Orchestration Improvements in run.rs

High-level before/after same as prior (discovery uses effective_id; print_startup uses describe for the pattern block(s) + reduced arms for stress; small helpers extracted; stress reuses reporter emission path; three execute_* kept parallel).

Keep the three `execute_*` functions.

No change to `ExecutionMode` enum (`config.rs:343` inside Config).

**Mermaid: Current vs Proposed Data Flow for Pattern Behavior (validate/describe)**

```mermaid
flowchart TD
    subgraph Current["Current (duplicated shape knowledge)"]
        E[TrafficPattern/StressPattern<br/>data only] --> M1[validate/desc in config.rs:438/566]
        E --> M2[pattern_description in auth.rs:200]
        E --> M3[print match in run.rs:319]
    end
    subgraph Proposed["Proposed (centralized on types)"]
        E2[...] --> I[impl validate + describe + Display<br/>in config.rs (full arms)]
        I --> V1[Config load delegates]
        I --> V2[auth warning]
        I --> V3[print_startup_info uses describe]
        D[exec dispatch (remains) ] --> P[...]
    end
```

### 4. Secondary Cleanups (Fits Small PRs)

- In `client.rs`, note (do not block) the parse_host_port dupe in stress methods.
- Address obvious `#[allow(dead_code)]` opportunistically.
- Optional later: `should_continue` helper for cancel+elapsed.

---

## API / Interface Changes

**No public API changes.**

Internal only:
- New inherent methods on `TrafficPattern` / `StressPattern`.
- `TargetConfig::effective_id`.
- `patterns::drain_join_set` pub(crate).
- `pattern_description` deleted from `authorization.rs`.
- `Config::validate_*` become thin delegates or direct calls to the methods.
- Stress reporter path adjusted for order (single Reporter instance).

`Config::get_execution_mode`, `PatternExecutor::new*`, etc. signatures unchanged.

Before/after example: traffic startup output byte-identical via describe(); stress "Stress Pattern:" line uses human describe() (improvement vs prior Debug).

---

## Data Model Changes

**None.** Enums, `Config`, `TargetConfig`, `SafetyLimits` structs are unchanged in layout/serialization.

---

## Alternatives Considered

1. **Trait objects / behavior trait** (as before): Rejected for boring/direct + zero cost + closed enum.

2. **Central control loop + command pattern**: Rejected (complexity, fidelity risk).

3. **Always normalize to multi-target + MultiTargetMetrics**: Larger change; consider post this work. Current design keeps behavior identical for single path.

4. **Macro to generate matches**: Rejected (obscures, not boring).

5. **Move execution/print logic fully into enum impls (e.g. `fn emit_startup(&self)` doing printlns)**: Considered; rejected for side-effect in data module (Config owns data per ARCHITECTURE). Using describe() for the block + println in run is boring compromise.

---

## Security & Privacy Considerations

- No change to authorization flow. `prepare_stress_run` (authorization.rs:104) + description strings identical (moved verbatim).
- Safety limit checks identical after move to `StressPattern::validate_against`.
- No new data exposure.

---

## Observability

- Existing tracing + println unchanged in volume/semantics for traffic; stress startup "Stress Pattern" line now uses human describe (improvement).
- New methods: can add `tracing::debug!` inside validate/describe if desired (low volume).
- No new metrics.
- Verification requires byte-identical traffic startup + exact stress warning strings + describe unit tests.

---

## Rollout Plan

Pure refactor — no feature flags.

- **Staged by PRs** (see below): each small, independently reviewable+mergeable with green tests.
- After every PR: `cargo fmt -- --check && cargo clippy -- -D warnings && cargo test && cargo test --doc` (full matrix).
- Manual verification: run example configs (single, multi, all stress-*.yaml with console + --output file); check relative output order for stress (FINAL / conn / exported); run discovery configs.
- Strengthen: unit tests for new methods assert exact describe strings + validate errors + effective_id cases.
- Rollback: `git revert` (small PRs).
- Post all: optional docs PR to update ARCHITECTURE.md "Extensibility" + test counts.

---

## Open Questions

- (Resolved in Key Decisions) Exact describe strings and usage in print_startup_info.
- Is pub(crate) drain acceptable long-term? (Yes for now.)
- Follow-on always-multi?
- Public re-export of describe? (Non-goal now.)

---

## Key Decisions

1. **Inherent methods on enums, not traits or free functions.** (As before.)
2. **describe() + Display + fidelity for print.** Resolved: `TrafficPattern::describe()` returns a multi-line String using verbatim text copied from the original print_startup_info println blocks (see full impl). Replacement `println!("{}", config.pattern.describe());` inside the if-non-stress block yields **byte-identical** startup output for traffic. `StressPattern::describe()` returns the exact compact single-line strings copied from old `pattern_description` (used in auth + now for the "Stress Pattern: {}" line in print_startup_info, replacing Debug -- a minor improvement to human-readable). This resolves the prior open question on "printed labels vs compact".
3. **effective_id with Option<usize>**. (As before; now documented with 6 sites.)
4. **drain_join_set lives in patterns.rs as pub(crate)**. (As before.)
5. **Unify reporter while preserving order.** Use single Reporter instance in stress with explicit if-console (show + conn) then if-file export. Preserves exact order. emit_results stays canonical for normal paths.
6. **Dispatch matches + runtime checks for execute() remain.** (As before; see "duplication removed vs remaining".)
7. **No changes to lib.rs exports or ExecutionMode.** (As before.)
8. **Test-driven verification at each step.** New methods get direct tests (sketched below) in `tests/config_integration_test.rs`; full `cargo test && cargo test --doc` + clippy + manual per PR. Byte-identical traffic startup + exact describe strings asserted.
9. **Keep client stress parsing duplication for now.** (As before.)
10. **Quantified success.** Before: multiple external matches for validate/describe + 6 ID sites. After: 1 impl for validate/describe (dispatch remains); ID logic in 1 method. "Deletes duplication" tightened to the validate/describe/print-description concerns.

**Defensive validation decision:** Runtime bails inside `PatternExecutor`/`StressExecutor` private methods (and pattern_from_args early rate check) will be kept for defense-in-depth + cases where patterns are constructed outside full Config::load (tests, benches). Primary check remains at load time via the new methods. No removal planned.

---

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Moved validate/describe produces different error strings or output | High | Copy exact `bail!` messages and format strings verbatim from original sites (see full impls). Add tests asserting on `pattern.validate()` errors and exact `pattern.describe()` strings (see sketches). |
| Discovery result application uses wrong IDs after effective_id change | High | Unit-test `effective_id(None) == "target"`, `effective_id(Some(2)) == "target-2"` + non-empty case; run full `discovery_integration_test.rs` (20 tests) + manual multi-port configs. |
| Stress execution fidelity (timing, conn counts) drifts | High | Keep private execute_* untouched. Full `cargo test && cargo test --doc` + stress example runs post each PR. |
| Tests only exercised construction, not new methods | Medium | Add small dedicated unit tests for `TrafficPattern::describe()` / `StressPattern::describe()` + regression test for default target ID assignment (see sketches below). |
| Reporter unification changes stress output order or presence of conn stats | Medium | Design uses explicit single-reporter + console(show+conn)+file to keep original relative order exactly. Verify with "stress config + --output file + console": check 'FINAL RESULTS', 'CONNECTION STATISTICS', then 'Results exported to:' line positions. |
| Adding future pattern still requires executor arm | Low | Document; acceptable (dispatch is execution concern). |
| clippy/fmt breakage on refactors | Low | Mandate `cargo fmt && cargo clippy -- -D warnings` + full test matrix in every PR. |
| describe() for Traffic returns multi-line (affects any non-startup print of it) | Low | Only used via println in startup (intended); other uses (logs) will show structured which is acceptable or can use a future compact variant. |
| pub(crate) drain_join_set if patterns module refactored later | Low | Internal; only stress imports it. If needed later, move to a shared internal module. |
| Doc-test / full matrix count variance vs "141" | Low | Use "full suite (~140 + --doc)" in claims; explicitly run `cargo test --doc` in checklist. |
| ARCHITECTURE.md test counts + extensibility section outdated | Low | Post-series optional docs-only PR (as planned). |

**Test sketches (small dedicated unit tests + regression test; add in tests/config_integration_test.rs for PR1/PR2/PR3):**

```rust
// PR1: small dedicated unit test for TrafficPattern::describe() + validate
#[test]
fn test_traffic_pattern_describe() {
    let fixed = TrafficPattern::Fixed { concurrent: 50, duration_secs: Some(60), total_requests: None };
    let d = fixed.describe();
    assert!(d.contains("Pattern:               Fixed Concurrency"));
    assert!(d.contains("Concurrent Clients:    50"));
    assert!(d.contains("Duration:              60s"));
}

#[test]
fn test_traffic_pattern_validate() {
    let bad_rate = TrafficPattern::RateLimit { rate: 0, duration_secs: None, total_requests: None };
    assert!(bad_rate.validate().is_err());
    // ... other cases
}

// PR2: small dedicated unit test for StressPattern::describe()
#[test]
fn test_stress_pattern_describe_exact() {
    let flood = StressPattern::ConnectionFlood { connections_per_second: 100, hold_time_ms: 5000, duration_secs: 60 };
    assert_eq!(flood.describe(), "Connection Flood - 100 conn/s, hold 5000ms, duration 60s");
    // exact assertions for all 7 variants (copied from former pattern_description)
}

// PR3: regression test for default target ID assignment (via effective_id + usage)
#[test]
fn test_target_id_default_assignment_regression() {
    let t = TargetConfig { id: String::new(), ..Default::default() };
    assert_eq!(t.effective_id(None), "target");
    assert_eq!(t.effective_id(Some(3)), "target-3");
    let named = TargetConfig { id: "api1".to_string(), ..Default::default() };
    assert_eq!(named.effective_id(Some(0)), "api1");
    // (integration coverage for the execute_multi_target mutation site is exercised by target_selector + multi-target integration tests)
}
```

---

## References

- Source files (verified via read/grep 2026-07-03; lines approximate, use fn names + context for stability):
  - `src/config.rs:160` (TrafficPattern enum), `226` (StressPattern), `343` (ExecutionMode), `438` (validate_traffic_pattern fn inside impl), `508` (rate check in pattern_from_args), `566` (validate_safety_limits inside impl), `534` (get_execution_mode).
  - `src/patterns.rs:15` (PatternExecutor), `47` (execute dispatch), `289` (drain).
  - `src/stress.rs:13` (StressExecutor), `28` (execute dispatch), `375` (drain).
  - `src/run.rs:24` (run), `63/89/142` (execute_* fns), `180` (emit_results), `279` (print_startup_info), `319` (inner pattern match inside print_startup_info), `384` (should_perform_discovery), `402/450` (perform_discovery / apply_discovery_results fns). ID logic at execute_multi_target:101, print_startup_info:298, perform_discovery:412/426, apply:461/489 (all 6).
  - `src/authorization.rs:104` (prepare_stress_run), `200` (pattern_description).
  - `src/client.rs:40` (HttpClient), `318` (parse_host_port).
  - `src/lib.rs:4-22` (module exports).
  - `src/metrics.rs:40` (categorize_and_increment), allows at 9/115/141.
- `ARCHITECTURE.md` (full module breakdown, "Extensibility" section...).
- Test files: `tests/config_integration_test.rs`, `tests/pattern_execution_test.rs`, `tests/discovery_integration_test.rs` (20 tests), etc.
- Example configs: `config.example.yaml`, `config.multi-target.example.yaml`, `config.stress-*.example.yaml`.
- Review findings (in /tmp/grok-design-review-e8fef800.md): 7 issues (counts, describe specificity + print fidelity, reporter order, PR detail, duplication claims, risks/tests, cites).
- Related: `CONTRIBUTING.md` (refactor/ branches, cargo test + clippy + fmt requirements).

---

## Incremental PR Plan

Order ensures each PR is independently reviewable + mergeable, builds safely on prior (data first, then helpers, then call sites, then orchestration). PR1+PR2 touch same file sequentially. Each keeps tests green. Target: 7 PRs.

**PR 1: "refactor: add validate + describe to TrafficPattern"**  
Affected: `src/config.rs`, `tests/config_integration_test.rs` (add tests).  
Dependencies: none.  
Description: Add `impl TrafficPattern { pub fn validate(&self) -> Result<()>; pub fn describe(&self) -> String; }` + `Display` with full 4 arms using verbatim startup strings. Update `Config::validate_traffic_pattern` (delegate) and `pattern_from_args` (construct then `.validate()?`). Add a small dedicated unit test for `TrafficPattern::describe()` (plus validate coverage for all 4 variants with exact string assertions on describe output). No other files touched. Verifies new methods in isolation. Removes first duplicated validate/describe shape knowledge.  
**Implementation sketch (key signatures + sites):**  
```rust
// config.rs (new impl after Default)
impl TrafficPattern { pub fn validate(&self) -> Result<()> { ... } pub fn describe(&self) -> String { ... full arms ... } }
// pattern_from_args (rate arm ~507):
if let Some(rate) = args.rate {
    let pattern = TrafficPattern::RateLimit { rate, duration_secs: args.duration, total_requests: args.requests };
    pattern.validate()?;
    return Ok(Some(pattern));
}
// similar for ramp construction (add validate); fixed no early check needed
// Config::validate_traffic_pattern:
fn validate_traffic_pattern(&self) -> Result<()> { self.pattern.validate() }
```
LOC impact ~ +80 (impl + tests) / -10 (old match body).

**PR 2: "refactor: add describe + validate_against to StressPattern"**  
Affected: `src/config.rs`, `tests/config_integration_test.rs`.  
Dependencies: PR 1.  
Description: Add equivalent `impl StressPattern` with full 7-arm describe (verbatim from old pattern_description) + validate_against (verbatim from safety). Update call in Config::validate_stress_authorization / safety path. Add a small dedicated unit test for `StressPattern::describe()` (plus validate_against + safety cases, with exact string assertions for all 7 variants).  
**Implementation sketch:**  
```rust
// config.rs (new impl)
impl StressPattern { pub fn describe(&self) -> String { match ... { verbatim 7 formats } } pub fn validate_against(...) { match ... { verbatim safety bails } } }
// in Config (after load auth):
if let Some(p) = &self.stress_pattern { p.validate_against(&self.safety_limits)?; }
```
(Old validate_safety_limits can delegate or be inlined.)

**PR 3: "refactor: centralize target ID defaults via TargetConfig::effective_id"**  
Affected: `src/config.rs` (add method), `src/run.rs` (replace 6 sites).  
Dependencies: none (parallelizable with early PRs for cleanliness).  
Description: Implement `effective_id`. Replace the 6 if-empty sites (with explicit before/after for mut site and discovery sites). Add a dedicated regression test for default target ID assignment (covering `effective_id(None)`, `effective_id(Some(i))`, named IDs, and the mutation site behavior in multi-target assignment). Behavior of generated IDs identical.  
**Implementation sketch (key changed call sites):**  
```rust
// config.rs
impl TargetConfig {
    pub fn effective_id(&self, index: Option<usize>) -> String { if !self.id.is_empty() { self.id.clone() } else { match index { Some(i)=>format!("target-{}",i), None=>"target".to_string() } } }
}
// run.rs execute_multi_target (~95-106):
let targets: Vec<Arc<TargetConfig>> = ... .map(|(i, t)| {
    let mut target = t.clone();
    target.id = target.effective_id(Some(i));  // was the if-empty + format
    Arc::new(target)
}).collect();
// print_startup_info (~297): let id = target.effective_id(Some(i));
// perform_discovery (~412): ...effective_id(None) ... ; (~426) effective_id(Some(i))
// apply_discovery_results (~461): effective_id(None); (~489): effective_id(Some(i))
```
Update PR desc/quant: "replace 6 sites".

**PR 4: "refactor: consolidate drain_join_set into single canonical definition"**  
Affected: `src/patterns.rs` (pub(crate)), `src/stress.rs` (delete dupe + use).  
Dependencies: none.  
Description: (As before; unchanged.)  
**Implementation sketch:** pub(crate) on patterns fn + `use crate::patterns::drain_join_set;` in stress; delete duplicate fn body.

**PR 5: "refactor: move StressPattern description to impl; delete pattern_description"**  
Affected: `src/authorization.rs`, `src/config.rs`.  
Dependencies: PR 2.  
Description: Replace calls with `.describe()`. Delete the 70-line match fn. Update docs. Verifies stress warning output identical (exact strings).  
**Implementation sketch:**  
`println!("Pattern: {}", pattern.describe());` (in display_stress_warning); delete fn pattern_description entirely.

**PR 6: "refactor: unify result emission and reduce Reporter duplication in run"**  
Affected: `src/run.rs` (execute_stress_test, emit_results callers).  
Dependencies: PR 3, PR 4.  
Description: Refactor stress to use single Reporter + explicit console (show+conn) then file export (preserves exact order: summary, conn stats, export msg). Reduces Reporter news in stress from 2 to 1. emit_results remains canonical for normal single/multi.  
**Implementation sketch:**  
```rust
// execute_stress_test (after stats=)
let reporter = Reporter::new(false);
if config.output.console {
    reporter.show_final_summary(&stats);
    print_connection_stats(&snapshot.connection_stats);
}
if let Some(output_path) = &config.output.file {
    reporter.export_json(&stats, output_path)?;
}
```
Verification note: "run stress config with console + output file; assert relative line positions of FINAL, CONNECTION STATISTICS, 'Results exported'."

**PR 7: "refactor: simplify print_startup_info and reduce orchestration branches using new methods"**  
Affected: `src/run.rs` (primarily print_startup_info:279 and the execute_* fns).  
Dependencies: PR 1, PR 2, PR 3, PR 5.  
Description: Replace the nested traffic pattern match (and stress Debug) with `println!("{}", config.pattern.describe());` (and `pattern.describe()` for stress). Use `effective_id` (prior). Extract 1-2 small helpers e.g. `fn assign_target_ids(targets: &[TargetConfig]) -> Vec<Arc<...>>` (using effective_id) and/or `fn prepare_common(...)`. Keep three execute_* but make prepare/execute/report structure visually parallel via shared emit path. Full startup output + behavior preserved (traffic identical; stress improved).  
**Implementation sketch (print part + example helper):**  
```rust
// inside print_startup_info (replacing ~318-378):
if mode != ExecutionMode::StressTest {
    println!("{}", config.pattern.describe());
}
if mode == ExecutionMode::StressTest {
    if let Some(ref p) = config.stress_pattern {
        println!("Stress Pattern:        {}", p.describe());
    }
}
// example extracted helper (for multi ids, called from execute_multi + discovery prep if wanted):
fn assign_target_ids(raw: &[TargetConfig]) -> Vec<Arc<TargetConfig>> {
    raw.iter().enumerate().map(|(i,t)| {
        let mut tt = t.clone(); tt.id = tt.effective_id(Some(i)); Arc::new(tt)
    }).collect()
}
```
Update orchestration comments for parallelism.

**Optional PR 8 ...** (as before; add --doc to matrix).

**Overall sequencing & verification notes:**
- PRs 1-4 provide early high-impact wins (sequence PR1 then PR2 for config.rs; PR3/4 parallelizable).
- PRs 5-7 depend on prior for clean compile/tests.
- After each: full matrix `cargo fmt -- --check && cargo clippy -- -D warnings && cargo test && cargo test --doc`; manual single + multi + stress (with --output) + discovery.
- No PR changes public signatures or enum variants.
- Strengthen: after each PR require traffic startup stdout byte-identical + the new dedicated unit tests for `TrafficPattern::describe()` / `StressPattern::describe()` (exact strings) + the regression test for default target ID assignment (PR3) + stress conn/export order check.
- Post all: optional docs PR for ARCHITECTURE.md.
- Hot-path optimizations (metrics / patterns / stress loops) are deliberately left for a separate follow-up refactor.
- Total expected diff: focused, reviewable. Cumulative fulfills criteria (no regression, targeted deletion of validate/describe dupes, cheaper addition of patterns, boring code).

---

*End of design document.*
