#!/bin/bash
# Setup script for Northroot Python SDK development environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Setting up Northroot Python SDK development environment"
echo ""

# Check Python version
PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
echo "Python version: $PYTHON_VERSION"

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
    echo "✅ Virtual environment created"
else
    echo "Virtual environment already exists"
fi

# Activate virtual environment
echo "Activating virtual environment..."
source venv/bin/activate

# Upgrade pip
echo "Upgrading pip..."
pip install --upgrade pip

# Install development dependencies
echo "Installing development dependencies..."
pip install -r requirements-dev.txt

# Install the package in development mode
echo "Installing northroot in development mode..."
maturin develop

echo ""
echo "✅ Development environment ready!"
echo ""
echo "To activate the environment in the future, run:"
echo "  cd $SCRIPT_DIR"
echo "  source venv/bin/activate"
echo ""
echo "To rebuild after code changes:"
echo "  maturin develop"
echo ""

