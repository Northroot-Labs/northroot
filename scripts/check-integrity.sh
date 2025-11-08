#!/bin/bash
# Integrity check script for critical files
# Prevents accidental modifications to test vectors, baselines, and schemas

set -euo pipefail

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

echo "🔍 Running integrity checks..."

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "⚠️  Not in a git repository, skipping git-based checks"
    exit 0
fi

# Determine base for comparison
if [ "${CI:-}" = "true" ] 2>/dev/null || [ -n "${CI:-}" ]; then
    # In CI, compare against base branch or previous commit
    if [ -n "${GITHUB_BASE_REF:-}" ]; then
        BASE_REF="$GITHUB_BASE_REF"
        git fetch origin "$BASE_REF:$BASE_REF" 2>/dev/null || true
        BASE_SHA="$BASE_REF"
    elif [ -n "${GITHUB_SHA:-}" ]; then
        BASE_SHA="${GITHUB_SHA}~1"
    else
        BASE_SHA="HEAD~1"
    fi
    # In CI, check all changes in the commit
    DIFF_CMD="git diff --name-only $BASE_SHA HEAD"
else
    # Local: check staged changes (pre-commit) or all changes (manual)
    if git diff --cached --quiet 2>/dev/null; then
        # No staged changes, check all changes since HEAD
        BASE_SHA="HEAD"
        DIFF_CMD="git diff --name-only HEAD"
    else
        # Has staged changes, check staged only
        BASE_SHA="HEAD"
        DIFF_CMD="git diff --name-only --cached HEAD"
    fi
fi

# Check for modified test vectors
echo ""
echo "📋 Checking test vectors..."
MODIFIED_VECTORS=$($DIFF_CMD 2>/dev/null | grep "^vectors/" || true)
if [ -n "$MODIFIED_VECTORS" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Test vectors were modified!${NC}"
    echo ""
    echo "Modified files:"
    echo "$MODIFIED_VECTORS" | sed 's/^/  - /'
    echo ""
    echo "Test vectors are golden files. If this is intentional:"
    echo "  1. Ensure drift detection baselines are updated"
    echo "  2. Document the change in commit message"
    echo "  3. Run: cargo test --test test_drift_detection -- --nocapture"
    ((WARNINGS++)) || true
else
    echo -e "${GREEN}✅ No vector modifications detected${NC}"
fi

# Check for modified baseline files
echo ""
echo "📋 Checking baseline values..."
MODIFIED_BASELINES=$($DIFF_CMD 2>/dev/null | \
    grep -E "(test_drift_detection\.rs)" || true)
if [ -n "$MODIFIED_BASELINES" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Baseline files were modified!${NC}"
    echo ""
    echo "Modified files:"
    echo "$MODIFIED_BASELINES" | sed 's/^/  - /'
    echo ""
    echo "If baselines are updated, ensure:"
    echo "  1. Test vectors are also updated (if needed)"
    echo "  2. Change is documented in commit message"
    echo "  3. All drift detection tests pass"
    ((WARNINGS++)) || true
else
    echo -e "${GREEN}✅ No baseline modifications detected${NC}"
fi

# Check for modified schema files
echo ""
echo "📋 Checking schema files..."
MODIFIED_SCHEMAS=$($DIFF_CMD 2>/dev/null | grep "^schemas/" || true)
if [ -n "$MODIFIED_SCHEMAS" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Schema files were modified!${NC}"
    echo ""
    echo "Modified files:"
    echo "$MODIFIED_SCHEMAS" | sed 's/^/  - /'
    echo ""
    echo "Schema changes may require:"
    echo "  1. Test vector regeneration"
    echo "  2. Baseline hash updates"
    echo "  3. Documentation updates"
    echo "  4. Version bump (if breaking)"
    ((WARNINGS++)) || true
else
    echo -e "${GREEN}✅ No schema modifications detected${NC}"
fi

# Check if vectors and baselines are out of sync
echo ""
echo "📋 Checking vector/baseline consistency..."
if [ -n "$MODIFIED_VECTORS" ] && [ -z "$MODIFIED_BASELINES" ]; then
    echo -e "${RED}❌ ERROR: Vectors modified but baselines NOT updated!${NC}"
    echo ""
    echo "If vectors are modified, baseline values MUST be updated:"
    echo "  - receipts: Update BASELINE_HASHES in test_drift_detection.rs"
    echo "  - engine: Update BASELINE_ROOTS in test_drift_detection.rs"
    echo ""
    echo "Run: cargo test --test test_drift_detection -- --nocapture"
    ((ERRORS++)) || true
elif [ -z "$MODIFIED_VECTORS" ] && [ -n "$MODIFIED_BASELINES" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Baselines modified but no vector changes detected${NC}"
    echo "This is OK if you're updating baselines after algorithm changes."
    ((WARNINGS++)) || true
else
    echo -e "${GREEN}✅ Vector/baseline consistency check passed${NC}"
fi

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}❌ Integrity check FAILED with $ERRORS error(s)${NC}"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Integrity check passed with $WARNINGS warning(s)${NC}"
    echo "Review warnings above and ensure changes are intentional."
    exit 0
else
    echo -e "${GREEN}✅ Integrity check passed${NC}"
    exit 0
fi

