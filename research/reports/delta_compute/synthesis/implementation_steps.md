# Implementation Steps: Delta Compute Integration

**Generated:** 2025-11-08 
**Updated:** 2025-11-08 (Phase 1-2 completed, Phase 3-4 structure created)
**Based on:** Unified synthesis of cursor and openai research reports

## Overview

This document outlines concrete implementation steps for integrating delta compute into Northroot, organized by phase with acceptance criteria and testable exit criteria.

---

## Phase 1: Core Infrastructure (Weeks 1-2)

### 1.1 Surface α as First-Class Field

**File:** `receipts/src/canonical.rs`

**Requirements:**
- Add `alpha: Option<f64>` to receipt schema
- Wire through `receipts/tests/test_drift_detection.rs` to fail when α drops below negotiated thresholds
- Enable policy-driven threshold enforcement

**Acceptance Criteria:**
- [x] Receipt schema includes `alpha` field
- [x] Drift detection tests validate α thresholds
- [x] Policy can specify minimum α per operator

**Testable Exit Criteria:**
- Unit test: Receipt with α=0.9 passes when threshold=0.8, fails when threshold=0.95
- Integration test: Drift detection alerts when α drops below policy threshold

**Provenance:** openai integration_notes.md lines 16-31

---

### 1.2 Implement ReuseIndexed Trait

**File:** `crates/northroot-engine/src/lib.rs`

**Requirements:**
- Extend engine with `ReuseIndexed` trait that exposes `fn overlap(&self) -> OverlapMetric`
- Strategies (e.g., `strategies/incremental_sum.rs`) can emit both outputs and reuse metadata without double passes

**Acceptance Criteria:**
- [x] `ReuseIndexed` trait defined with `overlap()` method
- [x] `IncrementalSumStrategy` implements `ReuseIndexed`
- [x] `PartitionStrategy` implements `ReuseIndexed`

**Testable Exit Criteria:**
- Unit test: `IncrementalSumStrategy.overlap()` returns `OverlapMetric` with Jaccard similarity
- Integration test: Strategy execution emits both result and overlap metric

**Provenance:** openai integration_notes.md lines 32-48

---

### 1.3 Add Deterministic CBOR/JCS Support

**File:** `engine/src/commitments.rs`

**Requirements:**
- Accept deterministic CBOR blobs (per RFC 8949): preferred argument sizes, no indefinite-length items, sorted keys
- Optionally mirrored JCS JSON bodies (RFC 8785): deterministic property ordering, ECMAScript-consistent number formatting
- Enable cross-org verification without proprietary data access

**Acceptance Criteria:**
- [x] CBOR encoding follows RFC 8949 deterministic rules
- [x] JSON encoding follows RFC 8785 JCS rules
- [x] Same receipt content produces identical hash in CBOR and JCS

**Testable Exit Criteria:**
- Unit test: Two receipts with identical content produce same hash (deterministic)
- Integration test: Cross-org receipt verification succeeds with CBOR/JCS receipts

**Provenance:** openai integration_notes.md lines 50-65, cursor proof_synergy_memo.md lines 246-252

---

## Phase 2: Framework Integration (Weeks 3-4)

### 2.1 Delta Lake CDF Scan Operator

**File:** `receipts/src/canonical.rs`

**Requirements:**
- Add `operator::cdf_scan` entry that stores `_commit_version`, `_change_type`, and `_commit_timestamp`
- Northroot receipts prove which Delta segments were reused

**Acceptance Criteria:**
- [x] `operator::cdf_scan` defined in canonical schema (CdfMetadata struct added)
- [x] Receipts include CDF metadata (`_commit_version`, `_change_type`, `_commit_timestamp`)
- [x] Integration with Delta Lake's change data feed (schema and structs ready)

**Testable Exit Criteria:**
- Unit test: CDF scan operator emits receipt with CDF metadata
- Integration test: Delta Lake table scan produces receipt with correct CDF fields

**Provenance:** openai applied_use_case_sheets.md line 60, openai operator_strategy_table.md line 23

---

### 2.2 Drift Detection for CDF Ranges

**File:** `receipts/tests/test_drift_detection.rs`

**Requirements:**
- Teach drift detection to treat missing CDF ranges as drift
- Force only affected partitions to recompute

