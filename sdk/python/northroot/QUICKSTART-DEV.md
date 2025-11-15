# Quick Start: Development Environment

## One-Command Setup

```bash
cd sdk/python/northroot
./setup-dev.sh
```

This creates a virtual environment, installs dependencies, and builds the package.

## Activate Environment

```bash
cd sdk/python/northroot
source venv/bin/activate
```

## Common Commands

```bash
# Rebuild after code changes
maturin develop

# Build release wheels
maturin build --release

# Run examples (from sdk/python/northroot directory)
./run-example.sh
# OR
source venv/bin/activate
python examples/quickstart.py
```

## Troubleshooting

**Build fails with "missing field `package`":**
- Make sure you're in `sdk/python/northroot` directory
- The `Cargo.toml` should NOT have `[workspace]` section

**Import errors:**
- Activate venv: `source venv/bin/activate`
- Rebuild: `maturin develop`

