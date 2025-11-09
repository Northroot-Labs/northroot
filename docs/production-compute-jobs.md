# Production Compute Jobs: Real Utility Examples

**Status:** Planning  
**Date:** 2025-11-08  
**Purpose:** Define real compute jobs that demonstrate end-to-end utility of delta compute with verifiable reuse

## Overview

These compute jobs move beyond demonstrations to show actual production value. Each job:
1. Processes real data (not simulated)
2. Stores receipts with chunk sets in persistent storage
3. Computes actual overlap from previous runs
4. Emits verifiable receipts proving reuse
5. Demonstrates measurable cost savings

## Job Catalog

### 1. FinOps Cost Attribution Job

**Domain:** Financial Operations  
**ROI:** 46% savings, $276K annual  
**Overlap:** 85-95% daily

**Description:**
- Processes cloud billing data (AWS, GCP, Azure) daily
- Groups costs by account/project/tag
- Generates cost attribution reports
- Only 5-15% of billing data changes daily

**Implementation:**
```rust
pub struct FinOpsCostAttributionJob {
    billing_files: Vec<PathBuf>,
    output_path: PathBuf,
    policy_ref: String,
}

impl ComputeJob for FinOpsCostAttributionJob {
    fn execute(&self, ctx: &JobContext) -> Result<Receipt, JobError> {
        // 1. Load previous execution receipt
        let prev_receipt = ctx.storage.get_previous_execution(
            &ctx.trace_id,
            &ctx.policy_ref,
        )?;
        
        // 2. Process billing files, extract resource tuples
        let resource_tuples = self.process_billing_files()?;
        let current_chunks = tuples_to_chunk_ids(&resource_tuples);
        
        // 3. Compute overlap with previous run
        let overlap_j = if let Some(prev) = &prev_receipt {
            let prev_chunks = ctx.storage.get_chunk_set(&prev.rid)?;
            jaccard_similarity(&current_chunks, &prev_chunks.unwrap_or_default())
        } else {
            0.0
        };
        
        // 4. Make reuse decision
        let cost_model = load_cost_model_from_policy(&ctx.policy_ref, Some(resource_tuples.len()))?;
        let (decision, justification) = decide_reuse(overlap_j, &cost_model, Some(resource_tuples.len()));
        
        // 5. Emit execution receipt with chunk set
        let exec_receipt = emit_execution_receipt(
            trace_id: &ctx.trace_id,
            chunk_set: current_chunks,
            prev_execution_rid: prev_receipt.map(|r| r.rid),
        )?;
        
        // 6. Store receipt
        ctx.storage.store(&exec_receipt)?;
        ctx.storage.store_chunk_set(&exec_receipt.rid, &current_chunks)?;
        
        // 7. Emit spend receipt
        let spend_receipt = emit_spend_receipt(
            exec_rid: exec_receipt.rid,
            justification: justification,
        )?;
        ctx.storage.store(&spend_receipt)?;
        
        Ok(spend_receipt)
    }
}
```

**Data Sources:**
- AWS: `aws ce get-cost-and-usage` API or Cost Explorer exports
- GCP: BigQuery billing export
- Azure: Cost Management API

**Output:**
- Cost attribution CSV/Parquet
- Execution receipt with resource tuple chunk set
- Spend receipt with reuse justification

### 2. ETL Partition Reuse Job

**Domain:** Data Engineering  
**ROI:** 39% savings, $372K annual  
**Overlap:** 80-90% partition reuse

**Description:**
- Processes Delta Lake tables with Change Data Feed (CDF)
- Only processes changed partitions
- Applies transformations (filter, aggregate, join)
- Typically 10-20% of partitions change daily

**Implementation:**
```rust
pub struct ETLPartitionReuseJob {
    delta_table_path: PathBuf,
    transformations: Vec<Transformation>,
    output_path: PathBuf,
    policy_ref: String,
}

impl ComputeJob for ETLPartitionReuseJob {
    fn execute(&self, ctx: &JobContext) -> Result<Receipt, JobError> {
        // 1. Load previous execution receipt
        let prev_receipt = ctx.storage.get_previous_execution(
            &ctx.trace_id,
            &ctx.policy_ref,
        )?;
        
        // 2. Scan Delta Lake CDF for changed partitions
        let changed_partitions = self.scan_cdf()?;
        let current_partitions = partitions_to_chunk_ids(&changed_partitions);
        
        // 3. Compute overlap with previous run
        let overlap_j = if let Some(prev) = &prev_receipt {
            let prev_chunks = ctx.storage.get_chunk_set(&prev.rid)?;
            let prev_partitions = prev_chunks.unwrap_or_default();
            // Compute Jaccard on unchanged partitions
            let unchanged: HashSet<String> = current_partitions
                .difference(&partitions_to_chunk_ids(&changed_partitions))
                .cloned()
                .collect();
            jaccard_similarity(&unchanged, &prev_partitions)
        } else {
            0.0
        };
        
        // 4. Make reuse decision
        let cost_model = load_cost_model_from_policy(&ctx.policy_ref, Some(changed_partitions.len() * 1000))?;
        let (decision, justification) = decide_reuse(overlap_j, &cost_model, Some(changed_partitions.len() * 1000));
        
        // 5. Process only changed partitions if reusing
        if decision == ReuseDecision::Reuse {
            // Reuse previous results for unchanged partitions
            // Only process changed partitions
        } else {
            // Full recomputation
        }
        
        // 6. Emit execution receipt with partition set
        let exec_receipt = emit_execution_receipt(
            trace_id: &ctx.trace_id,
            chunk_set: current_partitions,
            cdf_metadata: Some(changed_partitions),
            prev_execution_rid: prev_receipt.map(|r| r.rid),
        )?;
        
        ctx.storage.store(&exec_receipt)?;
        ctx.storage.store_chunk_set(&exec_receipt.rid, &current_partitions)?;
        
        Ok(exec_receipt)
    }
}
```

