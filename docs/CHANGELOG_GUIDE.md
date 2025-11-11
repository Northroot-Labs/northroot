# Changelog Management Guide

This guide explains how to use the automatic changelog generation toolchain in Northroot.

## Overview

The project uses an automatic changelog system that generates `CHANGELOG.md` entries from git commit messages. The changelog follows the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format and adheres to [Semantic Versioning](https://semver.org/).

## Quick Start

### Updating the Changelog

```bash
# Update changelog with commits since last release
bash scripts/update-changelog.sh update
```

### Creating a Release

```bash
# Create a new release section (e.g., for version 0.1.0)
bash scripts/update-changelog.sh release 0.1.0
```

## Commit Message Format

For automatic changelog generation, use the **Conventional Commits** format:

```
type(scope): description

Optional body with more details.

BREAKING CHANGE: description of breaking change (if applicable)
```

### Types

| Type | Changelog Category | Description |
|------|-------------------|-------------|
| `feat` or `feature` | **Added** | New features |
| `fix` or `bugfix` | **Fixed** | Bug fixes |
| `refactor` or `change` | **Changed** | Code changes, refactoring |
| `remove` | **Removed** | Removed features |
| `deprecate` | **Deprecated** | Deprecated features |
| `security` | **Security** | Security fixes |

### Scopes

Use crate names or component names as scope:
- `receipts` → `northroot-receipts`
- `engine` → `northroot-engine`
- `storage` → `northroot-storage`
- `docs` → Documentation changes
- `ci` → CI/CD changes (ignored in changelog)

### Examples

```bash
# New feature
git commit -m "feat(receipts): add CBOR canonicalization support"

# Bug fix
git commit -m "fix(engine): correct hash computation in Merkle tree"

# Breaking change
git commit -m "refactor(receipts): migrate from JCS to CBOR

BREAKING CHANGE: Receipt canonicalization now uses CBOR instead of JSON"

# Documentation (ignored in changelog)
git commit -m "docs(readme): update installation instructions"

# Multiple changes
git commit -m "feat(engine): add incremental sum strategy

- Support state-preserving aggregation
- Add Merkle Row-Map for deterministic state"
```

## Configuration

The changelog generation is configured via `.changelog.toml`:

```toml
[changelog]
file = "CHANGELOG.md"
unreleased_header = "## [Unreleased]"
date_format = "%Y-%m-%d"

[patterns]
# Type mappings
type_mapping = {
    "feat" = "Added",
    "fix" = "Fixed",
    # ...
}

# Ignore patterns
ignore_patterns = [
    "^Merge ",
    "^Revert ",
    "^chore",
    "^ci",
    "^test",
]
```

## Workflow

### Daily Development

1. Make changes and commit using conventional commit format
2. The changelog is automatically updated (if hook is installed)
3. Or manually run: `bash scripts/update-changelog.sh update`

### Before Release

1. Update changelog: `bash scripts/update-changelog.sh update`
2. Review the `[Unreleased]` section
3. Create release section: `bash scripts/update-changelog.sh release 0.1.0`
4. Commit the changelog update
5. Tag the release: `git tag -a v0.1.0 -m "Release v0.1.0"`

### Example Release Workflow

```bash
# 1. Update changelog with all commits since last release
bash scripts/update-changelog.sh update

# 2. Review CHANGELOG.md
git diff CHANGELOG.md

# 3. Create release section
bash scripts/update-changelog.sh release 0.1.0

# 4. Commit changelog
git add CHANGELOG.md
git commit -m "chore: prepare release 0.1.0"

# 5. Tag release
git tag -a v0.1.0 -m "Release v0.1.0"
git push --tags
```

## Git Hook Integration (Optional)

Install an automatic post-commit hook:

```bash
bash scripts/install-changelog-hook.sh
```

This will update the changelog automatically after each commit. The hook runs in the background and won't block commits.

To disable:
```bash
rm .git/hooks/post-commit
```

## Ignored Commits

The following commit types are automatically excluded from the changelog:

- Merge commits (`Merge ...`)
- Revert commits (`Revert ...`)
- Chore commits (`chore: ...`)
- CI commits (`ci: ...`)
- Test commits (`test: ...`)
- Changelog commits (`docs(changelog): ...`)

## Manual Entries

If you need to add entries manually, edit `CHANGELOG.md` directly under the `[Unreleased]` section:

```markdown
## [Unreleased]

### Added
- Your manual entry here
```

## Troubleshooting

### No entries generated

If `update` shows "No new entries to add to changelog":

1. Check if commits follow conventional commit format
2. Verify commits are not in ignore patterns
3. Check if there are commits since last release tag

### Entries in wrong category

1. Check commit message type (feat, fix, etc.)
2. Review `.changelog.toml` type mappings
3. Manually edit `CHANGELOG.md` if needed

### Hook not working

1. Verify hook is installed: `ls -la .git/hooks/post-commit`
2. Check hook permissions: `chmod +x .git/hooks/post-commit`
3. Test manually: `bash scripts/update-changelog.sh update`

## Best Practices

1. **Use conventional commits**: Always use conventional commit format
2. **Be descriptive**: Write clear, descriptive commit messages
3. **Scope appropriately**: Use scopes to indicate which crate/component changed
4. **Mark breaking changes**: Use `BREAKING CHANGE:` for breaking changes
5. **Review before release**: Always review the changelog before creating a release
6. **Keep it updated**: Run `update` regularly or use the git hook

## Related Documentation

- [Contributing Guide](../CONTRIBUTING.md#changelog-guidelines)
- [Releasing Guide](../RELEASING.md)
- [CHANGELOG.md](../CHANGELOG.md)

