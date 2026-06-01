Northroot Journal Format

Version: 1
Status: Stable (core)
Scope: On-disk representation of canonical events

---

## 1. Purpose

The Northroot Journal (.nrj) stores verifiable events in an append-only,
tamper-evident stream. It is designed to be portable, streamable,
forward-compatible, and suitable for offline verification and audit. It is the
canonical event journal format, not a workspace, exchange, backup, or artifact
bundle format. JSONL can be used for adapters, and tar/zip-style bundles can be
used for portable exports that contain journals, manifests, schemas, receipts,
and artifacts.

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

### 3.1 Hex walkthrough

An `.nrj` file with one EventJson frame starts with:

| Offset | Bytes | Meaning |
| --- | --- | --- |
| `0x00..0x03` | `4e 52 4a 31` | ASCII `NRJ1` magic |
| `0x04..0x05` | `01 00` | version `0x0001`, little-endian |
| `0x06..0x07` | `00 00` | flags, must be zero |
| `0x08..0x0f` | `00 00 00 00 00 00 00 00` | reserved header bytes |
| `0x10` | `01` | frame kind: EventJson |
| `0x11..0x13` | `00 00 00` | reserved frame bytes |
| `0x14..0x17` | `<len u32le>` | payload length |
| `0x18..` | JSON bytes | UTF-8 JSON object |

## 3.2 Minimal portable contract (v1)

The following fields are normative for portable verification:

- Header magic MUST equal `NRJ1`.
- Header version MUST equal `0x0001`.
- Header flags MUST equal `0`.
- Frame reserved bytes MUST equal `0`.
- `kind=0x01` payload MUST be UTF-8 JSON object.
- Unknown `kind` values MUST be skipped, not interpreted.

This contract is intentionally minimal so verifiers in Rust, Python, and Go can implement
identical framing behavior without coupling to orchestration/runtime semantics.

## 4. Record kinds

- `0x01` EventJson: UTF-8 JSON object representing a canonical Northroot event.
- All other values are reserved; readers must skip unknown kinds.

## 5. Event payload

EventJson payloads MUST:

1. Be valid UTF-8 JSON.
2. Reject duplicate object keys before parsing collapses them.
3. Be a single JSON object (flat, no `v` envelope).
4. Include a digest-shaped `event_id`.

The journal format is schema-agnostic - it stores untyped JSON event objects.
Domain layers define fields such as `event_type`, `event_version`,
`occurred_at`, `principal_id`, `canonical_profile_id`, and any domain-specific
properties. The core does not validate policy meaning, workflow state,
authorization, or domain payload semantics.

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

Stored JSON bytes are not canonicalized. Verifiers must strictly parse the
object, reject duplicate object keys at any depth, canonicalize it according to
the event's `event_version`, and confirm:
`event_id == H(domain_separator || canonical_json(event))`.  
This canonicalization covers the entire untyped event object. Domain schemas may
add further checks outside the journal core.

## 6. Limits

- Maximum record payload: 16 MiB (recommended).
- Readers should reject records exceeding that size.

## 7. Resilience

- Writers should append records atomically when possible and never mutate existing bytes.
- Readers may operate in:
  - Strict mode: truncated headers/payloads are errors.
  - Permissive mode: truncation is treated as end-of-file.
- v0.1 assumes single-writer / many-reader operation. Concurrent write
  coordination, leases, and multi-event transactions belong above the journal
  kernel or in a database adapter.

## 7.1 Segmentation and checkpoints

Large journals may be split into ordered `.nrj` segment files. Each segment is a
normal journal file and must remain independently readable and verifiable.
Segment manifests and checkpoints are structural metadata over ordered segments;
they are rebuildable and do not encode projection or policy meaning.
Segment verification reports include a verified prefix count so readers can
recover the structurally valid prefix without treating later corrupt or
profile-invalid bytes as accepted state.

See [Segmented Journals and Structural Checkpoints](segmented-journals.md).

## 8. Verification responsibilities

Readers must validate:

1. File header correctness.
2. Record framing (kind/reserved/len structure).
3. Valid UTF-8 JSON for every EventJson record.
4. Event identity (`event_id`) via canonicalization and hash computation.
5. Optional hash-chain references (`prev_event_id`).

Verification semantics are fail-closed:
- malformed header/frame/payload => Invalid
- missing required event identity fields => Invalid
- hash mismatch after canonicalization => Invalid


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
