# ADR-006: Signature Verification Strategy

**Date:** 2025-11-08  
**Status:** Accepted  
**Context:** Need to document signature verification implementation, DID resolution approach, and key management

## Context

Receipts include optional signatures for cryptographic proof over the receipt hash. We need to implement signature verification in the engine (per ADR-001) and make decisions about:
- Signature algorithm support
- DID resolution strategy
- Key management approach
- Multi-signature handling

## Decision

### Signature Algorithm

**Primary algorithm: Ed25519**

- Ed25519 is the default and primary supported signature algorithm
- Signatures are base64url-encoded over the receipt hash
- Signature format: `{alg: "ed25519", kid: "did:key:...", sig: "base64url..."}`

**Future algorithms:**
- Other algorithms (e.g., RSA, ECDSA) can be added in future versions
- Algorithm is specified in `sig.alg` field

### DID Resolution

**Basic `did:key` support (v0.1)**

- Currently supports only `did:key` format
- Resolution extracts Ed25519 public key from base58btc-encoded multibase
- Format: `did:key:z<base58-encoded-key>`
- Key encoding: `0xed01` (2 bytes) + public key (32 bytes) = 34 bytes total

**Future DID methods:**
- Full DID resolver supporting multiple methods (e.g., `did:web`, `did:ion`)
- DID document resolution and key rotation
- Key revocation lists

### Key Management

**Current approach:**
- Keys are resolved on-demand from DID identifiers
- No key caching (each verification resolves the key)
- No key rotation support (future)

**Future enhancements:**
- Key caching for performance
- Key rotation and revocation
- Trust anchors and key registries

### Multi-Signature Support

**Current:**
- Receipts support a single signature (`sig: Option<Signature>`)
- `verify_signature()` verifies the single signature
- `verify_all_signatures()` is a placeholder for future multi-sig support

**Future:**
- Multiple signatures in receipt (array of signatures)
- Signature aggregation
- Threshold signatures

### Implementation Location

- **Module**: `northroot-engine/src/signature.rs`
- **Dependencies**: `ed25519-dalek`, `base64`, `bs58`
- **Error type**: `SignatureError` enum

## Consequences

### Pros

- **Simple initial implementation**: Basic `did:key` support is straightforward
- **Extensible**: Can add more algorithms and DID methods later
- **Standard algorithms**: Ed25519 is widely supported and secure
- **Clear error handling**: `SignatureError` provides detailed failure reasons

### Cons

- **Limited DID support**: Only `did:key` initially (not a problem for v0.1)
- **No key caching**: May be slow for repeated verifications (can optimize later)
- **No key rotation**: Keys cannot be rotated without changing DID (future enhancement)

## Alternatives

**Alternative 1:** Full DID resolver from the start
- Rejected: Over-engineering for v0.1, adds unnecessary complexity

**Alternative 2:** Multiple signature algorithms from the start
- Rejected: Ed25519 is sufficient for initial version; others can be added incrementally

**Alternative 3:** Key caching and rotation from the start
- Rejected: Premature optimization; can be added when needed

## Implementation Details

### Signature Verification Flow

1. Extract signature from receipt (if present)
2. Validate hash format (`sha256:<64hex>`)
3. Decode signature bytes (base64url)
4. Resolve public key from DID (`did:key` format)
5. Verify signature over receipt hash using Ed25519

### DID Key Resolution

1. Parse `did:key:` prefix
2. Extract multibase-encoded key (after `z` prefix)
3. Decode base58btc
4. Verify Ed25519 prefix (`0xed01`)
5. Extract 32-byte public key

### Error Handling

- `MissingSignature`: Receipt has no signature
- `UnsupportedAlgorithm`: Algorithm not supported (e.g., "rsa")
- `DidResolutionFailed`: DID resolution failed (invalid format, unsupported method)
- `InvalidFormat`: Signature or hash format is invalid
- `VerificationFailed`: Signature verification failed (invalid signature)

## Migration

No migration needed - this is the initial implementation. Future changes:
- Adding new signature algorithms: MINOR version (additive)
- Changing DID resolution: MINOR version (if backward compatible) or MAJOR (if breaking)
- Multi-signature support: MINOR version (additive)

## References

- [ADR-001: Receipts vs Engine Boundaries](ADR-001-receipts-vs-engine.md)
- [Ed25519 Specification](https://ed25519.cr.yp.to/)
- [DID Key Specification](https://w3c-ccg.github.io/did-method-key/)
- [RFC 8032: EdDSA](https://tools.ietf.org/html/rfc8032)

