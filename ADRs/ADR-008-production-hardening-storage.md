# ADR-008: Proof-Addressable Storage and Incremental Compute Hardening

**Date:** 2025-11-09  
**Status:** Proposed  
**Context:** Delta compute implementation is complete with proof-of-concept examples. To move from demo to production, we need: (1) durable proof storage with content-addressed keys, (2) efficient overlap computation using sketches and manifests, (3) verifiable incremental compute with receipt chains, (4) production-ready retention and verification policies.

## Context

### Current State
- ✅ Delta compute logic implemented (Jaccard similarity, reuse decisions, economic delta)
- ✅ Policy-driven cost models working
- ✅ Examples demonstrate proof of reuse (FinOps, ETL, Analytics)
- ✅ MinHash sketch computation available
- ✅ CBOR deterministic encoding support
- ❌ Examples reconstruct previous state from hardcoded data (not from receipts)
- ❌ No persistent storage for receipts/proofs
- ❌ No content-addressed keys for deterministic lookup
- ❌ No separation between small receipts and large chunk manifests
- ❌ No retention policies or verification modes

### Problem Statement
To achieve verifiable incremental compute in production, we need:
1. **Proof-Addressable Storage**: Durable storage with deterministic, content-addressed keys (PAC)
2. **Efficient Overlap**: Fast estimation (sketches) + exact computation (manifests)
3. **Receipt Chains**: Traceable links between executions via `prev_execution_rid`
4. **Retention Policies**: Small receipts kept forever, large manifests managed by TTL
5. **Verification Modes**: Light (sketch-based) and Heavy (manifest-based) verification

### Use Cases
Based on research, high-ROI compute jobs include:
- **FinOps Cost Attribution**: Daily billing runs with 85-95% overlap (46% savings, $276K annual)
- **ETL Partition Reuse**: Delta Lake pipelines with 80-90% partition reuse (39% savings, $372K annual)
- **Analytics Dashboard Refresh**: BI queries with 90%+ result overlap (142% savings, $684K annual)
- **Feature Store Updates**: ML feature computation with 70-85% overlap
- **Data Quality Checks**: Incremental validation with 80-90% overlap

## Decision

### 1. Proof-Addressable Cache (PAC) Keys

**Create deterministic, content-addressed keys** for all reusable computations:

```
PAC = H(
  ns|version
  || data_shape_hash
  || method_shape_hash
  || change_epoch_id
  || determinism_class
  || policy_ref
  || output_schema_version
)
```

**Properties:**
- 32-byte binary (BLOB) stored in SQLite
- Uniquely identifies data × method × epoch lineage
- Enables deterministic lookup and de-duplication
- Used across receipts, manifests, and caches

**Implementation:**
```rust
pub fn compute_pac_key(
    namespace: &str,
    version: &str,
    data_shape_hash: &str,
    method_shape_hash: &str,
    change_epoch_id: &str,
    determinism_class: &str,
    policy_ref: &str,
    output_schema_version: &str,
) -> [u8; 32] {
    let combined = format!(
        "{}|{}||{}||{}||{}||{}||{}",
        namespace, version, data_shape_hash, method_shape_hash,
        change_epoch_id, determinism_class, policy_ref, output_schema_version
    );
    let hash = sha2::Sha256::digest(combined.as_bytes());
    hash.into()
}
```

### 2. Receipts (Small, Permanent)

**Receipts record proof commitments, not raw chunk lists.**

Each `ExecutionPayload` includes:

| Field | Purpose |
|-------|---------|
| `pac` | Proof-addressable cache key (32 bytes) |
| `change_epoch` | Snapshot or commit ID (e.g., "snap-123", "commit-abc") |
| `minhash_signature` | Fast overlap estimate (BLOB) |
| `hll_cardinality` | Cardinality estimate (HyperLogLog) |
| `chunk_manifest_hash` | Hash pointer to full manifest |
| `chunk_manifest_size_bytes` | Size for tracking/GC |
| `merkle_root` | Optional integrity root |
| `prev_execution_rid` | Chain link to prior run |
| `alpha`, `overlap_j`, `c_id`, `c_comp`, `decision` | Economic justification (in `ReuseJustification`) |

**Storage:**
- Receipts are canonicalized (CBOR), signed, and persisted immutably
- Typical size: < 5 KB
- Retention: Forever (audit, economics)

