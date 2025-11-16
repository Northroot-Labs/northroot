#!/bin/bash
# Local test script for PyPI publishing
# Tests the build and publish process without actually publishing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🧪 Testing PyPI publish process locally"
echo ""

# Check if we're in the right directory
if [ ! -f "pyproject.toml" ]; then
    echo "❌ Error: Must run from sdk/python/northroot directory"
    exit 1
fi

# Check for maturin
if ! command -v maturin &> /dev/null; then
    echo "Installing maturin..."
    pip install maturin
fi

# Check for twine
if ! command -v twine &> /dev/null; then
    echo "Installing twine..."
    pip install twine
fi

echo "1. Building package..."
maturin build --release --out dist

echo ""
echo "2. Checking build artifacts..."
if [ -d "dist" ]; then
    echo "✅ dist/ directory exists"
    ls -lah dist/
else
    echo "❌ dist/ directory not found"
    exit 1
fi

WHEEL_FILE=$(ls dist/*.whl 2>/dev/null | head -1)
SDIST_FILE=$(ls dist/*.tar.gz 2>/dev/null | head -1)

if [ -z "$WHEEL_FILE" ]; then
    echo "❌ No wheel file found in dist/"
    exit 1
fi

echo "✅ Found wheel: $WHEEL_FILE"
if [ -n "$SDIST_FILE" ]; then
    echo "✅ Found source distribution: $SDIST_FILE"
fi

echo ""
echo "3. Testing wheel installation..."
pip install --force-reinstall "$WHEEL_FILE" > /dev/null 2>&1
python -c "from northroot import Client; print('✅ Package imports successfully')"

echo ""
echo "4. Checking package metadata..."
python -c "
import importlib.metadata
try:
    meta = importlib.metadata.metadata('northroot')
    print(f'✅ Package name: {meta[\"Name\"]}')
    print(f'✅ Version: {meta[\"Version\"]}')
    print(f'✅ Description: {meta[\"Summary\"]}')
except Exception as e:
    print(f'⚠️  Could not read metadata: {e}')
"

echo ""
echo "5. Testing twine check (validates package without uploading)..."
if [ -n "$SDIST_FILE" ]; then
    twine check "$WHEEL_FILE" "$SDIST_FILE"
else
    twine check "$WHEEL_FILE"
fi

echo ""
echo "✅ All local tests passed!"
echo ""
echo "To actually publish to TestPyPI, run:"
echo "  export TWINE_USERNAME=__token__"
echo "  export TWINE_PASSWORD='your-testpypi-token'"
echo "  twine upload --repository testpypi dist/*"
echo ""
echo "Or use the GitHub Actions workflow for automated publishing."

