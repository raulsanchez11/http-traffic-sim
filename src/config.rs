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

    /// Multi-target configuration (optional, overrides single target)
    #[serde(default)]
    pub targets: Option<TargetGroup>,

    #[serde(default)]
    pub pattern: TrafficPattern,

    /// Stress testing pattern (optional, overrides regular pattern)
    pub stress_pattern: Option<StressPattern>,

    /// Authorization for stress testing (required when using stress_pattern)
    pub authorization: Option<AuthorizationConfig>,

    /// Optional safety limits (user-configurable)
    #[serde(default)]
    pub safety_limits: SafetyLimits,

    #[serde(default)]
    pub client: ClientConfig,

    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            target: TargetConfig::default(),
            targets: None,
            pattern: TrafficPattern::default(),
            stress_pattern: None,
            authorization: None,
            safety_limits: SafetyLimits::default(),
            client: ClientConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    #[serde(default)]
    pub id: String,

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
            id: String::new(),
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
pub struct TargetGroup {
    pub distribution: LoadDistribution,
    pub targets: Vec<TargetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "lowercase")]
pub enum LoadDistribution {
    RoundRobin,
    Weighted { weights: Vec<f64> },
    Random,
    Hash { field: HashField },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashField {
    SourceIp,
    SessionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "lowercase")]
pub enum StressPattern {
    ConnectionFlood {
        connections_per_second: usize,
        hold_time_ms: u64,
        duration_secs: u64,
    },
    Slowloris {
        connections: usize,
        headers_per_second: f64,
        duration_secs: u64,
    },
    SlowPost {
        connections: usize,
        bytes_per_second: usize,
        payload_size: usize,
    },
    RequestFlood {
        target_rps: usize,
        duration_secs: u64,
    },
    LargePayload {
        size_mb: usize,
        concurrent: usize,
        duration_secs: u64,
    },
    PipelineAbuse {
        requests_per_connection: usize,
        concurrent_connections: usize,
    },
    SlowRead {
        connections: usize,
        read_rate_bps: usize,
        duration_secs: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    pub confirmed: bool,
    pub target_owner: Option<String>,
    pub authorization_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyLimits {
    /// Maximum connections per second (None = unlimited)
    pub max_connections_per_second: Option<usize>,

    /// Maximum requests per second (None = unlimited)
    pub max_requests_per_second: Option<usize>,

    /// Maximum payload size in MB (None = unlimited)
    pub max_payload_size_mb: Option<usize>,

    /// Maximum concurrent connections (None = unlimited)
    pub max_concurrent_connections: Option<usize>,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            max_connections_per_second: None,
            max_requests_per_second: None,
            max_payload_size_mb: None,
            max_concurrent_connections: None,
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
    pub targets: Option<TargetGroup>,
    pub pattern: TrafficPattern,
    pub stress_pattern: Option<StressPattern>,
    pub authorization: Option<AuthorizationConfig>,
    pub safety_limits: SafetyLimits,
    pub client: ClientConfig,
    pub output: OutputConfig,
    pub verbose: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionMode {
    SingleTarget,
    MultiTarget,
    StressTest,
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
        // Check if we have either a single target or multi-target configuration
        let has_single_target = !config.target.url.is_empty();
        let has_multi_target = config.targets.is_some() &&
            config.targets.as_ref().unwrap().targets.iter().any(|t| !t.url.is_empty());
        let has_stress_pattern = config.stress_pattern.is_some();

        // For stress tests, require single target
        if has_stress_pattern && !has_single_target {
            anyhow::bail!("Stress testing requires a single target URL. Specify with --url or target.url in config file.");
        }

        // For normal tests, require either single target or multi-target
        if !has_stress_pattern && !has_single_target && !has_multi_target {
            anyhow::bail!("Target URL is required. Specify with --url, target.url, or targets section in config file.");
        }

        let result = Config {
            target: config.target,
            targets: config.targets,
            pattern: config.pattern,
            stress_pattern: config.stress_pattern,
            authorization: config.authorization,
            safety_limits: config.safety_limits,
            client: config.client,
            output: config.output,
            verbose: args.verbose,
        };

        // Validate stress testing authorization and safety limits
        result.validate_stress_authorization()?;

        Ok(result)
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

    pub fn get_execution_mode(&self) -> ExecutionMode {
        if self.stress_pattern.is_some() {
            ExecutionMode::StressTest
        } else if self.targets.is_some() {
            ExecutionMode::MultiTarget
        } else {
            ExecutionMode::SingleTarget
        }
    }

    pub fn validate_stress_authorization(&self) -> Result<()> {
        if self.stress_pattern.is_some() {
            match &self.authorization {
                Some(auth) if auth.confirmed => {
                    // Validate against safety limits if configured
                    self.validate_safety_limits()?;
                    Ok(())
                }
                Some(_) => anyhow::bail!(
                    "Stress testing requires authorization.confirmed to be true. \
                    Set authorization.confirmed: true in your config file."
                ),
                None => anyhow::bail!(
                    "Stress testing requires authorization configuration. \
                    Add an 'authorization' section with 'confirmed: true' to your config file."
                ),
            }
        } else {
            Ok(())
        }
    }

    fn validate_safety_limits(&self) -> Result<()> {
        if let Some(ref pattern) = self.stress_pattern {
            match pattern {
                StressPattern::ConnectionFlood { connections_per_second, .. } => {
                    if let Some(max) = self.safety_limits.max_connections_per_second {
                        if *connections_per_second > max {
                            anyhow::bail!(
                                "Connection rate {} exceeds safety limit of {} conn/s. \
                                Adjust your config or increase safety_limits.max_connections_per_second",
                                connections_per_second, max
                            );
                        }
                    }
                }
                StressPattern::RequestFlood { target_rps, .. } => {
                    if let Some(max) = self.safety_limits.max_requests_per_second {
                        if *target_rps > max {
                            anyhow::bail!(
                                "Request rate {} exceeds safety limit of {} req/s. \
                                Adjust your config or increase safety_limits.max_requests_per_second",
                                target_rps, max
                            );
                        }
                    }
                }
                StressPattern::LargePayload { size_mb, .. } => {
                    if let Some(max) = self.safety_limits.max_payload_size_mb {
                        if *size_mb > max {
                            anyhow::bail!(
                                "Payload size {} MB exceeds safety limit of {} MB. \
                                Adjust your config or increase safety_limits.max_payload_size_mb",
                                size_mb, max
                            );
                        }
                    }
                }
                StressPattern::Slowloris { connections, .. }
                | StressPattern::SlowPost { connections, .. }
                | StressPattern::SlowRead { connections, .. } => {
                    if let Some(max) = self.safety_limits.max_concurrent_connections {
                        if *connections > max {
                            anyhow::bail!(
                                "Concurrent connections {} exceeds safety limit of {}. \
                                Adjust your config or increase safety_limits.max_concurrent_connections",
                                connections, max
                            );
                        }
                    }
                }
                StressPattern::PipelineAbuse { concurrent_connections, .. } => {
                    if let Some(max) = self.safety_limits.max_concurrent_connections {
                        if *concurrent_connections > max {
                            anyhow::bail!(
                                "Concurrent connections {} exceeds safety limit of {}. \
                                Adjust your config or increase safety_limits.max_concurrent_connections",
                                concurrent_connections, max
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
