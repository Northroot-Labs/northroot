# Architecture Gaps Analysis: Path to 0.1.0 Release

**Research Date:** 2025-11-11  
**Status:** Analysis Complete  
**Version:** 0.1.0  
**Namespace:** northroot.research.release_0.1.0

## Executive Summary

This report analyzes the Northroot codebase architecture to identify gaps and requirements for a production-ready 0.1.0 release. The analysis covers SDK architecture, storage integration, job execution frameworks, end-to-end examples, and workload abstraction patterns.

**Key Findings:**
- ✅ Core engine is complete and well-architected
- ✅ Storage layer exists (SQLite-backed) but needs integration
- ⚠️ SDK exists as placeholder only (Python SDK not implemented)
- ⚠️ Examples are simulations, not real end-to-end jobs
- ❌ No job execution framework or abstraction layer
- ❌ No Rust SDK/client library for direct engine usage
- ❌ Missing workload-specific adapters (FinOps, DevOps, AI agents)

**Critical Path to 0.1.0:**
1. Create job execution framework (`northroot-runtime` or `northroot-jobs`)
2. Implement Rust SDK/client library (`northroot-sdk-rust`)
3. Complete storage integration with examples
4. Build real end-to-end compute jobs (not simulations)
5. Create workload abstraction layer for different domains

---

## 1. Current Architecture Analysis

### 1.1 Core Components (Complete)

**northroot-receipts** ✅
- Canonical data model with CBOR encoding
- Receipt validation and canonicalization
- JSON adapter layer for external compatibility
- **Status:** Production-ready, publishable

**northroot-engine** ✅
- Proof algebra implementation
- Delta compute strategies (Jaccard, reuse decisions)
- Receipt composition and validation
- Execution tracking and Merkle row-map
- **Status:** Core functionality complete

**northroot-policy** ✅
- Policy validation and cost models
- Reuse decision rules
- **Status:** Functional

**northroot-storage** ✅
- SQLite backend with WAL mode
- Receipt and manifest storage traits
- Content-addressed lookup (PAC keys)
- **Status:** Implementation complete, needs integration

### 1.2 Missing or Incomplete Components

**SDK Layer** ⚠️
- `northroot-sdk-python/` exists as placeholder only
- No PyO3 bindings or CFFI implementation
- No Rust SDK/client library
- **Impact:** Users cannot easily use the engine

**Job Execution Framework** ❌
- No `ComputeJob` trait implementation
- No `JobContext` or job orchestration
- Examples are simulations, not real jobs
- **Impact:** Cannot run production workloads

**Storage Integration** ⚠️
- Storage exists but examples don't use it
- Examples reconstruct state from hardcoded data
- No manifest storage in examples
- **Impact:** Cannot persist proofs or verify reuse

**Workload Adapters** ❌
- No FinOps-specific adapters
- No DevOps-specific adapters
- No AI agent-specific adapters
- **Impact:** Cannot abstract engine complexity for domain users

---

## 2. SDK Architecture Recommendations

### 2.1 Separation Strategy

**Option A: Separate Repository (Recommended for 0.1.0+)**
```
northroot/                    # Core engine (this repo)
├── crates/
│   ├── northroot-receipts/
│   ├── northroot-engine/
│   └── ...
└── ...

northroot-sdk-rust/           # Rust SDK (separate repo)
├── src/
│   ├── client.rs            # High-level client API
│   ├── jobs.rs              # Job execution framework
│   └── adapters/            # Workload-specific adapters
└── ...

northroot-sdk-python/         # Python SDK (separate repo)
├── src/
│   └── lib.rs               # PyO3 bindings
└── ...
```

**Option B: Monorepo with Workspace (Recommended for 0.1.0)**
```
northroot/
├── crates/
│   ├── northroot-receipts/
│   ├── northroot-engine/
│   └── ...
├── sdk/
│   ├── northroot-sdk-rust/  # Rust SDK crate
│   └── northroot-sdk-python/ # Python SDK crate
└── ...
```

**Recommendation:** Start with Option B (monorepo) for 0.1.0, migrate to Option A (separate repos) for 0.2.0+ when SDKs mature.

### 2.2 Rust SDK Design

