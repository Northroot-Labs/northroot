# Northroot Architecture

High-level system design and component relationships.

## Overview

Northroot is organized into focused crates, each with a clear responsibility:

```
┌─────────────────────────────────────────────────────────┐
│                   northroot-cli                          │
│              (Command-line interface)                    │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│                  northroot-store                         │
│         (Storage abstraction & filtering)                │
└────┬───────────────────────────────┬────────────────────┘
     │                               │
┌────▼──────────┐          ┌─────────▼──────────┐
│ northroot-    │          │  northroot-core    │
│ journal       │          │  (Event types &    │
│ (Journal      │          │   verification)    │
│  format)      │          └─────────┬──────────┘
└───────────────┘                    │
                              ┌───────▼────────┐
                              │ northroot-     │
                              │ canonical      │
                              │ (Canonicalization│
                              │  & digests)    │
                              └────────────────┘
```

## Core Components

### `northroot-canonical`

**Purpose**: Deterministic canonicalization and digest computation.

**Responsibilities**:
- Canonical JSON serialization (RFC 8785 + Northroot rules)
- Quantity encoding (Dec, Int, Rat, F64)
- Identifier validation
- Hygiene reporting

**Key Types**:
- `Canonicalizer` - Produces canonical bytes
- `Digest` - Content-addressed identifiers
- `Quantity` - Lossless numeric types

### `northroot-core`

**Purpose**: Event types and verification logic.

**Responsibilities**:
- Event type definitions (Authorization, Execution, Checkpoint, Attestation)
- Event ID computation
- Verification logic
- Linkage validation

**Key Types**:
- `AuthorizationEvent`, `ExecutionEvent`, `CheckpointEvent`, `AttestationEvent`
- `Verifier` - Verifies event integrity and constraints

### `northroot-journal`

**Purpose**: Append-only journal file format.

**Responsibilities**:
- Journal file format (`.nrj`)
- Frame encoding/decoding
- Resilience handling (strict vs permissive modes)

**Key Types**:
- `JournalWriter` - Writes journal files
- `JournalReader` - Reads journal files

### `northroot-store`

**Purpose**: Storage abstraction and event filtering.

**Responsibilities**:
- Trait-based storage interface (`StoreWriter`, `StoreReader`)
- Event filtering (`EventFilter`, `FilteredReader`)
- Typed event parsing
- Linkage navigation

**Key Types**:
- `StoreWriter`, `StoreReader` - Storage traits
- `EventFilter` - Filtering interface
- `JournalBackendWriter`, `JournalBackendReader` - Journal implementation

### `northroot-cli`

**Purpose**: Command-line interface for users.

**Responsibilities**:
- User-facing commands (`list`, `get`, `verify`, etc.)
- Output formatting
- Error reporting

## Data Flow

### Event Recording

```
1. Application creates event (JSON object)
2. northroot-core computes event_id from canonical bytes
3. northroot-store appends to journal via northroot-journal
4. Journal writes frame to disk
```

### Event Verification

```
1. northroot-journal reads frame from disk
2. northroot-store parses event JSON
3. northroot-core verifies event_id matches canonical bytes
4. Verifier checks constraints and linkages
```

## Design Principles

1. **Separation of Concerns**: Each crate has a single, clear responsibility
2. **Trait-Based Abstractions**: Storage and filtering are pluggable
3. **Determinism**: All core operations are deterministic and offline-capable
4. **Neutrality**: Core does not execute actions or make decisions
5. **Verifiability**: All events can be verified offline

## Extension Points

- **Custom Storage**: Implement `StoreWriter` and `StoreReader`
- **Custom Filters**: Implement `EventFilter`
- **Custom Verification**: Wrap `Verifier` with additional checks

See [Extending Northroot](extending.md) for details.

## Dependencies

- `northroot-canonical` - No dependencies on other Northroot crates
- `northroot-core` - Depends on `northroot-canonical`
- `northroot-journal` - Depends on `northroot-canonical` and `northroot-core`
- `northroot-store` - Depends on `northroot-core`, `northroot-journal`, and `northroot-canonical`
- `northroot-cli` - Depends on `northroot-store`, `northroot-core`, and `northroot-canonical`

This dependency structure ensures:
- Lower-level crates remain independent
- Higher-level crates compose functionality
- No circular dependencies

## Related Documentation

- [API Contract](api-contract.md) - Public API surface
- [Core Specification](../reference/spec.md) - Protocol details
- [Extending Northroot](extending.md) - How to extend the system

