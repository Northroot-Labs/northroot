# Proof Synergy Memo: How Receipts Verify Reuse Economics

**Research Agent:** cursor  
**Date:** 2025-01-27  
**Version:** 0.1  
**Namespace:** northroot.research.delta_compute

## Executive Summary

Northroot's receipt-based proof system uniquely enables verifiable reuse economics—cryptographically proving that incremental recomputation decisions were economically justified and correctly executed. This memo explains how receipts verify reuse economics, enabling trustless compute markets and cross-organizational netting.

**Key Insight:** Receipts transform delta compute from a performance optimization into a verifiable economic transaction, enabling auditability, settlement, and trustless markets.

---

## 1. The Reuse Economics Problem

### 1.1 Trust Gap in Incremental Compute

**Current State:**
- Frameworks optimize for performance but lack economic transparency
- Reuse decisions are opaque: no proof of correctness or justification
- Cross-organizational reuse is impossible without trust

**Problem:**
- How do you prove that reuse was economically justified?
- How do you verify that incremental recomputation matches full recomputation?
- How do you enable trustless compute markets?

### 1.2 Receipts as Economic Proofs

**Solution:**
- Receipts record reuse decisions in cryptographically signed documents
- `spend.justification` contains economic parameters (J, α, C_id, C_comp)
- Verifiers can independently validate reuse decisions
- Settlement receipts enable cross-org netting

---

## 2. Receipt Structure for Reuse Economics

### 2.1 Execution Receipt

**Purpose:** Prove what computation was performed

**Key Fields:**
```json
{
  "kind": "execution",
  "payload": {
    "method_ref": "acme/cost_attribution@1.0.0",
    "data_shape_hash": "sha256:...",
    "span_commitments": ["sha256:...", "sha256:..."],
    "meta": {
      "mode": "delta",
      "overlap_j": 0.88,
      "prev_state_hash": "sha256:...",
      "delta_ref": "sha256:..."
    }
  },
  "sig": {...},
  "hash": "sha256:..."
}
```

**Verification:**
- `span_commitments` prove what operators executed
- `meta.mode` indicates delta vs full recomputation
- `meta.overlap_j` records measured overlap
- `meta.prev_state_hash` links to previous execution

### 2.2 Spend Receipt

**Purpose:** Prove economic justification for reuse

**Key Fields:**
```json
{
  "kind": "spend",
  "payload": {
    "meter": {"vcpu_sec": 0.4, "gb_sec": 0.0},
    "unit_prices": {"vcpu_sec": 0.12},
    "total_value": 0.048,
    "justification": {
      "decision": "reuse",
      "overlap_j": 0.88,
      "alpha": 0.87,
      "c_id": 0.5,
      "c_comp": 10.0
    }
  },
  "sig": {...},
  "hash": "sha256:..."
}
```

**Verification:**
- `justification.decision` records reuse/recompute decision
- `justification.overlap_j` proves measured overlap
- `justification.alpha` records operator incrementality
- `justification.c_id` and `justification.c_comp` record cost model
- Verifier can independently compute: J > C_id / (α · C_comp) ✓

### 2.3 Settlement Receipt

**Purpose:** Prove cross-organizational netting

**Key Fields:**
```json
{
  "kind": "settlement",
  "payload": {
    "wur_refs": ["urn:uuid:...", "urn:uuid:..."],
    "net_positions": {
      "acme": 150.0,
      "beta": -75.0,
      "gamma": -75.0
    },
    "rules_ref": "acme/clearing_rules@1"
  },
  "sig": {...},
  "hash": "sha256:..."
}
```

**Verification:**
- `wur_refs` link to work receipts (execution + spend)
- `net_positions` prove conservation: Σ N_p = 0
- Verifier can independently compute net positions from work receipts

---

## 3. Verification Workflows

### 3.1 Reuse Decision Verification

**Workflow:**
1. Load execution receipt with `meta.mode = "delta"`
2. Load spend receipt with `justification.decision = "reuse"`
3. Extract economic parameters: J, α, C_id, C_comp
4. Verify decision rule: J > C_id / (α · C_comp)
5. Verify economic delta: ΔC = α · C_comp · J - C_id > 0

**Example:**
```python
def verify_reuse_decision(execution_receipt, spend_receipt):
    # Extract parameters
    j = spend_receipt.justification.overlap_j
    alpha = spend_receipt.justification.alpha
    c_id = spend_receipt.justification.c_id
    c_comp = spend_receipt.justification.c_comp
    
    # Verify decision rule
    threshold = c_id / (alpha * c_comp)
    assert j > threshold, "Reuse decision invalid"
    
    # Verify economic delta
    delta_c = alpha * c_comp * j - c_id
    assert delta_c > 0, "Reuse not economically justified"
    
    return True
```

### 3.2 Correctness Verification