**Core API Surface:**
```rust
// crates/northroot-sdk-rust/src/lib.rs

/// High-level client for Northroot engine
pub struct NorthrootClient {
    storage: Arc<dyn ReceiptStore>,
    policy_loader: PolicyLoader,
}

impl NorthrootClient {
    /// Create a new client with SQLite storage
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, ClientError>;
    
    /// Execute a compute job
    pub async fn execute_job<J: ComputeJob>(
        &self,
        job: J,
        ctx: JobContext,
    ) -> Result<JobResult, JobError>;
    
    /// Query receipts
    pub fn query_receipts(&self, query: ReceiptQuery) -> Result<Vec<Receipt>, ClientError>;
    
    /// Verify reuse decision
    pub fn verify_reuse(
        &self,
        receipt: &Receipt,
        mode: VerificationMode,
    ) -> Result<VerificationResult, ClientError>;
}

/// Job execution context
pub struct JobContext {
    pub policy_ref: String,
    pub trace_id: String,
    pub change_epoch: String,
    pub storage: Arc<dyn ReceiptStore>,
}

/// Result of job execution
pub struct JobResult {
    pub receipt: Receipt,
    pub manifest: Option<ChunkManifest>,
    pub metrics: JobMetrics,
}
```

**Workload Adapters:**
```rust
// crates/northroot-sdk-rust/src/adapters/finops.rs

/// FinOps cost attribution adapter
pub struct FinOpsAdapter {
    client: NorthrootClient,
}

impl FinOpsAdapter {
    /// Process billing data with automatic reuse detection
    pub async fn process_billing(
        &self,
        billing_files: Vec<PathBuf>,
        policy_ref: &str,
    ) -> Result<CostAttributionResult, FinOpsError>;
}
```

### 2.3 Python SDK Design

**PyO3 Bindings:**
```python
# Python API surface

from northroot_sdk import NorthrootClient, DeltaContext

# High-level client
client = NorthrootClient(db_path="./northroot.db")
result = await client.process_finops_billing(
    billing_files=["billing.csv"],
    policy_ref="pol:finops/cost-attribution@1"
)

# Context manager pattern
with DeltaContext(
    operator="partition_rows",
    policy_ref="pol:etl/partition-reuse@1"
) as ctx:
    result = process_data(df)
    receipt = ctx.emit_receipt(result)
```

**Implementation Requirements:**
- PyO3 bindings for core engine functions
- Async support (Tokio runtime)
- Error handling and type conversion
- Integration with Pandas, Spark, Dask

---

## 3. Storage Integration Requirements

### 3.1 Current State

**What Exists:**
- ✅ `northroot-storage` crate with SQLite backend
- ✅ `ReceiptStore` trait with full interface
- ✅ Manifest storage with compression (zstd)
- ✅ PAC key support for content-addressed lookup

**What's Missing:**
- ❌ Examples don't use storage (reconstruct state from hardcoded data)
- ❌ No manifest generation in examples
- ❌ No PAC key computation in examples
- ❌ No storage integration tests

### 3.2 Integration Requirements

**1. Update Examples to Use Storage:**
```rust
// examples/finops_cost_attribution/main.rs

use northroot_storage::SqliteStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize storage
    let store = SqliteStore::new("./northroot.db")?;
    
    // Load previous receipt from storage
    let prev_receipt = store.get_previous_execution(
        &pac_key,
        &trace_id,
    )?;
    
    // Process and store new receipt
    let receipt = simulate_billing_run(resource_tuples, prev_receipt.as_ref())?;
    store.store_receipt(&receipt)?;
    
    Ok(())
}
```

**2. Manifest Generation:**
```rust
// Generate chunk manifest from resource tuples
fn generate_manifest(
    resource_tuples: &[ResourceTuple],
    pac: &[u8; 32],
) -> Result<ChunkManifest, ManifestError> {
    let entries: Vec<ChunkEntry> = resource_tuples
        .iter()
        .map(|tuple| ChunkEntry {
            id: chunk_id_from_tuple(tuple),
            hash: compute_tuple_hash(tuple),
            // ... other fields
        })
        .collect();
    
    let manifest = ChunkManifest {
        schema: "northroot/chunk_manifest@1".to_string(),
        pac: *pac,
        entries,
        stats: ManifestStats { /* ... */ },
    };
    
    Ok(manifest)
}
```

