# Integration Notes: SDK/API Integration Strategies

**Research Agent:** cursor  
**Date:** 2025-01-27  
**Version:** 0.1  
**Namespace:** northroot.research.delta_compute

## Overview

This document outlines integration strategies for embedding Northroot's delta compute and verifiable receipt system into existing data processing frameworks. Focus areas: SDK hooks, MCP embedding, and trace observation.

---

## 1. SDK Integration Patterns

### 1.1 Python SDK (Primary Target)

**Target Frameworks:** Spark, Pandas, Dask, Ray, Dagster

**Integration Points:**

#### Pattern A: Decorator-Based Instrumentation

```python
from northroot_sdk import delta_compute, emit_receipt

@delta_compute(
    operator="partition_rows",
    alpha=0.95,
    cost_model={"c_id": 0.1, "c_comp": 10.0}
)
def process_partition(data: pd.DataFrame) -> pd.DataFrame:
    # Existing processing logic
    return transformed_data

# Receipt automatically emitted with spend.justification
```

**Benefits:**
- Minimal code changes
- Automatic receipt emission
- Policy-driven reuse decisions

**Implementation:**
- SDK intercepts function execution
- Computes overlap J with previous runs
- Applies reuse decision rule
- Emits receipt with justification

#### Pattern B: Context Manager

```python
from northroot_sdk import DeltaContext

with DeltaContext(
    operator="group_by_account",
    policy_ref="acme/reuse_thresholds@1"
) as ctx:
    result = df.groupby("account_id").sum()
    ctx.emit_receipt(result)
```

**Benefits:**
- Explicit control over receipt emission
- Context-aware state management
- Policy reference per operation

#### Pattern C: Operator Wrapper

```python
from northroot_sdk import DeltaOperator

partition_op = DeltaOperator(
    name="partition_rows",
    alpha=0.95,
    strategy="partition"
)

result, receipt = partition_op.execute(
    input_data=df,
    prev_state=prev_state_hash
)
```

**Benefits:**
- Functional API matching Northroot engine patterns
- Explicit state management
- Receipt returned for further processing

### 1.2 Rust SDK (Native Integration)

**Target:** High-performance pipelines, embedded systems

**API Design:**

```rust
use northroot_sdk::delta::{DeltaOperator, CostModel, ReuseDecision};

let cost_model = CostModel::new(
    c_id: 0.1,
    c_comp: 10.0,
    alpha: 0.95
);

let operator = DeltaOperator::new(
    name: "partition_rows",
    cost_model,
    strategy: Strategy::Partition
);

let (result, receipt) = operator.execute(
    input: &input_data,
    prev_state: prev_state_hash
)?;
```

**Benefits:**
- Zero-cost abstractions
- Direct integration with Northroot engine
- Type-safe receipt generation

### 1.3 Java/Scala SDK (JVM Integration)

**Target:** Spark, Flink, Kafka Streams

**Integration Pattern:**

```scala
import io.northroot.sdk.delta._

val operator = DeltaOperator(
  name = "group_by_account",
  alpha = 0.90,
  costModel = CostModel(cId = 0.5, cComp = 10.0)
)

val (result, receipt) = operator.execute(
  input = df,
  prevState = prevStateHash
)
```

**Spark UDF Integration:**

```scala
import io.northroot.sdk.spark._

val deltaUDF = DeltaUDF(
  operator = "incremental_sum",
  alpha = 0.92
)

df.withColumn("sum", deltaUDF(col("value")))
```

---

## 2. Framework-Specific Integration

### 2.1 Apache Spark

**Integration Strategy:** Custom Spark UDFs + Receipt Emission

**Implementation:**

```python
from pyspark.sql import SparkSession
from northroot_sdk.spark import DeltaUDF, emit_receipt

spark = SparkSession.builder.appName("DeltaCompute").getOrCreate()

# Register delta-aware UDF
@DeltaUDF(operator="incremental_sum", alpha=0.92)
def sum_values(values):
    return sum(values)

# Use in Spark pipeline
df = spark.read.parquet("input.parquet")
result = df.groupBy("account_id").agg(
    sum_values("amount").alias("total")
)

# Emit receipt for entire Spark job
receipt = emit_receipt(
    execution=result,
    method_ref="acme/cost_attribution@1.0.0",
    policy_ref="acme/reuse_thresholds@1"
)
```

