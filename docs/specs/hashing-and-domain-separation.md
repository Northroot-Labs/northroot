# Hashing and Domain Separation Rules

**Status:** Finalized for v0.1  
**Date:** 2025-11-15  
**Goal Grid Task:** P1-T2

This document defines the hashing and domain separation rules used throughout the Northroot engine. These rules ensure deterministic, collision-resistant hashing across all engine components.

## Core Principles

1. **Determinism**: Same input always produces same hash
2. **Domain Separation**: Different contexts use different hash prefixes to prevent collisions
3. **Canonicalization First**: Always canonicalize before hashing
4. **SHA-256**: All hashes use SHA-256 with `sha256:` prefix format

## Hash Format

All hashes are returned in the format:
```
sha256:<64-hex-characters>
```

Example: `sha256:abc123def456...` (64 hex characters, 256 bits)

## Domain Separation Patterns

### 1. Receipt Canonicalization

**Domain**: Receipt structure hashing  
**Method**: CBOR Deterministic Encoding (RFC 8949)  
**Location**: `northroot-receipts/src/canonical/`

- Receipts are serialized to CBOR using deterministic encoding
- `sig` and `hash` fields are excluded before hashing
- Hash format: `sha256:<hash>`

**Function**: `Receipt::compute_hash()`

### 2. Set Roots (Order-Independent)

**Domain**: Unordered set commitments  
**Method**: Sorted elements joined with `"|"` separator  
**Location**: `northroot-engine/src/commitments.rs::commit_set_root()`

- Elements are sorted lexicographically
- Joined with `"|"` separator
- Hashed with SHA-256
- Format: `sha256:<hash>`

**Example**:
```rust
let set = vec!["c", "a", "b"];
let root = commit_set_root(&set); // Sorted: "a|b|c", then hashed
```

### 3. Sequence Roots (Order-Dependent)

**Domain**: Ordered sequence commitments  
**Method**: Elements joined with `"|"` separator (preserves order)  
**Location**: `northroot-engine/src/commitments.rs::commit_seq_root()`

- Elements joined in original order with `"|"` separator
- Hashed with SHA-256
- Format: `sha256:<hash>`

**Example**:
```rust
let seq = vec!["a", "b", "c"];
let root = commit_seq_root(&seq); // "a|b|c", then hashed
```

### 4. Merkle Tree Roots

**Domain**: Merkle tree commitments (RFC-6962 style)  
**Method**: Prefix bytes for domain separation  
**Location**: `northroot-engine/src/rowmap.rs`

- **Leaf hash**: `H(0x00 || cbor_canonical({k, v}))`
- **Parent hash**: `H(0x01 || left_hash || right_hash)`
- **Empty tree root**: `H(0x00 || "")`

**Prefix Bytes**:
- `0x00`: Leaf nodes
- `0x01`: Internal/parent nodes

### 5. Data Shape Hashes

**Domain**: Input data shape commitments  
**Method**: Hash of canonical data representation  
**Location**: `northroot-engine/src/delta/data_shape.rs`

- Data is chunked according to `ChunkScheme`
- Each chunk is hashed
- Chunk hashes form a set (order-independent)
- Set root becomes the data shape hash

**Format**: `sha256:<hash>`

### 6. Method Shape Hashes

**Domain**: Method/code shape commitments  
**Method**: Hash of method signature or code  
**Location**: `northroot-engine/src/delta/method_shape.rs`

- Method signature is canonicalized
- Or method code is hashed directly
- Format: `sha256:<hash>`

### 7. Execution Roots

**Domain**: Execution state commitments  
**Method**: Multiple roots computed from span commitments  
**Location**: `northroot-engine/src/execution/state.rs`

- **Identity root**: Hash of identity context
- **Trace set root**: Set root of all span commitments in trace
- **Trace seq root**: Sequence root of spans (if order matters)

**Format**: `sha256:<hash>`

### 8. PAC (Proof-Addressable Cache) Keys

**Domain**: Cache key computation for reuse  
**Method**: Hash of method shape + data shape  
**Location**: `northroot-engine/src/delta/pac.rs`

- Combines method shape root and data shape hash
- Format: `sha256:<hash>`

## Canonicalization Rules

### CBOR Canonicalization (RFC 8949)

**Used for**: Receipts, Merkle tree leaves, all structured data

- Deterministic encoding (sorted keys, fixed-length encoding)
- No indefinite-length items
- Canonical integer encoding
- Canonical float encoding (if used)

**Location**: `northroot-receipts/src/canonical/`

### JSON Canonicalization (JCS - RFC 8785)

**Used for**: Engine-internal computations, Merkle tree leaves (legacy)

- Sorted keys
- No whitespace
- Canonical number representation

**Location**: `northroot-engine/src/commitments.rs::jcs()`

**Note**: JCS is used internally for Merkle tree computations. Receipts always use CBOR.

## Hash Computation Functions

### `sha256_prefixed(bytes: &[u8]) -> String`

Compute SHA-256 hash and return with `sha256:` prefix.

**Input**: Raw bytes  
**Output**: `sha256:<64-hex-chars>`

### `cbor_hash(value: &T) -> Result<String, Error>`

Compute hash of CBOR canonical representation.

**Input**: Any serializable value  
**Output**: `sha256:<64-hex-chars>`

### `commit_set_root(elements: &[String]) -> String`

Compute order-independent set root.

**Input**: Slice of strings  
**Output**: `sha256:<64-hex-chars>`

### `commit_seq_root(elements: &[String]) -> String`

Compute order-dependent sequence root.

**Input**: Slice of strings  
**Output**: `sha256:<64-hex-chars>`

## Collision Prevention

Domain separation prevents hash collisions by:

1. **Different prefixes**: Merkle trees use `0x00`/`0x01` prefixes
2. **Different separators**: Sets vs sequences use `"|"` but with different ordering
3. **Different canonicalization**: CBOR vs JCS for different contexts
4. **Explicit context**: Each hash function is context-specific

## Examples

### Receipt Hash

```rust
use northroot_receipts::Receipt;

let receipt = /* ... */;
let hash = receipt.compute_hash()?;
// Returns: "sha256:abc123..."
```

### Set Root

```rust
use northroot_engine::commitments::commit_set_root;

let elements = vec!["chunk1", "chunk2", "chunk3"];
let root = commit_set_root(&elements);
// Sorted: "chunk1|chunk2|chunk3", then hashed
```

### Merkle Tree Leaf

```rust
use northroot_engine::rowmap::MerkleRowMap;

let mut map = MerkleRowMap::new();
map.insert("key1", value1)?;
let root = map.root(); // H(0x00 || cbor({k, v}))
```

## Stability Guarantees

For v0.1, these rules are **frozen**:

- ✅ SHA-256 algorithm (no changes)
- ✅ `sha256:` prefix format (no changes)
- ✅ CBOR canonicalization (RFC 8949) (no changes)
- ✅ Domain separation patterns (no changes)
- ✅ Merkle tree prefix bytes (0x00/0x01) (no changes)

## Migration Notes

If hash algorithms need to change in future versions:

1. Version tags in receipts indicate hash algorithm
2. Receipts include `version` field for compatibility
3. Hash format may include algorithm identifier (future)

## References

- **CBOR Deterministic Encoding**: RFC 8949
- **JSON Canonicalization**: RFC 8785 (JCS)
- **Merkle Tree Domain Separation**: RFC 6962 (inspired)
- **SHA-256**: FIPS 180-4