**3. PAC Key Computation:**
```rust
// Compute PAC key for execution
fn compute_pac_key(
    data_shape_hash: &str,
    method_shape_hash: &str,
    change_epoch: &str,
    policy_ref: &str,
) -> [u8; 32] {
    let combined = format!(
        "northroot|0.1.0||{}||{}||{}||strict||{}||1.0",
        data_shape_hash, method_shape_hash, change_epoch, policy_ref
    );
    sha2::Sha256::digest(combined.as_bytes()).into()
}
```

---

## 4. Job Execution Framework

### 4.1 Design Requirements

**Core Trait:**
```rust
// crates/northroot-sdk-rust/src/jobs.rs

/// Trait for compute jobs that can be executed with delta compute
pub trait ComputeJob: Send + Sync {
    /// Execute the job and emit receipts
    fn execute(&self, ctx: &JobContext) -> Result<JobResult, JobError>;
    
    /// Get job metadata
    fn metadata(&self) -> JobMetadata;
    
    /// Extract chunk set from input data
    fn extract_chunks(&self, input: &JobInput) -> Result<ChunkSet, JobError>;
    
    /// Process data with reuse decision
    fn process(
        &self,
        input: &JobInput,
        prev_state: Option<&MerkleRowMap>,
        mode: ExecutionMode,
    ) -> Result<JobOutput, JobError>;
}

/// Job execution context
pub struct JobContext {
    pub policy_ref: String,
    pub trace_id: String,
    pub change_epoch: String,
    pub storage: Arc<dyn ReceiptStore>,
    pub client: Arc<NorthrootClient>,
}

/// Job result
pub struct JobResult {
    pub execution_receipt: Receipt,
    pub spend_receipt: Receipt,
    pub manifest: Option<ChunkManifest>,
    pub output: JobOutput,
    pub metrics: JobMetrics,
}
```

**Job Orchestrator:**
```rust
// crates/northroot-sdk-rust/src/jobs/orchestrator.rs

pub struct JobOrchestrator {
    client: Arc<NorthrootClient>,
}

impl JobOrchestrator {
    /// Execute a job with automatic reuse detection
    pub async fn execute<J: ComputeJob>(
        &self,
        job: J,
        ctx: JobContext,
    ) -> Result<JobResult, JobError> {
        // 1. Load previous execution
        let prev_receipt = ctx.storage.get_previous_execution(
            &compute_pac_key(...),
            &ctx.trace_id,
        )?;
        
        // 2. Extract current chunk set
        let current_chunks = job.extract_chunks(&input)?;
        
        // 3. Compute overlap
        let overlap_j = if let Some(prev) = &prev_receipt {
            let prev_chunks = load_chunk_set_from_manifest(&prev)?;
            jaccard_similarity(&current_chunks, &prev_chunks)
        } else {
            0.0
        };
        
        // 4. Make reuse decision
        let cost_model = load_cost_model_from_policy(&ctx.policy_ref)?;
        let (decision, justification) = decide_reuse(overlap_j, &cost_model, None);
        
        // 5. Execute job
        let mode = if decision == ReuseDecision::Reuse {
            ExecutionMode::Delta
        } else {
            ExecutionMode::Full
        };
        
        let output = job.process(&input, prev_state.as_ref(), mode)?;
        
        // 6. Generate receipts
        let exec_receipt = generate_execution_receipt(...)?;
        let spend_receipt = generate_spend_receipt(exec_receipt.rid, justification)?;
        
        // 7. Store receipts and manifest
        ctx.storage.store_receipt(&exec_receipt)?;
        ctx.storage.store_receipt(&spend_receipt)?;
        
        if let Some(manifest) = &manifest {
            ctx.storage.put_manifest(&manifest_hash, &manifest_bytes, &meta)?;
        }
        
        Ok(JobResult { /* ... */ })
    }
}
```

### 4.2 Built-in Job Types

**1. FinOps Cost Attribution Job:**
```rust
// crates/northroot-sdk-rust/src/jobs/finops.rs

pub struct FinOpsCostAttributionJob {
    billing_files: Vec<PathBuf>,
    output_path: PathBuf,
}

impl ComputeJob for FinOpsCostAttributionJob {
    fn execute(&self, ctx: &JobContext) -> Result<JobResult, JobError> {
        // Load billing data
        let billing_data = load_billing_files(&self.billing_files)?;
        
        // Extract resource tuples
        let resource_tuples = extract_resource_tuples(&billing_data)?;
        let chunks = tuples_to_chunk_ids(&resource_tuples);
        
        // ... rest of execution
    }
}
```

