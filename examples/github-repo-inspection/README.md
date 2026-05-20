# GitHub Repo Inspection Portable Evidence Bundle

This example is the first Northroot portable-evidence bundle. It records a
read-only repository inspection as local files that can be copied, reviewed, and
verified without a hosted service.

- JSON for actor, obligation, policy, receipt, and manifest objects
- JSONL for append-only execution events
- `sha256:<hex>` hashes over canonical JSON/JSONL evidence

The receipt binds the claim to the evidence files. The manifest records the
bundle contents. The event log stays append-oriented and can be replayed into an
`.nrj` journal when framed offline audit is justified.

`.nrj` is intentionally not checked in. Generated journals are derived artifacts;
the source bundle stays ordinary text.

## Bundle Contract

| File | Role |
| --- | --- |
| `actor.json` | Identifies the repo-inspection actor. |
| `obligation.json` | States the work item and required evidence. |
| `policy.json` | Constrains the work to read-only inspection. |
| `events.jsonl` | Records accepted obligation, tool invocation, and receipt generation events. |
| `receipt.json` | Binds evidence hashes to the completed inspection claim. |
| `manifest.json` | Lists bundle files and their canonical hashes. |

## Offline Verification

```bash
cargo run -p northroot -- validate examples/github-repo-inspection/actor.json
cargo run -p northroot -- validate examples/github-repo-inspection/obligation.json
cargo run -p northroot -- validate examples/github-repo-inspection/policy.json
cargo run -p northroot -- hash examples/github-repo-inspection/events.jsonl
cargo run -p northroot -- verify examples/github-repo-inspection/receipt.json \
  --base-dir examples/github-repo-inspection \
  --json
```

## Optional Journal Replay

The CLI appends one event object at a time. To inspect the journal path manually,
split the JSONL records into temporary files, append each event to a temporary
`.nrj`, then list and verify the journal:

```bash
cargo run -p northroot -- append /tmp/github-repo-inspection.nrj /tmp/event-1.json
cargo run -p northroot -- list /tmp/github-repo-inspection.nrj --json
cargo run -p northroot -- verify /tmp/github-repo-inspection.nrj --json --strict
```

The integration test builds that temporary journal from `events.jsonl` and
verifies it without committing the generated `.nrj`.
