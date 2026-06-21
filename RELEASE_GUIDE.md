# Northroot v0.1 Release Guide

**Status**: v0.1.0 release candidate
**Audience**: release engineers and downstream integrators
**Purpose**: build, verify, and tag the first stable Northroot kernel release while keeping incubating layers explicit

## Release Meaning

`v0.1.0` is the first stable Northroot kernel release. The stable kernel is
small, deterministic, offline-verifiable, and suitable for downstream adoption.
The repository may contain additional incubating crates and promoted packages,
but those do not expand the stable kernel contract unless this guide and the
stability contract say so.

Stable kernel in v0.1:

- `northroot-canonical`: deterministic canonicalization and event identity.
- `northroot-journal`: append-only `.nrj` journal read/write/verify behavior.
- Public CLI commands: `canonicalize`, `event-id`, `append`, `read`, `verify`.
- MIT project license.

Incubating or promoted-but-not-kernel in v0.1:

- `northroot-record`, JSONL segment import/export, and record stream CLI helpers.
- `northroot-node`, `northroot-state-eval`, `northroot-governance`,
  `northroot-execution`, `northroot-exchange`, and `northroot-ag`.
- hidden CLI command groups: `record`, `journal`, `work`, and `verify-bundle`.
- `northroot-durability` Python package.

Excluded from public release artifacts:

- private deployments and SaaS adapters;
- client workflows, operational runbooks, and real receipts;
- secrets, credentials, local machine custody, and raw tool state;
- generated website output and vendored web dependencies.

## Required Checks

Run the standard repository gate:

```bash
bash scripts/verify.sh
```

Codex compatibility wrapper:

```bash
bash scripts/codex_verify.sh
```

Run the release gate:

```bash
bash scripts/release-check.sh
```

Run readiness directly when diagnosing release blockers:

```bash
python3 scripts/v01_readiness.py --json
```

The readiness report fails on version drift, stale release claims, missing stable
CLI commands, missing required reference docs, fixture/schema validation
failures, receipt-boundary regressions, and fewer than two passing downstream
adoption reports.

## Current CLI Surface

Normal help exposes stable kernel commands:

- `canonicalize`
- `event-id`
- `append`
- `read`
- `verify`

Hidden support commands are checked where relevant for dogfood and release
continuity, but they are not stable kernel command semantics:

- `verify-bundle`
- `journal`
- `record`
- `work`

Build the standalone CLI with:

```bash
cargo build --release --manifest-path apps/northroot/Cargo.toml
```

## Adoption Evidence

Downstream adoption reports are imported under `docs/adoption/*.json`. Northroot
readiness reads only those reports, not downstream source data.

Each passing report must include repository name, check name, journal path,
event counts, kernel/profile validity counts, invalid count, projection rebuild
status, and `passed: true`.

The v0.1 release gate requires at least two passing reports.

## Publishing Preflight

Before publishing, recheck registry/package ownership and existing versions for
all release candidates. At minimum:

- Rust crates: `northroot-canonical`, `northroot-journal`.
- Any incubating crate intentionally published with the release.
- Python packages intentionally published, such as `northroot-durability`.

Treat registry availability as point-in-time information and recheck immediately
before release.

## Release Flow

1. Open a PR with the release changes.
2. Confirm `bash scripts/verify.sh` passes.
3. Confirm `bash scripts/release-check.sh` passes.
4. Merge to `main`.
5. Create a signed release tag on the merged commit:

```bash
git tag -s v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

6. Confirm GitHub Actions publishes the intended artifacts.

## Version Bump Rules After v0.1.0

- Patch: bug fixes, documentation corrections, fixture additions, compatible CLI hardening.
- Minor: additive APIs, optional fields, additional incubating commands, verifier reports.
- Major: canonicalization rule changes, journal format changes, public API removals, breaking CLI behavior.

Canonicalization and journal compatibility are kernel contracts. Incubating
profile and package layers can move faster, but they must not alter kernel event
identity or `.nrj` invariants.
