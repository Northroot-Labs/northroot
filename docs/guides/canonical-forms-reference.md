# Canonical Forms Reference

**Status:** v0.1 - Finalized  
**Date:** 2025-11-15  
**Goal Grid Task:** P1-T7  
**Audience:** Developers using Northroot SDK or integrating with the engine

This guide explains the canonical forms used in Northroot and when to use each one. For technical details on hashing and domain separation, see [`docs/specs/hashing-and-domain-separation.md`](../specs/hashing-and-domain-separation.md).

## Quick Reference

| Form | Standard | Use Case | Location |
|------|----------|----------|----------|
| **CBOR Deterministic** | RFC 8949 | Receipts, all structured data | `northroot-receipts/src/canonical/` |
| **JSON Canonical (JCS)** | RFC 8785 | Engine-internal Merkle trees | `northroot-engine/src/commitments.rs` |
| **JSON (via Adapter)** | N/A | API boundaries only | `northroot-receipts/src/adapters/json.rs` |

## CBOR Deterministic Encoding (RFC 8949)

**Primary canonical form for all receipts and structured data.**

### When to Use

- ✅ Computing receipt hashes
- ✅ Serializing receipts for storage
- ✅ Any structured data that needs deterministic hashing
- ✅ Cross-organizational verification

### Rules

1. **Key Sorting**: Map keys sorted by canonical CBOR byte order (not UTF-8)
2. **Minimal Integers**: Integers encoded in minimal form
3. **No Indefinite-Length Items**: All items have definite length
4. **Exclusion**: `sig` and `hash` fields excluded from canonical body before hashing
5. **Hash Format**: `sha256:<64hex>` (SHA-256 hash with "sha256:" prefix)

### Example

```rust
use northroot_receipts::canonical::encode_canonical;
use serde_json::json;

let data = json!({
    "b": 2,
    "a": 1,
    "c": [3, 4]
});

let canonical_bytes = encode_canonical(&data)?;
// Keys are sorted: a, b, c
// Result is deterministic CBOR bytes
```

### Python Example

```python
import northroot as nr

# Receipts are automatically canonicalized
receipt = nr.record_work(
    workload_id="example",
    payload={"b": 2, "a": 1}  # Keys will be sorted in canonical form
)

# Hash is computed from canonical CBOR
hash_value = receipt.get_hash()  # sha256:...
```

### Common Pitfalls

❌ **Don't assume JSON key order matters**  
Keys are sorted in CBOR canonical form, not JSON order.

❌ **Don't include `sig` or `hash` in canonical body**  
These fields are excluded before hashing.

✅ **Do use CBOR for all receipt operations**  
JSON is only available through adapters at API boundaries.

## JSON Canonicalization (JCS - RFC 8785)

**Used internally for Merkle tree computations only.**

### When to Use

- ✅ Merkle tree leaf hashing (engine-internal)
- ✅ Legacy engine computations
- ❌ **NOT for receipts** - receipts always use CBOR

### Rules

1. **Key Sorting**: Object keys sorted lexicographically (UTF-8)
2. **No Whitespace**: Compact JSON representation
3. **Canonical Numbers**: Standard JSON number representation
4. **Stable Order**: Array order preserved, object keys sorted

### Example

```rust
use northroot_engine::commitments::jcs;
use serde_json::json;

let data = json!({
    "b": 2,
    "a": 1
});

let canonical_json = jcs(&data);
// Result: {"a":1,"b":2} (sorted keys, no whitespace)
```

### When NOT to Use

❌ **Don't use JCS for receipts**  
Receipts must use CBOR canonicalization.

❌ **Don't use JCS for new code**  
Prefer CBOR for all new structured data.

## JSON Adapter (API Boundaries Only)

**JSON is available only through adapter layer for external APIs.**

### When to Use

- ✅ Python SDK API boundaries
- ✅ REST API responses
- ✅ Human-readable debugging
- ❌ **NOT for internal computations** - always use CBOR

### Example

```rust
use northroot_receipts::adapters::json;

// Convert receipt to JSON for API response
let receipt_json = json::receipt_to_json(&receipt)?;

// Parse JSON receipt (from API)
let receipt = json::receipt_from_json(&receipt_json)?;
```

### Python Example

```python
import northroot as nr
import json

# Receipts can be serialized to JSON for API responses
receipt = nr.record_work(...)
receipt_dict = receipt.to_dict()  # Returns Python dict

# JSON serialization (for API)
receipt_json = json.dumps(receipt_dict)
```

## Receipt Hash Computation

**Special rules for receipt hashing.**

### Exclusion Rules

The following fields are **excluded** from canonical body before hashing:

