mod client;
mod config;
mod metrics;
mod patterns;
mod reporter;
mod stats;

use anyhow::Result;
use tokio::signal;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::client::HttpClient;
use crate::config::Config;
use crate::metrics::MetricsCollector;
use crate::patterns::PatternExecutor;
use crate::reporter::Reporter;
use crate::stats::Statistics;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Setup logging
    setup_logging(config.verbose);

    // Print startup info
    print_startup_info(&config);

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

    // Setup cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();

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

            // Skip the first immediate tick
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
        // Give the reporter 2 seconds to shut down gracefully, then abort
        match tokio::time::timeout(Duration::from_secs(2), handle).await {
            Ok(_) => {},
            Err(_) => {
                tracing::warn!("Reporter task did not shut down gracefully, continuing...");
            }
        }
    }

    // Check if execution failed
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
    println!("Target URL:            {}", config.target.url);
    println!("Method:                {}", config.target.method);
    println!("Timeout:               {}s", config.client.timeout_secs);

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

    println!("{}\n", "=".repeat(80));
}
