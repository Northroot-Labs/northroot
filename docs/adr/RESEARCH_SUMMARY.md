# ADR Research Summary

**Generated:** 2025-11-12  
**Index Version:** 0.3  
**Last Updated:** 2025-11-12T07:10:11Z

## Executive Summary

This document summarizes the current state of all Architectural Decision Records (ADRs), focusing on active decisions and pending implementation work. ADR-0009 is the current active decision context.

### Status Overview

- **Accepted ADRs:** 7 (ADR-0001 through ADR-0006, ADR-0009)
- **Archived ADRs:** 2 (ADR-0007, ADR-0008) - Completed/archived, decisions incorporated into ADR-0009
- **Active Phases:** 3 pending phases in ADR-0009

---

## Accepted ADRs (Production Decisions)

### ADR-0001: Receipts vs Engine Boundaries
- **Status:** Accepted
- **Domain:** Engine
- **Tags:** canonicalization, delta-compute, receipts, engine, proofs
- **Key Decision:** Clear separation between receipts (evidence formats) and engine (execution semantics)

### ADR-0002: Canonicalization Strategy (CBOR)
- **Status:** Accepted
- **Domain:** Engine
- **Tags:** canonicalization, storage, receipts, engine
- **Key Decision:** CBOR for receipts, JSON adapters for API boundaries

### ADR-0003: Delta Compute Decisions Recorded Under spend.justification
- **Status:** Accepted
- **Domain:** Engine
- **Tags:** delta-compute, receipts, engine
- **Key Decision:** Reuse decisions and economic deltas recorded in `spend.justification`

### ADR-0004: Identity Root Commitment
- **Status:** Accepted
- **Domain:** Engine
- **Tags:** canonicalization, receipts, engine, proofs
- **Key Decision:** Identity root commitment strategy for receipts

### ADR-0005: Engine Architecture
- **Status:** Accepted
- **Domain:** Engine
- **Created:** 2025-01-15
- **Tags:** canonicalization, delta-compute, receipts, engine, proofs
- **Key Decision:** Core engine architecture decisions

### ADR-0006: Signature Verification Strategy
- **Status:** Accepted
- **Domain:** Engine
- **Tags:** delta-compute, receipts, engine, proofs
- **Key Decision:** Signature verification approach for receipts

### ADR-0009: Hybrid ByteStream/RowMap Evidence Substrate with Privacy-Preserving Resolver
- **Status:** Accepted (Phases 1-4, 6 Complete)
- **Domain:** Engine
- **Created:** 2025-11-12
- **Tags:** canonicalization, delta-compute, storage, receipts, engine, proofs
- **Key Decision:** Hybrid architecture for uniform byte stream representation with row-aware overlays

**Implementation Status:**
- ✅ **P1:** Engine-internal `DataShape` enum + hash helper
- ✅ **P2:** Extend `ExecutionPayload` to differentiate byte-level commitments
- ✅ **P3:** Refactor Merkle Row-Map and ByteStream manifest builders
- ✅ **P4:** Privacy-Preserving Resolver API
- ⏭️ **P5:** Summarized manifests for fast overlap (PENDING)
- ✅ **P6:** Storage extensions
- ⏭️ **P7:** Reuse reconciliation flow (PENDING)
- ⏭️ **P8:** Helper functions for shape hash computation (PENDING)

---

## Current Active Work: ADR-0009

**ADR-0009 is the current decision context.** All new decisions and implementations should align with ADR-0009's hybrid ByteStream/RowMap architecture. Decisions from ADR-0007 and ADR-0008 have been incorporated into ADR-0009's implementation.

---

## Pending Implementation Work

### ADR-0009 Phase 5: Summarized Manifests for Fast Overlap
**Status:** Proposed (Pending Implementation)  
**Phase ID:** ADR-0009-P05  
**Proposed:** 2025-11-12T06:41:23Z

**Tasks:**
- ⏭️ Create `crates/northroot-engine/src/delta/manifest_summary.rs` with `ManifestSummary` structure
- ⏭️ Implement MinHash sketch generation and overlap estimation functions
- ⏭️ Add HyperLogLog cardinality estimation
- ⏭️ Add optional Bloom filter support for fast negative checks
- ⏭️ Create `crates/northroot-storage/src/compaction.rs` with compaction tier support (Hot/Warm/Cold)

**Note:** `ManifestSummary` struct already exists in `crates/northroot-storage/src/traits.rs` (from Phase 6), but the engine-side module and overlap computation functions are not yet implemented.

**Dependencies:** Required for ADR-0009-P07 (Reuse Reconciliation Flow)

---

### ADR-0009 Phase 7: Reuse Reconciliation Flow
**Status:** Proposed (Pending Implementation)  
**Phase ID:** ADR-0009-P07  
**Proposed:** 2025-11-12T06:41:23Z

**Tasks:**
- ⏭️ Create `crates/northroot-engine/src/delta/reuse.rs` with `ReuseReconciliation` struct and `check_reuse()` function
- ⏭️ Implement fast path using manifest summaries (MinHash estimation)
- ⏭️ Implement exact path when overlap looks promising (full manifest loading)
- ⏭️ Integrate with cost model and economic delta computation
- ⏭️ Return output info (digest, locator ref, manifest root) for resolver to load
- ⏭️ Tie decisions back into `ExecutionPayload.justification` (overlap J, α, economic deltas)
- ⏭️ Implement `compute_manifest_root()` in `crates/northroot-engine/src/delta/manifest_root.rs` for Merkle root over output subparts (enables partial reuse proofs)

