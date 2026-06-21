# Script Inventory

This document classifies repo-owned scripts. Neutral script names are preferred
for new documentation and CI; Codex-named scripts are compatibility wrappers.

## Setup and Verification

- `scripts/dev_setup.sh` - prepares a local, CI, container, or agent development environment.
- `scripts/verify.sh` - full handoff gate for workspace crates, CLI tests, schemas, fixtures, receipt boundaries, readiness, and promoted packages.
- `scripts/codex_setup.sh` - compatibility wrapper around `dev_setup.sh`.
- `scripts/codex_verify.sh` - compatibility wrapper around `verify.sh`.

## Release Gates

- `scripts/release-check.sh` - release-oriented verification gate.
- `scripts/v01_readiness.py` - machine-readable v0.1 readiness report.
- `scripts/validate_schemas.py` - JSON Schema syntax and fixture validation.
- `scripts/validate_kernel_boundary.py` - validates the stable kernel crate boundary and protected policy paths.
- `scripts/validate_nrj_fixtures.py` - `.nrj` fixture integrity validation.
- `scripts/validate_receipt_boundaries.py` - verifies that compatibility receipt language does not cross into core semantics.
- `scripts/validate_state_recovery_invariants.py` - validates state-recovery reference invariants.

## Support Scripts

- `scripts/fetch_remote_refs.sh` - safely refreshes remote branch refs without updating local branches, tags, or working trees.
- `scripts/check_worktree_sync.sh` - preflight guard for clean, current worktrees.
- `scripts/safe_sync_main.sh` - fast-forwards local `main` to `origin/main` only when clean and safe.
- `scripts/install_git_hooks.sh` - installs local pre-commit hooks.
- `scripts/benchmark_journal_formats.py` - keeps the custom journal decision observable by comparing `.nrj` against simpler alternatives.

## Excluded From Public Tooling

The public repo does not ship private deployment, SaaS adapter, scheduler,
secret-management, or client-operation scripts. Those belong in private packs,
private deployments, or the Northroot-Labs refinery.
