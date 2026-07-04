//! Integration tests for traffic pattern configuration and validation.

use http_traffic_sim::config::TrafficPattern;

#[test]
fn test_fixed_pattern_creation() {
    let pattern = TrafficPattern::Fixed {
        concurrent: 50,
        duration_secs: Some(60),
        total_requests: None,
    };

    match pattern {
        TrafficPattern::Fixed {
            concurrent,
            duration_secs,
            total_requests,
        } => {
            assert_eq!(concurrent, 50);
            assert_eq!(duration_secs, Some(60));
            assert_eq!(total_requests, None);
        }
        _ => panic!("Expected Fixed pattern"),
    }
}

#[test]
fn test_fixed_pattern_with_request_limit() {
    let pattern = TrafficPattern::Fixed {
        concurrent: 10,
        duration_secs: None,
        total_requests: Some(1000),
    };

    match pattern {
        TrafficPattern::Fixed {
            concurrent,
            duration_secs,
            total_requests,
        } => {
            assert_eq!(concurrent, 10);
            assert_eq!(duration_secs, None);
            assert_eq!(total_requests, Some(1000));
        }
        _ => panic!("Expected Fixed pattern"),
    }
}

#[test]
fn test_fixed_pattern_with_both_limits() {
    let pattern = TrafficPattern::Fixed {
        concurrent: 25,
        duration_secs: Some(30),
        total_requests: Some(500),
    };

    match pattern {
        TrafficPattern::Fixed {
            concurrent,
            duration_secs,
            total_requests,
        } => {
            assert_eq!(concurrent, 25);
            assert_eq!(duration_secs, Some(30));
            assert_eq!(total_requests, Some(500));
        }
        _ => panic!("Expected Fixed pattern"),
    }
}

#[test]
fn test_rate_limit_pattern_creation() {
    let pattern = TrafficPattern::RateLimit {
        rate: 100,
        duration_secs: Some(60),
        total_requests: None,
    };

    match pattern {
        TrafficPattern::RateLimit {
            rate,
            duration_secs,
            total_requests,
        } => {
            assert_eq!(rate, 100);
            assert_eq!(duration_secs, Some(60));
            assert_eq!(total_requests, None);
        }
        _ => panic!("Expected RateLimit pattern"),
    }
}

#[test]
fn test_rate_limit_pattern_with_request_limit() {
    let pattern = TrafficPattern::RateLimit {
        rate: 50,
        duration_secs: None,
        total_requests: Some(2000),
    };

    match pattern {
        TrafficPattern::RateLimit {
            rate,
            duration_secs,
            total_requests,
        } => {
            assert_eq!(rate, 50);
            assert_eq!(duration_secs, None);
            assert_eq!(total_requests, Some(2000));
        }
        _ => panic!("Expected RateLimit pattern"),
    }
}

#[test]
fn test_ramp_pattern_creation() {
    let pattern = TrafficPattern::Ramp {
        from: 10,
        to: 100,
        ramp_duration_secs: 60,
        hold_duration_secs: Some(30),
    };

    match pattern {
        TrafficPattern::Ramp {
            from,
            to,
            ramp_duration_secs,
            hold_duration_secs,
        } => {
            assert_eq!(from, 10);
            assert_eq!(to, 100);
            assert_eq!(ramp_duration_secs, 60);
            assert_eq!(hold_duration_secs, Some(30));
        }
        _ => panic!("Expected Ramp pattern"),
    }
}

#[test]
fn test_ramp_pattern_without_hold() {
    let pattern = TrafficPattern::Ramp {
        from: 5,
        to: 50,
        ramp_duration_secs: 120,
        hold_duration_secs: None,
    };

    match pattern {
        TrafficPattern::Ramp {
            from,
            to,
            ramp_duration_secs,
            hold_duration_secs,
        } => {
            assert_eq!(from, 5);
            assert_eq!(to, 50);
            assert_eq!(ramp_duration_secs, 120);
            assert_eq!(hold_duration_secs, None);
        }
        _ => panic!("Expected Ramp pattern"),
    }
}