**Dependencies:** Requires ADR-0009-P05 (Summarized Manifests) to be complete

---

### ADR-0009 Phase 8: Helper Functions for Shape Hash Computation
**Status:** Proposed (Pending Implementation)  
**Phase ID:** ADR-0009-P08  
**Proposed:** 2025-11-12T06:41:23Z

**Tasks:**
- ⏭️ Create `crates/northroot-engine/src/delta/data_shape.rs` with helpers:
  - `compute_data_shape_hash_from_file()`
  - `compute_data_shape_hash_from_bytes()`
  - `compute_data_shape_hash_from_inputs()` (composite from multiple encrypted locators)
- ⏭️ Create `crates/northroot-engine/src/delta/method_shape.rs` with helpers:
  - `compute_method_shape_hash_from_code()`
  - `compute_method_shape_hash_from_signature()`

**Note:** These are convenience functions for PAC key computation, not strictly required for core functionality.

---

## Critical Path Analysis

### Immediate Priorities (ADR-0009 Focus)

1. **ADR-0009-P05 (Summarized Manifests)** - Blocking ADR-0009-P07
   - Enables fast overlap estimation without loading full manifests
   - Required for production-ready reuse reconciliation
   - **Status:** Next to implement

2. **ADR-0009-P07 (Reuse Reconciliation Flow)** - Core functionality
   - Implements the actual reuse decision logic
   - Integrates economic delta computation
   - Ties into receipt justification
   - **Status:** Blocked by P05

3. **ADR-0009-P08 (Helper Functions)** - Convenience layer
   - Not blocking, but improves developer experience
   - Simplifies PAC key computation
   - **Status:** Can be done in parallel or after P07

---

## Architectural Dependencies

### ADR-0009 Dependencies
- **P7 depends on P5:** Reuse reconciliation requires manifest summaries
- **P4 complete:** Resolver API is ready for integration
- **P6 complete:** Storage extensions provide persistence layer
- **P1-P4, P6 complete:** Foundation is in place for remaining phases

### Cross-ADR Relationships
- **ADR-0009** supersedes implicit assumptions in **ADR-0005**
- **ADR-0009** incorporates decisions from **ADR-0007** and **ADR-0008** (archived)
- **ADR-0009** builds on **ADR-0003** (delta compute decisions)

---

## Known Gaps and Open Questions

### From ADR-0009 (Current Context)
- **HLL Implementation:** Use existing library or implement custom? (TBD - evaluate libraries for P05)
- **Manifest Compression:** zstd vs gzip? (Decision: zstd for better compression ratio)
- **Manifest Root Computation:** Deferred from P4, can be implemented alongside P7
- **Breaking Changes:** RowMap domain separation migration invalidates existing roots - test vectors need regeneration
- **Cold Store Backend:** When to add S3/Glacier? (Future - after SQLite proven)
- **Concurrent Access:** SQLite WAL mode handles this, but consider Postgres for scale? (Future)

---

## Recommendations

### Short-Term (Next 2-4 Weeks)
1. **Complete ADR-0009-P05** - Unblocks reuse reconciliation flow
   - Implement manifest summary generation and overlap estimation
   - Add HLL cardinality and Bloom filter support
2. **Complete ADR-0009-P07** - Core functionality for production delta compute
   - Implement reuse reconciliation with fast/exact paths
   - Integrate economic delta computation
   - Add manifest root computation

### Medium-Term (1-2 Months)
1. **Complete ADR-0009-P08** - Developer experience improvements
   - Add convenience helpers for shape hash computation
2. **Test vector regeneration** - Update vectors after RowMap breaking changes
3. **Integration testing** - End-to-end tests with real compute jobs

### Long-Term (3+ Months)
1. **Evaluate and implement cold store backend** (S3/Glacier)
2. **Scale considerations** (Postgres for concurrent access)
3. **Cross-team reuse and compute-credit settlement**

---

## References

- [ADR Index](../../docs/adr/adr.index.json) - Machine-readable index
- [ADR README](../../docs/adr/README.md) - ADR structure and lifecycle
- [Phase Documentation](../../docs/phases/README.md) - Phase identification and standards
- [ADR Maintenance Rules](../../.cursor/rules/adr-maintenance.mdc) - Auto-enforcement rules

---

## Archived ADRs (Completed/Incorporated)

### ADR-0007: Delta Compute Implementation Strategy
- **Status:** Archived (Decisions incorporated into ADR-0009)
- **Domain:** Core
- **Created:** 2025-11-08
- **Archived:** 2025-11-12
- **Context:** Research synthesis findings have been incorporated into ADR-0009's implementation phases

### ADR-0008: Proof-Addressable Storage and Incremental Compute Hardening
- **Status:** Archived (Decisions incorporated into ADR-0009)
- **Domain:** Engine
- **Created:** 2025-11-09
- **Archived:** 2025-11-12
- **Context:** Storage architecture decisions (PAC keys, receipt/manifest separation) have been implemented in ADR-0009-P06

**Note:** ADR-0007 and ADR-0008 are archived. Their decisions have been incorporated into ADR-0009, which is the current active decision context. All new work should reference ADR-0009.

---

**Last Updated:** 2025-11-12  
**Next Review:** After ADR-0009-P05 completion

