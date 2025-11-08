Northroot Repo Structure Playbook (v0.1)

Purpose
A lightweight rubric so humans and agents can place code in the right place, evolve the repo safely, and keep refactors cheap.

⸻

Golden rules
	1.	Separation by concern, not by file type. Proof/receipt logic ≠ execution logic ≠ adapters.
	2.	Dependency arrows point inward. Outer layers may depend on inner layers; never the reverse.
	3.	Public APIs stay small. Expose types/traits; hide helpers behind pub(crate).
	4.	One source of truth. Canonicalization, hashing, and validation live in one crate.
	5.	Everything testable in isolation. Each crate has unit tests + vectors; top-level has integration tests.
	6.	Document intent with ADRs. One page per architectural decision.
	7.	Prefer composition over features. New behaviors become strategy types/traits, not booleans.

⸻

Where code belongs

If code primarily does…	It goes in…
Compute reuse, delta decisions, chunking, runners, kernels	engine
Receipt envelope/types, canonicalization (JCS), hashing, validation	receipts
Operator & method manifests (schemas, examples, validators)	ops
Policies & strategies (cost models, reuse thresholds, allow/deny, FP tolerances)	policy
DSL for intents + planner (capability matching)	planner
SDK & adapters (OTel exporter, CLI, language shims)	sdk/*
Cross-cutting utils (logging, error types)	commons
App/API surfaces (HTTP, TUI)	apps/*

Heuristic: if two crates would need each other, one of them is mis-scoped. Introduce a third, deeper crate they both depend on.

⸻

Decision tree (new work)
	1.	Does it change how compute is performed or reused? → engine (trait or operator impl).
	2.	Does it define or verify evidence of work? → receipts (types/validators).
	3.	Is it “what can be run” metadata? → ops (schemas + generators).
	4.	Is it “when/how to run” policy? → policy (strategies + validators).
	5.	Is it translating human/agent goal → method? → planner.
	6.	Is it integration code or UX? → sdk/* or apps/*.

If multiple, split by the direction of dependency: deeper concern owns the types; higher layer holds orchestrators.

⸻

Boundaries & allowed deps

commons   → (no deps inward)
receipts  → commons
engine    → commons, receipts
ops       → commons, receipts
policy    → commons, receipts
planner   → commons, receipts, ops, policy (read-only)
sdk/*     → commons, receipts, ops (never engine-internal)
apps/*    → sdk/*, planner, engine (through stable traits)

Forbidden: receipts depending on engine; policy depending on engine; cyclical edges.

⸻

Versioning & features
	•	Crate versions: semver; bump minor for additive schema/traits; major for breaking canonicalization.
	•	Feature flags: only for optional deps (e.g., serde_json/alloc, ring vs ed25519-dalek). Avoid feature matrices that change behavior.

⸻

Testing layout
	•	receipts/tests/ golden vectors, hash integrity, composition checks.
	•	engine/tests/ delta decisions, chunkers, deterministic runners.
	•	ops/tests/ manifest round-trips, schema validation.
	•	policy/tests/ threshold math, cost-model fixtures.
	•	tests/ (workspace) end-to-end: method → execution → spend → settlement.

⸻

Naming & paths
	•	Crates: northroot-{receipts|engine|ops|policy|planner|commons}.
	•	Modules: commitments, canonical, validation, delta, chunking, cost, strategy.
	•	Schemas live in /schemas/{receipts|ops|policy}/... with mirrors in code.
	•	Docs in /docs/specs/* (one spec per concept; link from READMEs).

⸻

When to split a new crate

Create a new crate if two or more existing crates would otherwise both need to depend on a new module and that module has a stable API that others can rely on.

⸻

ADR template (1-page)

# ADR-XXX: <Decision>
Date: YYYY-MM-DD
Status: Proposed | Accepted | Superseded by ADR-YYY
Context: <Why a decision is needed>
Decision: <What we are doing>
Consequences: <Pros/cons, trade-offs>
Alternatives: <Considered, rejected>
Migration: <Steps to adopt>

Store in /docs/adrs/ADR-XXX.md and reference from crate READMEs.

⸻

Examples
	•	Add a new delta strategy: trait in engine::delta::Strategy; register in policy with thresholds; expose via apps/tui for toggling.
	•	Add a new operator: manifest JSON + validator in ops; execution impl in engine behind Operator trait; include golden vectors.
	•	Add a new receipt kind field: update receipts types + schema + vectors; never touch engine unless semantics change.

⸻

Quick checklist before merging
	•	Code placed according to table
	•	No forbidden deps (run cargo machete/cargo depgraph)
	•	Public API documented; private helpers are pub(crate)
	•	Tests + vectors updated
	•	ADR added/updated if architectural