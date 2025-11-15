# v0.1 Execution Plan: Core Receipt Lifecycle

**Date:** 2025-11-15  
**Focus:** Recording, storing, retrieving, verifying, and listing receipts  
**Deferred:** Delta compute economics, advanced policies, full reuse mechanics

---

## Core v0.1 Goals

1. **Easy to use in real workloads** - Minimal friction, works with existing code
2. **Verifiability** - Prove work was done correctly
3. **Retrieval** - Get receipts at any time
4. **Listing** - Query/filter receipts when needed

---

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                    PHASE 1: FOUNDATION                       │
│              Pillar 1 - Engine Rigor (P1)                  │
│                    ✅ 8/8 COMPLETE                          │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│              PHASE 2: SDK CORE (CRITICAL PATH)              │
│            Pillar 2 - Python SDK (P2)                       │
│                                                             │
│  ✅ P2-T1: Filesystem store                                │
│  ✅ P2-T2: JSON adapters                                   │
│  ✅ P2-T3: Minimal API (record_work, verify_receipt)       │
│  ✅ P2-T4: Async/sync paths                                │
│  ✅ P2-T5: Exception hierarchy                             │
│  ✅ P2-T6: Typed results (PyReceipt)                       │
│  ✅ P2-T7: Quickstart example                                │
│  ❌ P2-T8: PyPI release ← BLOCKER                          │
└───────────────────────┬─────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  PHASE 3A    │ │  PHASE 3B    │ │  PHASE 3C    │
│ Observability│ │  Adoption    │ │  Narrative   │
│    (P5)      │ │   Path (P7)  │ │    (P6)      │
│              │ │              │ │              │
│ ✅ P5-T1:    │ │ ❌ P7-T1:    │ │ ❌ P6-T1:    │
│   OTEL       │ │   Hello demo │ │   Problem    │
│              │ │              │ │   framing    │
│ ❌ P5-T2:    │ │ ❌ P7-T2:    │ │ ❌ P6-T2:    │
│   Deterministic│ │   Install    │ │   Positioning│
│   IDs        │ │   guide      │ │              │
│              │ │              │ │ ❌ P6-T8:    │
│ ❌ P5-T3:    │ │ ❌ P7-T3:    │ │   README     │
│   Structured │ │   Docker-free│ │              │
│   logging    │ │              │ │              │
└──────────────┘ └──────────────┘ └──────────────┘
        │               │               │
        └───────────────┼───────────────┘
                        │
                        ▼
              ┌─────────────────┐
              │  v0.1 SHIP      │
              └─────────────────┘

