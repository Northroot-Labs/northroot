# Northroot Record V0 Stack

Northroot Core V0 is a tiny record contract:

- `Record`
- `Statement`
- `Context`
- `Refs`
- `Payload`

Core validates only boring guarantees: known schema and role, required statement fields, event/attestation context, typed refs, canonical JSON payloads, content-derived IDs, `.nrj`-backed append streams, contiguous sequence numbers, and immutable JSONL export segment seals.

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

1. `northroot-record`: record schema, canonical hash, validator, `.nrj` record stream wrapper, canonical JSONL segment export, segment seals.
2. `northroot-node`: node/workspace manifests and storage namespace conventions.
3. `northroot-governance`: policy records matched against command records.
4. `northroot-execution`: execution method registry validation.
5. `northroot-exchange`: constrained handoff/result record profile.
6. `northroot-ag`: sanitized agricultural profile example over records.

## Log And Segment Formats

Authoritative record streams use `.nrj`. Each record is wrapped in a kernel
event with `event_type = "northroot.record.appended"`, a contiguous `seq`, and
the validated `record` payload. The `.nrj` layer owns frame parsing, strict JSON
readback, event identity checks, truncation behavior, and append mechanics.

JSONL segments are the portable representation over that stream. They exist for
SDK fixtures, debugging, interchange, audit bundles, and boring line-oriented
tooling. Each line is:

```json
{"record":{ "...": "..." },"seq":1}
```

Seals are adjacent JSON files named `<segment>.seal.json` with a SHA-256 digest
over the exact segment bytes. Seal metadata is parsed with the same strict JSON
hygiene as segment entries, so duplicate object keys are rejected before any seal
field is trusted. When a segment is exported from `.nrj`, the seal also records
the source journal reference and exact source journal digest. A sealed segment is
immutable: appending to the segment changes the digest and fails verification.

JSONL is not a shadow source of truth when an `.nrj` source exists. It is either
a declared export from a source journal or a standalone interchange segment that
must be sealed and verified before becoming authoritative Northroot state.

The incubating CLI mirrors that boundary:

```sh
northroot record verify-nrj --journal records.nrj
northroot record import-jsonl --input records.jsonl --journal records.nrj
northroot record export-jsonl --journal records.nrj --out records-export.jsonl
northroot record verify-jsonl --input records-export.jsonl --require-source
```

Those commands are intentionally operator-facing wrappers over the kernel stream:
tooling can stay line-oriented while verification still flows through `.nrj`.
Import verifies the adjacent `<segment>.seal.json` before any record is appended
to the target stream.

## Extension Rule

Profiles constrain records. Governance decides over records. Execution produces records. Applications specialize records. None of those layers change the core record shape.
