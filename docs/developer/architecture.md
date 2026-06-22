# Northroot Architecture

Northroot is organized as a small trust kernel with explicit layers above it.
The kernel owns canonical bytes, event identity, and `.nrj` verification. Higher
layers may structure records, manifests, state/eval inputs, or sanitized domain
profiles, but they do not change kernel identity rules.

## Layer Map

```text
apps/northroot                 standalone CLI, outside Cargo workspace
packages/northroot-custody     Python package: northroot.custody
packages/northroot-durability  legacy compatibility boundary helpers

crates/northroot-ag            sanitized ag-domain example over records
crates/northroot-exchange      constrained handoff/result profile
crates/northroot-execution     execution method registry contracts
crates/northroot-governance    policy-record matching over records
crates/northroot-state-eval    product-agnostic state/eval shapes
crates/northroot-node          node/workspace manifest conventions
crates/northroot-record        neutral records and .nrj record streams
crates/northroot-journal       stable .nrj journal format
crates/northroot-canonical     stable canonical bytes and event IDs
```

Dependency direction is upward. `northroot-canonical` must not depend on other
Northroot crates. `northroot-journal` depends on canonicalization. Record and
profile crates compose those lower layers.

## Stable Kernel

### `northroot-canonical`

Purpose: deterministic canonicalization and event identity.

Responsibilities:

- canonical JSON serialization using RFC 8785 plus Northroot rules
- strict JSON parsing that rejects duplicate object keys before collapse
- digest, identifier, timestamp, and quantity primitives
- event ID computation and verification
- hygiene reporting

### `northroot-journal`

Purpose: portable append-only `.nrj` streams.

Responsibilities:

- journal header and frame encoding
- strict and permissive read modes
- writer/reader APIs
- event identity verification over journal payloads
- truncation and resilience behavior

## Record and Substrate Layers

### `northroot-record`

Neutral record contract over `.nrj` streams. It validates record grammar,
content-derived IDs, record stream wrappers, JSONL segment export/import, and
segment seals. It does not interpret profile meaning.

### `northroot-node`

Node and workspace manifest conventions. Node means accountable context;
workspace means materialized operational environment.

### `northroot-state-eval`

Product-agnostic state/eval data shapes: projection identity, ordered event
prefixes, three-valued predicates, evaluation deltas, and gate result shapes.
It is not a product policy engine.

## Capability and Profile Layers

- `northroot-governance`: matches policy records against command records.
- `northroot-execution`: validates execution method registry records.
- `northroot-exchange`: constrained handoff/result record profile.
- `northroot-ag`: sanitized agricultural vocabulary and profile example over records.

These crates are open capability examples. Real SaaS adapters, client workflows,
private deployments, and operational runbooks stay outside this public repo.

## Promoted Packages

`packages/northroot-custody` exposes the Python namespace `northroot.custody`
for the reusable custody vocabulary and steward helper layer. It owns
workspace inventories, custody policies, delegated snapshot plans, verification
results, retention decisions, run summaries, and agent-safe capability reports.
It does not implement a backup engine, scheduler, secret manager, storage
transport, or monitoring stack. Those jobs are delegated to commodity tools
such as `resticprofile`, `launchd`, `systemd`, 1Password service-account based
secret resolution, macOS Keychain, offsite-copy tools, and external health
monitors.

`apps/northroot` exposes that package through `nr steward`. The long-form
`northroot` binary remains available, but `nr` is the preferred operator
spelling for node and steward commands.

`packages/northroot-durability` remains only as a legacy compatibility package
for public/private artifact boundary checks and simple copy manifests. New
backup, restore, schedule, retention, and disaster-recovery workflows should
use `northroot-custody` and `nr steward`.

## CLI Application

`apps/northroot` is a standalone CLI with path dependencies into the workspace
crates. It is intentionally not a Cargo workspace member.

Public normal-help commands:

- `canonicalize`
- `event-id`
- `append`
- `read`
- `verify`

Hidden operator/development command groups include bundle verification, work
ledger dogfood, structural journal helpers, and record stream import/export.
Those are support surfaces, not stable kernel semantics.

## Design Principles

1. **Separation of concerns**: each crate has a narrow responsibility.
2. **Domain-agnostic kernel**: core validates identity and structure, not business meaning.
3. **Determinism**: verification must be offline and repeatable.
4. **Neutrality**: Northroot records what happened and what was allowed; it does not decide what should happen.
5. **Public/private split**: sanitized schemas and tooling can be public; private deployments and SaaS adapters stay private.

## Related Documentation

- [API Contract](api-contract.md)
- [Stewardship Workstream](stewardship-workstream.md)
- [Environment and Setup](environment.md)
- [Core Specification](../reference/spec.md)
- [Record V0 Stack](../reference/record-v0/stack.md)
- [Profiles and Consumer Protocols](../reference/profiles.md)
- [Core Invariants](../../CORE_INVARIANTS.md)
