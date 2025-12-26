# Northroot Architecture

High-level system design and component relationships.

## Overview

Northroot is organized as a minimal trust kernel with two core crates:

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
- Quantity encoding (Dec, Int, Rat, F64)
- Identifier validation (PrincipalId, ProfileId, Timestamp, Digest)
- Event ID computation (`compute_event_id`)
- Hygiene reporting

**Key Types**:
- `Canonicalizer` - Produces canonical bytes
- `Digest` - Content-addressed identifiers
- `Quantity` - Lossless numeric types
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
- User-facing commands (`canonicalize`, `event-id`, `verify`, `list`)
- Output formatting
- Error reporting

**Dependencies**: `northroot-canonical`, `northroot-journal`

**Note**: This is a standalone application (not a workspace crate), built with path dependencies to the kernel crates.

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
2. Parse event JSON object (untyped)
3. northroot-canonical verifies event_id matches canonical bytes
4. Optional: domain-specific verification (external to core)
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

## Extension Points

- **Custom Event Schemas**: Applications define domain-specific event types
- **Custom Verification**: Domain layers add semantic verification on top of core event identity checks
- **Custom Storage**: Journal format is portable; applications can implement custom storage backends

See [Extensions](../reference/extensions.md) for details.

---

## Dependencies

- `northroot-canonical` - No dependencies on other Northroot crates
- `northroot-journal` - Depends on `northroot-canonical`
- `apps/northroot/` - Depends on `northroot-canonical`, `northroot-journal`

This dependency structure ensures:
- Lower-level crates remain independent
- Higher-level components compose functionality
- No circular dependencies
- Domain-specific concerns are external to the trust kernel

---

## Domain-Specific Layers

Domain-specific event types (authorization, execution, checkpoint, attestation, etc.) and verification logic are **not** part of the core trust kernel. They should be implemented as separate repositories or crates that consume the core primitives:

- `northroot-canonical` for canonicalization and event identity
- `northroot-journal` for storage

See `wip/` for examples:
- `wip/governance/` - Checkpoint and attestation schemas
- `wip/agent-domain/` - Authorization and execution schemas
- `wip/store/` - Storage abstraction layer

---

## Related Documentation

- [API Contract](api-contract.md) - Public API surface
- [Core Specification](../reference/spec.md) - Protocol details
- [Extensions](../reference/extensions.md) - How to extend the system
- [Core Invariants](../../CORE_INVARIANTS.md) - Non-negotiable kernel constraints
