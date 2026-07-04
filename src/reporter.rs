//! Results reporting and output formatting.
//!
//! This module handles displaying and exporting load test results:
//!
//! - Real-time progress updates during test execution
//! - Comprehensive final summary with statistics
//! - JSON export for further analysis
//! - Discovery results export
//!
//! # Features
//!
//! - Real-time updates with terminal control sequences
//! - Formatted statistics with percentiles
//! - Status code and error distribution
//! - JSON export for automated processing
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::reporter::Reporter;
//! use http_traffic_sim::stats::Statistics;
//!
//! # fn example(stats: Statistics) -> anyhow::Result<()> {
//! let reporter = Reporter::new(true); // Enable real-time updates
//!
//! // Show final summary
//! reporter.show_final_summary(&stats);
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::discovery::DiscoveryResult;
use crate::stats::Statistics;
use std::collections::HashMap;

/// Reporter for displaying and exporting load test results.
///
/// Handles both real-time progress updates and final result summaries.
/// Can export results to JSON for further analysis.
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::reporter::Reporter;
/// use http_traffic_sim::stats::Statistics;
///
/// # fn example(stats: Statistics) -> anyhow::Result<()> {
/// // Create reporter with real-time updates
/// let mut reporter = Reporter::new(true);
///
/// // During test execution
/// // reporter.show_realtime_update(&stats);
///
/// // After test completion
/// reporter.show_final_summary(&stats);
/// # Ok(())
/// # }
/// ```
pub struct Reporter {
    show_realtime: bool,
    last_line_count: usize,
}

impl Reporter {
    /// Creates a new reporter instance.
    ///
    /// # Arguments
    ///
    /// * `show_realtime` - Whether to display real-time progress updates
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_traffic_sim::reporter::Reporter;
    ///
    /// // With real-time updates
    /// let reporter = Reporter::new(true);
    ///
    /// // Without real-time updates
    /// let reporter = Reporter::new(false);
    /// ```
    pub fn new(show_realtime: bool) -> Self {
        Self {
            show_realtime,
            last_line_count: 0,
        }
    }

    /// Displays a real-time progress update.
    ///
    /// Updates the terminal in-place by clearing previous output and
    /// displaying current statistics. Only shows updates if real-time
    /// mode is enabled.
    ///
    /// # Arguments
    ///
    /// * `stats` - Current statistics snapshot
    ///
    /// # Output Format
    ///
    /// Shows:
    /// - Elapsed time, total requests, RPS, success rate
    /// - Latency percentiles (P50, P90, P99)
    /// - Failed request count
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::reporter::Reporter;
    /// use http_traffic_sim::stats::Statistics;
    ///
    /// # fn example(stats: Statistics) {
    /// let mut reporter = Reporter::new(true);
    ///
    /// // Call periodically during test
    /// reporter.show_realtime_update(&stats);
    /// # }
    /// ```
    pub fn show_realtime_update(&mut self, stats: &Statistics) {
        if !self.show_realtime {
            return;
        }

        // Clear previous lines
        if self.last_line_count > 0 {
            for _ in 0..self.last_line_count {
                print!("\x1B[1A\x1B[2K"); // Move up and clear line
            }
        }

        let output = self.format_realtime(stats);
        print!("{}", output);
        io::stdout().flush().unwrap();

        self.last_line_count = output.lines().count();
    }

    fn format_realtime(&self, stats: &Statistics) -> String {
        format!(
            "Elapsed: {:.1}s | Requests: {} | RPS: {:.1} | Success: {:.1}% | Avg Latency: {:.2}ms\n\
             P50: {:.2}ms | P90: {:.2}ms | P99: {:.2}ms | Failed: {}\n",
            stats.duration.as_secs_f64(),
            stats.total_requests,
            stats.requests_per_second,
            stats.success_rate,
            stats.latency.mean_ms,
            stats.latency.p50_ms,
            stats.latency.p90_ms,
            stats.latency.p99_ms,
            stats.failed_requests,
        )
    }

