# Layering on Northroot

How to layer profile semantics, consumer protocols, domain-specific verification logic, and storage backends over the trust kernel.

## Overview

The Northroot trust kernel provides minimal primitives:
- Canonicalization (`northroot-canonical`)
- Event identity computation
- Journal format (`northroot-journal`)

Profile and consumer layers build on these primitives to add:
- Domain-specific event schemas
- Custom verification logic
- Alternative storage backends

The kernel may preserve and verify profile-bearing events. The kernel must not decide profile meaning.

---

## 1. Custom Event Schemas

Define domain-specific event types that use kernel primitives:

```rust
use northroot_canonical::{Digest, PrincipalId, Timestamp, ProfileId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CustomEvent {
    // Required envelope fields
    pub event_id: Digest,
    pub event_type: String,
    pub event_version: String,
    pub occurred_at: Timestamp,
    pub principal_id: PrincipalId,
    pub canonical_profile_id: ProfileId,
    
    // Optional envelope fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_event_id: Option<Digest>,
    
    // Your domain-specific fields
    pub action: String,
    pub resource: String,
    pub metadata: serde_json::Value,
}
```

Compute event identity using the kernel:

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};

let profile = ProfileId::parse("northroot-canonical-v1")?;
let canonicalizer = Canonicalizer::new(profile);

// Create event (without event_id)
let mut event = CustomEvent { /* ... */ };

// Compute event_id
let event_id = compute_event_id(&event, &canonicalizer)?;
event.event_id = event_id;

// Write to journal
let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
writer.append_event(&serde_json::to_value(&event)?)?;
writer.finish()?;
```

---

## 2. Custom Verification

Wrap kernel verification with domain-specific checks:

```rust
use northroot_canonical::{Canonicalizer, Digest, ProfileId, verify_event_id};
use northroot_journal::{JournalReader, ReadMode, EventJson};

pub struct CustomVerifier {
    canonicalizer: Canonicalizer,
    // Your verification state
}

impl CustomVerifier {
    pub fn new(profile: ProfileId) -> Self {
        Self {
            canonicalizer: Canonicalizer::new(profile),
        }
    }
    
    pub fn verify_event(
        &self,
        event: &EventJson,
        claimed_id: &Digest,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Use kernel's verify_event_id
        let id_valid = verify_event_id(event, claimed_id, &self.canonicalizer)?;
        if !id_valid {
            return Ok(false);
        }
        
        // Add domain-specific checks
        if !self.validate_event_structure(event)? {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    fn validate_event_structure(&self, event: &EventJson) -> Result<bool, Box<dyn std::error::Error>> {
        // Your custom validation logic
        // e.g., check required fields, validate field values, etc.
        Ok(true)
    }
}
```

---

## 3. Custom Storage Backends

Implement custom storage by wrapping or replacing journal I/O:

### Option A: Wrap JournalWriter/Reader

```rust
use northroot_journal::{JournalWriter, JournalReader, EventJson, WriteOptions, ReadMode};
use std::path::Path;

pub struct CustomStorage {
    // Your storage backend (database, S3, etc.)
}

impl CustomStorage {
    pub fn write_event(&mut self, event: &EventJson) -> Result<(), Box<dyn std::error::Error>> {
        // Option 1: Use journal format internally
        let mut writer = JournalWriter::open("backing.nrj", WriteOptions::default())?;
        writer.append_event(event)?;
        writer.finish()?;
        
        // Option 2: Store in your own format
        // ... your storage logic
        
        Ok(())
    }
    
    pub fn read_events(&mut self) -> Result<Vec<EventJson>, Box<dyn std::error::Error>> {
        // Read from your storage and return events
        Ok(vec![])
    }
}
```

### Option B: Implement Custom Reader/Writer

For advanced use cases, implement your own reader/writer that produces/consumes `EventJson`:

```rust
use northroot_journal::EventJson;

pub trait CustomEventSource {
    fn read_next(&mut self) -> Result<Option<EventJson>, Box<dyn std::error::Error>>;
}

pub trait CustomEventSink {
    fn write(&mut self, event: &EventJson) -> Result<(), Box<dyn std::error::Error>>;
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
```

---

## 4. Filtering and Querying

The core kernel does not provide filtering. Implement filtering in your application layer:

```rust
use northroot_journal::{JournalReader, ReadMode, EventJson};

pub fn filter_events(
    reader: &mut JournalReader,
    predicate: impl Fn(&EventJson) -> bool,
) -> Result<Vec<EventJson>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    
    while let Some(event) = reader.read_event()? {
        if predicate(&event) {
            results.push(event);
        }
    }
    
    Ok(results)
}

// Usage
let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;
let filtered = filter_events(&mut reader, |event| {
    event.get("event_type")
        .and_then(|v| v.as_str())
        .map(|s| s == "authorization")
        .unwrap_or(false)
})?;
```

---

## Best Practices

1. **Use kernel primitives**: Always use `compute_event_id` and `Canonicalizer` from the kernel
2. **Maintain determinism**: Custom logic should be deterministic and testable
3. **Validate at boundaries**: Validate event structure before computing event_id
4. **Handle errors explicitly**: Return appropriate error types
5. **Test thoroughly**: Write tests for custom implementations
6. **Document schemas**: Document your event schemas and validation rules

---

## Related Documentation

- [API Contract](api-contract.md) - Complete API reference
- [Architecture](architecture.md) - System design overview
- [Profiles and Consumer Protocols](../reference/profiles.md) - Layering patterns and examples
- [Testing Guide](testing.md) - How to test profile and consumer layers
