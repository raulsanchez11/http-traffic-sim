use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::discovery::DiscoveryResult;
use crate::stats::Statistics;

pub struct Reporter {
    show_realtime: bool,
    last_line_count: usize,
}

impl Reporter {
    pub fn new(show_realtime: bool) -> Self {
        Self {
            show_realtime,
            last_line_count: 0,
        }
    }

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

        println!("Duration:              {:.2}s", stats.duration.as_secs_f64());
        println!("Total Requests:        {}", stats.total_requests);
        println!("Successful:            {} ({:.1}%)", stats.successful_requests, stats.success_rate);
        println!("Failed:                {} ({:.1}%)", stats.failed_requests, stats.error_rate);
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
                println!("  ... and {} more error types", stats.errors.len() - max_errors_to_show);
            }
        }

        println!("\n{}\n", "=".repeat(80));
    }

    pub fn export_json(&self, stats: &Statistics, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(stats)?;
        std::fs::write(path, json)?;
        println!("Results exported to: {}", path.display());
        Ok(())
    }

    pub fn export_discovery_results(&self, results: &[DiscoveryResult], path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        std::fs::write(path, json)?;
        println!("Discovery results exported to: {}", path.display());
        Ok(())
    }
}
