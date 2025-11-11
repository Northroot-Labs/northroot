Unified proof algebra : 

Northroot Proof Algebra — v0.1 (Fresh Rewrite)

Status: Draft • Scope: Unified receipt algebra for verifiable work
Audience: Engine/SDK implementers • License: Apache-2.0 (intended)

⸻

0. Goals
	•	One algebra to express all proofs as typed, signed morphisms over shapes.
	•	One receipt envelope; multiple kind-specific payloads (modular).
	•	Composable by construction (sequential / parallel).
	•	Determinism + delta compute hook for reuse decisions.
	•	Stable canonicalization for cross-lang verification.

⸻

1. Core Sets & Commitments
	•	Let \mathcal{S} be the set of shapes (canonical, hashable structures).
	•	Each shape S has a canonical byte encoding \llbracket S \rrbracket \in \{0,1\}^*
(CBOR canonicalization: RFC 8949 deterministic encoding, stable DAG encoding, RFC3339 UTC timestamps).
	•	Commitment:
C(S) \;=\; \texttt{“sha256:”} \,\|\, \mathrm{hex}(\mathrm{SHA256}(\llbracket S \rrbracket))

Kinds (levels):

data_shape        # schema + optional sketches
method_shape      # operator contracts (multiset/DAG)
reasoning_shape   # decision/plan DAG over tools
execution         # span-commitment set/sequence roots
spend             # metered resources + pricing
settlement        # multi-party netting state


⸻

2. Morphisms (Transformations over Shapes)

A proofable step is a typed morphism:
f : (S, k) \longrightarrow (S’, k’)
with domain/codomain shapes S,S’ and kinds k,k’.

Composition:
	•	Sequential: (g \circ f):(S,k)\to(U,m) when codf=domg.
	•	Parallel (tensor): f \otimes g acts on product shapes (independent branches).

This forms a strict symmetric monoidal category (\mathcal{S}, \circ, \otimes, I).

Identity morphism: \mathrm{id}_{(S,k)}:(S,k)\to(S,k).

⸻

3. Receipts as Evidence (Unified Envelope)

A Receipt is the signed evidence of a morphism. One envelope, typed by kind.

3.1 Envelope (canonical body)

{
  "rid": "urn:uuid:...",                      // UUIDv7 recommended
  "version": "0.3.0",                         // envelope version
  "kind": "data_shape|method_shape|reasoning_shape|execution|spend|settlement",
  "dom": "sha256:...",                        // commitment of domain shape
  "cod": "sha256:...",                        // commitment of codomain shape
  "links": ["urn:uuid:...", "..."],           // child receipts for composition (optional)
  "ctx": {
    "policy_ref": "pol:...@v",
    "timestamp": "2025-11-07T00:00:00Z",
    "nonce": "base64url...",
    "determinism": "strict|bounded|observational",
    "identity_ref": "did:key:..."
  },
  "payload": { /* kind-specific (see §4) */ },
  "attest": { /* optional TEE/container attestations */ },
  "sig": { "alg":"ed25519", "kid":"did:key:...", "sig":"base64url..." },
  "hash": "sha256:..."                        // commitment of canonical body (without sig/hash)
}

Verification invariant: hash == sha256(canonical(body_without_sig_hash)).

⸻

4. Kind Payloads (Shape Definitions)

Each kind has a strict payload schema; envelope stays constant.

4.1 data_shape
	•	Shape: schema + optional sketches (stats).
	•	Payload:
	•	schema_hash: sha256:<64hex>
	•	sketch_hash?: sha256:<64hex>
	•	Morphism: \bot \to S_{\text{data}} (introduction).

4.2 method_shape
	•	Shape: operator contracts arranged as multiset or DAG.
	•	Payload:
	•	nodes: [{id, span_shape_hash}]
	•	edges?: [{from,to}]
	•	root_multiset: sha256:...  // H(sorted(span_shape_hash))
	•	dag_hash?: sha256:...      // H(canonical DAG)
	•	Morphism: S_{\text{data,in}} \to S_{\text{data,out}}.

4.3 reasoning_shape
	•	Shape: decision/plan DAG over tools, bound to intent/policy.
	•	Payload:
	•	intent_hash: sha256:...
	•	dag_hash: sha256:...
	•	node_refs: [{node_id, operator_ref, pox_ref?}]
	•	policy_ref?: string
	•	quality?: {success_score?, eval_method?, review_hash?, confidence?}
	•	Morphism: S_{\text{method*}} \to S_{\text{plan}}.

4.4 execution
	•	Shape: observable run structure (set/sequence of span commitments).
	•	Payload:
	•	trace_id: string
	•	method_ref: {method_id, version, method_shape_root}
	•	data_shape_hash: sha256:...
	•	span_commitments: [sha256:...]
	•	roots: {trace_set_root, identity_root, trace_seq_root?}
	•	Morphism: S_{\text{plan}} \to S_{\text{exec}}.

4.5 spend
	•	Shape: resource vector + pricing map (accounting state).
	•	Payload:
	•	meter: {vcpu_sec?, gpu_sec?, gb_sec?, requests?, energy_kwh?}
	•	unit_prices: {same keys}
	•	currency: "USD"|ISO-4217
	•	pricing_policy_ref?: string
	•	total_value: number  // dot(meter, unit_prices) ± ε
	•	pointers: {trace_id, span_ids?}
	•	justification?: {overlap_j?, alpha?, c_id?, c_comp?, decision?}
	•	Morphism: S_{\text{exec}} \to S_{\text{spend}}.

