#!/bin/bash
# Release readiness check script
# Runs all pre-release checks as specified in RELEASE_GUIDE.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo "=== Northroot Release Readiness Check ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_failed=0

# Function to run a check
run_check() {
    local name="$1"
    shift
    echo -n "Checking $name... "
    if "$@" > /tmp/northroot-release-check.log 2>&1; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        cat /tmp/northroot-release-check.log
        check_failed=1
        return 1
    fi
}

# 1. Formatting check
run_check "code formatting" cargo fmt --all --check || true

# 2. Clippy (all warnings as errors)
run_check "clippy lints" cargo clippy --all-targets --all-features -- -D warnings || true

# 3. All tests
run_check "unit and integration tests" cargo test --all --all-features || true

# 4. CLI-specific tests
run_check "CLI package tests" cargo test --manifest-path apps/northroot/Cargo.toml || true

# 5. v0.1 readiness report
run_check "v0.1 readiness report" python3 scripts/v01_readiness.py --json || true

# 6. Build release binary
echo -n "Building release binary... "
if cargo build --release --manifest-path apps/northroot/Cargo.toml > /tmp/northroot-release-check.log 2>&1; then
    echo -e "${GREEN}✓${NC}"
    # Binary location depends on workspace structure
    if [ -f "apps/northroot/target/release/northroot" ]; then
        BINARY_PATH="apps/northroot/target/release/northroot"
        echo "  Binary location: $BINARY_PATH"
    elif [ -f "target/release/northroot" ]; then
        BINARY_PATH="target/release/northroot"
        echo "  Binary location: $BINARY_PATH"
    else
        echo -e "  ${RED}✗${NC} Binary not found at expected location"
        check_failed=1
        BINARY_PATH=""
    fi
    
    if [ -n "$BINARY_PATH" ]; then
        # Test binary works
        if "$BINARY_PATH" --help > /dev/null 2>&1; then
            echo -e "  ${GREEN}✓${NC} Binary executes correctly"
        else
            echo -e "  ${RED}✗${NC} Binary execution test failed"
            check_failed=1
        fi
    fi
else
    echo -e "${RED}✗${NC}"
    cat /tmp/northroot-release-check.log
    check_failed=1
    BINARY_PATH=""
fi

# 6. Version check (if version flag exists)
if [ -n "$BINARY_PATH" ] && [ -f "$BINARY_PATH" ]; then
    echo -n "Checking CLI version... "
    if "$BINARY_PATH" --version > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        "$BINARY_PATH" --version
    else
        echo -e "${YELLOW}⚠${NC} --version flag not available (optional)"
    fi
    
    # 7. Verify public kernel commands are available
    echo ""
    echo "Verifying public kernel CLI commands:"
    commands=("canonicalize" "event-id" "append" "read" "verify")
    for cmd in "${commands[@]}"; do
        if "$BINARY_PATH" "$cmd" --help > /dev/null 2>&1; then
            echo -e "  ${GREEN}✓${NC} $cmd command available"
        else
            echo -e "  ${RED}✗${NC} $cmd command failed"
            check_failed=1
        fi
    done

    echo -n "Checking removed list command... "
    if "$BINARY_PATH" list --help > /dev/null 2>&1; then
        echo -e "${RED}✗${NC} list command is still available"
        check_failed=1
    else
        echo -e "${GREEN}✓${NC}"
    fi
fi

# Summary
echo ""
echo "=== Summary ==="
if [ $check_failed -eq 0 ]; then
    echo -e "${GREEN}All checks passed! Ready for release.${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed. Review output above.${NC}"
    exit 1
fi
