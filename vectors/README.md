# Test Vectors

**Test vectors for receipt validation and testing.**

This directory contains golden test vectors (JSON format for human readability) that engines and SDKs must round-trip correctly. All vectors are validated against their respective JSON schemas. Internally, receipts use CBOR canonicalization (RFC 8949) for hash computation, but test vectors are stored as JSON for readability and converted via adapter layer.

## Purpose

Test vectors serve as:

- **Golden examples**: Reference implementations for each receipt kind
- **Validation targets**: Engines/SDKs must pass these through validators
- **Interoperability**: Cross-language verification (all must produce same hashes)
- **Documentation**: Concrete examples of receipt structure

## Vector Files

### Receipt Kinds

- `data_shape.json`: Data shape receipt example
- `method_shape.json`: Method shape receipt example
- `reasoning_shape.json`: Reasoning shape receipt example
- `execution.json`: Execution receipt example
- `spend.json`: Spend receipt example
- `settlement.json`: Settlement receipt example

### Supporting Files

- `method_manifest.json`: Method manifest example
- `operators.json`: Operator manifest examples

### Engine Test Vectors

The `engine/` subdirectory contains test vectors for engine operations:

- `composition_chain_valid.json`: Valid sequential chain with all 6 receipt kinds (data_shape → method_shape → reasoning_shape → execution → spend → settlement) with matching cod/dom
- `composition_chain_invalid.json`: Invalid chain with mismatched cod/dom (for negative tests)
- `tensor_composition.json`: Parallel composition example with multiple child receipts
- `delta_compute_scenarios.json`: Delta compute test cases (Jaccard similarity, reuse decisions, economic delta)
- `execution_roots.json`: Execution root computation examples
- `merkle_row_map_examples.json`: Merkle Row-Map examples demonstrating CBOR canonicalization with domain-separated hashing

## Validation Requirements

All vectors must:

1. **Parse correctly**: Valid JSON matching receipt structure
2. **Validate against schemas**: Pass JSON Schema validation
3. **Hash integrity**: `hash == sha256(cbor_canonical(body_without_sig_hash))` (CBOR canonicalization per RFC 8949)
4. **Signature verification**: All signatures verify over `hash` (if present)
5. **Kind validation**: Payload matches kind-specific rules

## Usage

### Testing Receipt Parsing

```rust
use northroot_receipts::adapters::json;
use std::fs;

// Load receipt from JSON (uses adapter layer)
let json = fs::read_to_string("vectors/data_shape.json")?;
let receipt = json::receipt_from_json(&json)?;
```

### Validating Receipts

```rust
use northroot_receipts::Receipt;

// 1. Recompute hash (uses CBOR canonicalization)
let computed_hash = receipt.compute_hash()?;
assert_eq!(computed_hash, receipt.hash);

// 2. Validate against schema
receipt.validate()?;
```

### Round-Trip Testing

```rust
use northroot_receipts::{Receipt, adapters::json};
use ciborium::{ser::into_writer, de::from_reader};
use std::io::Cursor;

// JSON round-trip (via adapter)
let receipt1 = json::receipt_from_json(&json)?;
let json2 = json::receipt_to_json(&receipt1)?;
let receipt2 = json::receipt_from_json(&json2)?;
assert_eq!(receipt1, receipt2);

// CBOR round-trip (core format)
let mut cbor_bytes = Vec::new();
into_writer(&receipt1, &mut cbor_bytes)?;
let receipt3: Receipt = from_reader(Cursor::new(cbor_bytes))?;
assert_eq!(receipt1, receipt3);
```

## Vector Structure

Each vector file contains a complete receipt with:

- **Envelope**: All required fields (rid, version, kind, dom, cod, ctx, hash)
- **Payload**: Kind-specific data matching schema
- **Optional fields**: sig, attest, links (if applicable)

## Composition Examples

### Sequential Chain

A typical chain:
1. `data_shape.json` → `method_shape.json`
2. `method_shape.json` → `execution.json`
3. `execution.json` → `spend.json`
4. Multiple `spend.json` → `settlement.json`

**Invariant**: `cod(R_i) == dom(R_{i+1})` for sequential chains.

