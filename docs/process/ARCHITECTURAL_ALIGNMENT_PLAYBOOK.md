# Architectural Alignment Playbook

**Purpose**: Ensure architectural changes maintain invariants, preserve contracts, and are properly tested before moving to next phase.

**Scope**: Applied to each phase before implementation and after completion.

---

## Phase Classification

Each phase must be classified as one of:

- **`refactor`**: Structural changes that preserve behavior (e.g., moving code, renaming, extracting modules)
- **`change`**: Behavioral changes that modify outputs or contracts (e.g., algorithm changes, format changes, breaking changes)
- **`additive`**: New functionality that doesn't affect existing code (e.g., new optional fields, new modules)

---

## Pre-Implementation Checklist

### For All Phases

1. **Document Expected Changes**
   - List all behavioral changes (if any)
   - Identify breaking changes explicitly
   - Note backward compatibility requirements

2. **Identify Affected Components**
   - List all crates/modules that will be modified
   - Identify dependent code (strategies, tests, examples)
   - Note test vectors that may need updates

3. **Review ADR Alignment**
   - Verify changes align with relevant ADRs
   - Update ADR if phase introduces new architectural decisions
   - Document any ADR supersessions

---

## Phase Type: `refactor`

### Coverage Analysis

```bash
# Generate coverage baseline before refactor
cargo test --package <crate> --lib --all-features
cargo llvm-cov --package <crate> --lib --lcov --output-path coverage-before.lcov

# After refactor, generate coverage diff
cargo llvm-cov --package <crate> --lib --lcov --output-path coverage-after.lcov
diff coverage-before.lcov coverage-after.lcov > coverage-diff.txt
```

**Action Items**:
- Review coverage diff for any regressions
- Ensure new code paths are covered
- Document any intentional coverage gaps

### Property Tests

Generate property tests to verify invariants:

```rust
// Example: Property test for MerkleRowMap determinism
#[quickcheck]
fn prop_merkle_row_map_deterministic(entries: Vec<(String, i64)>) -> bool {
    let mut map1 = MerkleRowMap::new();
    let mut map2 = MerkleRowMap::new();
    
    for (key, value) in &entries {
        map1.insert(key.clone(), CborValue::Integer((*value).into()));
        map2.insert(key.clone(), CborValue::Integer((*value).into()));
    }
    
    map1.compute_root() == map2.compute_root()
}
```

**Property Test Categories**:
- **Determinism**: Same input → same output
- **Idempotency**: f(f(x)) == f(x)
- **Commutativity**: Order independence where applicable
- **Associativity**: Grouping independence where applicable
- **Invariants**: Core properties that must always hold

### Contract Tests

Generate contract tests to verify external interfaces:

```rust
// Example: Contract test for ExecutionPayload serialization
#[test]
fn contract_execution_payload_serialization() {
    let payload = ExecutionPayload { /* ... */ };
    
    // Contract: Must serialize to CBOR deterministically
    let cbor1 = cbor_deterministic(&payload).unwrap();
    let cbor2 = cbor_deterministic(&payload).unwrap();
    assert_eq!(cbor1, cbor2);
    
    // Contract: Must round-trip through serialization
    let deserialized: ExecutionPayload = ciborium::de::from_reader(cbor1.as_slice()).unwrap();
    assert_eq!(payload, deserialized);
}
```

**Contract Test Categories**:
- **Serialization contracts**: CBOR/JSON round-trips
- **API contracts**: Public function signatures and behaviors
- **Trait contracts**: Trait implementations meet specifications
- **Storage contracts**: Data persistence and retrieval

### Mutation Testing

Run cargo-mutants to verify test quality:

```bash
# Install cargo-mutants if not present
cargo install cargo-mutants

# Run mutation testing
cargo mutants --package <crate> --lib
```

