# ADR-002: Canonicalization Strategy (JCS)

**Date:** 2025-11-08 
**Status:** Accepted  
**Context:** Need deterministic serialization for hash computation

## Context

Receipts must have deterministic hashes for verification and composition. Different JSON serializations of the same data can produce different byte sequences, leading to different hashes. We need a canonicalization strategy.

## Decision

Use **JSON Canonicalization Scheme (JCS)** as defined in [RFC 8785](https://tools.ietf.org/html/rfc8785):

1. **Key sorting**: Object keys are sorted lexicographically
2. **No whitespace**: Compact JSON (no extra spaces)
3. **Deterministic formatting**: Consistent representation of numbers, strings, arrays, objects
4. **Exclusion**: `sig` and `hash` fields are excluded from canonical body before hashing

**Implementation location:** `northroot-receipts` crate (canonicalization is part of receipt structure)

**Hash format:** `sha256:<64hex>` (SHA-256 hash with "sha256:" prefix)

## Consequences

### Pros
- Standardized approach (RFC 8785)
- Deterministic hashing for verification
- Enables receipt composition and chaining
- Self-verifying receipts (hash integrity)

### Cons
- Slightly more complex than naive JSON serialization
- Requires careful implementation to match RFC exactly
- All consumers must use same canonicalization

## CBOR Experimental Support

While JCS remains the default and canonical form, we provide experimental CBOR support behind a feature flag for performance evaluation:

### Feature Flag: `c14n_cbor`

When enabled, CBOR deterministic encoding can be used for internal canonicalization. This is **experimental** and subject to change.

### Hash Namespace Separation

To prevent ambiguity and ensure compatibility:

- **`sha256:`** → JCS canonicalization (default, always available)
- **`sha256cbor:`** → CBOR deterministic canonicalization (experimental, requires `c14n_cbor` feature)

### Restrictions

1. **Golden vectors**: Never emit CBOR-based hashes (`sha256cbor:`) into golden test vectors
2. **Public API**: JCS remains the public, canonical form for v0.1
3. **Versioning**: If CBOR becomes default, it's a MAJOR version change (different hashes)
4. **Benchmarking**: Add benchmark harness to measure JCS vs CBOR performance before considering default flip

### Implementation Plan

1. Add `c14n_cbor` feature flag to `northroot-receipts`
2. Implement CBOR deterministic canonicalization alongside JCS
3. Add benchmark suite comparing JCS vs CBOR performance
4. Document feature flag usage and limitations
5. Keep JCS as default for all public interfaces

## Alternatives

**Alternative 1:** Use CBOR with deterministic encoding (as default)
- Rejected: Less human-readable, JSON is more widely supported. However, experimental support via feature flag is acceptable for performance evaluation.

**Alternative 2:** Custom canonicalization rules
- Rejected: Non-standard, harder to verify, potential compatibility issues

**Alternative 3:** No canonicalization (use serde_json default)
- Rejected: Non-deterministic, breaks hash verification

## Migration

No migration needed - this is the initial design. Future changes to canonicalization are breaking (MAJOR version bump).

## References

- [RFC 8785: JSON Canonicalization Scheme](https://tools.ietf.org/html/rfc8785)
- [Receipt Data Model](crates/northroot-receipts/docs/specs/data_model.md)
- [Canonicalization Implementation](crates/northroot-receipts/src/canonical.rs)