### Parallel (Tensor)

Multiple receipts composed in parallel:
- Parent receipt has `links: [rid1, rid2, ...]`
- Tensor root: `sha256(sorted(child_hashes).join("|"))`

## Regenerating Vectors

**⚠️ Warning**: Regenerating vectors updates the golden test files. Only do this when:
- You've intentionally changed canonicalization logic
- You've updated receipt structure and need new hashes
- You're fixing a bug in vector generation

### Regeneration Process

1. **Regenerate vectors with correct hashes**:
   ```bash
   cd receipts
   cargo test --test regenerate_vectors -- --ignored --nocapture
   ```
   This will:
   - Generate all receipt kinds with correct structure
   - Compute hashes using current canonicalization logic
   - Write updated vectors to `../vectors/`
   - Validate all regenerated vectors

2. **Verify regenerated vectors**:
   ```bash
   cargo test --test test_vector_integrity
   cargo test --test test_vectors_validate
   ```

3. **Update drift detection baselines** (if canonicalization changed):
   - Run `cargo test --test test_vector_integrity` to get new hashes
   - Update `BASELINE_HASHES` in `receipts/tests/test_drift_detection.rs`
   - Commit both vector files and baseline updates together

4. **Verify all tests pass**:
   ```bash
   cargo test
   ```

### When to Regenerate

- ✅ **DO regenerate** when:
  - Canonicalization logic changes (CBOR canonicalization updates)
  - Receipt structure changes (new required fields)
  - Fixing bugs in hash computation

- ❌ **DON'T regenerate** when:
  - Only adding optional fields (vectors should still validate)
  - Updating documentation or comments
  - Changing test code (unless it affects generation)

### Hash Drift

If `test_drift_detection` fails after regeneration:
1. Verify the canonicalization change was intentional
2. Update `BASELINE_HASHES` in `test_drift_detection.rs`
3. Document the change in commit message
4. Ensure all downstream consumers are aware of the change

## Adding New Vectors

When adding new vectors:

1. **Follow canonicalization**: Core uses CBOR (RFC 8949) for deterministic encoding. Test vectors are stored as JSON for readability and converted via adapter layer.
2. **Validate against schema**: Ensure payload matches kind schema
3. **Compute hash correctly**: `hash = sha256(cbor_canonical(body_without_sig_hash))` (CBOR canonicalization)
4. **Document purpose**: Add comments explaining the vector's purpose
5. **Test round-trip**: Verify serialization/deserialization preserves structure (both JSON and CBOR)
6. **Update baselines**: Add new vector to `BASELINE_HASHES` in drift detection test

## Schema References

Vectors validate against schemas in `receipts/schemas/`:

- `data_shape_schema.json`
- `method_shape_schema.json`
- `reasoning_shape_schema.json`
- `execution_schema.json`
- `spend_schema.json`
- `settlement_schema.json`

## Engine Vector Validation

Engine test vectors are validated by:

- `test_engine_vector_integrity`: Validates all engine vectors produce expected results
- `test_composition_vector_roundtrip`: Verifies composition vectors work correctly
- `test_drift_detection`: Ensures root computation algorithms haven't changed

### Root Computation Baselines

Engine root computation functions have locked baseline values in `crates/northroot-engine/tests/test_drift_detection.rs`:

- `commit_set_root()`: Order-independent set root computation
- `commit_seq_root()`: Order-dependent sequence root computation
- `compute_tensor_root()`: Parallel composition root
- `MerkleRowMap::compute_root()`: CBOR canonicalization with domain-separated hashing (leaf:/node: prefixes)
- `compute_execution_roots()`: Execution root combination

To update baselines after intentional algorithm changes:
1. Run `cargo test -p northroot-engine --test test_drift_detection -- --nocapture`
2. Update `BASELINE_ROOTS` in `test_drift_detection.rs` with new values
3. Document the change in commit message

## Documentation

- **[Data Model](../crates/northroot-receipts/docs/specs/data_model.md)**: Receipt structure
- **[Proof Algebra](../docs/specs/proof_algebra.md)**: Unified algebra
- **[Receipts README](../crates/northroot-receipts/README.md)**: Receipt crate documentation

