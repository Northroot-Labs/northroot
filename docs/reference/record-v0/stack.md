# Northroot Record V0 Stack

Northroot Core V0 is a tiny record contract:

- `Record`
- `Statement`
- `Context`
- `Refs`
- `Payload`

Core validates only boring guarantees: known schema and role, required statement fields, event/attestation context, typed refs, canonical JSON payloads, content-derived IDs, append-only JSONL record segments, contiguous sequence numbers, and immutable segment seals.

Core does not evaluate policy, execute commands, interpret custody class meaning, or define application vocabularies.

## Layer Order

1. `northroot-record`: record schema, canonical hash, validator, canonical JSONL segment journal, segment seals.
2. `northroot-node`: node/workspace manifests and storage namespace conventions.
3. `northroot-governance`: policy records matched against command records.
4. `northroot-execution`: execution method registry validation.
5. `northroot-exchange`: constrained handoff/result record profile.
6. `northroot-clearlyops`: ClearlyOps/ClientOps vocabulary over records.

## Journal Format

Record segments are canonical JSON Lines files. Each line is:

```json
{"record":{ "...": "..." },"seq":1}
```

Seals are adjacent JSON files named `<segment>.seal.json` with a SHA-256 digest over the exact segment bytes. A sealed segment is immutable: appending to the segment changes the digest and fails verification.

The record stack does not use the custom `.nrj` frame format.

## Extension Rule

Profiles constrain records. Governance decides over records. Execution produces records. Applications specialize records. None of those layers change the core record shape.
