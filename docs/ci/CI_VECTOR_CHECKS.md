# CI Test Vector Integrity Checks

This document describes the automated CI checks that ensure test vector integrity and prevent accidental modifications.

## Overview

Test vectors in `vectors/` are **golden files** - they serve as the source of truth for receipt structure, canonicalization, and hash computation. These files must remain stable to ensure:

1. **Deterministic hashing**: Same receipt structure always produces same hash
2. **Backward compatibility**: Existing receipts continue to validate
3. **Reproducibility**: Tests produce consistent results across environments

## CI Checks

The CI workflow (`.github/workflows/ci.yml`) includes three layers of protection:

### 1. Test Vector Integrity Tests

**Step:** `Test Vector Integrity`

Runs comprehensive tests to verify vector integrity:

```bash
cargo test -p northroot-receipts --test test_vector_integrity
cargo test -p northroot-receipts --test test_drift_detection
```

**What it checks:**
- ✅ All vectors have correct hash values (computed hash matches stored hash)
- ✅ All vectors validate against JSON schemas
- ✅ All vectors have valid hash format (`sha256:<64hex>`)
- ✅ Receipt chains compose correctly (dom/cod links)
- ✅ Hash computation is deterministic and consistent
- ✅ **Hash drift detection**: Computed hashes match baseline hashes

**Failure conditions:**
- Hash mismatch in any vector
- Schema validation failure
- Hash drift (canonicalization changed without updating baselines)
- Composition errors

### 2. Accidental Modification Detection

**Step:** `Prevent Accidental Vector Regeneration`

Checks if vector files were modified in the commit/PR:

```bash
# Compares against base branch/commit
git diff --name-only $BASE_SHA HEAD | grep "^vectors/"
```

**What it does:**
- ⚠️ **Warns** if any vector files are modified
- Provides clear instructions on when vector modification is acceptable
- Notes that the drift detection test will catch hash mismatches

**Note:** This is an **informational warning**, not a hard failure. The drift detection test (above) is the hard guardrail.

### 3. File Existence Verification

**Step:** `Verify Vector Files Exist`

Ensures all required vector files are present:

```bash
# Checks for required vectors:
- vectors/data_shape.json
- vectors/method_shape.json
- vectors/reasoning_shape.json
- vectors/execution.json
- vectors/spend.json
- vectors/settlement.json
```

**Failure conditions:**
- Any required vector file is missing

## When Vector Modification is Acceptable

Vector files should **only** be modified when:

1. **Intentional canonicalization change** (e.g., switching to CBOR, RFC update)
2. **Receipt structure change** that affects canonicalization
3. **Bug fix** in hash computation logic

### Required Steps for Intentional Vector Regeneration

If you need to regenerate vectors:

1. **Regenerate vectors:**
   ```bash
   cargo test --test regenerate_vectors -- --ignored --nocapture
   ```

2. **Update baseline hashes:**
   - Run: `cargo test --test test_vector_integrity`
   - Copy the computed hashes
   - Update `BASELINE_HASHES` in `crates/northroot-receipts/tests/test_drift_detection.rs`

3. **Verify all tests pass:**
   ```bash
   cargo test -p northroot-receipts --test test_vector_integrity
   cargo test -p northroot-receipts --test test_drift_detection
   ```

4. **Document the change:**
   - Explain why vectors were regenerated in commit message
   - If PR, explain in PR description
   - Update relevant ADRs if canonicalization changed

## Failure Scenarios

### Scenario 1: Hash Drift Detected

**Error:** `Hash drift detected in N vector(s)`

**Cause:** Canonicalization logic changed, but `BASELINE_HASHES` wasn't updated.

**Fix:**
1. If intentional: Update `BASELINE_HASHES` with new computed hashes
2. If unintentional: Revert canonicalization changes

### Scenario 2: Hash Mismatch in Vector

**Error:** `Hash mismatch in <vector>.json`

**Cause:** Vector file has incorrect hash value.

**Fix:**
1. Regenerate vectors: `cargo test --test regenerate_vectors -- --ignored --nocapture`
2. Update baseline hashes if needed

### Scenario 3: Vector Files Modified

**Warning:** `⚠️ WARNING: Test vectors were modified in this commit/PR!`

**Cause:** Vector files were changed in the commit.

**Action:**
- If intentional: Ensure `BASELINE_HASHES` is updated (drift test will catch if not)
- If accidental: Revert vector file changes

## Best Practices

1. **Never modify vectors manually** - Always use `regenerate_vectors` test
2. **Always update baselines** - When vectors change, update `BASELINE_HASHES`
3. **Document changes** - Explain why vectors were regenerated
4. **Test locally first** - Run integrity tests before pushing
5. **Review carefully** - Vector changes require careful review

## Local Testing

Before pushing, run these checks locally:

```bash
# Run all vector integrity tests
cargo test -p northroot-receipts --test test_vector_integrity
cargo test -p northroot-receipts --test test_drift_detection

# Check if vectors were modified (if in git repo)
git diff --name-only HEAD | grep "^vectors/"
```

## Related Documentation

- [Test Vector README](../vectors/README.md)
- [ADR-002: Canonicalization Strategy](../ADRs/ADR-002-canonicalization-strategy.md)
- [ADR-003: Identity Root Commitment](../ADRs/ADR-003-identity-root-commitment.md)
- [Contributing Guide](../CONTRIBUTING.md)