### 3. Manifests (Large, GC-able)

**Manifests describe exact chunks/files processed during execution:**

```json
{
  "schema": "northroot/chunk_manifest@1",
  "pac": "<32-byte hash>",
  "change_epoch": "snap-123",
  "entries": [
    {
      "id": "c:aa..",
      "hash": "sha256:..",
      "offset": 0,
      "len": 65536,
      "partition": "dt=2025-11-08"
    }
  ],
  "stats": {
    "rows": 1023456,
    "bytes": 134217728
  },
  "meta": {
    "table": "costs_daily"
  }
}
```

**Storage:**
- Stored compressed (zstd) under `chunk_manifest_hash` in CAS store
- Enables exact overlap computation and audit replay
- Multiple receipts may reference the same manifest
- Typical size: KB–MB
- Retention: 30–90 days (hot) → cold store (policy-driven)

### 4. Storage Architecture

**Create `northroot-storage` crate** (lightweight, local-first):

**Schema (SQLite v1):**
```sql
CREATE TABLE receipts (
    rid TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    version TEXT NOT NULL,
    hash TEXT NOT NULL UNIQUE,
    pac BLOB NOT NULL,
    change_epoch_id TEXT,
    policy_ref TEXT,
    timestamp TEXT NOT NULL,
    canonical_cbor BLOB NOT NULL,
    minhash_signature BLOB,
    hll_cardinality INTEGER,
    chunk_manifest_hash BLOB,
    chunk_manifest_size_bytes INTEGER,
    merkle_root BLOB,
    prev_execution_rid TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_receipts_pac ON receipts(pac);
CREATE INDEX idx_receipts_epoch ON receipts(change_epoch_id);
CREATE INDEX idx_receipts_policy_ref ON receipts(policy_ref);
CREATE INDEX idx_receipts_timestamp ON receipts(timestamp);

CREATE TABLE manifests (
    manifest_hash BLOB PRIMARY KEY,
    pac BLOB NOT NULL,
    change_epoch_id TEXT,
    encoding TEXT NOT NULL,  -- "zstd" or "raw"
    bytes BLOB NOT NULL,
    size_uncompressed INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER  -- NULL = never expire
);

CREATE INDEX idx_manifests_pac ON manifests(pac);
CREATE INDEX idx_manifests_expires ON manifests(expires_at);
```

**Trait Interface:**
```rust
pub trait ReceiptStore: Send + Sync {
    /// Store a receipt (immutable, permanent)
    fn store_receipt(&self, r: &Receipt) -> Result<(), StorageError>;
    
    /// Retrieve receipt by RID
    fn get_receipt(&self, rid: &Uuid) -> Result<Option<Receipt>, StorageError>;
    
    /// Query receipts by criteria (PAC, epoch, policy, timestamp range)
    fn query_receipts(&self, q: ReceiptQuery) -> Result<Vec<Receipt>, StorageError>;
    
    /// Store manifest (compressed, with TTL)
    fn put_manifest(&self, hash: &[u8; 32], data: &[u8], meta: &ManifestMeta) -> Result<(), StorageError>;
    
    /// Retrieve manifest by hash
    fn get_manifest(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>, StorageError>;
    
    /// Get previous execution receipt for reuse decision
    fn get_previous_execution(&self, pac: &[u8; 32], trace_id: &str) -> Result<Option<Receipt>, StorageError>;
    
    /// Garbage collect expired manifests
    fn gc_manifests(&self, before: i64) -> Result<usize, StorageError>;
}
```

**Implementation Strategy:**
- **Phase 1**: SQLite backend with WAL mode (`PRAGMA journal_mode=WAL;`)
- **Phase 2**: File-based backend for local dev (JSON files)
- **Phase 3**: Optional remote backend (HTTP API, S3 for cold manifests)

### 5. Verification Modes

**Light Verification** (O(1)):
- Verify receipt signature
- Recompute MinHash J_est vs policy threshold
- Check economic delta (ΔC) from receipt fields
- No manifest fetch required

**Heavy Verification** (O(n)):
- Fetch manifests for current and previous runs
- Compute exact Jaccard similarity
- Check Merkle proofs (if present)
- Verify ΔC economics with exact overlap
- Replay execution from manifest (optional)

