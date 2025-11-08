<!-- 29e78e83-af44-487f-bcac-666d78df2c93 3788594f-332c-4778-baa8-801eb299d815 -->
# Phase 1: Delta Compute Core Infrastructure

## Overview

Implement the three core infrastructure components for delta compute as specified in ADR-007 and implementation_steps.md. This enables reuse decision tracking, overlap metrics, and cross-organizational receipt verification.

## Tasks

### Task 1: Surface α as First-Class Field in Receipts

**File:** `crates/northroot-receipts/src/lib.rs`

**Current State:**

- `ReuseJustification` already has `alpha: Option<f64>` field (line 454)
- Drift detection test (`test_drift_detection.rs`) only checks hash consistency, not α thresholds

**Changes:**

1. Add helper method to `Receipt` to extract α from `SpendPayload.justification.alpha`
2. Add validation method to check α against policy thresholds
3. Update drift detection test to validate α thresholds per operator

**Implementation:**

- Add `Receipt::alpha()` method that extracts α from spend receipts
- Add `Receipt::validate_alpha_threshold(min_alpha: f64) -> Result<(), ValidationError>`
- Update `test_drift_detection.rs` to test α threshold enforcement
- Add test cases: receipt with α=0.9 passes when threshold=0.8, fails when threshold=0.95

**Acceptance Criteria:**

- Receipt schema includes α field (already exists in ReuseJustification)
- Drift detection tests validate α thresholds
- Policy can specify minimum α per operator

---

### Task 2: Implement ReuseIndexed Trait

**File:** `crates/northroot-engine/src/lib.rs`

**Current State:**

- `Strategy` trait exists in `strategies/trait_.rs`
- `IncrementalSumStrategy` and `PartitionStrategy` implement `Strategy`
- No overlap metric computation exposed

**Changes:**

1. Define `ReuseIndexed` trait with `fn overlap(&self) -> OverlapMetric`
2. Create `OverlapMetric` type to represent Jaccard similarity and related metrics
3. Implement `ReuseIndexed` for `IncrementalSumStrategy` and `PartitionStrategy`
4. Ensure strategies can emit both outputs and reuse metadata without double passes

**Implementation:**

- Add `ReuseIndexed` trait to `lib.rs`:
  ```rust
  pub trait ReuseIndexed {
      fn overlap(&self) -> OverlapMetric;
  }
  ```

- Create `OverlapMetric` struct in `delta/mod.rs`:
  ```rust
  pub struct OverlapMetric {
      pub jaccard: f64,
      pub chunk_count_current: usize,
      pub chunk_count_previous: usize,
      pub chunk_count_intersection: usize,
  }
  ```

- Implement `ReuseIndexed` for `IncrementalSumStrategy` in `strategies/incremental_sum.rs`
- Implement `ReuseIndexed` for `PartitionStrategy` in `strategies/partition.rs`
- Add unit tests for `overlap()` method returning `OverlapMetric` with Jaccard similarity

**Acceptance Criteria:**

- `ReuseIndexed` trait defined with `overlap()` method
- `IncrementalSumStrategy` implements `ReuseIndexed`
- `PartitionStrategy` implements `ReuseIndexed`
- Unit test: `IncrementalSumStrategy.overlap()` returns `OverlapMetric` with Jaccard similarity

---

### Task 3: Add Deterministic CBOR/JCS Support

**File:** `crates/northroot-engine/src/commitments.rs`

**Current State:**

- JCS (JSON Canonicalization Scheme) implemented in `canonical.rs` and `commitments.rs`
- No CBOR support exists
- `jcs()` function in `commitments.rs` implements RFC 8785

**Changes:**

1. Add CBOR deterministic encoding (RFC 8949) support
2. Ensure both CBOR and JCS produce identical hashes for same content
3. Add validation for deterministic encoding rules

**Implementation:**

- Add `cbor` dependency to `Cargo.toml` (use `cbor` or `serde_cbor` crate)
- Implement `cbor_deterministic()` function in `commitments.rs`:
  - Preferred argument sizes (no indefinite-length items)
  - Lexicographically sorted map keys
  - Deterministic encoding rules per RFC 8949
- Add `cbor_hash()` function that computes SHA-256 of deterministic CBOR
- Add validation function `validate_cbor_deterministic()` to check encoding rules
- Add unit test: Two receipts with identical content produce same hash in CBOR and JCS
- Add integration test: Cross-org receipt verification succeeds with CBOR/JCS receipts

**Acceptance Criteria:**

- CBOR encoding follows RFC 8949 deterministic rules
- JSON encoding follows RFC 8785 JCS rules (already implemented)
- Same receipt content produces identical hash in CBOR and JCS
- Unit test: Two receipts with identical content produce same hash (deterministic)
- Integration test: Cross-org receipt verification succeeds with CBOR/JCS receipts

---

## Dependencies

**Crates to add:**

- `cbor` or `serde_cbor` for CBOR encoding (Task 3)

**Files to modify:**

- `crates/northroot-receipts/src/lib.rs` (Task 1)
- `crates/northroot-receipts/tests/test_drift_detection.rs` (Task 1)
- `crates/northroot-engine/src/lib.rs` (Task 2)
- `crates/northroot-engine/src/delta/mod.rs` (Task 2)
- `crates/northroot-engine/src/strategies/incremental_sum.rs` (Task 2)
- `crates/northroot-engine/src/strategies/partition.rs` (Task 2)
- `crates/northroot-engine/src/commitments.rs` (Task 3)
- `crates/northroot-engine/Cargo.toml` (Task 3)

---

## Test Plan

**Task 1 Tests:**

- Unit test: Receipt with α=0.9 passes when threshold=0.8, fails when threshold=0.95
- Integration test: Drift detection alerts when α drops below policy threshold

**Task 2 Tests:**

- Unit test: `IncrementalSumStrategy.overlap()` returns `OverlapMetric` with Jaccard similarity
- Integration test: Strategy execution emits both result and overlap metric

**Task 3 Tests:**

- Unit test: Two receipts with identical content produce same hash (deterministic)
- Unit test: CBOR encoding follows RFC 8949 rules (no indefinite-length items, sorted keys)
- Integration test: Cross-org receipt verification succeeds with CBOR/JCS receipts

---

## Success Criteria

All acceptance criteria met:

- [ ] Receipt schema includes α field (already exists, add helper methods)
- [ ] Drift detection tests validate α thresholds
- [ ] `ReuseIndexed` trait defined and implemented for both strategies
- [ ] CBOR deterministic encoding implemented per RFC 8949
- [ ] All unit and integration tests pass
- [ ] No breaking changes to existing receipt structure

---

## References

- ADR-007: `/ADRs/ADR-007-delta-compute-implementation.md`
- Implementation Steps: `/research/reports/delta_compute/synthesis/implementation_steps.md`
- RFC 8949: CBOR Deterministic Encoding
- RFC 8785: JSON Canonicalization Scheme (already implemented)

### To-dos

- [ ] Surface α as first-class field: Add helper methods to Receipt for extracting/validating α, update drift detection tests
- [ ] Implement ReuseIndexed trait: Define trait, create OverlapMetric type, implement for IncrementalSumStrategy and PartitionStrategy
- [ ] Add deterministic CBOR/JCS support: Implement CBOR deterministic encoding per RFC 8949, ensure hash consistency with JCS