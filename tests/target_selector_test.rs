//! Integration tests for target selection strategies.

use http_traffic_sim::config::{LoadDistribution, TargetConfig, HashField};
use http_traffic_sim::target_selector::TargetSelector;
use std::collections::HashMap;
use std::sync::Arc;

fn create_test_targets(count: usize) -> Vec<Arc<TargetConfig>> {
    (0..count)
        .map(|i| {
            Arc::new(TargetConfig {
                id: format!("target-{}", i),
                url: format!("https://example{}.com", i),
                method: "GET".to_string(),
                headers: HashMap::new(),
                body: None,
                discovery: None,
            })
        })
        .collect()
}

#[test]
fn test_round_robin_selection() {
    let targets = create_test_targets(3);
    let selector = TargetSelector::new(targets.clone(), LoadDistribution::RoundRobin);

    // Select 6 times and verify round-robin pattern
    let selections: Vec<String> = (0..6).map(|_| selector.select().id.clone()).collect();

    assert_eq!(selections[0], "target-0");
    assert_eq!(selections[1], "target-1");
    assert_eq!(selections[2], "target-2");
    assert_eq!(selections[3], "target-0"); // Cycles back
    assert_eq!(selections[4], "target-1");
    assert_eq!(selections[5], "target-2");
}

#[test]
fn test_round_robin_with_single_target() {
    let targets = create_test_targets(1);
    let selector = TargetSelector::new(targets.clone(), LoadDistribution::RoundRobin);

    // All selections should return the same target
    for _ in 0..10 {
        let selected = selector.select();
        assert_eq!(selected.id, "target-0");
    }
}

#[test]
fn test_random_selection_distribution() {
    let targets = create_test_targets(3);
    let selector = TargetSelector::new(targets.clone(), LoadDistribution::Random);

    // Select many times and verify all targets are eventually selected
    let mut counts = HashMap::new();
    for _ in 0..300 {
        let selected = selector.select();
        *counts.entry(selected.id.clone()).or_insert(0) += 1;
    }

    // All targets should be selected at least once
    assert!(counts.contains_key("target-0"));
    assert!(counts.contains_key("target-1"));
    assert!(counts.contains_key("target-2"));

    // With 300 selections and 3 targets, expect roughly 100 each
    // Allow 50-150 range for randomness (should be within this with high probability)
    for (_target, count) in counts.iter() {
        assert!(*count >= 50 && *count <= 150, "Count {} outside expected range", count);
    }
}

#[test]
fn test_weighted_selection_equal_weights() {
    let targets = create_test_targets(3);
    let weights = vec![1.0, 1.0, 1.0]; // Equal weights
    let selector = TargetSelector::new(
        targets.clone(),
        LoadDistribution::Weighted { weights },
    );

    let mut counts = HashMap::new();
    for _ in 0..300 {
        let selected = selector.select();
        *counts.entry(selected.id.clone()).or_insert(0) += 1;
    }

    // With equal weights, should be roughly balanced
    for (_target, count) in counts.iter() {
        assert!(*count >= 70 && *count <= 130, "Count {} outside expected range", count);
    }
}

#[test]
fn test_weighted_selection_skewed_weights() {
    let targets = create_test_targets(3);
    let weights = vec![0.7, 0.2, 0.1]; // 70%, 20%, 10%
    let selector = TargetSelector::new(
        targets.clone(),
        LoadDistribution::Weighted { weights },
    );

    let mut counts = HashMap::new();
    for _ in 0..1000 {
        let selected = selector.select();
        *counts.entry(selected.id.clone()).or_insert(0) += 1;
    }

    // target-0 should get ~70% (650-750)
    // target-1 should get ~20% (150-250)
    // target-2 should get ~10% (50-150)
    let count_0 = *counts.get("target-0").unwrap();
    let count_1 = *counts.get("target-1").unwrap();
    let count_2 = *counts.get("target-2").unwrap();

    assert!(count_0 >= 650 && count_0 <= 750, "Target 0 count {} outside expected range", count_0);
    assert!(count_1 >= 150 && count_1 <= 250, "Target 1 count {} outside expected range", count_1);
    assert!(count_2 >= 50 && count_2 <= 150, "Target 2 count {} outside expected range", count_2);
}

#[test]
fn test_weighted_selection_with_zero_weight() {
    let targets = create_test_targets(3);
    let weights = vec![1.0, 0.0, 1.0]; // Middle target has zero weight
    let selector = TargetSelector::new(
        targets.clone(),
        LoadDistribution::Weighted { weights },
    );

    let mut counts = HashMap::new();
    for _ in 0..200 {
        let selected = selector.select();
        *counts.entry(selected.id.clone()).or_insert(0) += 1;
    }

    // target-1 should rarely or never be selected
    let count_1 = counts.get("target-1").unwrap_or(&0);
    assert!(*count_1 < 10, "Target with zero weight selected {} times", count_1);

    // target-0 and target-2 should split the load
    let count_0 = *counts.get("target-0").unwrap();
    let count_2 = *counts.get("target-2").unwrap();
    assert!(count_0 >= 80 && count_0 <= 120);
    assert!(count_2 >= 80 && count_2 <= 120);
}

