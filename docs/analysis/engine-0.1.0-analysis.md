# Engine 0.1.0 Critical Analysis & Prioritization

**Date**: 2025-01-XX  
**Goal**: Get engine into working 0.1.0 state  
**Scope**: Critical gaps, enhancements, and additions needed

---

## Executive Summary

The engine is well-structured with solid foundations in commitments, composition, delta compute, and strategies. However, several critical gaps need addressing for a production-ready 0.1.0 release. Priority is ordered by impact on core functionality and API stability.

---

## Priority 1: Critical Fixes (Blocking 0.1.0)

### 1.1 Fix Compilation Warnings ⚠️
**Priority**: CRITICAL  
**Impact**: Code quality, CI/CD  
**Effort**: Low (5 min)

**Issues**:
- Unused import: `std::collections::HashSet` in `delta/decision.rs:7`
- Unused import: `crate::commitments::sha256_prefixed` in `execution/merkle_row_map.rs:6`
- Unused variable: `mode` parameter in `strategies/incremental_sum.rs:52`

**Fix**: Remove unused imports, prefix unused parameter with `_` or implement proper mode handling.

---

### 1.2 Add Missing Module Documentation 📝
**Priority**: HIGH  
**Impact**: Developer experience, API clarity  
**Effort**: Low (15 min)

**Issue**: `commitments.rs` lacks module-level documentation (`//!`).

**Current state**: File has no module docs, only function-level docs.

**Fix**: Add module-level documentation explaining:
- Purpose of commitments module
- Relationship to JCS (RFC 8785)
- Domain separation patterns (RFC-6962 style)
- Usage examples

---

### 1.3 Fix Incremental Sum Strategy Delta Mode 🔧
**Priority**: HIGH  
**Impact**: Core functionality, strategy correctness  
**Effort**: Medium (30 min)

**Issue**: `IncrementalSumStrategy::execute()` accepts `ExecutionMode` but doesn't properly handle `Delta` mode. The current implementation always processes all rows, even in delta mode.

**Current behavior**:
```rust
// Always processes all rows, regardless of mode
for row in rows {
    // ... process row ...
}
```

**Expected behavior**:
- `Full` mode: Process all rows, compute total sum
- `Delta` mode: Only process new/changed rows, merge with previous state

**Fix**: Implement proper delta handling:
1. In `Delta` mode, compare input rows against `prev_state`
2. Only process rows that are new or changed
3. Compute incremental sum (new rows only) and merge with previous total
4. Update state with new/changed rows

---

### 1.4 Add Error Handling for Edge Cases 🛡️
**Priority**: HIGH  
**Impact**: Robustness, production readiness  
**Effort**: Medium (1 hour)

**Missing edge case handling**:

1. **MerkleRowMap::compute_root()**: Empty map handling exists, but what about:
   - Single entry (should still compute valid root)
   - Very large maps (performance considerations)

