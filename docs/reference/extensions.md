# Extensions

This document describes how to extend Northroot with domain-specific event types and verification logic.

## Overview

The Northroot trust kernel provides:
- Canonicalization primitives (`northroot-canonical`)
- Journal container format (`northroot-journal`)
- Event identity computation

Domain layers extend the kernel by:
- Defining event schemas
- Adding typed event structures
- Implementing domain-specific verification
- Publishing cross-language fixtures

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

Store your schemas in your extension repository:
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

### 4.2 Domain Verification

Add your own verification logic:

```rust
fn verify_your_domain_event(
    event: &YourEvent,
    canonicalizer: &Canonicalizer,
) -> Result<VerificationVerdict, Error> {
    // 1. Verify event_id (kernel)
    let computed_id = compute_event_id(event, canonicalizer)?;
    if event.event_id != computed_id {
        return Ok(VerificationVerdict::Invalid);
    }

    // 2. Domain-specific checks
    if !your_domain_validation(event) {
        return Ok(VerificationVerdict::Violation);
    }

    Ok(VerificationVerdict::Ok)
}
```

---

## 5. Cross-Language Compatibility

### 5.1 Publish Fixtures

Create golden test fixtures that prove byte-level determinism:

```
your-extension/fixtures/
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

If your extension needs cross-language determinism for verification logic:
- Implement verification in Rust
- Provide FFI or WASM bindings
- Document the interop contract

---

## 6. Example Extensions

### 6.1 Governance Events

See `wip/governance/` for checkpoint and attestation event schemas.

### 6.2 Agent Domain

See `wip/agent-domain/` for authorization and execution event schemas.

---

## 7. Best Practices

1. **Keep schemas versioned**: Use `event_version` to track schema evolution
2. **Reference canonical types**: Don't redefine Digest, Quantity, etc.
3. **Publish fixtures**: Enable cross-language verification
4. **Document interop**: Specify how other languages should implement your events
5. **Separate concerns**: Kernel = bytes/hashes, Domain = semantics/validation

---

## 8. Summary

Extensions add:
- Typed event schemas
- Domain-specific verification
- Cross-language fixtures

The kernel provides:
- Canonicalization
- Event identity
- Journal format

This separation keeps the kernel minimal while enabling rich domain semantics.

