# Work Ledger Extension Profile

Status: Incubating extension profile

Northroot's core journal remains neutral: it canonicalizes, stores, and verifies
events. The work ledger profile is an extension profile for accountable work
telemetry. It records observed work, runs, artifacts, and terminal states without
adding schedulers, policy engines, provider runtimes, or product semantics to
the trust kernel.

State recovery follows the binding [State Recovery Invariants v0](state-recovery-invariants-v0.md).

## Canonical Store

The canonical store is a `.nrj` journal, typically:

```text
.northroot/work-ledger.nrj
```

JSON projections under `.northroot/projections/` are derived state. They can be
rebuilt by replaying the journal and must not be treated as authoritative truth.

## Event Types

All work ledger events use the standard Northroot event envelope:

- `event_type`
- `event_version`
- `event_id`
- `occurred_at`
- `principal_id`
- `canonical_profile_id`

The v0 profile adds these low-domain event types:

- `work.observed`: a unit of work was discovered from telemetry.
- `run.observed`: a work execution attempt was observed.
- `artifact.observed`: an evidence pointer, command, tool output, message
  digest, receipt, PR/check URL, or similar artifact was observed.
- `run.completed`: a terminal successful state was observed.
- `run.blocked`: a terminal blocked state was observed.
- `snapshot.generated`: a content-addressed workspace snapshot artifact was
  generated from a covered journal position.
- `snapshot.restored`: a snapshot restore attempt replayed events after the
  covered journal position.
- `backup.receipt.observed`: a disaster-recovery backup receipt was observed.
  Backup receipts are pointers to external backup activity, not snapshots.

## Codex Ingestion

The CLI can ingest local Codex session JSONL:

```bash
northroot work ingest-codex \
  --sessions ~/.codex/sessions \
  --journal .northroot/work-ledger.nrj
```

The adapter records stable metadata and digests by default. It does not copy raw
transcripts into the canonical event body unless `--include-text` is set, and
text values are redacted before being hashed or emitted.

`--sync` fsyncs journal appends for stronger crash durability at lower ingest
throughput. Without `--sync`, ingestion is faster but recent appends may still
be in OS buffers if the machine loses power.

Malformed source JSON is counted and reported as warnings by default. Use
`--malformed-out .northroot/quarantine/codex-malformed.jsonl` to write a
redacted JSONL quarantine with source path, line, parse error, raw digest, and
bounded preview. Strict mode still fails on the first malformed source record.

## Projection

Replay creates a disposable summary:

```bash
northroot work project \
  --journal .northroot/work-ledger.nrj \
  --out .northroot/projections/work-ledger.json
```

The projection groups by `work_id` and reports derived status, run count,
observed actors, evidence refs, last activity, and terminal reason when present.
Projection output also records the source journal path, source journal digest,
event count, tip event id, and generation timestamp so a snapshot can prove what
derived view it captured.

## Snapshots

Snapshots are content-addressed recovery artifacts. They are derived from the
journal and may include selected projections or evidence indexes as accelerator
sections. Those sections are never authoritative; the event log remains the
system of record.

```bash
northroot work snapshot create \
  --journal .northroot/work-ledger.nrj \
  --out .northroot/snapshots/

northroot work snapshot verify \
  --snapshot snapshot:sha256:<digest> \
  --journal .northroot/work-ledger.nrj

northroot work snapshot restore \
  --snapshot snapshot:sha256:<digest> \
  --journal .northroot/work-ledger.nrj \
  --out .northroot/projections/work-ledger.json
```

Snapshot payloads live under `.northroot/snapshots/sha256/` next to their
manifests. The hash input is the Northroot canonical JSON bytes with the domain
separator `northroot:snapshot:v0\0`.

The default checkpoint policy is every 1,000 events or 1 hour, whichever occurs
first. Small workspaces may run daily; large agent fleets should shard by
workspace or stream before raising the threshold.

## Backups

Backups are separate disaster-recovery artifacts. Restic may back up journals,
snapshots, `.northroot` metadata, non-secret receipts, manifests, and vault
artifacts, but restore must still verify the journal, verify the latest
snapshot, replay the journal tail, and rebuild projections. Restic snapshots are
not operational checkpoints.

## Boundary

The work ledger records accountable work evidence. It does not decide whether
work is authorized, billable, product-complete, or semantically correct.
