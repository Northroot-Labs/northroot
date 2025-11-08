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
- [Integrity Checks Documentation](../docs/INTEGRITY_CHECKS.md)
- [CI Vector Checks Documentation](../docs/CI_VECTOR_CHECKS.md)

