CORE_INVARIANTS.md

Status: Binding
Audience: Core maintainers, reviewers, auditors
Scope: Trust kernel semantics only (canonicalization, event identity, journal format)
Change Control: Any violation of these invariants is a breaking change

⸻

0. Purpose

This document defines the non-negotiable invariants of the Northroot trust kernel.
They exist to ensure neutrality, auditability, survivability, and correctness under scale, automation, and unknown future intelligence.

The trust kernel provides:
- Deterministic canonicalization (RFC 8785 + Northroot rules)
- Content-derived event identity
- Append-only journal format (.nrj)
- Offline verification primitives

Domain-specific concerns (authorization, execution, policy evaluation) are defined in DOMAIN_INVARIANTS.md as guidance for domain implementations.

If a proposed change weakens any invariant herein, it must not be merged into core.

⸻

1. Definitions

Evidence
Deterministic, structured facts that can be verified offline without semantic interpretation. Every evidence event is the canonical JSON object defined by its schema; there is no separate envelope layer, and every field that affects verification is part of the schema-defined payload.

Event Identity
A content-derived identifier computed as:
`event_id = H(domain_separator || canonical_json(event))`

Where `H` is a versioned hash function (SHA-256 for v1) and `canonical_json` follows the Northroot canonicalization profile.

Journal
An append-only, framed binary format (.nrj) for storing canonical event JSON. The format is self-describing, portable, and suitable for offline verification.

⸻

2. Scope & Responsibility Invariants

INV-1 — Core does not execute

The core MUST NOT perform irreversible actions.

Rationale: An auditor cannot audit itself.
Enforcement: Core APIs are canonicalization, event identity computation, journal I/O, and verification only.

⸻

INV-2 — Core does not decide outcomes

The core MUST NOT judge intent, correctness, desirability, or optimality.

Rationale: Interpretation collapses neutrality.
Enforcement: No scoring, recommendations, or reasoning in core.

INV-2a — Canonical payloads are schema-defined

All core evidence is the canonical JSON object. There is no separate `v` envelope, and every field that affects verification must be present in the object hashed as `event_id`. The kernel operates on untyped `EventJson = serde_json::Value`; domain layers add typed schemas.

Operational metadata (request IDs, traces, retries, provider hints, tags, transport headers, etc.) lives outside the canonical event to keep hashes deterministic.

⸻

3. Determinism Invariants

INV-3 — Canonical encoding is deterministic

For any core type T, canonical_encode(T) MUST produce identical bytes for the same semantic value across platforms and time.

The canonicalization profile (RFC 8785 + Northroot numeric rules) must be strictly applied. No platform-specific variations are permitted.

⸻

INV-4 — Identity is content-derived

All core identifiers MUST be derived from canonical bytes:

id = HASH(domain_separator || canonical_bytes)

(Hash function is versioned and fixed per major version. Current: SHA-256 with base64url-no-pad encoding.)

⸻

INV-5 — Verification is offline and deterministic

verify() MUST produce identical results offline, without network access.

Forbidden dependencies: live APIs, clocks (except recorded timestamps), mutable global state.

⸻

4. Journal Format Invariants

INV-6 — Journal is append-only

Once written, journal records MUST NOT be modified or deleted.

Corrections are represented as new events. The journal format enforces append-only semantics at the framing level.

⸻

INV-7 — Journal is self-describing

The journal format MUST include:
- Explicit version markers
- Frame boundaries that enable partial reads
- Forward-compatible extension points

Readers must be able to skip unknown frame types without breaking verification.

⸻

INV-8 — Journal events are verifiable offline

Every event stored in a journal MUST be verifiable by:
1. Parsing the JSON object
2. Canonicalizing according to the event's `canonical_profile_id`
3. Recomputing `event_id` and comparing to stored value

No external dependencies required.

⸻

5. Violation & Failure Invariants

INV-9 — Outcome states are explicit

Verification MUST return one of:
- Ok (event_id matches, canonicalization valid)
- Invalid (event_id mismatch, malformed JSON, canonicalization failure)

No implicit success.

⸻

INV-10 — Fail closed on missing evidence

If evidence required to verify an event is missing or malformed, the result MUST be Invalid.

⸻

6. Tamper-Evidence & Survivability Invariants

INV-11 — Events are immutable and append-only

Once emitted, events MUST NOT be modified or deleted.

The journal format enforces this at the storage layer.

⸻

INV-12 — Evidence is exportable and replayable

Events and journals MUST be exportable in a self-contained bundle sufficient for offline verification.

The .nrj format is designed for portability across systems, regimes, and storage backends.

⸻

INV-13 — Continuity proofs are supported (optional)

The design SHOULD support hash chaining via optional `prev_event_id` fields to detect deletion/reordering.

Domain layers may implement checkpoint/attestation mechanisms for chain integrity, but these are not core primitives.

⸻

7. Neutrality Invariants

INV-14 — Vendor and framework agnostic

Core semantics MUST NOT depend on:
- specific AI providers
- agent frameworks
- orchestration systems
- cloud vendors
- storage backends (beyond the .nrj format itself)

⸻

INV-15 — Schema semantics remain external

Core provides canonicalization and identity computation for any JSON object conforming to a schema.

Schema validation and domain-specific verification remain external concerns.

⸻

8. Non-Goals (Core)

The following are explicitly out of scope:
- orchestration
- agent planning
- execution engines
- optimization logic
- dashboards
- distributed consensus
- policy evaluation
- authorization/execution lifecycle management

⸻

9. Amendment Rule

Any change weakening invariants 1–15 is a breaking change and must be rejected or moved outside core.

⸻

⸻

tests/golden/ — Golden Test Plan

This is how you lock correctness early and permanently.

A. Canonical Encoding Vectors

Create fixtures for core canonicalization:

tests/golden/canonical/
  simple_object.json
  nested_object.json
  with_array.json
  with_quantities.json
  unicode.json
  empty_values.json
  complex.json

Each test must assert:
- byte-for-byte canonical output
- stable across runs
- stable across platforms

⸻

B. Hash Identity Vectors

For each canonical fixture:
- compute expected hash
- store as literal

tests/golden/hashes.toml

Changing a hash requires an explicit, reviewed breaking change.

⸻

C. Event ID Vectors

Test cases:
1. Valid event with correct event_id
2. Event with tampered event_id
3. Event with missing event_id
4. Event with malformed canonical JSON

Expected verdicts are fixed.

⸻

D. Journal Format Vectors

Test cases:
1. Valid journal with single event
2. Valid journal with multiple events
3. Journal with truncated frame
4. Journal with unknown frame type (must skip)
5. Journal with malformed JSON in event

⸻

E. Offline Verification Test
- Serialize events + journal to a bundle
- Delete network access
- Re-run verifier
- Assert identical verdicts

⸻

F. Tamper Detection Tests

Modify:
- one byte of an event JSON
- reorder events in journal
- remove one event frame

All MUST result in Invalid.

⸻
