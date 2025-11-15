# v0.1 Dependency Graph

**Focus:** Core receipt lifecycle (record, store, retrieve, verify, list)  
**Date:** 2025-11-15

---

## Visual Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                    FOUNDATION LAYER                          │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Pillar 1: Engine Rigor (P1)                        │  │
│  │  ✅ 8/8 COMPLETE                                     │  │
│  │                                                      │  │
│  │  P1-T1: CBOR canonicalization ──┐                   │  │
│  │  P1-T2: Hashing rules ──────────┼─→ P1-T4: Tests   │  │
│  │  P1-T3: Chunk model ────────────┘                   │  │
│  │  P1-T5: Delta-reuse criteria                        │  │
│  │  P1-T6: Engine cleanup                              │  │
│  │  P1-T7: Documentation                               │  │
│  │  P1-T8: Test suite                                  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                        │
                        │ (All P1 complete)
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    SDK LAYER                                │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Pillar 2: Python SDK (P2)                           │  │
│  │  ✅ 7/8 COMPLETE | ❌ 1 BLOCKER                      │  │
│  │                                                      │  │
│  │  P2-T1: Filesystem store ──┐                        │  │
│  │  P2-T2: JSON adapters ──────┼─→ P2-T3: API surface │  │
│  │  P2-T3: Minimal API ────────┘                        │  │
│  │         │                                             │  │
│  │         ├─→ P2-T4: Async/sync                        │  │
│  │         ├─→ P2-T5: Exceptions                        │  │
│  │         ├─→ P2-T6: Typed results                     │  │
│  │         └─→ P2-T7: Quickstart                        │  │
│  │                                                      │  │
│  │  P2-T1..T7 ──→ P2-T8: PyPI release ← BLOCKER        │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ OBSERVABILITY│ │  ADOPTION    │ │  NARRATIVE   │
│    (P5)      │ │   PATH (P7)  │ │    (P6)      │
│              │ │              │ │              │
│ P2-T3 ──→    │ │ P2-T8 ──→    │ │              │
│ P5-T1: OTEL  │ │ P7-T1: Hello │ │ P6-T1: Problem│
│              │ │              │ │ P6-T2: Position│
│ P5-T1 ──→    │ │ P7-T1 ──→    │ │              │
│ P5-T2: IDs   │ │ P7-T2: Install│ │ P6-T1..T2 ──→│
│ P5-T3: Logging│ │ P7-T3: Docker│ │ P6-T8: README│
│ P5-T7: Demo   │ │ P7-T4: Config│ │              │
│              │ │ P7-T6: Pipeline│ │              │
│              │ │ P7-T7: Troubleshoot│              │
└──────────────┘ └──────────────┘ └──────────────┘
        │               │               │
        └───────────────┼───────────────┘
                        │
                        ▼
              ┌─────────────────┐
              │   v0.1 RELEASE   │
              └─────────────────┘
```

---

## Dependency Matrix

### Sequential Dependencies (Must Complete in Order)

| Task | Depends On | Can Start After |
|------|-----------|-----------------|
| **P1-T4** (Golden tests) | P1-T1, P1-T2 | P1-T1 ✅, P1-T2 ✅ |
| **P1-T7** (Documentation) | P1-T3 | P1-T3 ✅ |
| **P2-T3** (API surface) | P1-T1, P1-T2, P1-T3 | P1 complete ✅ |
| **P2-T4, T5, T6, T7** | P2-T3 | P2-T3 ✅ |
| **P2-T8** (PyPI) | P2-T1..T7 | P2-T1..T7 ✅ |
| **P5-T1** (OTEL) | P2-T3 | P2-T3 ✅ |
| **P5-T2, T3, T7** | P5-T1 | P5-T1 ✅ |
| **P7-T1** (Hello demo) | P2-T3 | P2-T3 ✅ |
| **P7-T2** (Install guide) | P2-T8 | P2-T8 ❌ |
| **P7-T6** (Pipeline) | P2-T3 | P2-T3 ✅ |
| **P6-T8** (README) | P7-T1, P7-T2 | P7-T1 ✅, P7-T2 ❌ |

### Parallel Opportunities (Can Run Concurrently)

**After P1 Complete:**
- P2-T1, P2-T2, P2-T3 can start in parallel
- P3 (Receipt Geometry) - **DEFERRED**
- P4 (Delta Economics) - **DEFERRED**
- P6-T1, P6-T2 (Narrative) - Can start now

**After P2-T3 Complete:**
- P2-T4, P2-T5, P2-T6 can run in parallel
- P5-T1 (OTEL) - ✅ Done
- P7-T1 (Hello demo) - Can start
- P7-T6 (Pipeline) - Can start

**After P2-T8 Complete:**
- P7-T2 (Install guide) - Can start
- P7-T3, P7-T4, P7-T7, P7-T8 - Can run in parallel

**After P5-T1 Complete:**
- P5-T2, P5-T3, P5-T7, P5-T8 - Can run in parallel

**Always Parallel:**
- P8 (Execution Discipline) - Ongoing, not blocking

---

## Critical Path (Longest Sequence)

```
P1-T1 → P1-T4
P1-T2 → P1-T4
P1-T3 → P1-T7
P1 (all) → P2-T3 → P2-T4/T5/T6/T7 → P2-T8 → P7-T2 → P6-T8
```

**Critical Path Length:** ~15-20 hours of work remaining

---

## Blockers

1. **P2-T8 (PyPI Release)** - Blocks P7-T2, P6-T8
2. **Receipt Listing Feature** - Missing, required for v0.1
3. **P7-T2 (Install Guide)** - Blocks P6-T8 (README)

---

## Deferred (Post-v0.1)

- **Pillar 3 (Receipt Geometry)** - Advanced shapes, schemas
- **Pillar 4 (Delta Economics)** - Reuse economics, break-even
- **P5-T4** (Sidecar collector) - Advanced feature
- **P5-T5** (Framework integrations) - Can be examples later
- **P5-T6** (Logs vs proofs) - Can be simple doc
- **P7-T5** (Expand workflow tutorial) - Post-v0.1

---

## Execution Priority

### 🔴 Critical (Must Complete for v0.1)
1. **Receipt Listing** (NEW - missing feature)
2. **P2-T8** (PyPI release)
3. **P7-T1** (Hello demo)
4. **P7-T2** (Install guide)
5. **P6-T8** (README)

### 🟡 High Priority (Strongly Recommended)
6. **P5-T2** (Deterministic IDs)
7. **P5-T3** (Structured logging)
8. **P7-T6** (Pipeline example)
9. **P7-T7** (Troubleshooting)

### 🟢 Nice to Have (Can Ship Without)
10. **P7-T3** (Docker-free setup)
11. **P7-T4** (Config template)
12. **P7-T8** (Issue templates)
13. **P5-T7** (Trace demo)
14. **P5-T8** (Logging cleanup)
15. **P6-T1, P6-T2** (Problem framing, positioning)

---

## Estimated Timeline

**Week 1:** Critical path (Receipt listing + P2-T8 + P7-T1 + P7-T2 + P6-T8)  
**Week 2:** High priority (P5-T2, T3, P7-T6, T7)  
**Week 3:** Polish + ship

**Total:** 15-20 hours of focused work to v0.1

