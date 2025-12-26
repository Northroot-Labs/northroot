# Northroot Architecture

High-level system design and component relationships.

## Overview

Northroot 1.0 is organized into focused crates, each with a clear responsibility:

```
┌─────────────────────────────────────────────────────────┐
│                   northroot-cli                          │
│              (Command-line interface)                    │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                          │
┌───────▼────────┐      ┌─────────▼──────────┐
│ northroot-     │      │  northroot-        │
│ journal        │      │  schemas           │
│ (Journal       │      │  (Governance       │
│  format)       │      │   events)         │
└───────┬────────┘      └─────────┬─────────┘
        │                          │
        └────────────┬─────────────┘
                     │
              ┌──────▼────────┐
              │ northroot-    │
              │ canonical     │
              │ (Canonicalization│
              │  & event_id)  │
              └───────────────┘
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

**Dependencies**: `northroot-canonical`

---

### `northroot-schemas`

**Purpose**: Domain-agnostic governance event schemas.

**Responsibilities**:
- Checkpoint event schema and types
- Attestation event schema and types
- Signature types
- JSON Schema definitions

**Key Types**:
- `CheckpointEvent` - Chain checkpoint event
- `AttestationEvent` - Checkpoint attestation event
- `Signature` - Cryptographic signature

**Dependencies**: `northroot-canonical`

---

### `northroot-cli`

**Purpose**: Command-line interface for users.

**Responsibilities**:
- User-facing commands (`list`, `verify`, `canonicalize`, `checkpoint`)
- Output formatting
- Error reporting
- Journal manipulation

**Dependencies**: `northroot-canonical`, `northroot-journal`, `northroot-schemas`

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
2. Parse event JSON object
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

---

## Extension Points

- **Custom Event Schemas**: Applications define domain-specific event types
- **Custom Verification**: Domain layers add semantic verification on top of core event identity checks
- **Custom Storage**: Journal format is portable; applications can implement custom storage backends

See [Extending Northroot](extending.md) for details.

---

## Dependencies

- `northroot-canonical` - No dependencies on other Northroot crates
- `northroot-journal` - Depends on `northroot-canonical`
- `northroot-schemas` - Depends on `northroot-canonical`
- `northroot-cli` - Depends on `northroot-canonical`, `northroot-journal`, and `northroot-schemas`

This dependency structure ensures:
- Lower-level crates remain independent
- Higher-level crates compose functionality
- No circular dependencies
- Domain-specific concerns are external to the trust kernel

---

## Domain-Specific Layers

Domain-specific event types (authorization, execution, etc.) and verification logic are **not** part of the core trust kernel. They should be implemented as separate crates or application code that consume the core primitives:

- `northroot-canonical` for canonicalization and event identity
- `northroot-journal` for storage
- `northroot-schemas` for governance events (optional)

See `wip/agent-domain/` for an example of domain-specific code that was moved out of core.

---

## Related Documentation

- [API Contract](api-contract.md) - Public API surface
- [Core Specification](../reference/spec.md) - Protocol details
- [Extending Northroot](extending.md) - How to extend the system
- [Core Invariants](../../CORE_INVARIANTS.md) - Non-negotiable kernel constraints
