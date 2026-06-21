# Journal Format Complexity Benchmarks

Northroot keeps `.nrj` as the canonical event journal and uses JSONL as an
adapter, import, export, and debugging format. The benchmark harness in
`scripts/benchmark_journal_formats.py` makes that decision reproducible without
turning small timing variance into a CI failure.

The benchmark asks which format reduces total system complexity for intended
Northroot workloads. Performance is measured, but it is not the primary
criterion.

## Run

```sh
python3 scripts/benchmark_journal_formats.py
```

By default the script writes isolated artifacts under
`/tmp/northroot-format-complexity-bench`:

- `mixed-workload.jsonl`
- `mixed-workload.nrj`
- truncated variants
- policy-failure variants
- `format-complexity-report.json`

Use a smaller smoke run when needed:

```sh
python3 scripts/benchmark_journal_formats.py --scale 0.1
```

The smoke run is also part of `scripts/verify.sh`. It checks record
counts, workload class counts, NRJ CLI verification, and truncation recovery
signals. It does not fail on byte-size or timing differences.

Use an explicit output directory for archival comparison:

```sh
python3 scripts/benchmark_journal_formats.py --out /tmp/northroot-format-proof
```

The script builds `apps/northroot` if the debug binary does not already exist.

## Workloads

The generated stream covers representative Northroot event-log pressure:

- Codex session/run metadata
- work-ledger observations
- tool invocation records
- small tool-output evidence records
- larger tool-output evidence records
- `snapshot.generated` pointer/provenance events
- a small number of full snapshot-payload stress records

The full snapshot-payload stress records are intentionally labeled
`snapshot_payload_stress_not_recommended`. Normal snapshots remain
content-addressed JSON artifacts outside the journal; only `snapshot.generated`
belongs in the canonical event stream.

## What The Harness Proves

The harness asserts:

- JSONL and NRJ contain the same number of records.
- JSONL and NRJ preserve the same workload class counts.
- Generated NRJ is readable by `northroot verify`.
- Truncated JSONL and truncated NRJ both preserve all complete records before
  the damaged tail.
- NRJ reports truncation at a precise frame boundary with expected and actual
  body length.

The harness reports, but does not assert, timing and byte-size measurements.
Those numbers are useful for trend comparison, not correctness.

## What To Compare

The primary comparison is format complexity:

- recovery ambiguity
- validation burden
- accidental misuse risk
- adapter complexity
- schema and version clarity
- corruption locality
- inspection and debugging ergonomics

Performance still matters, but a small throughput difference is less important
than whether every consumer must rediscover durable journal policy.

## Current Interpretation

JSONL can support early adapter and exchange flows. It is easy to inspect,
stream, import, and export. If JSONL becomes the canonical journal, Northroot
must define additional policies for header/version metadata, partial final
records, comments and blank lines, unknown record envelopes, and the boundary
between canonical event streams and loose exchange files.

NRJ is cleaner for canonical event storage because the file itself carries the
durability boundary: magic bytes, version, flags, explicit frame kind, explicit
payload length, and strict verifier compatibility.

The intended split remains:

- `.nrj`: canonical framed event journal
- JSONL: adapter/import/export/debug format
- snapshot JSON: content-addressed recovery artifact
- Restic or backup bundle: disaster-recovery unit
