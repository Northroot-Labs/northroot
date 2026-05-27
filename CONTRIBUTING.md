# Contributing to Northroot

Thank you for your interest in contributing to Northroot.

## Development Setup

### Prerequisites

- Rust (see `rust-toolchain.toml` for required version)
- `just` command runner (optional, for convenience commands)

### Quick Start

1. Clone the repository
2. Bootstrap the Codex/development environment:
   ```bash
   bash scripts/codex_setup.sh
   ```
3. Build the project:
   ```bash
   cargo build
   ```
4. Run tests:
   ```bash
   just test
   ```
5. Run QA checks:
   ```bash
   just qa
   ```

### Development Container

For a consistent development environment, use the devcontainer. See [.devcontainer/README.md](.devcontainer/README.md) for setup instructions. The devcontainer and Codex environment both use `scripts/codex_setup.sh`.

### Codex Environment

Repo-owned Codex setup lives in `.codex/environments/environment.toml` and calls `scripts/codex_setup.sh`. Use `scripts/codex_verify.sh` after setup or before handing off a branch to run the workspace checks, golden tests, doctests, schema validation, and the out-of-workspace CLI tests.

## Code Quality

### Pre-Commit Hooks

A pre-commit hook can be installed with:

```bash
bash scripts/install_git_hooks.sh
```

`scripts/codex_setup.sh` installs it by default. Set `NORTHROOT_CODEX_INSTALL_HOOKS=0` to skip hook installation in a disposable environment.

The hook runs critical CI checks before allowing commits:

- Format check (`cargo fmt --all --check`)
- Clippy linting (`cargo clippy --all-targets --all-features -- -D warnings`)
- All tests (`cargo test --all --all-features`)
- Golden tests (`cargo test --package northroot-canonical --test golden`)
- Documentation doctests (`cargo test --workspace --doc`)
- Schema validation (`python3 scripts/validate_schemas.py`)

If any check fails, the commit is blocked. Fix the issues and try again.

To bypass the hook (not recommended):
```bash
git commit --no-verify
```

### Pre-Push Checks

Always run the fast QA suite before pushing:

```bash
just qa
```

This runs:
- Format check (`just fmt`)
- Linting (`just lint`)
- Tests (`just test`)
- Golden tests (`just golden`)
- Schema validation (`just schema`)

Use `just codex-verify` for the full Codex handoff suite, including doctests and CLI tests.

### Coding Standards

- **Formatting**: Code must be formatted with `cargo fmt`
- **Linting**: Warnings are treated as errors (`-D warnings`)
- **Tests**: All public APIs must have tests
- **Documentation**: Public items must be documented

### Branch Protection

`main` is protected (GitHub Enterprise, tier A):
- PR required; no direct pushes.
- 1 approval required.
- Required status checks: `fmt`, `clippy`, `golden` (`test` runs on push only).
- Force push and branch deletion blocked.

### Code Review Process

1. Create a branch from `main`
2. Make changes following the project's principles (see [GOVERNANCE.md](GOVERNANCE.md))
3. Ensure all tests pass and QA checks succeed
4. Open a pull request with a clear description
5. Address review feedback
6. For Tier B changes (high-risk paths), see [Signing Policy](docs/security/signing-policy.md) for human attestation requirements

## Project Principles

Northroot follows strict principles defined in [GOVERNANCE.md](GOVERNANCE.md):

- **Neutrality**: We prove what was allowed and what happened, not what should have happened
- **Determinism**: All core logic must be deterministic and replayable offline
- **Separation**: Core does not execute actions or make decisions
- **Verifiability**: Receipts are the primary artifact for audit

Any contribution that violates these principles will be rejected.

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
cargo test --test integration
```

### Golden Tests

Golden tests verify canonicalization stability:

```bash
just golden
```

### Deep Checks (Nightly)

For comprehensive checks (coverage, security audits, fuzzing):

```bash
just nightly
```

See [docs/developer/testing.md](docs/developer/testing.md) for detailed testing guidelines.

## Documentation

- Code documentation: Use standard Rust doc comments
- User documentation: Update relevant files in `docs/user/`
- API documentation: Update `docs/developer/api-contract.md` for breaking changes
- Reference documentation: Update `docs/reference/` for specification changes

## Commit Messages

Follow conventional commit format:

```
<type>: <description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`

Example:
```
feat: add event filtering by principal ID

Implements PrincipalFilter for StoreReader to enable filtering
events by principal_id during replay.
```

## Questions?

- Check existing documentation in `docs/`
- Review [GOVERNANCE.md](GOVERNANCE.md) for project principles
- Open an issue for clarification
