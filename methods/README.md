# Methods

**Method definitions: composable DAGs of operators.**

Methods define reusable computational pipelines as directed acyclic graphs (DAGs) of operators. A method shape is committed in a `method_shape` receipt and referenced by execution receipts.

## Purpose

Methods provide:

- **Composition**: DAGs of atomic operators
- **Reusability**: Stable method IDs and versions
- **Shape commitment**: Method shape root for verification
- **Incremental support**: Methods can declare incremental-capable operators

## Method Structure

A method is defined by:

1. **Method ID**: Stable identifier (e.g., `com.acme/normalize-ledger`)
2. **Version**: Semantic version (e.g., `1.0.0`)
3. **Nodes**: Operator references with span shape hashes
4. **Edges**: Optional DAG structure (from → to)
5. **Root multiset**: SHA-256 of sorted span shape hashes
6. **DAG hash**: Optional canonical DAG commitment

## Method Shape Receipt

A `method_shape` receipt commits to a method definition:

```json
{
  "kind": "method_shape",
  "payload": {
    "nodes": [
      {"id": "partition", "span_shape_hash": "sha256:..."},
      {"id": "inc_sum", "span_shape_hash": "sha256:..."}
    ],
    "edges": [
      {"from": "partition", "to": "inc_sum"}
    ],
    "root_multiset": "sha256:...",
    "dag_hash": "sha256:..."
  }
}
```

## Incremental-Capable Methods

A method can claim incremental support if:

1. It includes operators that emit stable partition indices (e.g., `chunk_index` with per-row hashes)
2. It includes operators that emit state commitments (e.g., Merkle Row-Map root + `state_hash`)
3. The method DAG documents the flow: `partition → compute_state`
4. Operators pin determinism (text normalization, numeric rounding, etc.)

## Example Methods

### Incremental Sum Pipeline

```
partition_rows → inc_sum
```

- **partition_rows**: Emits `chunk_index` with row hashes
- **inc_sum**: Accepts delta input, maintains `row_map_root` state

### ETL Pipeline

```
read_csv → normalize → aggregate → write_parquet
```

## Method References

Execution receipts reference methods via `MethodRef`:

```rust
pub struct MethodRef {
    pub method_id: String,          // "com.acme/normalize-ledger"
    pub version: String,            // "1.0.0"
    pub method_shape_root: String,  // sha256:... (from method_shape receipt)
}
```

## Documentation

- **[Proof Algebra](../docs/specs/proof_algebra.md)**: Unified algebra
- **[Incremental Compute](../docs/specs/incremental_compute.md)**: Delta strategy
- **[Operators](../operators/README.md)**: Atomic operator definitions

