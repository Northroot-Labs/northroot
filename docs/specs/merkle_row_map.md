Merkle Row Map:

Great—here's a minimal, deterministic Merkle Row-Map you can drop in. It uses CBOR canonicalization (RFC 8949) with domain-separated hashing (string prefixes), and is tuned for numeric aggregation (e.g., incremental sum).

⸻

/docs/specs/merkle_row_map_v1.md

Merkle Row-Map — v1 (deterministic state for incremental compute)

Purpose

Represent a map {row_hash → value} with a single canonical root that:
	•	is independent of insertion order,
	•	is language-agnostic (CBOR canonical bytes per RFC 8949),
	•	supports incremental updates (delta recompute),
	•	admits standard inclusion proofs (optional).

Typical use: backing state for acme.frame.inc_sum@1 where row_hash = sha256(canonical_row_bytes) and value is a parsed numeric (e.g., amount).

⸻

Cryptographic primitives
	•	Hash: SHA-256.
	•	Domain separation (string prefixes):
	•	Leaf: "leaf:" || cbor_canonical({k, v})
	•	Node: "node:" || left_hash || right_hash
	•	Empty tree: H("leaf:" || "") (well-defined empty root)

⸻

Canonical encoding

Row key
	•	row_hash is a hex lowercase SHA-256 string with sha256: prefix, e.g. sha256:abcd….

Value
	•	Numeric values are serialized as CBOR integers or floats per RFC 8949:
	•	Integers encoded in minimal form
	•	Floats as IEEE-754 double precision
	•	If non-finite required, encode as CBOR tags or string tokens and pin in operator contract

Leaf object

A leaf corresponds to one key-value pair rendered as CBOR:

{"k":"sha256:<64hex>","v":<number>}

Leaf bytes are cbor_canonical(leaf_object) (deterministic CBOR encoding per RFC 8949).

Leaf hash = H( "leaf:" || cbor_canonical({k, v}) ).

Rationale: Keeping the key inside the leaf bytes removes ambiguity and makes inclusion proofs portable.

⸻

Tree construction
	1.	Collect leaves from the map: one leaf per distinct row_hash.
	2.	Sort leaves by the string k (lexicographic, bytewise UTF-8).
	3.	Hash leaves as above to get an ordered list of leaf hashes.
	4.	Build parent layers:
	•	Pair hashes left-to-right; each parent = H("node:" || left || right).
	•	If the layer has an odd count, promote the last hash upward unpaired (standard CT-tree rule: hash pairs; an odd one bubbles up unchanged).
	5.	Root:
	•	No leaves → empty root = H("leaf:" || "").
	•	Otherwise, the final remaining hash at the top is the row_map_root.

Note: Using promotion for odd nodes avoids duplicated hashing and keeps the structure deterministic and simple.

⸻

Inclusion proofs (optional)

To prove k → v:
	•	Provide leaf_bytes (the CBOR canonical leaf object), plus
	•	Sibling list siblings[] from leaf to root (left/right order significant).
	•	Verifier recomputes leaf_hash = H("leaf:" || cbor_canonical({k, v})) then folds with siblings to the claimed root.

⸻

Delta updates

Given previous state (row_map_root_prev) and delta sets:
	•	added:    [k_i → v_i]
	•	removed:  [k_j]
	•	changed:  [k_m → v_m_new] (implicitly removed(k_m_old) + added(k_m_new))

Update algorithm:
	1.	Apply set changes to the map (pure key operations).
	2.	Rebuild affected leaves (only keys in delta).
	3.	Recompute the minimal set of parent hashes along the paths from changed leaves to the root (standard Merkle path recomputation).
	4.	Emit new row_map_root.

Engine tip: If you store the previous tree in a persistent structure (e.g., MST / B-tree with node hashes), delta application is O(Δ log N) nodes.

⸻

Determinism pins (numerics)
	•	x_numeric.numeric_kind: fp64|decimal128|int64|…
	•	x_numeric.fp_mode.rounding: tiesToEven|towardZero|up|down
	•	x_numeric.nan_handling: canonical|forbid
	•	x_numeric.overflow: error|wrap|saturate

Two maps are equivalent iff keys and canonical values match byte-for-byte.

⸻

Empty cases
	•	Empty map → row_map_root = H("leaf:" || "") (v1 rule).
	•	A map with all values removed via delta becomes empty and yields the same root.

⸻

Integration with inc_sum@1

acme.frame.inc_sum@1 state payload:

{
  "column": "amount",
  "row_hash_scheme": "sha256-per-row",
  "row_map_root": "sha256:<64hex>",
  "row_count": 1234
}

	•	state_hash = sha256(cbor_canonical(state)) (used by PAC/delta keys, CBOR canonicalization per RFC 8949).
	•	For delta mode, inputs include:
	•	prev_state_hash (to fetch prior state),
	•	delta (arrays of row indices or keys),
	•	optional inclusion proofs for remote verification scenarios.

⸻

Security notes
	•	Domain separation ensures leaf/node ambiguity is impossible.
	•	Sorting by key prevents malleability from insertion order.
	•	Canonical numeric serialization avoids locale/formatter drift.
	•	For high-assurance modes, forbid NaN/Inf and pin decimal arithmetic.

⸻

/schemas/merkle_row_map.v1.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.northroot.dev/merkle_row_map.v1.schema.json",
  "title": "Merkle Row-Map v1 (state payload)",
  "type": "object",
  "additionalProperties": false,
  "required": ["column", "row_hash_scheme", "row_map_root", "row_count"],
  "properties": {
    "column": { "type": "string" },
    "row_hash_scheme": { "type": "string", "enum": ["sha256-per-row"] },
    "row_map_root": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
    "row_count": { "type": "integer", "minimum": 0 }
  }
}


⸻

Tiny worked example (illustrative)

Two rows (after parsing):

k1 = "sha256:...1111"
v1 = 100
k2 = "sha256:...2222"
v2 = 200

Leaves (CBOR canonical):

L1_bytes = cbor_canonical({"k":"sha256:...1111","v":100})
L2_bytes = cbor_canonical({"k":"sha256:...2222","v":200})
hL1 = H("leaf:" || L1_bytes)
hL2 = H("leaf:" || L2_bytes)
root = H("node:" || hL1 || hL2)

Add a third row k3,v3:
	•	Sort keys [k1,k2,k3]
	•	Hash hL1,hL2,hL3, parent layer: H("node:"||hL1||hL2), odd hL3 promotes, next layer: H("node:"||parent||hL3) → new root.

Delta update touching only k2:
	•	Rebuild hL2', recompute its parents → new root; hL1 and any subtrees not on the path remain intact.

⸻

Implementation checklist
	•	CBOR canonical encoder (RFC 8949) for leaf objects.
	•	Hex-lowercase, sha256:-prefixed keys.
	•	Domain-separated hashing with "leaf:"/"node:" string prefixes.
	•	Stable sort by k (canonical CBOR byte order).
	•	Odd-promotion rule.
	•	Empty root = H("leaf:" || "") (lock this in tests).
	•	Unit tests: (1) order independence, (2) delta localizes recompute, (3) numeric canonicalization, (4) NaN/Inf policy behavior.

⸻

Where this plugs in now
	•	Use row_map_root inside acme.frame.inc_sum@1.state.
	•	Key delta PAC as:
	•	PAC_incsum_delta = H(span_shape_hash_incsum, prev_state_hash, H(cbor_canonical(delta)), "delta")
	•	Optional: expose row_map_proof(k) to return a compact inclusion proof for auditing.

Test vectors with actual hex are available in `vectors/engine/merkle_row_map_examples.json` with CBOR canonicalization hashes.

