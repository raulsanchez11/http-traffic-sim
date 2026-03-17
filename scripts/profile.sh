#!/bin/bash
# Profiling script for http-traffic-sim using flamegraph

set -e

echo "🔥 Profiling with flamegraph..."
echo ""

# Check if flamegraph is installed
if ! command -v flamegraph &> /dev/null; then
    echo "Installing flamegraph..."
    cargo install flamegraph
fi

# Check for arguments
if [ $# -eq 0 ]; then
    echo "❌ Error: Please provide test configuration"
    echo ""
    echo "Usage:"
    echo "  ./scripts/profile.sh --url https://example.com --concurrent 100 --duration 30"
    echo ""
    exit 1
fi

# Run profiling
echo "Running load test with profiling..."
echo "Command: cargo flamegraph -- $@"
echo ""

# Run flamegraph
sudo cargo flamegraph -- "$@"

echo ""
echo "✅ Profiling complete!"
echo ""
echo "Flamegraph saved to: flamegraph.svg"
echo "Open with: open flamegraph.svg (macOS) or xdg-open flamegraph.svg (Linux)"
echo ""
