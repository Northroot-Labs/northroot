# Integration Notes: SDK/API Integration Strategies

**Research Agent:** openai  
**Date:** 2025-11-08  
**Version:** 0.1  
**Namespace:** northroot.research.delta_compute

## Overview

This document outlines integration strategies for embedding Northroot's delta compute and verifiable receipt system into existing data processing frameworks. Focus areas: SDK hooks, receipt emission, and deterministic serialization.

---

## 1. Core Integration Requirements

### 1.1 Surface α as First-Class Field

**Location:** `receipts/src/canonical.rs`

**Requirement:** Every operator receipt must carry overlap evidence (α) as a first-class field.

**Implementation:**
- Add `alpha: Option<f64>` to receipt schema
- Wire through `receipts/tests/test_drift_detection.rs` to fail when α drops below negotiated thresholds
- Enable policy-driven threshold enforcement

**Benefits:**
- Transparent reuse decisions
- Policy-driven threshold enforcement
- Audit trail of incrementality

### 1.2 ReuseIndexed Trait

**Location:** `crates/northroot-engine/src/lib.rs`

**Requirement:** Extend engine with a `ReuseIndexed` trait that exposes `fn overlap(&self) -> OverlapMetric`.

**Implementation:**
```rust
pub trait ReuseIndexed {
    fn overlap(&self) -> OverlapMetric;
}
```

**Benefits:**
- Strategies (e.g., `strategies/incremental_sum.rs`) can emit both outputs and reuse metadata without double passes
- Clean separation of concerns
- Composable reuse detection

### 1.3 Deterministic CBOR/JCS Support

**Location:** `engine/src/commitments.rs`

**Requirement:** Accept deterministic CBOR blobs (per RFC 8949) and optionally mirrored JCS JSON bodies so cross-org verifiers can recompute roots byte-for-byte.

**Implementation:**
- CBOR deterministic encoding (RFC 8949): preferred argument sizes, no indefinite-length items, sorted keys
- JCS (RFC 8785): deterministic property ordering, ECMAScript-consistent number formatting
- Enable cross-org verification without proprietary data access

**Benefits:**
- Cross-organizational receipt verification
- Deterministic proof roots
- Portable receipts

---

## 2. Framework Integration Patterns

### 2.1 Bazel-Style Cache Keys

**Pattern:** Package operator manifests (method/data/reasoning shapes) with Bazel-style cache keys: hash of canonical inputs, operator id, α, ΔC.

**Implementation:**
- When a cache hit occurs (from Bazel, DVC, Dagster, Ray), create a Northroot receipt instead of silently skipping work
- Link cache keys to receipt RIDs for auditability
- Enable verifiable cache hits

**Benefits:**
- Leverage existing cache infrastructure
- Add verifiability to cache hits
- Transparent reuse decisions

### 2.2 FinOps Pilot Integration

**Location:** `receipts/schemas/spend_schema.json`

**Requirement:** Bind receipts to spend schema and store MinHash sketches that prove two billing runs reused ≥80% of the graph.

**Implementation:**
- Emit drift alerts in `test_drift_detection.rs` when sketches diverge
- Track α per billing window by hashing resource tuples
- Compare reuse receipts against previous `execution_schema` commits

**Benefits:**
- Verifiable cost attribution
- Drift detection for billing graphs
- Chargeback ticket evidence

---

## 3. Operator-Specific Integration

### 3.1 Delta Lake CDF Scan Operator

**Location:** `receipts/src/canonical.rs`

**Requirement:** Add an `operator::cdf_scan` entry that stores `_commit_version`, `_change_type`, and `_commit_timestamp` so Northroot receipts prove which Delta segments were reused.

**Implementation:**
- Integrate with Delta Lake's change data feed
- Track row-level changes per commit version
- Emit receipts with CDF metadata

**Benefits:**
- Precise partition-level reuse tracking
- Verifiable ETL refresh decisions
- 30–45% compute savings

### 3.2 Incremental Sum Strategy Integration

**Location:** `crates/northroot-engine/src/strategies/incremental_sum.rs`

**Requirement:** Pair with a cost-allocation adapter that tags each partition with ΔC and expected reuse window to unlock deterministic FinOps receipts.

**Implementation:**
- Extend incremental_sum strategy with cost allocation
- Tag partitions with economic delta (ΔC)
- Track expected reuse windows

**Benefits:**
- FinOps cost attribution with verifiable reuse
- Economic transparency per partition
- Chargeback evidence

---

## 4. Drift Detection Integration

### 4.1 CDF Range Drift Detection

