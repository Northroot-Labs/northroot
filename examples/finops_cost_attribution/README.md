# FinOps Cost Attribution - Real End-to-End Example

This example demonstrates a complete end-to-end workflow for FinOps cost attribution with real data processing, storage persistence, reuse decisions, and receipt generation.

## Overview

This example processes billing data from CSV files, aggregates costs by account, and demonstrates:

1. **Real Data Processing**: Reads actual CSV billing data and performs real aggregations
2. **Manifest Generation**: Generates input and output manifests with chunking
3. **Storage Integration**: Stores receipts and manifests in SQLite database
4. **Reuse Decisions**: Computes overlap with previous executions and makes reuse/recompute decisions
5. **Receipt Generation**: Creates complete execution receipts with all required fields

## Prerequisites

- Rust toolchain (1.91.0 or later)
- Cargo

## Running the Example

```bash
cargo run --example finops_cost_attribution
```

## Data Files

The example uses two billing data files:

- `data/billing_run1.csv`: Initial billing run with 5 records
- `data/billing_run2.csv`: Second billing run with overlap (3 common records, 2 new)

### Data Schema

```csv
account_id,service,region,resource_type,cost_usd,usage_units
```

## Workflow

### Run 1: Initial Processing

1. Reads `billing_run1.csv`
2. Generates input manifest (chunked data)
3. Computes input data shape hash
4. Processes data: aggregates costs by account
5. Writes output to `output/aggregated_run1.csv`
6. Generates output manifest
7. Computes output data shape hash
8. Creates execution receipt (no previous execution)
9. Stores receipt and manifests in database

### Run 2: Processing with Overlap

1. Reads `billing_run2.csv`
2. Queries database for previous execution (same method shape)
3. Computes overlap: Jaccard similarity between resource tuples
4. Makes reuse decision based on overlap and cost model
5. Creates execution receipt linked to Run 1
6. Stores receipt and manifests

## Output

The example produces:

- **Receipts**: Stored in SQLite database (`northroot.db`)
- **Manifests**: Stored in database (input and output manifests)
- **Aggregated Results**: CSV files in `output/` directory
- **Console Output**: Detailed execution information including:
  - Reuse decisions
  - Jaccard similarity (J)
  - Economic delta (ΔC)
  - Receipt IDs and hashes

## Example Output

```
=== RUN 1: Processing Billing Data ===
Input: examples/finops_cost_attribution/data/billing_run1.csv
  Read 5 billing records
  Input manifest: 1 chunks, 250 bytes
  Aggregated to 3 accounts
  No previous execution found (first run)

  REUSE DECISION:
    Jaccard similarity (J): 0.0000
    Economic delta (ΔC): $-0.1000
    Decision: Recompute

=== RUN 2: Processing Billing Data ===
  Found previous execution: 019a7968-368d-7ac1-975d-bb6754c95c3c

  REUSE DECISION:
    Previous chunks: 5
    Current chunks: 5
    Intersection: 3
    Union: 7
    Jaccard similarity (J): 0.4286
    Economic delta (ΔC): $3.7571
    Decision: Reuse

=== VERIFICATION ===
✓ Run 2 receipt links to Run 1: true
✓ Both receipts stored in database
✓ Manifests stored for both runs
✓ Reuse decisions computed from real data
```

## Key Features Demonstrated

### 1. Real Data Processing

- CSV file reading and parsing
- Data aggregation (group by account, sum costs)
- Output file generation

### 2. Manifest Generation

- Input manifest: Chunks input data and builds Merkle tree
- Output manifest: Chunks output data and builds Merkle tree
- Manifest roots stored in receipts

### 3. Storage Integration

- SQLite database for receipts and manifests
- Receipt storage with PAC (Proof-Addressable Cache) indexing
- Manifest storage with compression support
- Query for previous executions

### 4. Reuse Decision Logic

- Extracts resource tuples from billing data
- Computes chunk sets (deterministic chunk IDs)
- Computes Jaccard similarity between current and previous chunk sets
- Makes reuse/recompute decision based on cost model
- Computes economic delta (savings from reuse)

### 5. Receipt Generation

- Complete execution receipts with:
  - Method shape hash (PAC)
  - Input/output data shape hashes
  - Manifest roots
  - MinHash sketches
  - Links to previous executions
  - Reuse justification

## Database Schema

The example creates a SQLite database with:

- **receipts**: Stores execution receipts with PAC indexing
- **manifests**: Stores input/output manifests (compressed)
- **output_digests**: Indexes receipts by output digest
- **manifest_summaries**: Stores MinHash sketches and HLL cardinality

## Extending the Example

To extend this example:

1. **Add more data files**: Create additional billing runs to demonstrate more reuse scenarios
2. **Add more processing**: Implement additional aggregations or transformations
3. **Add validation**: Verify receipts and manifests after storage
4. **Add querying**: Query receipts by various criteria (PAC, policy, timestamp)

## Notes

- The example uses a simplified overlap computation (reads previous CSV file)
- In production, chunk sets would be stored in execution receipts
- The cost model is loaded from policy files (see `policies/finops/cost-attribution@1.json`)
- Receipts use CBOR canonicalization (RFC 8949) for deterministic hashing

## Related Documentation

- [ADR-0010: v0.1.0 Release Readiness Evaluation](../../docs/adr/ADR-0010-v010-release-readiness-evaluation/)
- [ADR-0008: Proof-Addressable Storage](../../docs/adr/ADR-0008-proof-addressable-storage-and-incremental-compute-/)

