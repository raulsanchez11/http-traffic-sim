#!/bin/bash
# Benchmark script for http-traffic-sim

set -e

echo "🏃 Running benchmarks..."
echo ""

# Check if criterion is installed
if ! cargo bench --help | grep -q "criterion"; then
    echo "Installing criterion..."
    cargo install criterion
fi

# Run benchmarks
if [ "$1" == "--baseline" ]; then
    echo "Saving baseline..."
    cargo bench -- --save-baseline "$2"
elif [ "$1" == "--compare" ]; then
    echo "Comparing against baseline: $2"
    cargo bench -- --baseline "$2"
else
    echo "Running all benchmarks..."
    cargo bench
fi

echo ""
echo "✅ Benchmarks complete!"
echo ""
echo "Results saved in: target/criterion/"
echo ""
echo "Usage:"
echo "  ./scripts/bench.sh                    # Run all benchmarks"
echo "  ./scripts/bench.sh --baseline main    # Save baseline as 'main'"
echo "  ./scripts/bench.sh --compare main     # Compare against 'main' baseline"
echo ""
