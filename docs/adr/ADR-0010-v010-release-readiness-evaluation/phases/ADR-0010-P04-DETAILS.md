# ADR-0010-P04: Real End-to-End Example - Detailed Task Breakdown

This document provides the complete, detailed task breakdown for Phase 4 (P04) of ADR-0010: Real End-to-End Example with Actual Data Processing.

**Note**: This phase depends on ADR-0010-P02 (Python SDK) being complete, or can use Rust APIs directly as fallback.

## Task 1: Example Selection and Analysis
**Location**: `examples/`
**Dependencies**: ADR-0010-P01 complete (foundation phase)
**Estimated Effort**: 0.5 day
**Priority**: **HIGH** - Required for v0.1.0 (demonstrates real value)

### Subtasks:
1. Review existing examples:
   - `examples/finops_cost_attribution/` (RECOMMENDED)
   - `examples/etl_partition_reuse/`
   - `examples/analytics_dashboard/`

2. Select example based on:
   - Complexity (not too simple, not too complex)
   - Real-world applicability
   - Storage integration needs
   - Reuse decision demonstration potential

3. Analyze current example:
   - Identify simulation vs real work
   - Identify missing storage integration
   - Identify missing manifest generation
   - Identify missing receipt persistence
   - Document gaps

4. Create example enhancement plan:
   - List of changes needed
   - Data requirements
   - Storage requirements
   - Documentation requirements

## Task 2: Real Data Preparation
**Location**: `examples/finops_cost_attribution/data/` (or selected example)
**Dependencies**: Task 1 complete
**Estimated Effort**: 1 day

### Subtask 2.1: Data Source Selection
1. Choose appropriate data source:
   - Real CSV billing data (anonymized if needed)
   - Or generate realistic synthetic data
   - Or use public dataset

2. Data requirements:
   - Sufficient size to demonstrate reuse (multiple runs)
   - Realistic structure (columns, types)
   - Multiple partitions/files if applicable
   - Timestamps for change epochs

### Subtask 2.2: Data Preparation Script
1. Create data preparation script:
   - Download/generate data
   - Clean and validate data
   - Split into partitions if needed
   - Create metadata files

2. Script should:
   - Be reproducible
   - Document data schema
   - Include data validation
   - Handle missing data gracefully

### Subtask 2.3: Data Documentation
1. Document data:
   - Schema description
   - Sample records
   - Data size and characteristics
   - Privacy considerations (if real data)

## Task 3: Real Compute Work Implementation
**Location**: `examples/finops_cost_attribution/src/main.rs` (or selected example)
**Dependencies**: Task 2 complete
**Estimated Effort**: 2-3 days

### Subtask 3.1: Replace Simulation with Real Processing
1. Identify simulation code:
   - Hardcoded results
   - Mock computations
   - Placeholder logic

2. Implement real processing:
   - Actual CSV/Parquet file reading
   - Real aggregations (sum, group by, etc.)
   - Real transformations (filtering, mapping, etc.)
   - Real output generation

3. Example for FinOps:
   ```rust
   // Real processing
   let billing_data = read_csv("data/billing.csv")?;
   let aggregated = billing_data
       .group_by("account_id")
       .aggregate("cost", "sum")
       .aggregate("usage", "sum")
       .collect()?;
   
   let output = write_parquet(aggregated, "output/cost_by_account.parquet")?;
   ```

### Subtask 3.2: Data Shape Computation
1. Compute input data shape:
   - Read input files
   - Compute data shape hash
   - Store shape metadata

2. Compute output data shape:
   - After processing
   - Compute output shape hash
   - Store shape metadata

3. Integrate with engine:
   - Use `compute_data_shape_hash_from_file()`
   - Use `compute_data_shape_hash_from_bytes()`
   - Store shapes in execution context

### Subtask 3.3: Method Shape Computation
1. Compute method shape:
   - Extract operator logic
   - Compute method shape hash
   - Store in execution context

2. Integrate with engine:
   - Use `compute_method_shape_hash_from_code()`
   - Or `compute_method_shape_hash_from_signature()`
   - Store in receipt

