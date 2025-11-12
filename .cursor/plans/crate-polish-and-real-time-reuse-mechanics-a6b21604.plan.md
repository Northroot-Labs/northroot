<!-- a6b21604-baa0-4c57-beb4-e67d7ec45b4c 1556afa8-5c40-4d04-af43-f3b135954280 -->
# Core Engine Upgrade to v0.1.0: Hybrid ByteStream/RowMap Architecture

## Overview

This plan upgrades the core engine to support the hybrid ByteStream/RowMap architecture with unified proof structure and semantic overlays. This upgrade establishes the foundation for v0.1.0 stable release.

**Key Principles:**

- **Unify substrate**: Everything is bytes; add semantic overlays when helpful
- **Portable reuse**: Decisions driven by PAC keys, never raw data
- **Tight boundaries**: Receipts = types & validation; Engine = execution & roots; Resolver = private pointers
- **Hybrid structure**: Pure bytestreams over Merkle Row-Map overlays for tabular and unstructured data
- **Pragmatic canonicalization**: CBOR for MRM (already working, per ADR-002), JCS only at API boundaries (via adapters)

**Breaking Changes:**

- DataShape becomes enum (ByteStream | RowMap) instead of simple hash
- MerkleRowMap domain separation changes from string prefixes ("leaf:", "node:") to byte prefixes (0x00/0x01) per RFC-6962
- ExecutionPayload.data_shape_hash computation changes (must use DataShape enum)
- New modules: shapes.rs, cas.rs, rowmap.rs, resolver.rs
- Encrypted locators replace plain artifact_refs in storage

**Note**: SDK plans deferred - focus on core system readiness.

## Phase 1: DataShape Enum and Unified Proof Structure

### 1.1 Create DataShape Enum (Engine-Internal)

**File:** `crates/northroot-engine/src/shapes.rs` (new file)

**Core Enum:**

```rust
/// Data shape: unified representation of data for proof computation
/// 
/// Everything is bytes at the substrate; semantic overlays added when helpful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataShape {
    /// ByteStream: storage-level view (fast, universal)
    ByteStream {
        /// Merkle root over chunk list (RFC-6962 style)
        manifest_root: String,  // sha256:<64hex>
        /// Total bytes
        manifest_len: u64,
        /// Chunking scheme
        chunk_scheme: ChunkScheme,
    },
    /// RowMap: semantic view for structured compute (fine-grained deltas)
    RowMap {
        /// Merkle Row-Map root over {k -> v_or_ptr}
        merkle_root: String,  // sha256:<64hex>
        /// Number of rows
        row_count: u64,
        /// Key format (e.g., sha256:<64hex>)
        key_fmt: KeyFormat,
        /// Value representation
        value_repr: RowValueRepr,
    },
}

/// Chunking scheme for ByteStream
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkScheme {
    /// Content-Defined Chunking (Rabin fingerprinting)
    CDC { avg_size: u64 },
    /// Fixed-size chunks
    Fixed { size: u64 },
}

/// Key format for RowMap
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyFormat {
    /// SHA-256 hash format
    Sha256Hex,
}

/// Value representation for RowMap
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowValueRepr {
    /// Numeric value
    Number,
    /// String value
    String,
    /// Pointer to ByteStream chunk
    Pointer,
}
```

**Breaking Change:** This is a new engine-internal type. Receipts still use `data_shape_hash: String` (computed from DataShape).

### 1.2 Compute Data Shape Hash from DataShape

**File:** `crates/northroot-engine/src/shapes.rs`

**Function:**

```rust
/// Compute data shape hash from DataShape enum
/// 
/// Returns sha256:<64hex> format for use in receipts.
pub fn compute_data_shape_hash(shape: &DataShape) -> String {
    use sha2::{Digest, Sha256};
    use crate::commitments::jcs;
    use serde_json::json;
    
    let canonical = match shape {
        DataShape::ByteStream { manifest_root, manifest_len, chunk_scheme } => {
            json!({
                "kind": "bytestream",
                "manifest_root": manifest_root,
                "manifest_len": manifest_len,
                "chunk_scheme": chunk_scheme_to_json(chunk_scheme),
            })
        },
        DataShape::RowMap { merkle_root, row_count, key_fmt, value_repr } => {
            json!({
                "kind": "rowmap",
                "merkle_root": merkle_root,
                "row_count": row_count,
                "key_fmt": key_fmt_to_json(key_fmt),
                "value_repr": value_repr_to_json(value_repr),
            })
        },
    };
    
    // JCS canonicalization (sorted keys) for engine-internal computation
    let jcs_bytes = jcs(&canonical);
    sha256_prefixed(jcs_bytes.as_bytes())
}
```

