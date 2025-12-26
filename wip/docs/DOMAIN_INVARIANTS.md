DOMAIN_INVARIANTS.md

Status: Advisory
Audience: Domain implementers, application developers
Scope: Guidance for domain-specific event schemas and verification patterns
Change Control: These are recommendations, not binding core constraints

⸻

0. Purpose

This document provides guidance for implementing domain-specific event schemas and verification patterns on top of the Northroot trust kernel.

The trust kernel (see CORE_INVARIANTS.md) provides canonicalization, event identity, and journal format. Domain layers build on these primitives to implement authorization, execution tracking, policy evaluation, and other application-specific concerns.

These invariants are advisory: they represent best practices for building audit-grade domain systems, but are not enforced by the core.

⸻

1. Definitions

Irreversible action
Any action that materially changes external state or creates non-trivial cost, risk, or liability, including but not limited to:
- financial spend
- data mutation
- infrastructure changes
- privileged access
- automated communications
- LLM calls with non-trivial cost or side effects

Boundary / Gate
A pre-action enforcement point that binds policy, constraints, and evidence before an irreversible action occurs.

Authorization Receipt
A verifiable event recording a policy decision (allow/deny) for a proposed action, binding intent, policy identity, and constraints.

Execution Receipt
A verifiable event recording the outcome of an executed action, binding authorization reference, measurements, and artifacts.

⸻

2. Authorization Invariants (Advisory)

INV-D1 — No irreversible action without pre-authorization

Every irreversible action SHOULD be preceded by an AuthorizationReceipt.

No bypasses. No fast paths.

Rationale: Provides audit trail and enables policy enforcement.

⸻

INV-D2 — Authorization binds intent and policy by hash

An AuthorizationReceipt SHOULD bind:
- intent_hash
- policy_hash
- explicit constraints (budgets, limits, scope)

Human-readable policy names are metadata only.

⸻

INV-D3 — Authorization is time- and scope-bounded

Authorization SHOULD include:
- issuance time
- optional expiry
- explicit scope (org / principal / environment)

Expired or out-of-scope authorization is invalid.

⸻

3. Execution & Attestation Invariants (Advisory)

INV-D4 — Execution links to authorization

Every ExecutionReceipt SHOULD reference exactly one AuthorizationReceipt.

Unlinked execution is invalid.

⸻

INV-D5 — Attestations may carry multiple signatures

Attestation evidence SHOULD allow multiple independent signatures (1–16 entries) so different trust anchors can co-sign the same checkpoint without altering canonical identity. Each signature is part of the canonical payload and must be verified deterministically.

⸻

INV-D6 — Execution is attestable, not explainable

Execution evidence SHOULD be factual and measured:
- resource usage
- observed provider / model (if applicable)
- outcome classification
- artifact hashes

Explanations are NOT evidence.

⸻

INV-D7 — Execution reports are hash-bound

If an ExecutionReport is supplied, its hash SHOULD be bound in the ExecutionReceipt.

⸻

4. Violation & Failure Patterns (Advisory)

INV-D8 — Outcome states are explicit

Domain verification SHOULD return one of:
- Ok
- Denied
- Violation
- Invalid

No implicit success.

⸻

INV-D9 — Fail closed on missing evidence

If evidence required to verify a constraint is missing, the result SHOULD be Invalid.

⸻

INV-D10 — Violations reduce power

The design SHOULD support downstream enforcement where violations reduce future capability (budgets, quotas, revocation).

Core emits signals; enforcement is external.

⸻

5. Golden Test Patterns (Advisory)

For domain-specific event types, consider:

A. Authorization Receipt Vectors

Test cases:
1. Valid authorization
2. Expired authorization
3. Policy hash mismatch
4. Constraint mismatch

Expected verdicts should be fixed and documented.

⸻

B. Execution Receipt Vectors

Test cases:
1. Valid execution under limits
2. Execution exceeding cost ceiling
3. Execution missing report
4. Execution with mismatched report hash

⸻

C. Verification Verdict Matrix

Create a table-driven test:

Auth	Exec	Evidence	Expected
✓	✓	complete	Ok
✓	✓	missing	Invalid
✓	✗	complete	Violation
✗	✓	complete	Invalid
✗	✗	—	Invalid

No ambiguity allowed.

⸻

6. Notes for Domain Implementers

- Authorization and execution event schemas are domain-specific and not part of the core trust kernel.
- The trust kernel provides canonicalization and event identity; domain layers add semantic verification.
- Policy evaluation engines are external to the core and can be replaced without affecting event identity or journal format.
- Domain-specific schemas should follow the same canonicalization rules as core governance events (checkpoint, attestation).

⸻