**2. ETL Partition Reuse Job:**
```rust
// crates/northroot-sdk-rust/src/jobs/etl.rs

pub struct ETLPartitionReuseJob {
    delta_table_path: PathBuf,
    transformations: Vec<Transformation>,
    output_path: PathBuf,
}

impl ComputeJob for ETLPartitionReuseJob {
    fn execute(&self, ctx: &JobContext) -> Result<JobResult, JobError> {
        // Scan Delta Lake CDF
        let changed_partitions = scan_cdf(&self.delta_table_path)?;
        
        // ... rest of execution
    }
}
```

---

## 5. End-to-End Example Jobs

### 5.1 Requirements

**Current State:**
- Examples are simulations (hardcoded data)
- No real data processing
- No storage integration
- No manifest generation

**Target State:**
- Real data processing (CSV, Parquet, Delta Lake)
- Storage integration (load/store receipts)
- Manifest generation and storage
- Verifiable proof of reuse

### 5.2 Implementation Plan

**1. FinOps Cost Attribution (Priority: P0)**
```rust
// examples/finops_cost_attribution/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client and storage
    let client = NorthrootClient::new("./northroot.db")?;
    
    // Create job
    let job = FinOpsCostAttributionJob::new(
        billing_files: vec!["billing_2025-01-27.csv"],
        output_path: "./cost_attribution.parquet",
    );
    
    // Execute job
    let ctx = JobContext {
        policy_ref: "pol:finops/cost-attribution@1".to_string(),
        trace_id: generate_trace_id(),
        change_epoch: "snap-2025-01-27".to_string(),
        storage: client.storage(),
        client: client.clone(),
    };
    
    let result = client.execute_job(job, ctx).await?;
    
    println!("Job completed:");
    println!("  Execution RID: {}", result.execution_receipt.rid);
    println!("  Spend RID: {}", result.spend_receipt.rid);
    println!("  Reuse rate: {:.2}%", result.metrics.reuse_rate);
    println!("  Economic delta: ${:.2}", result.metrics.economic_delta);
    
    Ok(())
}
```

**2. ETL Partition Reuse (Priority: P1)**
- Similar structure but with Delta Lake integration
- Scan CDF for changed partitions
- Process only changed partitions when reusing

**3. Analytics Query (Priority: P2)**
- SQL query execution
- Result caching and incremental refresh
- Query result overlap computation

---

## 6. Workload Abstraction Layer

### 6.1 Design Goals

**Abstract away engine complexity:**
- Users shouldn't need to understand receipts, PAC keys, or manifests
- Domain-specific APIs (FinOps, DevOps, AI agents)
- Automatic receipt generation and storage
- Policy-driven reuse decisions

### 6.2 Domain Adapters

**1. FinOps Adapter:**
```rust
// crates/northroot-sdk-rust/src/adapters/finops.rs

pub struct FinOpsClient {
    client: NorthrootClient,
}

impl FinOpsClient {
    /// Process billing data with automatic reuse
    pub async fn process_billing(
        &self,
        billing_files: Vec<PathBuf>,
        output_path: PathBuf,
    ) -> Result<CostAttributionResult, FinOpsError> {
        let job = FinOpsCostAttributionJob::new(billing_files, output_path);
        let ctx = self.create_context("pol:finops/cost-attribution@1")?;
        let result = self.client.execute_job(job, ctx).await?;
        Ok(CostAttributionResult::from(result))
    }
    
    /// Query cost attribution by account
    pub fn query_by_account(
        &self,
        account_id: &str,
        date_range: DateRange,
    ) -> Result<Vec<CostRecord>, FinOpsError> {
        // Query receipts and reconstruct cost data
    }
}
```

**2. DevOps Adapter:**
```rust
// crates/northroot-sdk-rust/src/adapters/devops.rs

pub struct DevOpsClient {
    client: NorthrootClient,
}

impl DevOpsClient {
    /// Process infrastructure changes with reuse
    pub async fn process_infrastructure(
        &self,
        terraform_state: PathBuf,
        output_path: PathBuf,
    ) -> Result<InfrastructureResult, DevOpsError> {
        // Similar pattern to FinOps
    }
}
```

