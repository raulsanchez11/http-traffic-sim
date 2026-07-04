//! Authorization validation for stress testing.
//!
//! This module ensures that stress testing patterns are only executed with
//! proper authorization. It provides:
//!
//! - Authorization validation before stress tests
//! - Safety limit enforcement
//! - Prominent warning displays
//! - 5-second countdown before execution
//!
//! # Purpose
//!
//! Stress testing can generate extreme load and potentially impact service
//! availability. This module ensures users:
//!
//! - Explicitly confirm authorization via configuration
//! - Are warned about legal and ethical implications
//! - Have time to cancel before execution begins
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::authorization::prepare_stress_run;
//! use http_traffic_sim::config::{StressPattern, AuthorizationConfig, SafetyLimits};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let pattern = StressPattern::RequestFlood {
//!     target_rps: 1000,
//!     duration_secs: 60,
//! };
//!
//! let auth = Some(AuthorizationConfig {
//!     confirmed: true,
//!     target_owner: Some("Security Team - Ticket #12345".to_string()),
//!     authorization_notes: Some("Authorized load test".to_string()),
//! });
//!
//! let limits = SafetyLimits::default();
//!
//! prepare_stress_run(&pattern, &auth, &limits).await?;
//! # Ok(())
//! # }
//! ```

use crate::config::{AuthorizationConfig, SafetyLimits, StressPattern};
use anyhow::Result;
use std::io::{self, Write};

/// Validates authorization for stress testing and displays warnings.
///
/// This function must be called before executing any stress testing pattern.
/// It verifies that:
///
/// - Authorization configuration is present
/// - Authorization is explicitly confirmed (`confirmed: true`)
/// - Safety limits are configured and respected
///
/// After validation, displays a prominent warning with:
/// - Legal notice about unauthorized testing
/// - Pattern description and parameters
/// - Authorization details (who authorized, notes)
/// - Configured safety limits
/// - 5-second countdown before execution
///
/// # Arguments
///
/// * `stress_pattern` - The stress testing pattern to be executed
/// * `authorization` - Authorization configuration (must have `confirmed: true`)
/// * `safety_limits` - Safety limits to enforce and display
///
/// # Errors
///
/// Returns an error if:
/// - Authorization configuration is missing
/// - Authorization is not confirmed (`confirmed: false` or missing)
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::authorization::prepare_stress_run;
/// use http_traffic_sim::config::{StressPattern, AuthorizationConfig, SafetyLimits};
///
/// # async fn example() -> anyhow::Result<()> {
/// let pattern = StressPattern::RequestFlood {
///     target_rps: 1000,
///     duration_secs: 60,
/// };
///
/// let auth = Some(AuthorizationConfig {
///     confirmed: true,
///     target_owner: Some("Security Team".to_string()),
///     authorization_notes: Some("Approved load test".to_string()),
/// });
///
/// let limits = SafetyLimits::default();
///
/// // Validates and displays warning with 5-second countdown
/// prepare_stress_run(&pattern, &auth, &limits).await?;
/// # Ok(())
/// # }
/// ```
///
/// Display stress-test warning and countdown. Auth/safety limits are validated during config load.
pub async fn prepare_stress_run(
    stress_pattern: &StressPattern,
    authorization: &Option<AuthorizationConfig>,
    safety_limits: &SafetyLimits,
) -> Result<()> {
    let auth = authorization.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Stress testing requires authorization configuration.\n\
            Add an 'authorization' section with 'confirmed: true' to your config file."
        )
    })?;

    if !auth.confirmed {
        anyhow::bail!(
            "Stress testing requires authorization.confirmed to be true.\n\
            Set authorization.confirmed: true in your config file."
        );
    }

    display_stress_warning(stress_pattern, auth, safety_limits);
    countdown().await;
    Ok(())
}

async fn countdown() {
    println!("\nStarting stress test in 5 seconds... Press Ctrl+C to cancel.");
    for i in (1..=5).rev() {
        print!("{}... ", i);
        let _ = io::stdout().flush();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    println!("\n");
}

fn display_stress_warning(
    pattern: &StressPattern,
    auth: &AuthorizationConfig,
    safety_limits: &SafetyLimits,
) {
    println!("\n{}", "!".repeat(80));
    println!("                      ⚠️  STRESS TEST WARNING ⚠️");
    println!("{}\n", "!".repeat(80));

    println!("You are about to run a STRESS TEST which may:");
    println!("  • Generate extreme load that could impact service availability");
    println!("  • Consume significant network and system resources");
    println!("  • Trigger security alerts or rate limiting");
    println!("  • Affect other users if run against shared infrastructure");
    println!();

    println!("LEGAL NOTICE:");
    println!("  Unauthorized stress testing may be ILLEGAL in your jurisdiction.");
    println!("  Only run stress tests against infrastructure you own or have");
    println!("  explicit written permission to test.");
    println!();

    println!("Pattern: {}", pattern.describe());
    println!();

    if let Some(owner) = &auth.target_owner {
        println!("Authorized by: {}", owner);
    }

    if let Some(notes) = &auth.authorization_notes {
        println!("Notes: {}", notes);
    }

    // Display safety limits if configured
    println!();
    let has_limits = safety_limits.max_connections_per_second.is_some()
        || safety_limits.max_requests_per_second.is_some()
        || safety_limits.max_payload_size_mb.is_some()
        || safety_limits.max_concurrent_connections.is_some();

    if has_limits {
        println!("Safety Limits Configured:");
        if let Some(max) = safety_limits.max_connections_per_second {
            println!("  Max connections/sec: {}", max);
        }
        if let Some(max) = safety_limits.max_requests_per_second {
            println!("  Max requests/sec: {}", max);
        }
        if let Some(max) = safety_limits.max_payload_size_mb {
            println!("  Max payload size: {} MB", max);
        }
        if let Some(max) = safety_limits.max_concurrent_connections {
            println!("  Max concurrent connections: {}", max);
        }
    } else {
        println!("Safety Limits: None configured (unlimited)");
    }

    println!();
    println!("{}", "!".repeat(80));
}


