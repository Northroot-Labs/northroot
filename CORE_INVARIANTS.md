CORE_INVARIANTS.md

Status: Binding
Audience: Core maintainers, reviewers, auditors
Scope: Core semantics only
Change Control: Any violation of these invariants is a breaking change

⸻

0. Purpose

This document defines the non-negotiable invariants of the Northroot core.
They exist to ensure neutrality, auditability, survivability, and correctness under scale, automation, and unknown future intelligence.

If a proposed change weakens any invariant herein, it must not be merged into core.

⸻

1. Definitions

Irreversible action
Any action that materially changes external state or creates non-trivial cost, risk, or liability, including but not limited to:
	•	financial spend
	•	data mutation
	•	infrastructure changes
	•	privileged access
	•	automated communications
	•	LLM calls with non-trivial cost or side effects

Boundary / Gate
A pre-action enforcement point that binds policy, constraints, and evidence before an irreversible action occurs.

Evidence
Deterministic, structured facts that can be verified offline without semantic interpretation. Every evidence event is the canonical JSON object defined by its schema; there is no separate envelope layer, and every field that affects verification is part of the schema-defined payload.

Receipt
An immutable, content-addressed record binding:
	•	proposal / intent
	•	authorization constraints
	•	execution measurements
	•	verification context

⸻

2. Scope & Responsibility Invariants

INV-1 — Core does not execute

The core MUST NOT perform irreversible actions.

Rationale: An auditor cannot audit itself.
Enforcement: Core APIs are authorization, receipt emission, and verification only.

⸻

INV-2 — Core does not decide outcomes

The core MUST NOT judge intent, correctness, desirability, or optimality.

Rationale: Interpretation collapses neutrality.
Enforcement: No scoring, recommendations, or reasoning in core.

INV-2a — Canonical payloads are schema-defined

All core evidence is the canonical JSON object described by the schema (authorization, execution, checkpoint, attestation, etc.). There is no separate `v` envelope, and every schema field that affects verification must be present in the object hashed as `event_id`.

Operational metadata (request IDs, traces, retries, provider hints, tags, transport headers, etc.) lives outside the canonical event to keep hashes deterministic.

⸻

3. Determinism Invariants

INV-3 — Canonical encoding is deterministic

For any core type T, canonical_encode(T) MUST produce identical bytes for the same semantic value across platforms and time.

⸻

INV-4 — Identity is content-derived

All core identifiers MUST be derived from canonical bytes:

id = HASH(canonical_bytes)

(Hash function is versioned and fixed per major version.)

⸻

INV-5 — Verification is offline and deterministic

verify() MUST produce identical results offline, without network access.

Forbidden dependencies: live APIs, clocks (except recorded timestamps), mutable global state.

⸻

4. Authorization Invariants

INV-6 — No irreversible action without pre-authorization

Every irreversible action MUST be preceded by an AuthorizationReceipt.

No bypasses. No fast paths.

⸻

INV-7 — Authorization binds intent and policy by hash

An AuthorizationReceipt MUST bind:
	•	intent_hash
	•	policy_hash
	•	explicit constraints (budgets, limits, scope)

Human-readable policy names are metadata only.

⸻

INV-8 — Authorization is time- and scope-bounded

Authorization MUST include:
	•	issuance time
	•	optional expiry
	•	explicit scope (org / principal / environment)

Expired or out-of-scope authorization is invalid.

⸻

5. Execution & Attestation Invariants

INV-9 — Execution links to authorization

Every ExecutionReceipt MUST reference exactly one AuthorizationReceipt.

Unlinked execution is invalid.

INV-9a — Attestations may carry multiple signatures

Attestation evidence MUST allow multiple independent signatures (1–16 entries) so different trust anchors can co-sign the same checkpoint without altering canonical identity. Each signature is part of the canonical payload and must be verified deterministically.

⸻

INV-10 — Execution is attestable, not explainable

Execution evidence MUST be factual and measured:
	•	resource usage
	•	observed provider / model (if applicable)
	•	outcome classification
	•	artifact hashes

Explanations are NOT evidence.

⸻

INV-11 — Execution reports are hash-bound

If an ExecutionReport is supplied, its hash MUST be bound in the ExecutionReceipt.

⸻

6. Violation & Failure Invariants

INV-12 — Outcome states are explicit

Verification MUST return one of:
	•	Ok
	•	Denied
	•	Violation
	•	Invalid

No implicit success.

⸻

INV-13 — Fail closed on missing evidence

If evidence required to verify a constraint is missing, the result MUST be Invalid.

⸻

INV-14 — Violations reduce power

The design MUST support downstream enforcement where violations reduce future capability (budgets, quotas, revocation).

Core emits signals; enforcement is external.

⸻

7. Tamper-Evidence & Survivability Invariants

INV-15 — Receipts are immutable and append-only

Once emitted, receipts MUST NOT be modified or deleted.

⸻

INV-16 — Evidence is exportable and replayable

Receipts and artifacts MUST be exportable in a self-contained bundle sufficient for offline verification.

⸻

INV-17 — Continuity proofs are supported (optional)

The design SHOULD support hash chaining or checkpoints to detect deletion/reordering.

⸻

8. Neutrality Invariants

INV-18 — Vendor and framework agnostic

Core semantics MUST NOT depend on:
	•	specific AI providers
	•	agent frameworks
	•	orchestration systems
	•	cloud vendors

⸻

INV-19 — Policy semantics remain external

Core references policy by hash and decision result only.
Policy engines remain replaceable.

⸻

9. Non-Goals (Core)

The following are explicitly out of scope:
	•	orchestration
	•	agent planning
	•	execution engines
	•	optimization logic
	•	dashboards
	•	distributed consensus

⸻

10. Amendment Rule

Any change weakening invariants 1–7 is a breaking change and must be rejected or moved outside core.

⸻

⸻

tests/golden/ — Golden Test Plan

This is how you lock correctness early and permanently.

A. Canonical Encoding Vectors

Create fixtures for each core type:

tests/golden/canonical/
  intent.json
  policy.json
  authorization.json
  execution.json

Each test must assert:
	•	byte-for-byte canonical output
	•	stable across runs
	•	stable across platforms

⸻

B. Hash Identity Vectors

For each canonical fixture:
	•	compute expected hash
	•	store as literal

tests/golden/hashes.toml

Changing a hash requires an explicit, reviewed breaking change.

⸻

C. Authorization Receipt Vectors

Test cases:
	1.	Valid authorization
	2.	Expired authorization
	3.	Policy hash mismatch
	4.	Constraint mismatch

Expected verdicts are fixed.

⸻

D. Execution Receipt Vectors

Test cases:
	1.	Valid execution under limits
	2.	Execution exceeding cost ceiling
	3.	Execution missing report
	4.	Execution with mismatched report hash

⸻

E. Verification Verdict Matrix

Create a table-driven test:

Auth	Exec	Evidence	Expected
✓	✓	complete	Ok
✓	✓	missing	Invalid
✓	✗	complete	Violation
✗	✓	complete	Invalid
✗	✗	—	Invalid

No ambiguity allowed.

⸻

F. Offline Verification Test
	•	Serialize receipts + artifacts to a bundle
	•	Delete network access
	•	Re-run verifier
	•	Assert identical verdicts

⸻

G. Tamper Detection Tests

Modify:
	•	one byte of a receipt
	•	reorder receipts
	•	remove one artifact

All MUST result in Invalid.

⸻

H. Halt-Mode Readiness (future-proof)

Even if not enforced in v0:
	•	ensure severity classification is emitted
	•	ensure PANIC-level signals are testable

⸻