# Event Model

**Note**: This document provides event structure and governance event definitions. For the protocol specification (invariants, identity computation, verification model), see [Core Specification](spec.md). For canonicalization rules, see [Canonicalization](canonicalization.md). For journal format, see [Journal Format](format.md).

## 0. Purpose

Northroot emits **Verifiable Events**: append-only, audit-grade evidence records
that are canonicalized, hash-identifiable, and optionally attested.

Northroot is intentionally **neutral**:
- It standardizes the **evidence format** and **verification rules**.
- It does NOT mandate any specific policy language, enforcement engine, agent framework,
  orchestration model, or runtime semantics.
- Domain-specific event types (authorization, execution, etc.) are defined by consuming applications.

The trust kernel provides canonicalization and event identity primitives. Domain layers add semantic verification.

---

## 1. Definitions

### 1.1 Event
An immutable record in an append-only log.

### 1.2 Verifiable Event
An event with:
- canonical JSON payload bytes (per canonicalization profile)
- `event_id` derived from a digest of canonical bytes
- explicit linkage to prior events (optional)
- schema-defined structure

### 1.3 Principal
The responsible actor for an action. Can be a human, service, agent, or org.
Northroot models all of these uniformly as `PrincipalID`.

---

## 2. Core invariants

### E1 — Append-only
Events are immutable once written. Corrections are represented as new events.

### E2 — Stable identity
Each event has:
- `event_id = H(domain_separator || canonical_bytes(event))`

Where `H` is SHA-256 and the domain separator is `b"northroot:event:v1\0"`.

### E3 — Canonicalization binding
Each event binds:
- `canonical_profile_id` (ProfileID)
- All fields that affect verification are part of the canonical payload

Verification MUST recompute digests from canonicalization rules.

### E4 — Offline verifiability
A verifier must be able to validate:
- canonicalization + digests
- event link consistency (if present)
without network calls.

---

## 3. Event envelope

All events share a common envelope:

- `event_type`: string (e.g., "checkpoint", "attestation", or domain-specific types)
- `event_version`: string (e.g., "1")
- `event_id`: Digest
- `occurred_at`: Timestamp
- `principal_id`: PrincipalID
- `canonical_profile_id`: ProfileID
- Optional `prev_event_id`: Digest (for hash-chain ordering)

Operational metadata (request ids, traces, retries, tags, provider hints, etc.) is explicitly
out-of-band. Core schemas carry verifiable evidence only; deployments may attach ops metadata
in transport-specific envelopes that are excluded from canonicalization and hashing.

---

## 4. Governance event types

Northroot 1.0 defines two governance event types that are part of the trust kernel:

### 4.1 CheckpointEvent

A checkpoint event attests to a chain tip, providing continuity proofs.

**Required fields**:
- `chain_tip_event_id`: Digest (the latest event_id this checkpoint attests exists)
- `chain_tip_height`: u64 (monotonic counter for the chain tip)

**Optional fields**:
- `merkle_root`: Digest (optional Merkle root over a window of events)
- `window`: Object with `start_height` and `end_height` (required if `merkle_root` is present)

**Purpose**: Enables efficient partial verification and detects deletion/reordering.

**Schema location**: `crates/northroot-schemas/schemas/checkpoint.schema.json`

---

### 4.2 AttestationEvent

An attestation event attests to a checkpoint's `event_id` with cryptographic signatures.

**Required fields**:
- `checkpoint_event_id`: Digest (the checkpoint being attested)
- `signatures`: Array of 1–16 signature objects

**Signature object**:
- `alg`: string (e.g., "ed25519")
- `key_id`: string (identifier for the signing key)
- `sig`: string (base64url-no-pad encoded signature)

**Purpose**: Allows multiple trust anchors to co-sign the same checkpoint without altering canonical identity.

**Schema location**: `crates/northroot-schemas/schemas/attestation.schema.json`

---

## 5. Domain-specific events

Domain-specific event types (authorization, execution, reconciliation, etc.) are **not** part of the core trust kernel. Applications should:

1. Define their own event schemas following the canonicalization rules
2. Use `northroot-canonical` for canonicalization and event identity computation
3. Use `northroot-journal` for storage
4. Implement domain-specific verification logic externally

**Example**: An authorization event might include:
- `policy_id`, `policy_digest`
- `decision` ("allow" | "deny")
- `intent_digest`
- Domain-specific bounds and constraints

But these fields are application-defined, not core primitives.

---

## 6. Event identity computation

Event identity is computed as:

```rust
event_id = sha256(b"northroot:event:v1\0" || canonical_json(event))
```

Where:
- The domain separator is fixed: `b"northroot:event:v1\0"`
- `canonical_json(event)` is the RFC 8785 canonical form of the entire event object
- The `event_id` field itself is excluded from the hash (to avoid self-reference)
- The result is base64url-no-pad encoded

See `northroot-canonical::compute_event_id` for the reference implementation.

---

## 7. Verification

Verification of an event involves:

1. Parse the JSON object from the journal
2. Extract the `event_id` field
3. Remove `event_id` from the object (temporarily)
4. Canonicalize the remaining object according to `canonical_profile_id`
5. Compute `event_id` using the domain separator and canonical bytes
6. Compare computed `event_id` to stored `event_id`

If they match, the event is valid. Domain-specific verification (policy checks, constraint validation, etc.) is external to this core process.

---

## 8. Summary

The Northroot event model provides:
- A common envelope structure
- Deterministic event identity
- Governance events (checkpoint, attestation)
- Primitives for domain-specific event types

Everything else—policy interpretation, enforcement, domain semantics—layers on top of the verified history.
