# Getting Started with Northroot

This guide will help you get started with Northroot for recording and verifying events.

## What You'll Learn

- Installing and building Northroot
- Creating your first journal
- Recording events
- Verifying events
- Basic integration patterns

## Installation

### From Source

```bash
git clone <repository-url>
cd northroot
cargo build --release
```

The CLI binary will be at `apps/northroot/target/release/northroot`.

### Verify Installation

```bash
northroot --help
```

## Your First Journal

### Creating a Journal

Northroot stores events in journal files (`.nrj` format). You can create a journal programmatically or use the CLI to inspect existing journals.

### Recording Events

Events are typically created programmatically using the Northroot Rust crates:

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

See [Integration Examples](integration-examples.md) for complete code samples.

### Reading Events

```bash
northroot read events.nrj
```

### Verifying Events

Verify all events in a journal:

```bash
northroot verify events.nrj
```

This checks:
- Event identity (`event_id` matches canonical bytes)
- Journal format integrity

## Next Steps

- [Integration Examples](integration-examples.md) - Code samples for integration
- [API Contract](../developer/api-contract.md) - Complete API reference
- [Core Specification](../reference/spec.md) - Protocol details
