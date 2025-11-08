# Strategies

**Composable compute strategies for delta/incremental compute.**

Strategies are reusable patterns for implementing incremental recomputation with reuse decisions. They compose operators and methods to enable efficient delta compute workflows.

## Purpose

Strategies provide:

- **Incremental execution**: Delta mode for operators that support it
- **Reuse decisions**: Policy-driven overlap thresholds
- **State management**: Merkle Row-Map for deterministic state
- **Composition**: Strategies can be combined in pipelines

## Strategy Types

### Partition Strategy

Stable row-based chunking with deterministic hashing.

**Components**:
- Row parsing with canonical encoding
- Per-row SHA-256 hashing
- Stable chunk index generation

**Use case**: First step in incremental pipelines to establish stable row identities.

### Incremental Sum Strategy

State-preserving aggregation with Merkle Row-Map.

**Components**:
- Merkle Row-Map for `{row_hash → value}` state
- Delta updates: added/removed/changed rows
- State commitment: `row_map_root` + `state_hash`

**Use case**: Incremental aggregation over changing datasets.

### Delta Apply Strategy

Efficient updates to previous state.

**Components**:
- Overlap detection (Jaccard on chunk sets)
- Reuse decision: `J > C_id / (α · C_comp)`
- Delta application to affected operators only

**Use case**: Pipeline-level reuse across runs.

## Strategy Contract

A strategy must:

1. **Expose stable partition indices**: Operators emit `chunk_index` with row hashes
2. **Emit state commitments**: Operators produce `state_hash` for delta keying
3. **Document flow**: Method DAG shows `partition → compute_state`
4. **Pin determinism**: Operators declare numeric/IO pins

## Example Strategy: Incremental Sum Pipeline

```
partition_rows → inc_sum
```

### Full Mode

1. **partition_rows**: Parse CSV, hash each row → `chunk_index`
2. **inc_sum**: Sum all rows, build Merkle Row-Map → `{sum, state}`

### Delta Mode

1. **partition_rows**: Parse CSV, hash each row → `chunk_index`
2. **Overlap detection**: Compare current `chunk_index` with previous
3. **Reuse decision**: Apply policy rule (J > threshold?)
4. **inc_sum**: 
   - If reuse: Load `prev_state_hash`, apply delta → `{sum, state}`
   - If recompute: Full sum → `{sum, state}`

## Integration with Receipts

Strategies produce execution receipts with:

- **execution.payload.meta** (optional):
  - `mode`: `"full"` | `"delta"`
  - `overlap_j`: Jaccard overlap [0,1]
  - `prev_state_hash`: Previous state commitment
  - `delta_ref`: Content hash of delta payload

- **spend.payload.justification** (optional):
  - `decision`: `"reuse"` | `"recompute"` | `"hybrid"`
  - `overlap_j`, `alpha`, `c_id`, `c_comp`: Decision parameters

## Policy Integration

Strategies read policies via `ctx.policy_ref`:

1. Load policy JSON from registry
2. Extract cost model (`c_id`, `c_comp`, `alpha`)
3. Apply overlap measurement method
4. Evaluate reuse rule: `J > C_id / (α · C_comp)`
5. Log decision in `spend.justification`

## Merkle Row-Map

Strategies use Merkle Row-Map for deterministic state:

- **Leaf**: `{"k": "sha256:...", "v": <number>}` (JCS-canonical)
- **Tree**: RFC-6962 style (0x00 for leaves, 0x01 for nodes)
- **Root**: Deterministic regardless of insertion order
- **Delta updates**: O(Δ log N) recomputation

See [Merkle Row-Map spec](../../docs/specs/merkle_row_map.md) for details.

## Implementation Notes

### Determinism Pins

All strategies must pin:

- **Text**: UTF-8, LF endings, no trailing spaces
- **Hashing**: SHA-256 over canonical bytes
- **Numerics**: fp64, tiesToEven rounding, forbid NaN/Inf
- **Canonicalization**: JSON JCS (RFC 8785)

### Overlap Estimation → Verification

1. **Estimation**: Use sketches (MinHash, HLL) for fast candidate detection
2. **Verification**: Confirm with exact set operations before strict reuse
3. **Decision**: Apply policy rule with verified overlap

### Pipeline Walk

Topological execution:

1. For each node in topological order:
   - Compute effective input set (after upstream pruning)
   - Estimate overlap with cached state
   - Apply reuse decision rule
   - If reuse: Mark outputs reused; else produce delta outputs

## Testing

Strategies should be tested for:

- **Order independence**: Same inputs → same state regardless of order
- **Delta localization**: Only affected paths recompute
- **Numeric canonicalization**: Deterministic serialization
- **Policy compliance**: Decisions match policy rules

## Documentation

- **[Incremental Compute](../../docs/specs/incremental_compute.md)**: Delta strategy spec
- **[Delta Compute](../../docs/specs/delta_compute.md)**: Formal reuse spec
- **[Merkle Row-Map](../../docs/specs/merkle_row_map.md)**: State structure
- **[Proof Algebra](../../docs/specs/proof_algebra.md)**: Unified algebra
