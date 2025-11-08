# Architecture Diagrams

This document contains Mermaid diagrams illustrating the Northroot repository structure, data flows, and key use cases.

**📊 Interactive View**: For better visualization, open [architecture-diagrams.html](architecture-diagrams.html) in your browser to see all diagrams rendered with Mermaid.js.

## Table of Contents

1. [Repository Structure & Dependencies](#repository-structure--dependencies)
2. [Receipt Composition Flow](#receipt-composition-flow)
3. [Delta Compute Flow](#delta-compute-flow)
4. [Verified Spend Flow](#verified-spend-flow)
5. [Unified Proof Flow](#unified-proof-flow)

---

## Repository Structure & Dependencies

This diagram shows the crate dependency graph and architectural boundaries.

```mermaid
graph TB
    subgraph CoreLayer["Core Layer (No Internal Dependencies)"]
        Commons["northroot-commons<br/>Cross-cutting utilities"]
    end

    subgraph FoundationLayer["Foundation Layer"]
        Receipts["northroot-receipts<br/>Receipt types, canonicalization,<br/>syntactic validation"]
        Ops["northroot-ops<br/>Operator & method manifests<br/>what can be run"]
        Policy["northroot-policy<br/>Policy validation, strategies<br/>when/how to run"]
    end

    subgraph ComputationLayer["Computation Layer"]
        Engine["northroot-engine<br/>Proof algebra engine<br/>how to compute"]
    end

    subgraph ApplicationLayer["Application Layer (Future)"]
        Planner["planner<br/>DSL for intents<br/>capability matching"]
        SDK["sdk/*<br/>SDK & adapters<br/>OTel, CLI, shims"]
        Apps["apps/*<br/>App/API surfaces<br/>HTTP, TUI"]
    end

    %% Dependencies (arrows point inward)
    Receipts --> Commons
    Ops --> Commons
    Ops --> Receipts
    Policy --> Commons
    Policy --> Receipts
    Engine --> Commons
    Engine --> Receipts
    Engine --> Policy
    Planner --> Commons
    Planner --> Receipts
    Planner --> Ops
    Planner --> Policy
    SDK --> Commons
    SDK --> Receipts
    SDK --> Ops
    Apps --> SDK
    Apps --> Planner
    Apps --> Engine

    %% Forbidden dependencies (shown as red dashed)
    Receipts -.->|"FORBIDDEN"| Engine
    Policy -.->|"FORBIDDEN"| Engine
    Ops -.->|"FORBIDDEN"| Engine

    style Commons fill:#e1f5ff
    style Receipts fill:#fff4e1
    style Ops fill:#fff4e1
    style Policy fill:#fff4e1
    style Engine fill:#ffe1f5
    style Planner fill:#e1ffe1
    style SDK fill:#e1ffe1
    style Apps fill:#e1ffe1
```

**Key Boundaries:**
- **Commons**: No internal dependencies (foundation)
- **Receipts/Ops/Policy**: Depend only on commons (and receipts for ops/policy)
- **Engine**: Depends on receipts, policy (NOT the reverse)
- **Apps/SDK/Planner**: Outer layers, depend on inner layers

---

## Receipt Composition Flow

This diagram shows how receipts flow through the six kinds in a typical sequential chain.

```mermaid
graph LR
    subgraph ReceiptKinds["Receipt Kinds (Sequential Chain)"]
        DS["data_shape<br/>⊥ → S_data<br/>Schema + sketches"]
        MS["method_shape<br/>S_data → S'_data<br/>Operator contracts"]
        RS["reasoning_shape<br/>S_method → S_plan<br/>Decision DAG"]
        EX["execution<br/>S_plan → S_exec<br/>Span commitments"]
        SP["spend<br/>S_exec → S_spend<br/>Metered resources"]
        ST["settlement<br/>Σ S_spend → S_cleared<br/>Multi-party netting"]
    end

    DS -->|"cod == dom"| MS
    MS -->|"cod == dom"| RS
    RS -->|"cod == dom"| EX
    EX -->|"cod == dom"| SP
    SP -->|"cod == dom"| ST

    style DS fill:#e1f5ff
    style MS fill:#fff4e1
    style RS fill:#ffe1f5
    style EX fill:#e1ffe1
    style SP fill:#ffe1e1
    style ST fill:#f5e1ff
```

**Composition Rules:**
- Sequential: `cod(R_i) == dom(R_{i+1})` for all adjacent receipts
- Parallel: Tensor product `R₁ ⊗ R₂` for independent branches
- All receipts share the same envelope structure
- Each kind has a specific payload schema

---

## Delta Compute Flow

This diagram illustrates the delta compute reuse decision process.

```mermaid
graph TB
    subgraph InputProcessing["Input Processing"]
        Input["Input Chunks<br/>X = {x_i}"]
        Chunk["Chunking Policy χ<br/>CDC, fixed-size, row-groups"]
        ChunkIDs["Chunk IDs<br/>sha256(bytes)"]
    end

    subgraph OverlapDetection["Overlap Detection"]
        Prior["Prior Cache<br/>S' = {chunk_ids}"]
        Current["Current Input<br/>S = {chunk_ids}"]
        Jaccard["Jaccard Similarity<br/>J = |S ∩ S'| / |S ∪ S'|"]
    end

    subgraph CostModel["Cost Model"]
        CComp["Compute Cost<br/>C_comp(o, S)"]
        CId["Identity Cost<br/>C_id(o, S)"]
        Alpha["Incrementality<br/>α(o) ∈ (0,1]"]
    end

    subgraph ReuseDecision["Reuse Decision"]
        Threshold["Threshold<br/>C_id / (α · C_comp)"]
        Decision{"Reuse?<br/>J > Threshold"}
        Reuse["Reuse<br/>Use cached results"]
        Recompute["Recompute<br/>Execute operator"]
        Hybrid["Hybrid<br/>Delta + reuse"]
    end

    subgraph Recording["Recording"]
        Justification["Spend.justification<br/>overlap_j, alpha,<br/>c_id, c_comp, decision"]
    end

    Input --> Chunk
    Chunk --> ChunkIDs
    ChunkIDs --> Current
    Prior --> Jaccard
    Current --> Jaccard
    Jaccard --> Decision
    CComp --> Threshold
    CId --> Threshold
    Alpha --> Threshold
    Threshold --> Decision
    Decision -->|Yes| Reuse
    Decision -->|No| Recompute
    Decision -->|Partial| Hybrid
    Reuse --> Justification
    Recompute --> Justification
    Hybrid --> Justification

    style Input fill:#e1f5ff
    style Jaccard fill:#fff4e1
    style Decision fill:#ffe1f5
    style Justification fill:#e1ffe1
```

**Key Formula:**
```
Reuse iff: J(S, S') > C_id(o, S) / (α(o) · C_comp(o, S))
```

**Economic Delta:**
```
ΔC ≈ α(o) · C_comp(o, S) · J - C_id(o, S)
```

---

## Verified Spend Flow

This diagram shows the flow from execution through spend to settlement.

```mermaid
graph TB
    subgraph Execution["Execution"]
        Exec["Execution Receipt<br/>S_plan → S_exec<br/>span_commitments, roots"]
        Spans["Span Commitments<br/>Set/sequence roots"]
        Identity["Identity Root<br/>Merkle over identities"]
    end

    subgraph SpendCalculation["Spend Calculation"]
        Resources["Metered Resources<br/>CPU, memory, I/O, network"]
        Pricing["Pricing Model<br/>Cost per unit"]
        Justification["Justification<br/>Delta compute decisions<br/>Reuse rationale"]
        Spend["Spend Receipt<br/>S_exec → S_spend<br/>total_cost, resources"]
    end

    subgraph Settlement["Settlement"]
        MultiParty["Multi-Party Netting<br/>Σ_i S_spend_i"]
        Netting["Netting State<br/>Cleared balances"]
        SettlementReceipt["Settlement Receipt<br/>Σ S_spend → S_cleared<br/>wur_refs, balances"]
    end

    Exec --> Spans
    Exec --> Identity
    Spans --> Resources
    Identity --> Resources
    Resources --> Pricing
    Pricing --> Spend
    Justification --> Spend
    Spend --> MultiParty
    MultiParty --> Netting
    Netting --> SettlementReceipt

    style Exec fill:#e1f5ff
    style Spend fill:#ffe1e1
    style Settlement fill:#f5e1ff
```

**Key Points:**
- Execution receipts record span commitments and identity roots
- Spend receipts capture metered resources and pricing
- Settlement receipts perform multi-party netting
- All linked via `dom`/`cod` commitments

---

## Unified Proof Flow

This diagram shows the complete end-to-end flow from data shape to settlement.

```mermaid
graph TB
    subgraph DataLayer["Data Layer"]
        Data["Data Shape Receipt<br/>Schema + sketches<br/>⊥ → S_data"]
    end

    subgraph MethodLayer["Method Layer"]
        Method["Method Shape Receipt<br/>Operator contracts<br/>S_data → S'_data"]
        Operators["Operators<br/>DAG/multiset"]
    end

    subgraph PlanningLayer["Planning Layer"]
        Reasoning["Reasoning Shape Receipt<br/>Decision DAG<br/>S_method → S_plan"]
        Plan["Execution Plan<br/>Tool selection"]
    end

    subgraph ExecutionLayer["Execution Layer"]
        Execution["Execution Receipt<br/>Span commitments<br/>S_plan → S_exec"]
        Delta["Delta Compute<br/>Reuse decisions"]
        State["Merkle Row-Map<br/>Deterministic state"]
    end

    subgraph EconomicLayer["Economic Layer"]
        Spend["Spend Receipt<br/>Metered resources<br/>S_exec → S_spend"]
        Cost["Cost Calculation<br/>Pricing + justification"]
    end

    subgraph SettlementLayer["Settlement Layer"]
        SettlementReceipt["Settlement Receipt<br/>Multi-party netting<br/>Σ S_spend → S_cleared"]
        Cleared["Cleared State<br/>Final balances"]
    end

    Data --> Method
    Method --> Operators
    Operators --> Reasoning
    Reasoning --> Plan
    Plan --> Execution
    Execution --> Delta
    Execution --> State
    Delta --> Spend
    State --> Spend
    Spend --> Cost
    Cost --> SettlementReceipt
    SettlementReceipt --> Cleared

    style Data fill:#e1f5ff
    style Method fill:#fff4e1
    style Reasoning fill:#ffe1f5
    style Execution fill:#e1ffe1
    style Spend fill:#ffe1e1
    style Settlement fill:#f5e1ff
```

**Composition Rules:**
- Each receipt is a typed morphism: `f : (S, k) → (S', k')`
- Sequential composition: `cod(R_i) == dom(R_{i+1})`
- Parallel composition: Tensor product for independent branches
- All receipts share unified envelope structure
- Canonicalization ensures deterministic hashing

**Verification:**
- Hash integrity: `hash == sha256(canonical(body_without_sig_hash))`
- Signature verification: Detached signature over hash
- Composition validation: All `dom`/`cod` links must match
- Policy validation: Determinism class, tool/region constraints

---

## Boundary Enforcement

The following diagram shows how architectural boundaries are enforced.

```mermaid
graph TB
    subgraph ValidationLayers["Validation Layers"]
        Syntactic["Receipts: Syntactic Validation<br/>Format, structure, schema<br/>is this well-formed?"]
        Semantic["Policy: Semantic Validation<br/>Policy compliance, business rules<br/>is this allowed?"]
        Computation["Engine: Computation<br/>Root computation, composition<br/>how do I compute this?"]
    end

    subgraph DataFlow["Data Flow"]
        Receipt["Receipt Structure"]
        PolicyCheck["Policy Check"]
        Compute["Compute Roots"]
    end

    Receipt --> Syntactic
    Syntactic -->|"Valid structure"| PolicyCheck
    PolicyCheck --> Semantic
    Semantic -->|"Policy compliant"| Compute
    Compute --> Computation

    style Syntactic fill:#e1f5ff
    style Semantic fill:#fff4e1
    style Computation fill:#ffe1f5
```

**Key Boundaries:**
- **Receipts** (syntactic): Format checks, schema validation, structure integrity
- **Policy** (semantic): Policy compliance, determinism enforcement, constraints
- **Engine** (computation): Root computation, composition, execution tracking

**Dependency Rules:**
- Receipts → Commons only
- Policy → Commons, Receipts (NOT Engine)
- Engine → Commons, Receipts, Policy

---

## Related Documentation

- [Proof Algebra](proof_algebra.md) - Unified receipt algebra specification
- [Delta Compute](delta_compute.md) - Reuse decision formal spec
- [Merkle Row-Map](merkle_row_map.md) - Deterministic state representation
- [ADR Playbook](../ADR_PLAYBOOK.md) - Repository structure guide

