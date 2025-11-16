# Goal Grid v0.1 Progress Tracking

**Last Updated:** 2025-11-15  
**Goal Grid Version:** 0.1  
**ADR Reference:** ADR-0012

**About the Goal Grid:** This progress tracking follows the [goal grid](../../planning/goal-grid.md), a planning framework that organizes work into a 64-cell grid (8 pillars × 8 tasks) to maintain focus and prevent scope drift.

**Recent Fixes:**
- Fixed JSON deserialization errors for `[u8; 32]` byte arrays (pac, chunk_manifest_hash, merkle_root, manifest_root)
- Added custom serializer/deserializer with backward compatibility (handles both base64url strings and legacy number arrays)
- Quickstart example now runs without warnings

## Implementation Provenance

**Latest Commit:** `68037d7fa3ca317f48ac514ca833714451e19edd`  
**Commit Date:** 2025-11-15  
**Commit Message:** `feat(storage): implement filesystem receipt store (P2-T1)`

**Previous Commits:**
- `e1c46853875535f4a4c4cfdce1d2233fdfa84bbc`: `feat(sdk): implement minimal API (P2-T3) and quickstart (P2-T7)`
  - P2-T3: Minimal Python API surface (Rust API + Python bindings)
  - P2-T7: Quickstart example

## Status Summary

This document tracks progress against the goal grid (64-cell structure) defined in `goals/harada/northroot-active.md`. All work must align with goal grid tasks to prevent scope drift.

## Central Goal (CG-1)

**CG-1 — Ship a reliable, minimal verifiable-compute layer with a Python SDK and a working end-to-end demo that real developers can adopt.**

**Status:** In Progress  
**Blockers:** Python SDK bindings for minimal API not yet implemented

---

## Pillar 1 — Engine Rigor

**Objective:** A deterministic, mathematically correct, minimal engine with stable semantics.

| Task | Status | Notes |
|------|--------|-------|
| **P1-T1** — Lock CBOR canonicalization | ✅ **COMPLETE** | CBOR canonicalization implemented in `northroot-receipts/src/canonical/`. All tests passing (8/8). RFC 8949 compliant. |
| **P1-T2** — Finalize hashing and domain separation rules | ✅ **COMPLETE** | Comprehensive documentation in `docs/specs/hashing-and-domain-separation.md`. Rules frozen for v0.1. |
| **P1-T3** — Freeze v0.1 chunk model | ✅ **COMPLETE** | Chunk model frozen in `docs/specs/v01-chunk-model-freeze.md`. All Receipt, ReceiptKind, and Payload types declared frozen for v0.1. |
| **P1-T4** — Implement golden tests for serialization | ✅ **COMPLETE** | Golden tests exist in `test_drift_detection.rs` with baseline hashes. |
| **P1-T5** — Stabilize delta-reuse criteria in code | ✅ **COMPLETE** | Core decision logic documented and locked in `crates/northroot-engine/src/delta/decision.rs`. Reuse rule: `Reuse iff J > C_id / (α · C_comp)`. Economic delta calculation stabilized. Known limitations documented (exact path manifest parsing deferred to post-v0.1). All tests passing (7/7). |
| **P1-T6** — Clean up engine crate structure | ✅ **COMPLETE** | Structure cleaned: removed deprecated `execution/merkle_row_map.rs` (moved to `rowmap.rs`). All modules are clear and well-organized. No dead code found. |
| **P1-T7** — Document canonical forms | ✅ **COMPLETE** | Developer-facing reference created in `docs/guides/canonical-forms-reference.md`. Covers CBOR, JCS, JSON adapters, and best practices. |
| **P1-T8** — Establish reproducible test suite | ✅ **COMPLETE** | Test suite exists and runs reproducibly. |

**Phase 1 Progress:** 8/8 complete, 0/8 partial, 0/8 pending

---

## Pillar 2 — Python SDK

**Objective:** Provide a frictionless thin-client Python SDK that hides engine complexity.

