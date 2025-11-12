# ADR-0010-P02: Python SDK Development - Detailed Task Breakdown

This document provides the complete, detailed task breakdown for Phase 2 (P02) of ADR-0010: Python SDK Development with PyO3 Bindings.

**Note**: This phase depends on ADR-0010-P01 (Foundation) being complete.

## Task 1: PyO3 Project Setup
**Location**: `sdk/northroot-sdk-python/` (NOT `crates/` - see Architecture Decision in ADR-0010)
**Dependencies**: ADR-0010-P01 complete (foundation phase)
**Estimated Effort**: 1-2 days
**Priority**: **CRITICAL** - Required for v0.1.0

### Subtasks:
1. Create new crate structure:
   - `Cargo.toml` with PyO3 dependencies
   - `src/lib.rs` as PyO3 module entry point
   - `src/` directory structure
   - `pyproject.toml` for Python packaging
   - `README.md` with setup instructions

2. Configure PyO3 dependencies:
   - `pyo3 = { version = "0.21", features = ["extension-module", "abi3-py38"] }`
   - `northroot-engine` as dependency
   - `northroot-receipts` as dependency
   - `northroot-storage` as dependency (optional for v0.1.0)

3. Set up build configuration:
   - `maturin` for building Python wheels
   - CI/CD configuration for Python builds
   - Cross-platform build support (Linux, macOS, Windows)

4. Create initial module structure:
   - `src/delta.rs` - Delta compute bindings
   - `src/receipts.rs` - Receipt generation/validation bindings
   - `src/shapes.rs` - Shape hash computation bindings
   - `src/client.rs` - High-level client API (if time permits)

## Task 2: Core Rust Function Bindings
**Location**: `crates/northroot-sdk-python/src/`
**Dependencies**: Task 1 complete
**Estimated Effort**: 3-5 days

### Subtask 2.1: Reuse Decision Bindings
**File**: `src/delta.rs`

1. Expose `decide_reuse()` function:
   - Wrap `northroot_engine::delta::decide_reuse()`
   - Convert Rust types to Python types:
     - `f64` → `float`
     - `CostModel` → Python dict or custom class
     - `ReuseDecision` → Python enum/string
     - `ReuseJustification` → Python dict
   - Handle error conversion (`thiserror::Error` → Python exceptions)
   - Add Python docstrings

2. Expose `economic_delta()` function:
   - Wrap `northroot_engine::delta::economic_delta()`
   - Type conversions as above
   - Error handling

3. Expose `jaccard_similarity()` function:
   - Wrap `northroot_engine::delta::overlap::jaccard_similarity()`
   - Convert `HashSet<String>` ↔ Python `set[str]`
   - Handle empty sets edge case

### Subtask 2.2: Shape Hash Computation Bindings
**File**: `src/shapes.rs`

1. Expose `compute_data_shape_hash_from_bytes()`:
   - Wrap `northroot_engine::delta::compute_data_shape_hash_from_bytes()`
   - Convert Python `bytes` → Rust `&[u8]`
   - Convert `ChunkScheme` → Python dict/enum
   - Return Python string (sha256:... format)

2. Expose `compute_data_shape_hash_from_file()`:
   - Wrap `northroot_engine::delta::compute_data_shape_hash_from_file()`
   - Convert Python `str`/`PathLike` → Rust `Path`
   - Handle file I/O errors

3. Expose `compute_method_shape_hash_from_code()`:
   - Wrap `northroot_engine::delta::compute_method_shape_hash_from_code()`
   - Convert Python dict → `serde_json::Value`
   - Handle JSON serialization errors

4. Expose `compute_method_shape_hash_from_signature()`:
   - Wrap `northroot_engine::delta::compute_method_shape_hash_from_signature()`
   - Convert Python list → Rust `Vec<&str>`

### Subtask 2.3: Receipt Generation Bindings
**File**: `src/receipts.rs`

1. Expose receipt generation helpers:
   - Wrap `northroot_receipts::Receipt` creation
   - Convert Python dict → `ExecutionPayload`
   - Handle validation errors

2. Expose receipt validation:
   - Wrap `northroot_receipts::Receipt::validate()`
   - Convert validation errors to Python exceptions

3. Expose receipt serialization:
   - CBOR serialization → Python bytes
   - JSON serialization → Python dict
   - Handle serialization errors

## Task 3: High-Level Python API
**Location**: `crates/northroot-sdk-python/src/`
**Dependencies**: Task 2 complete
**Estimated Effort**: 5-7 days

### Subtask 3.1: Decorator Pattern Implementation
**File**: `src/decorator.rs` or Python module

1. Implement `@delta_compute` decorator:
   ```python
   @delta_compute(
       operator="partition_rows",
       alpha=0.95,
       cost_model={"c_id": 0.1, "c_comp": 10.0}
   )
   def process_partition(data: pd.DataFrame) -> pd.DataFrame:
       # Existing processing logic
       return transformed_data
   ```

2. Decorator functionality:
   - Wrap function execution
   - Extract function metadata (name, signature)
   - Compute method shape hash
   - Track input data shape
   - Call reuse decision logic
   - Generate receipt on completion
   - Inject receipt into function context (if needed)

3. Error handling:
   - Catch and wrap Rust errors
   - Provide meaningful Python exceptions
   - Preserve stack traces