**Data Sources:**
- Delta Lake tables with CDF enabled
- Partition metadata from Delta transaction log

**Output:**
- Transformed data (Parquet/Delta)
- Execution receipt with partition chunk set
- Spend receipt with reuse justification

### 3. Analytics Query Job

**Domain:** Business Intelligence  
**ROI:** 142% savings, $684K annual  
**Overlap:** 90%+ query result overlap

**Description:**
- Executes SQL queries against data warehouse
- Caches query results
- Incremental refresh when underlying data changes
- Typical: 90%+ of query results unchanged

**Implementation:**
```rust
pub struct AnalyticsQueryJob {
    query: String,
    data_source: DataSource,
    cache_path: PathBuf,
    policy_ref: String,
}

impl ComputeJob for AnalyticsQueryJob {
    fn execute(&self, ctx: &JobContext) -> Result<Receipt, JobError> {
        // 1. Load previous execution receipt
        let prev_receipt = ctx.storage.get_previous_execution(
            &ctx.trace_id,
            &ctx.policy_ref,
        )?;
        
        // 2. Execute query
        let query_results = self.execute_query()?;
        let current_chunks = query_results_to_chunk_ids(&query_results);
        
        // 3. Compute overlap with previous results
        let overlap_j = if let Some(prev) = &prev_receipt {
            let prev_chunks = ctx.storage.get_chunk_set(&prev.rid)?;
            jaccard_similarity(&current_chunks, &prev_chunks.unwrap_or_default())
        } else {
            0.0
        };
        
        // 4. Make reuse decision
        let cost_model = load_cost_model_from_policy(&ctx.policy_ref, None)?;
        let (decision, justification) = decide_reuse(overlap_j, &cost_model, None);
        
        // 5. Cache results if reusing
        if decision == ReuseDecision::Reuse {
            // Use cached results for overlapping rows
            // Only compute new/changed rows
        }
        
        // 6. Emit execution receipt with result chunk set
        let exec_receipt = emit_execution_receipt(
            trace_id: &ctx.trace_id,
            chunk_set: current_chunks,
            prev_execution_rid: prev_receipt.map(|r| r.rid),
        )?;
        
        ctx.storage.store(&exec_receipt)?;
        ctx.storage.store_chunk_set(&exec_receipt.rid, &current_chunks)?;
        
        Ok(exec_receipt)
    }
}
```

**Data Sources:**
- SQL queries against data warehouse (BigQuery, Snowflake, Redshift)
- Query result caching

**Output:**
- Query results (CSV/Parquet)
- Execution receipt with result chunk set
- Spend receipt with reuse justification

### 4. Feature Store Update Job

**Domain:** Machine Learning  
**ROI:** 30-40% savings (estimated)  
**Overlap:** 70-85% feature overlap

**Description:**
- Computes ML features from raw data
- Only recomputes features for changed data
- Typical: 10-25% of source data changes daily

**Implementation:**
Similar pattern to ETL job, but focused on feature computation.

### 5. Data Quality Check Job

**Domain:** Data Engineering  
**ROI:** 25-35% savings (estimated)  
**Overlap:** 80-90% validation overlap

**Description:**
- Validates data quality (schema, constraints, freshness)
- Only re-validates changed data
- Typical: 10-20% of data changes daily

**Implementation:**
Similar pattern to ETL job, but focused on validation.

## Implementation Priority

1. **P0 (Week 3-4)**: FinOps Cost Attribution Job
   - Highest ROI ($276K annual)
   - Clear use case
   - Real data sources available

2. **P1 (Week 4-5)**: ETL Partition Reuse Job
   - High ROI ($372K annual)
   - Delta Lake CDF integration
   - Common pattern

3. **P2 (Week 5-6)**: Analytics Query Job
   - Highest savings percentage (142%)
   - Requires query caching infrastructure
   - More complex

4. **P3 (Future)**: Feature Store & Data Quality Jobs
   - Lower priority
   - Similar patterns to ETL

## Testing Strategy

Each job should have:
1. **Unit Tests**: Test job logic in isolation
2. **Integration Tests**: Test with real data sources (small samples)
3. **Verification Tests**: Verify reuse decisions are correct
4. **Performance Tests**: Measure actual savings vs full recomputation

## Success Metrics

- **Reuse Rate**: % of work reused (target: 80%+)
- **Economic Delta**: Actual savings (target: positive ΔC)
- **Verification**: Independent verification of reuse decisions (target: 100%)
- **Performance**: Job runtime vs full recomputation (target: 50%+ faster)

## References

- [ADR-008: Production Hardening](ADRs/ADR-008-production-hardening-storage.md)
- [Applied Use Cases](research/reports/delta_compute/cursor/applied_use_case_sheets.md)
- [Delta Compute Spec](docs/specs/delta_compute.md)

