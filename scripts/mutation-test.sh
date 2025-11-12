#!/bin/bash
# Mutation testing runner
# Usage: ./scripts/mutation-test.sh <crate-name>

set -e

CRATE=$1
if [ -z "$CRATE" ]; then
    echo "Usage: $0 <crate-name>"
    echo "Example: $0 northroot-engine"
    exit 1
fi

# Check if cargo-mutants is installed
if ! command -v cargo-mutants &> /dev/null; then
    echo "⚠️  cargo-mutants not installed."
    echo "   Install with: cargo install cargo-mutants"
    exit 1
fi

echo "🧬 Running mutation testing on $CRATE..."
echo ""

cargo mutants --package $CRATE --lib -- --test-threads=1

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "✅ All mutations caught by tests"
else
    echo ""
    echo "❌ Some mutations survived - improve test coverage"
    exit 1
fi

