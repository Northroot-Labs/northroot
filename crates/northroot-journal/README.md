# northroot-journal

Append-only journal format for canonical Northroot events.

## Overview

The `northroot-journal` crate provides a durable, tamper-evident storage format for canonical Northroot events. It implements the journal format specified in `docs/reference/format.md`, providing:

- **Append-only framing**: Fixed header with magic bytes and version, followed by a stream of framed records
- **Event storage**: Canonical event JSON objects stored as `EventJson` frames
- **Verification hooks**: Integration with `northroot-canonical` for event identity validation
- **Resilience**: Strict and permissive read modes for handling truncation and corruption

## Usage

### Writing Events

```rust
use northroot_journal::{JournalWriter, WriteOptions};
use serde_json::json;

let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;

let event = json!({
    "event_id": { "alg": "sha-256", "b64": "..." },
    "event_type": "authorization",
    // ... other event fields
});

writer.append_event(&event)?;
writer.finish()?;
```

### Reading Events

```rust
use northroot_journal::{JournalReader, ReadMode};

let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;

while let Some(event) = reader.read_event()? {
    // Process event
    println!("Event: {:?}", event);
}
```

### Verification

```rust
use northroot_canonical::{Canonicalizer, ProfileId};
use northroot_journal::verify_event_id;

let canonicalizer = Canonicalizer::new(ProfileId::parse("northroot-canonical-v1")?);

// Verify event_id matches computed digest
let is_valid = verify_event_id(&event, &canonicalizer)?;
```

## Configuration

### Write Options

- `sync`: Whether to fsync after each append (default: `false`)
- `create`: Whether to create the file if it doesn't exist (default: `true`)
- `append`: Whether to append to an existing file (default: `true`)

### Read Modes

- `ReadMode::Strict`: Truncated frames are errors
- `ReadMode::Permissive`: Truncation is treated as end-of-file

## Format Specification

The journal format is fully specified in `docs/reference/format.md`. Key points:

- **Header**: 16 bytes (`NRJ1` magic, version `0x0001`, flags, reserved)
- **Frames**: 8-byte header (kind, reserved, length) + payload
- **Event frames**: Kind `0x01` contains UTF-8 JSON event objects
- **Limits**: Maximum payload size is 16 MiB (recommended)

## Error Handling

The crate uses `JournalError` for all error cases:

- `Io`: I/O errors during read/write
- `InvalidHeader`: Invalid magic, version, or flags
- `InvalidFrame`: Invalid frame structure or reserved bytes
- `PayloadTooLarge`: Payload exceeds 16 MiB limit
- `TruncatedFrame`: Truncation detected in strict mode
- `InvalidUtf8`: Invalid UTF-8 in event payload
- `JsonParse`: Invalid JSON in event payload

## Examples

See the `tests/` directory for comprehensive examples:

- `tests/integration.rs`: Basic read/write operations
- `tests/verification.rs`: Event verification examples
- `tests/resilience.rs`: Error handling and edge cases

## Dependencies

- `northroot-canonical`: Canonicalization and digest types
- `serde_json`: JSON serialization
- `thiserror`: Error types

## License

Licensed under either of Apache-2.0 or MIT at your option.