**Receipt Fields:**
- `execution.payload.trace_id`: Spark job ID
- `execution.payload.span_commitments`: Per-stage commitments
- `spend.justification.overlap_j`: Measured overlap
- `spend.justification.decision`: Reuse decision

### 2.2 Dagster

**Integration Strategy:** Asset Materialization Hooks

**Implementation:**

```python
from dagster import asset, MaterializeResult
from northroot_sdk.dagster import DeltaAsset, emit_receipt

@DeltaAsset(
    operator="partition_by_date",
    alpha=0.95,
    policy_ref="acme/reuse_thresholds@1"
)
@asset
def daily_cost_attribution(context, raw_billing_data):
    # Existing asset logic
    result = process_billing(raw_billing_data)
    
    # Receipt automatically emitted via DeltaAsset decorator
    return MaterializeResult(
        metadata={"receipt_rid": context.receipt_rid}
    )
```

**Benefits:**
- Native Dagster asset integration
- Automatic receipt emission on materialization
- Policy-driven reuse decisions

### 2.3 Ray

**Integration Strategy:** Task Decorators + Object Store Hooks

**Implementation:**

```python
from ray import remote
from northroot_sdk.ray import DeltaTask, emit_receipt

@DeltaTask(operator="group_by_account", alpha=0.90)
@remote
def process_account(account_data):
    # Existing task logic
    return aggregated_result

# Ray automatically handles receipt emission
results = ray.get([process_account.remote(data) for data in accounts])
```

**Benefits:**
- Transparent integration with Ray's object store
- Automatic state management
- Receipt emission per task

### 2.4 Delta Lake

**Integration Strategy:** Transaction Log Hooks

**Implementation:**

```python
from delta import DeltaTable
from northroot_sdk.delta_lake import DeltaLakeIntegration

# Initialize integration
delta_integration = DeltaLakeIntegration(
    policy_ref="acme/reuse_thresholds@1"
)

# Read with delta compute
df = delta_integration.read_delta(
    table_path="s3://bucket/table",
    operator="partition_by_date",
    alpha=0.95
)

# Write with receipt emission
delta_integration.write_delta(
    df=df,
    table_path="s3://bucket/table",
    receipt_metadata={"method_ref": "acme/etl_pipeline@1.0.0"}
)
```

**Benefits:**
- Leverages Delta Lake's transaction log for overlap detection
- Automatic partition-level reuse
- Receipt emission per transaction

---

## 3. MCP (Model Context Protocol) Integration

### 3.1 MCP Server for Receipt Queries

**Implementation:**

```typescript
// mcp-server-northroot.ts
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { ReceiptStore } from "northroot-mcp";

const server = new Server({
  name: "northroot-receipts",
  version: "0.1.0"
});

// Tool: query_receipts
server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [{
    name: "query_receipts",
    description: "Query Northroot receipts by method, domain, or time range",
    inputSchema: {
      type: "object",
      properties: {
        method_ref: { type: "string" },
        domain: { type: "string", enum: ["FinOps", "ETL", "ML"] },
        time_range: { type: "object" }
      }
    }
  }]
}));

// Tool: get_reuse_justification
server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [{
    name: "get_reuse_justification",
    description: "Get reuse justification for a receipt",
    inputSchema: {
      type: "object",
      properties: {
        receipt_rid: { type: "string" }
      }
    }
  }]
}));
```

**Use Cases:**
- Finance teams query cost attribution receipts
- Data engineers audit ETL pipeline receipts
- ML teams verify feature computation receipts

### 3.2 MCP Client Integration

**Example: Finance Tool Integration**

```python
from mcp import Client
from northroot_mcp import ReceiptQuery

client = Client("northroot-receipts")

# Query cost attribution receipts
receipts = client.call_tool(
    "query_receipts",
    {
        "method_ref": "acme/cost_attribution@1.0.0",
        "domain": "FinOps",
        "time_range": {"start": "2025-01-01", "end": "2025-01-31"}
    }
)

# Analyze reuse economics
for receipt in receipts:
    justification = client.call_tool(
        "get_reuse_justification",
        {"receipt_rid": receipt.rid}
    )
    print(f"Overlap: {justification.overlap_j}, Savings: {justification.delta_c}")
```

