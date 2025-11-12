# Phase 4 Preparation: Spec-First Tests and Playbook Alignment

## Classification

**Phase 4 Type**: `additive` (with storage schema changes)

**Rationale**:
- New traits (`ArtifactResolver`, `ManagedCache`) - additive
- New storage tables - additive (existing receipts remain valid)
- New optional fields already added in Phase 1 - backward compatible
- No breaking changes to existing behavior

## Spec-First Tests Created

All spec-first tests are in `crates/northroot-engine/tests/test_phase4_spec.rs`:

### Resolver Trait Contract Tests (6 tests)
- `test_resolver_trait_exists` - Verifies trait exists and is object-safe
- `test_resolver_resolve_locator_signature` - Verifies resolve_locator signature
- `test_resolver_store_artifact_signature` - Verifies store_artifact signature
- `test_resolver_batch_signature` - Verifies batch resolution signature

### ManagedCache Trait Contract Tests (2 tests)
- `test_managed_cache_trait_exists` - Verifies trait exists
- `test_cache_artifact_signature` - Verifies cache_artifact signature

### Storage Extensions Contract Tests (3 tests)
- `test_storage_encrypted_locator_methods` - Verifies new ReceiptStore methods
- `test_storage_output_digest_methods` - Verifies output digest query methods
- `test_storage_schema_has_encrypted_locators_table` - Verifies table structure
- `test_storage_schema_has_output_digests_table` - Verifies table structure
- `test_storage_schema_has_manifest_summaries_table` - Verifies table structure

### Privacy Invariant Tests (2 tests)
- `test_receipts_never_contain_plain_locations` - ✅ Passing (structural invariant)
- `test_encrypted_locator_contains_no_plain_data` - Verifies encryption

### Backward Compatibility Tests (2 tests)
- `test_receipts_without_locators_still_valid` - ✅ Passing (backward compatibility)
- `test_storage_backward_compatibility` - Verifies storage handles old receipts

### Integration Contract Tests (3 tests)
- `test_end_to_end_resolver_flow` - Complete resolver workflow
- `test_output_digest_lookup_flow` - Output digest query workflow
- `test_manifest_summary_storage_flow` - Manifest summary storage workflow

**Total**: 18 spec-first tests (2 passing, 16 ignored until implementation)

## Next Steps for Phase 4

1. **Implement resolver module** (`crates/northroot-engine/src/resolver.rs`)
   - Define `ArtifactResolver` trait
   - Define `ManagedCache` trait
   - Define error types (`ResolverError`, `CacheError`)
   - Define types (`ArtifactLocation`, `ArtifactMetadata`)

2. **Extend storage traits** (`crates/northroot-storage/src/traits.rs`)
   - Add `store_encrypted_locator()` method
   - Add `get_encrypted_locator()` method
   - Add `query_by_output_digest()` method
   - Add `get_output_info()` method

3. **Update storage schema** (`crates/northroot-storage/src/sqlite.rs`)
   - Create `encrypted_locators` table
   - Create `output_digests` table
   - Create `manifest_summaries` table
   - Add indexes and foreign keys

4. **Remove `#[ignore]` from tests** as implementation progresses
   - Tests will fail until implementation is complete
   - Verify each test passes as corresponding feature is implemented

5. **Run cargo-mutants** after implementation
   - Verify test quality
   - Fix any surviving mutants

6. **Add integration tests**
   - End-to-end resolver flow
   - Storage migration tests
   - Backward compatibility verification

## Playbook Compliance

✅ **Pre-Implementation Checklist**:
- [x] Documented expected changes
- [x] Identified affected components
- [x] Reviewed ADR alignment (ADR-009)

✅ **Spec-First Tests**:
- [x] Created 18 spec-first tests
- [x] Marked with `#[ignore]` until implementation
- [x] Documented expected behavior in comments

⏭️ **Implementation Checklist** (To Do):
- [ ] Implement resolver module
- [ ] Extend storage traits
- [ ] Update storage schema
- [ ] Remove `#[ignore]` from tests as features complete
- [ ] Run cargo-mutants
- [ ] Add integration tests
- [ ] Document migration path

⏭️ **Post-Implementation Verification** (To Do):
- [ ] All new tests passing
- [ ] Storage schema migration tested
- [ ] Backward compatibility verified
- [ ] ADR-009 updated with Phase 4 completion

