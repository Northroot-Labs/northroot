# Agent Guidelines for Northroot

**Purpose**: Guidelines for AI agents working on the Northroot codebase  
**Audience**: AI coding assistants, automated tools, contributors  
**Status**: Active

---

## Core Principles

Before making any changes, understand:

1. **Neutrality**: The core proves what was allowed and what happened, not what should have happened
2. **Determinism**: All core logic must be deterministic and replayable offline
3. **Separation**: Core does not execute actions or make decisions
4. **Verifiability**: Receipts are the primary artifact for audit

See [GOVERNANCE.md](GOVERNANCE.md) for the complete project constitution.

---

## Project Structure

```
northroot/
├── crates/
│   ├── northroot-canonical/  # Canonicalization + event_id (core)
│   └── northroot-journal/    # Journal format (core)
├── apps/
│   └── northroot/            # CLI application (not in workspace)
├── schemas/
│   └── canonical/            # Canonical primitive schemas
├── docs/                      # Documentation
└── wip/                       # Experimental code (not core)
```

**Key constraint**: `apps/northroot/` is NOT in the workspace. Use `--manifest-path apps/northroot/Cargo.toml` or `cd apps/northroot` when building.

---

## Pre-Commit Requirements

A pre-commit hook (`.git/hooks/pre-commit`) runs automatically and blocks commits if:

- ❌ Code formatting fails (`cargo fmt --all --check`)
- ❌ Clippy warnings exist (`cargo clippy --all-targets --all-features -- -D warnings`)
- ❌ Tests fail (`cargo test --all --all-features`)
- ❌ Golden tests fail (`cargo test --package northroot-canonical --test golden`)
- ❌ Doctests fail (`cargo test --workspace --doc`)

**Always run checks locally before committing:**
```bash
# Fast checks
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features

# Or use justfile
just qa
```

---

## Code Quality Standards

### Formatting
- **Always** run `cargo fmt --all` before committing
- Formatting is enforced in CI and pre-commit hooks

### Linting
- Warnings are treated as errors (`-D warnings`)
- Fix or justify all clippy warnings
- Never suppress warnings without justification

### Documentation
- All public items must be documented (`#![deny(missing_docs)]`)
- Include examples in rustdoc comments
- Mark file I/O examples as `no_run` (they require actual files)

### Testing
- All public APIs must have tests
- Golden tests must pass (canonicalization stability)
- Doctests must compile and run
- Integration tests for journal operations

---

## What Agents Can Do

✅ **Allowed:**
- Fix bugs and improve code quality
- Add tests and documentation
- Refactor for clarity (preserving behavior)
- Update documentation to match code
- Implement planned features following GOVERNANCE.md
- Optimize performance (without changing semantics)

---

## What Agents Must Not Do

❌ **Prohibited:**
- Add agent planners, workflow engines, or decision recommenders
- Implement policy evaluation or enforcement in core
- Add AI provider dependencies or agent frameworks
- Make the core execute actions or modify external state
- Add "smart" defaults that influence decisions
- Break determinism or offline verification
- Violate neutrality principles

See [GOVERNANCE.md](GOVERNANCE.md) section 10 for explicit non-goals.

---

## Crate-Specific Constraints

### `northroot-canonical`
- See `crates/northroot-canonical/AGENTS.md` for detailed constraints
- Must remain schema-aligned with `schemas/canonical/v1/types.schema.json`
- Canonicalization must be deterministic and platform-independent

### `northroot-journal`
- Journal format is append-only and tamper-evident
- Must support strict and permissive read modes
- Frame encoding must be stable and portable

### `apps/northroot` (CLI)
- Package is NOT in workspace (use `--manifest-path` or `cd` into directory)
- Current commands: `canonicalize`, `event-id`, `list`, `verify`
- Commands `get`, `inspect`, `append`, `gen` are planned but not yet implemented

---

## Testing Requirements

### Before Committing
1. Run `cargo fmt --all` to format code
2. Run `cargo clippy --all-targets --all-features -- -D warnings`
3. Run `cargo test --all --all-features`
4. Run `cargo test --package northroot-canonical --test golden`
5. Run `cargo test --workspace --doc`

### Golden Tests
- Located in `crates/northroot-canonical/tests/golden.rs`
- Verify canonical byte stability
- Update with `UPDATE_GOLDEN=1 cargo test --test golden` if intentional