#[test]
fn test_hash_based_selection_fallback() {
    // Hash-based currently falls back to round-robin
    let targets = create_test_targets(3);
    let selector = TargetSelector::new(
        targets.clone(),
        LoadDistribution::Hash { field: HashField::SourceIp },
    );

    // Should behave like round-robin
    let selections: Vec<String> = (0..6).map(|_| selector.select().id.clone()).collect();

    assert_eq!(selections[0], "target-0");
    assert_eq!(selections[1], "target-1");
    assert_eq!(selections[2], "target-2");
    assert_eq!(selections[3], "target-0");
}

#[test]
fn test_selector_with_many_targets() {
    let targets = create_test_targets(10);
    let selector = TargetSelector::new(targets.clone(), LoadDistribution::RoundRobin);

    // Verify cycling through all 10 targets
    let mut seen_targets = std::collections::HashSet::new();
    for _ in 0..10 {
        let selected = selector.select();
        seen_targets.insert(selected.id.clone());
    }

    assert_eq!(seen_targets.len(), 10);
}

#[test]
fn test_selector_preserves_target_config() {
    let mut targets = vec![];
    for i in 0..2 {
        let mut headers = HashMap::new();
        headers.insert("X-Target-ID".to_string(), i.to_string());

        targets.push(Arc::new(TargetConfig {
            id: format!("target-{}", i),
            url: format!("https://example{}.com/api", i),
            method: "POST".to_string(),
            headers,
            body: Some(format!(r#"{{"id": {}}}"#, i)),
            discovery: None,
        }));
    }

    let selector = TargetSelector::new(targets.clone(), LoadDistribution::RoundRobin);

    // First selection - target-0
    let selected = selector.select();
    assert_eq!(selected.id, "target-0");
    assert_eq!(selected.url, "https://example0.com/api");
    assert_eq!(selected.method, "POST");
    assert_eq!(selected.headers.get("X-Target-ID").unwrap(), "0");
    assert_eq!(selected.body.as_ref().unwrap(), r#"{"id": 0}"#);

    // Second selection - target-1
    let selected = selector.select();
    assert_eq!(selected.id, "target-1");
    assert_eq!(selected.url, "https://example1.com/api");
    assert_eq!(selected.headers.get("X-Target-ID").unwrap(), "1");
}

#[test]
fn test_weighted_selection_with_non_normalized_weights() {
    let targets = create_test_targets(2);
    let weights = vec![4.0, 1.0]; // Not normalized (sum is 5, not 1)
    let selector = TargetSelector::new(
        targets.clone(),
        LoadDistribution::Weighted { weights },
    );

    let mut counts = HashMap::new();
    for _ in 0..500 {
        let selected = selector.select();
        *counts.entry(selected.id.clone()).or_insert(0) += 1;
    }

    // Should still maintain 4:1 ratio (80% vs 20%)
    let count_0 = *counts.get("target-0").unwrap();
    let count_1 = *counts.get("target-1").unwrap();

    // target-0 should get ~80% (375-425)
    // target-1 should get ~20% (75-125)
    assert!(count_0 >= 375 && count_0 <= 425, "Target 0 count {} outside expected range", count_0);
    assert!(count_1 >= 75 && count_1 <= 125, "Target 1 count {} outside expected range", count_1);
}

#[test]
fn test_concurrent_round_robin_selection() {
    use std::sync::Arc;
    use std::thread;

    let targets = create_test_targets(3);
    let selector = Arc::new(TargetSelector::new(targets.clone(), LoadDistribution::RoundRobin));

    let mut handles = vec![];

    // Spawn multiple threads selecting concurrently
    for _ in 0..10 {
        let selector_clone = Arc::clone(&selector);
        let handle = thread::spawn(move || {
            let mut selections = vec![];
            for _ in 0..10 {
                selections.push(selector_clone.select().id.clone());
            }
            selections
        });
        handles.push(handle);
    }

    // Collect all selections
    let mut all_selections = vec![];
    for handle in handles {
        all_selections.extend(handle.join().unwrap());
    }

    // Verify we got 100 selections total
    assert_eq!(all_selections.len(), 100);

    // Count occurrences - should be roughly balanced
    let mut counts = HashMap::new();
    for selection in all_selections {
        *counts.entry(selection).or_insert(0) += 1;
    }

    // With round-robin and 100 selections, expect ~33 each (allow 25-42)
    for (_target, count) in counts.iter() {
        assert!(*count >= 25 && *count <= 42, "Count {} outside expected range in concurrent test", count);
    }
}

#[test]
fn test_selector_returns_arc() {
    let targets = create_test_targets(2);
    let selector = TargetSelector::new(targets.clone(), LoadDistribution::RoundRobin);

    let selected1 = selector.select();
    let selected2 = selector.select();

    // Verify we can clone the Arc (sharing is efficient)
    let _cloned = selected1.clone();

    // Verify different selections return different targets
    assert_ne!(selected1.id, selected2.id);
}