**Breaking Change:** ExecutionPayload.data_shape_hash must now be computed from DataShape enum, not raw data.

### 1.3 Extend ExecutionPayload with Output Digest and Manifest Root

**File:** `crates/northroot-receipts/src/lib.rs`

**Semantic Clarification:**

- **`output_digest`**: Flat SHA-256 hash of **materialized output bytes** (commitment only, for fast exact-hit cache). Format: "sha256:<64hex>". Tenants keep actual outputs; northroot stores only the commitment.
- **`manifest_root`**: Optional Merkle root over output subparts (files, partitions). Used when proving/recomputing at subpart granularity. Separate from existing `merkle_root` field (which is for integrity verification).

**Changes:**

- Add `output_digest: Option<String>` to `ExecutionPayload` (format: "sha256:<64hex>")
- Add `manifest_root: Option<[u8; 32]>` to `ExecutionPayload` (Merkle root over output subparts, binary format)
- Add `output_mime_type: Option<String>` to `ExecutionPayload` (e.g., "application/parquet", "application/json")
- Add `output_size_bytes: Option<u64>` to `ExecutionPayload` (for tracking)
- Keep existing `merkle_root: Option<[u8; 32]>` for integrity verification (different purpose)

**Rationale:**

- Output digest: Commitment to materialized bytes for fast exact-hit cache lookup
- Tenants store actual outputs; northroot only stores commitments
- Manifest root: Enables partial reuse proofs (e.g., reuse 3 of 5 parquet files)
- Clear separation: output_digest (flat) vs manifest_root (Merkle) vs merkle_root (integrity)

### 1.4 Add Encrypted Locator Reference

**File:** `crates/northroot-receipts/src/lib.rs`

**Key Design Decision:** Receipts contain encrypted locator references, not plain artifact_refs. Encryption key managed by tenant. Northroot stores encrypted locators; tenant decrypts via Resolver API.

**New Type:**

```rust
/// Encrypted locator reference: tenant-scoped, encrypted pointer
/// 
/// Northroot stores encrypted locators; tenant decrypts via Resolver API.
/// Actual storage locations never exposed to northroot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedLocatorRef {
    /// Encrypted locator data (tenant-specific encryption)
    pub encrypted_data: Vec<u8>,
    /// Content hash for verification (sha256 format: "sha256:<64hex>")
    pub content_hash: String,
    /// Encryption scheme identifier (e.g., "aes256-gcm", "tenant-key")
    pub encryption_scheme: String,
}
```

**Changes to ExecutionPayload:**

- Add `input_locator_refs: Option<Vec<EncryptedLocatorRef>>` to `ExecutionPayload`
- Add `output_locator_ref: Option<EncryptedLocatorRef>` to `ExecutionPayload`

**Rationale:** Privacy-preserving design. Northroot stores encrypted locators; tenant decrypts via Resolver API. Actual storage locations never exposed.

## Phase 2: Update MerkleRowMap Domain Separation (RFC-6962)

### 2.1 Refactor MerkleRowMap to rowmap.rs

**File:** `crates/northroot-engine/src/rowmap.rs` (new file, refactor from execution/merkle_row_map.rs)

**Breaking Change:** Domain separation changes from string prefixes to byte prefixes per RFC-6962.

**Current:**

```rust
// Leaf hash = H("leaf:" || cbor_canonical({k, v}))
// Parent hash = H("node:" || left || right)
// Empty tree = H("leaf:" || "")
```

**New (RFC-6962 style):**

```rust
// Leaf hash = H(0x00 || cbor_canonical({k, v}))
// Parent hash = H(0x01 || left || right)
// Empty tree = H(0x00 || "")
```

**Changes:**

- Move `MerkleRowMap` from `execution/merkle_row_map.rs` to `rowmap.rs`
- Update `compute_root()` to use byte prefixes (0x00, 0x01) instead of string prefixes
- **Keep CBOR for leaf canonicalization** (pragmatic - already working, per ADR-002)
- Update empty tree root computation
- Update all tests and vectors

