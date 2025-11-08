# Operators

**Operator manifests: atomic, deterministic transformation units.**

Operators are the atomic building blocks of methods. Each operator defines a stable tool identity, parameter schema, input/output shapes, and determinism pins.

## Purpose

Operators provide:

- **Atomicity**: Single, indivisible transformations
- **Determinism**: Pinned numeric/IO behavior for reproducibility
- **Reusability**: Stable tool IDs and versions
- **Incremental support**: Optional delta mode with state preservation

## Operator Manifest

An operator manifest (v1) defines:

```json
{
  "schema_version": "operator.v1",
  "tool_id": "acme.frame.inc_sum",
  "tool_version": "1",
  "param_schema": {
    "type": "object",
    "properties": {
      "column": {"type": "string"},
      "mode": {"type": "string", "enum": ["full", "delta"]},
      "row_hash_scheme": {"type": "string", "enum": ["sha256-per-row"]}
    }
  },
  "input_shape": {...},
  "output_shape": {...},
  "x_numeric": {
    "numeric_kind": "fp64",
    "fp_mode": {"rounding": "tiesToEven"},
    "nan_handling": "forbid",
    "overflow": "error"
  },
  "x_io": {
    "text_normalization": "UTF-8",
    "line_endings": "LF",
    "header_policy": "require"
  }
}
```

## Determinism Pins

Operators must pin deterministic behavior:

### Numeric Pins (`x_numeric`)

- **numeric_kind**: `fp64` | `decimal128` | `int64` | ...
- **fp_mode.rounding**: `tiesToEven` | `towardZero` | `up` | `down`
- **nan_handling**: `canonical` | `forbid`
- **overflow**: `error` | `wrap` | `saturate`

### I/O Pins (`x_io`)

- **text_normalization**: UTF-8 encoding
- **line_endings**: LF (Unix-style)
- **header_policy**: `require` | `optional` | `forbid`

### Hashing

- **row_hash_scheme**: `sha256-per-row` (canonical bytes per row)

## Incremental Operators

Operators can support incremental/delta mode:

### Parameters

- **mode**: `"full"` | `"delta"` (default: `"full"`)
- **row_hash_scheme**: `"sha256-per-row"` (required for delta)

### State

Incremental operators emit state commitments:

```json
{
  "row_map_root": "sha256:...",
  "row_count": 1234,
  "state_hash": "sha256:..."
}
```

Where `state_hash = sha256(JCS(state))` for delta keying.

## Example Operators

### `acme.frame.partition_rows@1`

Partitions input into rows with stable hashing.

**Params**:
- `format`: `"csv"` | `"json"`
- `has_header`: `true` | `false`
- `row_hash_scheme`: `"sha256-per-row"`

**Output**: `chunk_index = [{row, hash}]` (canonicalized)

### `acme.frame.inc_sum@1`

Incremental sum with Merkle Row-Map state.

**Params**:
- `column`: Column name to sum
- `mode`: `"full"` | `"delta"`
- `row_hash_scheme`: `"sha256-per-row"`

**Output**:
- `sum`: Numeric sum
- `state`: `{row_map_root, row_count, state_hash}`

**Delta mode**: Accepts `prev_state_hash` and `delta` (arrays of row keys/indices).

## Operator References

Methods reference operators via stable IDs:

```json
{
  "node_id": "sum_node",
  "operator_ref": "acme.frame.inc_sum@1",
  "params": {"column": "amount", "mode": "delta"}
}
```

## Documentation

- **[Operator v1 Spec](operator_v1.md)**: Manifest specification
- **[Proof Algebra](../docs/specs/proof_algebra.md)**: Unified algebra
- **[Incremental Compute](../docs/specs/incremental_compute.md)**: Delta strategy
- **[Merkle Row-Map](../docs/specs/merkle_row_map.md)**: State structure

## Schema

See `receipts/schemas/operator_v1_schema.json` for the operator manifest schema.

## Examples

See `receipts/schemas/examples/` for operator manifest examples:
- `operator_frame_inc_sum.json`
- `operator_frame_partition_rows.json`

