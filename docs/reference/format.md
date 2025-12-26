Northroot Journal Format

Version: 1
Status: Stable (core)
Scope: On-disk representation of canonical events

---

## 1. Purpose

The Northroot Journal (.nrj) stores verifiable events in an append-only, tamper-evident stream. It is designed to be portable, streamable, forward-compatible, and suitable for offline verification and audit.

## 2. Principles

- Append-only: bytes are never rewritten.
- Framed records: no delimiter ambiguity.
- Explicit versioning: readers never guess.
- Canonical identity: `event_id` is derived from the event payload, not file bytes.
- Neutral storage: no policy, enforcement, or runtime semantics baked in.

## 3. Layout

1. File header (16 bytes):  
   - `magic` (4 bytes): ASCII `"NRJ1"`  
   - `version` (2 bytes): `0x0001`  
   - `flags` (2 bytes): reserved (must be 0)  
   - `reserved` (8 bytes): zero-filled

2. Sequence of record frames (no footer). Each frame contains:
   - Record header (8 bytes):  
     - `kind` (1 byte)  
     - `reserved` (3 bytes, must be 0)  
     - `len` (4 bytes, little-endian payload length)
   - Payload: `len` bytes

## 4. Record kinds

- `0x01` EventJson: UTF-8 JSON object representing a canonical Northroot event.
- All other values are reserved; readers must skip unknown kinds.

## 5. Event payload

EventJson payloads MUST:

1. Be valid UTF-8 JSON.
2. Be a single JSON object (flat, no `v` envelope).
3. Include required fields such as `event_id`, `event_type`, `event_version`, `occurred_at`, `principal_id`, `canonical_profile_id`, and any domain-specific properties.

The journal format is schema-agnostic - it stores any valid JSON event. Domain layers define event schemas.

Example:

```json
{
  "event_id": { "alg": "sha-256", "b64": "..." },
  "event_type": "test",
  "event_version": "1",
  "occurred_at": "2024-01-01T00:00:00Z",
  "principal_id": "service:example",
  "canonical_profile_id": "northroot-canonical-v1",
  "data": "example event payload"
}
```

### Verification note

Stored JSON bytes are not canonicalized. Verifiers must parse the object, canonicalize it according to the event’s `event_version`, and confirm:  
`event_id == H(domain_separator || canonical_json(event))`.  
This canonicalization covers the entire event object as defined by the schema (including the `signatures` array for attestation events).

## 6. Limits

- Maximum record payload: 16 MiB (recommended).
- Readers should reject records exceeding that size.

## 7. Resilience

- Writers should append records atomically when possible and never mutate existing bytes.
- Readers may operate in:
  - Strict mode: truncated headers/payloads are errors.
  - Permissive mode: truncation is treated as end-of-file.

## 8. Verification responsibilities

Readers must validate:

1. File header correctness.
2. Record framing (kind/reserved/len structure).
3. Valid UTF-8 JSON for every EventJson record.
4. Event identity (`event_id`) via canonicalization and hash computation.
5. Optional hash-chain references (`prev_event_id`).


## 9. What the format does NOT guarantee

- Policy correctness.  
- Completeness of evidence.  
- Trustworthiness of principals.  
- Absence of malicious behavior.

It guarantees:

- Immutability of recorded bytes.  
- Deterministic replay.  
- Verifiable identity of events.

## 10. Extensibility

Future versions may add new record kinds, compression, checksums, or alternative encodings. Such changes must use new kind values or bump the journal version while remaining backward-compatible (skip unknown kinds/versions).

---

## 11. Summary

The Northroot Journal is a durable evidence container for canonical events. Everything else—policy interpretation, enforcement, tooling—layers on top of the verified history.

