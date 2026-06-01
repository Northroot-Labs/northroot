# Northroot API Contract

Version: 0.1
Status: Stable kernel
Scope: Trust kernel API surface

---

## 1. Purpose

This document provides an overview of the public API contract for Northroot trust kernel crates. For complete API reference with detailed signatures, examples, and cross-references, see the auto-generated [rustdoc documentation](https://docs.rs/northroot-canonical) and [rustdoc documentation](https://docs.rs/northroot-journal).

The kernel provides:
- **Canonicalization**: Deterministic JSON canonicalization (RFC 8785 + Northroot rules)
- **Event Identity**: Content-derived event identifiers
- **Journal Format**: Portable, append-only event container
- **Structural Journal Metadata**: rebuildable segment manifests and checkpoints

---

## 2. Crate Responsibilities

| Crate | Responsibility | Documentation |
|-------|----------------|---------------|
| `northroot-canonical` | Canonicalization, digests, quantities, identifiers, event ID computation | [API Docs](https://docs.rs/northroot-canonical) |
| `northroot-journal` | Append-only journal format (.nrj) | [API Docs](https://docs.rs/northroot-journal) |

---

## 3. Core APIs

### 3.1 Canonicalization (`northroot-canonical`)

**Key Types:**
- [`Canonicalizer`](https://docs.rs/northroot-canonical/latest/northroot_canonical/struct.Canonicalizer.html) - Produces deterministic canonical bytes
- [`compute_event_id`](https://docs.rs/northroot-canonical/latest/northroot_canonical/fn.compute_event_id.html) - Computes content-derived event identifiers
- [`verify_event_id`](https://docs.rs/northroot-canonical/latest/northroot_canonical/fn.verify_event_id.html) - Verifies event identity

**Primitive Types:**
- `Digest` - Content-addressed identifiers (alg + b64)
- `Quantity` - Lossless numeric types (Dec, Int, Rat, F64)
- `Timestamp` - RFC 3339 timestamps
- `PrincipalId` - Actor identifiers
- `ProfileId` - Canonicalization profile identifiers

See the [rustdoc API reference](https://docs.rs/northroot-canonical) for complete type definitions and method signatures.

### 3.2 Journal I/O (`northroot-journal`)

**Key Types:**
- [`JournalWriter`](https://docs.rs/northroot-journal/latest/northroot_journal/struct.JournalWriter.html) - Writes events to journal files
- [`JournalReader`](https://docs.rs/northroot-journal/latest/northroot_journal/struct.JournalReader.html) - Reads events from journal files
- [`verify_event_id`](https://docs.rs/northroot-journal/latest/northroot_journal/fn.verify_event_id.html) - Verifies event identity in journal context

**Supporting Types:**
- `EventJson` - Alias for `serde_json::Value` (untyped events)
- `ReadMode` - `Strict` or `Permissive`
- `WriteOptions` - Sync, create, append flags
- `JournalError` - Error types for journal operations

See the [rustdoc API reference](https://docs.rs/northroot-journal) for complete type definitions and method signatures.

---

## 4. Usage Patterns

### 4.1 Creating and Writing Events

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

### 4.2 Reading and Verifying Events

```rust
use northroot_canonical::{Canonicalizer, ProfileId};
use northroot_journal::{JournalReader, ReadMode, verify_event_id};

let profile = ProfileId::parse("northroot-canonical-v1")?;
let canonicalizer = Canonicalizer::new(profile);

let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;

while let Some(event) = reader.read_event()? {
    // Verify using journal's verify_event_id helper
    let is_valid = verify_event_id(&event, &canonicalizer)?;
    if !is_valid {
        eprintln!("Invalid event_id");
    }
}
```

---

## 5. Error Handling

### 5.1 Error Types

All error types are documented in the rustdoc API reference:

- [`CanonicalizationError`](https://docs.rs/northroot-canonical/latest/northroot_canonical/enum.CanonicalizationError.html) - Canonicalization failures
- [`EventIdError`](https://docs.rs/northroot-canonical/latest/northroot_canonical/enum.EventIdError.html) - Event ID computation failures
- [`JournalError`](https://docs.rs/northroot-journal/latest/northroot_journal/enum.JournalError.html) - Journal I/O failures

### 5.2 Error Handling Patterns

All errors implement `std::error::Error` and can be converted using `?`:

```rust
use northroot_canonical::CanonicalizationError;
use northroot_journal::JournalError;

fn process_event() -> Result<(), Box<dyn std::error::Error>> {
    // Errors propagate automatically
    let canonicalizer = Canonicalizer::new(profile)?;
    let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
    writer.append_event(&event)?;
    Ok(())
}
```

---

## 6. Invariants

- **Determinism**: All operations produce identical results across platforms
- **Offline**: No network dependencies
- **Untyped**: Kernel operates on `EventJson = serde_json::Value`
- **Schema-agnostic**: Journal format accepts any valid JSON event
- **Single-writer**: v0.1 assumes one writer with many readers; write
  coordination and multi-event transactions are outside the kernel
- **Structural checkpoints only**: Checkpoints identify verified prefixes, not
  semantic state

---

## 7. Profiles and Consumer Protocols

The kernel does not provide:
- Typed event schemas (domain layers add these)
- Domain-specific verification (profiles or consuming protocols implement this)
- Storage abstractions (consumer layers can add these)

See [Profiles and Consumer Protocols](../reference/profiles.md) and [Layering on Northroot](layering.md) for layering on the kernel.

---

## 8. Versioning

- API changes that break existing consumers require a major version bump
- New optional fields or additive changes are minor version bumps
- Canonicalization rule changes are breaking changes

---

## 9. Generating Documentation Locally

To generate and view rustdoc documentation locally:

```bash
# Generate docs for all crates
cargo doc --workspace --no-deps --open

# Generate docs for specific crate
cargo doc --package northroot-canonical --open
cargo doc --package northroot-journal --open
```

The generated documentation includes:
- Complete API reference with all types and methods
- Code examples from rustdoc comments
- Cross-references between related types
- Links to related documentation

---

## 10. Summary

The Northroot kernel API provides:
1. **Canonicalization**: [`Canonicalizer::canonicalize()`](https://docs.rs/northroot-canonical/latest/northroot_canonical/struct.Canonicalizer.html#method.canonicalize)
2. **Event Identity**: [`compute_event_id()`](https://docs.rs/northroot-canonical/latest/northroot_canonical/fn.compute_event_id.html), [`verify_event_id()`](https://docs.rs/northroot-canonical/latest/northroot_canonical/fn.verify_event_id.html)
3. **Journal I/O**: [`JournalWriter`](https://docs.rs/northroot-journal/latest/northroot_journal/struct.JournalWriter.html), [`JournalReader`](https://docs.rs/northroot-journal/latest/northroot_journal/struct.JournalReader.html)
4. **Structural Segments/Checkpoints**: `northroot journal verify-segments`,
   `northroot journal manifest`, and `northroot journal checkpoint`

All operations are deterministic, offline-capable, and operate on untyped JSON.

For complete API reference, see the [rustdoc documentation](https://docs.rs/northroot-canonical) and [rustdoc documentation](https://docs.rs/northroot-journal).
