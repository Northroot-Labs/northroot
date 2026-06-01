# Script Inventory

This document classifies repo-owned scripts for the v0.1 release surface.

## Release Gates

These scripts are commit-worthy and part of the v0.1 readiness path:

- `scripts/codex_setup.sh` - prepares a deterministic local development environment.
- `scripts/codex_verify.sh` - full handoff gate for workspace crates, CLI tests, schemas, fixtures, receipt boundaries, and readiness.
- `scripts/release-check.sh` - release-oriented verification gate.
- `scripts/v01_readiness.py` - machine-readable v0.1 readiness report.
- `scripts/validate_schemas.py` - JSON Schema syntax and fixture validation.
- `scripts/validate_nrj_fixtures.py` - `.nrj` fixture integrity validation.
- `scripts/validate_receipt_boundaries.py` - verifies that compatibility receipt language does not cross into core semantics.
- `scripts/validate_state_recovery_invariants.py` - validates state-recovery reference invariants.

## Support Scripts

These scripts are repo-local helpers, not public API:

- `scripts/install_git_hooks.sh` - installs local pre-commit hooks.
- `scripts/benchmark_journal_formats.py` - keeps the custom journal decision observable by comparing `.nrj` against simpler alternatives.

## Excluded From v0.1

The v0.1 kernel does not ship deployment, runtime, scheduler, agent, or application-service scripts. Those belong in consuming repositories or future profile/runtime projects.
