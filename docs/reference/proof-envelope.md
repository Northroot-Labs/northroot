# Proof Envelope Boundary

Northroot core verifies generic proof envelopes. Receipt is a platform, profile,
or domain name for a proof envelope; it is not a separate core semantic.

## Proof Envelope

A proof envelope is a generic verifiable event or evidence envelope with:

- `event_id` derived from canonical bytes
- `occurred_at`
- `principal_id`
- `canonical_profile_id`
- optional evidence bindings such as inputs, outputs, content refs, or blob
  digests

The core trust kernel can canonicalize the envelope, recompute its identity,
store it in `.nrj`, and verify journal membership. It does not decide what the
envelope means in a business or operational domain.

## Receipt Profile

A receipt is a named use of a proof envelope in a platform, profile, layer, or
application context. The existing `schemas/platform/v1/receipt.schema.json`
schema remains a compatibility contract for receipt-profile proof envelopes.

Receipt-shaped bundle fields such as `receipts` remain valid compatibility
terminology. `northroot verify-bundle` verifies paths, hashes, event IDs, and
journal membership for those receipt-shaped evidence artifacts; it does not
define payment, settlement, work acceptance, policy approval, backup success, or
other domain semantics.

## Domain Semantics

Domain receipt semantics belong outside the neutral core. Examples include:

- payment settled
- backup succeeded
- work accepted
- policy approved
- execution completed

Those meanings must be supplied by profiles, consuming apps, or control-plane
contracts. The core boundary remains mechanical: canonical
bytes, event identity, append-only journal storage, and offline verification.
