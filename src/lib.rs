// Library interface for http-traffic-sim
// Exposes modules for testing, benchmarking, and external use

pub mod authorization;
pub mod client;
pub mod config;
pub mod discovery;
pub mod metrics;
pub mod patterns;
pub mod reporter;
pub mod stats;
pub mod stress;
pub mod target_selector;

// Re-export commonly used types
pub use config::{Config, TargetConfig, TrafficPattern};
pub use discovery::{DiscoveryResult, PortDiscoveryConfig, PortInfo, PortSpec};
pub use metrics::{MetricsCollector, MultiTargetMetrics};
pub use stats::Statistics;