**Workflow:**
1. Load execution receipt with `meta.mode = "delta"`
2. Load previous execution receipt (via `meta.prev_state_hash`)
3. Recompute full execution on current data
4. Compare delta execution result with full execution result
5. Verify: delta_result == full_result (within tolerance)

**Example:**
```python
def verify_delta_correctness(delta_receipt, prev_receipt, current_data):
    # Recompute full execution
    full_result = execute_full(current_data)
    
    # Reconstruct delta execution
    delta_result = execute_delta(
        current_data,
        prev_state=prev_receipt.payload.meta.state_hash
    )
    
    # Verify equivalence
    assert delta_result == full_result, "Delta execution incorrect"
    
    return True
```

### 3.3 Settlement Verification

**Workflow:**
1. Load settlement receipt
2. Load all work receipts referenced in `wur_refs`
3. Compute net positions per party from work receipts
4. Verify: computed_net_positions == settlement.net_positions
5. Verify conservation: Σ net_positions = 0

**Example:**
```python
def verify_settlement(settlement_receipt, work_receipts):
    # Compute net positions from work receipts
    computed_net = {}
    for receipt in work_receipts:
        party = receipt.ctx.identity_ref
        value = receipt.payload.total_value
        computed_net[party] = computed_net.get(party, 0) + value
    
    # Verify net positions
    assert computed_net == settlement_receipt.net_positions
    
    # Verify conservation
    total = sum(settlement_receipt.net_positions.values())
    assert abs(total) < 0.01, "Conservation violated"
    
    return True
```

---

## 4. Economic Transparency

### 4.1 Audit Trail

**Receipts provide complete audit trail:**
- What computation was performed (execution receipt)
- Why reuse was chosen (spend.justification)
- Economic parameters (J, α, C_id, C_comp)
- Cost savings (economic delta ΔC)

**Use Cases:**
- Finance teams audit cost attribution calculations
- Compliance teams verify compute decisions
- Engineering teams debug reuse decisions

### 4.2 Cost Attribution

**Receipts enable precise cost attribution:**
- Per-operator costs recorded in spend receipts
- Reuse savings quantified in justification
- Cross-team compute credits via settlement receipts

**Example:**
```python
# Query receipts for cost attribution
receipts = query_receipts(
    method_ref="acme/cost_attribution@1.0.0",
    time_range="2025-01-01 to 2025-01-31"
)

# Compute total cost
total_cost = sum(r.payload.total_value for r in receipts)

# Compute reuse savings
reuse_savings = sum(
    r.justification.alpha * r.justification.c_comp * r.justification.overlap_j
    - r.justification.c_id
    for r in receipts
    if r.justification.decision == "reuse"
)

print(f"Total cost: ${total_cost}")
print(f"Reuse savings: ${reuse_savings}")
print(f"Net cost: ${total_cost - reuse_savings}")
```

---

## 5. Trustless Markets

### 5.1 Cross-Organizational Reuse

**Problem:** How do you enable reuse across organizations without trust?

**Solution:** Receipts enable trustless verification

**Workflow:**
1. Party A emits receipt for computation
2. Party B verifies receipt (cryptographic proof)
3. Party B reuses computation if overlap J > threshold
4. Party B emits receipt referencing Party A's receipt
5. Settlement receipt nets compute credits

**Example:**
```python
# Party A: Emit receipt
receipt_a = emit_receipt(
    execution=computation_result,
    method_ref="acme/cost_attribution@1.0.0"
)

# Party B: Verify and reuse
if verify_receipt(receipt_a):
    if receipt_a.justification.overlap_j > threshold:
        # Reuse computation
        result = reuse_computation(receipt_a)
        receipt_b = emit_receipt(
            execution=result,
            method_ref="beta/cost_attribution@1.0.0",
            reuse_ref=receipt_a.rid
        )
```

### 5.2 Compute Credit Markets

**Receipts enable compute credit markets:**
- Work receipts prove compute production/consumption
- Settlement receipts enable netting across parties
- Pricing determined by market or fixed rules

**Example:**
```python
# Generate settlement receipt
settlement = generate_settlement(
    work_receipts=[receipt_a, receipt_b, receipt_c],
    pricing_rules="acme/pricing@1"
)

# Net positions
# acme: +150.0 (produced compute)
# beta: -75.0 (consumed compute)
# gamma: -75.0 (consumed compute)

# Settlement: acme pays beta and gamma
```

---

## 6. Privacy-Preserving Verification

### 6.1 Zero-Knowledge Proofs

**Challenge:** How do you verify reuse without revealing data?

**Solution:** ZK proofs of overlap and reuse decisions

**Workflow:**
1. Generate ZK proof that J > threshold
2. Verify proof without revealing data
3. Emit receipt with ZK proof

