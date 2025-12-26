# Northroot API Contract

Version: 1
Status: Draft
Scope: Core API surface for storage, verification, and replay

---

## 1. Purpose

This document defines the public API contract for Northroot core crates. It specifies the interfaces that higher-level consumers (CLI, services, integrations) depend on for storing, reading, verifying, and filtering canonical events.

The goal is a minimal, composable API that enables:
- **Depend**: Straightforward integration via trait-based storage
- **Verify**: Deterministic, offline verification of event integrity and linkages
- **Replay**: Sequential or filtered iteration over stored events
- **View**: Typed access to event payloads

---

## 2. Crate Responsibilities

| Crate | Responsibility |
|-------|----------------|
| `northroot-canonical` | Canonicalization, digests, quantities, identifiers |
| `northroot-core` | Event types, event ID computation, verification logic |
| `northroot-journal` | Append-only journal format (reference storage backend) |
| `northroot-store` | Pluggable storage abstraction over backends |

---

## 3. Storage API (`northroot-store`)

### 3.1 Traits

```rust
/// Trait for appending events to a store.
pub trait StoreWriter {
    fn append(&mut self, event: &EventJson) -> Result<(), StoreError>;
    fn flush(&mut self) -> Result<(), StoreError>;
    fn finish(self) -> Result<(), StoreError>;
}

/// Trait for reading events from a store.
pub trait StoreReader {
    fn read_next(&mut self) -> Result<Option<EventJson>, StoreError>;
}
```

### 3.2 Types

- `EventJson`: Alias for `serde_json::Value` representing a canonical event object.
- `StoreError`: Enum covering I/O, backend, and validation errors.

### 3.3 Default Backend

The journal backend (`JournalBackendWriter`, `JournalBackendReader`) implements these traits using the append-only journal format.

### 3.4 Invariants

- Append-only: No updates or deletes.
- 16 MiB payload limit per event.
- Sync I/O (async deferred to v2).

---

## 4. Verification API (`northroot-core`)

### 4.1 Verifier

```rust
pub struct Verifier {
    canonicalizer: Canonicalizer,
}

impl Verifier {
    pub fn new(canonicalizer: Canonicalizer) -> Self;

    pub fn verify_authorization(
        &self,
        event: &AuthorizationEvent,
    ) -> Result<(Digest, VerificationVerdict), String>;

    pub fn verify_checkpoint(
        &self,
        event: &CheckpointEvent,
    ) -> Result<(Digest, VerificationVerdict), String>;

    pub fn verify_attestation(
        &self,
        event: &AttestationEvent,
    ) -> Result<(Digest, VerificationVerdict), String>;

    pub fn verify_execution(
        &self,
        exec: &ExecutionEvent,
        auth: &AuthorizationEvent,
        conversion: Option<&ConversionContext>,
    ) -> Result<(Digest, VerificationVerdict), String>;
}
```

### 4.2 Verdicts

```rust
pub enum VerificationVerdict {
    Ok,        // All constraints satisfied
    Denied,    // Authorization was denied
    Violation, // Constraint exceeded (e.g., meter cap)
    Invalid,   // Missing or malformed evidence
}
```

### 4.3 Invariants

- Verification is deterministic and offline.
- `event_id` must match `H(domain_separator || canonical_bytes(event))`.
- Execution must link to a valid authorization.
- Missing evidence → `Invalid`.

---

## 5. Replay API

Replay is built by combining `StoreReader` with `Verifier`. The pattern:

```rust
fn replay_and_verify<R: StoreReader>(
    reader: &mut R,
    verifier: &Verifier,
) -> Result<Vec<(EventJson, VerificationVerdict)>, StoreError> {
    let mut results = Vec::new();
    while let Some(event) = reader.read_next()? {
        let verdict = dispatch_verify(&event, verifier)?;
        results.push((event, verdict));
    }
    Ok(results)
}
```

Where `dispatch_verify` parses the event type and calls the appropriate verifier method.

### 5.1 Typed Event Parsing

```rust
pub fn parse_event(json: &EventJson) -> Result<TypedEvent, ParseError>;

pub enum TypedEvent {
    Authorization(AuthorizationEvent),
    Execution(ExecutionEvent),
    Checkpoint(CheckpointEvent),
    Attestation(AttestationEvent),
    Unknown(EventJson),
}
```

---

## 6. Filter API

Filtering is a higher-level composition over `StoreReader`. Filters do not require changes to the storage layer.

### 6.1 Filter Trait

```rust
pub trait EventFilter {
    fn matches(&self, event: &EventJson) -> bool;
}
```

### 6.2 Built-in Filters

