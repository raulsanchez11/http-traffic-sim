mod authorization;
mod client;
mod config;
mod metrics;
mod patterns;
mod reporter;
mod stats;
mod stress;
mod target_selector;

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::client::HttpClient;
use crate::config::{Config, ExecutionMode};
use crate::metrics::{MetricsCollector, MultiTargetMetrics};
use crate::patterns::PatternExecutor;
use crate::reporter::Reporter;
use crate::stats::Statistics;
use crate::stress::StressExecutor;
use crate::target_selector::TargetSelector;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Setup logging
    setup_logging(config.verbose);

    // Validate stress testing authorization if needed
    if let Some(ref stress_pattern) = config.stress_pattern {
        authorization::validate_and_warn(stress_pattern, &config.authorization, &config.safety_limits)?;
    }

    // Print startup info
    print_startup_info(&config);

    // Determine execution mode
    let execution_mode = config.get_execution_mode();

    // Setup cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();

    // Execute based on mode
    match execution_mode {
        ExecutionMode::SingleTarget => {
            execute_single_target(config, cancel_token).await?;
        }
        ExecutionMode::MultiTarget => {
            execute_multi_target(config, cancel_token).await?;
        }
        ExecutionMode::StressTest => {
            execute_stress_test(config, cancel_token).await?;
        }
    }

    Ok(())
}

async fn execute_single_target(config: Config, cancel_token: CancellationToken) -> Result<()> {
    // Create HTTP client
    let client = HttpClient::new(
        config.target.clone(),
        config.get_timeout(),
        config.client.pool_max_idle_per_host,
    )?;

    // Create metrics collector
    let metrics = MetricsCollector::new();
    metrics.reset_start_time();

    // Create pattern executor
    let executor = PatternExecutor::new(client, metrics.clone(), config.pattern.clone());

    // Spawn ctrl+c handler
    let cancel_token_signal = cancel_token.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        tracing::info!("Received interrupt signal, shutting down...");
        cancel_token_signal.cancel();
    });

    // Spawn realtime reporter
    let reporter_handle = if config.output.realtime_updates {
        let metrics_clone = metrics.clone();
        let cancel_clone = cancel_token.clone();
        Some(tokio::spawn(async move {
            let mut reporter = Reporter::new(true);
            let mut ticker = interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            ticker.tick().await;

            loop {
                tokio::select! {
                    biased;
                    _ = cancel_clone.cancelled() => {
                        break;
                    }
                    _ = ticker.tick() => {
                        let snapshot = metrics_clone.get_snapshot();
                        let stats = Statistics::from_snapshot(&snapshot);
                        reporter.show_realtime_update(&stats);
                    }
                }
            }
        }))
    } else {
        None
    };

    // Execute traffic pattern
    tracing::info!("Starting traffic generation...");
    let result = executor.execute(cancel_token.clone()).await;

    // Cancel reporter
    cancel_token.cancel();
    if let Some(handle) = reporter_handle {
        match tokio::time::timeout(Duration::from_secs(2), handle).await {
            Ok(_) => {}
            Err(_) => {
                tracing::warn!("Reporter task did not shut down gracefully, continuing...");
            }
        }
    }

    if let Err(e) = result {
        tracing::error!("Traffic generation failed: {}", e);
        return Err(e);
    }

    // Get final metrics
    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);

    // Display final summary
    if config.output.console {
        let reporter = Reporter::new(false);
        reporter.show_final_summary(&stats);
    }

    // Export to file if specified
    if let Some(output_path) = &config.output.file {
        let reporter = Reporter::new(false);
        reporter.export_json(&stats, output_path)?;
    }

    Ok(())
}

