# Contributing to http-traffic-sim

Thank you for considering contributing to http-traffic-sim! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)

---

## Code of Conduct

### Our Standards

- **Be respectful**: Treat all contributors with respect
- **Be constructive**: Provide helpful feedback
- **Be collaborative**: Work together toward common goals
- **Be patient**: Remember that everyone is learning

### Unacceptable Behavior

- Harassment or discrimination
- Trolling or insulting comments
- Publishing others' private information
- Other conduct inappropriate in a professional setting

---

## Getting Started

### Prerequisites

- **Rust**: 1.70 or later
- **Cargo**: Included with Rust
- **Git**: For version control

```bash
# Check versions
rustc --version
cargo --version
git --version
```

### Fork and Clone

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/http-traffic-sim.git
   cd http-traffic-sim
   ```

3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/anthropics/http-traffic-sim.git
   ```

### Build and Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with example
cargo run -- --url https://httpbin.org/get --concurrent 10 --duration 10
```

---

## Development Workflow

### 1. Create a Branch

```bash
# Update main branch
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/your-feature-name
```

**Branch Naming**:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `test/` - Test additions/improvements
- `refactor/` - Code refactoring

### 2. Make Changes

- Write clean, well-documented code
- Follow existing code style
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with verbose output
cargo test -- --nocapture

# Run benchmarks (if applicable)
cargo bench

# Check code formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

### 4. Commit Your Changes

```bash
# Stage changes
git add path/to/changed/files

# Commit with descriptive message
git commit -m "Add feature: description of what you did"
```

**Commit Message Guidelines**:
- Use present tense ("Add feature" not "Added feature")
- Use imperative mood ("Move cursor to..." not "Moves cursor to...")
- First line should be ≤72 characters
- Reference issues/PRs where appropriate

**Examples**:
```
Add weighted load distribution strategy

Implement weighted target selection for multi-target load testing.
Distribution is based on configurable weights per target.

Fixes #123
```

### 5. Push and Create Pull Request

```bash
# Push to your fork
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

---

## Code Style

### Rust Conventions

Follow standard Rust conventions:

1. **Formatting**: Use `rustfmt`
   ```bash
   cargo fmt
   ```

2. **Linting**: Pass `clippy` checks
   ```bash
   cargo clippy -- -D warnings
   ```

3. **Naming**:
   - `snake_case` for functions, variables, modules
   - `PascalCase` for types, traits, enums
   - `SCREAMING_SNAKE_CASE` for constants

### Code Organization

```rust
// 1. Module documentation
//! Module description
//!
//! Detailed explanation...

// 2. Imports (grouped)
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::Config;

// 3. Constants
const DEFAULT_TIMEOUT: u64 = 30;

// 4. Types
pub struct MyStruct {
    field: String,
}

// 5. Implementations
impl MyStruct {
    pub fn new() -> Self {
        Self {
            field: String::new(),
        }
    }
}

// 6. Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let s = MyStruct::new();
        assert_eq!(s.field, "");
    }
}
```

### Documentation Comments

Use rustdoc comments for all public items:

```rust
/// Brief description of the function.
///
/// More detailed explanation of what the function does,
/// how it works, and any important notes.
///
/// # Arguments
///
/// * `param1` - Description of param1
/// * `param2` - Description of param2
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Describes when this function returns an error
///
/// # Examples
///
/// ```
/// use http_traffic_sim::module::function;
///
/// let result = function(arg1, arg2);
/// ```
pub fn function(param1: Type1, param2: Type2) -> Result<ReturnType> {
    // Implementation
}
```

### Error Handling

1. **Use `Result<T, E>`** for fallible operations
2. **Use `anyhow::Result`** for application errors
3. **Provide context** with `.context()`:
   ```rust
   use anyhow::Context;

   std::fs::read_to_string(path)
       .context(format!("Failed to read file: {}", path.display()))?
   ```

4. **Don't panic** in library code (except for truly unrecoverable errors)

### Performance Considerations

1. **Avoid allocations** in hot paths
2. **Use `Arc` and `Clone`** instead of deep copying
3. **Pre-allocate** collections when size is known:
   ```rust
   let mut vec = Vec::with_capacity(expected_size);
   ```

4. **Profile before optimizing**:
   ```bash
   cargo bench
   cargo flamegraph -- [your command]
   ```

---

## Testing

### Test Organization

```
src/
  module.rs       # Contains unit tests in #[cfg(test)] mod tests
tests/
  integration_test.rs  # Integration tests
benches/
  benchmark.rs    # Performance benchmarks
```

### Unit Tests

Located in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = create_test_input();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### Integration Tests

Located in `tests/` directory:

```rust
// tests/feature_test.rs
use http_traffic_sim::module::*;

#[test]
fn test_feature_integration() {
    // Test cross-module functionality
}
```

### Test Guidelines

1. **Test edge cases**:
   - Empty inputs
   - Maximum/minimum values
   - Invalid inputs

2. **Test error conditions**:
   ```rust
   #[test]
   fn test_error_handling() {
       let result = function_that_fails(invalid_input);
       assert!(result.is_err());
   }
   ```

3. **Use descriptive names**:
   ```rust
   #[test]
   fn test_round_robin_cycles_through_all_targets() {
       // Test name clearly describes what is being tested
   }
   ```

4. **Keep tests focused**:
   - One test per behavior
   - Tests should be independent

5. **Avoid flaky tests**:
   - Don't rely on timing
   - Don't rely on external services
   - Use mocks/stubs where appropriate

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test integration

# Run specific test
cargo test test_round_robin

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored

# Run tests in release mode (faster)
cargo test --release
```

---

## Documentation

### Types of Documentation

