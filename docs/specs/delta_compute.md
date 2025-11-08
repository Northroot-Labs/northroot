Delta compute spec : 

Delta Compute — Formal Spec v0.1

Status: Draft • Scope: Reuse decisions for compute and I/O across runs, pipelines, and parties
Ties to receipts: record decisions under Spend.justification (and optionally a reuse_proof).

⸻

1) Objects & Notation
	•	Let a run operate on chunked inputs X = \{x_i\} and produce chunked outputs Y = \{y_j\}.
	•	A pipeline is a DAG of operators \mathcal{O} = \{o_k\} (span-level) forming a method.
	•	For any operator o, define a cost model:
	•	C_{\mathrm{comp}}(o, S): baseline compute cost to (re)execute o on shape/content set S.
	•	C_{\mathrm{id}}(o, S): identity/integration cost to locate, validate, and splice reused results for S.
	•	\alpha(o) \in (0,1]: incrementality factor (how efficiently deltas can be applied by o).
	•	Let U,V be chunk multisets (e.g., prior vs current input chunk IDs).
	•	Define overlap J(U,V)\in[0,1] (default Jaccard):
J(U,V) \;=\; \frac{|U \cap V|}{|U \cup V|}
	•	Weighted overlap (cost-aware): for weights w:U\cup V \rightarrow \mathbb{R}{>0},
J_w(U,V) \;=\; \frac{\sum{c\in U\cap V} w(c)}{\sum_{c\in U\cup V} w(c)}
	•	Typical w(c): byte-size, estimated compute, or empirical time.

Engines may use alternative locality-sensitive metrics (MinHash, SimHash, L2 on sketches) as estimators; verification requires exact membership when claiming strict reuse.

⸻

2) Single-Operator Reuse Decision

For operator o over content set S with prior cache S’:
	•	Reuse rule (locked):
\boxed{\quad \text{Reuse iff}\ \ J(S,S’) \;>\; \frac{C_{\mathrm{id}}(o,S)}{\alpha(o)\, C_{\mathrm{comp}}(o,S)} \quad}
	•	Interpretation
	•	LHS = fraction of work already “covered” by prior results.
	•	RHS = break-even threshold: higher identity cost or lower incrementality demands greater overlap to justify reuse.
	•	Economic delta (savings estimate):
\Delta C \approx \alpha(o)\, C_{\mathrm{comp}}(o,S)\, J \;-\; C_{\mathrm{id}}(o,S)
reuse is rational when \Delta C>0.
	•	Recording (in Spend.justification):
overlap_j = J, alpha = α(o), c_id = C_id, c_comp = C_comp, decision = "reuse"|"recompute"|"hybrid".

⸻

3) Pipeline-Level Reuse (DAG)

Let a pipeline have nodes o_k and edges E. For each node:
	1.	Compute J_k between current and cached effective input chunk sets (after upstream pruning).
	2.	Apply local rule above to decide reuse vs recompute.
	3.	Propagate only delta chunks downstream (skip emission for reused outputs).

Total expected cost:
C_{\mathrm{pipe}} \;=\; \sum_{k}
\begin{cases}
C_{\mathrm{id}}(o_k, S_k) & \text{if reuse} \\
C_{\mathrm{comp}}(o_k, S_k^{\Delta}) & \text{if hybrid (delta)} \\
C_{\mathrm{comp}}(o_k, S_k) & \text{if recompute}
\end{cases}
where S_k^{\Delta} is the subset actually recomputed after upstream reuse.

Topological execution: decide from sources to sinks; a reused parent reduces S_k.

⸻

4) Chunking & Sketching (Inputs and Intermediates)
	•	Chunking policy \chi: stable, deterministic segmentation:
	•	Files: content-defined chunking (CDC, e.g., Rabin), or fixed-size with alignment.
	•	Tables: row-group or partition + columnar stripes; include schema version in chunk IDs.
	•	Streams: windowed segments with sequence numbers.
	•	Chunk ID: sha256(bytes); include versioned shape in the namespace to avoid cross-shape collisions.
	•	Sketches (optional): compact summaries for candidate detection:
	•	MinHash signatures of chunk sets for Jaccard estimation.
	•	HLL for cardinality; Bloom filters for membership.
	•	Always confirm candidates with exact set ops before strict reuse.

⸻

5) Overlap for “Shape Families”

Delta can be evaluated at multiple layers:
	•	Data shape: chunk sets of input bytes (fastest to reuse; strongest determinism).
	•	Method shape: reuse validated compiled plans or operator parameterizations.
	•	Reasoning shape: reuse plan DAGs (advisory; not strictly deterministic).
	•	Execution shape: reuse span outputs when domain/codomain shapes match and lineage is satisfied.

Rule: Higher in the stack ⇒ cheaper to detect, weaker guarantees.
Record the layer used to justify reuse.

⸻

6) Determinism & Tolerances
	•	strict: bit-identical; reuse only when content & operator determinism guarantees hold.
	•	bounded: allow numeric tolerance \varepsilon (e.g., FP ops); record \varepsilon.
	•	observational: only logs/commitments; use for hints, not exact reuse.

Policies MAY restrict acceptable classes per operator or environment.

⸻

7) Multi-Party Netting (Clearing of Work)

Suppose parties p \in \mathcal{P} produce/consume work over a period.
For each equivalence class of work (same method/data shapes to tolerance), accumulate produced units W_p and consumed units D_p (e.g., vCPU·s or priced value).
	•	Net position per class: N_p = W_p - D_p.
	•	Conservation: \sum_p N_p = 0.
	•	Settlement: vector N with pricing P (fixed or auction/benchmark):
