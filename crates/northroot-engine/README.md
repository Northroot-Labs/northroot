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

- **Commitment computation**: SHA-256 hashing with canonical CBOR (RFC 8949) and JSON (JCS)
- **Receipt validation**: Hash integrity, signature verification, kind-specific rules
- **Composition**: Sequential (cod == dom) and parallel (tensor) receipt chains
- **Delta compute**: Overlap estimation (Jaccard similarity), reuse decisions, economic delta calculations

## Core Modules

### Commitments (`src/commitments.rs`)

Provides canonical hashing primitives:

- `sha256_prefixed()`: SHA-256 with `sha256:` prefix
- `jcs()`: JSON Canonicalization (RFC 8785) with sorted keys
- `commit_set_root()`: Merkle root for unordered sets
- `commit_seq_root()`: Merkle root for ordered sequences

### Delta Compute (`delta/`)

Incremental recomputation with reuse decisions:

- **Overlap estimation**: Jaccard similarity between chunk sets
- **Reuse decisions**: Economic threshold-based reuse logic
- **Cost models**: Policy-driven cost evaluation

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

The engine provides `decide_reuse()` and `economic_delta()` functions for reuse decisions.

## What We Actually Verify

**Cryptographically signed, tamper-evident receipts with economic metadata** - better than logs because:

- **Hash Integrity**: `receipt.hash == compute_hash(receipt)` - tamper-evident
- **Signature Verification**: Ed25519 signatures over hash - authenticates creator
- **Canonical Form**: CBOR deterministic encoding (RFC 8949) - independently verifiable
- **Composition Verification**: `cod(R_i) == dom(R_{i+1})` - chain integrity
- **Merkle Roots**: For span commitments and identity roots

**What we DON'T verify** (what we claim):

- **Proofs of Computation Correctness**: We don't verify computation actually ran or output is correct
- **Proofs of Reuse Decision Correctness**: We just record the decision, don't verify it was optimal
- **Proofs of Data Matching Shapes**: Shape commitments are hashes, not verified against actual data

**We have proofs of structure and authenticity, not proofs of computation.**

## Receipt Population Strategy

**Core Fields (Always Populated for Verifiable Compute):**
- Envelope: `rid`, `version`, `kind`, `dom`, `cod`, `links`, `hash`
- Context: `timestamp` (audit trail)
- Execution: `trace_id`, `method_ref`, `data_shape_hash`, `span_commitments`, `roots`

**Optional Fields (Policy/Feature-Driven):**
- Context: `policy_ref`, `identity_ref`, `nonce`, `determinism` (populated when policy enabled)
- Execution: `pac` (caching), `change_epoch` (versioning), `output_mime_type`, `output_size_bytes` (metadata)
- Execution: `input_locator_refs`, `output_locator_ref` (when resolver used)
- All other fields: Reserved for future features, always `None` in v0.1

The structure is extensible - optional fields are populated only when needed, keeping receipts minimal for core use cases while supporting advanced features when enabled.

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
- **[Delta Compute](../../docs/specs/delta_compute.md)**: Formal reuse spec

## Dependencies

- `sha2`: SHA-256 hashing
- `serde_json`: JSON serialization
- `hex`: Hex encoding