---

## 4. Trace Observation Integration

### 4.1 OpenTelemetry Integration

**Strategy:** Emit receipts as OpenTelemetry spans

**Implementation:**

```python
from opentelemetry import trace
from northroot_sdk.otel import ReceiptSpanExporter

tracer = trace.get_tracer("northroot-delta-compute")

with tracer.start_as_current_span("cost_attribution") as span:
    result = process_cost_attribution(data)
    
    # Emit receipt as OTel span
    receipt_span = ReceiptSpanExporter.export(
        span=span,
        receipt=emit_receipt(result),
        attributes={
            "northroot.overlap_j": justification.overlap_j,
            "northroot.alpha": justification.alpha,
            "northroot.decision": justification.decision
        }
    )
```

**Benefits:**
- Integrates with existing observability stacks
- Receipts visible in tracing UIs (Jaeger, Zipkin)
- Correlation with application traces

### 4.2 Distributed Tracing

**Strategy:** Receipts as trace events

**Implementation:**

```python
from northroot_sdk.trace import TraceObserver

observer = TraceObserver(
    trace_backend="jaeger",
    receipt_store="s3://receipts"
)

with observer.trace("etl_pipeline"):
    # Pipeline execution
    result = execute_pipeline(data)
    
    # Receipt automatically linked to trace
    receipt = observer.emit_receipt(result)
```

**Benefits:**
- End-to-end traceability
- Receipts linked to application traces
- Debugging reuse decisions in context

---

## 5. State Management

### 5.1 State Storage

**Options:**
1. **Local Filesystem:** For single-node deployments
2. **S3/GCS:** For distributed systems
3. **Database:** For queryable state (PostgreSQL, DynamoDB)
4. **Distributed Cache:** For high-performance (Redis, Memcached)

**Implementation:**

```python
from northroot_sdk.state import StateStore

# S3-backed state store
state_store = StateStore(
    backend="s3",
    bucket="northroot-state",
    prefix="delta-compute/"
)

# Load previous state
prev_state = state_store.load(
    operator="partition_rows",
    method_ref="acme/etl_pipeline@1.0.0"
)

# Save new state
state_store.save(
    operator="partition_rows",
    method_ref="acme/etl_pipeline@1.0.0",
    state=current_state,
    receipt_rid=receipt.rid
)
```

### 5.2 State Versioning

**Strategy:** State versions linked to receipts

**Implementation:**

```python
# State version includes receipt RID
state_version = {
    "operator": "partition_rows",
    "method_ref": "acme/etl_pipeline@1.0.0",
    "receipt_rid": receipt.rid,
    "state_hash": state.state_hash(),
    "timestamp": receipt.ctx.timestamp
}

# Query state by receipt
prev_state = state_store.load_by_receipt(
    receipt_rid=prev_receipt.rid
)
```

---

## 6. Policy Integration

### 6.1 Policy Registry

**Implementation:**

```python
from northroot_sdk.policy import PolicyRegistry

registry = PolicyRegistry(
    backend="s3",
    bucket="northroot-policies"
)

# Load policy
policy = registry.load("acme/reuse_thresholds@1")

# Apply policy to operator
operator = DeltaOperator(
    name="partition_rows",
    policy=policy
)
```

### 6.2 Policy-Driven Decisions

**Policy Schema:**

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
    "c_id": { "type": "constant", "value": 1.0 },
    "c_comp": { "type": "linear", "per_row": 0.00001, "base": 0.5 },
    "alpha": { "type": "constant", "value": 0.9 }
  },
  "decision": {
    "rule": "j > c_id/(alpha*c_comp)",
    "fallback": "recompute"
  }
}
```

---

## 7. Receipt Emission Patterns

### 7.1 Automatic Emission

**Pattern:** SDK automatically emits receipts

```python
@delta_compute(operator="partition_rows")
def process_data(data):
    return transformed_data
# Receipt automatically emitted
```

### 7.2 Explicit Emission

**Pattern:** Developer explicitly emits receipts

```python
def process_data(data):
    result = transform(data)
    receipt = emit_receipt(
        execution=result,
        method_ref="acme/pipeline@1.0.0",
        policy_ref="acme/reuse_thresholds@1"
    )
    return result, receipt
