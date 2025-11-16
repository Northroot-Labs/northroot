# Commit Message Guide

**Effective Date:** 2025-11-16  
**Status:** Active

## Quick Reference

All commits **must** follow [Conventional Commits](https://www.conventionalcommits.org/) format:

```
type(scope): description

Optional body with details.

BREAKING CHANGE: description (if applicable)
```

## Commit Types

| Type | Changelog | When to Use |
|------|-----------|-------------|
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

## Commit Scopes

Use crate/component names:

- `receipts` → `northroot-receipts`
- `engine` → `northroot-engine`
- `storage` → `northroot-storage`
- `sdk` or `python` → Python SDK
- `docs` → Documentation
- `ci` → CI/CD (ignored in changelog)

## Examples

### Good Commits

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

Implements async/await wrappers using asyncio.to_thread for
non-blocking receipt operations.

# Multiple changes
feat(engine): add incremental sum strategy

- Support state-preserving aggregation
- Add Merkle Row-Map for deterministic state
- Implement reuse decision logic
```

### Bad Commits

```bash
# Too vague
fix stuff
update
wip

# Missing type
add feature

# Missing scope (acceptable but less specific)
feat: add new feature

# No description
fix(engine):
```

## Breaking Changes

Always include `BREAKING CHANGE:` footer:

```bash
refactor(receipts): change Receipt structure

BREAKING CHANGE: Receipt.pac field is now required instead of optional.
Migration: Set pac to None if not available.
```

## Commit Message Best Practices

1. **One logical change per commit**: Don't mix unrelated changes
2. **Be descriptive**: Write clear, concise messages
3. **Use present tense**: "add feature" not "added feature"
4. **Keep first line under 72 characters**: Use body for details
5. **Explain why, not what**: Body should explain motivation

## Ignored Commits

These commit types are automatically excluded from changelog:

- `chore:` - Maintenance tasks
- `ci:` - CI/CD changes
- `test:` - Test changes
- `docs(changelog):` - Changelog updates
- `Merge ...` - Merge commits
- `Revert ...` - Revert commits

## Related Documentation

- [Versioning Strategy](./VERSIONING.md)
- [Release Workflow](./RELEASE_WORKFLOW.md)
- [Changelog Guide](./CHANGELOG_GUIDE.md)