```rust
/// Filter by event type.
pub struct EventTypeFilter {
    pub event_type: String,
}

/// Filter by principal ID.
pub struct PrincipalFilter {
    pub principal_id: String,
}

/// Filter by time range.
pub struct TimeRangeFilter {
    pub after: Option<Timestamp>,
    pub before: Option<Timestamp>,
}

/// Filter by event ID (exact match).
pub struct EventIdFilter {
    pub event_id: Digest,
}

/// Composite filter (all must match).
pub struct AndFilter {
    pub filters: Vec<Box<dyn EventFilter>>,
}

/// Composite filter (any must match).
pub struct OrFilter {
    pub filters: Vec<Box<dyn EventFilter>>,
}
```

### 6.3 Filtered Reader

```rust
pub struct FilteredReader<R: StoreReader, F: EventFilter> {
    reader: R,
    filter: F,
}

impl<R: StoreReader, F: EventFilter> StoreReader for FilteredReader<R, F> {
    fn read_next(&mut self) -> Result<Option<EventJson>, StoreError> {
        loop {
            match self.reader.read_next()? {
                None => return Ok(None),
                Some(event) if self.filter.matches(&event) => return Ok(Some(event)),
                Some(_) => continue, // skip non-matching
            }
        }
    }
}
```

---

## 7. View API

View provides typed, read-only access to parsed events.

### 7.1 Event Accessors

Events use public fields for direct access:

```rust
// AuthorizationEvent fields (public)
event.event_id: Digest
event.event_type: String
event.occurred_at: Timestamp
event.principal_id: PrincipalId
event.decision: Decision
event.policy_id: String
event.policy_digest: Digest
event.authorization: AuthorizationKind
// ... other fields (see northroot-core::events::AuthorizationEvent)

// ExecutionEvent fields (public)
event.event_id: Digest
event.auth_event_id: Digest
event.outcome: Outcome
event.tool_name: ToolName
event.meter_used: Vec<Meter>
// ... other fields (see northroot-core::events::ExecutionEvent)
```

### 7.2 Linkage Navigation

```rust
/// Resolve authorization event by ID from a reader.
pub fn resolve_auth<R: StoreReader>(
    reader: &mut R,
    auth_event_id: &Digest,
) -> Result<Option<AuthorizationEvent>, StoreError>;

/// Collect all execution events linked to an authorization.
pub fn executions_for_auth<R: StoreReader>(
    reader: &mut R,
    auth_event_id: &Digest,
) -> Result<Vec<ExecutionEvent>, StoreError>;
```

---

## 8. Error Handling

### 8.1 StoreError

```rust
pub enum StoreError {
    Io(std::io::Error),
    Journal(JournalError),
    PayloadTooLarge,
    Parse(ParseError),
    Other(String),
}
```

### 8.2 Verification Errors

Verification methods return `Result<(Digest, VerificationVerdict), String>`. The `String` represents structural or computation errors; the `VerificationVerdict` represents semantic outcomes.

---

## 9. Composition Patterns

### 9.1 Full Replay with Verification

```rust
let mut reader = JournalBackendReader::open(path, ReadMode::Strict)?;
let verifier = Verifier::new(canonicalizer);

while let Some(event) = reader.read_next()? {
    let typed = parse_event(&event)?;
    let verdict = match typed {
        TypedEvent::Authorization(e) => verifier.verify_authorization(&e)?.1,
        TypedEvent::Execution(e) => {
            let auth = resolve_auth(&mut reader, &e.auth_event_id)?
                .ok_or("missing authorization")?;
            verifier.verify_execution(&e, &auth, None)?.1
        }
        // ...
    };
    println!("{:?} -> {:?}", event["event_id"], verdict);
}
```

### 9.2 Filtered Scan

```rust
let reader = JournalBackendReader::open(path, ReadMode::Strict)?;
let filter = EventTypeFilter { event_type: "execution".into() };
let mut filtered = FilteredReader::new(reader, filter);

while let Some(event) = filtered.read_next()? {
    // Only execution events
}
```

---

## 10. What This API Does NOT Provide

- **CLI bindings**: This is library-level; CLI is a separate layer.
- **Async I/O**: Deferred to v2.
- **Indexing**: Sequential scan only; indexing is a layer above.
- **Distributed consensus**: Out of scope for core.
- **Policy evaluation**: Core verifies evidence, not policy semantics.

---

## 11. Versioning

- API changes that break existing consumers require a major version bump.
- New optional fields or additive changes are minor version bumps.
- Verification logic changes that affect verdicts for existing events are breaking.

---

## 12. Extension Points

| Extension | How |
|-----------|-----|
| Custom backend | Implement `StoreWriter` + `StoreReader` |
| Custom filter | Implement `EventFilter` |
| Custom verification | Wrap `Verifier` with additional checks |
| Async support | Trait variants `AsyncStoreWriter`, `AsyncStoreReader` (v2) |

---

## 13. Summary

The Northroot API provides:

1. **Storage**: Trait-based append/read with journal default
2. **Verification**: Deterministic, offline event verification
3. **Replay**: Sequential iteration with optional filtering
4. **View**: Typed event access and linkage navigation

All operations are composable, sync, and offline-capable.


