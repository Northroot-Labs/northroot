#!/bin/bash
# Coverage diff generator for refactor phases
# Usage: ./scripts/coverage-diff.sh <crate-name>

set -e

CRATE=$1
if [ -z "$CRATE" ]; then
    echo "Usage: $0 <crate-name>"
    echo "Example: $0 northroot-engine"
    exit 1
fi

echo "📊 Generating coverage baseline for $CRATE..."
cargo llvm-cov --package $CRATE --lib --lcov --output-path coverage-before.lcov 2>/dev/null || {
    echo "⚠️  cargo llvm-cov not installed. Install with: cargo install cargo-llvm-cov"
    exit 1
}

echo ""
echo "✅ Coverage baseline saved to coverage-before.lcov"
echo ""
echo "👉 Now run your refactor, then press Enter to generate diff..."
read

echo ""
echo "📊 Generating coverage after refactor..."
cargo llvm-cov --package $CRATE --lib --lcov --output-path coverage-after.lcov

echo ""
echo "📈 Generating diff..."
diff coverage-before.lcov coverage-after.lcov > coverage-diff.txt || true

if [ -s coverage-diff.txt ]; then
    echo "✅ Coverage diff saved to coverage-diff.txt"
    echo ""
    echo "📋 Review the diff for any regressions:"
    echo "   cat coverage-diff.txt"
else
    echo "✅ No coverage changes detected"
    rm coverage-diff.txt
fi

echo ""
echo "🧹 Cleaning up..."
rm -f coverage-before.lcov coverage-after.lcov

