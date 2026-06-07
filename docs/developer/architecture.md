# Northroot Architecture

High-level system design and component relationships.

## Overview

Northroot is open governance and accountability infrastructure for verifiable
economic activity. Its trust kernel provides canonical identity, append-only
evidence journals, replay, and offline verification. Higher layers provide
projection, evaluation, authority, receipts, and financial/accountability
profiles without polluting the kernel.

The current stable architecture is the trust kernel component. This repository
is focused on making the core canonicalization and journal reference crates
solid before moving on to state/eval core.

The stable kernel is organized around two core crates and a standalone CLI:

```
┌─────────────────────────────────────────────────────────┐
│                   apps/northroot                        │
│              (CLI application)                          │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                          │
┌───────▼────────┐      ┌─────────▼──────────┐
│ northroot-     │      │  northroot-        │
│ journal        │      │  canonical        │
│ (Journal       │      │  (Canonicalization│
│  format)       │      │   & event_id)     │
└───────┬────────┘      └────────────────────┘
        │
        └───────────────┐
                        │
                  (depends on)
```

## Core Components

### `northroot-canonical`

**Purpose**: Deterministic canonicalization and event identity computation.

**Responsibilities**:
- Canonical JSON serialization (RFC 8785 + Northroot rules)
- Strict JSON parsing that rejects duplicate object keys before they collapse
- Quantity encoding (Dec, Int, Rat, F64)
- Identifier validation (PrincipalId, ProfileId, Timestamp, Digest)
- Event ID computation (`compute_event_id`)
- Hygiene reporting

**Key Types**:
- `Canonicalizer` - Produces canonical bytes
- `Digest` - Content-addressed identifiers
- `Quantity` - Lossless numeric types
- `parse_json_strict` - Parses untyped JSON while rejecting duplicate keys
- `compute_event_id` - Computes event identity from canonical bytes

**Dependencies**: None (foundational crate)

---

### `northroot-journal`

**Purpose**: Append-only journal file format (.nrj).

**Responsibilities**:
- Journal file format specification
- Frame encoding/decoding
- Reader/writer implementations
- Resilience handling (strict vs permissive modes)
- Event ID verification (using `northroot-canonical`)

**Key Types**:
- `JournalWriter` - Writes journal files
- `JournalReader` - Reads journal files
- `JournalHeader` - File header structure
- `RecordFrame` - Frame encoding
- `EventJson` - Alias for `serde_json::Value` (untyped events)

**Dependencies**: `northroot-canonical`

---

## Applications

### `apps/northroot/`

**Purpose**: Command-line interface for trust kernel operations.

**Responsibilities**:
- Public kernel commands (`canonicalize`, `event-id`, `append`, `read`,
  `verify`)
- Incubating profile and structural helpers kept outside the public kernel
  command set
- Output formatting
- Error reporting

**Dependencies**: `northroot-canonical`, `northroot-journal`

**Note**: This is a standalone application that uses path dependencies to the kernel crates.

---

## Incubating Components

### `northroot-state-eval`

**Purpose**: Product-agnostic projection and policy evaluation primitives over
projected state.

**Responsibilities**:
- Byte-stream-friendly event prefix and cursor metadata
- Projection identity wrappers
- Three-valued predicate composition
- Satisfaction and evaluation result shapes
- `EvaluationDelta` derivation
- Gate result shapes for callers that need pre-action or pre-append checks

**Boundary**: This crate is not part of the v0.1 stable kernel surface. It does
not own product policy authority, policy language dependencies, agent runtimes,
queues, database adapters, provider SDKs, or network integrations.

---

## Data Flow

### Event Recording

```
1. Application creates event (JSON object)
2. northroot-canonical computes event_id from canonical bytes
3. northroot-journal appends event to journal file
4. Journal writes frame to disk
```

### Event Verification

```
1. northroot-journal reads frame from disk
2. Strictly parse event JSON object (untyped, duplicate keys rejected)
3. Confirm event payload is an object with a digest-shaped event_id
4. northroot-canonical verifies event_id matches canonical bytes
5. Optional: domain-specific verification (external to core)
```

---

## Design Principles

1. **Separation of Concerns**: Each crate has a single, clear responsibility
2. **Domain-Agnostic Kernel**: Core provides primitives only; domain semantics are external
3. **Determinism**: All core operations are deterministic and offline-capable
4. **Neutrality**: Core does not execute actions or make decisions
5. **Verifiability**: All events can be verified offline using canonicalization and event identity
6. **Untyped Core**: Kernel operates on `EventJson = serde_json::Value`; domain layers add types

---

## Profile and Layering Points

- **Custom Event Schemas**: Applications define domain-specific event types
- **Custom Verification**: Domain layers add semantic verification on top of core event identity checks
- **Custom Storage**: Journal format is portable; applications can implement custom storage backends
- **Structural Segmentation**: large event streams can use ordered `.nrj`
  segments plus rebuildable manifests and checkpoints without adding projection
  meaning

See [Profiles and Consumer Protocols](../reference/profiles.md) and
[Segmented Journals](../reference/segmented-journals.md) for details.

---

## Dependencies

- `northroot-canonical` - No dependencies on other Northroot crates
- `northroot-journal` - Depends on `northroot-canonical`
- `northroot-state-eval` - Incubating, product-agnostic state/eval primitives
- `apps/northroot/` - Depends on `northroot-canonical`, `northroot-journal`

This dependency structure ensures:
- Lower-level crates remain independent
- Higher-level components compose functionality
- No circular dependencies
- Domain-specific concerns are external to the trust kernel

---

## Domain-Specific Layers

Domain-specific event types and higher-layer semantics (projection, evaluation,
authority, receipts, financial/accountability profiles, authorization,
execution, semantic checkpoints, attestation, etc.) are **not** part of the core
trust kernel. They should be implemented as separate repositories or crates that
consume the core primitives:

- `northroot-canonical` for canonicalization and event identity
- `northroot-journal` for storage

---

## Related Documentation

- [API Contract](api-contract.md) - Public API surface
- [Core Specification](../reference/spec.md) - Protocol details
- [Profiles and Consumer Protocols](../reference/profiles.md) - How to layer on the system
- [Segmented Journals](../reference/segmented-journals.md) - Structural segment and checkpoint contract
- [Core Invariants](../../CORE_INVARIANTS.md) - Non-negotiable kernel constraints