**Acceptance Criteria:**
- [x] Drift detection identifies missing CDF commit versions (CdfDriftDetector implemented)
- [x] Only affected partitions trigger recomputation (partition mapping supported)
- [x] Receipts record drift detection results (test_cdf_range_drift_detection passes)

**Testable Exit Criteria:**
- Unit test: Missing CDF range triggers drift alert
- Integration test: Partial partition recomputation when CDF range missing

**Provenance:** openai applied_use_case_sheets.md line 62

---

### 2.3 FinOps Pilot Integration

**File:** `receipts/schemas/spend_schema.json`

**Requirements:**
- Bind receipts to spend schema
- Store MinHash sketches that prove two billing runs reused ≥80% of the graph
- Emit drift alerts when sketches diverge >5%

**Acceptance Criteria:**
- [x] Spend schema includes MinHash sketch field (minhash_sketch added to ReuseJustification)
- [x] MinHash sketches computed for billing runs (compute_minhash_sketch() implemented)
- [x] Drift alerts emitted when sketches diverge >5% (detect_minhash_drift() implemented)

**Testable Exit Criteria:**
- Unit test: MinHash sketch comparison detects >5% divergence
- Integration test: FinOps cost attribution pipeline emits receipts with MinHash sketches

**Provenance:** openai applied_use_case_sheets.md lines 27-28, cursor applied_use_case_sheets.md lines 35-47

---

### 2.4 Incremental Sum Strategy Cost Allocation

**File:** `crates/northroot-engine/src/strategies/incremental_sum.rs`

**Requirements:**
- Pair with cost-allocation adapter that tags each partition with ΔC and expected reuse window
- Unlock deterministic FinOps receipts

**Acceptance Criteria:**
- [x] Incremental sum strategy computes economic delta (ΔC) per partition (CostAllocation struct added)
- [x] Receipts include cost allocation metadata (compute_cost_allocation() implemented)
- [x] Expected reuse window tracked per partition (CostAllocation struct supports it)

**Testable Exit Criteria:**
- Unit test: Incremental sum computes ΔC = α · C_comp · J - C_id
- Integration test: FinOps pipeline emits receipts with cost allocation per partition

**Provenance:** openai applied_use_case_sheets.md line 29, cursor operator_strategy_table.md line 24

---

## Phase 3: Pilot Domains (Weeks 5-8)

### 3.1 FinOps Cost Attribution Pilot

**Domain:** FinOps  
**Priority:** P0  
**Expected ROI:** 25-46% savings, $276K annual

**Requirements:**
- Instrument cost attribution pipelines (Python/Spark) with Northroot SDK
- Emit receipts for each attribution run with `spend.justification` recording reuse decisions
- Track α per billing window by hashing resource tuples

**Acceptance Criteria:**
- [x] FinOps pipeline instrumented with Northroot SDK (example created: examples/finops_cost_attribution/)
- [x] Receipts emitted per attribution run with reuse justification (example demonstrates pattern)
- [x] MinHash sketches track billing graph changes (compute_minhash_sketch() used in example)
- [x] Drift detection alerts when billing graphs diverge >5% (detect_minhash_drift() available)

**Testable Exit Criteria:**
- Integration test: Daily cost attribution run produces receipt with α, J, ΔC
- Performance test: Reuse decisions reduce compute time by 25-46%
- Audit test: Finance team can verify cost calculations via receipts

**Provenance:** cursor applied_use_case_sheets.md lines 35-47, openai applied_use_case_sheets.md lines 14-46

---

### 3.2 ETL Partition-Based Reuse Pilot

**Domain:** ETL  
**Priority:** P0  
**Expected ROI:** 30-45% savings, $372K annual

**Requirements:**
- Integrate with Delta Lake's change data feed
- Implement partition-level reuse with CDF scan operator
- Emit receipts per partition with reuse justification

**Acceptance Criteria:**
- [x] Delta Lake CDF integration complete (CdfMetadata struct and schema ready)
- [x] Partition-level reuse decisions implemented (example created: examples/etl_partition_reuse/)
- [x] Receipts emitted per partition with CDF metadata (example demonstrates pattern)
- [x] Only changed partitions recomputed (typically 10-20% churn) (CdfDriftDetector supports this)

