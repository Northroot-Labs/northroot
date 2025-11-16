---
Title: Northroot — Goal Grid v0.1
Version: 0.1
Status: active
Epoch: "v0.1 - Ship minimal verifiable compute layer + Python SDK. 
Effective: 2025-11-15

Note: This plan uses a goal grid framework that organizes work into a 64-cell
grid (8 pillars × 8 tasks). See docs/planning/goal-grid.md for more information.
---

## Northroot - Goal Grid v0.1

## Central Goal

**CG-1 — Central Goal**

Ship a reliable, minimal verifiable-compute layer with a Python SDK and a working end-to-end demo that real developers can adopt.

---

## Pillar 1 — Engine Rigor

**P1 — Objective**  
A deterministic, mathematically correct, minimal engine with stable semantics.

**Subtasks**

- **P1-T1** — Lock CBOR canonicalization (inputs → stable bytes).  
- **P1-T2** — Finalize hashing and domain separation rules.  
- **P1-T3** — Freeze v0.1 chunk model (Proof, Evidence, WorkReceipt).  
- **P1-T4** — Implement golden tests for serialization.  
- **P1-T5** — Stabilize delta-reuse criteria in code.  
- **P1-T6** — Clean up engine crate structure (no dead code, clear modules).  
- **P1-T7** — Document canonical forms (developer-facing reference).  
- **P1-T8** — Establish a reproducible test suite for the core engine.

---

## Pillar 2 — Python SDK

**P2 — Objective**  
Provide a frictionless thin-client Python SDK that hides engine complexity.

**Subtasks**

- **P2-T1** — Implement simple local receipt store (filesystem-based).  
- **P2-T2** — Add JSON boundary adapter (JCS → CBOR at the edges).  
- **P2-T3** — Define a stable, minimal Python API surface.  
- **P2-T4** — Provide both async and sync call paths.  
- **P2-T5** — Add a clear exception hierarchy for SDK errors.  
- **P2-T6** — Create typed result objects (Receipt, Proof, etc.).  
- **P2-T7** — Produce a 10–15 line quickstart example.  
- **P2-T8** — Package and publish a clean PyPI release.

---

## Pillar 3 — Receipt Geometry

**P3 — Objective**  
Standardized, composable receipt types that are stable across versions.

**Subtasks**

- **P3-T1** — Lock shapes for Proof, Evidence, WorkReceipt, Settlement.  
- **P3-T2** — Define optional fragments (e.g., Quality, Economics).  
- **P3-T3** — Produce schema definitions (JSON/CBOR) for all receipts.  
- **P3-T4** — Implement version tags and migration strategy.  
- **P3-T5** — Build canonical decode → encode test paths.  
- **P3-T6** — Document receipt semantics and lifecycle.  
- **P3-T7** — Ship visual diagrams of receipt relationships.  
- **P3-T8** — Provide sample receipts as fixtures in the repo.

---

## Pillar 4 — Delta Compute Economics

**P4 — Objective**  
A principled economic model that justifies reuse and clarifies “when it’s worth it.”

**Subtasks**

- **P4-T1** — Finalize Jaccard/weighted overlap implementation.  
- **P4-T2** — Integrate operator incrementality factor α.  
- **P4-T3** — Implement reuse break-even condition in code.  
- **P4-T4** — Add “full / delta / none” reuse classification.  
- **P4-T5** — Encode deflationary compute inequality tests.  
- **P4-T6** — Produce worked numerical examples in docs.  
- **P4-T7** — Add a small benchmark/simulation for reuse scenarios.  
- **P4-T8** — Validate the model across 3 synthetic workloads.

---

## Pillar 5 — Observability & Integration

**P5 — Objective**  
Make adoption easy by mapping existing logs/traces to proofs.

**Subtasks**

- **P5-T1** — Build an OTEL span → proof transformer.  
- **P5-T2** — Tag compute steps with deterministic IDs.  
- **P5-T3** — Provide structured-logging example in Python.  
- **P5-T4** — Add a local “sidecar” receipts collector mode.  
- **P5-T5** — Build minimal integration examples (Airflow/Prefect/Dagster).  
- **P5-T6** — Document the difference between logs and proofs.  
- **P5-T7** — Provide a trace → receipt conversion demo.  
- **P5-T8** — Ensure engine/SDK logs are minimal and clean.

---

## Pillar 6 — Product Narrative

**P6 — Objective**  
A simple, credible explanation that engineers and investors understand instantly.

**Subtasks**

- **P6-T1** — Write problem framing around redundant/opaque compute.  
- **P6-T2** — Define a succinct positioning: “verifiable compute plumbing.”  
- **P6-T3** — Explain why proofs beat logs (determinism + reuse + auditability).  
- **P6-T4** — Author a before/after cost example using a toy pipeline.  
- **P6-T5** — Draft a YC-ready explanation (problem, wedge, upside).  
- **P6-T6** — Document 3 concrete use cases (FinOps, ML workflows, ETL).  
- **P6-T7** — Produce 2–3 architecture diagrams for docs/README.  
- **P6-T8** — Write a clear, minimal README that aligns with all of the above.

---

## Pillar 7 — Adoption Path

**P7 — Objective**  
Reduce friction so a new user can run Northroot in under 10 minutes.

**Subtasks**

- **P7-T1** — Implement a “hello receipts” demo (3 steps → 3 receipts).  
- **P7-T2** — Write a simple local-only install guide.  
- **P7-T3** — Provide a Docker-free setup path.  
- **P7-T4** — Build a minimal config file template.  
- **P7-T5** — Add a short “expand your workflow” tutorial.  
- **P7-T6** — Show a Python pipeline example (3–4 functions + receipts).  
- **P7-T7** — Add a troubleshooting guide for common failures.  
- **P7-T8** — Create issue templates and a contribution guide.

---

## Pillar 8 — Execution Discipline

**P8 — Objective**  
Maintain stability, cut scope, and reliably ship v0.1 without drift.

**Subtasks**

- **P8-T1** — Do a weekly 1-page status + scope check.  
- **P8-T2** — Define a 4-week feature freeze cadence for v0.x.  
- **P8-T3** — Enforce prioritization: SDK + demos > advanced engine work.  
- **P8-T4** — Aggressively reject side-quests (agents, extra frameworks).  
- **P8-T5** — Maintain fewer than 5 active TODOs at any time.  
- **P8-T6** — Enforce tests only for critical paths (no test bloat).  
- **P8-T7** — Keep repo structure small and legible.  
- **P8-T8** — Re-evaluate “next milestone” only after shipping one.

---