# Schemas

This directory contains the normative JSON Schemas for the Northroot protocol.

The schemas are organized into **three layers**:

1) **Canonical types** (foundational building blocks)  
2) **Receipts** (protocol messages that are hashed, stored, and verified)  
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

### `schemas/receipts/`
**Purpose:** The actual protocol messages that Northroot emits and verifies
(e.g., authorization receipts, execution receipts, verification receipts).

Receipt schemas `$ref` canonical types instead of redefining primitives.

Examples:
- `AuthorizationReceipt`
- `ExecutionReceipt`
- `VerificationReceipt`

**Audience:** Integrators and application developers.

**Stability:** High. Changes are versioned and typically breaking when receipt
semantics change.

Suggested layout:
- `schemas/receipts/v1/authorization_receipt.schema.json`
- `schemas/receipts/v1/execution_receipt.schema.json`

---

### `schemas/profiles/`
**Purpose:** Optional, deployment- or domain-specific constraints that tighten the
canonical types and receipt shapes without changing wire encoding.

Profiles are used for stricter validation in specific contexts (finance, AI cost
gating, regulated environments). They often define field-specific aliases such as:

- `MoneyDec2` (scale fixed to 2)
- `MoneyDec6` (scale fixed to 6)
- `TokenBudgetInt` (bounded digits / range)
- `StrictNFCStrings` (if enabled in a future profile)

**Audience:** Operators and teams enforcing stricter rules.

**Stability:** Medium-to-high. Profiles may evolve faster than the protocol core,
but still must be versioned and pinned in receipts/policies if used.

Suggested layout:
- `schemas/profiles/v1/ai_cost.schema.json`
- `schemas/profiles/v1/finance.schema.json`

---

## How schemas relate to the wire format

### Canonical bytes and hashing
Northroot computes digests and identifiers from **canonical bytes**.

For JSON payloads, canonical bytes are produced by:

1. Parsing and validating per schema (strict mode)
2. Applying the canonicalization profile (RFC 8785 + Northroot constraints)
3. Serializing to UTF-8 bytes

Only these canonical bytes are used for hashing and verification.

---

## Validation modes

### Strict mode (default for verification)
Used for:
- hashing
- receipts
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
- receipt field semantics

requires a new schema version.

---

## Implementation notes

- Receipt schemas should reference canonical types via `$ref` rather than copying
  definitions.
- Profiles should use `allOf` overlays to constrain canonical types without
  changing representation.
- The CLI should provide:
  - `northroot validate <file> --schema <...>`
  - `northroot inspect <file>` (digests + hygiene report)

---

## Files to start with

- `canonical/v1/types.schema.json`  
  Canonical primitives (quantities, hygiene report, etc.)

Next:
- `receipts/v1/*.schema.json`  
  The protocol’s hashed/verifiable message types

Optional:
- `profiles/v1/*.schema.json`  
  Deployment-specific constraint overlays