**3. AI Agent Adapter (Future):**
```rust
// crates/northroot-sdk-rust/src/adapters/ai_agent.rs

pub struct AIAgentClient {
    client: NorthrootClient,
}

impl AIAgentClient {
    /// Process agent execution with proof
    pub async fn execute_agent(
        &self,
        agent_config: AgentConfig,
        task: Task,
    ) -> Result<AgentResult, AIAgentError> {
        // Track agent tool calls, reasoning, execution
        // Emit receipts for each step
    }
}
```

---

## 7. 0.1.0 Release Checklist

### 7.1 Core Requirements

- [ ] **Storage Integration**
  - [ ] Update examples to use `SqliteStore`
  - [ ] Implement manifest generation in examples
  - [ ] Add PAC key computation
  - [ ] Storage integration tests

- [ ] **Job Execution Framework**
  - [ ] Create `northroot-sdk-rust` crate
  - [ ] Implement `ComputeJob` trait
  - [ ] Implement `JobOrchestrator`
  - [ ] Built-in job types (FinOps, ETL)

- [ ] **Rust SDK**
  - [ ] High-level `NorthrootClient` API
  - [ ] Job execution methods
  - [ ] Receipt querying
  - [ ] Verification methods

- [ ] **End-to-End Examples**
  - [ ] Real FinOps job (process CSV billing data)
  - [ ] Real ETL job (process Delta Lake table)
  - [ ] Storage integration
  - [ ] Manifest generation

- [ ] **Workload Adapters**
  - [ ] FinOps adapter (high-level API)
  - [ ] Documentation and examples

### 7.2 Nice-to-Have (Post-0.1.0)

- [ ] Python SDK (PyO3 bindings)
- [ ] DevOps adapter
- [ ] AI agent adapter
- [ ] Remote storage backend (S3, Postgres)
- [ ] Web UI for receipt browsing
- [ ] CLI tool for job execution

---

## 8. Recommended Architecture for 0.1.0

### 8.1 Crate Structure

```
northroot/
├── crates/
│   ├── northroot-receipts/      # ✅ Complete
│   ├── northroot-engine/        # ✅ Complete
│   ├── northroot-policy/        # ✅ Complete
│   ├── northroot-storage/       # ✅ Complete (needs integration)
│   ├── northroot-commons/       # ✅ Complete
│   ├── northroot-ops/          # ✅ Complete
│   └── northroot-sdk-rust/     # ❌ NEW - Job execution + client API
├── sdk/
│   └── northroot-sdk-python/    # ⚠️ Placeholder (post-0.1.0)
├── examples/
│   ├── finops_cost_attribution/ # ⚠️ Update to use storage + real data
│   ├── etl_partition_reuse/     # ⚠️ Update to use storage + real data
│   └── analytics_dashboard/     # ⚠️ Update to use storage + real data
└── ...
```

### 8.2 New Crate: `northroot-sdk-rust`

**Purpose:** High-level client API and job execution framework

**Dependencies:**
- `northroot-engine` (delta compute, receipts)
- `northroot-storage` (receipt storage)
- `northroot-policy` (policy loading)
- `northroot-receipts` (receipt types)

**Modules:**
```rust
// crates/northroot-sdk-rust/src/lib.rs

pub mod client;        // NorthrootClient
pub mod jobs;          // ComputeJob trait, JobOrchestrator
pub mod adapters;      // Domain-specific adapters
pub mod error;         // Error types

pub use client::NorthrootClient;
pub use jobs::{ComputeJob, JobContext, JobResult};
```

---

## 9. Migration Path for Examples

### 9.1 Current Example Structure

```rust
// examples/finops_cost_attribution/main.rs (current)
fn simulate_billing_run(
    resource_tuples: Vec<ResourceTuple>,
    prev_receipt: Option<&Receipt>,
) -> (Receipt, String) {
    // Hardcoded previous state
    // No storage
    // No manifest
}
```

### 9.2 Target Structure