**Action Items**:
- Fix any surviving mutants (tests that don't catch mutations)
- Add missing test cases for uncovered mutations
- Document any intentionally ignored mutations

### Golden Vectors

Ensure all golden vectors are present and up-to-date:

```bash
# List all golden vector files
find vectors/ -name "*.json" -o -name "*.cbor"

# Verify vectors are used in tests
grep -r "load.*vector\|golden" crates/*/tests/
```

**Golden Vector Checklist**:
- [ ] All data structures have corresponding vectors
- [ ] Vectors include edge cases (empty, single, multiple)
- [ ] Vectors document expected outputs
- [ ] Vectors are versioned (if format changes)

---

## Phase Type: `change`

### Spec-First Tests (Fail Now, Pass Later)

Write tests that specify the new behavior before implementation:

```rust
// Example: Spec test for RFC-6962 domain separation
#[test]
#[should_panic(expected = "not yet implemented")] // Or use Result return
fn test_rfc6962_domain_separation() {
    // SPEC: Empty map root should be H(0x00 || "")
    let map = MerkleRowMap::new();
    let root = map.compute_root();
    
    // This will fail until RFC-6962 is implemented
    assert_eq!(
        root,
        "sha256:6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
    );
}
```

**Spec Test Structure**:
1. Document the specification in test name/comments
2. Write test that fails with current implementation
3. Mark with `#[ignore]` or `#[should_panic]` until implemented
4. Implement feature to make test pass
5. Remove ignore/panic markers

### Compatibility Tests (Old vs New)

Test that old and new behaviors are correctly differentiated:

```rust
// Example: Compatibility test for domain separation change
#[test]
fn test_domain_separation_compatibility() {
    let mut map = MerkleRowMap::new();
    map.insert("key1".to_string(), CborValue::Integer(42.into()));
    
    let new_root = map.compute_root();
    
    // Old root (with "leaf:" prefix) - should NOT match
    let old_root = "sha256:efc2744e9b28f4e70e3fa27213eff9a64c98bae3fd14c159cde674dcb607d66d";
    assert_ne!(
        new_root, old_root,
        "New RFC-6962 root must differ from old 'leaf:' prefix root"
    );
    
    // Verify new root format is correct
    assert!(new_root.starts_with("sha256:"));
    assert_eq!(new_root.len(), 71);
}
```

**Compatibility Test Categories**:
- **Output format changes**: Verify new format is correct
- **Breaking changes**: Verify old outputs are rejected/invalid
- **Migration paths**: Test conversion utilities (if provided)
- **Backward compatibility**: Verify old code still works (if applicable)

### Temporary Feature Flag

For breaking changes, use a feature flag to allow gradual migration:

```rust
// In Cargo.toml
[features]
default = []
rfc6962_domain_separation = []  // Temporary flag

// In code
#[cfg(feature = "rfc6962_domain_separation")]
fn compute_root(&self) -> String {
    // New RFC-6962 implementation
}

#[cfg(not(feature = "rfc6962_domain_separation"))]
fn compute_root(&self) -> String {
    // Old "leaf:"/"node:" implementation
}
```

**Flag Lifecycle**:
1. Add flag with old behavior as default
2. Implement new behavior behind flag
3. Update tests to use new behavior
4. After migration period, remove flag and old code

### Delta Verification

Prove that changes match specification exactly:

```rust
// Example: Delta verification for root change
#[test]
fn test_delta_verification_rfc6962() {
    // SPEC: Empty root = H(0x00 || "")
    let expected = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&[0x00u8]);  // RFC-6962 leaf prefix
        hasher.update(b"");
        format!("sha256:{:x}", hasher.finalize())
    };
    
    let actual = MerkleRowMap::new().compute_root();
    
    assert_eq!(
        actual, expected,
        "Delta must match specification exactly: H(0x00 || \"\")"
    );
}
```

**Delta Verification Checklist**:
- [ ] All outputs match specification exactly
- [ ] Edge cases handled per spec
- [ ] Error cases match spec
- [ ] Performance characteristics match spec (if specified)

---

## Post-Implementation Verification

### For All Phases

1. **Run Full Test Suite**
   ```bash
   cargo test --workspace --all-features
   ```

2. **Check Linter**
   ```bash
   cargo clippy --workspace --all-features -- -D warnings
   ```

3. **Verify Documentation**
   ```bash
   cargo doc --workspace --no-deps --open
   ```

4. **Update ADR**
   - Mark phase as complete in ADR
   - Document any deviations from plan
   - Note any follow-up work needed

### For `refactor` Phases

- [ ] Coverage diff reviewed (no regressions)
- [ ] Property tests added and passing
- [ ] Contract tests added and passing
- [ ] Mutation testing passes (no surviving mutants)
- [ ] Golden vectors updated if needed

### For `change` Phases

- [ ] Spec-first tests now passing
- [ ] Compatibility tests verify old vs new
- [ ] Feature flag removed (if used)
- [ ] Delta verification proves exact match to spec
- [ ] Test vectors updated with new values
- [ ] Migration guide written (if breaking change)

---

## Phase 1-3 Retrospective

### Phase 1: DataShape Enum and ExecutionPayload Extensions

**Classification**: `additive`
**Breaking**: No

**What We Did Well ✅**:
- ✅ All new fields are optional (backward compatible)
- ✅ Comprehensive invariant tests added
- ✅ Test utilities updated correctly

**What We Should Have Done (Playbook)**:
- ⚠️ Could have written spec-first tests before implementation
- ⚠️ Could have run cargo-mutants (additive phase, lower priority)

**Status**: ✅ Complete and verified

---

### Phase 2: MerkleRowMap RFC-6962 Domain Separation

**Classification**: `change` (breaking behavioral change)
**Breaking**: Yes (all MerkleRowMap roots changed)

**What We Did Well ✅**:
- ✅ Wrote comprehensive invariant tests (`test_phase_invariants.rs`)
- ✅ Updated test vectors with new root values
- ✅ Verified core invariants still hold (determinism, order independence)
- ✅ Documented breaking change in ADR-009
- ✅ Updated drift detection baselines
- ✅ Verified strategies still work correctly

**What We Should Have Done (Playbook)**:
- ⚠️ Could have used temporary feature flag for gradual migration
- ⚠️ Could have run cargo-mutants to verify test quality
- ⚠️ Could have added property tests (quickcheck) for edge cases
- ⚠️ Could have written compatibility tests earlier (old vs new roots)

**Lessons Learned**:
- Breaking changes need more upfront planning
- Feature flags help with gradual migration
- Mutation testing catches test gaps
- Property tests catch edge cases
- Compatibility tests help verify migration paths

**Status**: ✅ Complete and verified

---

### Phase 3: ByteStream Manifest Builder (CAS Module)

**Classification**: `additive`
**Breaking**: No

**What We Did Well ✅**:
- ✅ New module doesn't affect existing code
- ✅ Comprehensive tests for chunking and manifest building
- ✅ Verified determinism and data preservation

**What We Should Have Done (Playbook)**:
- ⚠️ Could have written spec-first tests before implementation
- ⚠️ Could have run cargo-mutants (additive phase, lower priority)
- ⚠️ Could have added property tests for chunking invariants

**Status**: ✅ Complete and verified

---

### Overall Assessment

**Test Coverage**:
- ✅ 14 phase invariant tests
- ✅ 81 engine library tests
- ✅ 13 strategy tests
- ✅ 6 vector integrity tests
- ✅ 2 drift detection tests
- ✅ All passing

**Areas for Improvement**:
- Add property tests (quickcheck) for future phases
- Run cargo-mutants before Phase 4
- Use feature flags for breaking changes
- Write compatibility tests earlier in the process

---

## Automation Scripts

### Coverage Diff Generator

```bash
#!/bin/bash
# scripts/coverage-diff.sh

CRATE=$1
if [ -z "$CRATE" ]; then
    echo "Usage: $0 <crate-name>"
    exit 1
fi

echo "Generating coverage baseline..."
cargo llvm-cov --package $CRATE --lib --lcov --output-path coverage-before.lcov

echo "Run your refactor, then press Enter to generate diff..."
read

echo "Generating coverage after refactor..."
cargo llvm-cov --package $CRATE --lib --lcov --output-path coverage-after.lcov

echo "Generating diff..."
diff coverage-before.lcov coverage-after.lcov > coverage-diff.txt
echo "Coverage diff saved to coverage-diff.txt"
```

### Mutation Test Runner

```bash
#!/bin/bash
# scripts/mutation-test.sh

CRATE=$1
if [ -z "$CRATE" ]; then
    echo "Usage: $0 <crate-name>"
    exit 1
fi

echo "Running mutation testing on $CRATE..."
cargo mutants --package $CRATE --lib -- --test-threads=1

if [ $? -eq 0 ]; then
    echo "✅ All mutations caught by tests"
else
    echo "❌ Some mutations survived - improve test coverage"
    exit 1
fi
```

### Golden Vector Checker

```bash
#!/bin/bash
# scripts/check-golden-vectors.sh

echo "Checking golden vectors..."

# Find all vector files
VECTORS=$(find vectors/ -name "*.json" -o -name "*.cbor")

for vec in $VECTORS; do
    echo "Checking $vec..."
    # Verify it's referenced in tests
    if ! grep -r "$(basename $vec)" crates/*/tests/ > /dev/null; then
        echo "⚠️  Warning: $vec not referenced in tests"
    fi
done

echo "Done"
```

---

## Integration with CI/CD

Add to `.github/workflows/ci.yml` or similar:

```yaml
- name: Run mutation testing
  run: |
    cargo install cargo-mutants
    cargo mutants --package northroot-engine --lib

- name: Check coverage
  run: |
    cargo llvm-cov --package northroot-engine --lib --lcov --output-path coverage.lcov
    # Upload to coverage service

- name: Verify golden vectors
  run: |
    ./scripts/check-golden-vectors.sh
```

---

## Quick Reference

### Refactor Phase Checklist
- [ ] Generate coverage baseline
- [ ] Implement refactor
- [ ] Generate coverage diff
- [ ] Add property tests
- [ ] Add contract tests
- [ ] Run cargo-mutants
- [ ] Update golden vectors
- [ ] Verify all tests pass

### Change Phase Checklist
- [ ] Classify as breaking or non-breaking
- [ ] Write spec-first tests (failing)
- [ ] Add temporary feature flag (if breaking)
- [ ] Implement change
- [ ] Write compatibility tests
- [ ] Verify deltas match spec exactly
- [ ] Update test vectors
- [ ] Remove feature flag (after migration)
- [ ] Document breaking changes
- [ ] Write migration guide (if needed)

---

## Phase 4 Classification and Checklist

### Phase 4: Privacy-Preserving Resolver API and Storage Extensions

**Classification**: `additive` (with storage schema changes)

**Rationale**:
- New traits (`ArtifactResolver`, `ManagedCache`) - additive
- New storage tables - additive (existing receipts remain valid)
- New optional fields already added in Phase 1 - backward compatible
- No breaking changes to existing behavior

### Pre-Implementation Checklist

- [x] **Document Expected Changes**: New resolver traits, storage extensions
- [x] **Identify Affected Components**: 
  - `northroot-engine/src/resolver.rs` (new)
  - `northroot-storage/src/traits.rs` (extend)
  - `northroot-storage/src/sqlite.rs` (extend)
- [x] **Review ADR Alignment**: Aligns with ADR-009

### Implementation Checklist (Additive Phase)

- [ ] **Write Spec-First Tests** (for new functionality):
  ```rust
  #[test]
  fn test_artifact_resolver_trait_contract() {
      // SPEC: ArtifactResolver must resolve encrypted locators
      // This will be implemented by tenants, but we test the trait contract
  }
  ```

- [ ] **Add Contract Tests**:
  - Resolver trait contracts (resolve, store, batch)
  - Storage trait contracts (store/retrieve encrypted locators)
  - Storage schema contracts (table creation, indexes)

- [ ] **Add Integration Tests**:
  - End-to-end: receipt → encrypted locator → storage → retrieval
  - Verify privacy: receipts never contain plain locators

- [ ] **Update Storage Schema Tests**:
  - Verify new tables created correctly
  - Verify indexes created correctly
  - Verify foreign key constraints

- [ ] **Document Migration Path**:
  - SQL migration script for new tables
  - Backward compatibility: existing receipts work without locators

### Post-Implementation Verification

- [ ] All new tests passing
- [ ] Storage schema migration tested
- [ ] Backward compatibility verified (old receipts still work)
- [ ] ADR-009 updated with Phase 4 completion

---

## Next Steps

Before starting Phase 4:

1. ✅ Review this playbook
2. ✅ Classify Phase 4 as `additive`
3. ⏭️ Follow additive checklist (spec-first tests, contract tests, integration tests)
4. ⏭️ Document any deviations or improvements

