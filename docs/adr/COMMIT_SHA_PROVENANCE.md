# Commit SHA Provenance Guardrails

**Date**: 2025-11-12  
**Status**: ✅ Implemented

## Problem Statement

The ADR index tracks `implementation_commit` SHAs for implemented phases. Previously, commit SHAs could be added to the index before the actual commits existed, breaking provenance and making it impossible to verify which commit actually implemented a phase.

## Solution: Automated Validation

### 1. Validation Script Enhancement

**File**: `tools/adr/validate.py`

Added `validate_commit_sha()` function that:
- Verifies commit SHA exists in git repository using `git cat-file -e`
- Checks commit is accessible (not from uncommitted changes)
- Provides clear error messages for invalid SHAs

**Usage**: Automatically runs as part of `make adr.validate`

### 2. Updated ADR Maintenance Rules

**File**: `.cursor/rules/adr-maintenance.mdc`

Added **Commit SHA Provenance Rules** section:
- Enforces correct workflow: implement → commit → update index → commit index
- Prohibits adding `implementation_commit` before commit exists
- Documents validation requirements

### 3. Updated Commit Automation Rules

**File**: `.cursor/rules/commit-automation.mdc`

Added **ADR Index Commit SHA Workflow** section:
- Step-by-step workflow for implementing phases
- Clear instructions on when to get commit SHA
- Validation reminder

## Validation Output

When running `make adr.validate`, you'll see:

```
Validating commit SHA provenance:
  ✓ ADR-0010-P01: Commit SHA 9a254283... exists
  ✓ ADR-0010-P02: Commit SHA 9a254283... exists
  ⚠ ADR-0010-P03: No implementation_commit specified (optional)
  ✗ ADR-0010-P04: Commit SHA invalid123... does not exist in git repository
```

## Correct Workflow

1. **Implement the phase** (code changes, tests, etc.)
2. **Commit the implementation**:
   ```bash
   git commit -m "feat(engine): implement ADR-0010-P03 - property tests"
   ```
3. **Get the commit SHA**:
   ```bash
   git rev-parse HEAD
   ```
4. **Update ADR index** with the actual commit SHA
5. **Validate**:
   ```bash
   make adr.validate
   ```
6. **Commit the index update**:
   ```bash
   git commit -m "docs(adr): mark ADR-0010-P03 as implemented"
   ```

## Enforcement Points

- **Pre-commit hook**: Runs `make adr.validate` automatically (via `.pre-commit-config.yaml`)
- **CI/CD**: GitHub Actions workflow validates on PR/push (`.github/workflows/adr.yml`)
- **Local validation**: Run `make adr.validate` before committing

## Error Handling

If validation fails:
- Pre-commit hook blocks the commit
- CI/CD fails the build
- Clear error messages indicate which phase has invalid commit SHA

## Future Enhancements

Potential improvements:
- Verify commit actually contains relevant changes (not just exists)
- Check commit message matches phase description
- Validate commit timestamp matches `implemented_at` date

## Related Files

- `tools/adr/validate.py` - Validation implementation
- `.cursor/rules/adr-maintenance.mdc` - ADR maintenance rules
- `.cursor/rules/commit-automation.mdc` - Commit automation rules
- `.pre-commit-config.yaml` - Pre-commit hook configuration
- `.github/workflows/adr.yml` - CI/CD validation