```rust
// examples/finops_cost_attribution/main.rs (target)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = NorthrootClient::new("./northroot.db")?;
    
    let job = FinOpsCostAttributionJob {
        billing_files: vec!["billing.csv"],
        output_path: "./output.parquet",
    };
    
    let ctx = JobContext {
        policy_ref: "pol:finops/cost-attribution@1".to_string(),
        trace_id: generate_trace_id(),
        change_epoch: "snap-2025-01-27".to_string(),
        storage: client.storage(),
        client: client.clone(),
    };
    
    let result = client.execute_job(job, ctx).await?;
    // ...
}
```

---

## 10. Implementation Priority

### Phase 1: Foundation (Week 1-2)
1. Create `northroot-sdk-rust` crate
2. Implement `NorthrootClient` with storage integration
3. Implement `ComputeJob` trait and `JobOrchestrator`
4. Update one example to use new framework

### Phase 2: Job Types (Week 2-3)
1. Implement `FinOpsCostAttributionJob`
2. Implement `ETLPartitionReuseJob`
3. Add manifest generation
4. Add PAC key computation

### Phase 3: Adapters (Week 3-4)
1. Implement `FinOpsClient` adapter
2. High-level API for FinOps use cases
3. Documentation and examples

### Phase 4: Polish (Week 4)
1. Error handling improvements
2. Integration tests
3. Documentation
4. Release preparation

---

## 11. Success Criteria for 0.1.0

**Must Have:**
- ✅ Users can run real compute jobs (not simulations)
- ✅ Receipts are stored and retrievable
- ✅ Reuse decisions are verifiable
- ✅ At least one end-to-end example works with real data
- ✅ Rust SDK provides usable API

**Should Have:**
- ✅ FinOps adapter with high-level API
- ✅ Multiple job types (FinOps, ETL)
- ✅ Manifest generation and storage
- ✅ Documentation and examples

**Nice to Have:**
- ⚠️ Python SDK (can defer to 0.2.0)
- ⚠️ DevOps adapter (can defer to 0.2.0)
- ⚠️ AI agent adapter (can defer to 0.3.0)

---

## 12. Recommendations

### 12.1 Immediate Actions

1. **Create `northroot-sdk-rust` crate**
   - High-level client API
   - Job execution framework
   - Workload adapters

2. **Integrate storage into examples**
   - Replace hardcoded state with storage lookups
   - Add manifest generation
   - Add PAC key computation

3. **Build real end-to-end jobs**
   - FinOps job with real CSV processing
   - ETL job with Delta Lake integration
   - Storage and verification

### 12.2 Architecture Decisions

1. **SDK Location:** Start with monorepo (`crates/northroot-sdk-rust`), migrate to separate repo later
2. **Job Framework:** Implement `ComputeJob` trait with `JobOrchestrator`
3. **Storage:** SQLite-first, forward-compatible with remote backends
4. **Adapters:** Domain-specific adapters to abstract engine complexity

### 12.3 Future Considerations

1. **Python SDK:** Implement PyO3 bindings post-0.1.0
2. **Remote Storage:** Add S3/Postgres backends post-0.1.0
3. **AI Agent Adapter:** Design and implement for 0.3.0+
4. **Web UI:** Consider for 0.2.0+ for receipt browsing

---

## 13. Conclusion

The Northroot engine is architecturally sound and functionally complete. The primary gaps for 0.1.0 are:

1. **SDK Layer:** No high-level client API for users
2. **Job Framework:** No execution framework for running workloads
3. **Storage Integration:** Examples don't use storage
4. **Real Jobs:** Examples are simulations, not real workloads
5. **Workload Abstraction:** No domain-specific adapters

**Recommended Path:**
1. Create `northroot-sdk-rust` with job execution framework
2. Integrate storage into examples
3. Build real end-to-end jobs (FinOps, ETL)
4. Create FinOps adapter for high-level API
5. Document and release 0.1.0

This path provides a usable, production-ready tool that abstracts away engine complexity while maintaining the power of verifiable compute and proof generation.

---

## References

- [ADR-008: Production Hardening](ADRs/ADR-008-production-hardening-storage.md)
- [Production Compute Jobs](../../docs/planning/production-compute-jobs.md)
- [Delta Compute Spec](docs/specs/delta_compute.md)
- [Architecture Diagrams](docs/specs/architecture-diagrams.md)

