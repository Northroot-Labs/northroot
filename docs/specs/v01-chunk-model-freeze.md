# v0.1 Chunk Model Freeze

**Status:** v0.1 - FROZEN  
**Date:** 2025-11-15  
**Harada Task:** P1-T3  
**Effective:** All v0.1 releases

This document freezes the chunk model (receipt structure and payload types) for v0.1. No breaking changes to these structures are permitted until v0.2.

## Core Model

### Receipt (Unified Envelope)

The `Receipt` struct is the unified envelope for all proof types. It is **frozen** for v0.1.

**Location**: `crates/northroot-receipts/src/lib.rs`

```rust
pub struct Receipt {
    pub rid: Uuid,              // UUIDv7 recommended, UUIDv4 acceptable for v0.1
    pub version: String,        // Envelope version, e.g., "0.1.0"
    pub kind: ReceiptKind,      // One of six kinds (see below)
    pub dom: String,            // sha256:<64hex> commitment of domain shape
    pub cod: String,            // sha256:<64hex> commitment of codomain shape
    pub links: Vec<Uuid>,       // Child receipts for composition (optional)
    pub ctx: Context,           // Execution context
    pub payload: Payload,       // Kind-specific payload
    pub attest: Option<serde_json::Value>, // Optional TEE/container attest
    pub sig: Option<Signature>,  // Detached signature over `hash`
    pub hash: String,           // sha256 of canonical body (without sig/hash)
}
```

**Frozen Fields** (v0.1):
- All fields listed above are **required** (except `attest` and `sig` which are optional)
- Field names are **frozen** (no renaming)
- Field types are **frozen** (no type changes)
- Field order in serialization is **determined by CBOR canonicalization** (sorted keys)

### ReceiptKind (Enum)

The `ReceiptKind` enum defines the six receipt types. It is **frozen** for v0.1.

```rust
pub enum ReceiptKind {
    DataShape,
    MethodShape,
    ReasoningShape,
    Execution,
    Spend,
    Settlement,
}
```

**Frozen Values** (v0.1):
- All six kinds are **required** (no additions, no removals)
- Enum variant names are **frozen** (snake_case in JSON, PascalCase in Rust)
- Serialization format is **frozen** (snake_case in JSON)

### Payload (Enum)

The `Payload` enum contains kind-specific payloads. It is **frozen** for v0.1.

```rust
pub enum Payload {
    DataShape(DataShapePayload),
    MethodShape(MethodShapePayload),
    ReasoningShape(ReasoningShapePayload),
    Execution(ExecutionPayload),
    Spend(SpendPayload),
    Settlement(SettlementPayload),
}
```

**Frozen Structure** (v0.1):
- All six payload variants are **required**
- Tagged union format: `{"kind": "...", "payload": {...}}` in JSON
- CBOR uses tagged encoding

## Payload Types (Frozen)

### 1. DataShapePayload

```rust
pub struct DataShapePayload {
    pub schema_hash: String,        // sha256:<64hex>
    pub sketch_hash: Option<String>, // sha256:<64hex> (optional)
}
```

**Frozen** (v0.1):
- `schema_hash`: Required, format `sha256:<64hex>`
- `sketch_hash`: Optional, format `sha256:<64hex>`

### 2. MethodShapePayload

```rust
pub struct MethodShapePayload {
    pub nodes: Vec<MethodNodeRef>,
    pub edges: Option<Vec<Edge>>,      // Optional in v0.1
    pub root_multiset: String,         // sha256:<64hex>
    pub dag_hash: Option<String>,       // sha256:<64hex> (optional)
}
```

**Frozen** (v0.1):
- `nodes`: Required, non-empty vector
- `edges`: Optional (may be None in v0.1)
- `root_multiset`: Required, format `sha256:<64hex>`
- `dag_hash`: Optional, format `sha256:<64hex>`

### 3. ReasoningShapePayload

```rust
pub struct ReasoningShapePayload {
    pub intent_hash: String,             // sha256:<64hex>
    pub trace_id: Option<String>,        // Optional
    pub nodes: Vec<ReasoningNodeRef>,    // Required
    pub edges: Option<Vec<Edge>>,        // Optional
    pub root_hash: Option<String>,       // sha256:<64hex> (optional)
}
```

**Frozen** (v0.1):
- `intent_hash`: Required, format `sha256:<64hex>`
- `trace_id`: Optional
- `nodes`: Required, non-empty vector
- `edges`: Optional
- `root_hash`: Optional, format `sha256:<64hex>`

### 4. ExecutionPayload

```rust
pub struct ExecutionPayload {
    pub trace_id: String,
    pub method_ref: MethodRef,
    pub data_shape_hash: String,        // sha256:<64hex>
    pub span_commitments: Vec<String>,   // sha256:<64hex> each
    pub roots: ExecutionRoots,
    // ... many optional fields for v0.1
}
```

**Frozen Core Fields** (v0.1):
- `trace_id`: Required
- `method_ref`: Required
- `data_shape_hash`: Required, format `sha256:<64hex>`
- `span_commitments`: Required, non-empty vector
- `roots`: Required

