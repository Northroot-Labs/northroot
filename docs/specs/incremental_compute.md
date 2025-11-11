Here’s a tight, strategy-first spec you can drop in. It defines what “delta compute” means in your unified algebra, how it relates to methods & operators, and where policy fits—without requiring PAC or any envelope/kind changes.

⸻

Incremental Compute Strategy — v0.1

Status: Draft • Scope: How to express and verify delta vs full recomputation decisions in receipts using existing kinds (data_shape, method_shape, execution, spend) with policy-driven thresholds.
Non-goals: No new receipt kind; no PAC; no cache network.

0) Purpose

Provide a composable optimization that:
	•	Recomputes only changed portions of data,
	•	Records determinism & reuse decisions inside existing receipts,
	•	Keeps the algebra stable (no breaking changes),
	•	Moves the decision logic into policy, not ad-hoc code.

1) Roles & Responsibilities
	•	Operators (atomic transforms)
Declare parameters that enable incremental behavior (e.g., mode: "full"|"delta", row_hash_scheme, numeric pins). Operators do the work.
	•	Methods (DAGs of operators)
Compose operators; unchanged. A method can support incremental strategy if its nodes expose the right params and stable outputs (e.g., chunk index, state).
	•	Policies (strategy control)
Decide when delta is allowed/required, how overlap is measured, and determinism constraints. Bound via ctx.policy_ref on receipts.
	•	Receipts (facts)
Record what happened (execution) and why (spend.justification), plus supporting shapes (data/method).

2) Determinism Pins (must be fixed up-front)

Pinned in operator manifests (and optionally referenced by policy):
	•	Text normalization: UTF-8; LF endings; no trailing spaces; header rules.
	•	Hashing: sha256 over canonical bytes.
	•	Numerics: fp64 with rounding: tiesToEven, overflow: error, NaN/Inf: forbid (or pinned alternatives).
	•	Canonicalization: CBOR deterministic encoding (RFC 8949) for all committed bytes.

3) Overlap & Decision Rule

Let J ∈ [0,1] be an overlap estimate between prior input state and current input state (e.g., Jaccard over row-hash sets).

Reuse rule (from algebra §7):
\text{Reuse (delta) iff}\quad J \;>\; \frac{C_{id}}{\alpha\,C_{comp}}
	•	C_id — cost to identify overlap (diff/scan/IO).
	•	C_comp — baseline full compute cost.
	•	α — operator incrementality factor (0–1).

Policy supplies thresholds and how to measure/estimate J, C_id, C_comp, α.

4) Receipt Fields (additive, optional)

No envelope/kind change. Add optional fields as follows:

4.1 data_shape (unchanged, optional sketches)
	•	payload.schema_hash: unchanged.
	•	payload.sketch_hash?: canonical sketches used for overlap (e.g., HLL/minhash over row hashes).

4.2 method_shape (unchanged)
	•	Operators’ param_schema include incremental params (mode, row_hash_scheme, etc.).

4.3 execution (advisory meta only)

Add under payload.meta (all optional):

{
  "mode": "full" | "delta",
  "overlap_j": 0.73,
  "prev_state_hash": "sha256:...",
  "delta_ref": "sha256:..."   // content-hash of canonical delta payload, if emitted
}

4.4 spend (economic reasoning; optional)

Under payload.justification:

{
  "decision": "reuse" | "recompute",
  "overlap_j": 0.73,
  "alpha": 0.9,
  "c_id": 1.2,
  "c_comp": 10.0
}

Validators treat these as advisory: they must be well-formed but don’t affect PoX acceptance.

5) Strategy Contract (what an “incremental-capable” method must expose)

A method can claim incremental-capable if:
	1.	It includes an operator that emits a stable partition index (e.g., chunk_index with per-row hashes).
	2.	It includes an operator that emits a state commitment (e.g., Merkle Row-Map root + state_hash).
	3.	The method DAG documents the flow: partition → compute_state.
	4.	Operators pin determinism (Section 2).

This is a label in docs/README, not a schema change.

6) Policy: minimal scaffold (spec)

Purpose: control when delta is allowed and how overlap is computed.

Identifier: pol:<namespace>/<name>@<version>, referenced from receipt.ctx.policy_ref.

Policy payload (conceptual, engine-readable):

