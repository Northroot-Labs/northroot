# Schemas

This directory contains the normative JSON Schemas for the Northroot protocol.

The schemas are organized into **three layers**:

1) **Canonical types** (foundational building blocks)  
2) **Events** (protocol messages that are hashed, stored, and verified)  
3) **Profiles** (optional constraint overlays for specific deployments or domains)

The key rule:

> **Canonical types define the wire representation. Profiles only constrain; they do not change representation.**

---

## Directory layout

### `schemas/canonical/`
**Purpose:** Stable, cross-language building blocks used by all protocol messages.

Examples:
- deterministic numeric quantities (`dec`, `int`, `rat`, `f64`)
- digests / content references
- timestamps
- identifiers
- hygiene report types

**Audience:** Anyone implementing Northroot in any language.

**Stability:** Highest. Changes here are usually breaking and require a version bump.

Suggested layout:
- `schemas/canonical/v1/types.schema.json`

---

### `schemas/events/`
**Purpose:** The actual protocol messages that Northroot emits and verifies
(e.g., authorization events, execution events, checkpoint events, attestation events).

Event schemas `$ref` canonical types instead of redefining primitives.

Examples:
- `AuthorizationEvent`
- `ExecutionEvent`
- `CheckpointEvent`
- `AttestationEvent`

**Audience:** Integrators and application developers.

**Stability:** High. Changes are versioned and typically breaking when event
semantics change.

Current layout:
- `schemas/events/v1/authorization_event.schema.json`
- `schemas/events/v1/execution_event.schema.json`
- `schemas/events/v1/checkpoint_event_schema.json`
- `schemas/events/v1/attestation_event_schema.json`

---

### `schemas/profiles/`
**Purpose:** Optional, deployment- or domain-specific constraints that tighten the
canonical types and event shapes without changing wire encoding.

Profiles are used for stricter validation in specific contexts (finance, AI cost
gating, regulated environments). They often define field-specific aliases such as:

- `MoneyDec2` (scale fixed to 2)
- `MoneyDec6` (scale fixed to 6)
- `TokenBudgetInt` (bounded digits / range)
- `StrictNFCStrings` (if enabled in a future profile)

**Audience:** Operators and teams enforcing stricter rules.

**Stability:** Medium-to-high. Profiles may evolve faster than the protocol core,
but still must be versioned and pinned in events/policies if used.

**Note:** Profile schemas are optional and not currently implemented. They are
reserved for future use when domain-specific constraint overlays are needed.

Suggested layout (when implemented):
- `schemas/profiles/v1/ai_cost.schema.json`
- `schemas/profiles/v1/finance.schema.json`

---

## How schemas relate to the wire format

### Journal format
The Northroot journal format (`.nrj`) stores events as JSON objects that must conform
to the event schemas in `schemas/events/v1/`. The journal format itself is schema-agnostic:
it stores raw JSON bytes. Schema validation happens during verification, not during
journal I/O operations.

See [Journal Format](../docs/reference/format.md) for details on the on-disk representation.

### Canonical bytes and hashing
Northroot computes digests and identifiers from **canonical bytes**.

For JSON payloads, canonical bytes are produced by:

1. Parsing and validating per schema (strict mode)
2. Applying the canonicalization profile (RFC 8785 + Northroot constraints)
3. Serializing to UTF-8 bytes

Only these canonical bytes are used for hashing and verification. The journal format
stores the original JSON (not canonical bytes); canonicalization is applied during
verification to recompute `event_id` and validate integrity.

---

## Validation modes

### Strict mode (default for verification)
Used for:
- hashing
- events
- offline verification
- policy evaluation

Strict mode rejects:
- duplicate keys
- non-minimal integer encodings (leading zeros, `-0`)
- disallowed numeric representations (JSON numbers for quantities)
- values exceeding profile/schema bounds
- unknown fields unless explicitly permitted

### Permissive mode (ingestion only)
Permissive mode may:
- accept certain non-conforming inputs
- perform lossless coercions only when explicitly allowed
- emit `HygieneReport` warnings

Permissive mode MUST NOT be used to produce canonical bytes for hashing.

---

## Versioning

Schemas are versioned by directory (e.g., `v1`).

Any breaking change to:
- canonical encoding rules
- numeric encodings
- bounds / invariants
- event field semantics

requires a new schema version.

---

## Implementation notes

- Event schemas should reference canonical types via `$ref` rather than copying
  definitions.
- Profiles should use `allOf` overlays to constrain canonical types without
  changing representation.
- The CLI should provide:
  - `northroot validate <file> --schema <...>`
  - `northroot inspect <file>` (digests + hygiene report)

## Schema validation and journal format

The journal format stores events as JSON objects without schema validation during
write operations. This design allows:

- **Flexibility**: Journal I/O is fast and doesn't require schema parsing
- **Verification**: Schema validation happens during verification via `northroot-core`
- **Forward compatibility**: New schema versions can be added without breaking existing journals

When reading from a journal:
1. Parse the JSON object from the journal frame
2. Validate structure against the appropriate event schema (`schemas/events/v1/...`)
3. Canonicalize according to the event's `canonical_profile_id`
4. Verify `event_id` matches the computed digest

This separation ensures the journal format remains simple and schema-agnostic while
maintaining strong verification guarantees.

---

## Files to start with

- `canonical/v1/types.schema.json`  
  Canonical primitives (quantities, hygiene report, etc.)

Next:
- `events/v1/*.schema.json`  
  The protocol's hashed/verifiable message types

Optional (future):
- `profiles/v1/*.schema.json`  
  Deployment-specific constraint overlays (not currently implemented)