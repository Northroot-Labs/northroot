# Getting Started with Northroot

This guide builds the current Northroot workspace and records a first journal.

## Install From Source

```bash
git clone <repository-url>
cd northroot
bash scripts/dev_setup.sh
cargo build --workspace
cargo build --release --manifest-path apps/northroot/Cargo.toml
```

The CLI binary is at `apps/northroot/target/release/northroot`.

Verify the CLI:

```bash
apps/northroot/target/release/northroot --help
```

## Public CLI Commands

Normal help shows the stable kernel commands:

```bash
northroot canonicalize input.json
northroot event-id event.json
northroot append events.nrj event.json
northroot read events.nrj
northroot verify events.nrj
```

Hidden support command groups exist for record streams, structural journals,
work-ledger dogfood, and bundle verification. Treat those as incubating operator
surfaces unless a reference document says otherwise.

## Record and Verify an Event

Create `event.json`:

```json
{
  "event_type": "test",
  "event_version": "1",
  "occurred_at": "2024-01-01T00:00:00Z",
  "principal_id": "service:example",
  "canonical_profile_id": "northroot-canonical-v1",
  "data": "example payload"
}
```

Compute its event ID:

```bash
northroot event-id event.json
```

Append and verify:

```bash
northroot append events.nrj event.json
northroot read events.nrj
northroot verify events.nrj
```

## Use The Rust Crates

```rust
use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
use northroot_journal::{JournalWriter, WriteOptions};
use serde_json::json;

let profile = ProfileId::parse("northroot-canonical-v1")?;
let canonicalizer = Canonicalizer::new(profile);
let mut event = json!({
    "event_type": "test",
    "event_version": "1",
    "occurred_at": "2024-01-01T00:00:00Z",
    "principal_id": "service:example",
    "canonical_profile_id": "northroot-canonical-v1"
});
let event_id = compute_event_id(&event, &canonicalizer)?;
event["event_id"] = serde_json::to_value(&event_id)?;
let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
writer.append_event(&event)?;
writer.finish()?;
```

## Next Steps

- [Integration Examples](integration-examples.md)
- [API Contract](../developer/api-contract.md)
- [Architecture](../developer/architecture.md)
- [Record V0 Stack](../reference/record-v0/stack.md)
