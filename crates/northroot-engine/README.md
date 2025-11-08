# northroot-engine

[![MSRV](https://img.shields.io/badge/MSRV-1.86-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-internal-orange)](https://github.com/Northroot-Labs/northroot)

**Type:** Library  
**Publish:** No (private for now, publishable future)  
**MSRV:** 1.86 (Rust 1.91.0 recommended)

**Proof algebra engine for receipt validation, composition, and commitment computation.**

The engine implements the core proof algebra operations: hash computation, receipt validation, sequential/parallel composition, and delta compute strategies.

## Purpose

The engine provides:

- **Commitment computation**: SHA-256 hashing with canonical JSON (JCS)
- **Receipt validation**: Hash integrity, signature verification, kind-specific rules
- **Composition**: Sequential (cod == dom) and parallel (tensor) receipt chains
- **Delta compute**: Overlap estimation, reuse decisions, Merkle Row-Map operations
- **Strategies**: Composable compute strategies for incremental execution

## Core Modules

### Commitments (`src/commitments.rs`)

Provides canonical hashing primitives:

- `sha256_prefixed()`: SHA-256 with `sha256:` prefix
- `jcs()`: JSON Canonicalization (RFC 8785) with sorted keys
- `commit_set_root()`: Merkle root for unordered sets
- `commit_seq_root()`: Merkle root for ordered sequences

### Strategies (`strategies/`)

Composable compute strategies for delta/incremental compute:

- **Partition strategies**: Row-based chunking with stable hashing
- **Incremental operators**: State-preserving transformations
- **Reuse decision logic**: Policy-driven overlap thresholds

## Usage

### Computing Commitments

```rust
use northroot_engine::commitments::*;
use serde_json::json;

// Canonical JSON
let value = json!({"b": 2, "a": 1});
let canonical = jcs(&value); // {"a":1,"b":2}

// SHA-256 with prefix
let hash = sha256_prefixed(b"hello");
// "sha256:2cf24dba5f0a3e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

// Set root (sorted)
let parts = vec!["c".to_string(), "a".to_string(), "b".to_string()];
let root = commit_set_root(&parts); // sorted before hashing
```

### Receipt Validation

```rust
// 1. Recompute hash from canonical body
let canonical_body = jcs(&receipt_body);
let computed_hash = sha256_prefixed(canonical_body.as_bytes());
assert_eq!(computed_hash, receipt.hash);

// 2. Verify signature(s)
// verify_signature(&receipt.sig, &receipt.hash)?;

// 3. Validate payload by kind
// validate_payload(&receipt.kind, &receipt.payload)?;

// 4. Check composition (if chained)
// assert_eq!(prev_receipt.cod, next_receipt.dom);
```

### Composition

**Sequential composition** (cod == dom):

```rust
// Chain receipts where cod(R_i) == dom(R_{i+1})
let chain = vec![receipt1, receipt2, receipt3];
for i in 0..chain.len() - 1 {
    assert_eq!(chain[i].cod, chain[i+1].dom);
}
```

**Parallel composition** (tensor):

```rust
// Tensor commitment: sorted child commitments joined with "|"
let child_hashes = vec![r1.hash.clone(), r2.hash.clone()];
let tensor_root = commit_set_root(&child_hashes);
```

## Delta Compute

The engine implements the reuse decision rule:

```
Reuse iff J > C_id / (α · C_comp)
```

Where:
- `J`: Jaccard overlap [0,1] between prior and current chunk sets
- `C_id`: Identity/integration cost
- `C_comp`: Baseline compute cost
- `α`: Operator incrementality factor [0,1]

See [Incremental Compute Strategy](strategies/README.md) for details.

## Strategies

Strategies are composable compute patterns:

- **Partition**: Stable row-based chunking
- **Incremental Sum**: State-preserving aggregation with Merkle Row-Map
- **Delta Apply**: Efficient updates to previous state

See `strategies/README.md` for strategy documentation.

## Testing

```bash
cargo test
```

Test execution and method validation:

```bash
cargo test --test test_execution_and_method
```

## Documentation

- **[Proof Algebra](../../docs/specs/proof_algebra.md)**: Unified algebra spec
- **[Incremental Compute](../../docs/specs/incremental_compute.md)**: Delta strategy
- **[Delta Compute](../../docs/specs/delta_compute.md)**: Formal reuse spec
- **[Merkle Row-Map](../../docs/specs/merkle_row_map.md)**: State structure

## Dependencies

- `sha2`: SHA-256 hashing
- `serde_json`: JSON serialization
- `hex`: Hex encoding

