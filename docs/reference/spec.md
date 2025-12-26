Northroot Core Specification

Version: 1.0
Status: Stable core (additive changes only)
Scope: Verifiable events, canonical identities, journal storage

**Note**: This document defines the protocol specification (invariants, identity computation, verification model). For canonicalization rules, see [Canonicalization](canonicalization.md). For journal format, see [Journal Format](format.md). For extension patterns, see [Extensions](extensions.md).

---

## 1. Purpose

Northroot defines a minimal, neutral surface for recording verifiable events. The specification exists to deliver deterministic identity, append-only ordering, offline verification, and a foundation for audit-grade correctness without prescribing policy engines, runtimes, or storage backends.

The trust kernel provides:
- Canonicalization primitives (RFC 8785 + Northroot rules)
- Event identity computation
- Journal format (.nrj)

Domain-specific event types (authorization, execution, checkpoint, attestation, etc.) are defined by consuming applications or extension layers.

---

## 2. Core invariants

### 2.1 Canonical identity

Every event is a canonical JSON object. Identity is computed as:  
`event_id = H(domain_separator || canonical_json(event))`  
`canonical_json` follows the Northroot canonicalization profile, and `event` is the entire JSON object (type, version, principals, payload fields, etc.).

The domain separator for event identity is: `b"northroot:event:v1\0"`

### 2.2 Determinism and versioning

`event_version` captures schema evolution so readers know which canonicalization rules to apply. There is no separate `v` envelope: the event object itself is canonical, and every field contributes to the hash.

### 2.3 Append-only

Writers append events to a journal without mutation or deletion. Optional `prev_event_id` values may be present for hash-chain ordering but are not required for the core model.

### 2.4 Metadata isolation

Operational metadata (request IDs, traces, retries, provider hints, tags, transport headers, etc.) lives outside the canonical event so it cannot affect hashes or policy decisions.

---

## 3. Event structure

All events share a common envelope structure:

- `event_id`: Digest (computed from canonical bytes)
- `event_type`: string (e.g., "test", "checkpoint", "authorization", etc.)
- `event_version`: string (e.g., "1")
- `occurred_at`: Timestamp
- `principal_id`: PrincipalID
- `canonical_profile_id`: ProfileID
- Optional `prev_event_id`: Digest (for hash-chain ordering)
- Type-specific payload fields (defined by domain schemas)

Every field in the event is part of the canonical payload; the kernel operates on untyped `EventJson = serde_json::Value`. Domain layers add typed schemas and validation.

---

## 4. Verification model

Verifiers must:

1. Parse the journal record into the canonical JSON event object.
2. Apply the canonicalization profile associated with `canonical_profile_id`.
3. Recompute `event_id` from the canonical bytes and ensure it matches the stored digest.

Optional: use `prev_event_id` for hash-chain checks.

Domain-specific verification (policy checks, constraint validation, signature verification, etc.) is external to the core.

---

## 5. Evolution

Adding new fields or behaviors requires bumping `event_version` or introducing a new event type. Document every change so implementers can reconstruct the canonical bytes unambiguously.

Domain-specific event types can evolve independently of the core trust kernel.

---
