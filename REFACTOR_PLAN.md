# Receipt Population Cleanup Plan

**Goal:** Clean up receipt population to only populate what's needed for verifiable compute, while keeping structure extensible for future features.

**Constraint:** Receipt structure is FROZEN for v0.1.0 - cannot change struct definitions, only what we populate.

## Changes

### 1. Remove Redundant `output_digest`
- **Current:** Always set to `data_shape_hash` (redundant with `dom`)
- **Change:** Set to `None` (already redundant, not needed)
- **Impact:** Low - field is optional, tests already handle None
- **Files:** `crates/northroot-engine/src/api.rs`

### 2. Keep `trace_seq_root` Optional
- **Current:** Always computed and set to `Some(...)`
- **Change:** Keep computation but make it truly optional (already `Option<String>`)
- **Note:** Some tests expect it, so we'll keep computing it for now but document it's optional
- **Future:** Can be made None if not needed, but requires test updates

### 3. Document Core vs Optional Fields
- **Add:** Documentation in `api.rs` explaining what's core vs optional
- **Core:** Fields needed for verifiable compute and composition
- **Optional:** Fields populated by policy/features when needed

## Implementation Steps

1. ✅ Analyze tests and golden vectors
2. Remove `output_digest` population (set to None)
3. Add documentation for core vs optional fields
4. Run full test suite
5. Verify golden vectors still work
6. Update API documentation

## Testing Strategy

- Run all engine tests: `cargo test --package northroot-engine`
- Run all receipt tests: `cargo test --package northroot-receipts`
- Verify golden vectors: Check that existing receipts still deserialize
- Run SDK tests: Ensure SDK still works

## Backward Compatibility

- All changes are backward compatible (only removing redundant data)
- Existing receipts will still deserialize (None fields are optional)
- No breaking changes to API surface

