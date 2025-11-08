<!-- 8d743b29-bffb-4a3f-bec7-ec6941bf15ee dd0cfa0e-0497-4bed-9403-e2f9d95f3655 -->
# Finalize Receipts Crate

## Current State Assessment

The receipts crate is functionally complete with:

- ✅ All 6 receipt kinds defined
- ✅ Canonicalization (JCS) implemented
- ✅ Hash computation and validation
- ✅ Comprehensive test suite with golden vectors
- ✅ JSON Schema validation
- ✅ Error types and handling
- ✅ Custom deserialization

## Finishing Touches Needed

### 1. Cargo.toml Metadata (`receipts/Cargo.toml`)

**Issue**: Missing crate metadata for publishing

- Add `description`, `repository`, `documentation`, `keywords`, `categories`
- Add `license-file` if using LICENSE file
- Consider `publish = true` to control publishing

**Files**: `receipts/Cargo.toml`

### 2. Documentation Completeness (`receipts/src/lib.rs`)

**Issue**: Some public types lack doc comments

- Add doc comments for: `ReceiptKind`, `DeterminismClass`, `Context`, `Signature`
- Add doc comments for payload structs: `MethodNodeRef`, `Edge`, `ReasoningNodeRef`, `ReasoningQuality`, `MethodRef`, `ExecutionRoots`, `ResourceVector`, `SpendPointers`
- Add examples to key types (Receipt, Payload variants)

**Files**: `receipts/src/lib.rs`

### 3. Validation Enhancements (`receipts/src/validation.rs`)

**Issue**: Missing format validations for some fields

- Validate `timestamp` format (RFC3339) in `Context`
- Validate `currency` format (ISO-4217, 3 chars) in `SpendPayload`
- Validate `version` format (semver-like) in `Receipt`
- Validate UUID format in `links` array
- Consider validating `policy_ref`, `identity_ref` formats if they have standards

**Files**: `receipts/src/validation.rs`

### 4. Schema Validation Integration (`receipts/src/validation.rs`)

**Issue**: `SchemaViolation` error variant exists but isn't used

- Integrate JSON Schema validation into `validate_payload()`
- Load schemas and validate payloads against them
- Return `SchemaViolation` errors when schema checks fail
- This ensures runtime validation matches schema definitions

**Files**: `receipts/src/validation.rs`, potentially new `receipts/src/schema.rs`

### 5. API Convenience Functions (`receipts/src/lib.rs` or new module)

**Issue**: No helper functions for common operations

- Consider adding: `Receipt::builder()` or constructors for each kind
- Consider: `Receipt::from_json()` convenience wrapper
- Consider: `Receipt::verify_signature()` if signature verification is needed
- Keep API minimal - only add if engine will need these

**Files**: `receipts/src/lib.rs` or `receipts/src/helpers.rs` (if needed)

### 6. Golden Vectors Verification

**Issue**: Need to verify all vectors are present and match current implementation

- Verify all 6 receipt kinds have vectors
- Run `cargo test --test test_vector_integrity` to ensure all pass
- Verify `test_drift_detection` baselines are correct
- Check that vectors demonstrate all optional fields

**Files**: `vectors/*.json`, `receipts/tests/test_drift_detection.rs`

### 7. Error Handling Completeness (`receipts/src/error.rs`)

**Issue**: Some error variants may be unused

- Verify all `ValidationError` variants are used
- Consider adding `source()` implementation for error chaining if needed
- Ensure error messages are clear and actionable

**Files**: `receipts/src/error.rs`

### 8. Test Coverage Review

**Issue**: Need to verify edge cases are covered

- Review test suites for gaps:
- Invalid hash formats
- Invalid UUIDs
- Invalid timestamp formats
- Invalid currency codes
- Empty required arrays
- Null in required fields
- Add tests for any missing edge cases

**Files**: `receipts/tests/test_edge_cases.rs`, potentially new tests

### 9. README Completeness (`receipts/README.md`)

**Issue**: May need additional examples or clarification

- Verify all examples compile
- Add examples for less common operations (composition, parallel chains)
- Add troubleshooting section if common issues exist
- Link to spec documents

**Files**: `receipts/README.md`

## Implementation Priority

**High Priority** (Must do before moving to engine):

1. Cargo.toml metadata (#1)
2. Documentation completeness (#2)
3. Validation enhancements (#3)
4. Golden vectors verification (#6)

**Medium Priority** (Should do, but not blocking):

5. Schema validation integration (#4)
6. Test coverage (#8)

**Low Priority** (Nice to have, can add later):

7. API convenience functions (#5) - only if engine needs them
8. Error handling polish (#7)
9. README enhancements (#9)

## Questions to Resolve

1. Do we need runtime JSON Schema validation, or is test-time validation sufficient?
2. Should we add convenience constructors/builders, or keep API minimal?
3. Are there specific format standards for `policy_ref`, `identity_ref` that need validation?
4. Should `Cargo.lock` be committed for this library crate?

## Success Criteria

- All tests pass
- All golden vectors validate
- Cargo.toml has complete metadata
- All public items have doc comments
- Core validations (hash, format, composition) are complete
- README is clear and examples work
- No TODO/FIXME comments in production code
- Ready for engine integration without breaking changes

### To-dos

- [ ] Add complete Cargo.toml metadata (description, repository, documentation, keywords)
- [ ] Add doc comments to all public types (ReceiptKind, DeterminismClass, Context, Signature, payload structs)
- [ ] Add format validations (timestamp RFC3339, currency ISO-4217, version, UUIDs)
- [ ] Integrate JSON Schema validation into validate_payload() using SchemaViolation errors
- [ ] Verify all golden vectors are present, valid, and pass integrity tests
- [ ] Review and enhance test coverage for edge cases (invalid formats, empty arrays, nulls)
- [ ] Update README with additional examples and verify all code examples compile
- [ ] Run full test suite and verify no regressions, update drift detection baselines if needed