## Task 4: Storage Integration
**Location**: `examples/finops_cost_attribution/src/main.rs`
**Dependencies**: Task 3 complete
**Estimated Effort**: 2-3 days

### Subtask 4.1: Storage Backend Setup
1. Initialize storage:
   - Create SQLite database
   - Initialize schema
   - Set up connection

2. Storage configuration:
   - Database path
   - WAL mode enabled
   - Connection pooling (if needed)

### Subtask 4.2: Receipt Storage
1. Store receipts after execution:
   - Generate receipt
   - Store via `ReceiptStore::store_receipt()`
   - Verify storage success
   - Handle storage errors

2. Receipt retrieval:
   - Query previous receipts
   - Filter by criteria (trace_id, method_ref, etc.)
   - Load receipt data
   - Verify receipt integrity

### Subtask 4.3: Manifest Storage
1. Generate manifests:
   - For input data
   - For output data
   - Store manifest summaries

2. Store manifests:
   - Store full manifests
   - Store manifest summaries (MinHash, HLL, Bloom)
   - Link manifests to receipts

### Subtask 4.4: Output Digest and Locator Storage
1. Compute output digests:
   - Hash output files
   - Store output digests
   - Link to receipts

2. Store encrypted locators:
   - Create locator references
   - Store encrypted locators
   - Link to receipts

## Task 5: Reuse Decision Demonstration
**Location**: `examples/finops_cost_attribution/src/main.rs`
**Dependencies**: Task 4 complete
**Estimated Effort**: 2-3 days

### Subtask 5.1: Previous Execution Lookup
1. Query for previous executions:
   - Same method shape
   - Similar data shape
   - Within time window (if applicable)

2. Load previous execution data:
   - Previous receipt
   - Previous manifest
   - Previous output digest

### Subtask 5.2: Overlap Computation
1. Compute overlap with previous execution:
   - Load current chunk set
   - Load previous chunk set
   - Compute Jaccard similarity
   - Use manifest summaries for fast path

2. Integrate with reuse reconciliation:
   - Use `ReuseReconciliation::check_reuse()`
   - Get overlap estimate
   - Get exact overlap if needed

### Subtask 5.3: Reuse Decision and Execution
1. Make reuse decision:
   - Evaluate cost model
   - Compute threshold
   - Make decision (reuse/recompute)

2. Execute based on decision:
   - If reuse: Load previous output
   - If recompute: Execute computation
   - Generate new receipt
   - Store results

3. Demonstrate economic delta:
   - Compute savings from reuse
   - Display in output
   - Include in receipt justification

### Subtask 5.4: Multiple Run Scenario
1. Create scenario with multiple runs:
   - First run: Full computation
   - Second run: Partial reuse (some overlap)
   - Third run: Full reuse (high overlap)
   - Fourth run: No reuse (no overlap)

2. Demonstrate reuse decisions:
   - Show overlap values
   - Show decisions
   - Show economic deltas
   - Show receipts for each run

## Task 6: Manifest Generation
**Location**: `examples/finops_cost_attribution/src/main.rs`
**Dependencies**: Task 5 complete
**Estimated Effort**: 1-2 days

### Subtask 6.1: Input Manifest Generation
1. Generate input manifest:
   - Chunk input data
   - Build manifest
   - Compute manifest root
   - Store manifest

2. Generate manifest summary:
   - Compute MinHash sketch
   - Compute HLL cardinality
   - Compute Bloom filter
   - Store summary

### Subtask 6.2: Output Manifest Generation
1. Generate output manifest:
   - Chunk output data
   - Build manifest
   - Compute manifest root
   - Store manifest

2. Link manifests to receipts:
   - Store manifest hash in receipt
   - Store manifest size
   - Link in storage

## Task 7: Receipt Generation and Validation
**Location**: `examples/finops_cost_attribution/src/main.rs`
**Dependencies**: Task 6 complete
**Estimated Effort**: 1-2 days

### Subtask 7.1: Receipt Generation
1. Build execution payload:
   - Trace ID
   - Method reference
   - Data shape hash
   - Span commitments
   - Execution roots

2. Generate receipt:
   - Create receipt with payload
   - Add reuse justification
   - Add output digest
   - Add manifest root
   - Validate receipt

