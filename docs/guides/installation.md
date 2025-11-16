# Installation Guide

**Effective Date:** 2025-11-16  
**Goal Grid Task:** P7-T2

## Quick Install (Recommended)

The fastest way to get started with Northroot is via PyPI:

```bash
pip install northroot
```

That's it! You're ready to use the Python SDK.

## Verify Installation

```bash
python -c "from northroot import Client; print('✅ Northroot installed successfully!')"
```

## Local Development Setup

If you want to develop or contribute to Northroot, set up a local development environment:

### Prerequisites

- **Python**: 3.10 or later (3.12 recommended)
- **Rust**: 1.91.0 or later
- **Cargo**: Comes with Rust installation
- **maturin**: Python-Rust build tool (installed automatically)

### Quick Setup Script

```bash
cd sdk/python/northroot
./setup-dev.sh
```

This script will:
1. Create a Python virtual environment
2. Install development dependencies
3. Build the SDK in development mode
4. Install the package locally

### Manual Setup

If you prefer manual setup:

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install maturin
pip install maturin

# 3. Navigate to SDK directory
cd sdk/python/northroot

# 4. Create virtual environment
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# 5. Install development dependencies
pip install -e ".[dev]"

# 6. Build and install SDK
maturin develop
```

## Platform Support

### Linux

✅ **Fully Supported**
- x86_64 (64-bit)
- aarch64 (ARM64)

**Requirements:**
- Python 3.10+
- Rust 1.91.0+
- Standard build tools (gcc, make)

### macOS

✅ **Fully Supported**
- x86_64 (Intel)
- Apple Silicon (M1/M2/M3)

**Requirements:**
- Python 3.10+
- Rust 1.91.0+
- Xcode Command Line Tools: `xcode-select --install`

### Windows

✅ **Supported**
- x86_64 (64-bit)

**Requirements:**
- Python 3.10+
- Rust 1.91.0+ (MSVC toolchain)
- Visual Studio Build Tools or Visual Studio Community

**Note:** On Windows, use `venv\Scripts\activate` instead of `source venv/bin/activate`.

## Docker-Free Setup

Northroot is designed to work without Docker. All dependencies are:
- **Rust crates**: Built from source or available via crates.io
- **Python packages**: Available via PyPI or built from source

No container runtime required!

## Troubleshooting

### Python Version Issues

**Problem:** `python` command not found or wrong version

**Solution:**
```bash
# Check Python version
python3 --version  # Should be 3.10+

# Use python3 explicitly
python3 -m pip install northroot
```

### Rust Not Found

**Problem:** `cargo: command not found`

**Solution:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Reload shell
source ~/.cargo/env

# Verify
cargo --version
```

### Build Errors on macOS

**Problem:** Missing Xcode Command Line Tools

**Solution:**
```bash
xcode-select --install
```

### Build Errors on Linux

**Problem:** Missing build dependencies

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install build-essential python3-dev

# Fedora/RHEL
sudo dnf install gcc python3-devel
```

### Permission Errors

**Problem:** `Permission denied` when installing

**Solution:**
```bash
# Use --user flag
pip install --user northroot

# Or use virtual environment (recommended)
python3 -m venv venv
source venv/bin/activate
pip install northroot
```

## Next Steps

After installation:

1. **Try the Hello Receipts demo:**
   ```bash
   python examples/hello_receipts.py
   ```

2. **Run the Quickstart example:**
   ```bash
   python examples/quickstart.py
   ```

3. **Read the API documentation:**
   - [Python SDK README](../sdk/python/northroot/README.md)
   - [Quick Start Guide](./quickstart.md)

## Development Dependencies

For development, install additional tools:

```bash
pip install -e ".[dev]"
```

This includes:
- `maturin` - Build tool for Rust-Python packages
- `pytest` - Testing framework
- `black` - Code formatter
- `mypy` - Type checker
- `ruff` - Linter

## Uninstallation

```bash
pip uninstall northroot
```

## Related Documentation

- [Quick Start Guide](./quickstart.md)
- [Python SDK README](../sdk/python/northroot/README.md)
- [Troubleshooting Guide](./troubleshooting.md)
- [Contributing Guide](../../CONTRIBUTING.md)