async fn execute_multi_target(config: Config, cancel_token: CancellationToken) -> Result<()> {
    let target_group = config.targets.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Multi-target mode requires targets configuration")
    })?;

    // Create target selector
    let targets: Vec<Arc<crate::config::TargetConfig>> = target_group
        .targets
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut target = t.clone();
            if target.id.is_empty() {
                target.id = format!("target-{}", i);
            }
            Arc::new(target)
        })
        .collect();

    let selector = Arc::new(TargetSelector::new(
        targets,
        target_group.distribution.clone(),
    ));

    // Create HTTP client with multi-target support
    let client = HttpClient::new_multi_target(
        selector,
        config.get_timeout(),
        config.client.pool_max_idle_per_host,
    )?;

    // Create multi-target metrics
    let metrics = MultiTargetMetrics::new();
    metrics.reset_start_time();

    // Create pattern executor
    let executor = PatternExecutor::new_multi_target(client, metrics.clone(), config.pattern.clone());

    // Spawn ctrl+c handler
    let cancel_token_signal = cancel_token.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        tracing::info!("Received interrupt signal, shutting down...");
        cancel_token_signal.cancel();
    });

    // Execute traffic pattern
    tracing::info!("Starting multi-target traffic generation...");
    let result = executor.execute(cancel_token.clone()).await;

    cancel_token.cancel();

    if let Err(e) = result {
        tracing::error!("Traffic generation failed: {}", e);
        return Err(e);
    }

    // Get final metrics
    let global_snapshot = metrics.get_global_snapshot();
    let per_target_snapshots = metrics.get_per_target_snapshots();

    // Display results
    if config.output.console {
        println!("\n{}", "=".repeat(80));
        println!("                   MULTI-TARGET FINAL RESULTS");
        println!("{}\n", "=".repeat(80));

        // Global summary
        let global_stats = Statistics::from_snapshot(&global_snapshot);
        println!("GLOBAL SUMMARY:");
        println!("Duration:              {:.2}s", global_stats.duration.as_secs_f64());
        println!("Total Requests:        {}", global_stats.total_requests);
        println!("Successful:            {} ({:.1}%)", global_stats.successful_requests, global_stats.success_rate);
        println!("Failed:                {} ({:.1}%)", global_stats.failed_requests, global_stats.error_rate);
        println!("Requests/sec:          {:.2}", global_stats.requests_per_second);

        // Per-target breakdown
        println!("\n{}", "-".repeat(80));
        println!("PER-TARGET BREAKDOWN:");
        println!("{}", "-".repeat(80));

        for (target_id, snapshot) in per_target_snapshots.iter() {
            let stats = Statistics::from_snapshot(snapshot);
            let percentage = (stats.total_requests as f64 / global_stats.total_requests as f64) * 100.0;

            println!("\nTarget: {} ({:.1}% of traffic)", target_id, percentage);
            println!("  Total Requests:     {}", stats.total_requests);
            println!("  Success Rate:       {:.1}%", stats.success_rate);
            println!("  Avg Latency:        {:.2}ms", stats.latency.mean_ms);
            println!("  P99 Latency:        {:.2}ms", stats.latency.p99_ms);

            if stats.failed_requests > 0 {
                println!("  Connection Errors:");
                let conn_stats = &snapshot.connection_stats;
                if conn_stats.refused_count > 0 {
                    println!("    - Refused:        {}", conn_stats.refused_count);
                }
                if conn_stats.timeout_count > 0 {
                    println!("    - Timeout:        {}", conn_stats.timeout_count);
                }
                if conn_stats.reset_by_peer_count > 0 {
                    println!("    - Reset:          {}", conn_stats.reset_by_peer_count);
                }
                if conn_stats.tls_handshake_errors > 0 {
                    println!("    - TLS:            {}", conn_stats.tls_handshake_errors);
                }
            }
        }

        println!("\n{}\n", "=".repeat(80));
    }

    Ok(())
}