- `sig` - Signature (computed after hash)
- `hash` - Hash field itself (circular dependency)

### Process

1. Serialize receipt to CBOR Value
2. Remove `sig` and `hash` fields
3. Canonicalize remaining fields (sort keys, etc.)
4. Encode to canonical CBOR bytes
5. Compute SHA-256 hash
6. Format as `sha256:<64hex>`

### Example

```rust
use northroot_receipts::Receipt;

let receipt = /* ... */;
let hash = receipt.compute_hash()?;
// Hash computed from canonical CBOR (excluding sig and hash fields)
```

## Verification

### Validating Canonical Forms

```rust
use northroot_receipts::canonical::validate_cbor_deterministic;

// Validate CBOR bytes are deterministic
validate_cbor_deterministic(&cbor_bytes)?;
```

### Hash Format Validation

```rust
use northroot_receipts::canonical::validate_hash_format;

// Validate hash format: sha256:<64hex>
assert!(validate_hash_format("sha256:abc123..."));
```

## Common Patterns

### Pattern 1: Receipt Creation and Hashing

```rust
use northroot_receipts::Receipt;

// Create receipt
let mut receipt = Receipt { /* ... */ };

// Compute hash (automatically uses CBOR canonicalization)
receipt.hash = receipt.compute_hash()?;

// Hash is now: sha256:<64hex>
```

### Pattern 2: Data Shape Hashing

```rust
use northroot_engine::delta::data_shape::compute_data_shape_hash_from_bytes;

let data = b"some binary data";
let chunk_scheme = /* ... */;
let shape_hash = compute_data_shape_hash_from_bytes(data, chunk_scheme)?;
// Returns: sha256:<64hex>
```

### Pattern 3: Method Shape Hashing

```rust
use northroot_engine::delta::method_shape::compute_method_shape_hash_from_signature;

let signature = vec!["arg1", "arg2"];
let method_hash = compute_method_shape_hash_from_signature(&signature)?;
// Returns: sha256:<64hex>
```

## Migration Notes

### From JCS to CBOR

If you have legacy code using JCS:

1. **Receipts**: Already migrated to CBOR (automatic)
2. **Engine internals**: JCS still used for Merkle trees (acceptable)
3. **New code**: Always use CBOR

### Hash Changes

⚠️ **Breaking Change**: Receipt hashes changed when migrating from JCS to CBOR.

- Old receipts (JCS): Different hash format
- New receipts (CBOR): Current hash format
- Version field in receipt indicates canonicalization method

## Best Practices

### ✅ Do

- Use CBOR canonicalization for all receipts
- Use `encode_canonical()` for deterministic encoding
- Use `compute_hash()` for receipt hashing (handles exclusions)
- Validate hash format before storing
- Use JSON adapters only at API boundaries

### ❌ Don't

- Don't manually sort JSON keys (use canonical functions)
- Don't include `sig`/`hash` in canonical body
- Don't use JCS for receipts
- Don't assume JSON key order matters
- Don't mix canonicalization methods

## Troubleshooting

### Hash Mismatch

**Problem**: Receipt hash doesn't match expected value.

**Solutions**:
1. Ensure `sig` and `hash` fields are excluded
2. Verify keys are sorted (use canonical functions)
3. Check for non-deterministic data (timestamps, random values)
4. Validate CBOR is deterministic: `validate_cbor_deterministic()`

### Non-Deterministic Encoding

**Problem**: Same data produces different hashes.

**Solutions**:
1. Use `encode_canonical()` instead of manual encoding
2. Ensure all map keys are sorted
3. Check for floating-point precision issues
4. Verify no indefinite-length CBOR items

### JSON vs CBOR Confusion

**Problem**: Using JSON when CBOR is required.

**Solutions**:
1. Receipts: Always use CBOR
2. API boundaries: Use JSON adapters
3. Internal computations: Use CBOR
4. Debugging: Use CBOR Diagnostic Notation (CDN)

## References

- **CBOR Deterministic Encoding**: [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html)
- **JSON Canonicalization**: [RFC 8785 (JCS)](https://www.rfc-editor.org/rfc/rfc8785.html)
- **Hashing Rules**: [`docs/specs/hashing-and-domain-separation.md`](../specs/hashing-and-domain-separation.md)
- **Implementation**: `northroot-receipts/src/canonical/`

## Quick Checklist

When working with canonical forms:

- [ ] Receipts use CBOR canonicalization
- [ ] `sig` and `hash` fields excluded from hash computation
- [ ] Keys are sorted (use canonical functions, don't sort manually)
- [ ] Hash format is `sha256:<64hex>`
- [ ] JSON only used at API boundaries (via adapters)
- [ ] CBOR validated as deterministic before hashing

