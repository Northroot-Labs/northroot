# Releasing Northroot Crates

This document describes the versioning and release process for Northroot crates.

## Versioning Policy

We follow [Semantic Versioning](https://semver.org/) (SemVer):

- **MAJOR**: Breaking changes (API incompatibilities, canonicalization changes)
- **MINOR**: Additive changes (new features, backward-compatible)
- **PATCH**: Bug fixes, internal refactors (no API change)

## Crate Release Status

### Publishable Crates

- **northroot-receipts**: Published to crates.io
  - Breaking changes require MAJOR version bump
  - Canonicalization changes are breaking (MAJOR)
  - Schema additions are MINOR (additive)

### Private Crates (Future Publishable)

- **northroot-engine**: Private for now, publishable in future
  - Versioned independently
  - Breaking changes require MAJOR version bump

### Internal Crates

- **northroot-ops**: Internal, versioned independently
- **northroot-policy**: Internal, versioned independently
- **northroot-commons**: Internal, versioned independently

## Release Process

### For Publishable Crates (northroot-receipts)

1. **Update version** in `crates/northroot-receipts/Cargo.toml`
2. **Update CHANGELOG.md** (if maintained)
3. **Run all checks**: `cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings`
4. **Tag release**: `git tag -a v0.1.0 -m "Release v0.1.0"`
5. **Publish**: `cargo publish -p northroot-receipts`
6. **Push tags**: `git push --tags`

### For Private/Internal Crates

1. **Update version** in crate `Cargo.toml`
2. **Update dependent crates** if needed
3. **Run all checks**: `cargo build && cargo test`
4. **Tag release** (optional): `git tag -a northroot-engine-v0.1.0 -m "Release northroot-engine v0.1.0"`

## Workspace Versioning

The workspace uses a shared version in `[workspace.package]`, but individual crates can override if needed.

## Tagging Policy

- **Publishable crates**: Use crate name and version: `northroot-receipts-v0.1.0`
- **Workspace releases**: Use simple version: `v0.1.0` (for major milestones)

## Pre-Release Checklist

- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation is up to date
- [ ] CHANGELOG.md updated (if maintained)
- [ ] Version bumped in Cargo.toml
- [ ] Dependent crates updated if needed

## Using cargo-release

If using `cargo-release`:

```bash
# Release a specific crate
cargo release --package northroot-receipts --execute

# Release with workspace support
cargo release --workspace --execute
```

## Post-Release

- Update documentation links if version changed
- Announce release (if publishable) via appropriate channels
- Update any downstream dependencies