\text{cash}_p = P \cdot N_p
	•	Attach to a settlement receipt with net_positions, rules_ref, and referenced wur_refs (work receipts).

This enables proof-exchange: parties share receipts, not raw data, and net the value of reused compute.

⸻

8) Algorithms (Engine-Level)

8.1 Overlap Estimation → Verification

function decide_reuse(o, S_cur, S_prev, cost, alpha):
  // Estimation phase
  J_est = estimate_overlap(S_cur.sketch, S_prev.sketch)
  if J_est < lower_bound_threshold:
      return RECOMPUTE

  // Verification phase (exact)
  U = exact_chunk_set(S_cur)
  V = exact_chunk_set(S_prev)
  J = jaccard(U, V) or weighted_jaccard(U,V,w)

  // Decision rule
  if J > cost.C_id / (alpha * cost.C_comp):
      return REUSE with record(J, alpha, C_id, C_comp)
  else:
      return RECOMPUTE

8.2 Pipeline Walk (Topological)

for node in topo_order(G):
  decide using node’s effective input set (after upstream pruning)
  if REUSE: mark outputs reused; else produce delta outputs

8.3 Multi-Party Netting

group receipts by equivalence class E (method_shape_root, data_shape_hash*, policy, eps)
for each E:
  for each party p:
     W_p = sum(value of compute produced and reused by others)
     D_p = sum(value of compute consumed from others)
     N_p = W_p - D_p
  assert sum_p N_p == 0
  emit settlement receipt with net_positions {p -> N_p}

* Data shape may use sketches/tolerance for equivalence class definition when allowed by policy.

⸻

9) Edge Cases & Safety
	•	False positives from sketches: always confirm with exact sets before strict reuse.
	•	Drifted operators (version/params): treat as different; \alpha \to 0.
	•	Schema changes: chunk ID namespace MUST incorporate schema hash; otherwise reject or map with migration operator (explicit o_{\text{migrate}}).
	•	Small jobs: identity cost dominates ⇒ recompute often wins; rule captures this.
	•	Security/tenancy: cross-party reuse only on anonymized commitments; never share raw chunks unless policy permits. Sign all reuse claims.

⸻

10) What Goes Into Receipts
	•	execution: include chunk-level content hashes in span commitments (or their Merkle roots).
	•	spend.justification:
	•	overlap_j, alpha, c_id, c_comp, decision
	•	layer: "data"|"method"|"reasoning"|"execution" (delta evaluation scope)
	•	optional eps for bounded determinism
	•	settlement:
	•	wur_refs: list of receipts contributing to netting
	•	net_positions: per-party vector
	•	rules_ref: pricing/clearing policy
	•	optional cash_instr for payouts

⸻

10.1) Layer (Delta Evaluation Scope)

layer (string) indicates the semantic level at which overlap J and reuse decisions are computed.

Allowed values (v0.1):
	•	"data" — raw or structured chunk equivalence (chunk IDs, partitions, file blocks)
	•	"method" — identical operator/method shapes (method shape roots, operator parameters)
	•	"reasoning" — shared plan semantics or intent graphs (reasoning DAGs, policy references)
	•	"execution" — verified span outputs from prior runs (span commitments, trace roots)

Purpose:
	•	Audit transparency: verifiers know what kind of equivalence was assumed
	•	Policy scoping: a policy can declare allowed reuse layers (e.g., "strict data-level only")
	•	Economic clarity: when multiple reuse layers stack (e.g., data + execution), you can compute blended savings or weighted justification
	•	Future extensibility: allows adding new layers like "spend" or "semantic" without breaking the spec

Policies MAY constrain allowable layers.

Future versions may add "spend" or "semantic".

Layer	Meaning	What's Compared	Typical Use	Reuse Strength

data	Raw or structured data inputs	chunk IDs, partitions, file blocks	ETL, ingestion, analytics, training data	Strong (bit-level determinism)

method	Operator/method plans	method shape roots, operator parameters	compiled workflows, pipeline DAGs	Medium (plan equivalence, not data)

reasoning	Logical or semantic plans	reasoning DAGs, policy references	AI planning, policy reasoning, inference	Weak (intent-level)

execution	Observed span outputs	span commitments, trace roots	run-to-run reuse of actual outputs	Strong (runtime provenance)

⸻

11) Examples (Qualitative)
	•	High overlap (J≈0.95): nightly ETL with one new partition; \alpha=0.9. Identity cost small ⇒ reuse almost all; recompute only new partition.
	•	Medium overlap (J≈0.4): monthly ledger with moderate churn; reuse some dimension joins and caches; recompute fact table deltas.
	•	Low overlap (J≈0.05): new schema or data domain; \alpha small, C_{\mathrm{id}} high ⇒ recompute.

⸻

12) Compliance with Proof Algebra
	•	Delta decisions do not change envelope semantics.
	•	Reuse affects which spans execute and thus execution.span_commitments, but type safety (dom/cod) and composition laws remain intact.
	•	All economic decisions are transparent via spend.justification.

⸻

13) Tuning & Defaults
	•	Default J: unweighted Jaccard on chunk IDs.
	•	Default w(c): byte-size.
	•	Default \alpha: per-operator class registry (e.g., map/filter 0.9, joins 0.6, ML-train 0.2 unless warm-start).
	•	Default \varepsilon: 0 (strict); allow policy to raise for bounded tasks.

⸻

This spec is “drop-in” with the current receipts model. The engine implements the estimators, exact checks, and decision rule; receipts record the why.