**Optional Fields** (v0.1 - may be None):
- `pac`, `minhash_signature`, `hll_cardinality`, `chunk_manifest_hash`, `merkle_root`, `prev_execution_rid`, `output_digest`, `manifest_root`, `output_mime_type`, `output_size_bytes`, `input_locator_refs`, `output_locator_ref`, `cdf_metadata`, `change_epoch`, `chunk_manifest_size_bytes`

**Note**: Optional fields may be added in v0.2, but core fields are frozen.

### 5. SpendPayload

```rust
pub struct SpendPayload {
    pub meter: Meter,
    pub justification: Option<ReuseJustification>,
    pub wur_refs: Option<Vec<String>>,  // Work unit receipt references
}
```

**Frozen** (v0.1):
- `meter`: Required
- `justification`: Optional
- `wur_refs`: Optional

### 6. SettlementPayload

```rust
pub struct SettlementPayload {
    pub parties: Vec<String>,
    pub net_state: serde_json::Value,   // Flexible for v0.1
    pub settlement_hash: String,        // sha256:<64hex>
}
```

**Frozen** (v0.1):
- `parties`: Required, non-empty vector
- `net_state`: Required (flexible JSON value for v0.1)
- `settlement_hash`: Required, format `sha256:<64hex>`

## Supporting Types (Frozen)

### Context

```rust
pub struct Context {
    pub policy_ref: Option<String>,   // e.g., "pol:standard-v1"
    pub timestamp: String,            // RFC3339 UTC
    pub nonce: Option<String>,        // base64url
    pub determinism: Option<DeterminismClass>,
    pub identity_ref: Option<String>, // e.g., "did:key:..."
}
```

**Frozen** (v0.1):
- All fields optional except `timestamp`
- `timestamp` format: RFC3339 UTC (milliseconds precision)

### DeterminismClass

```rust
pub enum DeterminismClass {
    Strict,        // bit-identical reproducible
    Bounded,       // bounded nondeterminism (e.g., float tolerances)
    Observational, // observational log (no reproducibility claim)
}
```

**Frozen** (v0.1):
- All three variants required
- Serialization: snake_case in JSON

### Signature

```rust
pub struct Signature {
    pub alg: String, // "ed25519" (frozen for v0.1)
    pub kid: String, // DID key id
    pub sig: String, // base64url over canonical body hash
}
```

**Frozen** (v0.1):
- `alg`: Currently only "ed25519" supported
- `kid`: DID format (flexible for v0.1)
- `sig`: base64url encoding

## Serialization Rules (Frozen)

### CBOR (Primary)

- **Format**: CBOR Deterministic Encoding (RFC 8949)
- **Key Sorting**: Canonical CBOR byte order
- **Exclusions**: `sig` and `hash` excluded from canonical body
- **Hash Format**: `sha256:<64hex>`

### JSON (Adapter Only)

- **Format**: Via `northroot-receipts/src/adapters/json.rs`
- **Key Format**: snake_case
- **Tagged Union**: `{"kind": "...", "payload": {...}}`
- **Use Case**: API boundaries only, not for internal storage

## Versioning Strategy

### Envelope Version

The `version` field in `Receipt` indicates the envelope version:
- `"0.1.0"`: v0.1 envelope (frozen)
- `"0.2.0"`: Future version (may add fields, but must maintain backward compatibility)

### Migration Path

For v0.2:
- New optional fields may be added to payloads
- New `ReceiptKind` variants may be added
- Existing fields **must not** be removed or renamed
- Existing field types **must not** change (except to add optional variants)

## Validation Rules (Frozen)

### Hash Validation

- Format: `^sha256:[0-9a-f]{64}$`
- Computed from canonical CBOR (excluding `sig` and `hash`)
- Must match `receipt.hash` field

### UUID Format

- `rid`: UUID format (UUIDv7 recommended, UUIDv4 acceptable for v0.1)
- `links`: Vector of UUIDs

### Hash Format

All hash fields must match: `sha256:<64hex>` (64 hex characters)

## Breaking Changes Prohibited

The following changes are **prohibited** in v0.1:

- ❌ Removing any field from `Receipt` or payload structs
- ❌ Renaming any field
- ❌ Changing field types (except adding optional variants)
- ❌ Removing any `ReceiptKind` variant
- ❌ Changing serialization format (CBOR canonicalization rules)
- ❌ Changing hash computation algorithm (SHA-256)
- ❌ Changing hash format (`sha256:<64hex>`)

## Allowed Changes (v0.1)

The following changes are **allowed** in v0.1 patches:

- ✅ Adding optional fields to payloads (with default `None`)
- ✅ Adding validation logic
- ✅ Improving error messages
- ✅ Performance optimizations (must preserve semantics)
- ✅ Documentation improvements

## References

- **Implementation**: `crates/northroot-receipts/src/lib.rs`
- **Schemas**: `schemas/receipts/`
- **Canonicalization**: `docs/guides/canonical-forms-reference.md`
- **Hashing Rules**: `docs/specs/hashing-and-domain-separation.md`

## Freeze Declaration

**This chunk model is FROZEN for v0.1 as of 2025-11-15.**

All structures, types, and serialization rules defined above are stable and will not change until v0.2. Any proposed changes must go through the ADR process and be approved as breaking changes for a future version.

