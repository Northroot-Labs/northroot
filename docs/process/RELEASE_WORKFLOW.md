# Release Workflow

**Effective Date:** 2025-11-16  
**Status:** Active  
**Current Version:** 0.1.0

## Overview

This document defines the step-by-step workflow for creating releases, maintaining changelogs, and ensuring clean commit history.

## Release Types

### Patch Release (0.1.0 → 0.1.1)

**Trigger:** Bug fixes, internal refactors, documentation updates

**Process:**
1. Fix the issue
2. Commit with `fix(scope):` or `refactor(scope):`
3. Update changelog: `bash scripts/update-changelog.sh update`
4. Bump version: `0.1.0` → `0.1.1`
5. Create release: `bash scripts/update-changelog.sh release 0.1.1`
6. Tag: `git tag -a v0.1.1 -m "Release v0.1.1"`
7. Push: `git push origin main --tags`

### Minor Release (0.1.0 → 0.2.0)

**Trigger:** New features, backward-compatible additions

**Process:**
1. Implement feature(s)
2. Commit with `feat(scope):`
3. Update changelog: `bash scripts/update-changelog.sh update`
4. Bump version: `0.1.0` → `0.2.0`
5. Create release: `bash scripts/update-changelog.sh release 0.2.0`
6. Tag: `git tag -a v0.2.0 -m "Release v0.2.0"`
7. Push: `git push origin main --tags`

### Major Release (0.1.0 → 1.0.0)

**Trigger:** Breaking changes, API incompatibilities

**Process:**
1. Implement breaking change(s)
2. Commit with `BREAKING CHANGE:` footer
3. Update changelog: `bash scripts/update-changelog.sh update`
4. Bump version: `0.1.0` → `1.0.0`
5. Create release: `bash scripts/update-changelog.sh release 1.0.0`
6. Tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
7. Push: `git push origin main --tags`

## Step-by-Step Release Process

### 1. Pre-Release Validation

```bash
# Run all tests
cargo test --workspace --all-features

# Check formatting
cargo fmt --all -- --check

# Check linter
cargo clippy --workspace --all-features -- -D warnings

# Verify integrity
bash scripts/check-integrity.sh
```

### 2. Update Changelog

```bash
# Generate changelog entries from commits since last release
bash scripts/update-changelog.sh update

# Review the changes
git diff CHANGELOG.md
```

### 3. Create Release Section

```bash
# Replace [Unreleased] with version and date
bash scripts/update-changelog.sh release 0.1.0
```

### 4. Update Version Files

```bash
# Update VERSION file
echo "0.1.0" > VERSION

# Update Cargo.toml (if workspace version changed)
# Edit Cargo.toml: [workspace.package].version = "0.1.0"

# Update pyproject.toml (if Python SDK version changed)
# Edit sdk/python/northroot/pyproject.toml: [project].version = "0.1.0"
```

### 5. Commit Release Preparation

```bash
git add CHANGELOG.md VERSION Cargo.toml sdk/python/northroot/pyproject.toml
git commit -m "chore: prepare release 0.1.0"
```

### 6. Tag Release

```bash
# Create annotated tag
git tag -a v0.1.0 -m "Release v0.1.0

See CHANGELOG.md for details."
```

### 7. Push Release

```bash
# Push commits and tags
git push origin main
git push origin main --tags
```

### 8. Create GitHub Release (Optional)

1. Go to: https://github.com/Northroot-Labs/Northroot/releases/new
2. Select tag: `v0.1.0`
3. Title: `v0.1.0`
4. Description: Copy from `CHANGELOG.md` release section
5. Publish release

**Note:** If using GitHub Actions for PyPI publishing, the workflow will automatically trigger on tag push.

### 9. Post-Release

```bash
# Update version for next development cycle
# For patch: 0.1.0 → 0.1.1
# For minor: 0.1.0 → 0.2.0
# For major: 0.1.0 → 1.0.0

echo "0.1.1" > VERSION  # Example: preparing for next patch
git add VERSION
git commit -m "chore: bump version to 0.1.1-dev"
```