4.6 settlement
	•	Shape: multi-party position vector with clearing rules.
	•	Payload:
	•	wur_refs: [string]         // work-unit/receipt refs
	•	net_positions: {party -> amount}
	•	rules_ref: string
	•	cash_instr?: {...}          // ACH/stablecoin/credits
	•	Morphism: S_{\text{spend}}^{n} \to S_{\text{cleared}}.

⸻

5. Composition Laws (Engine MUST Enforce)
	1.	Sequential type safety: cod(R_i) == dom(R_{i+1}).
	2.	Hash safety: All referenced commitments recompute from canonical encodings.
	3.	Signature safety: All sig entries verify over hash.
	4.	Policy safety: ctx.policy_ref constraints satisfied (tools, regions, tolerances, privacy).
	5.	Monoidal (parallel) safety: For parallel composition, use tensor commitment:
C(S_1 \otimes S_2) = \mathrm{sha256}(\text{sorted}(C(S_1),C(S_2))\ \text{joined with} \ “|”)
Parent may store a Merkle root over child rids in its payload or links.

Identity receipts set dom == cod with a no-op payload.

⸻

6. Determinism Classes
	•	strict — bit-identical outputs reproducible.
	•	bounded — bounded nondeterminism (float tolerances, seeded RNG).
	•	observational — execution/log proof (no reproducibility claim).

Policies may constrain acceptable classes per environment.

⸻

7. Delta Compute Hook (Reuse Decision)

Let J(S,S’) \in [0,1] measure overlap (e.g., Jaccard on chunk sets, weighted by cost).
Let C_{id} be identity/integration cost; C_{comp} baseline compute cost; \alpha operator incrementality factor.

Reuse rule (locked):
\text{Reuse iff}\quad J \;>\; \frac{C_{id}}{\alpha\,C_{comp}}

Record decision and parameters in spend.justification or in a separate reuse_proof (future).

Applicable at: data_shape, method_shape, reasoning_shape, execution (span sets).

⸻

8. Verification Algorithm (High Level)

For a receipt R:
	1.	Recompute hash from canonical body → must equal R.hash.
	2.	Verify sig(s) over hash (and attest if present).
	3.	Validate payload by kind rules (hash formats, units, DAG integrity, dot product).
	4.	If part of a chain/graph:
	•	Check cod(R_i) == dom(R_{i+1}).
	•	Verify parent ↔ child consistency via links and optional Merkle root.

Any subchain that satisfies the above is independently valid and reusable.

⸻

9. API Surface (Minimal)
	•	POST /v1/receipts — ingest any kind; accepts JSON (converted to CBOR internally for storage).
	•	GET /v1/receipts/{rid} — fetch by id; returns JSON or CBOR based on Accept header.
	•	GET /v1/receipts?kind=&dom=&cod=&policy_ref=&trace_id= — query.
	•	POST /v1/compose/seq — check cod==dom.
	•	POST /v1/compose/tensor — return tensor commitment + Merkle bundle.

Content types:
	•	application/vnd.northroot.receipt+json (external API, adapter layer)
	•	application/vnd.northroot.receipt+cbor (internal storage, canonical format)
	•	(Optional) legacy endpoints map to/from unified receipts.

⸻

10. Versioning & Compatibility
	•	Envelope version (here 0.3.0 in examples) changes only on canonicalization or envelope structure change.
	•	Kind payloads version inside their schemas; evolve additively.
	•	Receipts MUST validate against the envelope version and their kind schema version.

⸻

11. Security & Identity
	•	Signers use ed25519 (default). kid SHOULD be did:key or equivalent.
	•	attest MAY carry TEE/container quotes; policies can require them.
	•	identity_ref is bound into execution/spend/settlement contexts (tenancy, role).

⸻

12. Testing (Golden Vectors)

Provide test vectors (JSON format for readability, converted to CBOR internally):
	•	data_shape, method_shape, reasoning_shape, execution, spend, settlement
	•	A sequential chain with matching dom/cod
	•	Parallel (tensor) example with Merkle bundle
	•	All vectors use CBOR canonicalization (RFC 8949) for hash computation

Engines/SDKs MUST round-trip hash and pass kind validators.

⸻

13. Practical Mapping (Typical Chain)

data_shape (⊥ → S_data)
   → method_shape (S_data → S'_data)
   → reasoning_shape (S_method* → S_plan)
   → execution (S_plan → S_exec)
   → spend (S_exec → S_spend)
   → settlement (Σ_i S_spend → S_cleared)

Each arrow is a receipt; the chain is a verifiable DAG of work and value.

⸻

14. Notes on Unification
	•	“Unification” = one envelope + one algebra across kinds.
	•	Keep kinds modular; avoid over-generalized payloads.
	•	Later (v0.3+), reasoning_shape MAY fold under a generic “shape family” without changing the envelope.

⸻

15. Reserved Identifiers (Initial)
	•	Hash prefix: sha256:
	•	Receipt media type: application/vnd.northroot.receipt+json
	•	Determinism: strict|bounded|observational
	•	Metrics keys in spend.meter: vcpu_sec,gpu_sec,gb_sec,requests,energy_kwh

⸻

16. Non-Goals (v0.1)
	•	ZK compression of receipts (future).
	•	Cross-chain notarization (future).
	•	On-chain settlement instruments (future).

⸻

Appendix A — Canonicalization Rules (Summary)
	•	JSON objects: lexicographically sorted keys; no insignificant whitespace.
	•	Arrays: preserve declared order unless explicitly specified as sets (then sort before hashing).
	•	Timestamps: RFC3339 UTC with millisecond precision.
	•	DAG encoding: stable node order + edge list sorted by (from,to); include node ids and committed attributes only.

⸻

End of v0.1 — This document defines the algebra, envelope, kinds, composition, determinism, and delta compute rule.
Implementation notes, JSON Schemas, and code live with engine and receipts directories. 