#[test]
fn test_ramp_pattern_edge_case_same_from_to() {
    let pattern = TrafficPattern::Ramp {
        from: 50,
        to: 50,
        ramp_duration_secs: 10,
        hold_duration_secs: Some(60),
    };

    match pattern {
        TrafficPattern::Ramp { from, to, .. } => {
            assert_eq!(from, to);
        }
        _ => panic!("Expected Ramp pattern"),
    }
}

#[test]
fn test_burst_pattern_creation() {
    let pattern = TrafficPattern::Burst {
        size: 100,
        interval_secs: 5,
        duration_secs: Some(60),
        total_bursts: None,
    };

    match pattern {
        TrafficPattern::Burst {
            size,
            interval_secs,
            duration_secs,
            total_bursts,
        } => {
            assert_eq!(size, 100);
            assert_eq!(interval_secs, 5);
            assert_eq!(duration_secs, Some(60));
            assert_eq!(total_bursts, None);
        }
        _ => panic!("Expected Burst pattern"),
    }
}

#[test]
fn test_burst_pattern_with_burst_limit() {
    let pattern = TrafficPattern::Burst {
        size: 50,
        interval_secs: 10,
        duration_secs: None,
        total_bursts: Some(20),
    };

    match pattern {
        TrafficPattern::Burst {
            size,
            interval_secs,
            duration_secs,
            total_bursts,
        } => {
            assert_eq!(size, 50);
            assert_eq!(interval_secs, 10);
            assert_eq!(duration_secs, None);
            assert_eq!(total_bursts, Some(20));
        }
        _ => panic!("Expected Burst pattern"),
    }
}

#[test]
fn test_burst_pattern_high_frequency() {
    let pattern = TrafficPattern::Burst {
        size: 1000,
        interval_secs: 1, // Every second
        duration_secs: Some(300),
        total_bursts: None,
    };

    match pattern {
        TrafficPattern::Burst {
            size,
            interval_secs,
            ..
        } => {
            assert_eq!(size, 1000);
            assert_eq!(interval_secs, 1);
        }
        _ => panic!("Expected Burst pattern"),
    }
}

#[test]
fn test_pattern_cloning() {
    let pattern = TrafficPattern::Fixed {
        concurrent: 50,
        duration_secs: Some(60),
        total_requests: None,
    };

    let cloned = pattern.clone();

    match (&pattern, &cloned) {
        (
            TrafficPattern::Fixed { concurrent: c1, .. },
            TrafficPattern::Fixed { concurrent: c2, .. },
        ) => {
            assert_eq!(c1, c2);
        }
        _ => panic!("Pattern type mismatch after cloning"),
    }
}

#[test]
fn test_pattern_debug_format() {
    let pattern = TrafficPattern::Fixed {
        concurrent: 50,
        duration_secs: Some(60),
        total_requests: None,
    };

    let debug_str = format!("{:?}", pattern);
    assert!(debug_str.contains("Fixed"));
    assert!(debug_str.contains("50"));
}

#[test]
fn test_various_concurrency_levels() {
    let concurrency_levels = vec![1, 10, 50, 100, 500, 1000];

    for level in concurrency_levels {
        let pattern = TrafficPattern::Fixed {
            concurrent: level,
            duration_secs: Some(60),
            total_requests: None,
        };

        match pattern {
            TrafficPattern::Fixed { concurrent, .. } => {
                assert_eq!(concurrent, level);
            }
            _ => panic!("Expected Fixed pattern"),
        }
    }
}

#[test]
fn test_various_rate_limits() {
    let rates = vec![1, 10, 100, 1000, 10000];

    for rate in rates {
        let pattern = TrafficPattern::RateLimit {
            rate,
            duration_secs: Some(60),
            total_requests: None,
        };

        match pattern {
            TrafficPattern::RateLimit { rate: r, .. } => {
                assert_eq!(r, rate);
            }
            _ => panic!("Expected RateLimit pattern"),
        }
    }
}

