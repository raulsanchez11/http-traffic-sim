//! Application orchestration entry point.

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::client::HttpClient;
use crate::config::{Config, ExecutionMode, TargetConfig};
use crate::discovery::{
    self, discover_targets, find_best_port, handle_failures, update_url_port, FailureAction,

};
use crate::metrics::{MetricsCollector, MultiTargetMetrics};
use crate::patterns::PatternExecutor;
use crate::reporter::Reporter;
use crate::stats::Statistics;
use crate::stress::StressExecutor;
use crate::target_selector::TargetSelector;

pub async fn run() -> Result<()> {
    let config = Config::load()?;
    setup_logging(config.verbose);

    let config = if should_perform_discovery(&config) {
        perform_discovery(config).await?
    } else {
        config
    };

    print_startup_info(&config);

    let cancel_token = CancellationToken::new();
    spawn_ctrl_c_handler(cancel_token.clone());

    match config.get_execution_mode() {
        ExecutionMode::SingleTarget => execute_single_target(config, cancel_token).await,
        ExecutionMode::MultiTarget => execute_multi_target(config, cancel_token).await,
        ExecutionMode::StressTest => execute_stress_test(config, cancel_token).await,
    }
}

fn spawn_ctrl_c_handler(cancel_token: CancellationToken) {
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        tracing::info!("Received interrupt signal, shutting down...");
        cancel_token.cancel();
    });
}

async fn execute_single_target(config: Config, cancel_token: CancellationToken) -> Result<()> {
    let client = HttpClient::new(config.target.clone(), &config.client)?;
    let metrics = MetricsCollector::new();
    metrics.reset_start_time();

    let executor = PatternExecutor::new(client, metrics.clone(), config.pattern.clone());

    let reporter_handle = spawn_realtime_reporter(&config, metrics.clone(), cancel_token.clone());

    tracing::info!("Starting traffic generation...");
    let result = executor.execute(cancel_token.clone()).await;

    cancel_token.cancel();
    shutdown_reporter(reporter_handle).await;

    if let Err(e) = result {
        tracing::error!("Traffic generation failed: {e}");
        return Err(e);
    }

    let snapshot = metrics.get_snapshot();
    let stats = Statistics::from_snapshot(&snapshot);
    emit_results(&config, &stats, None)?;
    Ok(())
}

async fn execute_multi_target(config: Config, cancel_token: CancellationToken) -> Result<()> {
    let target_group = config
        .targets
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Multi-target mode requires targets configuration"))?;

    let targets = assign_target_ids(&target_group.targets);

    let selector = Arc::new(TargetSelector::new(
        targets,
        target_group.distribution.clone(),
    ));

    let client = HttpClient::new_multi_target(selector, &config.client)?;
    let metrics = MultiTargetMetrics::new();
    metrics.reset_start_time();

    let executor =
        PatternExecutor::new_multi_target(client, metrics.clone(), config.pattern.clone());

    tracing::info!("Starting multi-target traffic generation...");
    let result = executor.execute(cancel_token.clone()).await;
    cancel_token.cancel();

    if let Err(e) = result {
        tracing::error!("Traffic generation failed: {e}");
        return Err(e);
    }

    let global_snapshot = metrics.get_global_snapshot();
    let per_target_snapshots = metrics.get_per_target_snapshots();

    let global_stats = Statistics::from_snapshot(&global_snapshot);
    let per_target_stats: std::collections::HashMap<String, Statistics> = per_target_snapshots
        .iter()
        .map(|(id, snap)| (id.clone(), Statistics::from_snapshot(snap)))
        .collect();

    emit_results(&config, &global_stats, Some(&per_target_stats))?;
    Ok(())
}

async fn execute_stress_test(config: Config, cancel_token: CancellationToken) -> Result<()> {
    let stress_pattern = config
        .stress_pattern
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Stress test mode requires stress_pattern configuration"))?;

    let client = HttpClient::new(config.target.clone(), &config.client)?;
    let metrics = MetricsCollector::new();
    metrics.reset_start_time();

    let executor = StressExecutor::new(client, metrics.clone(), stress_pattern.clone());

    tracing::info!("Starting stress test...");
    let result = executor.execute(cancel_token.clone()).await;
    cancel_token.cancel();

    if let Err(e) = result {
        tracing::error!("Stress test failed: {e}");
        return Err(e);
    }

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

    Ok(())
}

fn assign_target_ids(targets: &[TargetConfig]) -> Vec<Arc<TargetConfig>> {
    targets
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut target = t.clone();
            target.id = target.effective_id(Some(i));
            Arc::new(target)
        })
        .collect()
}

fn emit_results(
    config: &Config,
    stats: &Statistics,
    per_target: Option<&std::collections::HashMap<String, Statistics>>,
) -> Result<()> {
    if config.output.console {
        let reporter = Reporter::new(false);
        if let Some(per_target) = per_target {
            reporter.show_multi_target_summary(stats, per_target);
        } else {
            reporter.show_final_summary(stats);
        }
    }

    if let Some(output_path) = &config.output.file {
        let reporter = Reporter::new(false);
        if let Some(per_target) = per_target {
            reporter.export_multi_target_json(stats, per_target, output_path)?;
        } else {
            reporter.export_json(stats, output_path)?;
        }
    }

    Ok(())
}

