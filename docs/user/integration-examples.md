# Integration Examples

Practical code examples for integrating Northroot into your application.

## Basic Event Recording

### Recording an Authorization Event

```rust
use northroot_store::{JournalBackendWriter, StoreWriter};
use northroot_core::events::AuthorizationEvent;
use serde_json::json;

fn record_authorization() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = JournalBackendWriter::create("events.nrj")?;
    
    let event = json!({
        "event_type": "authorization",
        "event_version": "1",
        "event_id": { "alg": "sha-256", "b64": "..." },
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:api",
        "canonical_profile_id": "profile:v1",
        // ... other fields
    });
    
    writer.append(&event)?;
    writer.finish()?;
    Ok(())
}
```

### Recording an Execution Event

```rust
use northroot_store::{JournalBackendWriter, StoreWriter};

fn record_execution(auth_event_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = JournalBackendWriter::open("events.nrj")?;
    
    let event = json!({
        "event_type": "execution",
        "event_version": "1",
        "auth_event_id": auth_event_id,
        "tool_name": "llm:gpt-4",
        "outcome": "success",
        // ... other fields
    });
    
    writer.append(&event)?;
    writer.finish()?;
    Ok(())
}
```

## Reading and Verifying Events

### Reading All Events

```rust
use northroot_store::{JournalBackendReader, StoreReader};

fn read_events() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = JournalBackendReader::open("events.nrj")?;
    
    while let Some(event) = reader.read_next()? {
        println!("Event: {}", event["event_id"]);
    }
    
    Ok(())
}
```

### Verifying Events

```rust
use northroot_core::Verifier;
use northroot_canonical::Canonicalizer;

fn verify_events() -> Result<(), Box<dyn std::error::Error>> {
    let canonicalizer = Canonicalizer::new()?;
    let verifier = Verifier::new(canonicalizer);
    
    let mut reader = JournalBackendReader::open("events.nrj")?;
    
    while let Some(event) = reader.read_next()? {
        // Parse and verify based on event type
        let typed = parse_event(&event)?;
        match typed {
            TypedEvent::Authorization(auth) => {
                let (digest, verdict) = verifier.verify_authorization(&auth)?;
                println!("Authorization: {:?}", verdict);
            }
            // ... other event types
        }
    }
    
    Ok(())
}
```

## Filtering Events

### Filter by Event Type

```rust
use northroot_store::{JournalBackendReader, EventTypeFilter, FilteredReader};

fn filter_by_type() -> Result<(), Box<dyn std::error::Error>> {
    let reader = JournalBackendReader::open("events.nrj")?;
    let filter = EventTypeFilter {
        event_type: "execution".to_string(),
    };
    let mut filtered = FilteredReader::new(reader, filter);
    
    while let Some(event) = filtered.read_next()? {
        // Only execution events
        println!("Execution: {}", event["event_id"]);
    }
    
    Ok(())
}
```

### Filter by Principal

```rust
use northroot_store::{JournalBackendReader, PrincipalFilter, FilteredReader};

fn filter_by_principal() -> Result<(), Box<dyn std::error::Error>> {
    let reader = JournalBackendReader::open("events.nrj")?;
    let filter = PrincipalFilter {
        principal_id: "service:api".to_string(),
    };
    let mut filtered = FilteredReader::new(reader, filter);
    
    while let Some(event) = filtered.read_next()? {
        println!("Event: {}", event["event_id"]);
    }
    
    Ok(())
}
```

## Custom Storage Backend

Implement `StoreWriter` and `StoreReader` for custom backends:

```rust
use northroot_store::{StoreWriter, StoreReader, StoreError};
use serde_json::Value;

struct CustomBackend {
    // Your storage implementation
}

impl StoreWriter for CustomBackend {
    fn append(&mut self, event: &Value) -> Result<(), StoreError> {
        // Write to your storage
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), StoreError> {
        Ok(())
    }
    
    fn finish(self) -> Result<(), StoreError> {
        Ok(())
    }
}
```

## Error Handling

```rust
use northroot_store::StoreError;

fn handle_errors() -> Result<(), StoreError> {
    match JournalBackendReader::open("events.nrj") {
        Ok(reader) => {
            // Use reader
            Ok(())
        }
        Err(StoreError::Io(e)) => {
            eprintln!("I/O error: {}", e);
            Err(StoreError::Io(e))
        }
        Err(e) => {
            eprintln!("Store error: {:?}", e);
            Err(e)
        }
    }
}
```

## Best Practices

1. **Always verify events** before trusting them
2. **Use typed event parsing** for type safety
3. **Handle errors explicitly** - don't ignore store errors
4. **Flush writers** when batching multiple events
5. **Use filters** for efficient event scanning

For more details, see:
- [API Contract](../developer/api-contract.md) - Complete API reference
- [Core Specification](../reference/spec.md) - Event structure and semantics