#[test]
fn test_ramp_various_durations() {
    let durations = vec![10, 30, 60, 120, 300];

    for duration in durations {
        let pattern = TrafficPattern::Ramp {
            from: 10,
            to: 100,
            ramp_duration_secs: duration,
            hold_duration_secs: None,
        };

        match pattern {
            TrafficPattern::Ramp {
                ramp_duration_secs, ..
            } => {
                assert_eq!(ramp_duration_secs, duration);
            }
            _ => panic!("Expected Ramp pattern"),
        }
    }
}

#[test]
fn test_burst_various_sizes() {
    let sizes = vec![10, 50, 100, 500, 1000];

    for size in sizes {
        let pattern = TrafficPattern::Burst {
            size,
            interval_secs: 5,
            duration_secs: Some(60),
            total_bursts: None,
        };

        match pattern {
            TrafficPattern::Burst { size: s, .. } => {
                assert_eq!(s, size);
            }
            _ => panic!("Expected Burst pattern"),
        }
    }
}

#[test]
fn test_burst_various_intervals() {
    let intervals = vec![1, 5, 10, 30, 60];

    for interval in intervals {
        let pattern = TrafficPattern::Burst {
            size: 100,
            interval_secs: interval,
            duration_secs: Some(300),
            total_bursts: None,
        };

        match pattern {
            TrafficPattern::Burst { interval_secs, .. } => {
                assert_eq!(interval_secs, interval);
            }
            _ => panic!("Expected Burst pattern"),
        }
    }
}

#[test]
fn test_pattern_default_is_fixed() {
    let pattern = TrafficPattern::default();

    match pattern {
        TrafficPattern::Fixed { .. } => {
            // Success - default is Fixed
        }
        _ => panic!("Default pattern should be Fixed"),
    }
}

#[test]
fn test_short_duration_patterns() {
    // Test patterns with very short durations (useful for testing)
    let pattern = TrafficPattern::Fixed {
        concurrent: 10,
        duration_secs: Some(1), // 1 second
        total_requests: None,
    };

    match pattern {
        TrafficPattern::Fixed { duration_secs, .. } => {
            assert_eq!(duration_secs, Some(1));
        }
        _ => panic!("Expected Fixed pattern"),
    }
}

#[test]
fn test_long_duration_patterns() {
    // Test patterns with long durations (stress tests)
    let pattern = TrafficPattern::Fixed {
        concurrent: 50,
        duration_secs: Some(3600), // 1 hour
        total_requests: None,
    };

    match pattern {
        TrafficPattern::Fixed { duration_secs, .. } => {
            assert_eq!(duration_secs, Some(3600));
        }
        _ => panic!("Expected Fixed pattern"),
    }
}

#[test]
fn test_ramp_up_vs_ramp_down() {
    // Test ramp up (from < to)
    let ramp_up = TrafficPattern::Ramp {
        from: 10,
        to: 100,
        ramp_duration_secs: 60,
        hold_duration_secs: None,
    };

    // Test ramp down (from > to)
    let ramp_down = TrafficPattern::Ramp {
        from: 100,
        to: 10,
        ramp_duration_secs: 60,
        hold_duration_secs: None,
    };

    match (&ramp_up, &ramp_down) {
        (
            TrafficPattern::Ramp {
                from: f1, to: t1, ..
            },
            TrafficPattern::Ramp {
                from: f2, to: t2, ..
            },
        ) => {
            assert!(f1 < t1); // Ramp up
            assert!(f2 > t2); // Ramp down
        }
        _ => panic!("Expected Ramp patterns"),
    }
}

// Note: Actual pattern execution would require running the full system
// These tests focus on pattern configuration and validation