```

### 7.3 Batch Emission

**Pattern:** Emit receipts for entire pipeline

```python
with DeltaPipeline(
    method_ref="acme/etl_pipeline@1.0.0"
) as pipeline:
    stage1 = pipeline.stage("partition", data)
    stage2 = pipeline.stage("aggregate", stage1)
    result = pipeline.stage("write", stage2)

# Single receipt emitted for entire pipeline
receipt = pipeline.emit_receipt()
```

---

## 8. Error Handling & Rollback

### 8.1 Receipt Validation

**Implementation:**

```python
from northroot_sdk.validation import validate_receipt

try:
    receipt = emit_receipt(execution=result)
    validate_receipt(receipt)  # Raises on validation failure
except ReceiptValidationError as e:
    # Rollback state, log error
    state_store.rollback(prev_state)
    logger.error(f"Receipt validation failed: {e}")
```

### 8.2 State Rollback

**Implementation:**

```python
# Checkpoint state before execution
checkpoint = state_store.checkpoint(
    operator="partition_rows",
    method_ref="acme/etl_pipeline@1.0.0"
)

try:
    result, receipt = operator.execute(data, prev_state)
    state_store.save(result.state)
except Exception as e:
    # Rollback to checkpoint
    state_store.rollback(checkpoint)
    raise
```

---

## 9. Performance Considerations

### 9.1 Overlap Detection Optimization

**Strategy:** Use sketches for fast estimation, exact sets for verification

```python
# Fast estimation with MinHash
j_est = minhash_estimate(current_sketch, prev_sketch)
if j_est < lower_bound:
    return RecomputeDecision()

# Exact verification
j_exact = jaccard_exact(current_chunks, prev_chunks)
return decide_reuse(j_exact, cost_model)
```

### 9.2 Receipt Generation Overhead

**Mitigation:**
- Async receipt emission
- Batch receipt generation
- Receipt compression (CBOR vs JSON)

```python
# Async receipt emission
receipt_future = emit_receipt_async(execution=result)

# Continue processing
result = next_stage(result)

# Await receipt when needed
receipt = receipt_future.await()
```

---

## 10. Testing & Validation

### 10.1 Unit Tests

```python
def test_delta_operator_reuse():
    operator = DeltaOperator(
        name="partition_rows",
        alpha=0.95,
        cost_model=CostModel(c_id=0.1, c_comp=10.0)
    )
    
    # First run: full compute
    result1, receipt1 = operator.execute(data1, None)
    assert receipt1.justification.decision == "recompute"
    
    # Second run: reuse (high overlap)
    result2, receipt2 = operator.execute(data2, result1.state)
    assert receipt2.justification.decision == "reuse"
    assert receipt2.justification.overlap_j > 0.8
```

### 10.2 Integration Tests

```python
def test_spark_integration():
    df = spark.read.parquet("input.parquet")
    result = df.groupBy("account_id").agg(
        delta_sum("amount").alias("total")
    )
    
    receipt = emit_receipt(result)
    assert receipt.kind == "execution"
    assert receipt.payload.method_ref == "acme/cost_attribution@1.0.0"
```

---

## 11. Migration Path

### Phase 1: Instrumentation (Weeks 1-2)
- Add SDK to existing pipelines
- Instrument high-ROI operators (partition, group-by)
- Emit receipts without reuse decisions

### Phase 2: Reuse Decisions (Weeks 3-4)
- Enable overlap detection
- Apply reuse decision rule
- Validate savings

### Phase 3: Policy Integration (Weeks 5-6)
- Define policies per domain
- Policy-driven thresholds
- Audit reuse decisions

### Phase 4: Settlement (Weeks 7-8)
- Generate settlement receipts
- Cross-team compute credits
- Netting across parties

---

## 12. Next Steps

1. **Prototype Python SDK:** Implement decorator-based instrumentation
2. **Spark Integration:** Custom UDFs with receipt emission
3. **MCP Server:** Receipt query API for finance/engineering teams
4. **Benchmarking:** Measure overhead of receipt generation
5. **Documentation:** SDK usage guides per framework

---

**References:**
- Northroot Engine: `crates/northroot-engine/src/`
- Delta Compute Spec: `docs/specs/delta_compute.md`
- Receipt Schema: `schemas/receipts/`

