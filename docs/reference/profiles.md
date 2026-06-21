# Profiles and Consumer Protocols

This document describes how to layer profile semantics, consumer protocols, and domain-specific verification on top of Northroot.

## Overview

The Northroot trust kernel provides:
- Canonicalization primitives (`northroot-canonical`)
- Journal container format (`northroot-journal`)
- Event identity computation

Profile and consumer layers use the kernel by:
- Defining event schemas
- Adding typed event structures
- Implementing domain-specific verification
- Publishing cross-language fixtures

The kernel may preserve and verify profile-bearing events. The kernel must not decide profile meaning.

---

## 1. Defining Domain Event Schemas

### 1.1 Schema Structure

Domain events must follow the common envelope structure:

```json
{
  "event_id": { "alg": "sha-256", "b64": "..." },
  "event_type": "your_domain_event",
  "event_version": "1",
  "occurred_at": "2024-01-01T00:00:00Z",
  "principal_id": "service:example",
  "canonical_profile_id": "northroot-canonical-v1",
  "prev_event_id": { "alg": "sha-256", "b64": "..." },  // optional
  // ... your domain-specific fields
}
```

### 1.2 Using Kernel Primitives

Reference canonical types from `schemas/canonical/`:
- `Digest` for identifiers and hashes
- `Quantity` for numeric values (Dec, Int, Rat, F64)
- `Timestamp` for time values
- `PrincipalId` for actor identifiers
- `ProfileId` for canonicalization profiles

### 1.3 Schema Location

Store your schemas in your consuming repository:
- `schemas/events/v1/your_event.schema.json`
- Reference canonical types via `$ref`

---

## 2. Computing Event Identity

Use the kernel's `compute_event_id` function:

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};

let profile = ProfileId::parse("northroot-canonical-v1")?;
let canonicalizer = Canonicalizer::new(profile);

// Your event as serde_json::Value
let event_json: serde_json::Value = serde_json::to_value(&your_event)?;

// Compute event_id
let event_id = compute_event_id(&event_json, &canonicalizer)?;

// Add event_id to your event
// ... then serialize and write to journal
```

The kernel ensures byte-level determinism across languages.

---

## 3. Writing to Journal

Use the kernel's journal writer:

```rust
use northroot_journal::{JournalWriter, WriteOptions};

let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
writer.append_event(&event_json)?;
writer.finish()?;
```

The journal format is schema-agnostic - it stores any valid JSON event.

---

## 4. Verification

### 4.1 Core Verification

The kernel provides `verify_event_id`:

```rust
use northroot_journal::verify_event_id;

let is_valid = verify_event_id(&event_json, &canonicalizer)?;
```

This verifies:
- Event ID matches computed hash
- Canonicalization is correct

### 4.2 Profile or Domain Verification

Add your own verification logic:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DomainVerdict {
    Ok,
    InvalidIdentity,
    DomainViolation,
}

fn verify_your_domain_event(
    event: &YourEvent,
    canonicalizer: &Canonicalizer,
) -> Result<DomainVerdict, Error> {
    // 1. Verify event_id with kernel primitives.
    let computed_id = compute_event_id(event, canonicalizer)?;
    if event.event_id != computed_id {
        return Ok(DomainVerdict::InvalidIdentity);
    }

    // 2. Add profile/domain checks above the kernel.
    if !your_domain_validation(event) {
        return Ok(DomainVerdict::DomainViolation);
    }

    Ok(DomainVerdict::Ok)
}
```

---

## 5. Cross-Language Compatibility

### 5.1 Publish Fixtures

Create golden test fixtures that prove byte-level determinism:

```
your-profile/fixtures/
  canonical/
    your_event_input.json
    your_event_canonical.hex
  event-id/
    your_event_input.json
    your_event_event_id.json
  nrj/
    your_event.nrj
```

### 5.2 Test Suite

Provide a test suite that:
- Regenerates fixtures deterministically
- Verifies canonical bytes match across languages
- Verifies event_id computation matches across languages

### 5.3 FFI/WASM Bindings

If your profile needs cross-language determinism for verification logic:
- Implement verification in Rust
- Provide FFI or WASM bindings
- Document the interop contract

---

## 6. Example Profiles and Consumer Protocols

Northroot v0.1 keeps profile schemas outside the kernel. Consuming repositories
may define their own schemas, validators, admission gates, and projections as
long as they treat kernel verification as byte/integrity verification only.

The work-ledger profile is incubating and currently documented in
[Work Ledger](work-ledger.md). It is useful for dogfooding but is not a stable
kernel API.

---

## 7. Best Practices

1. **Keep schemas versioned**: Use `event_version` to track schema evolution
2. **Reference canonical types**: Don't redefine Digest, Quantity, etc.
3. **Publish fixtures**: Enable cross-language verification
4. **Document interop**: Specify how other languages should implement your events
5. **Separate concerns**: Kernel = bytes/hashes, profile = schema/validation, policy = admissibility

---

## 8. Summary

Profiles and consumer protocols add:
- Typed event schemas
- Domain-specific verification
- Cross-language fixtures

The kernel provides:
- Canonicalization
- Event identity
- Journal format

This separation keeps the kernel minimal while enabling rich domain semantics.
