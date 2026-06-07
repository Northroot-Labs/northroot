# Northroot

Open governance and accountability infrastructure for verifiable economic
activity.

## What is Northroot?

Northroot is open governance and accountability infrastructure for verifiable
economic activity.

Its trust kernel provides canonical identity, append-only evidence journals,
replay, and offline verification.

Higher layers provide projection, evaluation, authority, receipts, and
financial/accountability profiles without polluting the kernel.

This repository is currently focused on making the core canonicalization and
journal reference crates solid before moving on to state/eval core.

**Current stable kernel capabilities:**
- Deterministic canonicalization (RFC 8785 + Northroot rules)
- Content-derived event identity
- Portable journal format (.nrj)
- Offline verification

**What the kernel does NOT do:**
- Make decisions or optimize outcomes
- Execute actions or orchestrate workflows
- Prescribe domain event schemas
- Evaluate policy, authority, or financial/accountability semantics

See [GOVERNANCE.md](GOVERNANCE.md) for the project's foundational principles.

## v0.1 Kernel Charter

Northroot v0.1 is the small, stable verifiable-event kernel component of the
broader Northroot system. It is intentionally not a runtime, scheduler, policy
engine, deployment stack, or application framework.

### Goals
- Keep core primitives deterministic and offline-verifiable.
- Standardize identity semantics for verifiable evidence.
- Keep `.nrj` as a portable append-only audit container.
- Publish minimal contracts that downstream repos can adopt without pulling orchestration into core.

### Non-goals
- No orchestration logic, workflow execution, or policy engines.
- No model, runtime, or budget decisioning logic.
- No domain-specific business semantics in core crates.
- No projection, evaluation, authority, receipt, or accounting profile semantics
  in the stable kernel.

## Quick Start

### Building

```bash
# Build kernel crates
cargo build --workspace

# Build CLI application
cd apps/northroot && cargo build --release
```

The CLI binary will be at `apps/northroot/target/release/northroot`.

### Using the CLI

```bash
# Canonicalize JSON input
echo '{"b":2,"a":1}' | northroot canonicalize

# Compute event_id for JSON
echo '{"event_type":"test","event_version":"1",...}' | northroot event-id

# Append an event to a journal
northroot append events.nrj event.json

# Read events from a journal
northroot read events.nrj

# Verify all events in a journal
northroot verify events.nrj
```

The public kernel CLI command set is `canonicalize`, `event-id`, `append`,
`read`, and `verify`.

## Documentation

### For Users
- [Getting Started](docs/user/getting-started.md) - Tutorial and examples
- [Integration Examples](docs/user/integration-examples.md) - Code samples

### For Developers
- [API Contract](docs/developer/api-contract.md) - Public API surface
- [Architecture](docs/developer/architecture.md) - System design
- [Testing Guide](docs/developer/testing.md) - QA harness and test patterns
- [Profiles](docs/reference/profiles.md) - How to layer profile semantics over the kernel
- [State Eval Core](docs/reference/state-eval-core.md) - Incubating product-agnostic evaluation primitives

### Reference
- [v0.1 Stability Contract](docs/reference/v0.1-stability.md) - Stable kernel and incubating profile boundaries
- [Economic Accountability North Star](docs/reference/economic-accountability-north-star.md) - Governed economic-action direction without product semantics
- [Core Specification](docs/reference/spec.md) - Protocol specification
- [Journal Format](docs/reference/format.md) - On-disk format
- [Segmented Journals](docs/reference/segmented-journals.md) - Structural segment manifests and checkpoints
- [Canonicalization](docs/reference/canonicalization.md) - Canonical JSON rules
- [Event Model](docs/reference/events.md) - Event structure

## Project Structure

```
northroot/
├── crates/
│   ├── northroot-canonical/  # Canonicalization + event_id
│   ├── northroot-journal/    # .nrj container format
│   └── northroot-state-eval/ # Incubating state/eval primitives
├── apps/
│   └── northroot/            # CLI application
├── fixtures/                  # Golden test vectors
├── schemas/
│   └── canonical/             # Canonical primitive schemas
├── docs/                      # Documentation
└── GOVERNANCE.md              # Project constitution
```

## Trust Kernel

The kernel provides:
- **Canonicalization**: RFC 8785 + Northroot hygiene rules
- **Event Identity**: `sha256(domain_separator || canonical_json(event))`
- **Journal Format**: Portable, append-only container (.nrj)

Everything else (projection, evaluation, authority, receipts, financial and
accountability profiles, typed schemas, domain verification, policy evaluation)
is a profile, layer, or consumer protocol over the kernel.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines, coding standards, and contribution process.

## License

Licensed under the MIT License ([LICENSE](LICENSE)).
