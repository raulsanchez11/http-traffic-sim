use crate::config::{AuthorizationConfig, SafetyLimits, StressPattern};
use anyhow::Result;
use std::io::{self, Write};

/// Validates authorization for stress testing and displays warnings
pub fn validate_and_warn(
    stress_pattern: &StressPattern,
    authorization: &Option<AuthorizationConfig>,
    safety_limits: &SafetyLimits,
) -> Result<()> {
    // Check if authorization is present and confirmed
    match authorization {
        Some(auth) if auth.confirmed => {
            // Display warning with authorization details
            display_stress_warning(stress_pattern, auth, safety_limits);
            Ok(())
        }
        Some(_) => {
            anyhow::bail!(
                "Stress testing requires authorization.confirmed to be true.\n\
                Set authorization.confirmed: true in your config file."
            )
        }
        None => {
            anyhow::bail!(
                "Stress testing requires authorization configuration.\n\
                Add an 'authorization' section with 'confirmed: true' to your config file.\n\
                \n\
                Example:\n\
                authorization:\n  \
                  confirmed: true\n  \
                  target_owner: \"Your Name/Team - Ticket #12345\"\n  \
                  authorization_notes: \"Load testing authorized infrastructure\""
            )
        }
    }
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

    println!("Pattern: {}", pattern_description(pattern));
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

    // 5-second countdown
    println!("\nStarting stress test in 5 seconds... Press Ctrl+C to cancel.");
    for i in (1..=5).rev() {
        print!("{}... ", i);
        io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("\n");
}

fn pattern_description(pattern: &StressPattern) -> String {
    match pattern {
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
