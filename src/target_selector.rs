use rand::Rng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::config::{LoadDistribution, TargetConfig};

/// Manages target selection across multiple targets based on distribution strategy
pub struct TargetSelector {
    targets: Vec<Arc<TargetConfig>>,
    distribution: LoadDistribution,
    counter: AtomicUsize,
}

impl TargetSelector {
    pub fn new(targets: Vec<Arc<TargetConfig>>, distribution: LoadDistribution) -> Self {
        Self {
            targets,
            distribution,
            counter: AtomicUsize::new(0),
        }
    }

    /// Select a target based on the configured distribution strategy
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
