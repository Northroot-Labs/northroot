# Versioning Strategy

**Effective Date:** 2025-11-16  
**Status:** Active  
**Version:** 0.1.0

## Overview

Northroot follows [Semantic Versioning](https://semver.org/) (SemVer) for all releases starting with v0.1.0. This document defines the versioning strategy, release process, and commit conventions.

## Version Format

All versions follow SemVer: `MAJOR.MINOR.PATCH`

- **MAJOR** (x.0.0): Breaking changes (API incompatibilities, canonicalization changes)
- **MINOR** (0.x.0): Additive changes (new features, backward-compatible)
- **PATCH** (0.0.x): Bug fixes, internal refactors (no API change)

## Version Coordination

### v0.1.0 Strategy (Coordinated Release)

For the initial v0.1.0 release, all components share the same version:

- **Rust Crates**: All at `0.1.0` (via `workspace.package.version`)
- **Python SDK**: `0.1.0` (in `pyproject.toml`)
- **Single Release Tag**: `v0.1.0`

### Post-v0.1.0 Strategy (Independent Versioning)

After v0.1.0, components can version independently:

- **Rust Crates**: Can version independently (e.g., `northroot-receipts` at `0.1.1`, `northroot-engine` at `0.2.0`)
- **Python SDK**: Can version independently (e.g., `0.1.1`, `0.2.0`)
- **Release Tags**: Use format `v<component>-<version>` or `v<version>` for coordinated releases

## Version Files

### Single Source of Truth

- **Rust Workspace**: `Cargo.toml` → `[workspace.package].version`
- **Python SDK**: `sdk/python/northroot/pyproject.toml` → `[project].version`
- **Release Tag**: Git tag format `v0.1.0` (no prefix)

### VERSION File

The root `VERSION` file tracks the current workspace version for scripts and automation:

```
0.1.0
```

**Usage:**
```bash
# Read version
VERSION=$(cat VERSION)

# Update version
echo "0.1.1" > VERSION
```

## Commit Message Conventions

All commits **must** follow [Conventional Commits](https://www.conventionalcommits.org/) format for automatic changelog generation:

```
type(scope): description

Optional body with details.

BREAKING CHANGE: description (if applicable)
```

### Commit Types

| Type | Changelog Category | When to Use |
|------|-------------------|-------------|
| `feat` | **Added** | New features |
| `fix` | **Fixed** | Bug fixes |
| `refactor` | **Changed** | Code restructuring |
| `change` | **Changed** | Behavioral changes |
| `remove` | **Removed** | Removed features |
| `deprecate` | **Deprecated** | Deprecated features |
| `security` | **Security** | Security fixes |
| `docs` | *(ignored)* | Documentation only |
| `chore` | *(ignored)* | Maintenance tasks |
| `ci` | *(ignored)* | CI/CD changes |
| `test` | *(ignored)* | Test changes |

### Commit Scopes

Use crate/component names as scope:

- `receipts` → `northroot-receipts`
- `engine` → `northroot-engine`
- `storage` → `northroot-storage`
- `sdk` or `python` → Python SDK
- `docs` → Documentation
- `ci` → CI/CD (ignored in changelog)

### Examples

```bash
# New feature
feat(receipts): add CBOR canonicalization support

# Bug fix
fix(engine): correct hash computation in Merkle tree

# Breaking change
refactor(receipts): migrate from JCS to CBOR

BREAKING CHANGE: Receipt canonicalization now uses CBOR instead of JSON

# Python SDK feature
feat(sdk): add async API support for record_work

# Documentation (ignored)
docs(readme): update installation instructions
```

## Release Process

### Pre-Release Checklist

1. **All tests pass**: `cargo test --workspace --all-features`
2. **Linter passes**: `cargo clippy --workspace --all-features -- -D warnings`
3. **Format check**: `cargo fmt --all -- --check`
4. **Changelog updated**: `bash scripts/update-changelog.sh update`
5. **Version bumped**: Update `VERSION`, `Cargo.toml`, and `pyproject.toml`
6. **Documentation reviewed**: All public APIs documented
7. **Breaking changes documented**: If any, clearly marked

### Release Steps

1. **Update changelog**:
   ```bash
   bash scripts/update-changelog.sh update
   ```

2. **Review changelog**:
   ```bash
   git diff CHANGELOG.md
   ```

3. **Create release section**:
   ```bash
   bash scripts/update-changelog.sh release 0.1.0
   ```

4. **Update version files**:
   ```bash
   # Update VERSION file
   echo "0.1.0" > VERSION
   
   # Update Cargo.toml (if needed)
   # Update pyproject.toml (if needed)
   ```

5. **Commit release preparation**:
   ```bash
   git add CHANGELOG.md VERSION
   git commit -m "chore: prepare release 0.1.0"
   ```

6. **Tag release**:
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   ```

7. **Push tags**:
   ```bash
   git push origin main --tags
   ```

8. **Publish** (if applicable):
   - PyPI: GitHub Actions workflow will auto-publish on tag
   - Crates.io: Manual publish (not yet implemented)

### Post-Release

1. **Create GitHub release** (if using GitHub):
   - Go to: https://github.com/Northroot-Labs/Northroot/releases/new
   - Tag: `v0.1.0`
   - Title: `v0.1.0`
   - Description: Copy from `CHANGELOG.md`

2. **Update version for next development**:
   ```bash
   # For patch release: 0.1.0 → 0.1.1
   # For minor release: 0.1.0 → 0.2.0
   # For major release: 0.1.0 → 1.0.0
   ```

## Version Bump Rules

### Patch Release (0.1.0 → 0.1.1)

**When:**
- Bug fixes
- Internal refactors (no API change)
- Documentation updates
- Test improvements

**Example commits:**
- `fix(engine): correct hash computation`
- `refactor(storage): simplify SQLite adapter`
- `docs(readme): update installation instructions`

### Minor Release (0.1.0 → 0.2.0)

**When:**
- New features (backward-compatible)
- New APIs
- Enhanced functionality

**Example commits:**
- `feat(receipts): add JSON adapter support`
- `feat(sdk): add async API methods`
- `feat(storage): add query filtering`

### Major Release (0.1.0 → 1.0.0)

**When:**
- Breaking API changes
- Canonicalization changes
- Removal of deprecated features

**Example commits:**
- `refactor(receipts): migrate from JCS to CBOR` (with `BREAKING CHANGE:`)
- `remove(engine): remove deprecated delta API`

## Changelog Management

### Automatic Generation

The changelog is automatically generated from commit messages:

```bash
# Update with commits since last release
bash scripts/update-changelog.sh update

# Create release section
bash scripts/update-changelog.sh release 0.1.0
```

### Manual Entries

If needed, manually edit `CHANGELOG.md` under `[Unreleased]`:

```markdown
## [Unreleased]

### Added
- Your manual entry here
```

### Ignored Commits

These commit types are automatically excluded:

- `chore:`` - Maintenance tasks
- `ci:` - CI/CD changes
- `test:` - Test changes
- `docs(changelog):` - Changelog updates
- `Merge ...` - Merge commits
- `Revert ...` - Revert commits

## Git Tag Format

### Release Tags

- **Format**: `v0.1.0` (no prefix, just version)
- **Annotated tags**: Use `-a` flag for release notes
- **Message**: `Release v0.1.0` or copy from changelog

### Pre-Release Tags (Optional)

- **Development**: `v0.1.0-dev.1`, `v0.1.0-dev.2`
- **Alpha**: `v0.1.0-alpha.1`
- **Release Candidate**: `v0.1.0-rc.1`

## Version Synchronization

### Rust Workspace

All crates inherit from `[workspace.package].version`:

```toml
[workspace.package]
version = "0.1.0"  # Single source of truth
```

Individual crates can override if needed:

```toml
[package]
version = "0.2.0"  # Override for independent versioning
```

### Python SDK

Independent version in `pyproject.toml`:

```toml
[project]
version = "0.1.0"
```

## Best Practices

1. **Always use conventional commits**: Enables automatic changelog generation
2. **One logical change per commit**: Makes history clean and reviewable
3. **Be descriptive**: Write clear commit messages
4. **Mark breaking changes**: Always include `BREAKING CHANGE:` footer
5. **Update changelog before release**: Review all entries
6. **Tag releases**: Always create annotated tags for releases
7. **Keep versions in sync**: For coordinated releases, ensure all components match

## Related Documentation

- [Changelog Guide](./CHANGELOG_GUIDE.md)
- [Releasing Guide](../RELEASING.md)
- [Commit Automation Rules](../../.cursor/rules/commit-automation.mdc)
- [Contributing Guide](../../CONTRIBUTING.md)

