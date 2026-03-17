#!/bin/bash
# Setup git hooks for http-traffic-sim development

set -e

echo "Setting up git hooks..."

# Get the repository root
REPO_ROOT=$(git rev-parse --show-toplevel)

# Configure git to use .githooks directory
git config core.hooksPath .githooks

# Make hooks executable
chmod +x "$REPO_ROOT/.githooks/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs formatting, linting, and tests"
echo ""
echo "To skip hooks temporarily, use: git commit --no-verify"
echo ""