{
  "schema_version": "policy.delta.v1",
  "policy_id": "acme/reuse_thresholds@1",
  "determinism": "strict",
  "overlap": {
    "measure": "jaccard:row-hash",          // or "sketch:minhash:128"
    "min_sample": 0,                         // 0 => exact; else sample size
    "tolerance": 0.0
  },
  "cost_model": {
    "c_id": { "type": "constant", "value": 1.0 },
    "c_comp": { "type": "linear", "per_row": 0.00001, "base": 0.5 },
    "alpha": { "type": "constant", "value": 0.9 }
  },
  "decision": {
    "rule": "j > c_id/(alpha*c_comp)",
    "fallback": "recompute",                 // if metrics unavailable
    "bounds": { "min_rows": 1, "max_rows": 1000000000 }
  },
  "constraints": {
    "forbid_nan": true,
    "header_policy": "require",
    "row_hash_scheme": "sha256-per-row"
  }
}

Notes
	•	This is a policy document, not a receipt.
	•	You can store policies in a simple JSON registry; no special kind required.
	•	The engine reads policy_ref, loads policy JSON, and logs the decision into spend.justification.

7) Operator Pins (example)
	•	acme.frame.partition_rows@1
Params: {format:"csv", has_header:true, row_hash_scheme:"sha256-per-row"}
Output: chunk_index = [{row, hash}] (canonicalized)
	•	acme.frame.inc_sum@1
Params: {column, mode:"full"|"delta", row_hash_scheme}
Outputs: {sum, state:{row_map_root,...}, state_hash}

(These match your earlier operator examples; no schema changes needed.)

8) Worked Receipt Sketch (illustrative)

execution (delta run)

{
  "rid": "urn:uuid:...",
  "version": "0.3.0",
  "kind": "execution",
  "dom": "sha256:...plan...",
  "cod": "sha256:...exec...",
  "ctx": {
    "policy_ref": "acme/reuse_thresholds@1",
    "timestamp": "2025-11-08T16:05:00Z",
    "determinism": "strict",
    "identity_ref": "did:key:zOrg"
  },
  "payload": {
    "trace_id": "tr_01XYZ",
    "method_ref": { "method_id":"acme/inc_sum_pipeline","version":"1.0.0","method_shape_root":"sha256:..." },
    "data_shape_hash": "sha256:...csv-schema...",
    "span_commitments": ["sha256:...partition...", "sha256:...incsum..."],
    "roots": { "trace_set_root":"sha256:...", "identity_root":"sha256:..." },
    "meta": {
      "mode": "delta",
      "overlap_j": 0.73,
      "prev_state_hash": "sha256:...state_a...",
      "delta_ref": "sha256:...delta_bytes..."
    }
  },
  "sig": { "...": "..." },
  "hash": "sha256:..."
}

spend (decision recorded)

{
  "rid": "urn:uuid:...",
  "version": "0.3.0",
  "kind": "spend",
  "dom": "sha256:...exec...",
  "cod": "sha256:...spend...",
  "ctx": {
    "policy_ref": "acme/reuse_thresholds@1",
    "timestamp": "2025-11-08T16:05:01Z",
    "determinism": "strict",
    "identity_ref": "did:key:zOrg"
  },
  "payload": {
    "meter": { "vcpu_sec": 0.4, "gb_sec": 0.0, "requests": 2 },
    "unit_prices": { "vcpu_sec": 0.12, "gb_sec": 0.0, "requests": 0.0 },
    "currency": "USD",
    "total_value": 0.048,
    "pointers": { "trace_id": "tr_01XYZ", "span_ids": ["sp1","sp2"] },
    "justification": {
      "decision": "reuse",
      "overlap_j": 0.73,
      "alpha": 0.9,
      "c_id": 1.0,
      "c_comp": 10.0
    }
  },
  "sig": { "...": "..." },
  "hash": "sha256:..."
}

9) Compliance & Validation
	•	Receipts: unchanged validators apply; added fields are optional but must validate if present (number bounds, string formats).
	•	Policy: validate separately (simple JSON Schema) before execution; engine logs the ruling in spend.justification.
	•	Composition: still data_shape → method_shape → (reasoning_shape?) → execution → spend → settlement.

10) Where to place in repo

docs/specs/incremental_compute_strategy_v0.1.md   ← this file
policies/examples/reuse_thresholds.acme.v1.json   ← example policy doc (above)
receipts/schemas/                                  ← (no schema changes required)
engine/src/delta/{partition_rows.rs,row_map.rs,inc_sum.rs}  ← behavior

11) Quick test plan
	•	Full run vs delta run produce identical sum and consistent state_hash when recomputing full on the new data.
	•	execution.payload.meta.mode reflects chosen path; overlap_j matches engine’s estimator.
	•	spend.payload.justification satisfies the rule in the referenced policy.
	•	Changing policy flips the decision boundary without touching operators/methods.

⸻