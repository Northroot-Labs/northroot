# Northroot

Rust substrate for accountable agentic work.

## What is Northroot?

Northroot provides open primitives for obligations, policies, execution events,
receipts, actors, and cost attribution. It standardizes canonical hashing and
evidence verification without importing client-specific workflows or domain
semantics.

**Core capabilities:**
- JSON primitives for actors, obligations, policies, events, receipts, and cost
- JSONL execution logs for append-oriented evidence
- Deterministic canonical hashes
- Optional `.nrj` export container when framed offline audit is justified

**What Northroot does NOT do:**
- Store customer data
- Encode agriculture or client-specific nouns
- Provide hosted orchestration in the first build phase
- Replace ClearlyOps service/client workflows

See [GOVERNANCE.md](GOVERNANCE.md) for the project's foundational principles.

## Clean Reset Charter (v0)

This repo is the single public Northroot workspace. Separate crates keep the
substrate modular without spreading primitives across many repos.

### Goals
- Keep public primitives generic and reusable outside ClearlyOps.
- Use common formats first: JSON, JSONL, canonical SHA-256 hashes.
- Keep `.nrj` optional unless its framing and offline verification properties
  are necessary and tested.
- Ship examples that prove each primitive with inspectable evidence.

### Non-goals
- No hosted API, inbox automation, GitHub mutation workflow, MCP service, or
  agent orchestration in phase one.
- No client-specific compliance models in this repo.
- No mandatory custom binary format for first-use adoption.

## Quick Start

### Building

```bash
# Build all crates
cargo build --workspace

# Validate the first proof example
cargo run -p northroot -- validate examples/github-repo-inspection/obligation.json
cargo run -p northroot -- hash examples/github-repo-inspection/events.jsonl
cargo run -p northroot -- verify examples/github-repo-inspection/receipt.json
```

### Using the CLI

```bash
# Validate common primitives
northroot validate examples/github-repo-inspection/actor.json
northroot validate examples/github-repo-inspection/obligation.json
northroot validate examples/github-repo-inspection/policy.json

# Hash JSON or JSONL evidence
northroot hash examples/github-repo-inspection/events.jsonl

# Verify receipt evidence hashes
northroot verify examples/github-repo-inspection/receipt.json

# Canonicalize JSON input
echo '{"b":2,"a":1}' | northroot canonicalize

# Compute event_id for JSON when needed
echo '{"event_type":"test","event_version":"1",...}' | northroot event-id

# Optional .nrj journal support
northroot list events.nrj
northroot verify events.nrj
```

## Documentation

### For Users
- [Getting Started](docs/user/getting-started.md) - Tutorial and examples
- [Integration Examples](docs/user/integration-examples.md) - Code samples

### For Developers
- [API Contract](docs/developer/api-contract.md) - Public API surface
- [Architecture](docs/developer/architecture.md) - System design
- [Testing Guide](docs/developer/testing.md) - QA harness and test patterns
- [Extensions](docs/reference/extensions.md) - How to extend the kernel

### Reference
- [Core Specification](docs/reference/spec.md) - Protocol specification
- [Journal Format](docs/reference/format.md) - On-disk format
- [Canonicalization](docs/reference/canonicalization.md) - Canonical JSON rules
- [Event Model](docs/reference/events.md) - Event structure

## Project Structure

```
northroot/
├── crates/
│   ├── northroot-core/        # Shared canonical hash + validation helpers
│   ├── northroot-actor/       # Actor primitive
│   ├── northroot-obligation/  # Obligation primitive
│   ├── northroot-policy/      # Policy primitive
│   ├── northroot-event/       # Execution event primitive
│   ├── northroot-receipt/     # Receipt primitive
│   ├── northroot-cost/        # Cost attribution primitive
│   ├── northroot-canonical/   # Canonicalization + event_id
│   └── northroot-journal/     # Optional .nrj container format
├── apps/
│   └── northroot/            # CLI application
├── examples/
│   └── github-repo-inspection/
├── fixtures/                 # Golden test vectors
├── schemas/                  # Future schema publishing surface
├── wip/                      # Historical incubation code
├── docs/                      # Documentation
└── GOVERNANCE.md              # Project constitution
```

## Trust Kernel

The kernel provides:
- **Canonicalization**: RFC 8785 + Northroot hygiene rules
- **Event Identity**: `sha256(domain_separator || canonical_json(event))`
- **Journal Format**: Portable, append-only container (.nrj)

Everything else (typed schemas, domain verification, policy evaluation) is extension.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines, coding standards, and contribution process.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
