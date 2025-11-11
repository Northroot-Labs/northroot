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

## Quick Start

### Prerequisites

- **Rust**: 1.91.0 or later (MSRV: 1.86)
- **Cargo**: Comes with Rust installation
- **Platform Support**: 
  - ✅ Linux (x86_64, aarch64)
  - ✅ macOS (x86_64, Apple Silicon)
  - ✅ Windows (x86_64, MSVC toolchain)

### Installation

```bash
# Clone the repository
git clone https://github.com/Northroot-Labs/northroot.git
cd northroot

# Verify Rust version
rustc --version  # Should be 1.91.0 or later

# Build all crates
cargo build
```

### Running the Demos

Northroot includes three interactive demos that showcase proof-of-reuse for different compute scenarios:

#### 1. FinOps Cost Attribution Demo

Demonstrates proof of compute reuse for FinOps cost attribution with Jaccard similarity computation and economic delta calculations.

```bash
cargo run --package northroot-engine --example finops_cost_attribution
```

**What it shows:**
- Resource tuple overlap between billing runs
- Jaccard similarity computation
- Economic delta (ΔC) from reuse
- Receipt linking chain

#### 2. ETL Partition Reuse Demo

Demonstrates partition-level reuse for ETL workloads using Delta Lake CDF metadata.

```bash
cargo run --package northroot-engine --example etl_partition_reuse
```

**What it shows:**
- Partition-level overlap computation
- CDF metadata for changed partitions
- Reuse rate calculations (e.g., 85% reuse with only 15 partitions recomputed)

#### 3. Analytics Dashboard Demo

Demonstrates query result reuse for analytics dashboards with high-overlap scenarios.

```bash
cargo run --package northroot-engine --example analytics_dashboard
```

**What it shows:**
- Query result set overlap
- High-overlap reuse scenarios (90%+ similarity)
- Economic proof of incremental refresh

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test --package northroot-receipts

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test --test test_drift_detection
```

## Repository Map

**Libraries (crates/):**
- `northroot-receipts` — **Library. Publishable.** Canonical receipt data model & validation. Source of truth for receipt structure, canonicalization (CBOR RFC 8949), and hash computation.
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

**📊 Architecture Diagrams**: See [docs/specs/architecture-diagrams.md](docs/specs/architecture-diagrams.md) for visual representations of:
- Repository structure & dependencies
- Receipt composition flow
- Delta compute flow
- Verified spend flow
- Unified proof flow

## Core Concepts

### Receipts

A **receipt** is a unified envelope containing:
- **Envelope**: `rid`, `version`, `kind`, `dom`/`cod`, `ctx`, `sig`, `hash`
- **Payload**: Kind-specific data (data_shape, method_shape, reasoning_shape, execution, spend, settlement)

All receipts are:
- **Canonical**: CBOR deterministic encoding (RFC 8949) for deterministic hashing
- **Signed**: Detached signatures over canonical body hash
- **Composable**: Sequential (cod == dom) and parallel (tensor) composition
- **JSON Support**: JSON available via adapter layer for external compatibility only

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

## Building and Development

### Build Commands

```bash
# Build all crates
cargo build

# Build specific crate
cargo build --package northroot-receipts

# Build in release mode
cargo build --release

# Check without building
cargo check
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Lint code
cargo clippy -- -D warnings

# Run integrity checks
bash scripts/check-integrity.sh
```

### Cross-Platform Development

Northroot uses standard Rust toolchains and supports cross-compilation:

```bash
# Install cross-compilation target (example: Linux from macOS)
rustup target add x86_64-unknown-linux-gnu

# Build for specific target
cargo build --target x86_64-unknown-linux-gnu

# List available targets
rustup target list
```

**Common targets:**
- `x86_64-unknown-linux-gnu` - Linux x86_64
- `aarch64-unknown-linux-gnu` - Linux ARM64
- `x86_64-apple-darwin` - macOS Intel
- `aarch64-apple-darwin` - macOS Apple Silicon
- `x86_64-pc-windows-msvc` - Windows x86_64

## Usage Examples

### Creating a Receipt

```rust
use northroot_receipts::{Receipt, ReceiptKind, Payload};
use uuid::Uuid;

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

### Loading from JSON (Adapter Layer)

```rust
use northroot_receipts::adapters::json;

let json_str = r#"{"rid": "...", ...}"#;
let receipt = json::receipt_from_json(json_str)?;
```

### CBOR Serialization (Core Format)

```rust
use northroot_receipts::canonical;
use ciborium::ser;

let mut buf = Vec::new();
ser::into_writer(&receipt, &mut buf)?;

// Compute canonical hash
let hash = canonical::compute_hash(&receipt)?;
```

## Documentation

### Core Specifications

- **[Proof Algebra](docs/specs/proof_algebra.md)**: Unified algebra specification
- **[Data Model](crates/northroot-receipts/docs/specs/data_model.md)**: Receipt schema and types
- **[Incremental Compute](docs/specs/incremental_compute.md)**: Delta compute strategy
- **[Delta Compute](docs/specs/delta_compute.md)**: Formal reuse decision spec
- **[Merkle Row-Map](docs/specs/merkle_row_map.md)**: Deterministic state structure

### Development Guides

- **[ADR Playbook](docs/ADR_PLAYBOOK.md)**: Repository structure and code placement guide
- **[Contributing Guide](CONTRIBUTING.md)**: Contribution guidelines and workflow
- **[LLM Context](llms.txt)**: LLM-friendly codebase navigation and context

### Architecture

- **[Architecture Diagrams](docs/specs/architecture-diagrams.md)**: Visual representations of system architecture
- **[ADR-002: Canonicalization Strategy](ADRs/ADR-002-canonicalization-strategy.md)**: CBOR canonicalization approach

## Project Structure

```
northroot/
├── crates/
│   ├── northroot-receipts/    # Canonical data model (publishable)
│   ├── northroot-engine/      # Proof algebra engine (private for now)
│   ├── northroot-ops/         # Operator manifests (internal)
│   ├── northroot-policy/      # Policy definitions (internal)
│   ├── northroot-commons/     # Shared utilities (internal)
│   └── northroot-storage/     # Storage adapters (internal)
├── examples/                  # Demo examples
│   ├── finops_cost_attribution/
│   ├── etl_partition_reuse/
│   └── analytics_dashboard/
├── docs/                      # Project-level specifications and ADRs
├── schemas/                   # JSON Schemas organized by crate
└── vectors/                   # Test vectors and golden examples
```

## Compatibility

### Supported Platforms

| Platform | Architecture | Status | Notes |
|----------|-------------|--------|-------|
| Linux | x86_64 | ✅ Fully tested | Primary development platform |
| Linux | aarch64 | ✅ Supported | ARM64 support |
| macOS | x86_64 | ✅ Fully tested | Intel Macs |
| macOS | aarch64 | ✅ Fully tested | Apple Silicon (M1/M2/M3) |
| Windows | x86_64 | ✅ Supported | MSVC toolchain required |

### Rust Version

- **Minimum Supported Rust Version (MSRV)**: 1.86
- **Recommended**: 1.91.0 or later
- **Edition**: 2021

### Dependencies

- Standard library only for core functionality
- `serde` for serialization
- `ciborium` for CBOR canonicalization (RFC 8949)
- `sha2` for cryptographic hashing
- `uuid` for receipt identifiers

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Code organization guidelines
- Testing requirements
- Pull request process
- Code style and formatting

For LLM-assisted development, see [llms.txt](llms.txt) for codebase navigation and context.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