**Location:** `receipts/tests/test_drift_detection.rs`

**Requirement:** Teach drift detection to treat missing CDF ranges as drift, forcing only the affected partitions to recompute.

**Implementation:**
- Compare CDF ranges between runs
- Detect missing commit versions
- Trigger recomputation for affected partitions only

**Benefits:**
- Precise drift detection
- Minimal recomputation
- Verifiable partition-level reuse

### 4.2 MinHash Sketch Drift Detection

**Location:** `receipts/tests/test_drift_detection.rs`

**Requirement:** Emit drift alerts when MinHash sketches diverge, indicating billing graph changes >5%.

**Implementation:**
- Compare MinHash sketches between billing runs
- Detect significant divergence (>5%)
- Emit alerts with verifiable evidence

**Benefits:**
- Early drift detection
- Verifiable billing graph changes
- Chargeback ticket evidence

---

## 5. Cross-Organizational Integration

### 5.1 Deterministic Serialization

**Requirement:** Standardize on CBOR deterministic encoding for binary receipts and JCS for JSON mirrors so the same proof roots (`engine/src/commitments.rs`) can be verified cross-org.

**Implementation:**
- CBOR (RFC 8949): deterministic encoding rules
- JCS (RFC 8785): deterministic property sorting
- Enable cross-org verification

**Benefits:**
- Portable receipts across organizations
- Eliminate duplicate compliance reruns
- Trustless verification

### 5.2 Chunk-Level Proofs

**Requirement:** Pair chunk-level proofs (FastCDC) with deterministic serialization so third parties can trust reuse attestations without replaying workloads.

**Implementation:**
- FastCDC chunk IDs for content-defined chunking
- Deterministic serialization for chunk proofs
- Enable third-party verification

**Benefits:**
- Verifiable chunk-level reuse
- Trustless attestations
- Cross-org compute markets

---

## 6. Receipt Emission Patterns

### 6.1 Cache Hit Receipts

**Pattern:** When a cache hit occurs (from Bazel, DVC, Dagster, Ray), create a Northroot receipt instead of silently skipping work.

**Implementation:**
- Intercept cache hit events
- Generate receipt with reuse justification
- Link to cache key for auditability

**Benefits:**
- Verifiable cache hits
- Transparent reuse decisions
- Audit trail

### 6.2 Operator Manifest Receipts

**Pattern:** Package operator manifests (method/data/reasoning shapes) with cache keys and emit receipts for each operator execution.

**Implementation:**
- Link operator manifests to cache keys
- Emit receipts per operator execution
- Track α and ΔC per operator

**Benefits:**
- Operator-level reuse tracking
- Economic transparency
- Policy enforcement

---

## 7. Testing & Validation

### 7.1 Drift Detection Tests

**Location:** `receipts/tests/test_drift_detection.rs`

**Requirements:**
- Test CDF range drift detection
- Test MinHash sketch divergence
- Test α threshold enforcement

**Implementation:**
- Unit tests for drift detection logic
- Integration tests with Delta Lake CDF
- Policy threshold validation

### 7.2 Cross-Org Verification Tests

**Requirements:**
- Test CBOR/JCS deterministic encoding
- Test cross-org receipt verification
- Test chunk-level proof verification

**Implementation:**
- Unit tests for deterministic serialization
- Integration tests for cross-org verification
- Chunk proof validation

---

## 8. Migration Path

### Phase 1: Core Infrastructure (Weeks 1-2)
- Surface α as first-class field in receipts
- Implement ReuseIndexed trait
- Add deterministic CBOR/JCS support

### Phase 2: Framework Integration (Weeks 3-4)
- Integrate with Bazel/DVC/Dagster cache systems
- Implement Delta Lake CDF scan operator
- Add FinOps pilot integration

### Phase 3: Drift Detection (Weeks 5-6)
- Implement CDF range drift detection
- Add MinHash sketch divergence alerts
- Policy threshold enforcement

### Phase 4: Cross-Org Support (Weeks 7-8)
- Standardize on CBOR/JCS
- Enable cross-org verification
- Chunk-level proof support

---

## 9. Next Steps

1. **Implement ReuseIndexed trait** in engine
2. **Add α field** to receipt schema
3. **Integrate Delta Lake CDF** scan operator
4. **Implement drift detection** for CDF ranges and MinHash sketches
5. **Standardize on CBOR/JCS** for cross-org receipts

---

**References:**
- Northroot Engine: `crates/northroot-engine/src/`
- Receipt Schema: `receipts/schemas/`
- Delta Compute Spec: `docs/specs/delta_compute.md`

