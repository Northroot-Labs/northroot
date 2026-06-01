# Integration Examples

Practical code examples for integrating Northroot into your application.

## Basic Event Recording

### Recording a Simple Event

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
use northroot_journal::{JournalWriter, WriteOptions};
use serde_json::json;

fn record_event() -> Result<(), Box<dyn std::error::Error>> {
    // Create canonicalizer
    let profile = ProfileId::parse("northroot-canonical-v1")?;
    let canonicalizer = Canonicalizer::new(profile);
    
    // Create event (without event_id)
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
    
    Ok(())
}
```

### Recording Multiple Events

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
use northroot_journal::{JournalWriter, WriteOptions};
use serde_json::json;

fn record_multiple_events() -> Result<(), Box<dyn std::error::Error>> {
    let profile = ProfileId::parse("northroot-canonical-v1")?;
    let canonicalizer = Canonicalizer::new(profile);
    
    let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
    
    for i in 0..10 {
        let mut event = json!({
            "event_type": "test",
            "event_version": "1",
            "occurred_at": "2024-01-01T00:00:00Z",
            "principal_id": "service:example",
            "canonical_profile_id": "northroot-canonical-v1",
            "sequence": i
        });
        
        let event_id = compute_event_id(&event, &canonicalizer)?;
        event["event_id"] = serde_json::to_value(&event_id)?;
        
        writer.append_event(&event)?;
    }
    
    writer.finish()?;
    Ok(())
}
```

## Reading and Verifying Events

### Reading All Events

```rust
use northroot_journal::{JournalReader, ReadMode};

fn read_events() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;
    
    while let Some(event) = reader.read_event()? {
        println!("Event ID: {}", event["event_id"]);
        println!("Event Type: {}", event["event_type"]);
    }
    
    Ok(())
}
```

### Verifying Events

```rust
use northroot_canonical::{Canonicalizer, ProfileId};
use northroot_journal::{JournalReader, ReadMode, verify_event_id};

fn verify_events() -> Result<(), Box<dyn std::error::Error>> {
    let profile = ProfileId::parse("northroot-canonical-v1")?;
    let canonicalizer = Canonicalizer::new(profile);
    
    let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;
    
    while let Some(event) = reader.read_event()? {
        let is_valid = verify_event_id(&event, &canonicalizer)?;
        if !is_valid {
            eprintln!("Invalid event_id for event: {}", event["event_id"]);
        } else {
            println!("Valid event: {}", event["event_id"]);
        }
    }
    
    Ok(())
}
```

## Filtering Events

The core kernel does not provide filtering. Implement filtering in your application layer:

```rust
use northroot_journal::{JournalReader, ReadMode};

fn filter_events_by_type(
    event_type: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;
    let mut results = Vec::new();
    
    while let Some(event) = reader.read_event()? {
        if event.get("event_type")
            .and_then(|v| v.as_str())
            .map(|s| s == event_type)
            .unwrap_or(false)
        {
            results.push(event);
        }
    }
    
    Ok(results)
}

// Usage
let auth_events = filter_events_by_type("authorization")?;
```

## Custom Event Types

Define your own event types following the canonical envelope structure:

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, Digest, PrincipalId, ProfileId, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct CustomEvent {
    pub event_id: Digest,
    pub event_type: String,
    pub event_version: String,
    pub occurred_at: Timestamp,
    pub principal_id: PrincipalId,
    pub canonical_profile_id: ProfileId,
    
    // Your domain-specific fields
    pub action: String,
    pub resource: String,
}

fn create_custom_event() -> Result<(), Box<dyn std::error::Error>> {
    let profile = ProfileId::parse("northroot-canonical-v1")?;
    let canonicalizer = Canonicalizer::new(profile);
    
    // Create event as JSON (without event_id)
    let mut event_json = json!({
        "event_type": "custom_action",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:example",
        "canonical_profile_id": "northroot-canonical-v1",
        "action": "read",
        "resource": "file://example.txt"
    });
    
    // Compute event_id
    let event_id = compute_event_id(&event_json, &canonicalizer)?;
    event_json["event_id"] = serde_json::to_value(&event_id)?;
    
    // Write to journal
    let mut writer = northroot_journal::JournalWriter::open(
        "events.nrj",
        northroot_journal::WriteOptions::default()
    )?;
    writer.append_event(&event_json)?;
    writer.finish()?;
    
    Ok(())
}
```

## Error Handling

```rust
use northroot_canonical::CanonicalizationError;
use northroot_journal::JournalError;

fn handle_errors() -> Result<(), Box<dyn std::error::Error>> {
    match northroot_journal::JournalReader::open("events.nrj", northroot_journal::ReadMode::Strict) {
        Ok(reader) => {
            // Use reader
            Ok(())
        }
        Err(JournalError::Io(e)) => {
            eprintln!("I/O error: {}", e);
            Err(e.into())
        }
        Err(e) => {
            eprintln!("Journal error: {:?}", e);
            Err(e.into())
        }
    }
}
```

## Best Practices

1. **Always verify events** before trusting them using `verify_event_id`
2. **Compute event_id before writing** - events must include `event_id` when written
3. **Handle errors explicitly** - don't ignore canonicalization or journal errors
4. **Use appropriate read modes** - `Strict` for production, `Permissive` for recovery
5. **Implement filtering in application layer** - core kernel doesn't provide filtering

For more details, see:
- [API Contract](../developer/api-contract.md) - Complete API reference
- [Layering on Northroot](../developer/layering.md) - How to layer consumer protocols over the kernel
- [Core Specification](../reference/spec.md) - Event structure and semantics