**Rationale:**

- RFC-6962 standard domain separation (byte prefixes)
- CBOR already working and aligned with ADR-002
- JCS only needed at API boundaries (already handled by adapters)
- Breaking change: All MerkleRowMap roots will change

### 2.2 Row Normalization

**File:** `crates/northroot-engine/src/rowmap.rs`

**Function:**

```rust
/// Normalize row for deterministic hashing
/// 
/// Rules: UTF-8, LF line endings, no trailing spaces, header excluded
pub fn normalize_row(row: &[u8]) -> Result<Vec<u8>, RowMapError> {
    // Normalize per spec:
    // - UTF-8 encoding
    // - LF line endings (normalize CRLF to LF)
    // - Remove trailing spaces
    // - Exclude header row
}
```

**Rationale:** Deterministic row normalization ensures same data produces same keys.

### 2.3 Merkle Frontier for Delta Recompute

**File:** `crates/northroot-engine/src/rowmap.rs`

**New Structure:**

```rust
/// Merkle frontier: subset of internal node hashes for delta recompute
/// 
/// Limits recompute to touched subtrees only.
pub struct MerkleFrontier {
    /// Internal node hashes along paths from changed leaves to root
    pub internal_nodes: Vec<[u8; 32]>,
    /// Changed leaf keys
    pub changed_keys: Vec<String>,
}

impl MerkleRowMap {
    /// Compute frontier for delta update
    /// 
    /// Returns internal node hashes needed to recompute only touched subtrees.
    pub fn compute_frontier(
        &self,
        changed_keys: &[String],
    ) -> Result<MerkleFrontier, RowMapError> {
        // 1. Identify paths from changed leaves to root
        // 2. Collect internal node hashes along those paths
        // 3. Return frontier (unchanged subtrees can be reused)
    }
    
    /// Apply delta using frontier
    /// 
    /// Only recomputes touched subtrees using frontier nodes.
    pub fn apply_delta_with_frontier(
        &mut self,
        delta: &DeltaUpdate,
        frontier: &MerkleFrontier,
    ) -> Result<String, RowMapError> {
        // 1. Apply changes to entries
        // 2. Recompute only affected paths using frontier
        // 3. Return new root
    }
}
```

**Rationale:** Efficient delta recompute - only touched subtrees need recomputation.

## Phase 3: ByteStream Manifest Builder (CAS Module)

### 3.1 Create CAS Module for ByteStream

**File:** `crates/northroot-engine/src/cas.rs` (new file)

**Core Functions:**

```rust
/// Build ByteStream manifest from chunks
/// 
/// Chunk → hash → ordered list → RFC-6962 Merkle → manifest_root
pub fn build_bytestream_manifest(
    chunks: &[Chunk],
    scheme: ChunkScheme,
) -> Result<ByteStreamManifest, CasError> {
    // 1. Compute chunk hashes
    // 2. Sort chunks by hash (deterministic order)
    // 3. Build RFC-6962 Merkle tree:
    //    - Leaf hash = H(0x00 || chunk_hash)
    //    - Parent hash = H(0x01 || left || right)
    //    - Odd nodes promoted
    // 4. Return manifest with manifest_root
}

/// Chunk for ByteStream
pub struct Chunk {
    pub id: String,  // sha256:<64hex>
    pub offset: u64,
    pub len: u64,
}

/// ByteStream manifest
pub struct ByteStreamManifest {
    pub manifest_root: String,  // sha256:<64hex>
    pub manifest_len: u64,
    pub chunks: Vec<Chunk>,
    pub chunk_scheme: ChunkScheme,
}
```

**Rationale:** ByteStream is the universal substrate. All data starts as bytes.

### 3.2 Implement CDC Chunking (Rabin Fingerprinting)

**File:** `crates/northroot-engine/src/cas.rs`

**Function:**

```rust
/// Content-Defined Chunking using Rabin fingerprinting
/// 
/// Average chunk size configurable (default 64KiB).
pub fn chunk_by_cdc(
    data: &[u8],
    avg_size: u64,
) -> Result<Vec<Chunk>, CasError> {
    // Implement Rabin fingerprinting algorithm
    // Return chunks with deterministic boundaries
}

/// Fixed-size chunking
pub fn chunk_by_fixed(
    data: &[u8],
    size: u64,
) -> Result<Vec<Chunk>, CasError> {
    // Fixed-size chunking
}
```

