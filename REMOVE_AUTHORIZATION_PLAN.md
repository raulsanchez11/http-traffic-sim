# Plan: Remove Authorization Requirement

**Date:** 2026-07-03  
**Goal:** Remove the authorization gate for stress testing patterns.  
**Rationale:** Authorization is generally a good practice, but we will operate under the assumption that the user running the tool is authorized to perform the tests. This simplifies usage and removes mandatory configuration friction.

## Current State (before change)

- Stress patterns (`stress_pattern`) **require** an `authorization` section with `confirmed: true`.
- `Config::load()` calls `validate_stress_authorization()` which fails without proper auth.
- `run()` calls `authorization::prepare_stress_run()` which:
  - Re-validates auth
  - Displays a large legal warning
  - Shows pattern details + safety limits
  - Runs a 5-second countdown
- `AuthorizationConfig` lives in `config.rs`
- The `authorization` module is public and exported
- All stress example configs include the `authorization` block
- Documentation repeatedly states "Mandatory authorization for stress tests"

## Goals of Removal

- Stress tests can be executed by simply providing a `stress_pattern` (no `authorization` key needed).
- Remove `AuthorizationConfig` struct and all related fields.
- Remove the `src/authorization.rs` module (or keep as no-op if backward compat is desired — recommended to fully remove).
- Keep `safety_limits` (they remain useful and user-configurable).
- Preserve the ability to configure and enforce safety limits for stress patterns.
- Update all examples, docs, tests, and architecture references.
- No behavioral change for non-stress usage.

## Non-Goals

- Do not remove `safety_limits` support.
- Do not add any new "assumed authorized" logging (keep it silent per the "assumption" directive).
- Do not keep the warning/countdown (they are part of the auth gate).

## High-Level Changes

1. **Configuration**
   - Remove `authorization: Option<AuthorizationConfig>` from `ConfigFile` and `Config`.
   - Remove the `AuthorizationConfig` struct.
   - Remove `validate_stress_authorization()`.
   - In stress path, only call safety limit validation (if limits are configured).

2. **Runtime**
   - Remove the `if let Some(ref stress_pattern) ... authorization::prepare_stress_run(...)` block in `run()`.
   - Stress execution will proceed directly.

3. **Module**
   - Delete `src/authorization.rs`.
   - Remove `pub mod authorization;` from `src/lib.rs`.

4. **Examples**
   - Remove the `authorization:` block from all `config.stress-*.example.yaml` files.
   - Optionally keep `safety_limits:` examples.

5. **Documentation**
   - Update `README.md`, `docs/DOCUMENTATION.md`, `ARCHITECTURE.md`.
   - Remove claims of "mandatory authorization".
   - Update stress testing sections to show minimal required config.
   - Remove or deprecate references to `AuthorizationConfig` and `prepare_stress_run`.

6. **Tests**
   - Remove `test_authorization_config` (or repurpose).
   - Add/ensure tests that stress patterns work without `authorization` in config.
   - Update any integration tests that assert on auth errors.

7. **Other**
   - Clean up any mentions in `CONTRIBUTING.md`, example yamls in docs, etc.

## Recommended Incremental Implementation (PR-style)

### PR 1: Remove authorization from configuration model
- Delete `AuthorizationConfig` struct.
- Remove `authorization` field from `ConfigFile` and `Config`.
- Remove `validate_stress_authorization()`.
- Update `Config::load()` to still call safety validation for stress patterns directly:
  ```rust
  if let Some(_) = &self.stress_pattern {
      self.validate_safety_limits()?;
  }
  ```
- Update `Config` construction in `load()`.

**Files:** `src/config.rs`

### PR 2: Remove runtime authorization call
- Remove the `authorization::prepare_stress_run(...)` block in `run()`.
- Remove the `use crate::authorization;` import if unused.
- Stress execution path now goes straight to `StressExecutor`.

**Files:** `src/run.rs`

### PR 3: Remove the authorization module
- Delete `src/authorization.rs`.
- Remove `pub mod authorization;` from `src/lib.rs`.
- Remove any re-exports.

**Files:** `src/lib.rs`

### PR 4: Update example configuration files
- Remove `authorization:` sections from:
  - `config.stress-flood.example.yaml`
  - `config.stress-requestflood.example.yaml`
  - `config.stress-slowloris.example.yaml`
  - `config.stress-largepayload.example.yaml`
- Keep or add `safety_limits:` examples where appropriate.

**Files:** `config.stress-*.example.yaml`

### PR 5: Update documentation
- `README.md`: Remove "Mandatory authorization" bullet and example sections.
- `docs/DOCUMENTATION.md`: 
  - Update stress testing section.
  - Remove `AuthorizationConfig` from library API examples.
  - Update "Related Guides" table.
- `ARCHITECTURE.md`: Remove authorization module description and update flow diagrams/text.
- Any other mentions (e.g. in `CONTRIBUTING.md` examples).

### PR 6: Update tests
- Remove or update `test_authorization_config` in `tests/config_integration_test.rs`.
- Add test case that loads a stress config **without** `authorization` and succeeds.
- Ensure existing stress tests still pass.

**Files:** `tests/config_integration_test.rs`

### PR 7: Final cleanup & verification
- Run full test suite.
- Run `cargo clippy -- -D warnings`.
- Manually test at least one stress pattern (e.g. `config.stress-requestflood.example.yaml`).
- Update any remaining references in docs or comments.
- Consider adding a note in `docs/DOCUMENTATION.md` under stress testing:
  > "The tool assumes the operator has proper authorization to run stress tests against the target."

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Users accidentally run destructive stress tests | Medium | Keep `safety_limits` support. Document assumption clearly. |
| Breaking change for existing configs | Low | Authorization section will simply be ignored (or we can warn once on unknown field, but simplest is to drop support). |
| Loss of the educational warning | Low | The warning was the main value of the auth module. We accept this per the request. |
| Safety limits no longer enforced | Low | Move safety validation to be unconditional for stress patterns (see PR 1). |

## Verification Steps

1. `cargo test`
2. `cargo clippy -- -D warnings`
3. Load a minimal stress config without `authorization:` and run it (should succeed).
4. Verify `safety_limits` still work when provided.
5. Check that `authorization:` key in a config is no longer required (and ideally produces a clear error or is ignored cleanly).
6. Review generated docs after `cargo doc`.

## Open Questions

- Should we keep the `authorization` field as a no-op for backward compatibility (with a deprecation warning)?
- Do we still want to display any kind of "you are running a stress test" message (lighter version)?
- Should `safety_limits` move under `stress_pattern` in the config schema for clarity in the future? (Out of scope for this removal.)

## Post-Removal State

A minimal stress config will look like:

```yaml
target:
  url: "https://example.com"

stress_pattern:
  category: requestflood
  target_rps: 10000
  duration_secs: 30

# safety_limits are still supported and recommended
safety_limits:
  max_requests_per_second: 5000
```

This aligns with the goal of assuming authorization while retaining useful guardrails (safety limits).

