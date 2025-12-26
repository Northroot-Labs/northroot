# Northroot API Contract

Version: 1.0
Status: Stable
Scope: Trust kernel API surface

---

## 1. Purpose

This document defines the public API contract for Northroot trust kernel crates. It specifies the interfaces that applications and extensions depend on for canonicalization, event identity computation, and journal operations.

The kernel provides:
- **Canonicalization**: Deterministic JSON canonicalization (RFC 8785 + Northroot rules)
- **Event Identity**: Content-derived event identifiers
- **Journal Format**: Portable, append-only event container

---

## 2. Crate Responsibilities

| Crate | Responsibility |
|-------|----------------|
| `northroot-canonical` | Canonicalization, digests, quantities, identifiers, event ID computation |
| `northroot-journal` | Append-only journal format (.nrj) |

---

## 3. Canonicalization API (`northroot-canonical`)

### 3.1 Canonicalizer

```rust
pub struct Canonicalizer {
    // ...
}

impl Canonicalizer {
    pub fn new(profile: ProfileId) -> Self;
    
    pub fn canonicalize(&self, value: &Value) -> Result<CanonicalResult, CanonicalizationError>;
}
```

### 3.2 Event ID Computation

```rust
pub fn compute_event_id<T: Serialize>(
    event: &T,
    canonicalizer: &Canonicalizer,
) -> Result<Digest, EventIdError>;

pub fn verify_event_id(
    event: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<bool, EventIdError>;
```

### 3.3 Primitive Types

- `Digest` - Content-addressed identifiers (alg + b64)
- `Quantity` - Lossless numeric types (Dec, Int, Rat, F64)
- `Timestamp` - RFC 3339 timestamps
- `PrincipalId` - Actor identifiers
- `ProfileId` - Canonicalization profile identifiers

---

## 4. Journal API (`northroot-journal`)

### 4.1 Writer

```rust
pub struct JournalWriter {
    // ...
}

impl JournalWriter {
    pub fn open<P: AsRef<Path>>(path: P, options: WriteOptions) -> Result<Self, JournalError>;
    
    pub fn append_event(&mut self, event: &EventJson) -> Result<(), JournalError>;
    
    pub fn finish(mut self) -> Result<(), JournalError>;
}
```

### 4.2 Reader

```rust
pub struct JournalReader {
    // ...
}

impl JournalReader {
    pub fn open<P: AsRef<Path>>(path: P, mode: ReadMode) -> Result<Self, JournalError>;
    
    pub fn read_event(&mut self) -> Result<Option<EventJson>, JournalError>;
}
```

### 4.3 Verification

```rust
pub fn verify_event_id(
    event: &EventJson,
    canonicalizer: &Canonicalizer,
) -> Result<bool, JournalError>;
```

### 4.4 Types

- `EventJson` - Alias for `serde_json::Value` (untyped events)
- `ReadMode` - `Strict` or `Permissive`
- `WriteOptions` - Sync, create, append flags
- `JournalError` - Error types for journal operations

---

## 5. Usage Patterns

### 5.1 Creating and Writing Events

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
use northroot_journal::{JournalWriter, WriteOptions};
use serde_json::json;

// Create canonicalizer
let profile = ProfileId::parse("northroot-canonical-v1")?;
let canonicalizer = Canonicalizer::new(profile);

// Create event (as JSON)
let mut event = json!({
    "event_type": "test",
    "event_version": "1",
    "occurred_at": "2024-01-01T00:00:00Z",
    "principal_id": "service:example",
    "canonical_profile_id": "northroot-canonical-v1",
    "data": "example payload"
});

// Compute event_id
let event_id = compute_event_id(&event, &canonicalizer)?;
event["event_id"] = serde_json::to_value(&event_id)?;

// Write to journal
let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
writer.append_event(&event)?;
writer.finish()?;
```

### 5.2 Reading and Verifying Events

```rust
use northroot_canonical::{Canonicalizer, ProfileId};
use northroot_journal::{JournalReader, ReadMode, verify_event_id};

let profile = ProfileId::parse("northroot-canonical-v1")?;
let canonicalizer = Canonicalizer::new(profile);

let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;

while let Some(event) = reader.read_event()? {
    let is_valid = verify_event_id(&event, &canonicalizer)?;
    if !is_valid {
        eprintln!("Invalid event_id");
    }
}
```

---

## 6. Error Handling

### 6.1 Canonicalization Errors

```rust
pub enum CanonicalizationError {
    // ...
}
```

### 6.2 Journal Errors

```rust
pub enum JournalError {
    Io(std::io::Error),
    InvalidHeader(String),
    InvalidJson(String),
    // ...
}
```

### 6.3 Event ID Errors

```rust
pub enum EventIdError {
    Serialization(String),
    Canonicalization(CanonicalizationError),
    Digest(ValidationError),
    InvalidJson(String),
}
```

---

## 7. Invariants

- **Determinism**: All operations produce identical results across platforms
- **Offline**: No network dependencies
- **Untyped**: Kernel operates on `EventJson = serde_json::Value`
- **Schema-agnostic**: Journal format accepts any valid JSON event

---

## 8. Extension Points

The kernel does not provide:
- Typed event schemas (domain layers add these)
- Domain-specific verification (extensions implement this)
- Storage abstractions (extensions can layer on top)

See [Extensions](../reference/extensions.md) for how to extend the kernel.

---

## 9. Versioning

- API changes that break existing consumers require a major version bump
- New optional fields or additive changes are minor version bumps
- Canonicalization rule changes are breaking changes

---

## 10. Summary

The Northroot kernel API provides:
1. **Canonicalization**: `Canonicalizer::canonicalize()`
2. **Event Identity**: `compute_event_id()`, `verify_event_id()`
3. **Journal I/O**: `JournalWriter`, `JournalReader`

All operations are deterministic, offline-capable, and operate on untyped JSON.