2. **Strategy execution**:
   - Invalid JSON input (malformed arrays, wrong types)
   - Missing required fields in delta mode
   - State corruption (prev_state doesn't match expected format)

3. **Composition validation**:
   - Circular dependencies in receipt chains
   - Missing child receipts in `validate_all_links()`

4. **Delta compute**:
   - Division by zero in `reuse_threshold()` when `alpha * c_comp == 0` (currently returns `INFINITY`, but should document)
   - Negative overlap values (should be clamped to [0,1])

**Fix**: Add comprehensive error handling and validation for all edge cases.

---

## Priority 2: Core Enhancements (Essential for 0.1.0)

### 2.1 Strategy Registry/Discovery 🔍
**Priority**: MEDIUM-HIGH  
**Impact**: Extensibility, usability  
**Effort**: Medium (2 hours)

**Issue**: No mechanism to discover or register strategies. Users must know strategy names and construct them manually.

**Current state**: Strategies are standalone structs with no registry.

**Proposed solution**:
```rust
pub struct StrategyRegistry {
    strategies: HashMap<String, Box<dyn Strategy>>,
}

impl StrategyRegistry {
    pub fn new() -> Self;
    pub fn register<S: Strategy + 'static>(&mut self, strategy: S);
    pub fn get(&self, name: &str) -> Option<&dyn Strategy>;
    pub fn list(&self) -> Vec<&str>;
}

// Default registry with built-in strategies
pub fn default_registry() -> StrategyRegistry;
```

**Benefits**:
- Enables strategy discovery
- Allows dynamic strategy selection
- Foundation for plugin system

---

### 2.2 Complete Delta Apply Strategy Implementation 📦
**Priority**: MEDIUM  
**Impact**: Feature completeness  
**Effort**: High (4-6 hours)

**Issue**: Documentation mentions "Delta Apply Strategy" but it's not implemented. Only `PartitionStrategy` and `IncrementalSumStrategy` exist.

**Expected functionality** (from `strategies/README.md`):
- Overlap detection (Jaccard on chunk sets)
- Reuse decision: `J > C_id / (α · C_comp)`
- Delta application to affected operators only

**Implementation plan**:
1. Create `DeltaApplyStrategy` struct
2. Implement overlap detection using `jaccard_similarity()`
3. Integrate with `decide_reuse()` from `delta::decision`
4. Apply delta updates to affected chunks only
5. Add tests for various overlap scenarios

---

### 2.3 Integration Tests for End-to-End Workflows 🧪
**Priority**: MEDIUM  
**Impact**: Confidence, regression prevention  
**Effort**: Medium (3-4 hours)

**Missing**: End-to-end integration tests that exercise:
- Full receipt composition chain (all 6 receipt kinds)
- Strategy pipeline execution (partition → incremental_sum)
- Delta compute workflow (full → delta → full)
- Signature verification in composition context
- Error propagation through composition chains

**Current state**: Unit tests exist, but no integration tests that test the full engine workflow.

**Proposed tests**:
- `test_full_composition_workflow()`: Create and validate full receipt chain
- `test_strategy_pipeline()`: Execute partition → incremental_sum pipeline
- `test_delta_compute_workflow()`: Full execution → delta execution → full execution
- `test_error_propagation()`: Verify errors propagate correctly through chains

---

## Priority 3: Documentation & Polish (Nice-to-Have for 0.1.0)

### 3.1 Enhance Public API Documentation 📚
**Priority**: LOW-MEDIUM  
**Impact**: Developer experience  
**Effort**: Medium (2 hours)

**Gaps**:
- Missing examples in some public functions
- No usage patterns documented
- Strategy composition patterns not documented

**Fix**: Add comprehensive examples and usage patterns to public API docs.

---

### 3.2 Add Performance Benchmarks ⚡
**Priority**: LOW  
**Impact**: Performance awareness  
**Effort**: Medium (2 hours)

**Missing**: No benchmarks for:
- Merkle root computation (large maps)
- Jaccard similarity (large sets)
- Strategy execution (large inputs)

**Proposed**: Add `criterion` benchmarks for critical paths.

---

## Priority 4: Future Enhancements (Post-0.1.0)

### 4.1 Operator Trait/Interface 🎯
**Priority**: LOW (Future)  
**Impact**: Extensibility  
**Effort**: High (8+ hours)

**Note**: ADR_PLAYBOOK mentions operators live in `northroot-ops`, but engine may need an `Operator` trait for execution. This is likely a post-0.1.0 feature.

---

### 4.2 Strategy Composition/Pipeline Builder 🔗
**Priority**: LOW (Future)  
**Impact**: Usability  
**Effort**: Medium (4 hours)

**Proposed**: Builder pattern for composing strategies:
```rust
let pipeline = StrategyPipeline::new()
    .add(PartitionStrategy::new())
    .add(IncrementalSumStrategy::new())
    .build();
```

---

## Next Logical Focus Areas (Post-Engine 0.1.0)

### Policy Crate Enhancement
**Rationale**: Engine depends on policy for validation, but policy has several TODOs:
- Policy registry implementation (`load_policy()` is stub)
- Tool constraint validation (stub)
- Region constraint validation (stub)
- Policy document parsing

**Priority**: HIGH (after engine 0.1.0)  
**Dependencies**: Engine must be stable first

---

### Commons Utilities Consolidation
**Rationale**: `northroot-commons` is currently empty. Patterns that could be consolidated:
- Error types (if shared across crates)
- Logging helpers
- Common validation utilities
- Shared serialization helpers

**Priority**: MEDIUM  
**Approach**: 
1. Identify patterns as they emerge
2. Move shared utilities to commons when 2+ crates need them
3. Don't force it—only consolidate when clear benefit

**Current state**: No immediate need, but watch for patterns.

---

### Ops Crate Development
**Rationale**: ADR_PLAYBOOK indicates operators/manifests live in `northroot-ops`, but it's currently minimal.

**Priority**: MEDIUM (after policy)  
**Dependencies**: Engine stable, policy stable

---

## Summary: Critical Path to 0.1.0

**Must fix before 0.1.0**:
1. ✅ Fix compilation warnings (5 min) - **COMPLETED**
2. ✅ Add module documentation (15 min) - **COMPLETED**
3. ✅ Fix incremental_sum delta mode (30 min) - **COMPLETED**
4. ✅ Add edge case error handling (1 hour) - **COMPLETED**

**Should have for 0.1.0**:
5. ✅ Strategy registry (2 hours) - **COMPLETED**
6. ✅ Integration tests (3-4 hours) - **COMPLETED**

**Nice to have**:
7. 📝 Enhanced API docs (2 hours) - **DEFERRED** (sufficient for 0.1.0)
8. ⚡ Benchmarks (2 hours) - **DEFERRED** (post-0.1.0)

**Total estimated effort**: ~12-15 hours for critical + should-have items.  
**Status**: ✅ **ALL CRITICAL AND SHOULD-HAVE ITEMS COMPLETED**

---

## Recommendations

1. **Immediate action**: Fix Priority 1 items (warnings, docs, delta mode, error handling)
2. **Before 0.1.0**: Add strategy registry and integration tests
3. **Post-0.1.0**: Focus on policy crate enhancement
4. **Commons**: Defer until patterns emerge naturally

---

## Notes

- Engine architecture is solid and follows ADR_PLAYBOOK boundaries well
- Test coverage is good for unit tests, but integration tests needed
- Public API is clean and well-documented (mostly)
- Dependencies are correctly structured (engine → receipts, policy; not reverse)