### Subtask 3.2: Context Manager Pattern
**File**: `src/context.rs` or Python module

1. Implement `DeltaContext` class:
   ```python
   with DeltaContext(
       operator="group_by_account",
       policy_ref="acme/reuse_thresholds@1"
   ) as ctx:
       result = df.groupby("account_id").sum()
       ctx.emit_receipt(result)
   ```

2. Context manager functionality:
   - Initialize reuse reconciliation
   - Track execution context
   - Store intermediate state
   - Emit receipt on exit
   - Handle exceptions gracefully

3. Integration with storage:
   - Optional storage backend
   - Receipt persistence
   - Manifest storage

### Subtask 3.3: Operator Wrapper Pattern
**File**: `src/operator.rs` or Python module

1. Implement `DeltaOperator` class:
   ```python
   partition_op = DeltaOperator(
       name="partition_rows",
       alpha=0.95,
       strategy="partition"
   )
   
   result, receipt = partition_op.execute(
       input_data=df,
       prev_state=prev_state_hash
   )
   ```

2. Operator wrapper functionality:
   - Encapsulate operator logic
   - Manage state transitions
   - Handle reuse decisions
   - Return results + receipt tuple
   - Support async execution (if time permits)

## Task 4: Python Unit Tests
**Location**: `crates/northroot-sdk-python/tests/` or `tests/`
**Dependencies**: Tasks 1-3 complete
**Estimated Effort**: 2-3 days

### Subtask 4.1: Core Function Tests
1. Test `decide_reuse()`:
   - Test with various overlap values
   - Test with different cost models
   - Test edge cases (0.0, 1.0 overlap)
   - Test error handling

2. Test `jaccard_similarity()`:
   - Test with identical sets
   - Test with disjoint sets
   - Test with empty sets
   - Test with large sets

3. Test shape hash functions:
   - Test with various data types
   - Test with files
   - Test with bytes
   - Test error handling

### Subtask 4.2: High-Level API Tests
1. Test decorator:
   - Test function wrapping
   - Test receipt generation
   - Test error propagation
   - Test with pandas DataFrames (if integrated)

2. Test context manager:
   - Test normal execution
   - Test exception handling
   - Test receipt emission
   - Test storage integration

3. Test operator wrapper:
   - Test execute method
   - Test state management
   - Test reuse decisions
   - Test result + receipt tuple

### Subtask 4.3: Integration Tests
1. End-to-end workflow:
   - Create receipt
   - Store receipt
   - Retrieve receipt
   - Validate receipt

2. Cross-language compatibility:
   - Verify Rust ↔ Python type conversions
   - Verify error propagation
   - Verify serialization round-trips

## Task 5: Python API Documentation
**Location**: `crates/northroot-sdk-python/docs/` or inline docstrings
**Dependencies**: Tasks 1-4 complete
**Estimated Effort**: 1-2 days

### Subtask 5.1: Docstrings
1. Add comprehensive docstrings to all public functions/classes
2. Include usage examples in docstrings
3. Document type signatures
4. Document error conditions

### Subtask 5.2: API Reference
1. Generate API reference from docstrings (Sphinx or mkdocs)
2. Include usage examples
3. Include migration guide (if applicable)
4. Include troubleshooting section

## Success Criteria

Phase 2 (P02) is complete when:

1. ✅ PyO3 project structure is set up and builds successfully
2. ✅ Core Rust functions are exposed via PyO3 bindings:
   - `decide_reuse()`
   - `jaccard_similarity()`
   - `compute_data_shape_hash_from_bytes()`
   - `compute_method_shape_hash_from_code()`
   - Receipt generation/validation
3. ✅ At least one high-level Python API pattern is implemented:
   - Decorator OR context manager OR operator wrapper
4. ✅ Python unit tests pass for all exposed functions
5. ✅ Python API documentation is generated and reviewable
6. ✅ Python SDK can be installed via `pip install` (local or PyPI)

## Dependencies

- **Blocks**: 
  - ADR-0010-P04 (Real End-to-End Example) - Example needs Python SDK
  - ADR-0010-P05 (Documentation) - Docs need SDK to exist
- **Blocked by**: 
  - ADR-0010-P01 (Foundation) - Must have clean compilation and error handling
- **Can run in parallel with**: 
  - ADR-0010-P02 (Property Tests) - No dependencies between them
- **Priority**: **CRITICAL** - Required for v0.1.0 launch readiness

## Risks & Mitigations

- **Risk**: PyO3 bindings are complex and time-consuming
  - **Mitigation**: Start with core APIs only, expand in 0.2.0
  - **Mitigation**: Leverage existing PyO3 patterns and examples
  - **Mitigation**: Focus on one high-level pattern first (decorator recommended)

- **Risk**: Type conversions between Rust and Python are error-prone
  - **Mitigation**: Comprehensive unit tests for type conversions
  - **Mitigation**: Use PyO3's built-in type conversions where possible

- **Risk**: Python packaging and distribution is complex
  - **Mitigation**: Use `maturin` for simplified builds
  - **Mitigation**: Start with local installation, defer PyPI to later

## Estimated Timeline

- **Total Effort**: 12-19 days (2.5-4 weeks)
- **Critical Path**: Tasks 1 → 2 → 3 → 4 → 5
- **Parallel Work**: Can start Task 4 (tests) while Task 3 is in progress