**Rationale:** CDC enables stable chunking even when data shifts slightly.

## Phase 4: Privacy-Preserving Resolver API

### 4.1 Define Resolver Trait

**File:** `crates/northroot-engine/src/resolver.rs` (new file)

**Core Trait:**

```rust
/// Privacy-preserving resolver for reconciling proof index with tenant artifacts
/// 
/// This trait is implemented by tenants and stays private. Never exposed publicly.
/// The proof index only sees encrypted locators, never actual storage locations.
pub trait ArtifactResolver: Send + Sync {
    /// Decrypt and resolve encrypted locator to actual storage location
    /// 
    /// Returns location information that stays in tenant's control.
    /// Proof index never sees this - only the resolver implementation.
    fn resolve_locator(
        &self,
        encrypted_ref: &EncryptedLocatorRef,
    ) -> Result<ArtifactLocation, ResolverError>;
    
    /// Encrypt and store artifact, return encrypted locator reference
    /// 
    /// Tenant stores artifact in their storage and returns encrypted reference
    /// for inclusion in receipts.
    fn store_artifact(
        &self,
        data: &[u8],
        metadata: &ArtifactMetadata,
    ) -> Result<EncryptedLocatorRef, ResolverError>;
    
    /// Batch resolve (for efficiency)
    fn resolve_locators_batch(
        &self,
        encrypted_refs: &[EncryptedLocatorRef],
    ) -> Result<Vec<ArtifactLocation>, ResolverError>;
}

/// Artifact location (private to tenant, never in receipts)
pub struct ArtifactLocation {
    /// Storage type: "s3", "gcs", "local", etc.
    pub storage_type: String,
    /// Location path/URI
    pub location: String,
    /// Content hash for verification
    pub content_hash: String,
    /// Optional metadata (credentials, region, etc.) - tenant-controlled
    pub metadata: Option<serde_json::Value>,
}

/// Artifact metadata for storage
pub struct ArtifactMetadata {
    /// MIME type
    pub mime_type: Option<String>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Optional tenant-specific metadata
    pub custom: Option<serde_json::Value>,
}
```

**Rationale:** Clean separation. Resolver stays in tenant, proof index only sees encrypted locators.

### 4.2 Optional Managed Artifact Cache

**File:** `crates/northroot-engine/src/resolver.rs`

**Extension:**

```rust
/// Optional managed artifact cache (paid add-on)
/// 
/// Northroot can optionally cache hot shards for speed.
/// Tenant can opt-in to managed cache for frequently accessed artifacts.
pub trait ManagedCache: Send + Sync {
    /// Store artifact in managed cache
    fn cache_artifact(
        &self,
        artifact_ref: &EncryptedLocatorRef,
        data: &[u8],
        ttl: Option<u64>,
    ) -> Result<(), CacheError>;
    
    /// Retrieve artifact from managed cache
    fn get_cached_artifact(
        &self,
        artifact_ref: &EncryptedLocatorRef,
    ) -> Result<Option<Vec<u8>>, CacheError>;
}
```

**Rationale:** Optional paid add-on for performance. Default: defer on demand via tenant resolver.

### 4.3 Manifest Root Computation

**File:** `crates/northroot-engine/src/delta/manifest_root.rs` (new file)

**Functions:**

```rust
/// Compute manifest root (Merkle root) over multiple output files/partitions
/// 
/// Used when output consists of multiple parts (e.g., multiple parquet files)
/// and we need to prove reuse of partial results.
pub fn compute_manifest_root(
    parts: &[OutputPart],
) -> Result<[u8; 32], ManifestRootError> {
    // Build Merkle tree over parts using RFC-6962 style
    // Leaf hash = H(0x00 || part_id || part_hash)
    // Parent hash = H(0x01 || left || right)
    // Return root
}

/// Output part for manifest root computation
pub struct OutputPart {
    /// Part identifier (e.g., "file1.parquet", "partition=2025-01-27")
    pub part_id: String,
    /// Content hash of this part (sha256)
    pub part_hash: String,
}
```

**Rationale:** Enables partial reuse proofs when output has multiple files/partitions.

## Phase 5: Summarized Manifests for Fast Overlap

