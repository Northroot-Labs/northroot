# northroot-canonical: Agent Constraints

## Scope

This crate provides **deterministic canonicalization primitives** for Northroot verifiable events and proof envelopes. It implements the canonical JSON profile (RFC 8785 + Northroot hygiene rules) and schema-aligned type definitions.

## Constraints

### Must
- Produce identical canonical bytes for identical semantic values across platforms and time
- Enforce RFC 8785 ordering (lexicographic by Unicode code point)
- Reject duplicate object keys
- Validate quantity minimality (no leading zeros, no `-0`, scale bounds)
- Validate identifier patterns per schema (`ProfileId`, `PrincipalId`, `ToolName`, `Timestamp`)
- Emit hygiene reports for all canonicalization operations
- Remain schema-aligned with `schemas/canonical/v1/types.schema.json`

### Must Not
- Perform irreversible actions or policy decisions
- Interpret or transform data semantics (canonicalization is structural only)
- Accept JSON numbers for quantity fields in strict mode
- Normalize or coerce numeric types
- Reorder arrays
- Drop unknown fields (unless schema explicitly permits)

### Out of Scope
- Event-specific types (authorization, execution, checkpoint, attestation)
- Policy evaluation or enforcement
- Storage or transport concerns
- Business logic or orchestration

## Invariants

- **INV-CANON-1**: Canonical bytes are deterministic and platform-independent
- **INV-CANON-2**: All validation errors are explicit and typed (`ValidationError`, `CanonicalizationError`)
- **INV-CANON-3**: Hygiene reports are always emitted (status may be `Ok`, `Lossy`, `Ambiguous`, or `Invalid`)
- **INV-CANON-4**: Quantity encoding is lossless by default (Dec/Int/Rat); F64 is opt-in and explicitly lossy

## Testing Requirements

- Golden tests must verify canonical byte stability
- Hash fixtures must remain stable across runs
- Validation must reject non-conforming inputs
- Generator example must produce CI-verifiable outputs
