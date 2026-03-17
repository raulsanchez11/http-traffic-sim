//! Target selection for multi-target load testing.
//!
//! This module manages target selection across multiple targets using different
//! load distribution strategies:
//!
//! - **Round-robin**: Distributes load evenly across targets in order
//! - **Weighted**: Distributes load based on configured weights
//! - **Random**: Selects targets randomly for each request
//! - **Hash**: Consistent hashing based on request fields (planned)
//!
//! # Thread Safety
//!
//! The target selector is thread-safe and can be shared across multiple
//! async tasks using `Arc`. Internal state is managed with atomic operations
//! for lock-free performance.
//!
//! # Examples
//!
//! ```rust,no_run
//! use http_traffic_sim::target_selector::TargetSelector;
//! use http_traffic_sim::config::{TargetConfig, LoadDistribution};
//! use std::sync::Arc;
//!
//! # fn example() {
//! let targets = vec![
//!     Arc::new(TargetConfig::default()),
//!     Arc::new(TargetConfig::default()),
//! ];
//!
//! let selector = TargetSelector::new(
//!     targets,
//!     LoadDistribution::RoundRobin
//! );
//!
//! // Select targets in round-robin fashion
//! let target1 = selector.select();
//! let target2 = selector.select();
//! # }
//! ```

use rand::Rng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::config::{LoadDistribution, TargetConfig};

/// Manages target selection for multi-target load testing.
///
/// Implements various load distribution strategies to distribute traffic
/// across multiple targets. Thread-safe for concurrent use.
///
/// # Distribution Strategies
///
/// - **RoundRobin**: Sequential selection, distributes evenly
/// - **Weighted**: Probabilistic selection based on weights
/// - **Random**: Uniform random selection
/// - **Hash**: Consistent hashing (falls back to round-robin for now)
///
/// # Examples
///
/// ```rust,no_run
/// use http_traffic_sim::target_selector::TargetSelector;
/// use http_traffic_sim::config::{TargetConfig, LoadDistribution};
/// use std::sync::Arc;
///
/// # fn example() {
/// let targets = vec![
///     Arc::new(TargetConfig::default()),
///     Arc::new(TargetConfig::default()),
/// ];
///
/// // Round-robin distribution
/// let selector = TargetSelector::new(
///     targets,
///     LoadDistribution::RoundRobin
/// );
///
/// let target = selector.select();
/// # }
/// ```
pub struct TargetSelector {
    targets: Vec<Arc<TargetConfig>>,
    distribution: LoadDistribution,
    counter: AtomicUsize,
}

impl TargetSelector {
    /// Creates a new target selector with the specified distribution strategy.
    ///
    /// # Arguments
    ///
    /// * `targets` - List of target configurations to select from
    /// * `distribution` - Load distribution strategy to use
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::target_selector::TargetSelector;
    /// use http_traffic_sim::config::{TargetConfig, LoadDistribution};
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let targets = vec![
    ///     Arc::new(TargetConfig::default()),
    ///     Arc::new(TargetConfig::default()),
    /// ];
    ///
    /// // Create round-robin selector
    /// let selector = TargetSelector::new(
    ///     targets.clone(),
    ///     LoadDistribution::RoundRobin
    /// );
    ///
    /// // Create weighted selector
    /// let weighted = TargetSelector::new(
    ///     targets,
    ///     LoadDistribution::Weighted { weights: vec![0.7, 0.3] }
    /// );
    /// # }
    /// ```
    pub fn new(targets: Vec<Arc<TargetConfig>>, distribution: LoadDistribution) -> Self {
        Self {
            targets,
            distribution,
            counter: AtomicUsize::new(0),
        }
    }

    /// Selects a target based on the configured distribution strategy.
    ///
    /// This method is thread-safe and can be called concurrently from
    /// multiple tasks. The selection algorithm depends on the configured
    /// distribution strategy.
    ///
    /// # Returns
    ///
    /// An `Arc<TargetConfig>` for the selected target. The Arc allows
    /// efficient sharing of the target configuration across tasks.
    ///
    /// # Distribution Behavior
    ///
    /// - **RoundRobin**: Returns targets sequentially, cycling through the list
    /// - **Weighted**: Returns targets with probability proportional to weights
    /// - **Random**: Returns a uniformly random target
    /// - **Hash**: Currently falls back to round-robin (full implementation pending)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_traffic_sim::target_selector::TargetSelector;
    /// use http_traffic_sim::config::{TargetConfig, LoadDistribution};
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// # let targets = vec![Arc::new(TargetConfig::default())];
    /// let selector = TargetSelector::new(
    ///     targets,
    ///     LoadDistribution::RoundRobin
    /// );
    ///
    /// // Select target for each request
    /// let target = selector.select();
    /// println!("Selected target: {}", target.id);
    /// # }
    /// ```
    pub fn select(&self) -> Arc<TargetConfig> {
        match &self.distribution {
            LoadDistribution::RoundRobin => self.round_robin(),
            LoadDistribution::Weighted { weights } => self.weighted(weights),
            LoadDistribution::Random => self.random(),
            LoadDistribution::Hash { field: _ } => {
                // For now, hash-based routing falls back to round-robin
                // Full implementation would hash request fields
                self.round_robin()
            }
        }
    }

    fn round_robin(&self) -> Arc<TargetConfig> {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.targets.len();
        self.targets[idx].clone()
    }

    fn weighted(&self, weights: &[f64]) -> Arc<TargetConfig> {
        let mut rng = rand::thread_rng();
        let random: f64 = rng.gen();

        let total: f64 = weights.iter().sum();
        let mut cumulative = 0.0;

        for (idx, weight) in weights.iter().enumerate() {
            cumulative += weight / total;
            if random < cumulative {
                return self.targets[idx.min(self.targets.len() - 1)].clone();
            }
        }

        // Fallback to last target
        self.targets.last().unwrap().clone()
    }

    fn random(&self) -> Arc<TargetConfig> {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.targets.len());
        self.targets[idx].clone()
    }

    #[allow(dead_code)]
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}
