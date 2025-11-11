# northroot-receipts

[![docs.rs](https://docs.rs/northroot-receipts/badge.svg)](https://docs.rs/northroot-receipts)
[![crates.io](https://img.shields.io/crates/v/northroot-receipts.svg)](https://crates.io/crates/northroot-receipts)
[![MSRV](https://img.shields.io/badge/MSRV-1.86-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Type:** Library  
**Publish:** Yes (crates.io)  
**MSRV:** 1.86 (Rust 1.91.0 recommended)

**Canonical data model for the Northroot proof algebra system.**

This crate defines the unified receipt envelope and all kind-specific payloads (data_shape, method_shape, reasoning_shape, execution, spend, settlement) as Rust types with JSON Schema validation.

## Purpose

The `northroot-receipts` crate is the **source of truth** for receipt structure. It provides:

- **Type definitions**: Rust structs for all receipt kinds
- **JSON Schemas**: Validation schemas for each payload type
- **Canonicalization**: Deterministic serialization rules
- **Syntactic validation**: Format checks, structure validation, schema validation

## Architecture Boundaries

**What this crate does**: Receipt structure, canonicalization, syntactic validation
- Receipt envelope and payload types
- Hash computation and canonicalization (CBOR RFC 8949)
- Format validation (timestamps, UUIDs, hash formats, policy_ref format)
- Schema validation (JSON Schema)
- Basic composition validation (cod/dom matching)
- JSON adapter layer for external compatibility

**What this crate does NOT do**:
- **Policy validation** (see `northroot-policy`) - answers "is this allowed?" (semantic validation)
- **Computation logic** (see `northroot-engine`) - answers "how do I compute this?"
- **Operator definitions** (see `northroot-ops`) - answers "what can be run?"

**Dependencies**:
- Depends on: `commons` only
- Must NOT be depended on by: `policy`, `ops` (forbidden by ADR_PLAYBOOK.md)
- Can be depended on by: `engine`, `policy`, `ops`, `planner`, `sdk/*`, `apps/*`

**Validation layers**:
1. **Syntactic (this crate)**: Format, structure, schema - "is this well-formed?"
2. **Semantic (northroot-policy)**: Policy compliance - "is this allowed?"
3. **Computation (northroot-engine)**: Execution logic - "how do I compute this?"

## Receipt Structure

### Unified Envelope

All receipts share a common envelope:

```rust
pub struct Receipt {
    pub rid: Uuid,              // UUIDv7 recommended
    pub version: String,        // envelope version, e.g., "0.3.0"
    pub kind: ReceiptKind,      // one of six kinds
    pub dom: String,            // sha256:<64hex> domain shape commitment
    pub cod: String,            // sha256:<64hex> codomain shape commitment
    pub links: Vec<Uuid>,       // child receipts for composition
    pub ctx: Context,          // policy, timestamp, determinism, identity
    pub payload: Payload,       // kind-specific payload
    pub attest: Option<Value>, // optional TEE/container attestations
    pub sig: Option<Signature>,// detached signature over hash
    pub hash: String,          // sha256 of canonical body (without sig/hash)
}
```

### Receipt Kinds

1. **DataShape**: Schema + optional sketches
2. **MethodShape**: Operator contracts as multiset/DAG
3. **ReasoningShape**: Decision/plan DAG over tools
4. **Execution**: Observable run structure (span commitments, roots)
5. **Spend**: Metered resources + pricing + reuse justification
6. **Settlement**: Multi-party netting state

## Usage

### Creating Receipts

```rust
use northroot_receipts::*;
use uuid::Uuid;

let mut receipt = Receipt {
    rid: Uuid::new_v7(),
    version: "0.3.0".to_string(),
    kind: ReceiptKind::DataShape,
    dom: "sha256:0000...".to_string(),
    cod: "sha256:abcd...".to_string(),
    links: vec![],
    ctx: Context {
        policy_ref: Some("pol:standard-v1".to_string()),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: Some(DeterminismClass::Strict),
        identity_ref: Some("did:key:...".to_string()),
    },
    payload: Payload::DataShape(DataShapePayload {
        schema_hash: "sha256:...".to_string(),
        sketch_hash: None,
    }),
    attest: None,
    sig: None,
    hash: String::new(), // Will be computed
};

// Compute and set hash
receipt.hash = receipt.compute_hash().unwrap();
```

### Hash Computation

```rust
// Compute hash from canonical body (excludes sig and hash fields)
let hash = receipt.compute_hash()?;
```

### Validation

Receipts must satisfy:

- **Hash integrity**: `hash == sha256(canonical(body_without_sig_hash))`
- **Signature verification**: All signatures verify over `hash`
- **Kind validation**: Payload matches kind-specific schema
- **Composition safety**: `cod(R_i) == dom(R_{i+1})` for sequential chains

```rust
// Validate a single receipt
receipt.validate()?;

// Validate a sequential chain
let chain = vec![receipt1, receipt2, receipt3];
validate_composition(&chain)?;
```

### Loading from JSON (Adapter Layer)

```rust
use northroot_receipts::adapters::json;
use std::fs;

// Load receipt from JSON file (uses adapter layer)
let json_str = fs::read_to_string("receipt.json")?;
let receipt = json::receipt_from_json(&json_str)?;

// Verify hash integrity
receipt.validate()?;
```

### CBOR Serialization (Core Format)

```rust
use northroot_receipts::canonical;
use ciborium::ser::into_writer;
use std::fs::File;

// Serialize receipt to CBOR (canonical format)
let mut file = File::create("receipt.cbor")?;
into_writer(&receipt, &mut file)?;

// Compute canonical hash
let hash = receipt.compute_hash()?; // Uses CBOR canonicalization

// Pretty-print using CDN (CBOR Diagnostic Notation)
let cbor_value = ciborium::value::Value::from(receipt);
let cdn = canonical::to_cdn(&cbor_value);
println!("Receipt (CDN):\n{}", cdn);
```

## Schemas

JSON Schemas are located in `../../schemas/receipts/`:

- `data_shape_schema.json`
- `method_shape_schema.json`
- `reasoning_shape_schema.json`
- `execution_schema.json`
- `spend_schema.json`
- `settlement_schema.json`

## Determinism Classes

- **strict**: Bit-identical reproducible outputs
- **bounded**: Bounded nondeterminism (float tolerances, seeded RNG)
- **observational**: Execution/log proof (no reproducibility claim)

## Delta Compute Support

The `SpendPayload` includes optional `justification` for reuse decisions:

```rust
pub struct ReuseJustification {
    pub overlap_j: Option<f64>,     // [0,1] Jaccard overlap
    pub alpha: Option<f64>,         // operator incrementality factor
    pub c_id: Option<f64>,          // identity/integration cost
    pub c_comp: Option<f64>,        // baseline compute cost
    pub decision: Option<String>,   // "reuse" | "recompute" | "hybrid"
    pub layer: Option<String>,      // "data" | "method" | "reasoning" | "execution"
}
```

The `layer` field indicates the semantic level of shape equivalence at which the reuse decision is evaluated:
- **"data"**: Raw or structured chunk equivalence (chunk IDs, partitions, file blocks)
- **"method"**: Operator/method plans (method shape roots, operator parameters)
- **"reasoning"**: Logical or semantic plans (reasoning DAGs, policy references)
- **"execution"**: Observed span outputs (span commitments, trace roots)

## Documentation

- **[Data Model Spec](docs/specs/data_model.md)**: Complete specification
- **[Proof Algebra](../docs/specs/proof_algebra.md)**: Unified algebra
- **[Incremental Compute](../docs/specs/incremental_compute.md)**: Delta strategy

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test test_vector_integrity
cargo test --test test_vectors_validate
cargo test --test test_drift_detection

# Regenerate vectors (use with caution)
cargo test --test regenerate_vectors -- --ignored --nocapture
```

### Test Suites

- **`test_vector_integrity`**: Verifies hash integrity of golden vectors
- **`test_vectors_validate`**: Validates vectors against JSON schemas
- **`test_drift_detection`**: Detects canonicalization changes
- **`test_roundtrip`**: Ensures lossless serialization/deserialization
- **`test_composition`**: Validates receipt chain composition
- **`test_hash_integrity`**: Tests hash computation
- **`test_edge_cases`**: Tests optional fields and edge cases

### Continuous Integration

The test harness includes several checks that should run in CI:

1. **Hash Integrity Check**: Ensures all vectors have correct hashes
   ```bash
   cargo test --test test_vector_integrity
   ```

2. **Schema Validation**: Validates all vectors against JSON schemas
   ```bash
   cargo test --test test_vectors_validate
   ```

3. **Drift Detection**: Alerts if canonicalization changes
   ```bash
   cargo test --test test_drift_detection
   ```

4. **Composition Validation**: Verifies receipt chains are valid
   ```bash
   cargo test --test test_composition
   ```

**CI Failure Conditions**:
- Any hash mismatch in vectors
- Schema validation failures
- Hash drift (canonicalization changes)
- Composition errors in chains
- Round-trip serialization failures

**Recommended CI Configuration**:
```yaml
# Example GitHub Actions workflow
- name: Test Receipts
  run: |
    cd receipts
    cargo test --test test_vector_integrity
    cargo test --test test_vectors_validate
    cargo test --test test_drift_detection
    cargo test --test test_composition
```

### Test Vectors

Test vectors are in `../../vectors/`:

- `data_shape.json`
- `method_shape.json`
- `execution.json`
- `spend.json`
- `settlement.json`

See [vectors/README.md](../../vectors/README.md) for details on regeneration and maintenance.

## Versioning

- **Envelope version**: Changes only on canonicalization or envelope structure changes
- **Kind payloads**: Version inside their schemas; evolve additively
- **Receipts MUST validate** against both envelope version and kind schema version

