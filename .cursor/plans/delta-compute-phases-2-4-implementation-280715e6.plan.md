<!-- 280715e6-caa1-4edf-a45c-54f4f7d54541 e708e8a8-fabb-4517-8821-3e36085fca12 -->
# Architectural Refactoring: Policy-Driven Cost Models

## Overview

Refactor the delta compute implementation to align with ADR_PLAYBOOK.md architectural boundaries. The primary goal is to make cost models policy-driven rather than hardcoded, implement policy loading from file system, and ensure proper separation of concerns between `northroot-policy`, `northroot-engine`, and `northroot-ops`.

## Current Issues

1. **CostModel in wrong crate**: `CostModel` is defined in `engine::delta::decision` but should be policy-driven
2. **Policy loading is stub**: `load_policy()` returns `PolicyNotFound` error
3. **Examples hardcode CostModel**: All examples create `CostModel::new()` directly instead of loading from policy
4. **northroot-ops is empty**: Only placeholder, needs minimal types to match schemas
5. **Architectural drift**: Cost models should come from policy, not be constructed in engine

## Refactoring Steps

### Phase 1: Policy Types and Loading (northroot-policy)

**File**: `crates/northroot-policy/src/cost_model.rs` (new)

- Define `CostModel` struct matching policy JSON structure:
  ```rust
  pub struct CostModel {
      pub c_id: CostValue,
      pub c_comp: CostValue,
      pub alpha: CostValue,
  }
  
  pub enum CostValue {
      Constant(f64),
      Linear { per_row: f64, base: f64 },
  }
  ```

- Add method to extract `CostModel` from policy JSON
- Add validation for cost model values (non-negative, alpha in [0,1])

**File**: `crates/northroot-policy/src/policy.rs` (new)

- Define `DeltaComputePolicy` struct matching spec from `docs/specs/incremental_compute.md`:
  ```rust
  pub struct DeltaComputePolicy {
      pub schema_version: String,
      pub policy_id: String,
      pub determinism: Option<String>,
      pub overlap: OverlapConfig,
      pub cost_model: CostModelDefinition,
      pub decision: DecisionRule,
      pub constraints: Option<PolicyConstraints>,
  }
  ```

- Implement `From<serde_json::Value>` for `DeltaComputePolicy`

**File**: `crates/northroot-policy/src/validation.rs`

- Update `load_policy()` to:

  1. Parse policy_ref to extract namespace/name/version
  2. Load JSON from file system: `policies/{namespace}/{name}@{version}.json`
  3. Parse and validate policy structure
  4. Return `DeltaComputePolicy` instead of `serde_json::Value`

- Add `extract_cost_model(policy: &DeltaComputePolicy, row_count: Option<usize>) -> Result<CostModel, PolicyError>`
  - Evaluates `CostValue` variants (Constant vs Linear) based on row_count
  - Returns concrete `CostModel` with f64 values

**File**: `crates/northroot-policy/src/lib.rs`

- Export new types: `DeltaComputePolicy`, `CostModel`, `CostValue`
- Export `extract_cost_model()` function

**File**: `schemas/policy/delta_compute_v1_schema.json` (new)

- Create JSON schema matching policy structure from spec
- Validate against `docs/specs/incremental_compute.md` lines 100-126

**File**: `policies/` directory (new)

- Create example policy files:
  - `policies/finops/cost-attribution@1.json`
  - `policies/etl/partition-reuse@1.json`
  - `policies/analytics/dashboard@1.json`
- Each should match the policy structure from spec

### Phase 2: Engine Integration (northroot-engine)

**File**: `crates/northroot-engine/src/delta/decision.rs`

- **Remove** `CostModel` struct definition (move to policy)
- **Update** `decide_reuse()` signature to accept `&northroot_policy::CostModel` instead of `&CostModel`
- **Update** `economic_delta()` signature similarly
- **Update** `decide_reuse_with_layer()` signature similarly
- Keep `ReuseDecision` enum (this is engine logic, not policy)

**File**: `crates/northroot-engine/src/delta/mod.rs`

- Update exports to remove `CostModel`
- Add helper function: `load_cost_model_from_policy(policy_ref: &str, row_count: Option<usize>) -> Result<northroot_policy::CostModel, PolicyError>`
  - Calls `northroot_policy::load_policy()` and `extract_cost_model()`
  - Convenience wrapper for engine use

**File**: `crates/northroot-engine/src/strategies/incremental_sum.rs`