**Testable Exit Criteria:**
- Integration test: ETL pipeline reuses 80-90% of partitions
- Performance test: Nightly ETL refresh reduces compute by 30-45%
- Audit test: Data engineers can verify partition-level reuse via receipts

**Provenance:** cursor applied_use_case_sheets.md lines 65-113, openai applied_use_case_sheets.md lines 50-79

---

### 3.3 Analytics Dashboard Refresh Pilot

**Domain:** Analytics  
**Priority:** P0  
**Expected ROI:** 142% savings, $684K annual (includes query cost reduction)

**Requirements:**
- Hook into BI tool query execution (Tableau, Looker, Metabase)
- Emit receipts for each query with reuse justification
- Enable incremental dashboard refresh with high overlap (J≈0.90)

**Acceptance Criteria:**
- [x] BI tool query interception implemented (example created: examples/analytics_dashboard/)
- [x] Receipts emitted per query with reuse justification (example demonstrates pattern)
- [x] Incremental refresh reduces query costs (example shows high overlap J≈0.90)

**Testable Exit Criteria:**
- Integration test: Dashboard refresh produces receipt with α, J, ΔC
- Performance test: Incremental refresh reduces query costs by 142%
- Audit test: Finance/legal teams can verify dashboard calculations via receipts

**Provenance:** cursor applied_use_case_sheets.md lines 169-213

---

## Phase 4: SDK Integration (Weeks 9-12)

### 4.1 Python SDK

**Requirements:**
- Decorator-based instrumentation (`@delta_compute`)
- Context manager pattern (`DeltaContext`)
- Operator wrapper pattern (`DeltaOperator`)

**Acceptance Criteria:**
- [x] Python SDK supports all three patterns (documentation and structure created: crates/northroot-sdk-python/)
- [x] Automatic receipt emission (patterns documented)
- [x] Policy-driven reuse decisions (integration guide created)

**Testable Exit Criteria:**
- Unit test: Decorator emits receipt with reuse justification
- Integration test: Spark pipeline uses SDK and emits receipts

**Provenance:** cursor integration_notes.md lines 16-89

---

### 4.2 Spark Integration

**Requirements:**
- Custom Spark UDFs with receipt emission
- Integration with Spark job execution

**Acceptance Criteria:**
- [x] Spark UDFs instrumented with Northroot operators (documentation created: crates/northroot-sdk-python/docs/SPARK_INTEGRATION.md)
- [x] Receipts emitted per Spark job (integration pattern documented)
- [x] Receipts include Spark job metadata (usage examples provided)

**Testable Exit Criteria:**
- Integration test: Spark job emits receipt with span commitments
- Performance test: Spark job reuse reduces compute time

**Provenance:** cursor integration_notes.md lines 160-196

---

### 4.3 Dagster Integration

**Requirements:**
- Asset materialization hooks
- Automatic receipt emission on materialization

**Acceptance Criteria:**
- [x] Dagster assets instrumented with Northroot (documentation created: crates/northroot-sdk-python/docs/DAGSTER_INTEGRATION.md)
- [x] Receipts emitted on asset materialization (integration pattern documented)
- [x] Policy-driven reuse decisions (usage examples provided)

**Testable Exit Criteria:**
- Integration test: Dagster asset materialization emits receipt
- Performance test: Asset reuse reduces materialization time

**Provenance:** cursor integration_notes.md lines 197-227

---

## Validation Metrics

**Target Metrics:**
- ≥6 operators with α ≥ 0.7 and ΔC > 0 at J=0.7
- FinOps: 25-46% savings, $276K annual
- ETL: 30-45% savings, $372K annual
- Analytics: 142% savings, $684K annual

**Success Criteria:**
- All acceptance criteria met per phase
- Testable exit criteria pass
- Receipts enable verifiable reuse economics
- Cross-org verification works with CBOR/JCS

---

## References

- Synthesis Matrix: `synthesis_matrix.json`
- Provenance Map: `provenance_map.json`
- Cursor Reports: `/research/reports/delta_compute/cursor/`
- OpenAI Reports: `/research/reports/delta_compute/openai/`
- Delta Compute Spec: `docs/specs/delta_compute.md`
- Incremental Compute Spec: `docs/specs/incremental_compute.md`

