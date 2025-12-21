# Development Container

This directory contains the configuration for a reproducible development environment using VS Code Dev Containers.

## What's Included

- **Rust 1.91.0** (stable) with clippy and rustfmt
- **Rust nightly** with miri for undefined behavior detection
- **Development tools**:
  - `just` - Command runner for QA harness
  - `cargo-llvm-cov` - Code coverage
  - `cargo-deny` - License and security checks
  - `cargo-audit` - Security vulnerability scanning
  - `cargo-fuzz` - Fuzzing support
  - `rust-analyzer` - Language server for IDE support

## Usage

1. Open the project in VS Code
2. When prompted, click "Reopen in Container" or use Command Palette: "Dev Containers: Reopen in Container"
3. Wait for the container to build (first time may take a few minutes)
4. All tools are pre-installed and ready to use

## Quick Start

Once in the container:

```bash
# Run fast QA checks
just qa

# Run all tests
just test

# Check formatting
just fmt

# Run clippy
just lint
```

## Environment Variables

- `CARGO_TERM_COLOR=always` - Colored output
- `RUSTFLAGS=-Dwarnings` - Warnings as errors

## VS Code Extensions

Automatically installed:
- `rust-lang.rust-analyzer` - Rust language support
- `tamasfe.even-better-toml` - TOML file support
- `serayuzgur.crates` - Cargo.toml dependency management
- `vadimcn.vscode-lldb` - Debugging support
- `ms-vscode.vscode-json` - JSON support

## Building Manually

To build the Docker image manually:

```bash
docker build -t northroot-dev -f Dockerfile .
```

## Troubleshooting

- **Container won't start**: Check Docker is running and has enough resources (4GB+ RAM recommended)
- **Tools missing**: Run `postCreateCommand` manually: `rustup component add clippy rustfmt && cargo fetch`
- **Slow performance**: Ensure Docker has sufficient CPU and memory allocated