### Doctests
- All rustdoc examples must compile
- File I/O examples should use `no_run` attribute
- Examples must use actual APIs (no non-existent types)

---

## Documentation Standards

### Code Documentation
- Use rustdoc comments (`///` for items, `//!` for modules)
- Include examples in public API docs
- Cross-reference related types using `[`Type`]` syntax
- Link to reference docs: `[Format Reference](../../../docs/reference/format.md)`

### Markdown Documentation
- Keep API signatures in rustdoc (auto-generated)
- Manual docs focus on concepts, patterns, and usage
- Update docs when APIs change
- Verify all code examples compile

### Schema Documentation
- Canonical schemas in `schemas/canonical/v1/`
- Event schemas are domain-specific (not in core)
- Governance event examples in `wip/governance/` (not core)

---

## Common Pitfalls

### ❌ Don't Reference Non-Existent Crates
- `northroot-core` - does not exist
- `northroot-store` - experimental, in `wip/store/`
- `northroot-cli` - package name is `northroot`, path is `apps/northroot/`

### ❌ Don't Reference Non-Existent Types
- `Verifier`, `VerificationVerdict` - not in core
- `AuthorizationEvent`, `ExecutionEvent` - domain-specific, not in core
- `StoreWriter`, `StoreReader` - experimental, in `wip/store/`

### ✅ Use Actual Core APIs
- `northroot-canonical`: `Canonicalizer`, `compute_event_id`, `verify_event_id`
- `northroot-journal`: `JournalWriter`, `JournalReader`, `verify_event_id`
- See [API Contract](docs/developer/api-contract.md) for complete reference

---

## Building and Testing

### Workspace Crates
```bash
# Build all workspace crates
cargo build --workspace

# Test all workspace crates
cargo test --workspace

# Build specific crate
cargo build --package northroot-canonical
cargo build --package northroot-journal
```

### CLI Application (Not in Workspace)
```bash
# Build CLI
cd apps/northroot
cargo build --release

# Or from workspace root
cargo build --release --manifest-path apps/northroot/Cargo.toml

# Test CLI
cd apps/northroot
cargo test
```

---

## Documentation Generation

### Generate Rustdoc
```bash
# Generate docs for all crates
cargo doc --workspace --no-deps --open

# Generate docs for specific crate
cargo doc --package northroot-canonical --open
```

### Verify Doctests
```bash
# Run all doctests
cargo test --workspace --doc

# Run doctests for specific crate
cargo test --package northroot-canonical --doc
```

---

## Error Handling

### Common Errors

**"package ID specification did not match"**
- CLI package is not in workspace
- Use `--manifest-path apps/northroot/Cargo.toml` or `cd apps/northroot`

**"cannot find type"**
- Check if type exists in actual codebase
- Don't reference types from `wip/` directories in core code

**"doctest failed"**
- Check if example requires files (use `no_run`)
- Verify example uses actual APIs
- Ensure all imports are correct

---

## Key Documents

- [GOVERNANCE.md](GOVERNANCE.md) - Project constitution and principles
- [CORE_INVARIANTS.md](CORE_INVARIANTS.md) - Non-negotiable kernel constraints
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines
- [API Contract](docs/developer/api-contract.md) - Public API reference
- [Architecture](docs/developer/architecture.md) - System design
- [Testing Guide](docs/developer/testing.md) - Testing patterns

---

## Quick Reference

### Run All Checks
```bash
just qa  # Format, lint, test, golden
```

### Build Everything
```bash
cargo build --workspace
cd apps/northroot && cargo build --release
```

### Test Everything
```bash
cargo test --all --all-features
cargo test --workspace --doc
cd apps/northroot && cargo test
```

### Format Code
```bash
cargo fmt --all
```

### Fix Lints
```bash
cargo clippy --all-targets --all-features --fix -- -D warnings
```

---

## Remember

- **Neutrality is non-negotiable** - core does not decide or execute
- **Determinism is required** - all operations must be replayable offline
- **Documentation must match code** - update docs when APIs change
- **Tests must pass** - pre-commit hook blocks failures
- **Use actual APIs** - don't reference non-existent crates or types

When in doubt, check the actual codebase before making assumptions.

