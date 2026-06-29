# Agent Guidelines for Northroot

**Purpose**: Guidelines for AI agents and automated tools working on Northroot.
**Audience**: AI coding assistants, local automation, CI helpers, and human reviewers.
**Status**: Active

## Core Principles

Before making changes, understand:

1. **Neutrality**: Northroot proves what was allowed and what happened, not what should have happened.
2. **Determinism**: Verification logic must be deterministic and replayable offline.
3. **Separation**: Kernel crates do not execute actions or make decisions.
4. **Verifiability**: Proof envelopes / verifiable events are the primary artifact for audit.

See [GOVERNANCE.md](GOVERNANCE.md) for the complete constitution.

## Current Repository Shape

```text
crates/northroot-canonical   stable kernel: canonical bytes and event IDs
crates/northroot-journal     stable kernel: .nrj append/read/verify
crates/northroot-record      neutral record contract and .nrj record streams
crates/northroot-node        node/workspace manifest conventions
crates/northroot-state-eval  product-agnostic state/eval primitives
crates/northroot-governance  policy-record matching over records
crates/northroot-execution   execution method registry contracts
crates/northroot-exchange    constrained handoff/result profile
crates/northroot-ag          sanitized ag-domain example over records
packages/northroot-custody     Python package: northroot.custody steward/custody service
packages/northroot-durability  legacy Python compatibility helpers
apps/northroot               standalone CLI; not a workspace member
```

`apps/northroot` is intentionally outside the Cargo workspace. Build or test it
with `--manifest-path apps/northroot/Cargo.toml` or by changing into that
directory.

## Environment Entry Points

Use neutral scripts by default:

```bash
bash scripts/dev_setup.sh
bash scripts/verify.sh
```

Codex compatibility wrappers still exist:

```bash
bash scripts/codex_setup.sh
bash scripts/codex_verify.sh
```

Do not make new tooling depend on Codex-specific paths or environment names
unless the feature is explicitly a Codex importer. Prefer `NORTHROOT_*`
configuration names; legacy `NORTHROOT_CODEX_*` names are compatibility only.

## Before Editing

1. Confirm the current branch and working tree with `git status --short --branch`.
2. Pull or rebase only when the worktree is clean and the user asked for it.
3. Read the relevant crate docs and tests before changing behavior.
4. Keep public/open code sanitized: no client data, local paths, real receipts,
   secrets, raw tool state, or deployment-specific adapter details.

## Required Checks

For normal changes:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --manifest-path apps/northroot/Cargo.toml
python3 scripts/validate_schemas.py
python3 scripts/validate_kernel_boundary.py
```

For handoff or commits touching verification, schemas, records, CLI behavior, or
promoted packages:

```bash
bash scripts/verify.sh
```

The pre-commit hook runs formatting, clippy, workspace tests, golden tests,
doctests, and schema validation. Do not bypass it unless the change is an
intentional enforcement/tooling update and you have run equivalent checks.

## Allowed Work

- Fix bugs and improve code quality.
- Add tests, fixtures, and documentation that match current code.
- Refactor within existing layer boundaries.
- Add sanitized open capabilities, schemas, and validators.
- Improve setup and verification as environment-neutral tooling.

## Prohibited Work

- Add AI provider dependencies or agent frameworks to the kernel.
- Make kernel crates execute workflows, pick outcomes, or enforce product policy.
- Put private deployments, SaaS adapters, client workflows, secrets, receipts, or
  real machine custody data in this public repo.
- Collapse profile/domain semantics into `northroot-canonical` or
  `northroot-journal`.
- Invent crates or public APIs in docs without implementing them.

## Layer Constraints

### Stable Kernel

Machine-enforced boundary: `kernel-boundary.json` plus `scripts/validate_kernel_boundary.py` define the stable kernel crates and fail if kernel crates depend upward.

- `northroot-canonical`: deterministic canonical JSON, strict parsing, digest and identifier types.
- `northroot-journal`: portable `.nrj` frame format and event identity verification.

These crates must stay deterministic, offline-capable, and free of domain semantics.

### Record/Substrate Layers

- `northroot-record`: validates neutral records and wraps them in `.nrj` streams.
- `northroot-node`: validates node/workspace manifest conventions.
- `northroot-state-eval`: provides product-agnostic state/eval shapes.

These layers may structure higher-level facts but must not own private product authority.

### Capability/Profile Layers

- `northroot-governance`, `northroot-execution`, `northroot-exchange`, and
  `northroot-ag` are sanitized capability/profile examples over records.
- Keep real SaaS adapters and client deployments private.

### Promoted Packages

- `packages/northroot-custody` exposes `northroot.custody` for public-safe
  custody contracts, delegated snapshot plans, retention gates, restore
  verification, run summaries, and the steward helper layer used by
  `nr steward`.
- `packages/northroot-durability` exposes `northroot.durability` as a legacy
  compatibility boundary for public/private artifact checks and simple copy
  manifests. Do not add new steward, restore, retention, scheduler, or secret
  provider vocabulary there.
- Python package tests are not covered by Cargo; run them with:

```bash
PYTHONPATH=packages/northroot-custody/src python3 -m unittest discover packages/northroot-custody/tests
PYTHONPATH=packages/northroot-durability/src python3 -m unittest discover packages/northroot-durability/tests
```

## Common Pitfalls

- `northroot-core` does not currently exist. Do not document it as real.
- `apps/northroot` is not a workspace member.
- Normal CLI help shows only public kernel commands; hidden support commands are incubating.
- `northroot-ag` is sanitized open domain tooling. A real client or SaaS adapter is private.
- Codex session ingestion is one importer, not the only supported environment model.

## Useful Commands

```bash
cargo build --workspace
cargo test --workspace
cargo test --manifest-path apps/northroot/Cargo.toml
cargo doc --workspace --no-deps
just qa
bash scripts/verify.sh
```
