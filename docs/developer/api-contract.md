# Northroot API Contract

Version: 0.1
Status: stable kernel plus incubating promoted layers
Scope: public crate/package surfaces in this repository

## Purpose

This document summarizes the API ownership boundaries. Rustdoc remains the
source for exact signatures; package READMEs document non-Rust packages.

## Stable Kernel APIs

| Crate | Responsibility |
| --- | --- |
| `northroot-canonical` | canonical JSON, strict parsing, digests, identifiers, quantities, event IDs |
| `northroot-journal` | append-only `.nrj` frame format, reader/writer APIs, event verification |

The stable public CLI commands are `canonicalize`, `event-id`, `append`, `read`,
and `verify`.

## Record and Substrate APIs

| Crate | Responsibility |
| --- | --- |
| `northroot-record` | neutral record schema, content IDs, validators, `.nrj` record streams, JSONL segment import/export |
| `northroot-node` | node and workspace manifest conventions |
| `northroot-state-eval` | product-agnostic projection/evaluation shapes and gate result data |

These crates are open substrate layers over the kernel. They must not embed
private product policy authority or deployment-specific behavior.

## Capability/Profile APIs

| Crate | Responsibility |
| --- | --- |
| `northroot-governance` | policy-record matching over records |
| `northroot-execution` | execution method registry contracts |
| `northroot-exchange` | constrained handoff/result profile |
| `northroot-ag` | sanitized ag-domain profile example over records |

These are public, sanitized capabilities. Concrete SaaS adapters, customer
workflow automation, and private deployment logic do not belong in this repo.

## Promoted Packages

| Package | Namespace | Responsibility |
| --- | --- | --- |
| `northroot-custody` | `northroot.custody` | custody inventory, policy, snapshot plan, verification result, retention decision, run summary, and steward helper contracts |
| `northroot-durability` | `northroot.durability` | legacy compatibility helpers for public/private artifact checks and simple copy manifests; not the backup/DR operation model |

The Python packages are tested separately from Cargo:

```bash
PYTHONPATH=packages/northroot-custody/src python3 -m unittest discover packages/northroot-custody/tests
PYTHONPATH=packages/northroot-durability/src python3 -m unittest discover packages/northroot-durability/tests
```

`northroot-custody` is the reusable stewardship surface. It defines public-safe
contracts and delegates execution to commodity tools such as `resticprofile`,
platform schedulers, 1Password service-account secret resolution, macOS
Keychain, offsite-copy tools, and external health monitors. Future backup,
restore, schedule, retention, and DR automation should call `nr steward` or the
`northroot.custody` APIs rather than adding new durability vocabulary.

## Core Invariants

- canonical bytes and event identity are deterministic;
- `.nrj` is the authoritative journal format when a journal exists;
- JSONL is an interchange/export shape, not a shadow source of truth;
- hidden CLI support commands may help operators but do not expand stable kernel semantics;
- domain meaning, product policy, private adapters, and deployments live above or outside the kernel.

## Local Documentation

Generate Rust docs with:

```bash
cargo doc --workspace --no-deps --open
```

Run the full repository gate with:

```bash
bash scripts/verify.sh
```
