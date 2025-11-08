# Integrity Checks

This document describes the integrity checks that prevent accidental modifications to critical files.

## Overview

Critical files that preserve system integrity are protected by automated checks:

1. **Test vectors** (`vectors/`) - Golden files for receipt structure and canonicalization
2. **Baseline values** (`test_drift_detection.rs`) - Locked hash/root values for drift detection
3. **Schema files** (`schemas/`) - JSON schemas for validation

## Local Checks

### Pre-commit Hook

A git pre-commit hook automatically runs integrity checks before each commit:

```bash
# The hook runs automatically on: git commit
# To bypass (not recommended): git commit --no-verify
```

**What it checks:**
- ✅ Test vector modifications
- ✅ Baseline value modifications
- ✅ Schema file modifications
- ✅ Vector/baseline consistency (vectors changed → baselines must be updated)

**Installation:**
The hook is automatically installed in `.git/hooks/pre-commit`. If missing, copy from:
```bash
cp scripts/check-integrity.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### Manual Check

Run integrity checks manually:

```bash
# Check staged changes
bash scripts/check-integrity.sh

# Check specific commit range
git diff --name-only HEAD~5 HEAD | bash scripts/check-integrity.sh
```

## CI/CD Checks

The CI workflow (`.github/workflows/ci.yml`) includes comprehensive integrity checks:

### 1. Test Vector Integrity Tests

Runs all vector integrity and drift detection tests:
- `test_vector_integrity` - Validates all vectors
- `test_drift_detection` - Checks hash/root baselines
- `test_engine_vector_integrity` - Engine vector validation
- `test_composition_vector_roundtrip` - Composition validation

**Failure:** Hard failure if any test fails

### 2. Integrity Check Script

Runs the same integrity check script as local pre-commit hook:
- Detects modified vectors, baselines, schemas
- Validates vector/baseline consistency
- Provides clear error messages

**Failure:** Hard failure if vectors modified without baseline updates

### 3. Vector File Verification

Verifies all required vector files exist:
- Checks for missing vectors
- Validates file structure

**Failure:** Hard failure if required vectors are missing

## Integrity Rules

### Rule 1: Vectors and Baselines Must Stay in Sync

**If vectors are modified:**
- ✅ Baselines MUST be updated
- ✅ Change MUST be documented in commit message
- ✅ Drift detection tests MUST pass

**If baselines are modified:**
- ✅ Vectors should be updated (if algorithm changed)
- ✅ Change MUST be documented
- ✅ All tests MUST pass

### Rule 2: Schema Changes Require Careful Review

**If schemas are modified:**
- ⚠️ May require vector regeneration
- ⚠️ May require baseline updates
- ⚠️ May require version bump (if breaking)
- ⚠️ MUST be documented

### Rule 3: Intentional Changes Must Be Explicit

**Acceptable scenarios:**
1. Canonicalization algorithm change → Update vectors + baselines
2. Schema evolution → Update schemas + vectors + baselines
3. Bug fix in hash computation → Update vectors + baselines
4. New test vectors → Add vectors + update baselines

**Unacceptable scenarios:**
1. Vectors modified without baseline updates ❌
2. Baselines modified without documentation ❌
3. Schema changes without version bump (if breaking) ❌

## Error Messages

### Error: "Vectors modified but baselines NOT updated"

**Cause:** Test vectors were changed but baseline values weren't updated.

**Fix:**
```bash
# 1. Regenerate vectors (if needed)
cargo test --test regenerate_vectors -- --ignored --nocapture

# 2. Update baselines
cargo test --test test_drift_detection -- --nocapture

# 3. Copy new baseline values to test_drift_detection.rs
#    - receipts: Update BASELINE_HASHES
#    - engine: Update BASELINE_ROOTS

# 4. Commit with clear message explaining the change
```

### Warning: "Vectors modified"

**Cause:** Vector files were changed (may be intentional).

**Action:**
- If intentional: Ensure baselines are updated (check will fail if not)
- If accidental: Revert vector changes

### Warning: "Baselines modified but no vector changes"

**Cause:** Baseline values were updated without vector changes.

**Action:**
- This is OK if updating baselines after algorithm changes
- Ensure all tests pass
- Document the change

## Bypassing Checks

### Local (Not Recommended)

```bash
# Bypass pre-commit hook
git commit --no-verify

# ⚠️ Only do this if you understand the risks!
```

### CI (Not Possible)

CI checks cannot be bypassed. All integrity checks must pass for CI to succeed.

## Best Practices

1. **Run checks locally first:**
   ```bash
   bash scripts/check-integrity.sh
   cargo test --test test_drift_detection
   ```

2. **Update baselines with vectors:**
   Always update baselines when vectors change.

3. **Document changes:**
   Explain why vectors/baselines were modified in commit message.

4. **Test before committing:**
   Run integrity checks and drift detection tests locally.

5. **Review carefully:**
   Vector and baseline changes require careful review.

## Related Documentation

- [CI Vector Checks](CI_VECTOR_CHECKS.md) - Detailed CI check documentation
- [Test Vector README](../vectors/README.md) - Vector file documentation
- [ADR-002: Canonicalization Strategy](../ADRs/ADR-002-canonicalization-strategy.md)
- [Contributing Guide](../CONTRIBUTING.md)

