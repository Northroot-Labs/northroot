# ADR-001: Receipts vs Engine Boundaries

**Date:** 2025-11-08 
**Status:** Accepted  
**Context:** Need to clearly define the boundary between receipt data model and proof algebra engine

## Context

The Northroot system has two core crates: `northroot-receipts` and `northroot-engine`. It's important to clearly define their responsibilities to avoid circular dependencies and maintain clean separation of concerns.

## Decision

**northroot-receipts** owns:
- Receipt envelope structure and types
- Payload types (DataShape, MethodShape, ReasoningShape, Execution, Spend, Settlement)
- Canonicalization (JCS) implementation
- Hash computation (SHA-256 with canonical JSON)
- JSON Schema validation
- Receipt deserialization/serialization

**northroot-engine** owns:
- Proof algebra operations (composition, validation logic)
- Commitment computation utilities
- Delta compute strategies
- Execution tracking and state management
- Operator implementations

**Dependency direction:**
- `northroot-engine` depends on `northroot-receipts`
- `northroot-receipts` does NOT depend on `northroot-engine`

## Consequences

### Pros
- Clear separation: data model vs. computation logic
- No circular dependencies
- Receipts crate can be used independently
- Engine can evolve without breaking receipts API

### Cons
- Some validation logic may need to be duplicated or moved
- Engine cannot influence receipt structure directly

## Alternatives

**Alternative 1:** Merge receipts and engine into one crate
- Rejected: Violates separation of concerns, makes receipts less reusable

**Alternative 2:** Engine owns receipt types
- Rejected: Receipts should be independent and publishable

## Migration

No migration needed - this is the initial structure. Future changes should respect these boundaries.

## References

- [ADR Playbook](docs/ADR_PLAYBOOK.md): Code placement guidance
- [Proof Algebra Spec](docs/specs/proof_algebra.md): Unified algebra specification

