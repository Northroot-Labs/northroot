# Northroot

**A unified proof algebra system for verifiable compute and value exchange.**

Northroot provides a canonical data model and proof system for expressing, verifying, and composing computational work as typed morphisms over shapes. The system enables delta compute strategies, multi-party netting, and transparent economic accounting of computational resources.

## Overview

Northroot implements a **unified receipt algebra** where all proofs are expressed as signed, typed morphisms over shapes. The system supports:

- **Receipts**: Canonical, signed evidence of computational transformations
- **Shapes**: Typed commitments (data, method, reasoning, execution, spend, settlement)
- **Composition**: Sequential and parallel composition of receipts
- **Delta Compute**: Incremental recomputation with reuse decisions
- **Settlement**: Multi-party netting and value exchange

## Repository Map

**Libraries (crates/):**
- `northroot-receipts` — **Library. Publishable.** Canonical receipt data model & validation. Source of truth for receipt structure, canonicalization (JCS), and hash computation.
- `northroot-engine` — **Library. Private (publishable future).** Proof/compute kernel. Receipt validation, composition, commitment computation, and delta compute strategies.
- `northroot-ops` — **Library. Internal.** Operator & method manifests (schemas, examples, validators).
- `northroot-policy` — **Library. Internal.** Policies & strategies (cost models, reuse thresholds, allow/deny, FP tolerances).
- `northroot-commons` — **Library. Internal.** Cross-cutting utilities (logging, error types, shared data structures).

**Binaries (apps/):**
- (Future binaries will be added here)

**Tools (tools/):**
- (Future tool binaries will be added here)

**Other:**
- `docs/` — Project-level specifications and ADRs
- `schemas/` — JSON Schemas organized by crate (receipts, ops, policy)
- `vectors/` — Golden test vectors (CI-validated)

## Architecture

```
northroot/
├── crates/
│   ├── northroot-receipts/    # Canonical data model (publishable)
│   ├── northroot-engine/      # Proof algebra engine (private for now)
│   ├── northroot-ops/         # Operator manifests (internal)
│   ├── northroot-policy/      # Policy definitions (internal)
│   └── northroot-commons/     # Shared utilities (internal)
├── apps/                      # Binaries (future)
├── tools/                      # Tool binaries (future)
├── docs/                       # Specifications and ADRs
├── schemas/                    # JSON Schemas
└── vectors/                    # Test vectors & golden examples
```

## Core Concepts

### Receipts

A **receipt** is a unified envelope containing:
- **Envelope**: `rid`, `version`, `kind`, `dom`/`cod`, `ctx`, `sig`, `hash`
- **Payload**: Kind-specific data (data_shape, method_shape, reasoning_shape, execution, spend, settlement)

All receipts are:
- **Canonical**: JSON Canonicalization (JCS) for deterministic hashing
- **Signed**: Detached signatures over canonical body hash
- **Composable**: Sequential (cod == dom) and parallel (tensor) composition

### Shapes & Kinds

Six receipt kinds form a typical chain:

1. **data_shape**: Schema + optional sketches (⊥ → S_data)
2. **method_shape**: Operator contracts as multiset/DAG (S_data → S'_data)
3. **reasoning_shape**: Decision/plan DAG over tools (S_method* → S_plan)
4. **execution**: Observable run structure (S_plan → S_exec)
5. **spend**: Metered resources + pricing (S_exec → S_spend)
6. **settlement**: Multi-party netting (Σ_i S_spend → S_cleared)

### Delta Compute

The system supports incremental recomputation via:
- **Overlap measurement**: Jaccard similarity on chunk sets
- **Reuse decision rule**: `J > C_id / (α · C_comp)`
- **Merkle Row-Map**: Deterministic state for incremental operators
- **Policy-driven**: Thresholds and strategies controlled by policies

## Quick Start

### Prerequisites

- Rust 1.91.0 (MSRV 1.86)
- JSON Schema validator (for receipt validation)

### Building

```bash
# Build all crates
cargo build

# Build specific crate
cargo build -p northroot-receipts

# Run all tests
cargo test

# Run tests for specific crate
cargo test -p northroot-receipts
```

### Using Receipts

```rust
use northroot_receipts::{Receipt, ReceiptKind, Payload};

// Create a data_shape receipt
let receipt = Receipt {
    rid: Uuid::new_v7(),
    version: "0.3.0".to_string(),
    kind: ReceiptKind::DataShape,
    dom: "sha256:0000...".to_string(),
    cod: "sha256:abcd...".to_string(),
    // ... other fields
};
```

## Documentation

- **[Proof Algebra](docs/specs/proof_algebra.md)**: Unified algebra specification
- **[Data Model](crates/northroot-receipts/docs/specs/data_model.md)**: Receipt schema and types
- **[Incremental Compute](docs/specs/incremental_compute.md)**: Delta compute strategy
- **[Delta Compute](docs/specs/delta_compute.md)**: Formal reuse decision spec
- **[Merkle Row-Map](docs/specs/merkle_row_map.md)**: Deterministic state structure
- **[ADR Playbook](docs/ADR_PLAYBOOK.md)**: Repository structure and code placement guide

## Project Structure

- **crates/northroot-receipts/**: Rust crate defining receipt types and schemas (publishable)
- **crates/northroot-engine/**: Proof algebra engine, commitments, validation (private for now)
- **crates/northroot-ops/**: Operator manifests (internal)
- **crates/northroot-policy/**: Policy definitions for strategy control (internal)
- **crates/northroot-commons/**: Shared utilities (internal)
- **docs/**: Project-level specifications and ADRs
- **schemas/**: JSON Schemas organized by crate
- **vectors/**: Test vectors and golden examples

## License

See [LICENSE](LICENSE) file.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines and [llms.txt](llms.txt) for LLM-friendly codebase navigation and context.
