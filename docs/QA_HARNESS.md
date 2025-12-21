_# Quality & Testing Harness

This document describes the QA harness for maintaining code quality and catching regressions. The harness is designed to be runnable by both humans and agents, with fast pre-push checks and scheduled deep checks.

## Quick Start

Run the fast QA suite before pushing:

```bash
just qa
```

This runs: format check, clippy, tests, and golden tests.

## Available Commands

### Fast Checks (Pre-Push)

- `just fmt` - Check code formatting
- `just lint` - Run clippy with warnings-as-errors
- `just test` - Run all tests
- `just golden` - Run golden tests for canonicalization
- `just qa` - Run all fast checks (fmt + lint + test + golden)

### Deep Checks (Scheduled/Nightly)

- `just coverage` - Generate coverage report (requires `cargo-llvm-cov`)
- `just audit` - Run security audits (requires `cargo-deny` and `cargo-audit`)
- `just miri` - Run Miri for undefined behavior detection (requires nightly)
- `just fuzz target <name>` - Run fuzz target (requires `cargo-fuzz`)
- `just docs` - Build documentation
- `just nightly` - Run full nightly suite (all checks)

## CI Workflows

### Fast Lane (`ci.yml`)

Runs on every push and pull request:

- Format check
- Clippy (warnings-as-errors)
- All tests
- Golden tests

**Expected runtime:** < 5 minutes

### Nightly Lane (`nightly.yml`)

Runs daily at 2 AM UTC (or manually via `workflow_dispatch`):

- Coverage report (HTML + LCOV artifacts)
- Security audits (`cargo-deny` + `cargo-audit`)
- Miri on critical crates (`northroot-core`, `northroot-canonical`)
- Fuzzing (time-boxed to 60s per target)

**Expected runtime:** 15-30 minutes

Artifacts are uploaded for:
- Coverage HTML (30 day retention)
- Coverage LCOV (30 day retention)
- Miri logs on failure (7 day retention)
- Fuzz findings on failure (30 day retention)

## Fuzz Targets

Fuzz targets are located in `crates/northroot-canonical/fuzz/`:

- `canonicalizer` - Fuzzes JSON canonicalization with arbitrary inputs
- `validation` - Fuzzes identifier and quantity parsing

To run locally:

```bash
cd crates/northroot-canonical
cargo fuzz run canonicalizer -- -max_total_time=60
cargo fuzz run validation -- -max_total_time=60
```

## Adding New Checks

1. **Fast checks**: Add command to `justfile`, include in `qa` target, add to `ci.yml`
2. **Deep checks**: Add command to `justfile`, add to `nightly.yml` with appropriate artifact uploads
3. **Fuzz targets**: Add new target in `crates/northroot-canonical/fuzz/fuzz_targets/`, update `nightly.yml` fuzz job

## For Agents

- Always run `just qa` before pushing code
- Reserve slow checks (miri, fuzz, coverage) for scheduled runs
- If a check fails, fix the issue before proceeding
- Warnings are treated as errors in CI (`-D warnings`)

## Runtime Budgets

- Fast checks: < 5 minutes (must pass on PR)
- Coverage: < 10 minutes (nightly only)
- Miri: < 15 minutes per crate (nightly only)
- Fuzz: 60 seconds per target (nightly only, continue-on-error)

## Dependencies

### Using DevContainer (Recommended)

All tools are pre-installed in the development container. See `.devcontainer/README.md` for setup.

### Manual Installation

Install required tools:

```bash
# Coverage
cargo install cargo-llvm-cov --locked

# Security
cargo install cargo-deny --locked
cargo install cargo-audit --locked

# Fuzzing
cargo install cargo-fuzz --locked

# Miri (nightly)
rustup toolchain install nightly
rustup component add miri --toolchain nightly

# Command runner
curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin
```

## Troubleshooting

- **Format fails**: Run `cargo fmt --all` to auto-fix
- **Clippy fails**: Review warnings, fix or allow with justification
- **Golden tests fail**: Review changes, update golden files if intentional
- **Miri fails**: Investigate undefined behavior, fix memory safety issues
- **Fuzz finds crash**: Minimize input, add regression test, fix bug

