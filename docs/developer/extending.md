# Extending Northroot

How to implement custom backends, filters, and verification logic.

## Custom Storage Backends

Implement `StoreWriter` and `StoreReader` for custom storage:

```rust
use northroot_store::{StoreWriter, StoreReader, StoreError, EventJson};

pub struct CustomBackend {
    // Your storage implementation
}

impl StoreWriter for CustomBackend {
    fn append(&mut self, event: &EventJson) -> Result<(), StoreError> {
        // Write event to your storage
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), StoreError> {
        // Flush any buffered writes
        Ok(())
    }
    
    fn finish(self) -> Result<(), StoreError> {
        // Finalize and close storage
        Ok(())
    }
}

impl StoreReader for CustomBackend {
    fn read_next(&mut self) -> Result<Option<EventJson>, StoreError> {
        // Read next event from storage
        Ok(None)
    }
}
```

## Custom Filters

Implement `EventFilter` for custom filtering logic:

```rust
use northroot_store::{EventFilter, EventJson};

pub struct CustomFilter {
    // Filter configuration
}

impl EventFilter for CustomFilter {
    fn matches(&self, event: &EventJson) -> bool {
        // Return true if event matches filter criteria
        true
    }
}
```

Use with `FilteredReader`:

```rust
use northroot_store::{FilteredReader, JournalBackendReader, ReadMode};

let reader = JournalBackendReader::open("events.nrj", ReadMode::Strict)?;
let filter = CustomFilter { /* ... */ };
let mut filtered = FilteredReader::new(reader, filter);

while let Some(event) = filtered.read_next()? {
    // Only matching events
}
```

## Custom Verification

Wrap `Verifier` with additional checks:

```rust
use northroot_core::Verifier;
use northroot_core::events::AuthorizationEvent;

pub struct CustomVerifier {
    verifier: Verifier,
    // Additional verification state
}

impl CustomVerifier {
    pub fn verify_with_custom_checks(
        &self,
        event: &AuthorizationEvent,
    ) -> Result<VerificationVerdict, String> {
        // Use base verifier
        let (digest, verdict) = self.verifier.verify_authorization(event)?;
        
        // Add custom checks
        if !self.custom_check(event) {
            return Ok(VerificationVerdict::Invalid);
        }
        
        Ok(verdict)
    }
    
    fn custom_check(&self, event: &AuthorizationEvent) -> bool {
        // Your custom verification logic
        true
    }
}
```

## Composite Filters

Combine multiple filters:

```rust
use northroot_store::{AndFilter, OrFilter, EventTypeFilter, PrincipalFilter};

// All filters must match
let filter = AndFilter {
    filters: vec![
        Box::new(EventTypeFilter { event_type: "execution".into() }),
        Box::new(PrincipalFilter { principal_id: "service:api".into() }),
    ],
};

// Any filter must match
let filter = OrFilter {
    filters: vec![
        Box::new(EventTypeFilter { event_type: "authorization".into() }),
        Box::new(EventTypeFilter { event_type: "execution".into() }),
    ],
};
```

## Best Practices

1. **Follow trait contracts**: Implement all required methods correctly
2. **Handle errors explicitly**: Return appropriate `StoreError` variants
3. **Maintain determinism**: Custom logic should be deterministic
4. **Document behavior**: Document any custom behavior or constraints
5. **Test thoroughly**: Write tests for custom implementations

## Related Documentation

- [API Contract](api-contract.md) - Complete API reference
- [Architecture](architecture.md) - System design overview
- [Testing Guide](testing.md) - How to test extensions

