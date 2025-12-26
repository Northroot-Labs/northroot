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

The CLI binary will be at `target/release/northroot`.

### Verify Installation

```bash
northroot --version
```

## Your First Journal

### Creating a Journal

Northroot stores events in journal files (`.nrj` format). You can create a journal programmatically or use the CLI to inspect existing journals.

### Recording Events

Events are typically created programmatically using the Northroot Rust crates:

```rust
use northroot_store::JournalBackendWriter;
use northroot_core::events::AuthorizationEvent;

// Create a writer
let mut writer = JournalBackendWriter::create("events.nrj")?;

// Create and append an event
let event = create_authorization_event(...);
writer.append(&event)?;
writer.finish()?;
```

See [Integration Examples](integration-examples.md) for complete code samples.

### Listing Events

```bash
northroot list events.nrj
```

Filter by type:
```bash
northroot list events.nrj --type authorization
```

Filter by principal:
```bash
northroot list events.nrj --principal service:api
```

### Verifying Events

Verify all events in a journal:

```bash
northroot verify events.nrj
```

This checks:
- Event identity (`event_id` matches canonical bytes)
- Linkage consistency (executions reference valid authorizations)
- Signature validity (for attestation events)

### Getting a Specific Event

```bash
northroot get events.nrj <event_id>
```

Event IDs can be truncated if unique (first 5+ characters).

## Next Steps

- [CLI Guide](../crates/northroot-cli/README.md) - Complete command reference
- [Integration Examples](integration-examples.md) - Code samples for integration
- [Deployment Guide](../operator/deployment.md) - Production deployment
- [Core Specification](../reference/spec.md) - Protocol details