DEFERRED (Post-v0.1):
- Pillar 3 (Receipt Geometry) - Advanced shapes
- Pillar 4 (Delta Economics) - Reuse economics
- Pillar 8 (Discipline) - Ongoing, not blocking
```

---

## Execution Plan: Core Receipt Lifecycle

### Phase 1: Foundation ✅ COMPLETE

**Status:** All P1 tasks done (8/8)

**What's Done:**
- CBOR canonicalization locked
- Hashing rules finalized
- Chunk model frozen
- Golden tests implemented
- Delta-reuse criteria stabilized
- Engine structure cleaned
- Canonical forms documented
- Test suite established

**No action needed** - Foundation is solid.

---

### Phase 2: SDK Core (CRITICAL PATH)

**Status:** 7/8 complete, 1 blocker

#### ✅ Completed (7/8)

- **P2-T1:** Filesystem receipt store
- **P2-T2:** JSON boundary adapters
- **P2-T3:** Minimal API (`record_work`, `verify_receipt`)
- **P2-T4:** Async/sync call paths
- **P2-T5:** Exception hierarchy
- **P2-T6:** Typed result objects (`PyReceipt`)
- **P2-T7:** Quickstart example

#### ❌ Remaining: P2-T8 - PyPI Release

**Task:** Package and publish clean PyPI release

**Requirements:**
1. Update `pyproject.toml` metadata (description, keywords, classifiers)
2. Ensure README is complete
3. Add LICENSE file reference
4. Test local build: `maturin build`
5. Test install: `pip install dist/*.whl`
6. Create PyPI account (if needed)
7. Publish: `maturin publish` or `twine upload`

**Estimated Time:** 2-3 hours

**Dependencies:** None (can start immediately)

---

### Phase 3A: Observability & Integration

**Status:** 1/8 complete

**Focus:** Make it easy to use in real workloads

#### ✅ Completed

- **P5-T1:** OTEL span → receipt transformer

#### 🔄 Next Priority Tasks

**P5-T2: Tag compute steps with deterministic IDs**
- **Goal:** Ensure every receipt has a stable, deterministic ID
- **Current:** Using UUID v4 (random)
- **Action:** Document ID generation strategy, ensure trace_id is deterministic
- **Estimated Time:** 1-2 hours
- **Dependencies:** P2-T3 (API exists)

**P5-T3: Provide structured-logging example**
- **Goal:** Show how to integrate with Python logging
- **Action:** Create `examples/structured_logging.py`
- **Estimated Time:** 1-2 hours
- **Dependencies:** P2-T3 (API exists)

**P5-T7: Provide trace → receipt conversion demo**
- **Goal:** Show end-to-end trace conversion
- **Action:** Enhance `examples/otel_integration.py` with full trace demo
- **Estimated Time:** 1-2 hours
- **Dependencies:** P5-T1 (OTEL transformer exists)

**P5-T8: Ensure engine/SDK logs are minimal and clean**
- **Goal:** No verbose logging in production
- **Action:** Audit logging, set appropriate log levels
- **Estimated Time:** 1 hour
- **Dependencies:** None

**Deferred (Post-v0.1):**
- P5-T4: Sidecar receipts collector (advanced)
- P5-T5: Airflow/Prefect/Dagster integrations (can be examples)
- P5-T6: Logs vs proofs documentation (can be simple)

---

### Phase 3B: Adoption Path

**Status:** 0/8 complete

**Focus:** "Under 10 minutes" onboarding

#### 🔄 Critical Tasks

**P7-T1: Implement "hello receipts" demo**
- **Goal:** 3 steps → 3 receipts, dead simple
- **Action:** Create `examples/hello_receipts.py` (even simpler than quickstart)
- **Estimated Time:** 1 hour
- **Dependencies:** P2-T3 (API exists)

**P7-T2: Write simple local-only install guide**
- **Goal:** `pip install northroot` → working in 2 minutes
- **Action:** Create `docs/guides/installation.md`
- **Estimated Time:** 1 hour
- **Dependencies:** P2-T8 (PyPI release)

**P7-T3: Provide Docker-free setup path**
- **Goal:** No Docker required
- **Action:** Document native install (already true, just document)
- **Estimated Time:** 30 minutes
- **Dependencies:** None

**P7-T4: Build minimal config file template**
- **Goal:** Optional config for storage paths, etc.
- **Action:** Create `examples/config.toml` template
- **Estimated Time:** 1 hour
- **Dependencies:** P2-T1 (filesystem store exists)

**P7-T6: Show Python pipeline example**
- **Goal:** 3-4 functions + receipts
- **Action:** Create `examples/pipeline_example.py`
- **Estimated Time:** 2 hours
- **Dependencies:** P2-T3 (API exists)

**P7-T7: Add troubleshooting guide**
- **Goal:** Common failures and fixes
- **Action:** Create `docs/guides/troubleshooting.md`
- **Estimated Time:** 1-2 hours
- **Dependencies:** None (can start now)

**P7-T8: Create issue templates**
- **Goal:** GitHub issue templates
- **Action:** Create `.github/ISSUE_TEMPLATE/` files
- **Estimated Time:** 1 hour
- **Dependencies:** None

**Deferred:**
- P7-T5: "Expand your workflow" tutorial (can be post-v0.1)

---

### Phase 3C: Product Narrative

**Status:** 0/8 complete

**Focus:** Clear communication

#### 🔄 Critical Tasks

**P6-T8: Write clear, minimal README**
- **Goal:** Single-page explanation that works
- **Action:** Rewrite `README.md` with:
  - What it is (1 sentence)
  - Why it matters (2-3 sentences)
  - Quick install
  - 3-line example
  - Link to docs
- **Estimated Time:** 2-3 hours
- **Dependencies:** P7-T2 (install guide), P7-T1 (hello demo)

**P6-T1: Write problem framing**
- **Goal:** "Redundant/opaque compute" explanation
- **Action:** Add to README or separate doc
- **Estimated Time:** 1-2 hours
- **Dependencies:** None

**P6-T2: Define positioning**
- **Goal:** "Verifiable compute plumbing"
- **Action:** Add to README
- **Estimated Time:** 30 minutes
- **Dependencies:** None

**Deferred (Post-v0.1):**
- P6-T3: Why proofs beat logs (can be blog post)
- P6-T4: Before/after cost example (needs delta compute)
- P6-T5: YC-ready explanation (can be post-v0.1)
- P6-T6: 3 use cases (can be examples)
- P6-T7: Architecture diagrams (can be post-v0.1)

---

## Prioritized Execution Order

### Week 1: SDK Completion + Core Features

**Day 1-2: PyPI Release (P2-T8)**
- [ ] Finalize `pyproject.toml`
- [ ] Test local build
- [ ] Publish to PyPI
- **Blocker:** Must complete before adoption tasks

**Day 3: Receipt Listing/Query (NEW - Critical for v0.1)**
- [ ] Add `list_receipts()` to `FilesystemStore`
- [ ] Expose `client.list_receipts()` in Python SDK
- [ ] Add filtering by `workload_id`, `trace_id`, date range
- **Estimated:** 3-4 hours

**Day 4: Hello Receipts Demo (P7-T1)**
- [ ] Create `examples/hello_receipts.py`
- [ ] Test end-to-end
- **Estimated:** 1 hour

**Day 5: Installation Guide (P7-T2)**
- [ ] Write `docs/guides/installation.md`
- [ ] Test install steps
- **Estimated:** 1 hour

### Week 2: Observability + Adoption

**Day 1: Structured Logging (P5-T3)**
- [ ] Create `examples/structured_logging.py`
- **Estimated:** 1-2 hours

**Day 2: Deterministic IDs (P5-T2)**
- [ ] Document ID strategy
- [ ] Ensure trace_id is deterministic
- **Estimated:** 1-2 hours

**Day 3: Pipeline Example (P7-T6)**
- [ ] Create `examples/pipeline_example.py`
- **Estimated:** 2 hours

**Day 4: Troubleshooting Guide (P7-T7)**
- [ ] Create `docs/guides/troubleshooting.md`
- **Estimated:** 1-2 hours

**Day 5: README Rewrite (P6-T8)**
- [ ] Rewrite `README.md`
- [ ] Add problem framing (P6-T1)
- [ ] Add positioning (P6-T2)
- **Estimated:** 3-4 hours

### Week 3: Polish + Ship

**Day 1-2: Remaining Adoption Tasks**
- [ ] Docker-free setup (P7-T3) - 30 min
- [ ] Config template (P7-T4) - 1 hour
- [ ] Issue templates (P7-T8) - 1 hour
- [ ] Trace conversion demo (P5-T7) - 1-2 hours

**Day 3: Logging Cleanup (P5-T8)**
- [ ] Audit all logging
- [ ] Set appropriate levels
- **Estimated:** 1 hour

**Day 4-5: Final Testing + Documentation**
- [ ] End-to-end testing
- [ ] Documentation review
- [ ] Release preparation

---

## Critical Missing Feature: Receipt Listing

**Current State:** We can record, store, retrieve by RID, and verify receipts, but **cannot list/query receipts**.

**Required for v0.1:**
- List all receipts
- Filter by `workload_id`
- Filter by `trace_id`
- Filter by date range
- Get receipt count

**Implementation:**
1. Extend `FilesystemStore::query_receipts()` (already stubbed)
2. Add `Client.list_receipts()` method
3. Add filtering parameters
4. Add example usage

**Estimated Time:** 3-4 hours

---

## Summary: What Ships in v0.1

### ✅ Core Features
- Record receipts (`client.record_work()`)
- Store receipts (filesystem store)
- Retrieve receipts (`client.get_receipt(rid)`)
- Verify receipts (`client.verify_receipt()`)
- **List receipts** (`client.list_receipts()` - TO BE IMPLEMENTED)
- Async/sync support
- OTEL integration

### ✅ Developer Experience
- Simple installation (`pip install northroot`)
- Hello receipts demo
- Quickstart example
- Pipeline example
- Structured logging example
- Clear README
- Installation guide
- Troubleshooting guide

### ❌ Deferred (Post-v0.1)
- Advanced receipt shapes (P3)
- Delta compute economics (P4)
- Full reuse mechanics
- Advanced policy enforcement
- Sidecar collector (P5-T4)
- Workflow framework integrations (P5-T5) - can be examples

---

## Success Criteria for v0.1

1. ✅ Developer can install in < 2 minutes
2. ✅ Developer can record a receipt in < 5 lines of code
3. ✅ Developer can verify a receipt
4. ✅ Developer can list/query receipts
5. ✅ Developer can integrate with existing OTEL traces
6. ✅ All examples run without errors
7. ✅ README is clear and complete
8. ✅ Package is published to PyPI

---

## Next Immediate Actions

1. **Implement receipt listing** (3-4 hours) - Critical missing feature
2. **Complete P2-T8** (PyPI release) - Blocker for adoption
3. **Create hello receipts demo** (P7-T1) - Simplest onboarding
4. **Write installation guide** (P7-T2) - Required for "under 10 minutes"
5. **Rewrite README** (P6-T8) - First impression

**Total estimated time to v0.1:** 15-20 hours of focused work

