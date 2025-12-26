Northroot Core Specification

Version: 0.2
Status: Stable core (additive changes only)
Scope: Verifiable events, canonical identities, journal storage

**Note**: This document defines the protocol specification (invariants, identity computation, verification model). For detailed event type definitions and Rust API surface, see [Event Model](events.md). For canonicalization rules, see [Canonicalization](canonicalization.md). For journal format, see [Journal Format](format.md).

---

## 1. Purpose

Northroot defines a minimal, neutral surface for recording side-effectful actions as verifiable events. The specification exists to deliver deterministic identity, append-only ordering, offline verification, and a foundation for enforcement without prescribing policy engines, runtimes, or storage backends.

---

## 2. Core invariants

### 2.1 Canonical identity

Every event is the canonical JSON object produced by its schema (`schemas/events/v1`). Identity is computed as:  
`event_id = H(domain_separator || canonical_json(event))`  
`canonical_json` follows the Northroot canonicalization profile, and `event` is the entire schema-defined object (type, version, principals, payload fields, signatures, etc.).

### 2.2 Determinism and versioning

`event_version` captures schema evolution so readers know which canonicalization rules to apply. There is no separate `v` envelope: the event object itself is canonical, and every field listed in the schema contributes to the hash.

### 2.3 Append-only

Writers append events to a journal without mutation or deletion. Optional `prev_event_id` values may be present for hash-chain ordering but are not required for the core model.

### 2.4 Metadata isolation

Operational metadata (request IDs, traces, retries, provider hints, tags, transport headers, etc.) lives outside the canonical event so it cannot affect hashes or policy decisions.

---

## 3. Event structure

Each core event type exposes:

- `event_id`, `event_type`, `event_version`
- `occurred_at`, `principal_id`, `canonical_profile_id`
- Optional `prev_event_id`
- Type-specific payload fields (decision/authorization bounds, meters, checkpoint metadata, signatures, etc.)

Every field in the schema is part of the canonical payload; the schema enumerates required and optional properties so verifiers know what to expect.

### 3.1 Attestation events

Attestation events attest to a checkpoint’s `event_id`. The schema now exposes a `signatures` array (1–16 entries), each including `alg`, `key_id`, and `sig`, so multiple trust anchors can co-sign a checkpoint. Verifiers replay the canonical event to ensure every signature covers the same digest.

---

## 4. Verification model

Verifiers must:

1. Parse the journal record into the canonical JSON event object.
2. Apply the canonicalization profile associated with `event_version`.
3. Recompute `event_id` from the canonical bytes and ensure it matches the stored digest.
4. Validate any referenced digests (`policy_digest`, `intent_digest`, `auth_event_id`, etc.).
5. For attestations, verify each entry in `signatures`.

Optional: use `prev_event_id` for hash-chain checks or combine checkpoints with attestations for additional trust.

---

## 5. Evolution

Adding new fields or behaviors requires bumping `event_version` or introducing a new event type. Document every change so implementers can reconstruct the canonical bytes unambiguously.

