#!/bin/bash
# Run quickstart example with proper environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check if venv exists
if [ ! -d "venv" ]; then
    echo "❌ Virtual environment not found!"
    echo "Run ./setup-dev.sh first to set up the environment"
    exit 1
fi

# Activate virtual environment
source venv/bin/activate

# Check if package is installed
if ! python -c "import northroot" 2>/dev/null; then
    echo "Package not installed. Installing in development mode..."
    maturin develop
fi

# Run the example
echo "Running quickstart example..."
python examples/quickstart.py