| Task | Status | Notes |
|------|--------|-------|
| **P2-T1** — Implement simple local receipt store | ✅ **COMPLETE** | Filesystem store implemented in `crates/northroot-storage/src/filesystem.rs`. Stores receipts as JSON files. |
| **P2-T2** — Add JSON boundary adapter | ✅ **COMPLETE** | JSON adapters exist in `northroot-receipts/src/adapters/json.rs`. |
| **P2-T3** — Define a stable, minimal Python API surface | ✅ **COMPLETE** | **Rust API created** (`northroot-engine/src/api.rs`). **Python bindings created** (`sdk/northroot-sdk-python/src/receipts.rs`). Both `record_work` and `verify_receipt` exposed to Python. |
| **P2-T4** — Provide both async and sync call paths | ✅ **COMPLETE** | Async wrappers in `northroot/__init__.py` using `asyncio.to_thread`. Both sync and async APIs available. |
| **P2-T5** — Add a clear exception hierarchy | ✅ **COMPLETE** | Error types mapped from Rust ApiError, clear hierarchy documented. |
| **P2-T6** — Create typed result objects | ✅ **COMPLETE** | `PyReceipt` integrated with new API. Thin client wrapper created. |
| **P2-T7** — Produce a 10–15 line quickstart example | ✅ **COMPLETE** | Quickstart example created at `sdk/northroot-sdk-python/examples/quickstart.py`. |
| **P2-T8** — Package and publish a clean PyPI release | ✅ **COMPLETE** | Published to PyPI as `northroot` v0.1.0. GitHub Actions workflow configured for automated publishing. |

**Phase 2 Progress:** 8/8 complete, 0/8 partial, 0/8 pending

**Recent Completion:** 
- Python SDK bindings for `record_work` and `verify_receipt` created (P2-T3 ✅)
- Quickstart example created (P2-T7 ✅)
- Filesystem receipt store implemented (P2-T1 ✅)
- Exception hierarchy for SDK errors (P2-T5 ✅)
- Thin client wrapper for ergonomic API (P2-T6 ✅)
- Async/sync call paths implemented (P2-T4 ✅)

---

## Pillar 3 — Receipt Geometry

**Objective:** Standardized, composable receipt types that are stable across versions.

| Task | Status | Notes |
|------|--------|-------|
| **P3-T1** through **P3-T8** | ❌ **PENDING** | All tasks pending. Receipt shapes exist but not frozen. |

**Phase 3 Progress:** 0/8 complete

---

## Pillar 4 — Delta Compute Economics

**Objective:** A principled economic model that justifies reuse and clarifies "when it's worth it."

| Task | Status | Notes |
|------|--------|-------|
| **P4-T1** through **P4-T8** | ❌ **PENDING** | All tasks pending. Delta compute exists but economics not finalized. |

**Phase 4 Progress:** 0/8 complete

---

## Pillar 5 — Observability & Integration

**Objective:** Make adoption easy by mapping existing logs/traces to proofs.

| Task | Status | Notes |
|------|--------|-------|
| **P5-T1** — Build an OTEL span → proof transformer | ✅ **COMPLETE** | OTEL integration module created (`northroot/otel.py`). `span_to_receipt()` converts OTEL spans to receipts. `@trace_work` decorator for automatic receipt generation. Example created (`examples/otel_integration.py`). Optional dependency: `opentelemetry-api`. |
| **P5-T2** through **P5-T8** | ❌ **PENDING** | Remaining tasks pending. |

**Phase 5 Progress:** 1/8 complete, 0/8 in progress, 7/8 pending

---

## Pillar 6 — Product Narrative

**Objective:** A simple, credible explanation that engineers and investors understand instantly.

| Task | Status | Notes |
|------|--------|-------|
| **P6-T1** — Write problem framing around redundant/opaque compute | ✅ **COMPLETE** | Problem framing added to README: redundant compute, opaque decisions, no audit trail, trust gaps. |
| **P6-T2** — Define a succinct positioning: "verifiable compute plumbing" | ✅ **COMPLETE** | Positioning defined in README: "Verifiable compute plumbing for transparent, reusable computation." |
| **P6-T3** — Explain why proofs beat logs | ✅ **COMPLETE** | "Why Proofs Beat Logs" section added to README with comparison table (determinism, reuse, auditability, trust). |
| **P6-T6** — Document 3 concrete use cases | ✅ **COMPLETE** | Three use cases documented in README: FinOps cost attribution, ETL pipeline reuse, Analytics dashboard refresh. |
| **P6-T8** — Write a clear, minimal README | ✅ **COMPLETE** | README rewritten to align with goal grid narrative: problem framing, positioning, proofs vs logs, use cases, quick start. |
| **P6-T4**, **P6-T5**, **P6-T7** | ❌ **PENDING** | Before/after cost example, YC-ready explanation, architecture diagrams deferred. |

**Phase 6 Progress:** 5/8 complete, 0/8 in progress, 3/8 pending

---

## Pillar 7 — Adoption Path

