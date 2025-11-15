# Northroot Python SDK - Development Setup

## Quick Start

### Option 1: Automated Setup (Recommended)

```bash
cd sdk/python/northroot
./setup-dev.sh
```

This will:
- Create a Python virtual environment
- Install all development dependencies
- Build and install the package in development mode

### Option 2: Manual Setup

```bash
cd sdk/python/northroot

# Create virtual environment
python3 -m venv venv

# Activate virtual environment
source venv/bin/activate

# Install development dependencies
pip install -r requirements-dev.txt

# Install package in development mode
maturin develop
```

### Option 3: Using Make

```bash
cd sdk/python/northroot

# Create venv and install dev dependencies
make venv
make install-dev

# Build the package
make build

# Or install in development mode
make develop
```

## Activating the Environment

After setup, activate the environment:

```bash
cd sdk/python/northroot
source venv/bin/activate
```

## Development Workflow

### Rebuild After Code Changes

```bash
# In the activated virtual environment
maturin develop
```

Or use make:
```bash
make develop
```

### Run Examples

**Important:** Always run examples from the `sdk/python/northroot` directory with the virtual environment activated.

```bash
# Option 1: Use the run script (recommended)
cd sdk/python/northroot
./run-example.sh

# Option 2: Manual (activate venv first)
cd sdk/python/northroot
source venv/bin/activate
python examples/quickstart.py

# OTEL integration example
python examples/otel_integration.py
```

### Build for Release

```bash
# Build release wheels
maturin build --release

# Wheels will be in dist/
```

### Test Installation

```bash
# Install from local wheel
pip install dist/northroot-0.1.0-*.whl

# Test import
python -c "from northroot import Client; print('✅ Import successful')"
```

## Troubleshooting

### Maturin Build Fails

If you see "missing field `package`" error:
- Make sure you're in `sdk/python/northroot` directory
- The `Cargo.toml` should have `[package]` section, not `[workspace]`

### Import Errors

If imports fail after `maturin develop`:
- Make sure virtual environment is activated
- Try rebuilding: `maturin develop --release`
- Check that `northroot/__init__.py` exists

### Rust Compilation Errors

- Make sure Rust toolchain is installed: `rustc --version`
- Update Rust: `rustup update`
- Clean and rebuild: `cargo clean && maturin develop`

## Environment Variables

- `MATURIN_PEP517_ARGS`: Additional arguments for maturin build
- `RUST_LOG`: Rust logging level (e.g., `RUST_LOG=debug`)

## Directory Structure

```
sdk/python/northroot/
├── venv/              # Virtual environment (created by setup)
├── src/               # Rust source code
├── northroot/         # Python package
├── examples/          # Example scripts
├── Cargo.toml         # Rust package config
├── pyproject.toml     # Python package config
├── requirements-dev.txt # Development dependencies
├── setup-dev.sh       # Setup script
└── Makefile           # Development commands
```

