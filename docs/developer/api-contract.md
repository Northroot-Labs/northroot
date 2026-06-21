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
| `northroot-durability` | `northroot.durability` | durability naming, tiered policy, manifests, and public/private artifact checks |

The Python package is tested separately from Cargo:

```bash
PYTHONPATH=packages/northroot-durability/src python3 -m unittest discover packages/northroot-durability/tests
```

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
