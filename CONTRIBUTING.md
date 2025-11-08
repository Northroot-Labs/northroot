# Contributing to Northroot

Thank you for your interest in contributing to Northroot! This guide will help you get started.

## Quick Start

### Prerequisites

- Rust 1.91.0 (MSRV 1.86)
- `cargo` (comes with Rust)

### Running All Checks

```bash
# Build all crates
cargo build

# Run all tests
cargo test

# Format code
cargo fmt --check

# Lint code
cargo clippy -- -D warnings

# Check for dependency issues
cargo deny check  # if cargo-deny is installed
```

## Code Organization

See [docs/ADR_PLAYBOOK.md](docs/ADR_PLAYBOOK.md) for detailed guidance on where code belongs.

### Quick Reference

- **Receipt types/validation**: `crates/northroot-receipts/`
- **Proof algebra engine**: `crates/northroot-engine/`
- **Operator manifests**: `crates/northroot-ops/`
- **Policies/strategies**: `crates/northroot-policy/`
- **Shared utilities**: `crates/northroot-commons/`
- **Binaries**: `apps/`
- **Tools**: `tools/`

## Code Ownership

### Crates

- `northroot-receipts`: Core team (publishable, requires careful review)
- `northroot-engine`: Core team (private for now)
- `northroot-ops`: Core team (internal)
- `northroot-policy`: Core team (internal)
- `northroot-commons`: Core team (internal)

## Version Bumping

### When to Bump Versions

- **PATCH**: Bug fixes, internal refactors (no API change)
- **MINOR**: Additive changes that preserve compatibility
- **MAJOR**: Breaking changes

### Which Crates Need Bumping

- Changes to `northroot-receipts` may require version bumps in dependent crates
- Changes to `northroot-engine` may require version bumps in dependent crates
- Internal crates (`ops`, `policy`, `commons`) can be versioned independently

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes following the [ADR Playbook](docs/ADR_PLAYBOOK.md)
3. Add tests for new functionality
4. Update documentation as needed
5. Run all checks: `cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings`
6. Create a pull request with a clear description

## Testing

- Each crate has its own test suite in `tests/`
- Golden vectors are in `vectors/` and must pass validation
- Integration tests may be added at the workspace level

## Documentation

- Public APIs must be documented with `///` doc comments
- Module-level docs use `//!`
- Update README.md files when adding significant features
- Add ADRs for architectural decisions (see `ADRs/` directory)

## Questions?

See the [ADR Playbook](docs/ADR_PLAYBOOK.md) for detailed guidance on code placement and architecture.