**Implementation:**
```rust
pub enum VerificationMode {
    Light,  // Sketch-based, fast
    Heavy,  // Manifest-based, exact
}

pub struct VerificationResult {
    pub mode: VerificationMode,
    pub decision_correct: bool,
    pub overlap_j: f64,
    pub economic_delta: f64,
    pub errors: Vec<VerificationError>,
}

pub fn verify_reuse_decision(
    current_receipt: &Receipt,
    previous_receipt: &Receipt,
    policy: &DeltaComputePolicy,
    mode: VerificationMode,
    store: &dyn ReceiptStore,
) -> Result<VerificationResult, VerificationError>;
```

### 6. Incremental Compute Loop

**Standard workflow for verifiable incremental compute:**

1. **Select candidate receipts** by PAC or lineage (`prev_execution_rid`)
2. **Estimate overlap** using MinHash/HLL sketches (fast, O(1))
3. **If J ≈ threshold**, load manifests and compute exact overlap (O(n))
4. **Decide reuse vs recompute** via `J > C_id / (α · C_comp)`
5. **Execute delta or full job** based on decision
6. **Emit new receipt + manifest**; sign + store

This loop implements verifiable, reliable incremental compute.

### 7. Retention & Governance

**Retention Policy:**

| Artifact | Typical Size | Retention | Rationale |
|----------|--------------|-----------|-----------|
| Receipt | < 5 KB | Forever | Audit, economics |
| Manifest | KB–MB | 30–90 days (hot) → cold store | Reduce footprint; rarely queried |

**Policy-Driven:**
- Retention days stored in `policy_ref` metadata
- Manifests compressed + deduped (multiple receipts → one manifest)
- Optional S3/Glacier backend for cold manifests
- Receipts reference expired manifests safely through hashes

**Garbage Collection:**
- Periodic GC job removes expired manifests
- Receipts remain intact (only manifest references may become stale)
- Policy can specify `retention_days` and `verify_mode`

### 8. Real Compute Job Examples

**Implement actual compute jobs (not simulations):**

#### 8.1 FinOps Cost Attribution Job
- **Input**: CSV/Parquet billing files (AWS, GCP, Azure)
- **Processing**: Group by account/project, aggregate costs, apply tags
- **Output**: Cost attribution report + execution receipt with PAC + manifest
- **Reuse**: Daily runs with 85-95% overlap

#### 8.2 ETL Partition Reuse Job
- **Input**: Delta Lake table with CDF enabled
- **Processing**: Scan changed partitions, apply transformations
- **Output**: Transformed data + execution receipt with partition manifest
- **Reuse**: Only process changed partitions (80-90% reuse)

#### 8.3 Analytics Query Job
- **Input**: SQL query + data source
- **Processing**: Execute query, cache results
- **Output**: Query results + execution receipt with result manifest
- **Reuse**: Incremental refresh when query results overlap (90%+)

**Job Structure:**
```rust
pub trait ComputeJob: Send + Sync {
    /// Execute job and emit receipt
    fn execute(&self, ctx: &JobContext) -> Result<Receipt, JobError>;
    
    /// Get job metadata
    fn metadata(&self) -> JobMetadata;
}

pub struct JobContext {
    pub policy_ref: String,
    pub trace_id: String,
    pub storage: Arc<dyn ReceiptStore>,
    pub previous_receipt: Option<Receipt>,
    pub change_epoch: String,  // e.g., "snap-123" or "commit-abc"
}
```

## Consequences

### Pros
- **Provable Reuse**: Manifest + receipt = verifiable evidence
- **Durable Incremental Compute**: Audit-ready, works offline (SQLite)
- **Efficient Lookup**: PAC keys enable deterministic, fast retrieval
- **Scalable**: Receipts inline sketches → fast selection; full manifests external → exactness on demand
- **Production Ready**: SQLite first → Postgres/CAS later (forward compatible)
- **Cost Effective**: Small receipts forever, large manifests GC-able

### Cons
- **Manifests Increase Storage**: Mitigated by compression + TTL
- **Extra Latency on Heavy Verification**: Acceptable for audit trail
- **Slight Complexity**: Dual-store design (receipts + manifests)

