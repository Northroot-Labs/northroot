# Northroot

Neutral, verifiable infrastructure for automated systems.

## What is Northroot?

Northroot provides deterministic, audit-grade evidence recording for automated systems. It standardizes how actions are authorized, executed, and verified—without making decisions or executing actions itself.

**Core capabilities:**
- Deterministic schemas and canonicalization
- Evidence and receipt generation
- Policy-bound authorization and execution verification
- Offline, replayable verification

**What Northroot does NOT do:**
- Make decisions or optimize outcomes
- Execute actions or orchestrate workflows
- Replace judgment or automate execution

See [GOVERNANCE.md](GOVERNANCE.md) for the project's foundational principles.

## Quick Start

### Building

```bash
cargo build --release
```

The CLI binary will be at `target/release/northroot`.

### Using the CLI

```bash
# List events in a journal
northroot list events.nrj

# Verify all events
northroot verify events.nrj

# Get a specific event
northroot get events.nrj <event_id>
```

See [crates/northroot-cli/README.md](crates/northroot-cli/README.md) for full CLI documentation.

## Documentation

### For Users
- [Getting Started](docs/user/getting-started.md) - Tutorial and examples
- [CLI Guide](crates/northroot-cli/README.md) - Command reference
- [Integration Examples](docs/user/integration-examples.md) - Code samples

### For Developers
- [API Contract](docs/developer/api-contract.md) - Public API surface
- [Architecture](docs/developer/architecture.md) - System design
- [Testing Guide](docs/developer/testing.md) - QA harness and test patterns
- [Extending Northroot](docs/developer/extending.md) - Custom backends and filters

### For Operators
- [Deployment Guide](docs/operator/deployment.md) - Production deployment
- [Kubernetes Security](docs/operator/k8s-security.md) - K8s security practices
- [Secrets Management](docs/operator/secrets.md) - Secret handling

### Reference
- [Core Specification](docs/reference/spec.md) - Protocol specification
- [Journal Format](docs/reference/format.md) - On-disk format
- [Canonicalization](docs/reference/canonicalization.md) - Canonical JSON rules
- [Event Model](docs/reference/events.md) - Event types and structure

## Project Structure

```
northroot/
├── crates/
│   ├── northroot-canonical/  # Canonicalization and digests
│   ├── northroot-core/       # Event types and verification
│   ├── northroot-journal/   # Journal format implementation
│   ├── northroot-store/     # Storage abstraction
│   └── northroot-cli/        # Command-line interface
├── schemas/                  # JSON Schemas (canonical, events, profiles)
├── docs/                     # Documentation
└── GOVERNANCE.md             # Project constitution
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines, coding standards, and contribution process.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

