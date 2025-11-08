# Releasing Northroot Crates

This document describes the versioning and release process for Northroot crates.

## Current Status: Pre-v0.1.0 Development

**⚠️ Important:** We are currently in active development and building towards a stable **v0.1.0** release. The codebase is **not finalized** and APIs may change without notice.

### Versioning Cadence

- **Current phase:** Pre-v0.1.0 (development/alpha)
- **Target:** Stable v0.1.0 release (not yet reached)
- **Stability:** APIs are subject to change until v0.1.0 is released
- **Breaking changes:** Expected and acceptable during pre-v0.1.0 development

### Pre-v0.1.0 Guidelines

During this phase:

1. **No stability guarantees** - APIs may change between commits
2. **Breaking changes are acceptable** - No need for MAJOR version bumps
3. **Rapid iteration** - Focus on getting the API right before v0.1.0
4. **Documentation may lag** - Code changes faster than docs during development
5. **Test vectors may change** - Golden vectors may be regenerated as we refine canonicalization

### Post-v0.1.0

Once v0.1.0 is released:

- **Stability commitment begins** - Breaking changes require MAJOR version bumps
- **SemVer applies** - Follow standard semantic versioning rules
- **API stability** - Public APIs are stable and documented
- **Test vector stability** - Golden vectors are frozen (changes require MAJOR version)

## Versioning Policy

We follow [Semantic Versioning](https://semver.org/) (SemVer), **effective from v0.1.0 onwards**:

- **MAJOR**: Breaking changes (API incompatibilities, canonicalization changes)
- **MINOR**: Additive changes (new features, backward-compatible)
- **PATCH**: Bug fixes, internal refactors (no API change)

**Note:** During pre-v0.1.0 development, these rules are relaxed. Breaking changes are expected and do not require version bumps.

## Crate Release Status

### Publishable Crates (Post v0.1.0)

- **northroot-receipts**: **Not yet published** - Will be published to crates.io after v0.1.0 is stable
  - Currently in pre-v0.1.0 development
  - Breaking changes are acceptable during development
  - Once v0.1.0 is released:
    - Breaking changes require MAJOR version bump
    - Canonicalization changes are breaking (MAJOR)
    - Schema additions are MINOR (additive)

### Private Crates (Future Publishable)

- **northroot-engine**: Private for now, publishable in future
  - Currently in pre-v0.1.0 development
  - Versioned independently
  - Breaking changes are acceptable during development
  - Once published, breaking changes require MAJOR version bump

### Internal Crates

- **northroot-ops**: Internal, versioned independently
- **northroot-policy**: Internal, versioned independently
- **northroot-commons**: Internal, versioned independently

## Release Process

### Pre-v0.1.0 Development Releases

During development, we may create development tags for milestones:

```bash
# Development milestone tags (not published to crates.io)
git tag -a v0.1.0-dev.1 -m "Development milestone: v0.1.0-dev.1"
git push --tags
```

These tags mark progress but do not indicate stability.

### For Publishable Crates (northroot-receipts) - Post v0.1.0

**⚠️ Not applicable until v0.1.0 is finalized and released.**

Once v0.1.0 is stable:

1. **Update version** in `crates/northroot-receipts/Cargo.toml`
2. **Update CHANGELOG.md** (if maintained)
3. **Run all checks**: `cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings`
4. **Tag release**: `git tag -a northroot-receipts-v0.1.0 -m "Release northroot-receipts v0.1.0"`
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

### Pre-v0.1.0 Development Tags

- **Development milestones**: `v0.1.0-dev.N` (e.g., `v0.1.0-dev.1`, `v0.1.0-dev.2`)
- **Alpha releases**: `v0.1.0-alpha.N` (for testing)
- **Release candidates**: `v0.1.0-rc.N` (when approaching stable release)

### Post-v0.1.0 Release Tags

- **Publishable crates**: Use crate name and version: `northroot-receipts-v0.1.0`
- **Workspace releases**: Use simple version: `v0.1.0` (for major milestones)

## Pre-v0.1.0 Development Checklist

For development milestones:

- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] CI checks pass (vector integrity, drift detection)
- [ ] Major features documented in ADRs
- [ ] Test vectors are stable (or regeneration documented)

## Pre-Release Checklist (Post v0.1.0)

For stable releases (v0.1.0 and beyond):

- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation is complete and up to date
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] Dependent crates updated if needed
- [ ] API stability reviewed
- [ ] Breaking changes documented
- [ ] Test vectors frozen (no changes without MAJOR version)

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