- Update `CostModel` type to `northroot_policy::CostModel`
- Update `with_cost_model()` to accept `northroot_policy::CostModel`
- Update cost allocation computation to use policy-driven cost model

**File**: `crates/northroot-engine/src/lib.rs`

- Remove `CostModel` from public exports
- Update documentation to indicate cost models come from policy

**File**: `crates/northroot-engine/tests/test_delta.rs`

- Update all tests to:

  1. Create policy JSON files in test fixtures
  2. Load policies via `load_policy()`
  3. Extract cost models via `extract_cost_model()`
  4. Use policy-driven cost models in tests

### Phase 3: Update Examples

**File**: `examples/finops_cost_attribution/main.rs`

- Remove `CostModel::new()` direct construction
- Load policy via `ctx.policy_ref`: `load_policy("pol:finops-cost-attribution@1")`
- Extract cost model: `extract_cost_model(&policy, Some(resource_tuples.len()))`
- Use policy-driven cost model in `decide_reuse()`

**File**: `examples/etl_partition_reuse/main.rs`

- Same pattern: load policy, extract cost model, use in decisions

**File**: `examples/analytics_dashboard/main.rs`

- Same pattern: load policy, extract cost model, use in decisions

### Phase 4: Stub Ops Model (northroot-ops)

**File**: `crates/northroot-ops/src/operator.rs` (new)

- Define minimal `OperatorManifest` struct matching `schemas/receipts/operator_v1_schema.json`:
  ```rust
  pub struct OperatorManifest {
      pub schema_version: String,
      pub tool_id: String,
      pub tool_version: String,
      pub param_schema: serde_json::Value,
      pub input_shape: serde_json::Value,
      pub output_shape: serde_json::Value,
      // ... other fields from schema
  }
  ```

- Add `From<serde_json::Value>` implementation
- Add basic validation (required fields present)

**File**: `crates/northroot-ops/src/lib.rs`

- Export `OperatorManifest`
- Add note: "Minimal implementation - full validation and methods TBD"

**File**: `crates/northroot-ops/README.md`

- Update to indicate minimal stub implementation
- Note that schemas exist and full implementation will come when needed

### Phase 5: Update Tests and Documentation

**File**: `crates/northroot-policy/tests/test_cost_model.rs` (new)

- Test `extract_cost_model()` with constant values
- Test `extract_cost_model()` with linear values (various row counts)
- Test validation (negative costs, alpha out of range)

**File**: `crates/northroot-policy/tests/test_policy_loading.rs` (new)

- Test loading policy from file system
- Test policy parsing and validation
- Test error cases (file not found, invalid JSON, missing fields)

**File**: `crates/northroot-engine/tests/test_delta.rs`

- Update all tests to use policy-driven cost models
- Ensure tests still pass with new approach

**File**: `docs/ADR_PLAYBOOK.md`

- Verify no changes needed (architecture already correct, just implementation)

## Migration Notes

- **Backward compatibility**: Examples will break until policies are created

-

### To-dos

- [ ] Implement Delta Lake CDF Scan Operator: Add CdfMetadata struct, extend ExecutionPayload with cdf_metadata field, update execution schema
- [ ] Implement CDF drift detection: Add CdfDriftDetector, detect_cdf_drift() function, test_cdf_range_drift_detection() test
- [ ] Add MinHash sketches to FinOps integration: Extend ReuseJustification with minhash_sketch, update spend schema, add compute_minhash_sketch() helper, drift detection for >5% divergence
- [ ] Implement cost allocation in IncrementalSumStrategy: Add CostAllocation struct, compute_cost_allocation() method, integrate ΔC computation into execute()
- [ ] FinOps Cost Attribution Pilot: Create integration module, instrument billing runs, emit receipts with MinHash sketches, integration test
- [ ] ETL Partition-Based Reuse Pilot: Create Delta Lake CDF integration, implement partition-level reuse, emit receipts per partition, integration test
- [ ] Analytics Dashboard Refresh Pilot: Create BI tool integration, implement incremental refresh, emit receipts per query, integration test
- [ ] Python SDK: Create package structure, implement @delta_compute decorator, DeltaContext manager, DeltaOperator wrapper, Rust FFI bindings, tests
- [ ] Spark Integration: Create DeltaUDF class, implement Spark job receipt emission, integration test
- [ ] Dagster Integration: Create NorthrootAsset wrapper, implement materialization hooks, policy-driven decisions, integration test
- [ ] Update implementation_steps.md: Mark completed criteria, add completion dates, link to code, document deviations