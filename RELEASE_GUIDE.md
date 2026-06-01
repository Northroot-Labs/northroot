# Northroot v0.1 Release Guide

**Status**: v0.1.0 stable kernel release candidate
**Audience**: release engineers and downstream integrators
**Purpose**: build, verify, and tag the first stable Northroot verifiable-event kernel release

---

## Release Meaning

`v0.1.0` is the first stable Northroot kernel release. It means the core substrate is small, deterministic, offline-verifiable, and dogfooded by at least two downstream repositories.

Historical note: earlier local docs and experimental branches used later-looking
version labels during alpha work. Those labels are stale; `v0.1.0` is the
canonical release identity for this candidate.

Stable in v0.1:
- `northroot-canonical`: deterministic canonicalization and event identity.
- `northroot-journal`: append-only `.nrj` journal read/write/verify behavior.
- Structural segmented-journal manifests and checkpoints.
- CLI kernel commands: `canonicalize`, `event-id`, `append`, `list`, `verify`, `verify-bundle`, `journal`.
- CLI work-ledger dogfood command group: `work` (incubating).
- Release/readiness docs and repeatable checks.
- MIT project license.

Incubating in v0.1:
- Work-ledger profile semantics.
- Profile verifiers and projection contracts.
- Snapshot/state-recovery workflows.
- Work-ledger CLI commands and semantics.
- Downstream runtime, scheduler, policy, vault, or product semantics.

The kernel may preserve and verify profile-bearing events. The kernel must not decide profile meaning.

Excluded from v0.1:
- Generated website output and vendored web dependencies.
- `wip/` experimental store, governance, and agent-domain code.
- API deployment, Kubernetes, secrets-management, runtime, scheduler, and product-operation docs.
- Dual-license release metadata.

---

## Required Checks

Run the standard repository checks:

```bash
scripts/codex_verify.sh
```

Run the release gate:

```bash
scripts/release-check.sh
```

Run the machine-readable v0.1 readiness report directly when diagnosing release blockers:

```bash
python3 scripts/v01_readiness.py --json
```

The readiness report fails on:
- version drift away from `0.1.0` in package metadata or lockfiles,
- stale public release claims from older major-release or alpha-version plans,
- missing stable CLI commands,
- missing required reference docs,
- missing MIT license metadata,
- leftover non-core release surfaces such as `wip/`, generated `site/`, deployment docs, or old extension-doc links,
- schema, fixture, or receipt-boundary validation failures,
- fewer than two passing downstream adoption reports.

---

## Current CLI Surface

The release gate must confirm these commands exist:

- `canonicalize`
- `event-id`
- `append`
- `list`
- `verify`
- `verify-bundle`
- `journal`
- `work`

`work` is checked for dogfood continuity, not as stable kernel semantics.

The CLI app lives at `apps/northroot/` and is intentionally not in the workspace. Build it with:

```bash
cargo build --release --manifest-path apps/northroot/Cargo.toml
```

---

## Adoption Evidence

Downstream adoption reports are imported under `docs/adoption/*.json`. Northroot readiness reads only those reports, not downstream source data.

Each passing report must include:
- repository name,
- check name,
- journal path,
- total event count,
- kernel-valid event count,
- profile-valid event count,
- invalid event count,
- projection rebuild status.

The v0.1 release gate requires at least two passing reports.

## crates.io Preflight

Before publishing, recheck crates.io ownership and existing versions for:

- `northroot`
- `northroot-canonical`
- `northroot-journal`

As of the v0.1 cleanup pass, these crate names returned 404 from the crates.io
API, so there was no existing published package cleanup to perform. Treat that
as a point-in-time preflight result and recheck immediately before release.

---

## Release Flow

1. Open a PR with the release changes.
2. Confirm `scripts/codex_verify.sh` passes.
3. Confirm `scripts/release-check.sh` passes.
4. Merge to `main`.
5. Create a signed release tag on the merged commit:

```bash
git tag -s v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

6. Confirm GitHub Actions publishes the release artifacts.

---

## Version Bump Rules After v0.1.0

- Patch: bug fixes, documentation corrections, fixture additions, compatible CLI hardening.
- Minor: additive APIs, new optional fields, additional CLI commands, new verifier reports.
- Major: canonicalization rule changes, journal format changes, removal of public APIs, breaking CLI behavior.

Canonicalization and journal compatibility are kernel contracts. Work-ledger profile changes can move faster while they remain marked incubating, but they must not alter kernel event identity or `.nrj` invariants.