    /// Displays a comprehensive final summary of test results.
    ///
    /// Shows complete statistics including:
    /// - Total requests, success/failure counts and rates
    /// - Requests per second (throughput)
    /// - Latency statistics (min, max, mean, stddev)
    /// - Latency percentiles (P50, P90, P95, P99, P99.9)
    /// - Status code distribution
    /// - Error distribution (top 10 errors)
    ///
    /// # Arguments
    ///
    /// * `stats` - Final statistics from completed test
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::reporter::Reporter;
    /// use http_traffic_sim::stats::Statistics;
    ///
    /// # fn example(stats: Statistics) {
    /// let reporter = Reporter::new(false);
    ///
    /// // Display final results
    /// reporter.show_final_summary(&stats);
    /// # }
    /// ```
    pub fn show_final_summary(&self, stats: &Statistics) {
        // Clear realtime updates
        if self.show_realtime && self.last_line_count > 0 {
            for _ in 0..self.last_line_count {
                print!("\x1B[1A\x1B[2K");
            }
            io::stdout().flush().unwrap();
        }

        println!("\n{}", "=".repeat(80));
        println!("                        FINAL RESULTS");
        println!("{}\n", "=".repeat(80));

        println!(
            "Duration:              {:.2}s",
            stats.duration.as_secs_f64()
        );
        println!("Total Requests:        {}", stats.total_requests);
        println!(
            "Successful:            {} ({:.1}%)",
            stats.successful_requests, stats.success_rate
        );
        println!(
            "Failed:                {} ({:.1}%)",
            stats.failed_requests, stats.error_rate
        );
        println!("Requests/sec:          {:.2}", stats.requests_per_second);

        println!("\n{}", "-".repeat(80));
        println!("LATENCY STATISTICS (milliseconds)");
        println!("{}", "-".repeat(80));
        println!("Min:                   {:.2}", stats.latency.min_ms);
        println!("Max:                   {:.2}", stats.latency.max_ms);
        println!("Mean:                  {:.2}", stats.latency.mean_ms);
        println!("Std Dev:               {:.2}", stats.latency.stddev_ms);
        println!("\nPercentiles:");
        println!("  50th (median):       {:.2}", stats.latency.p50_ms);
        println!("  90th:                {:.2}", stats.latency.p90_ms);
        println!("  95th:                {:.2}", stats.latency.p95_ms);
        println!("  99th:                {:.2}", stats.latency.p99_ms);
        println!("  99.9th:              {:.2}", stats.latency.p99_9_ms);

        if !stats.status_codes.is_empty() {
            println!("\n{}", "-".repeat(80));
            println!("STATUS CODE DISTRIBUTION");
            println!("{}", "-".repeat(80));
            for (code, count) in &stats.status_codes {
                let percentage = (*count as f64 / stats.total_requests as f64) * 100.0;
                println!("  {}: {} ({:.1}%)", code, count, percentage);
            }
        }

        if !stats.errors.is_empty() {
            println!("\n{}", "-".repeat(80));
            println!("ERROR DISTRIBUTION");
            println!("{}", "-".repeat(80));
            let max_errors_to_show = 10;
            for (error, count) in stats.errors.iter().take(max_errors_to_show) {
                let percentage = (*count as f64 / stats.total_requests as f64) * 100.0;
                println!("  {}: {} ({:.1}%)", error, count, percentage);
            }
            if stats.errors.len() > max_errors_to_show {
                println!(
                    "  ... and {} more error types",
                    stats.errors.len() - max_errors_to_show
                );
            }
        }

        println!("\n{}\n", "=".repeat(80));
    }

