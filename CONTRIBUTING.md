# Contributing to Northroot

Thank you for your interest in contributing to Northroot.

## Development Setup

### Prerequisites

- Rust from `rust-toolchain.toml`
- Python 3 for schema and package validators
- `just` command runner, optional but recommended

### Quick Start

```bash
git clone <repository-url>
cd northroot
bash scripts/dev_setup.sh
cargo build --workspace
cargo build --manifest-path apps/northroot/Cargo.toml
just qa
```

Run the full handoff gate before publishing or handing off a branch:

```bash
bash scripts/verify.sh
```

Codex users may still use the compatibility wrappers:

```bash
bash scripts/codex_setup.sh
bash scripts/codex_verify.sh
```

Codex is not the only supported development environment. New setup docs and CI
should use the neutral `scripts/dev_setup.sh`, `scripts/verify.sh`, and
`NORTHROOT_*` environment variables. See
[Environment and Setup](docs/developer/environment.md).

## Worktree Sync

Keep local `main` as a clean mirror of `origin/main`. Task work should happen on
branches created from fetched remote truth, not directly on `main`.

```bash
just fetch-refs
just sync-check
just sync-main
```

For new agent or automation work:

```bash
just fetch-refs
git worktree add -b codex/<task-name> <path> origin/main
```

The `codex/` branch prefix is a convention for Codex-authored work, not a
requirement for human contributors.

## Code Quality

Install hooks:

```bash
bash scripts/install_git_hooks.sh
```

`dev_setup.sh` installs hooks by default. Disable that with:

```bash
NORTHROOT_INSTALL_HOOKS=0 bash scripts/dev_setup.sh
```

The pre-commit hook runs:

- format check: `cargo fmt --all --check`
- clippy: `cargo clippy --all-targets --all-features -- -D warnings`
- workspace tests: `cargo test --all --all-features`
- canonical golden tests
- doctests
- schema validation

To bypass the hook, use `git commit --no-verify` only for intentional tooling or
enforcement changes after running equivalent checks.

## Pre-Push Checks

Fast gate:

```bash
just qa
```

Full gate:

```bash
bash scripts/verify.sh
```

The full gate includes CLI tests, release/readiness validators, fixture checks,
and promoted Python package checks.

## Branch Protection

`main` is protected:

- PR required; no direct pushes.
- Required checks include formatting, clippy, tests, golden tests, schemas, docs,
  CLI tests, and repository verification.
- High-risk paths may require human attestation; see
  [Signing Policy](docs/security/signing-policy.md).

## Project Principles

Northroot follows [GOVERNANCE.md](GOVERNANCE.md):

- **Neutrality**: prove what was allowed and what happened, not what should have happened.
- **Determinism**: verification must be deterministic and replayable offline.
- **Separation**: kernel crates do not execute actions or make decisions.
- **Verifiability**: Proof envelopes / verifiable events are the primary artifact for audit.

Any contribution that violates these principles will be rejected.

## Public vs Private Capability Boundary

Open capabilities may include sanitized schemas, validators, domain vocabulary,
and generic contracts. Private deployments, SaaS adapters, client workflows,
operational runbooks, secrets, receipts, and real machine custody state do not
belong in this public repo.

## Testing

```bash
cargo test --workspace
cargo test --manifest-path apps/northroot/Cargo.toml
PYTHONPATH=packages/northroot-durability/src python3 -m unittest discover packages/northroot-durability/tests
```

See [Testing Guide](docs/developer/testing.md) and [QA Harness](docs/qa/harness.md).

## Documentation

- User docs: `docs/user/`
- Developer docs: `docs/developer/`
- Reference docs: `docs/reference/`
- Security docs: `docs/security/`
- API surface: `docs/developer/api-contract.md`

Update docs whenever APIs, commands, crates, packages, or setup flows change.

## Commit Messages

Use concise conventional-style messages when practical:

```text
feat: add profile event validation
fix: reject duplicate record keys before import
docs: refresh setup guide
```

Commit authorship should identify whether work was human-only, agent-only, or
human plus agent. Human+agent commits should use the human's GitHub-linked
identity with a `Co-authored-by: Codex <codex@northroot.local>` trailer. Pure
agent commits should use `Codex <codex@northroot.local>` as the repo-local Git
identity. See [Git Authorship](docs/developer/git-authorship.md).