**Example:**
```python
# Generate ZK proof of overlap
zk_proof = generate_zk_overlap_proof(
    current_chunks=current_sketch,
    prev_chunks=prev_sketch,
    threshold=0.7
)

# Verify proof
assert verify_zk_overlap_proof(zk_proof)

# Emit receipt with ZK proof
receipt = emit_receipt(
    execution=result,
    zk_proof=zk_proof
)
```

### 6.2 Multi-Party Computation (MPC)

**Challenge:** How do you compute overlap across parties without sharing data?

**Solution:** MPC protocols for private overlap detection

**Workflow:**
1. Parties compute overlap via MPC
2. Reuse decision made without revealing data
3. Receipts record decision with MPC proof

---

## 7. Receipt Composition

### 7.1 Pipeline-Level Receipts

**Challenge:** How do you verify reuse across entire pipelines?

**Solution:** Receipt composition

**Workflow:**
1. Each operator emits receipt
2. Pipeline receipt composes operator receipts
3. Verifier validates entire pipeline

**Example:**
```python
# Operator receipts
receipt_partition = emit_receipt(operator="partition", ...)
receipt_aggregate = emit_receipt(operator="aggregate", ...)

# Pipeline receipt
pipeline_receipt = compose_receipts(
    method_ref="acme/etl_pipeline@1.0.0",
    operator_receipts=[receipt_partition, receipt_aggregate]
)

# Verification
verify_pipeline_receipt(pipeline_receipt)
```

### 7.2 Cross-Run Receipts

**Challenge:** How do you verify reuse across multiple runs?

**Solution:** Receipt chains

**Workflow:**
1. Run 1 emits receipt with state_hash
2. Run 2 references Run 1 receipt via prev_state_hash
3. Verifier validates receipt chain

**Example:**
```python
# Run 1
receipt_1 = emit_receipt(execution=result_1)
state_hash_1 = receipt_1.payload.meta.state_hash

# Run 2 (reuse)
receipt_2 = emit_receipt(
    execution=result_2,
    prev_state_hash=state_hash_1
)

# Verify chain
verify_receipt_chain([receipt_1, receipt_2])
```

---

## 8. Implementation in Northroot

### 8.1 Receipt Emission

**Location:** `crates/northroot-receipts/src/`

**Key Components:**
- `SpendPayload.justification`: ReuseJustification struct
- `ReuseJustification`: Records J, α, C_id, C_comp, decision
- Receipt signing and validation

### 8.2 Reuse Decision Logic

**Location:** `crates/northroot-engine/src/delta/decision.rs`

**Key Functions:**
- `decide_reuse(overlap_j, cost_model)`: Implements decision rule
- `economic_delta(overlap_j, cost_model)`: Computes savings
- Returns `ReuseJustification` for receipt emission

### 8.3 Verification

**Location:** `crates/northroot-receipts/src/validation.rs`

**Key Functions:**
- Receipt validation (signature, schema)
- Reuse decision verification
- Economic delta verification

---

## 9. Benefits Summary

### 9.1 Economic Transparency
- Complete audit trail of reuse decisions
- Quantified cost savings (economic delta)
- Cost attribution per operator/pipeline

### 9.2 Trustless Markets
- Cross-organizational reuse without trust
- Compute credit markets via settlement receipts
- Privacy-preserving verification (ZK, MPC)

### 9.3 Compliance & Audit
- Verifiable proof of compute decisions
- Receipts as audit evidence
- Regulatory compliance (finance, healthcare)

### 9.4 Debugging & Optimization
- Debug reuse decisions via receipts
- Optimize cost models based on historical receipts
- Learn optimal thresholds from receipt data

---

## 10. Future Directions

### 10.1 ZK-Incremental Proofs
- Zero-knowledge proofs of incremental recomputation
- Verify correctness without revealing data
- Enable privacy-preserving compute markets

### 10.2 Learned Cost Models
- ML models to predict α, C_id, C_comp from historical receipts
- Adaptive thresholds based on receipt data
- Reinforcement learning for optimal reuse decisions

### 10.3 Cross-Org Netting
- Multi-party settlement receipts
- Compute credit markets
- Federated delta compute

---

## 11. Conclusion

Receipts transform delta compute from a performance optimization into a verifiable economic transaction. By recording reuse decisions in cryptographically signed receipts, Northroot enables:

1. **Economic Transparency:** Complete audit trail of reuse decisions
2. **Trustless Markets:** Cross-organizational reuse without trust
3. **Compliance & Audit:** Verifiable proof of compute decisions
4. **Settlement:** Compute credit markets via netting receipts

**Key Differentiator:** Northroot is the only system that combines incremental compute with verifiable economic proofs, enabling trustless compute markets with auditable justification.

---

**References:**
- Receipt Schema: `schemas/receipts/`
- Delta Compute Spec: `docs/specs/delta_compute.md`
- ADR-003: `ADRs/ADR-003-delta-compute-decisions.md`