    /// Exports test statistics to a JSON file.
    ///
    /// Serializes all statistics to pretty-printed JSON format for:
    /// - Further analysis with external tools
    /// - Automated CI/CD integration
    /// - Historical comparison
    /// - Report generation
    ///
    /// # Arguments
    ///
    /// * `stats` - Statistics to export
    /// * `path` - File path for JSON output
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::reporter::Reporter;
    /// use http_traffic_sim::stats::Statistics;
    /// use std::path::PathBuf;
    ///
    /// # fn example(stats: Statistics) -> anyhow::Result<()> {
    /// let reporter = Reporter::new(false);
    /// let path = PathBuf::from("results.json");
    ///
    /// reporter.export_json(&stats, &path)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn show_multi_target_summary(
        &self,
        global: &Statistics,
        per_target: &HashMap<String, Statistics>,
    ) {
        println!("\n{}", "=".repeat(80));
        println!("                   MULTI-TARGET FINAL RESULTS");
        println!("{}\n", "=".repeat(80));

        println!("GLOBAL SUMMARY:");
        println!(
            "Duration:              {:.2}s",
            global.duration.as_secs_f64()
        );
        println!("Total Requests:        {}", global.total_requests);
        println!(
            "Successful:            {} ({:.1}%)",
            global.successful_requests, global.success_rate
        );
        println!(
            "Failed:                {} ({:.1}%)",
            global.failed_requests, global.error_rate
        );
        println!("Requests/sec:          {:.2}", global.requests_per_second);

        println!("\n{}", "-".repeat(80));
        println!("PER-TARGET BREAKDOWN:");
        println!("{}", "-".repeat(80));

        for (target_id, stats) in per_target {
            let percentage = if global.total_requests > 0 {
                (stats.total_requests as f64 / global.total_requests as f64) * 100.0
            } else {
                0.0
            };

            println!("\nTarget: {target_id} ({percentage:.1}% of traffic)");
            println!("  Total Requests:     {}", stats.total_requests);
            println!("  Success Rate:       {:.1}%", stats.success_rate);
            println!("  Avg Latency:        {:.2}ms", stats.latency.mean_ms);
            println!("  P99 Latency:        {:.2}ms", stats.latency.p99_ms);
        }

        println!("\n{}\n", "=".repeat(80));
    }

    pub fn export_multi_target_json(
        &self,
        global: &Statistics,
        per_target: &HashMap<String, Statistics>,
        path: &PathBuf,
    ) -> Result<()> {
        #[derive(serde::Serialize)]
        struct MultiTargetExport<'a> {
            global: &'a Statistics,
            per_target: &'a HashMap<String, Statistics>,
        }

        let payload = MultiTargetExport { global, per_target };
        let json = serde_json::to_string_pretty(&payload)?;
        std::fs::write(path, json)?;
        println!("Results exported to: {}", path.display());
        Ok(())
    }

    pub fn export_json(&self, stats: &Statistics, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(stats)?;
        std::fs::write(path, json)?;
        println!("Results exported to: {}", path.display());
        Ok(())
    }

    /// Exports port discovery results to a JSON file.
    ///
    /// Serializes discovery results including:
    /// - Discovered ports with status and response times
    /// - Failed ports with error messages
    /// - Service type detection results (HTTP/HTTPS)
    /// - Discovery duration per target
    ///
    /// # Arguments
    ///
    /// * `results` - Discovery results to export
    /// * `path` - File path for JSON output
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::reporter::Reporter;
    /// use http_traffic_sim::discovery::DiscoveryResult;
    /// use std::path::PathBuf;
    ///
    /// # fn example(results: Vec<DiscoveryResult>) -> anyhow::Result<()> {
    /// let reporter = Reporter::new(false);
    /// let path = PathBuf::from("discovery.json");
    ///
    /// reporter.export_discovery_results(&results, &path)?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(dead_code)] // kept for API completeness (discovery export not used in main flows)
    pub fn export_discovery_results(
        &self,
        results: &[DiscoveryResult],
        path: &PathBuf,
    ) -> Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        std::fs::write(path, json)?;
        println!("Discovery results exported to: {}", path.display());
        Ok(())
    }
}