### Trade-offs
- **Receipts Inline Sketches**: Fast selection, no manifest fetch needed
- **Full Manifests External**: Exactness on demand, GC-able
- **SQLite First**: Easy local dev, strong forward compatibility
- **PAC Keys**: Deterministic lookup, enables cross-team reuse

## Alternatives

### Alternative 1: Store Chunk Sets in Receipts
- **Rejected**: Receipts become too large, cannot GC
- **Better**: Separate manifests (GC-able) from receipts (permanent)

### Alternative 2: No PAC Keys, Use RID Only
- **Rejected**: Cannot deterministically find reusable computations
- **Better**: PAC keys enable content-addressed lookup

### Alternative 3: No Sketches, Always Load Manifests
- **Rejected**: Too slow for fast path (every reuse decision)
- **Better**: Sketches for estimation, manifests for exact verification

### Alternative 4: External Storage Only (S3, Database)
- **Rejected**: Too heavy for local development, adds network dependency
- **Better**: Support both local (SQLite) and remote (future)

## Implementation Plan

### Phase 1: PAC Builder + CBOR Canonicalizer (Week 1)
1. Implement `compute_pac_key()` function
2. Add PAC field to `ExecutionPayload`
3. Update CBOR canonicalization to include PAC
4. Write tests for PAC key determinism

### Phase 2: SQLite ReceiptStore + ManifestStore (Week 1-2)
1. Create `northroot-storage` crate
2. Implement SQLite backend with schema
3. Add manifest compression (zstd)
4. Write tests and benchmarks

### Phase 3: Extend ExecutionPayload + Receipt Schema (Week 2)
1. Add PAC, change_epoch, minhash_signature, hll_cardinality
2. Add chunk_manifest_hash, chunk_manifest_size_bytes
3. Add prev_execution_rid
4. Update schemas and validation

### Phase 4: Light & Heavy Verify Functions (Week 3)
1. Implement `verify_reuse_decision()` with Light mode
2. Implement Heavy mode with manifest loading
3. Add verification tests
4. Document verification workflows

### Phase 5: Integrate with Real Jobs (Week 3-4)
1. Implement FinOps cost attribution job
2. Implement ETL partition reuse job
3. Implement analytics query job
4. Add integration tests

### Phase 6: TTL/GC Policy + Cold-Store Option (Week 4)
1. Add retention policy support
2. Implement manifest GC
3. Add S3 backend stub (future)
4. Add monitoring/metrics

## Migration

### For Existing Examples
1. Update to use `ReceiptStore` instead of reconstructing state
2. Compute PAC keys for executions
3. Store manifests separately from receipts
4. Load previous receipts from storage by PAC
5. Add verification checks

### For New Jobs
1. Implement `ComputeJob` trait
2. Use `ReceiptStore` for state retrieval
3. Emit receipts with PAC + sketches
4. Store manifests with TTL
5. Add verification in tests

## References

- [ADR-002: Canonicalization Strategy](ADRs/ADR-002-canonicalization-strategy.md) - CBOR deterministic encoding
- [ADR-003: Delta Compute Decisions](ADRs/ADR-003-delta-compute-decisions.md)
- [ADR-007: Delta Compute Implementation](ADRs/ADR-007-delta-compute-implementation.md)
- [Proof Analysis](examples/PROOF_ANALYSIS.md)
- [Delta Compute Spec](docs/specs/delta_compute.md)
- [Applied Use Cases](research/reports/delta_compute/cursor/applied_use_case_sheets.md)
- [Proof Synergy Memo](research/reports/delta_compute/openai/proof_synergy_memo.md) - Deterministic serialization

## Open Questions

1. **HLL Implementation**: Use existing library or implement custom? (TBD - evaluate libraries)
2. **Manifest Compression**: zstd vs gzip? (Chose zstd for better compression ratio)
3. **Cold Store Backend**: When to add S3/Glacier? (Future - after SQLite proven)
4. **Concurrent Access**: SQLite WAL mode handles this, but consider Postgres for scale? (Future)

## Decision

**Adopt the proof-addressable storage model (PAC + manifest + retention policy) as the canonical production foundation for all Northroot delta-compute and FinOps workloads.**

This design provides:
- A unified proof-addressable storage layer for receipts + manifests
- Deterministic PAC keys enabling verifiable reuse and caching
- Scalable incremental compute grounded in proofs and cost economics
- Production-ready path to cross-team reuse and future compute-credit settlement
