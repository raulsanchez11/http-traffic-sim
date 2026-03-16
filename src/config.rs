use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Parser)]
#[command(name = "http-traffic-sim")]
#[command(about = "HTTP/HTTPS traffic simulator and benchmarking tool")]
pub struct CliArgs {
    /// Path to configuration file (YAML or TOML)
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Target URL to test
    #[arg(short, long)]
    pub url: Option<String>,

    /// Number of concurrent requests (fixed concurrency mode)
    #[arg(long)]
    pub concurrent: Option<usize>,

    /// Test duration in seconds
    #[arg(short, long)]
    pub duration: Option<u64>,

    /// Total number of requests to send
    #[arg(short = 'n', long)]
    pub requests: Option<usize>,

    /// Requests per second (rate-limited mode)
    #[arg(long)]
    pub rate: Option<usize>,

    /// Ramp-up: starting concurrent clients
    #[arg(long)]
    pub ramp_from: Option<usize>,

    /// Ramp-up: ending concurrent clients
    #[arg(long)]
    pub ramp_to: Option<usize>,

    /// Ramp-up duration in seconds
    #[arg(long)]
    pub ramp_duration: Option<u64>,

    /// Burst mode: requests per burst
    #[arg(long)]
    pub burst_size: Option<usize>,

    /// Burst mode: interval between bursts in seconds
    #[arg(long)]
    pub burst_interval: Option<u64>,

    /// Output file for results (JSON format)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    #[arg(short, long, default_value = "GET")]
    pub method: String,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Verbosity level (0=error, 1=warn, 2=info, 3=debug, 4=trace)
    #[arg(short, long, default_value = "1")]
    pub verbose: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub target: TargetConfig,

    #[serde(default)]
    pub pattern: TrafficPattern,

    #[serde(default)]
    pub client: ClientConfig,

    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            target: TargetConfig::default(),
            pattern: TrafficPattern::default(),
            client: ClientConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub url: String,

    #[serde(default = "default_method")]
    pub method: String,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub body: Option<String>,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: default_method(),
            headers: HashMap::new(),
            body: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TrafficPattern {
    Fixed {
        concurrent: usize,
        #[serde(default)]
        duration_secs: Option<u64>,
        #[serde(default)]
        total_requests: Option<usize>,
    },
    RateLimit {
        rate: usize,
        #[serde(default)]
        duration_secs: Option<u64>,
        #[serde(default)]
        total_requests: Option<usize>,
    },
    Ramp {
        from: usize,
        to: usize,
        ramp_duration_secs: u64,
        #[serde(default)]
        hold_duration_secs: Option<u64>,
    },
    Burst {
        size: usize,
        interval_secs: u64,
        #[serde(default)]
        duration_secs: Option<u64>,
        #[serde(default)]
        total_bursts: Option<usize>,
    },
}

impl Default for TrafficPattern {
    fn default() -> Self {
        TrafficPattern::Fixed {
            concurrent: 10,
            duration_secs: Some(30),
            total_requests: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    #[serde(default = "default_pool_size")]
    pub pool_max_idle_per_host: usize,

    #[serde(default = "default_true")]
    pub http2_prior_knowledge: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            pool_max_idle_per_host: default_pool_size(),
            http2_prior_knowledge: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub file: Option<PathBuf>,

    #[serde(default = "default_true")]
    pub console: bool,

    #[serde(default)]
    pub realtime_updates: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            file: None,
            console: true,
            realtime_updates: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub target: TargetConfig,
    pub pattern: TrafficPattern,
    pub client: ClientConfig,
    pub output: OutputConfig,
    pub verbose: u8,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_pool_size() -> usize {
    128
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load() -> Result<Self> {
        let args = CliArgs::parse();

        // Load config file if specified
        let mut config = if let Some(config_path) = &args.config {
            Self::load_file(config_path)?
        } else {
            ConfigFile::default()
        };

        // CLI overrides
        if let Some(ref url) = args.url {
            config.target.url = url.clone();
        }

        if !args.method.is_empty() {
            config.target.method = args.method.to_uppercase();
        }

        config.client.timeout_secs = args.timeout;

        if let Some(ref output) = args.output {
            config.output.file = Some(output.clone());
        }

        // Determine traffic pattern from CLI args
        if let Some(pattern) = Self::pattern_from_args(&args)? {
            config.pattern = pattern;
        }

        // Validate configuration
        if config.target.url.is_empty() {
            anyhow::bail!("Target URL is required. Specify with --url or in config file.");
        }

        Ok(Config {
            target: config.target,
            pattern: config.pattern,
            client: config.client,
            output: config.output,
            verbose: args.verbose,
        })
    }

    fn load_file(path: &PathBuf) -> Result<ConfigFile> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "yaml" | "yml" => {
                serde_yaml::from_str(&contents)
                    .with_context(|| "Failed to parse YAML config")
            }
            "toml" => {
                toml::from_str(&contents)
                    .with_context(|| "Failed to parse TOML config")
            }
            _ => {
                anyhow::bail!("Unsupported config file format. Use .yaml, .yml, or .toml")
            }
        }
    }

    fn pattern_from_args(args: &CliArgs) -> Result<Option<TrafficPattern>> {
        // Burst mode
        if args.burst_size.is_some() || args.burst_interval.is_some() {
            let size = args.burst_size.context("--burst-size is required for burst mode")?;
            let interval = args.burst_interval.context("--burst-interval is required for burst mode")?;

            return Ok(Some(TrafficPattern::Burst {
                size,
                interval_secs: interval,
                duration_secs: args.duration,
                total_bursts: None,
            }));
        }

        // Ramp-up mode
        if args.ramp_from.is_some() || args.ramp_to.is_some() || args.ramp_duration.is_some() {
            let from = args.ramp_from.context("--ramp-from is required for ramp mode")?;
            let to = args.ramp_to.context("--ramp-to is required for ramp mode")?;
            let ramp_duration = args.ramp_duration.context("--ramp-duration is required for ramp mode")?;

            return Ok(Some(TrafficPattern::Ramp {
                from,
                to,
                ramp_duration_secs: ramp_duration,
                hold_duration_secs: args.duration,
            }));
        }

        // Rate-limited mode
        if let Some(rate) = args.rate {
            return Ok(Some(TrafficPattern::RateLimit {
                rate,
                duration_secs: args.duration,
                total_requests: args.requests,
            }));
        }

        // Fixed concurrency mode
        if let Some(concurrent) = args.concurrent {
            return Ok(Some(TrafficPattern::Fixed {
                concurrent,
                duration_secs: args.duration,
                total_requests: args.requests,
            }));
        }

        Ok(None)
    }

    pub fn get_timeout(&self) -> Duration {
        Duration::from_secs(self.client.timeout_secs)
    }
}