### 5.1 Manifest Summary Structure

**File:** `crates/northroot-engine/src/delta/manifest_summary.rs` (new file)

**Structure:**

```rust
/// Summarized manifest for fast overlap computation
/// 
/// Contains only what's needed for overlap detection, not full manifest.
pub struct ManifestSummary {
    /// PAC key
    pub pac: [u8; 32],
    /// Chunk count
    pub chunk_count: usize,
    /// MinHash sketch (for fast overlap estimation)
    pub minhash_sketch: Vec<u8>,
    /// HyperLogLog cardinality estimate
    pub hll_cardinality: Option<u64>,
    /// Bloom filter (optional, for fast negative checks)
    pub bloom_filter: Option<Vec<u8>>,
}

/// Load manifest summary (fast path)
pub fn load_manifest_summary(
    store: &dyn ReceiptStore,
    chunk_manifest_hash: &[u8; 32],
) -> Result<ManifestSummary, ManifestError> {
    // Load full manifest, extract summary
    // Or store summary separately for faster access
}
```

**Rationale:** First pull summary to compute overlap, only resolve handles for parts we'll actually reuse.

### 5.2 Compaction Tiers

**File:** `crates/northroot-storage/src/compaction.rs` (new file)

**Design:**

- **Tier 1 (Hot)**: Full manifests, summaries, Bloom filters (fast access)
- **Tier 2 (Warm)**: Summaries only, manifests compressed
- **Tier 3 (Cold)**: Summaries only, manifests archived

**Implementation:**

```rust
pub struct CompactionTier {
    pub tier: u8,  // 1=hot, 2=warm, 3=cold
    pub has_full_manifest: bool,
    pub has_summary: bool,
    pub has_bloom_filter: bool,
}

pub fn get_manifest_tier(
    store: &dyn ReceiptStore,
    manifest_hash: &[u8; 32],
) -> Result<CompactionTier, StorageError>;
```

**Rationale:** Optimize storage and access patterns based on usage frequency.

## Phase 6: Storage Extensions

### 6.1 Encrypted Locator Storage

**File:** `crates/northroot-storage/src/traits.rs`

**Add to ReceiptStore trait:**

```rust
/// Store encrypted locator for execution output
fn store_encrypted_locator(
    &self,
    execution_rid: &Uuid,
    encrypted_locator: &EncryptedLocatorRef,
) -> Result<(), StorageError>;

/// Retrieve encrypted locator for execution
fn get_encrypted_locator(
    &self,
    execution_rid: &Uuid,
) -> Result<Option<EncryptedLocatorRef>, StorageError>;
```

**File:** `crates/northroot-storage/src/sqlite.rs`

**Schema Extension:**

```sql
CREATE TABLE IF NOT EXISTS encrypted_locators (
    execution_rid TEXT PRIMARY KEY,
    encrypted_data BLOB NOT NULL,
    content_hash TEXT NOT NULL,
    encryption_scheme TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (execution_rid) REFERENCES receipts(rid)
);

CREATE INDEX idx_locators_content_hash ON encrypted_locators(content_hash);
```

**Rationale:** Store encrypted locators separately from receipts. Tenant decrypts via Resolver API.

### 6.2 Output Digest Storage

**File:** `crates/northroot-storage/src/traits.rs`

**Add to ReceiptStore trait:**

```rust
/// Query receipts by output digest (for fast exact-hit cache lookup)
fn query_by_output_digest(
    &self,
    output_digest: &str,
) -> Result<Vec<Receipt>, StorageError>;

/// Get output digest and encrypted locator for execution
fn get_output_info(
    &self,
    execution_rid: &Uuid,
) -> Result<Option<(String, EncryptedLocatorRef)>, StorageError>;
```

**File:** `crates/northroot-storage/src/sqlite.rs`

**Schema Extension:**

```sql
CREATE TABLE IF NOT EXISTS output_digests (
    execution_rid TEXT PRIMARY KEY,
    output_digest TEXT NOT NULL,
    manifest_root BLOB,
    output_mime_type TEXT,
    output_size_bytes INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (execution_rid) REFERENCES receipts(rid)
);

CREATE INDEX idx_output_digests_digest ON output_digests(output_digest);
```

**Rationale:** Fast lookup by output_digest for exact-hit cache checks.

