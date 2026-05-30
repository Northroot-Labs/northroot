# Northroot State Recovery Architecture v0

Status: Proposal

Northroot state recovery keeps four layers separate:

| Layer | Purpose | Authority |
| --- | --- | --- |
| Event log | Append-only source of truth | Canonical `.nrj` journal |
| Projection | Fast operational view | Rebuildable derived state |
| Snapshot / checkpoint | Fast recovery from a covered journal position | Content-addressed artifact |
| Backup | Disaster recovery | External encrypted backup repository |

Snapshots are not backups. A snapshot accelerates replay inside a live
workspace. A backup restores bytes after local loss or corruption.

## Event Log

The canonical store is the Northroot Journal (`.nrj`). Events use the core
event envelope and domain-specific extension fields. Required guarantees are:

- append-only writes; corrections are new events;
- deterministic event identity from canonical JSON;
- optional `prev_event_id` hash-chain linkage where a profile needs it;
- idempotent ingestion through stable event ids;
- offline verification without network calls.

The v0 local storage engine is `.nrj` on the filesystem. SQLite may index
operational reads, but indexes are rebuildable. PostgreSQL and object storage
are downstream adapters for hosted or enterprise deployments after the local
artifact model is stable.

## Projections

The first-class v0 projection is the work-ledger workspace summary. It is
materialized at `.northroot/projections/work-ledger.json` and records:

- source journal path and digest;
- event count and tip event id;
- generation timestamp;
- derived work status, run counts, actors, evidence refs, and terminal reason.

Projections are disposable. Rebuild procedure: verify the journal, replay every
event in order, write the projection, and compare source metadata when restoring
from a snapshot.

## Snapshots

Snapshots are content-addressed artifacts under
`.northroot/snapshots/sha256/`. The payload captures derived workspace state and
may include selected projection and evidence-index accelerator sections. Those
sections are not authoritative.

Digest input is:

```text
northroot:snapshot:v0\0 || canonical_json(snapshot_payload)
```

The manifest records workspace id, covered journal ref, covered event count,
covered tip event id, covered journal byte offset, payload digest, projection
digests, redaction profile, generator version, and lineage.

Restore procedure:

1. Verify manifest and payload digests.
2. Confirm the covered tip event id matches the journal at the covered event
   count.
3. Load the derived state from the snapshot.
4. Replay journal events after the covered event count.
5. Write the rebuilt projection.

Default generation policy is every 1,000 events or 1 hour, whichever occurs
first. Small workspaces can snapshot daily or every 1,000 events. Medium
workspaces use the default. Large agent fleets should shard by workspace or
stream before raising the event threshold to 10,000.

## Backups

Restic is the baseline disaster-recovery engine. Backup scope includes `.nrj`
journals, snapshot artifacts, `.northroot` metadata, non-secret receipts,
manifests, and object/vault artifacts. Backup scope excludes provider tokens,
browser cookies, private keys, restic passwords, credential values, and other
secret material.

Default retention is:

- 24 hourly;
- 30 daily;
- 12 monthly.

Disaster recovery drill:

1. Restore workspace bytes from Restic.
2. Verify journals.
3. Verify the latest snapshot.
4. Replay from snapshot to journal tip.
5. Rebuild projections.

No distributed broker is part of v0.
