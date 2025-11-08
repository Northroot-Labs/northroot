# Policies

**Policy definitions: strategy control for delta compute and execution constraints.**

Policies control when delta compute is allowed, how overlap is measured, determinism constraints, and cost models. Policies are referenced from receipt contexts via `ctx.policy_ref`.

## Purpose

Policies provide:

- **Strategy control**: When to use delta vs full recompute
- **Overlap measurement**: How to estimate Jaccard similarity
- **Cost models**: Identity cost, compute cost, incrementality factors
- **Determinism constraints**: Strict/bounded/observational requirements
- **Security**: Tenancy, region, privacy constraints

## Policy Structure

A policy document (conceptual JSON):

```json
{
  "schema_version": "policy.delta.v1",
  "policy_id": "acme/reuse_thresholds@1",
  "determinism": "strict",
  "overlap": {
    "measure": "jaccard:row-hash",
    "min_sample": 0,
    "tolerance": 0.0
  },
  "cost_model": {
    "c_id": {"type": "constant", "value": 1.0},
    "c_comp": {"type": "linear", "per_row": 0.00001, "base": 0.5},
    "alpha": {"type": "constant", "value": 0.9}
  },
  "decision": {
    "rule": "j > c_id/(alpha*c_comp)",
    "fallback": "recompute",
    "bounds": {"min_rows": 1, "max_rows": 1000000000}
  },
  "constraints": {
    "forbid_nan": true,
    "header_policy": "require",
    "row_hash_scheme": "sha256-per-row"
  }
}
```

## Policy Identifier

Policies are identified by:

```
pol:<namespace>/<name>@<version>
```

Examples:
- `pol:acme/reuse_thresholds@1`
- `pol:standard-v1`
- `pol:production/strict@2`

## Reuse Decision Rule

Policies define the reuse threshold:

```
Reuse iff J > C_id / (α · C_comp)
```

Where:
- **J**: Jaccard overlap [0,1] (measured per policy)
- **C_id**: Identity/integration cost (from cost_model)
- **C_comp**: Baseline compute cost (from cost_model)
- **α**: Operator incrementality factor (from cost_model)

## Overlap Measurement

Policies specify how to measure overlap:

- **jaccard:row-hash**: Exact Jaccard on row hash sets
- **sketch:minhash:128**: MinHash sketch with 128 signatures
- **sketch:hll**: HyperLogLog cardinality estimation

**Note**: Sketches are for estimation; exact set operations confirm before strict reuse.

## Cost Models

### Identity Cost (`c_id`)

Cost to identify overlap, validate, and splice reused results.

- **constant**: Fixed value (e.g., `1.0`)
- **linear**: `base + per_row * row_count`

### Compute Cost (`c_comp`)

Baseline cost to (re)execute operator.

- **constant**: Fixed value
- **linear**: `base + per_row * row_count`

### Incrementality Factor (`alpha`)

How efficiently deltas can be applied [0,1].

- **constant**: Fixed value (e.g., `0.9` for map/filter, `0.6` for joins)
- **per-operator**: Operator-specific registry

## Determinism Constraints

Policies can require determinism classes:

- **strict**: Bit-identical reproducibility required
- **bounded**: Bounded nondeterminism allowed (float tolerances)
- **observational**: Log/commitment proof only

## Constraints

Additional execution constraints:

- **forbid_nan**: Reject NaN/Inf values
- **header_policy**: CSV header requirements
- **row_hash_scheme**: Required hashing scheme
- **region**: Allowed execution regions
- **tenancy**: Identity/access constraints

## Policy Storage

Policies are stored as JSON documents in a registry (not receipts). The engine:

1. Reads `ctx.policy_ref` from receipt
2. Loads policy JSON from registry
3. Applies policy rules during execution
4. Logs decision in `spend.justification`

## Example Policies

### Strict Production Policy

```json
{
  "policy_id": "production/strict@1",
  "determinism": "strict",
  "overlap": {"measure": "jaccard:row-hash"},
  "cost_model": {
    "c_id": {"type": "constant", "value": 0.5},
    "c_comp": {"type": "linear", "per_row": 0.00001, "base": 1.0},
    "alpha": {"type": "constant", "value": 0.9}
  },
  "constraints": {
    "forbid_nan": true,
    "header_policy": "require"
  }
}
```

### Development Policy (Bounded)

```json
{
  "policy_id": "dev/bounded@1",
  "determinism": "bounded",
  "overlap": {"measure": "sketch:minhash:64", "tolerance": 0.01},
  "cost_model": {
    "c_id": {"type": "constant", "value": 0.1},
    "c_comp": {"type": "linear", "per_row": 0.000005, "base": 0.5},
    "alpha": {"type": "constant", "value": 0.85}
  }
}
```

## Documentation

- **[Incremental Compute](../docs/specs/incremental_compute.md)**: Delta strategy
- **[Delta Compute](../docs/specs/delta_compute.md)**: Formal reuse spec
- **[Proof Algebra](../docs/specs/proof_algebra.md)**: Unified algebra