fn spawn_realtime_reporter(
    config: &Config,
    metrics: MetricsCollector,
    cancel_token: CancellationToken,
) -> Option<tokio::task::JoinHandle<()>> {
    if !config.output.realtime_updates {
        return None;
    }

    Some(tokio::spawn(async move {
        let mut reporter = Reporter::new(true);
        let mut ticker = interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        ticker.tick().await;

        loop {
            tokio::select! {
                biased;
                _ = cancel_token.cancelled() => break,
                _ = ticker.tick() => {
                    let snapshot = metrics.get_snapshot();
                    let stats = Statistics::from_snapshot(&snapshot);
                    reporter.show_realtime_update(&stats);
                }
            }
        }
    }))
}

async fn shutdown_reporter(handle: Option<tokio::task::JoinHandle<()>>) {
    if let Some(handle) = handle {
        match tokio::time::timeout(Duration::from_secs(2), handle).await {
            Ok(_) => {}
            Err(_) => tracing::warn!("Reporter task did not shut down gracefully, continuing..."),
        }
    }
}

fn print_connection_stats(conn_stats: &crate::metrics::ConnectionStatsSnapshot) {
    println!("{}", "-".repeat(80));
    println!("CONNECTION STATISTICS");
    println!("{}", "-".repeat(80));
    println!("Refused:               {}", conn_stats.refused_count);
    println!("Timeout:               {}", conn_stats.timeout_count);
    println!("Reset by peer:         {}", conn_stats.reset_by_peer_count);
    println!("TLS handshake errors:  {}", conn_stats.tls_handshake_errors);
    println!("DNS errors:            {}", conn_stats.dns_errors);
    println!("Other errors:          {}", conn_stats.other_errors);
    println!("\n{}\n", "=".repeat(80));
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
                    let id = target.effective_id(Some(i));
                    println!("  {id} - {} ({})", target.url, target.method);
                }
            }
        }
        ExecutionMode::StressTest => {
            println!("Mode:                  Stress Test");
            println!("Target URL:            {}", config.target.url);
            if let Some(ref pattern) = config.stress_pattern {
                println!("Stress Pattern:        {}", pattern.describe());
            }
        }
    }

    println!("Timeout:               {}s", config.client.timeout_secs);

    if mode != ExecutionMode::StressTest {
        println!("{}", config.pattern.describe());
    }

    println!("{}\n", "=".repeat(80));
}

fn should_perform_discovery(config: &Config) -> bool {
    if config
        .target
        .discovery
        .as_ref()
        .is_some_and(|d| d.enabled)
    {
        return true;
    }

    config.targets.as_ref().is_some_and(|targets| {
        targets
            .targets
            .iter()
            .any(|t| t.discovery.as_ref().is_some_and(|d| d.enabled))
    })
}

async fn perform_discovery(mut config: Config) -> Result<Config> {
    println!("\n{}", "=".repeat(80));
    println!("                    PORT DISCOVERY PHASE");
    println!("{}\n", "=".repeat(80));

    let mut targets_to_discover = Vec::new();

    if let Some(ref discovery) = config.target.discovery {
        if discovery.enabled && !config.target.url.is_empty() {
            let host = discovery::extract_host_from_url(&config.target.url)?;
            let id = config.target.effective_id(None);
            targets_to_discover.push((id, host, discovery.clone()));
        }
    }

    if let Some(ref targets) = config.targets {
        for (i, target) in targets.targets.iter().enumerate() {
            if let Some(ref discovery) = target.discovery {
                if discovery.enabled {
                    let host = discovery::extract_host_from_url(&target.url)?;
                    let id = target.effective_id(Some(i));
                    targets_to_discover.push((id, host, discovery.clone()));
                }
            }
        }
    }

    if targets_to_discover.is_empty() {
        println!("No targets with discovery enabled.\n");
        return Ok(config);
    }

    let results = discover_targets(&targets_to_discover).await?;
    discovery::display_results(&results);
    handle_failures(&results, &targets_to_discover)?;
    config = apply_discovery_results(config, results)?;

    Ok(config)
}

fn apply_discovery_results(
    mut config: Config,
    results: Vec<discovery::DiscoveryResult>,
) -> Result<Config> {
    let mut results_map: std::collections::HashMap<String, discovery::DiscoveryResult> = results
        .into_iter()
        .map(|r| (r.target_id.clone(), r))
        .collect();

    if let Some(ref discovery_config) = config.target.discovery {
        if discovery_config.enabled {
            let id = config.target.effective_id(None);

            if let Some(result) = results_map.remove(&id) {
                if discovery_config.on_failure == FailureAction::Skip
                    && result.discovered_ports.is_empty()
                {
                    config.target.discovery = None;
                    tracing::warn!("Skipping single target '{id}' after discovery failure");
                } else if let Some(best_port) = find_best_port(&result) {
                    config.target.url = update_url_port(&config.target.url, best_port)?;
                    tracing::info!(
                        "Updated target URL to use discovered port: {}",
                        config.target.url
                    );
                }
            }
        }
    }

    if let Some(ref mut targets) = config.targets {
        let mut retained = Vec::new();
        for (i, target) in targets.targets.iter_mut().enumerate() {
            if let Some(ref discovery_config) = target.discovery {
                if discovery_config.enabled {
                    let id = target.effective_id(Some(i));

                    if let Some(result) = results_map.remove(&id) {
                        if discovery_config.on_failure == FailureAction::Skip
                            && result.discovered_ports.is_empty()
                        {
                            tracing::warn!("Skipping target '{id}' after discovery failure");
                            continue;
                        }
                        if let Some(best_port) = find_best_port(&result) {
                            target.url = update_url_port(&target.url, best_port)?;
                            tracing::info!(
                                "Updated target '{id}' URL to use discovered port: {}",
                                target.url
                            );
                        }
                    }
                }
            }
            retained.push(target.clone());
        }
        targets.targets = retained;
    }

    Ok(config)
}