**Objective:** Reduce friction so a new user can run Northroot in under 10 minutes.

| Task | Status | Notes |
|------|--------|-------|
| **P7-T1** — Implement a "hello receipts" demo | ✅ **COMPLETE** | Hello receipts demo created at `sdk/python/northroot/examples/hello_receipts.py`. Simplest possible demo: 3 steps → 3 receipts. |
| **P7-T2** — Write a simple local-only install guide | ✅ **COMPLETE** | Installation guide created at `docs/guides/installation.md`. Covers PyPI install, local dev setup, platform support, and troubleshooting. |
| **P7-T3** through **P7-T8** | ❌ **PENDING** | Remaining tasks pending. |

**Phase 7 Progress:** 2/8 complete, 0/8 in progress, 6/8 pending

---

## Pillar 8 — Execution Discipline

**Objective:** Maintain stability, cut scope, and reliably ship v0.1 without drift.

| Task | Status | Notes |
|------|--------|-------|
| **P8-T1** — Weekly status check | ✅ **ACTIVE** | This document serves as status tracking. |
| **P8-T2** through **P8-T8** | ⚠️ **PARTIAL** | Discipline rules exist but not all enforced. |

**Phase 8 Progress:** 1/8 active, 7/8 partial

---

## Implementation Status

### Completed Work

1. **CBOR Canonicalization (P1-T1)** ✅
   - Location: `crates/northroot-receipts/src/canonical/`
   - Status: Fully implemented, tested, RFC 8949 compliant
   - Tests: 8/8 passing

2. **Minimal SDK API (P2-T3)** ✅
   - Rust API: `crates/northroot-engine/src/api.rs`
   - Python Bindings: `sdk/northroot-sdk-python/src/receipts.rs`
   - Functions: `record_work()`, `verify_receipt()`
   - Status: Fully implemented and tested (Rust: 4/4 tests passing, Python: bindings compile)
   - **Implementation Commit:** `e1c46853875535f4a4c4cfdce1d2233fdfa84bbc`

3. **JSON Adapters (P2-T2)** ✅
   - Location: `crates/northroot-receipts/src/adapters/json.rs`
   - Status: Complete, handles JSON ↔ CBOR conversion

4. **Golden Tests (P1-T4)** ✅
   - Location: `crates/northroot-receipts/tests/test_drift_detection.rs`
   - Status: Baseline hashes locked, drift detection working

5. **Quickstart Example (P2-T7)** ✅
   - Location: `sdk/northroot-sdk-python/examples/quickstart.py`
   - Status: Complete, demonstrates minimal API usage
   - **Implementation Commit:** `e1c46853875535f4a4c4cfdce1d2233fdfa84bbc`

6. **Filesystem Receipt Store (P2-T1)** ✅
   - Location: `crates/northroot-storage/src/filesystem.rs`
   - Status: Complete, stores receipts as JSON files, all tests passing (3/3)
   - **Implementation Commit:** `68037d7fa3ca317f48ac514ca833714451e19edd`

7. **Delta-Reuse Criteria Stabilization (P1-T5)** ✅
   - Location: `crates/northroot-engine/src/delta/decision.rs`
   - Status: Complete, core decision logic documented and locked. Reuse rule stabilized: `Reuse iff J > C_id / (α · C_comp)`. Economic delta calculation implemented. All tests passing (7/7).
   - **Implementation Commit:** (pending commit)

8. **Engine Crate Structure Cleanup (P1-T6)** ✅
   - Location: `crates/northroot-engine/src/`
   - Status: Complete, removed deprecated `execution/merkle_row_map.rs` (moved to `rowmap.rs`). All modules are clear and well-organized. No dead code found.
   - **Implementation Commit:** (pending commit)

### Critical Next Steps (Aligned with Goal Grid)

1. **P1-T2:** Finalize and document hashing/domain separation rules
   - Document in `docs/` or `crates/northroot-engine/src/commitments.rs`

3. **P1-T7:** Create developer-facing canonical forms reference
   - Document CBOR canonicalization rules
   - Provide examples

### Drift Prevention

- ✅ All work references goal grid task IDs
- ✅ ADR-0012 is the single source of truth
- ✅ No work done outside goal grid scope
- ✅ Python SDK bindings created (P2-T3 complete)

---

## Notes

- **Date Format:** All dates use `yyyy-mm-dd` format per workspace rules
- **ADR Alignment:** All work must map to ADR-0012 phases
- **Scope Discipline:** Following P8-T3: SDK + demos > advanced engine work