### Subtask 7.2: Receipt Validation
1. Validate receipt:
   - Call `receipt.validate()`
   - Check all fields
   - Verify signatures (if applicable)
   - Handle validation errors

2. Display receipt:
   - Print receipt summary
   - Show key fields
   - Show reuse decision
   - Show economic delta

## Task 8: Example Documentation
**Location**: `examples/finops_cost_attribution/README.md`
**Dependencies**: Task 7 complete
**Estimated Effort**: 1 day

### Subtask 8.1: Usage Documentation
1. Write README:
   - Overview of example
   - Prerequisites
   - Setup instructions
   - Running instructions
   - Expected output

2. Include examples:
   - Command-line usage
   - Code snippets
   - Output examples
   - Troubleshooting

### Subtask 8.2: Workflow Documentation
1. Document workflow:
   - Step-by-step execution flow
   - Data flow diagram
   - Storage operations
   - Reuse decision flow

2. Document outputs:
   - Receipt structure
   - Manifest structure
   - Storage contents
   - How to verify results

### Subtask 8.3: Integration Guide
1. Document integration points:
   - How to use in other projects
   - How to adapt for other use cases
   - How to extend functionality

2. Document lessons learned:
   - Common pitfalls
   - Best practices
   - Performance considerations

## Success Criteria

Phase 4 (P04) is complete when:

1. ✅ Example processes **real data** (not simulation):
   - Reads actual CSV/Parquet files
   - Performs real computations (aggregations, transformations)
   - Generates real output files

2. ✅ Example uses **storage for persistence**:
   - Stores receipts in database
   - Stores manifests and summaries
   - Stores output digests and locators
   - Can retrieve previous executions

3. ✅ Example **demonstrates reuse decisions**:
   - Computes overlap with previous executions
   - Makes reuse/recompute decisions
   - Shows economic deltas
   - Handles multiple run scenarios

4. ✅ Example generates **complete receipts**:
   - Includes all required fields
   - Includes reuse justification
   - Includes output digests
   - Includes manifest roots
   - Receipts validate successfully

5. ✅ Example is **well-documented**:
   - README with usage instructions
   - Workflow documentation
   - Integration guide
   - Code comments

6. ✅ Example can be **run end-to-end**:
   - Setup is straightforward
   - Execution completes successfully
   - Outputs are verifiable
   - No manual intervention required

## Dependencies

- **Blocks**: 
  - ADR-0010-P05 (Documentation) - Docs need working example
- **Blocked by**: 
  - ADR-0010-P01 (Foundation) - Must have clean compilation
  - ADR-0010-P02 (Python SDK) - If example uses Python SDK (preferred)
  - Can work with Rust APIs directly if Python SDK not ready (fallback)
- **Can run in parallel with**: 
  - ADR-0010-P03 (Property Tests) - No dependencies
- **Priority**: **HIGH** - Required for v0.1.0 (demonstrates real value)

## Risks & Mitigations

- **Risk**: Real data processing is more complex than simulation
  - **Mitigation**: Start with simple processing, add complexity incrementally
  - **Mitigation**: Use existing data processing libraries (polars, arrow, etc.)
  - **Mitigation**: Keep example focused, don't over-engineer

- **Risk**: Storage integration reveals bugs
  - **Mitigation**: Test storage integration separately first
  - **Mitigation**: Use transaction rollback for testing
  - **Mitigation**: Fix storage bugs as they're discovered

- **Risk**: Reuse decisions don't demonstrate value clearly
  - **Mitigation**: Create clear scenario with obvious reuse opportunities
  - **Mitigation**: Add logging/display to show decision process
  - **Mitigation**: Include multiple run scenarios

- **Risk**: Example takes longer than expected
  - **Mitigation**: Use existing example as base
  - **Mitigation**: Focus on one use case, don't try to cover everything
  - **Mitigation**: Defer advanced features to later

## Estimated Timeline

- **Total Effort**: 10-15 days (2-3 weeks)
- **Critical Path**: Tasks 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8
- **Can overlap with**: ADR-0010-P03 (Property Tests) - no dependencies
- **Can start after**: Task 1 of ADR-0010-P02 (PyO3 setup) if using Python SDK
