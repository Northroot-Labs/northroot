# Environment and Setup

Northroot supports ordinary local development, CI runners, containers, and agent
worktrees. Codex is a supported environment, not a required environment.

## Required Tools

- Rust toolchain from `rust-toolchain.toml`
- `cargo`, `rustc`, `rustfmt`, and `clippy`
- Python 3 for schema/package validators
- `git`
- `just` is optional but recommended

## Neutral Setup

Run:

```bash
bash scripts/dev_setup.sh
```

The setup script installs or verifies the Rust toolchain, baseline development
packages, optional `just`, git hooks, and locked Cargo dependencies where the
platform allows it.

Configuration:

```bash
# Skip local git-hook installation.
NORTHROOT_INSTALL_HOOKS=0 bash scripts/dev_setup.sh

# Install optional deep-check tools such as cargo-audit, cargo-deny, fuzz, cov.
NORTHROOT_DEEP_TOOLS=1 bash scripts/dev_setup.sh
```

Legacy compatibility variables are still honored:

```bash
NORTHROOT_CODEX_INSTALL_HOOKS=0 bash scripts/codex_setup.sh
NORTHROOT_CODEX_DEEP_TOOLS=1 bash scripts/codex_setup.sh
```

Prefer the `NORTHROOT_*` names for new docs, CI, and scripts.

## Verification

Run the full repository handoff gate with:

```bash
bash scripts/verify.sh
```

`verify.sh` checks Rust formatting, clippy, workspace tests, CLI tests, doctests,
schema and fixture validators, release readiness, journal benchmark smoke, and
promoted Python package tests.

The Codex wrapper remains available:

```bash
bash scripts/codex_verify.sh
```

## Worktree Hygiene

Use these helpers when working with local branches and agent worktrees:

```bash
just fetch-refs
just sync-check
just sync-main
```

They are normal repo helpers and do not require Codex.