### 6.3 Manifest Summary Storage

**File:** `crates/northroot-storage/src/sqlite.rs`

**Schema Extension:**

```sql
CREATE TABLE IF NOT EXISTS manifest_summaries (
    manifest_hash BLOB PRIMARY KEY,
    pac BLOB NOT NULL,
    chunk_count INTEGER NOT NULL,
    minhash_sketch BLOB NOT NULL,
    hll_cardinality INTEGER,
    bloom_filter BLOB,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_manifest_summaries_pac ON manifest_summaries(pac);
```

**Rationale:** Store summaries separately for fast overlap computation without loading full manifests.

## Phase 7: Reuse Reconciliation Flow (Core Engine)

### 7.1 Reuse Check with Summarized Manifests

**File:** `crates/northroot-engine/src/delta/reuse.rs` (new file)

**Core Function:**

```rust
/// Reuse reconciliation result
pub struct ReuseReconciliation {
    pub decision: ReuseDecision,
    pub overlap_j: f64,
    pub economic_delta: f64,
    pub previous_execution_rid: Option<Uuid>,
    pub output_digest: Option<String>,
    pub output_locator_ref: Option<EncryptedLocatorRef>,
    pub manifest_root: Option<[u8; 32]>,  // For partial reuse
}

/// Check if previous execution can be reused
pub fn check_reuse(
    store: &dyn ReceiptStore,
    pac: &[u8; 32],
    current_chunks: &ChunkSet,
    cost_model: &CostModel,
) -> Result<ReuseReconciliation, ReuseError> {
    // 1. Find previous execution by PAC
    let prev_receipt = find_reusable_execution(store, pac)?;
    
    if let Some(prev) = prev_receipt {
        // 2. Load manifest summary (fast path)
        let prev_summary = if let Some(manifest_hash) = prev.execution_payload()?.chunk_manifest_hash {
            load_manifest_summary(store, &manifest_hash)?
        } else {
            return Ok(ReuseReconciliation { decision: ReuseDecision::Recompute, ... });
        };
        
        // 3. Fast overlap estimation with MinHash
        let j_est = estimate_overlap_minhash(&current_chunks, &prev_summary.minhash_sketch)?;
        
        // 4. If overlap looks promising, compute exact overlap
        let overlap_j = if j_est > 0.5 {
            // Load full manifest for exact computation
            let prev_manifest = load_chunk_manifest(store, &prev)?;
            let prev_chunks = manifest_to_chunk_set(&prev_manifest)?;
            jaccard_similarity(current_chunks, &prev_chunks)
        } else {
            j_est  // Use estimate
        };
        
        // 5. Make reuse decision
        let (decision, justification) = decide_reuse(overlap_j, cost_model, None);
        let economic_delta = economic_delta(overlap_j, cost_model, None);
        
        // 6. Get output info if reusing
        let (output_digest, output_locator_ref, manifest_root) = if decision == ReuseDecision::Reuse {
            let exec_payload = prev.execution_payload()?;
            (
                exec_payload.output_digest.clone(),
                exec_payload.output_locator_ref.clone(),
                exec_payload.manifest_root,
            )
        } else {
            (None, None, None)
        };
        
        Ok(ReuseReconciliation {
            decision,
            overlap_j,
            economic_delta,
            previous_execution_rid: Some(prev.rid),
            output_digest,
            output_locator_ref,
            manifest_root,
        })
    } else {
        Ok(ReuseReconciliation {
            decision: ReuseDecision::Recompute,
            overlap_j: 0.0,
            economic_delta: 0.0,
            previous_execution_rid: None,
            output_digest: None,
            output_locator_ref: None,
            manifest_root: None,
        })
    }
}
```

**Rationale:** Fast path with summaries, exact path when needed. Returns output info for resolver to load.

## Phase 8: Helper Functions for Shape Hash Computation

### 8.1 Data Shape Hash Helpers

**File:** `crates/northroot-engine/src/delta/data_shape.rs` (new file)

**Functions:**

```rust
/// Compute data shape hash from input file
pub fn compute_data_shape_hash_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<String, DataShapeError>;

/// Compute data shape hash from input bytes
pub fn compute_data_shape_hash_from_bytes(
    data: &[u8],
) -> Result<String, DataShapeError>;

/// Compute data shape hash from multiple inputs (composite)
pub fn compute_data_shape_hash_from_inputs(
    inputs: &[EncryptedLocatorRef],
) -> Result<String, DataShapeError>;
```

