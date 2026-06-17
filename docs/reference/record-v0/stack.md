# Northroot Record V0 Stack

Northroot Core V0 is a tiny record contract:

- `Record`
- `Statement`
- `Context`
- `Refs`
- `Payload`

Core validates only boring guarantees: known schema and role, required statement fields, event/attestation context, typed refs, canonical JSON payloads, content-derived IDs, append-only JSONL record segments, contiguous sequence numbers, and immutable segment seals.

Core does not evaluate policy, execute commands, interpret custody class meaning, or define application vocabularies.

## Core Grammar

Record IDs are public content identifiers in lowercase hex form:

```text
sha256:<64 lowercase hex characters>
```

The structured digest shape from `northroot-canonical` can still be useful in richer metadata, but `record.id` and record refs use the compact string form for stable logs, manifests, CLI output, and cross-system references.

Typed refs use explicit prefixes:

```text
resource:<segment>[:<segment>]*
attestation:<segment>[:<segment>]*
event:sha256:<64 lowercase hex characters>
```

Name segments are lowercase ASCII alphanumeric strings with optional `_` or `-` after the first character. Predicate and profile IDs are dot-separated lowercase names. Profile IDs end in a version segment such as `v0`.

Records may declare layered profiles with `profiles`, for example:

```json
["northroot.exchange.v0"]
```

Core validates profile ID grammar only. Profile resolution and interpretation belong above core.

Timestamps are UTC second timestamps:

```text
YYYY-MM-DDTHH:MM:SSZ
```

The validator rejects impossible calendar dates and out-of-range clock values. Leap seconds are not accepted in Core V0.

Payload is opaque JSON owned by profiles/applications. Core requires it to be canonicalizable by `northroot-canonical`; callers that parse untrusted JSON should use strict parsing so duplicate object keys are rejected before conversion to `serde_json::Value`.

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