## Quick Reference

### Common Commands

```bash
# Update changelog
bash scripts/update-changelog.sh update

# Create release
bash scripts/update-changelog.sh release 0.1.0

# Tag release
git tag -a v0.1.0 -m "Release v0.1.0"

# Push tags
git push origin main --tags

# Check current version
cat VERSION
```

### Version Bump Script

Create a helper script for version bumps:

```bash
#!/bin/bash
# scripts/bump-version.sh

NEW_VERSION="$1"
if [ -z "$NEW_VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

# Update VERSION file
echo "$NEW_VERSION" > VERSION

# Update Cargo.toml workspace version
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Update pyproject.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" sdk/python/northroot/pyproject.toml

echo "✓ Version bumped to $NEW_VERSION"
echo "Review changes and commit:"
echo "  git diff VERSION Cargo.toml sdk/python/northroot/pyproject.toml"
```

## Commit History Best Practices

### Clean History

1. **One logical change per commit**: Don't mix unrelated changes
2. **Descriptive messages**: Write clear, concise commit messages
3. **Conventional format**: Always use `type(scope): description`
4. **Group related changes**: Commit related files together
5. **Avoid merge commits in main**: Use rebase or squash when possible

### Commit Message Quality

**Good:**
```bash
feat(sdk): add receipt listing with filtering

Implements list_receipts() method with support for filtering by
workload_id, trace_id, and date range. Uses FilesystemStore query
capabilities.

Related to P2-T1 (receipt storage).
```

**Bad:**
```bash
fix stuff
update
wip
```

### Squashing Commits

Before merging PRs, consider squashing:

```bash
# Interactive rebase to squash
git rebase -i HEAD~5

# Or use GitHub's "Squash and merge" option
```

## Release Checklist

### Pre-Release

- [ ] All tests pass (`cargo test --workspace --all-features`)
- [ ] Linter passes (`cargo clippy --workspace --all-features -- -D warnings`)
- [ ] Format check passes (`cargo fmt --all -- --check`)
- [ ] Integrity checks pass (`bash scripts/check-integrity.sh`)
- [ ] Changelog updated (`bash scripts/update-changelog.sh update`)
- [ ] Version files updated (VERSION, Cargo.toml, pyproject.toml)
- [ ] Documentation reviewed
- [ ] Breaking changes documented (if any)

### Release

- [ ] Release section created (`bash scripts/update-changelog.sh release X.Y.Z`)
- [ ] Release commit created (`chore: prepare release X.Y.Z`)
- [ ] Tag created (`git tag -a vX.Y.Z`)
- [ ] Tag pushed (`git push origin main --tags`)
- [ ] GitHub release created (if applicable)

### Post-Release

- [ ] Version bumped for next development cycle
- [ ] PyPI package published (if applicable, via GitHub Actions)
- [ ] Release announcement (if applicable)
- [ ] Documentation updated (if needed)

## Troubleshooting

### Changelog Not Updating

1. Check commit format: Must follow `type(scope): description`
2. Check ignore patterns: Commits matching ignore patterns are skipped
3. Verify commits exist: `git log --oneline` since last release

### Version Mismatch

1. Check VERSION file: `cat VERSION`
2. Check Cargo.toml: `grep version Cargo.toml`
3. Check pyproject.toml: `grep version sdk/python/northroot/pyproject.toml`
4. Update all to match

### Tag Already Exists

```bash
# Delete local tag
git tag -d v0.1.0

# Delete remote tag (if pushed)
git push origin :refs/tags/v0.1.0

# Recreate tag
git tag -a v0.1.0 -m "Release v0.1.0"
```

## Related Documentation

- [Versioning Strategy](./VERSIONING.md)
- [Changelog Guide](./CHANGELOG_GUIDE.md)
- [Contributing Guide](../../CONTRIBUTING.md)

