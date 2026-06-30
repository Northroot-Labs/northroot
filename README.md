# Northroot

Open governance and accountability infrastructure for verifiable state transitions.

## What Is Northroot?

Northroot is a small verifiable-state substrate plus promoted capabilities that
build on it. The kernel provides canonical identity, append-only `.nrj` streams,
strict JSON hygiene, replay, and offline verification. Higher layers add record
contracts, node/workspace manifests, governance matching, execution method
registries, exchange handoffs, and sanitized domain examples without changing
kernel identity rules.

The repository is intentionally split by layer:

- **Stable kernel crates**: deterministic canonicalization and `.nrj` journal I/O.
- **Record/substrate crates**: neutral records, node/workspace manifests, and
  state/eval helpers over verified streams.
- **Capability/profile crates**: governance, execution, exchange, and sanitized
  ag-domain examples.
- **Promoted packages**: importable non-Rust capabilities such as
  `northroot.custody`.
- **Capability index**: a public-safe seed registry in `capabilities/` for
  absorbing promoted packages without taking all of Northroot as an app.
- **Standalone CLI**: operator-facing commands in `apps/northroot`.

The public kernel proves what was recorded and whether it verifies. It does not
choose outcomes, execute workflows, or define private product semantics.

## Quick Start

```bash
# Optional one-time setup for humans, CI images, and agents.
bash scripts/dev_setup.sh

# Build Rust workspace crates.
cargo build --workspace

# Build the standalone CLI.
cargo build --release --manifest-path apps/northroot/Cargo.toml

# Run the normal fast gate.
just qa

# Run the full repository verification gate.
bash scripts/verify.sh
```

Legacy Codex entrypoints still exist as wrappers:

```bash
bash scripts/codex_setup.sh
bash scripts/codex_verify.sh
```

The CLI binaries are produced under `apps/northroot/target/release/northroot`
and `apps/northroot/target/release/nr`. `nr` is the short operator spelling;
`northroot` remains the long-form binary name.

## CLI Surface

Public kernel commands shown in normal help:

```bash
northroot canonicalize input.json
northroot event-id event.json
northroot append events.nrj event.json
northroot read events.nrj
northroot verify events.nrj
```

Node substrate commands initialize portable custody roots outside repositories:

```bash
northroot node init --slug local-node
northroot node status --json
```

Steward custody commands are the promoted backup/restore/scheduling surface.
They are exposed through the same CLI, with `nr` as the preferred short form:

```bash
nr steward init --inventory inventory.json --policy custody-policy.json --output .northroot/steward
nr steward preflight --state .northroot/steward
nr steward run --state .northroot/steward --execute
nr steward verify --state .northroot/steward --snapshot-id snap-001 --execute
nr steward restore-drill --state .northroot/steward --snapshot-id snap-001 --execute
nr steward report --state .northroot/steward --snapshot-id snap-001
nr steward schedule create --state .northroot/steward --scheduler launchd --operation verify --every-minutes 60
```

Steward delegates operational work to `resticprofile`, platform schedulers,
1Password or macOS Keychain secret resolution, offsite-copy tools, and external
monitors. Northroot owns the contracts, readiness checks, command surface, and
auditable summaries; it is not a custom backup engine or secret manager.

Hidden operator/development command groups also exist for record streams,
structural journal helpers, work-ledger dogfood, and bundle verification. Those
commands are incubating support surfaces, not stable kernel semantics.

## Project Structure

```text
northroot/
├── crates/
│   ├── northroot-canonical/   # canonical JSON, identifiers, event IDs
│   ├── northroot-journal/     # .nrj append/read/verify format
│   ├── northroot-record/      # neutral record contract and .nrj record streams
│   ├── northroot-node/        # node/workspace manifest conventions
│   ├── northroot-state-eval/  # product-agnostic state/eval primitives
│   ├── northroot-governance/  # policy-record matching over records
│   ├── northroot-execution/   # execution method registry contracts
│   ├── northroot-exchange/    # constrained handoff/result profile
│   └── northroot-ag/          # sanitized ag-domain example over records
├── packages/
│   ├── northroot-custody/     # Python package: northroot.custody
│   └── northroot-durability/  # Legacy compatibility boundary helpers
├── apps/
│   └── northroot/             # standalone CLI; not a workspace member
├── schemas/                   # public JSON schemas
├── docs/                      # user, developer, reference, QA docs
└── GOVERNANCE.md              # project constitution
```

## Layer Boundaries

Stable kernel:

- `northroot-canonical`
- `northroot-journal`
- public CLI commands: `canonicalize`, `event-id`, `append`, `read`, `verify`

Promoted or incubating layers over the kernel:

- `northroot-record`, `northroot-node`, `northroot-state-eval`
- `northroot-governance`, `northroot-execution`, `northroot-exchange`, `northroot-ag`
- `northroot-custody`
- `northroot-durability` compatibility helpers
- hidden CLI support commands

Private deployments, SaaS adapters, client workflows, real receipts, local
machine custody, and operational evidence do not belong in this public repo.
They belong in private/internal repos or the governed Northroot-Labs refinery.

## Documentation

- [Getting Started](docs/user/getting-started.md)
- [Architecture](docs/developer/architecture.md)
- [API Contract](docs/developer/api-contract.md)
- [Environment and Setup](docs/developer/environment.md)
- [Git Authorship](docs/developer/git-authorship.md)
- [Testing Guide](docs/developer/testing.md)
- [QA Harness](docs/qa/harness.md)
- [Node Substrate](docs/reference/node.md)
- [Record V0 Stack](docs/reference/record-v0/stack.md)
- [v0.1 Stability Contract](docs/reference/v0.1-stability.md)
- [Governance](GOVERNANCE.md)

## Promoted Packages

Promoted packages live under `packages/` when they are importable Northroot
capabilities but not kernel crates. They may be incubating, but they belong here
once other projects should import them through the `northroot.*` namespace. Lab
custody, local machine state, and promotion evidence stay in the Northroot-Labs
refinery.

Current packages:

- `northroot-custody`: Python distribution exposing `northroot.custody` for
  public-safe custody contracts, delegated snapshot plans, retention gates,
  restore verification, run summaries, and the `steward` helper layer used by
  `nr steward`.
- `northroot-durability`: legacy compatibility distribution exposing
  `northroot.durability` for public/private boundary checks and simple copy
  manifests. New backup, restore, scheduling, and disaster-recovery workflows
  should use `northroot-custody` and `nr steward`.

## Capability Index

`capabilities/index.public.json` is a static, public-safe capability catalog.
It records reusable Northroot capabilities, install refs, exported commands and
imports, verification commands, and ownership boundaries. It exists so another
project can absorb a package such as `northroot-custody` without depending on
the entire Northroot repository as an application.

Validate it with:

```bash
python3 scripts/validate_capability_index.py capabilities/index.public.json
```

Private downstream projects may keep local indexes under the ignored
`capabilities/private*.json` pattern or outside this repo.

## License

Licensed under the MIT License ([LICENSE](LICENSE)).
