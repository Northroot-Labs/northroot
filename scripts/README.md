# Scripts

Utility scripts for development and CI/CD.

## check-integrity.sh

Integrity check script that prevents accidental modifications to critical files.

**Usage:**
```bash
# Check staged changes (pre-commit)
bash scripts/check-integrity.sh

# Check all changes since HEAD
git diff HEAD | bash scripts/check-integrity.sh
```

**What it checks:**
- Test vector modifications (`vectors/`)
- Baseline value modifications (`test_drift_detection.rs`)
- Schema file modifications (`schemas/`)
- Vector/baseline consistency (vectors changed → baselines must be updated)

**Exit codes:**
- `0` - All checks passed
- `1` - Errors detected (vectors modified without baseline updates)

**See also:**
- [Integrity Checks Documentation](../docs/process/INTEGRITY_CHECKS.md)
- [CI Vector Checks Documentation](../docs/ci/CI_VECTOR_CHECKS.md)

## update-changelog.sh

Automatic changelog generation script that updates `CHANGELOG.md` based on git commit messages.

**Usage:**
```bash
# Update changelog with commits since last release
bash scripts/update-changelog.sh update

# Create a new release section
bash scripts/update-changelog.sh release 0.1.0
```

**What it does:**
- Parses git commit messages since last release tag
- Categorizes commits by type (feat, fix, refactor, etc.)
- Updates the `[Unreleased]` section in `CHANGELOG.md`
- Supports conventional commit format: `type(scope): description`

**Commit message format:**
- `feat(scope): add new feature` → Added category
- `fix(scope): correct bug` → Fixed category
- `refactor(scope): simplify code` → Changed category
- `security(scope): fix vulnerability` → Security category
- `BREAKING CHANGE:` prefix marks breaking changes

**Configuration:**
- Configuration file: `.changelog.toml`
- Customize categories, patterns, and mappings

**See also:**
- [CHANGELOG.md](../CHANGELOG.md)
- [Contributing Guide](../CONTRIBUTING.md#changelog-guidelines)

