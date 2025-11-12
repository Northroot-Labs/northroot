#!/bin/bash
# Golden vector checker
# Verifies all golden vectors are referenced in tests

set -e

echo "🔍 Checking golden vectors..."
echo ""

VECTORS_DIR="vectors"
if [ ! -d "$VECTORS_DIR" ]; then
    echo "⚠️  vectors/ directory not found"
    exit 1
fi

# Find all vector files
VECTORS=$(find "$VECTORS_DIR" -name "*.json" -o -name "*.cbor" | sort)

if [ -z "$VECTORS" ]; then
    echo "⚠️  No vector files found in $VECTORS_DIR"
    exit 1
fi

MISSING=0
FOUND=0

for vec in $VECTORS; do
    VEC_NAME=$(basename "$vec")
    VEC_RELATIVE=$(echo "$vec" | sed "s|^$VECTORS_DIR/||")
    
    # Check if referenced in tests
    if grep -r "$VEC_NAME\|$VEC_RELATIVE" crates/*/tests/ > /dev/null 2>&1; then
        echo "✅ $vec"
        FOUND=$((FOUND + 1))
    else
        echo "⚠️  $vec (not referenced in tests)"
        MISSING=$((MISSING + 1))
    fi
done

echo ""
echo "📊 Summary:"
echo "   Found: $FOUND"
echo "   Missing: $MISSING"

if [ $MISSING -gt 0 ]; then
    echo ""
    echo "⚠️  Some vectors are not referenced in tests"
    exit 1
else
    echo ""
    echo "✅ All vectors are referenced in tests"
fi

