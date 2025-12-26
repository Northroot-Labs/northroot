# northroot-store

Pluggable storage backend abstraction for Northroot events.

## Overview

The `northroot-store` crate provides a storage abstraction layer for canonical Northroot events. It defines `StoreWriter` and `StoreReader` traits that allow different storage backends to be used interchangeably, while shipping a default journal-backed implementation using `northroot-journal`.

Key features:

- **Pluggable backends**: Implement `StoreWriter` and `StoreReader` to add new storage backends
- **Journal backend**: Default implementation using the append-only journal format (see `docs/reference/format.md`)
- **Simple API**: Unified interface for writing and reading events regardless of backend
- **Error handling**: `StoreError` wraps backend-specific errors for consistent handling

## Usage

### Writing Events

```rust
use northroot_store::{JournalBackendWriter, StoreWriter, WriteOptions};
use serde_json::json;

let mut writer = JournalBackendWriter::open("events.nrj", WriteOptions::default())?;

let event = json!({
    "event_id": { "alg": "sha-256", "b64": "..." },
    "event_type": "authorization",
    "event_version": "1",
    "occurred_at": "2024-01-01T00:00:00Z",
    "principal_id": "service:test",
    "canonical_profile_id": "northroot-canonical-v1",
    // ... other event fields
});

writer.append(&event)?;
writer.finish()?;
```

### Reading Events

```rust
use northroot_store::{JournalBackendReader, ReadMode, StoreReader};

let mut reader = JournalBackendReader::open("events.nrj", ReadMode::Strict)?;

while let Some(event) = reader.read_next()? {
    // Process event
    println!("Event: {:?}", event);
}
```

### Using Different Backends

The store abstraction allows you to swap backends without changing your application code:

```rust
use northroot_store::{StoreWriter, StoreReader};

// Use journal backend
let mut writer: Box<dyn StoreWriter> = Box::new(
    JournalBackendWriter::open("events.nrj", WriteOptions::default())?
);

// Future: use S3 backend, in-memory backend, etc.
// let mut writer: Box<dyn StoreWriter> = Box::new(
//     S3BackendWriter::new(bucket, key)?
// );

writer.append(&event)?;
```

## Implementing a Custom Backend

To implement a custom storage backend, implement the `StoreWriter` and `StoreReader` traits:

```rust
use northroot_store::{StoreWriter, StoreReader, StoreError, EventJson};

pub struct MyBackendWriter { /* ... */ }

impl StoreWriter for MyBackendWriter {
    fn append(&mut self, event: &EventJson) -> Result<(), StoreError> {
        // Your implementation
        Ok(())
    }

    fn flush(&mut self) -> Result<(), StoreError> {
        // Your implementation (no-op if unbuffered)
        Ok(())
    }

    fn finish(self) -> Result<(), StoreError> {
        // Your implementation
        Ok(())
    }
}

pub struct MyBackendReader { /* ... */ }

impl StoreReader for MyBackendReader {
    fn read_next(&mut self) -> Result<Option<EventJson>, StoreError> {
        // Your implementation
        Ok(None)
    }
}
```

## Configuration

### Write Options

The journal backend uses `WriteOptions` from `northroot-journal`:

- `sync`: Whether to fsync after each append (default: `false`)
- `create`: Whether to create the file if it doesn't exist (default: `true`)
- `append`: Whether to append to an existing file (default: `true`)

### Read Modes

The journal backend uses `ReadMode` from `northroot-journal`:

- `ReadMode::Strict`: Truncated frames are errors
- `ReadMode::Permissive`: Truncation is treated as end-of-file

## Limits and Constraints

- **Payload size**: Maximum 16 MiB per event (enforced by `StoreWriter::append`)
- **Append-only**: The store abstraction assumes append-only semantics; updates and deletes are not supported
- **Sync I/O**: The current trait design uses synchronous I/O; async variants are planned for a future release

## Error Handling

The crate uses `StoreError` for all error cases:

- `Io`: I/O errors during read/write
- `Journal`: Errors from the journal backend
- `PayloadTooLarge`: Payload exceeds 16 MiB limit
- `Other`: Generic error for other cases

## What This Crate Does

- Provides a unified storage abstraction for Northroot events
- Ships a default journal-backed implementation
- Enforces payload size limits
- Preserves append-only semantics

## What This Crate Does NOT Do (v1)

The following features are explicitly out of scope for v1 and may be added in future releases:

- **Async I/O**: Traits are synchronous only; async variants will be added in v2+
- **Receipts and checkpoints**: Higher-level receipt/checkpoint storage is not included
- **Transactions**: Multi-event transactions are not supported
- **Alternative backends**: Only the journal backend is provided; S3, in-memory, and other backends are planned for v2+

## Examples

See the `tests/` directory for comprehensive examples:

- `tests/integration.rs`: Basic read/write operations, round-trip tests, payload size limits, and strict/permissive mode handling

## Dependencies

- `northroot-journal`: Journal format implementation (reference backend)
- `serde_json`: JSON serialization
- `thiserror`: Error types

## Reference Implementation

The journal backend is the reference implementation and follows the format specified in `docs/reference/format.md`. For details on the journal format, see:

- `docs/reference/format.md`: Journal format specification
- `crates/northroot-journal/README.md`: Journal crate documentation

## License

Licensed under either of Apache-2.0 OR MIT at your option.

