Merkle Row Map:

Great—here’s a minimal, deterministic Merkle Row-Map you can drop in. It’s RFC-6962–style (domain-separated leaves/nodes), JSON-canonical (JCS), and tuned for numeric aggregation (e.g., incremental sum).

⸻

/docs/specs/merkle_row_map_v1.md

Merkle Row-Map — v1 (deterministic state for incremental compute)

Purpose

Represent a map {row_hash → value} with a single canonical root that:
	•	is independent of insertion order,
	•	is language-agnostic (JCS-canonical bytes),
	•	supports incremental updates (delta recompute),
	•	admits standard inclusion proofs (optional).

Typical use: backing state for acme.frame.inc_sum@1 where row_hash = sha256(canonical_row_bytes) and value is a parsed numeric (e.g., amount).

⸻

Cryptographic primitives
	•	Hash: SHA-256.
	•	Domain separation (RFC-6962 style):
	•	Leaf: 0x00 || leaf_bytes
	•	Node: 0x01 || left_hash || right_hash
	•	Empty tree: defined by the RFC-6962 construction (i.e., no leaves → well-defined “empty root”, see below).

⸻

Canonical encoding

Row key
	•	row_hash is a hex lowercase SHA-256 string with sha256: prefix, e.g. sha256:abcd….

Value
	•	Numeric values are serialized as JSON numbers per RFC 8785 / JCS canonicalization:
	•	no leading +,
	•	no leading zeros (except 0),
	•	finite IEEE-754 preferred; if non-finite required, encode as a string token ("NaN", "+Inf", "-Inf") and pin this in the operator contract (x_numeric.nan_handling="canonical").

Leaf object

A leaf corresponds to one key-value pair rendered as JCS:

{"k":"sha256:<64hex>","v":<number or string token>}

Leaf bytes are JCS(leaf_object) (UTF-8, no trailing newline).

Leaf hash = H( 0x00 || leaf_bytes ).

Rationale: Keeping the key inside the leaf bytes removes ambiguity and makes inclusion proofs portable.

⸻

Tree construction
	1.	Collect leaves from the map: one leaf per distinct row_hash.
	2.	Sort leaves by the string k (lexicographic, bytewise UTF-8).
	3.	Hash leaves as above to get an ordered list of leaf hashes.
	4.	Build parent layers:
	•	Pair hashes left-to-right; each parent = H(0x01 || left || right).
	•	If the layer has an odd count, promote the last hash upward unpaired (standard CT-tree rule: hash pairs; an odd one bubbles up unchanged).
	5.	Root:
	•	No leaves → empty root = SHA-256 of the empty string with leaf domain prefix applied to empty bytes:
	•	i.e., H(0x00 || ""). (This matches the “hash of an empty leaf set” convention. If you prefer pure RFC-6962 empty root H(""), choose it once and encode in the spec; both are acceptable if fixed. For v1 we use H(0x00 || "").)
	•	Otherwise, the final remaining hash at the top is the row_map_root.

Note: Using promotion for odd nodes avoids duplicated hashing and keeps the structure deterministic and simple.

⸻

Inclusion proofs (optional)

To prove k → v:
	•	Provide leaf_bytes (the JCS leaf object), plus
	•	Sibling list siblings[] from leaf to root (left/right order significant).
	•	Verifier recomputes leaf_hash = H(0x00 || leaf_bytes) then folds with siblings to the claimed root.

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
	•	Empty map → row_map_root = H(0x00 || "") (v1 rule).
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

	•	state_hash = sha256(JCS(state)) (used by PAC/delta keys).
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

Leaves (JCS):

L1_bytes = {"k":"sha256:...1111","v":100}
L2_bytes = {"k":"sha256:...2222","v":200}
hL1 = H(0x00 || L1_bytes)
hL2 = H(0x00 || L2_bytes)
root = H(0x01 || hL1 || hL2)

Add a third row k3,v3:
	•	Sort keys [k1,k2,k3]
	•	Hash hL1,hL2,hL3, parent layer: H(0x01||hL1||hL2), odd hL3 promotes, next layer: H(0x01||parent||hL3) → new root.

Delta update touching only k2:
	•	Rebuild hL2', recompute its parents → new root; hL1 and any subtrees not on the path remain intact.

⸻

Implementation checklist
	•	JCS serializer (RFC 8785) for leaf objects.
	•	Hex-lowercase, sha256:-prefixed keys.
	•	RFC-6962 hashing with 0x00/0x01 domain bytes.
	•	Stable sort by k (UTF-8 bytewise).
	•	Odd-promotion rule.
	•	Empty root = H(0x00 || "") (lock this in tests).
	•	Unit tests: (1) order independence, (2) delta localizes recompute, (3) numeric canonicalization, (4) NaN/Inf policy behavior.

⸻

Where this plugs in now
	•	Use row_map_root inside acme.frame.inc_sum@1.state.
	•	Key delta PAC as:
	•	PAC_incsum_delta = H(span_shape_hash_incsum, prev_state_hash, H(JCS(delta)), "delta")
	•	Optional: expose row_map_proof(k) to return a compact inclusion proof for auditing.

If you want test vectors with actual hex next, point me to your engine’s JCS and SHA-256 implementations (leaf construction specifics), and I’ll crank out golden hashes to drop into /testdata/merkle_row_map/.