async fn execute_stress_test(config: Config, cancel_token: CancellationToken) -> Result<()> {
    let stress_pattern = config.stress_pattern.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Stress test mode requires stress_pattern configuration")
    })?;

    // Create HTTP client
    let client = HttpClient::new(
        config.target.clone(),
        config.get_timeout(),
        config.client.pool_max_idle_per_host,
    )?;

    // Create metrics collector
    let metrics = MetricsCollector::new();
    metrics.reset_start_time();

    // Create stress executor
    let executor = StressExecutor::new(client, metrics.clone(), stress_pattern.clone());

    // Spawn ctrl+c handler
    let cancel_token_signal = cancel_token.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        tracing::info!("Received interrupt signal, shutting down...");
        cancel_token_signal.cancel();
    });

    // Execute stress pattern
    tracing::info!("Starting stress test...");
    let result = executor.execute(cancel_token.clone()).await;

    cancel_token.cancel();

    if let Err(e) = result {
        tracing::error!("Stress test failed: {}", e);
        return Err(e);
    }

    // Get final metrics
    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);

    // Display final summary
    if config.output.console {
        let reporter = Reporter::new(false);
        reporter.show_final_summary(&stats);

        // Show connection stats
        println!("{}", "-".repeat(80));
        println!("CONNECTION STATISTICS");
        println!("{}", "-".repeat(80));
        let conn_stats = &snapshot.connection_stats;
        println!("Refused:               {}", conn_stats.refused_count);
        println!("Timeout:               {}", conn_stats.timeout_count);
        println!("Reset by peer:         {}", conn_stats.reset_by_peer_count);
        println!("TLS handshake errors:  {}", conn_stats.tls_handshake_errors);
        println!("DNS errors:            {}", conn_stats.dns_errors);
        println!("Other errors:          {}", conn_stats.other_errors);
        println!("\n{}\n", "=".repeat(80));
    }

    // Export to file if specified
    if let Some(output_path) = &config.output.file {
        let reporter = Reporter::new(false);
        reporter.export_json(&stats, output_path)?;
    }

    Ok(())
}

fn setup_logging(verbose: u8) {
    let level = match verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}

fn print_startup_info(config: &Config) {
    println!("\n{}", "=".repeat(80));
    println!("           HTTP/HTTPS TRAFFIC SIMULATOR");
    println!("{}", "=".repeat(80));

    // Show execution mode
    let mode = config.get_execution_mode();
    match mode {
        ExecutionMode::SingleTarget => {
            println!("Mode:                  Single Target");
            println!("Target URL:            {}", config.target.url);
            println!("Method:                {}", config.target.method);
        }
        ExecutionMode::MultiTarget => {
            if let Some(ref targets) = config.targets {
                println!("Mode:                  Multi-Target");
                println!("Target Count:          {}", targets.targets.len());
                println!("Distribution:          {:?}", targets.distribution);
                println!("\nTargets:");
                for (i, target) in targets.targets.iter().enumerate() {
                    let id = if target.id.is_empty() {
                        format!("target-{}", i)
                    } else {
                        target.id.clone()
                    };
                    println!("  {} - {} ({})", id, target.url, target.method);
                }
            }
        }
        ExecutionMode::StressTest => {
            println!("Mode:                  Stress Test");
            println!("Target URL:            {}", config.target.url);
            if let Some(ref pattern) = config.stress_pattern {
                println!("Stress Pattern:        {:?}", pattern);
            }
        }
    }

    println!("Timeout:               {}s", config.client.timeout_secs);

    // Show pattern info (if not stress test)
    if mode != ExecutionMode::StressTest {
        match &config.pattern {
        config::TrafficPattern::Fixed { concurrent, duration_secs, total_requests } => {
            println!("Pattern:               Fixed Concurrency");
            println!("Concurrent Clients:    {}", concurrent);
            if let Some(duration) = duration_secs {
                println!("Duration:              {}s", duration);
            }
            if let Some(total) = total_requests {
                println!("Total Requests:        {}", total);
            }
        }
        config::TrafficPattern::RateLimit { rate, duration_secs, total_requests } => {
            println!("Pattern:               Rate Limited");
            println!("Rate:                  {} req/s", rate);
            if let Some(duration) = duration_secs {
                println!("Duration:              {}s", duration);
            }
            if let Some(total) = total_requests {
                println!("Total Requests:        {}", total);
            }
        }
        config::TrafficPattern::Ramp { from, to, ramp_duration_secs, hold_duration_secs } => {
            println!("Pattern:               Ramp-up");
            println!("From:                  {} clients", from);
            println!("To:                    {} clients", to);
            println!("Ramp Duration:         {}s", ramp_duration_secs);
            if let Some(hold) = hold_duration_secs {
                println!("Hold Duration:         {}s", hold);
            }
        }
        config::TrafficPattern::Burst { size, interval_secs, duration_secs, total_bursts } => {
            println!("Pattern:               Burst");
            println!("Burst Size:            {} requests", size);
            println!("Burst Interval:        {}s", interval_secs);
            if let Some(duration) = duration_secs {
                println!("Duration:              {}s", duration);
            }
            if let Some(total) = total_bursts {
                println!("Total Bursts:          {}", total);
            }
        }
    }
    }

    println!("{}\n", "=".repeat(80));
}