1. **Code Documentation** (`///` comments)
   - Public functions, structs, enums
   - Module-level documentation (`//!`)

2. **User Documentation** (Markdown files)
   - README.md - Getting started
   - ARCHITECTURE.md - System design
   - TROUBLESHOOTING.md - Common issues

3. **Examples**
   - Config file examples
   - Code examples in documentation

### Writing Good Documentation

**Do**:
- Explain *why*, not just *what*
- Provide examples
- Keep it up-to-date with code changes
- Use simple, clear language

**Don't**:
- Assume prior knowledge
- Use jargon without explanation
- Write obvious documentation
- Let documentation become stale

### Building Documentation

```bash
# Generate and view API docs
cargo doc --open

# Check for broken links
cargo doc --no-deps
```

---

## Submitting Changes

### Before Submitting

**Checklist**:
- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] New code has tests
- [ ] Code is formatted (`cargo fmt`)
- [ ] Code passes linter (`cargo clippy`)
- [ ] Documentation is updated
- [ ] Commit messages are clear

### Pull Request Process

1. **Update your branch**:
   ```bash
   git checkout main
   git pull upstream main
   git checkout your-feature-branch
   git rebase main
   ```

2. **Push to your fork**:
   ```bash
   git push origin your-feature-branch
   ```

3. **Create Pull Request** on GitHub:
   - Use descriptive title
   - Explain what and why
   - Reference related issues
   - Include test results

### Pull Request Template

```markdown
## Description

Brief description of the changes.

## Motivation

Why is this change needed? What problem does it solve?

## Changes

- List of specific changes made
- Bullet points for each major change

## Testing

How was this tested?

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)

## Related Issues

Closes #123
Related to #456
```

---

## Review Process

### What to Expect

1. **Initial Review** (1-3 days):
   - Maintainers review your PR
   - May request changes or ask questions

2. **Iteration**:
   - Address feedback
   - Push new commits
   - Discussion continues

3. **Approval**:
   - Once approved, PR is merged
   - Your contribution is now part of the project!

### Review Criteria

Reviewers check for:
- **Correctness**: Does it work as intended?
- **Tests**: Is functionality tested?
- **Code Quality**: Is code clean and maintainable?
- **Documentation**: Are changes documented?
- **Performance**: Any negative performance impact?
- **Security**: Any security concerns?

### Responding to Feedback

**Do**:
- Be open to suggestions
- Ask questions if unclear
- Explain your reasoning
- Make requested changes promptly

**Don't**:
- Take feedback personally
- Argue defensively
- Make changes without understanding why
- Let PR go stale

---

## Development Guidelines

### Adding New Features

1. **Check existing issues** first
2. **Discuss large changes** before implementing
3. **Follow existing patterns** in the codebase
4. **Add tests** for new functionality
5. **Update documentation**

### Example: Adding a New Traffic Pattern

```rust
// 1. Add to config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TrafficPattern {
    // ... existing patterns ...
    NewPattern {
        param1: usize,
        param2: u64,
    },
}

// 2. Implement in patterns.rs
impl PatternExecutor {
    async fn execute_new_pattern(
        &self,
        param1: usize,
        param2: u64,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        // Implementation
    }

    pub async fn execute(&self, cancel_token: CancellationToken) -> Result<()> {
        match &self.pattern {
            // ... existing patterns ...
            TrafficPattern::NewPattern { param1, param2 } => {
                self.execute_new_pattern(*param1, *param2, cancel_token).await
            }
        }
    }
}

// 3. Add tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_new_pattern() {
        // Test implementation
    }
}

// 4. Update documentation
/// New traffic pattern that does X.
///
/// This pattern is useful for Y scenarios.
```

### Performance Optimization

Before optimizing:
1. **Profile first**: Use benchmarks to identify bottlenecks
2. **Measure impact**: Verify improvements with benchmarks
3. **Document trade-offs**: Note any complexity added

```bash
# Run benchmarks
cargo bench

# Profile with flamegraph
cargo flamegraph -- --url https://example.com --concurrent 100 --duration 10

# Compare before/after
cargo bench --bench metrics_bench > before.txt
# Make changes
cargo bench --bench metrics_bench > after.txt
diff before.txt after.txt
```

---

## Getting Help

### Resources

- **Documentation**: README.md, ARCHITECTURE.md, TROUBLESHOOTING.md
- **Code**: Read existing code for patterns and examples
- **Issues**: Search for similar issues/questions
- **Rust**: https://doc.rust-lang.org/book/

### Asking Questions

**Good Question**:
```
I'm trying to add a new load distribution strategy. I've implemented
it in target_selector.rs, but I'm not sure how to integrate it with
the configuration system. Should I add a new enum variant to
LoadDistribution? Here's what I have so far: [code snippet]
```

**Less Helpful**:
```
How do I add a feature?
```

### Where to Ask

1. **GitHub Issues**: For bugs and feature requests
2. **Pull Request Comments**: For questions about specific code
3. **GitHub Discussions**: For general questions (if enabled)

---

## Release Process

(For maintainers)

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag: `git tag v0.x.0`
5. Push tag: `git push origin v0.x.0`
6. Create GitHub release
7. Publish to crates.io: `cargo publish`

---

## Recognition

Contributors will be:
- Listed in git history
- Mentioned in release notes (for significant contributions)
- Added to CONTRIBUTORS.md (if they wish)

---

## License

By contributing to http-traffic-sim, you agree that your contributions
will be licensed under the same license as the project.

---

## Thank You!

Every contribution, no matter how small, is valuable. Whether you're
fixing typos, adding features, or improving documentation, thank you
for helping make http-traffic-sim better!

---

**Questions?** Open an issue or discussion on GitHub.

**Ready to contribute?** Pick an issue labeled "good first issue" to get started!