**Rationale:** Needed for PAC key computation.

### 8.2 Method Shape Hash Helpers

**File:** `crates/northroot-engine/src/delta/method_shape.rs` (new file)

**Functions:**

```rust
/// Compute method shape hash from code hash and parameters
pub fn compute_method_shape_hash_from_code(
    code_hash: &str,
    params: &serde_json::Value,
) -> Result<String, MethodShapeError>;

/// Compute method shape hash from function signature
pub fn compute_method_shape_hash_from_signature(
    function_name: &str,
    input_types: &[&str],
    output_type: &str,
) -> Result<String, MethodShapeError>;
```

**Rationale:** Needed for PAC key computation.

## Implementation Order

1. **Week 1: Core Extensions**

   - Create DataShape enum
   - Update MerkleRowMap domain separation (0x00/0x01)
   - Extend ExecutionPayload with output_digest, manifest_root, encrypted_locator_refs
   - Update schemas and validation

2. **Week 2: ByteStream and CAS**

   - Implement CAS module with ByteStream manifest builder
   - Implement CDC chunking (Rabin fingerprinting)
   - Add row normalization
   - Implement Merkle frontier

3. **Week 3: Resolver and Storage**

   - Define ArtifactResolver trait
   - Implement encrypted locator storage
   - Add output digest storage
   - Add manifest summary storage

4. **Week 4: Delta and Integration**

   - Implement reuse reconciliation flow
   - Add manifest summary structure
   - Integration tests
   - Update test vectors

## Breaking Changes Summary

1. **MerkleRowMap domain separation**: String prefixes ("leaf:", "node:") → byte prefixes (0x00/0x01)

   - **Impact**: All MerkleRowMap roots will change
   - **Migration**: Regenerate test vectors, update baselines

2. **DataShape becomes enum**: Engine-internal change

   - **Impact**: ExecutionPayload.data_shape_hash computation changes
   - **Migration**: Update all data_shape_hash computations to use DataShape enum

3. **Encrypted locators**: New storage model

   - **Impact**: Storage schema changes, new table
   - **Migration**: Existing receipts without locators remain valid (optional field)

4. **Output digest semantics**: Now commitment to materialized bytes

   - **Impact**: Clarification of purpose (fast exact-hit cache)
   - **Migration**: No breaking change, semantic clarification

## Success Criteria for v0.1.0

- ✅ DataShape enum implemented (ByteStream | RowMap)
- ✅ MerkleRowMap uses RFC-6962 domain separation (0x00/0x01)
- ✅ CBOR canonicalization for MRM leaves (pragmatic, aligned with ADR-002)
- ✅ JCS only at API boundaries (via adapters)
- ✅ Output digest = commitment to materialized bytes (fast exact-hit cache)
- ✅ Encrypted locators stored in northroot
- ✅ Tenants keep actual outputs (privacy-preserving)
- ✅ Optional managed artifact cache (paid add-on)
- ✅ Merkle frontier for efficient delta recompute
- ✅ Summarized manifests for fast overlap
- ✅ Clean split: Receipts (proofs) vs Artifact Catalog (private) vs Resolver (tenant API)
- ✅ ByteStream manifest builder with CDC chunking
- ✅ Unified proof structure with semantic overlays

### To-dos

- [ ] Extend ExecutionPayload with data_locators, output_digest, and output_locator fields
- [ ] Add data_shape_hash computation helpers in northroot-engine/src/delta/data_shape.rs
- [ ] Add method_shape_hash computation helpers in northroot-engine/src/delta/method_shape.rs
- [ ] Extend ReceiptStore trait with store_output_digest, get_output_digest, and query_by_output_digest methods
- [ ] Implement output digest storage in SqliteStore with new table and indexes
- [ ] Add find_reusable_execution and get_reuse_chain methods to ReceiptStore trait
- [ ] Implement reuse query methods in SqliteStore
- [ ] Create reuse reconciliation flow in northroot-engine/src/delta/reuse.rs with check_reuse function
- [ ] Design DataLoader trait interface for SDK (user-provided data loading)
- [ ] Create integration test for full reuse flow (Job 1 -> Job 2 with transparent reuse)